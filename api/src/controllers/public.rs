use {
	crate::{constants::*, models::*, status::*, util::*, middleware::jwt::*},
	argon2::{
		Argon2,
		password_hash::PasswordHasher,
	},
	axum::{Json, extract::{ConnectInfo, State}},
	base64::engine::{Engine, general_purpose::URL_SAFE_NO_PAD},
	http::{HeaderMap, StatusCode},
	setback::*,
	sha2::{Digest, Sha256},
	sqlx::{PgPool, types::ipnet::IpNet},
	std::
	net::{IpAddr, SocketAddr},
	tracing::error,
	utoipa_axum::{router::OpenApiRouter as Router, routes},
};

#[utoipa::path(
	post,
	path = "/signup",
	request_body = BeginSignupRequest,
	responses((status = 200)),
	responses((status = 403)),
	responses((status = 409)),
	responses((status = 500)),
	tag = "public"
)]
async fn api_begin_signup(
	State(db): State<PgPool>,
	Json(creds): Json<BeginSignupRequest>,
) -> Result<(), Status> {
	// validate request
	let email = email_validated(creds.email)?;
	let username = username_validated(creds.username)?;
	let password = password_validated(creds.password)?;
	let email_normalized = email_normalized(&email);
	
	// create temporary secure code; only its hash is stored
	let token = new_token32();
	let token_hash = hash_token32(token.as_bytes());
	
	// hash the password before opening the transaction, so the tx (and its locks) isn't held open during the expensive step
	let password_hash = hash_password(password).await?;
	
	// One transaction, two statements
	let mut tx = db.begin().await?;
	
	// Clear expired pending signups that would otherwise block this
	// username/email. Kept as its own statement: inside the CTE below, the
	// delete and the insert would share a snapshot, so the insert could
	// still collide with the row being deleted.
	sqlx::query!(
		r#" delete from stage_account
			where expires_at < now()
				and (lower(username) = lower($1) or email_normalized = $2)
		"#,
		username,
		email_normalized
	).execute(&mut *tx).await?;
	
	// All checks + the upsert in a single statement (one snapshot).
	// The insert only produces a row when every check passes; the final
	// select reports which check (if any) failed.
	// Explicit casts on the insert params are needed so Postgres/sqlx can
	// infer their types (e.g. $5 must be bytea, not text).
	let upsert = sqlx::query_scalar!(
		r#" with chk as (
				select
				    -- if suspended
					exists(
						select 1
						from profile_email pe
						join profile p using (profile_id)
						where pe.email_normalized = $2
							and p.suspended_at is not null
							and (p.suspended_until is null or p.suspended_until > now())
					) as suspended,
				    -- if username is taken
					exists(
						select 1 from account where lower(username) = lower($1)
					) as username_taken,
				    -- if username is taken in staging table
					exists(
						select 1 from stage_account
						where lower(username) = lower($1) and email_normalized <> $2
					) as username_staged,
				    -- get number of accounts on profile
					(
						select count(*)
						from account a
						join profile_email pe using (profile_id)
						where pe.email_normalized = $2
							and a.deleted_at is null
					) as account_count
			),
			ins as (
				insert into stage_account
					(username, email_normalized, email, password_hash, tmp_token_hash, expires_at)
				select
					$1::varchar, $2::varchar, $3::varchar, $4::text, $5::bytea,
					now() + make_interval(mins => $6::int)
				from chk
				where not suspended
					and not username_taken
					and not username_staged
					and account_count < $7
				on conflict (email_normalized) do update set
					username       = excluded.username,
					email          = excluded.email,
					password_hash  = excluded.password_hash,
					tmp_token_hash = excluded.tmp_token_hash,
					expires_at     = excluded.expires_at
				returning 1
			)
			select case
				when suspended                         then 'suspended'
				when username_taken or username_staged then 'username_taken'
				when account_count >= $7               then 'limit'
				else 'ok'
			end as "outcome!"
			from chk
		"#,
		username,
		email_normalized,
		email,
		password_hash,
		&token_hash,
		SIGNUP_TOKEN_TTL_MINUTES,
		MAX_ACCOUNTS_PER_OWNER
	).fetch_one(&mut *tx).await;
	
	let outcome = match upsert {
		Ok(o) => o,
		// Lost a race on username (PK) against a concurrent signup
		Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23505") => {
			return Err(Status::s409("username is already taken"));
		}
		Err(e) => return Err(e.into()),
	};
	
	match outcome.as_str() {
		"ok" => {}
		"suspended" => return Err(Status::s403("This email cannot be used to sign up")),
		"username_taken" => return Err(Status::s409("Username is already taken")),
		"limit" => return Err(Status::s403("Account limit reached for this email")),
		other => {
			error!("unexpected signup outcome: {other}");
			return Err(Status::s500());
		}
	}
	tx.commit().await?;
	
	// send email with the code
	let email = Email {
		to: email,
		subject: "Confirm your signup".to_owned(),
		body: format!(
			"Welcome, {username}!\n\n\
			 Your confirmation code is:\n\n{token}\n\n\
			 It expires in {SIGNUP_TOKEN_TTL_MINUTES} minutes. \
			 If you didn't request this, you can ignore this email."
		),
	};
	email.send().await?;
	
	Ok(())
}

#[utoipa::path(
	get,
	path = "/signup",
	request_body = FinishSignupRequest,
	responses((status = 201, body = LoginResponse)),
	responses((status = 403)),
	responses((status = 404)),
	responses((status = 500)),
	tag = "public"
)]
async fn api_finish_signup(
	State(state): State<AppState>,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	headers: HeaderMap,
	Json(FinishSignupRequest {token}): Json<FinishSignupRequest>,
) -> Result<Json<LoginResponse>, Status> {
	let ip_addr = client_ip(&headers, peer);
	
	let mut tx = state.db.begin().await?;
	
	struct Checks {
		token_valid: bool,
		profile_id: Option<i32>,
	}
	let checks: Option<Checks> = sqlx::query_as!(
		Checks,
		r#" with staged as (
				select expires_at, email_normalized from stage_account
				where tmp_token_hash = $1
			)
 			select
     			staged.expires_at >= now() as "token_valid!",
     			(
					select profile_id from profile_email
					where email_normalized = staged.email_normalized
				) as profile_id
 			from staged
		"#,
		token.as_bytes()
	).fetch_optional(&mut *tx).await?;
	
	let Some(checks) = checks else {
		return Err(Status::s404("Code not found"));
	};
	if !checks.token_valid {
		return Err(Status::s403("Code expired"));
	}
	
	let profile_id = checks.profile_id.unwrap_or(
		sqlx::query_scalar!(
			r#" with staged as (
					select email, email_normalized from stage_account
					where tmp_token_hash = $1
				),
				prof_id as (
                    insert into profile default values
                    returning profile_id
				),
				ins_profile_email as (
                    insert into profile_email(profile_id, email, email_normalized)
					select (select * from prof_id), email, email_normalized
					from staged
					returning 1
				),
				ins_profile_ip as (
                    insert into profile_ip(profile_id, ip)
					select *, $2 from prof_id
					returning 1
				)
 				select * from prof_id
			"#,
			token.as_bytes(),
			ip_addr,
		).fetch_one(&mut *tx).await?
	);
	
	let refresh_token = new_token32();
	let refresh_token_hash = hash_token32(refresh_token.as_bytes());
	
	struct Account {
		id: i32,
		username: String,
		email: String,
	}
	let account = sqlx::query_as!(
		Account,
		r#" with staged as (
				select username, email_normalized, password_hash from stage_account
				where tmp_token_hash = $1
			),
			acc_id as (
                insert into account(profile_id, username, username_archive, password_hash)
                select $2, username, username, password_hash
				from staged
				returning account_id
			),
			ins_refresh_token as (
                insert into refresh_token(token_hash, profile_id, account_id, expires_at)
				select $3, $2, *, now() + make_interval(hours => $4::int)
				from acc_id
				returning 1
			),
			rm_staged as (
                delete from stage_account
				where tmp_token_hash = $1
				returning 1
			)
 			select
				(select * from acc_id) as "id!",
				username,
				email_normalized as email
 			from staged
		"#,
		token.as_bytes(),
		profile_id,
		&refresh_token_hash,
		REFRESH_TOKEN_TTL_HOURS,
	).fetch_one(&mut *tx).await?;
	
	tx.commit().await?;
	
	let jwt = create_jwt(account.id as u32, profile_id as u32, &account.username, &account.email, &state)?;
	
	Ok(Json(LoginResponse {
		auth_token: jwt,
		refresh_token,
		username: account.username,
	}))
}

#[utoipa::path(
	post,
	path = "/login",
	request_body = LoginRequest,
	responses((status = 200, body = LoginResponse)),
	tag = "public"
)]
async fn api_login(
	State(db): State<PgPool>,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	headers: HeaderMap,
	Json(creds): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
	todo!()
	// validate credentials
	// create new jwt and refresh token,
	// return response
}

#[utoipa::path(
	post,
	path = "/refresh-token",
	request_body = RefreshTokenRequest,
	responses((status = 200, body = RefreshTokenResponse)),
	responses((status = 401)),
	responses((status = 403)),
	responses((status = 500)),
	tag = "public"
)]
async fn api_refresh_token(
	State(state): State<AppState>,
	Json(RefreshTokenRequest {refresh_token}): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, Status> {
	let refresh_token_hash = hash_token32(refresh_token.as_bytes());
	struct Token {
		expired: bool,
		profile_id: i32,
		account_id: i32,
		username: String,
		email: String,
	}
	let token = sqlx::query_as!(
		Token,
		r#" with token as (
     			select r.profile_id, r.account_id, r.expires_at, a.username, p.email_normalized
     			from refresh_token r
     			inner join account a on a.account_id = r.account_id
     			inner join profile_email p on p.profile_id = r.profile_id
 				where token_hash = $1
			)
 			select (expires_at < now()) as "expired!", profile_id, account_id, username as "username!", email_normalized as "email!"
 			from token
		"#,
		&refresh_token_hash,
	).fetch_optional(&state.db).await?;
	
	let Some(token) = token else {
		return Err(Status::s401("Refresh token not found"))
	};
	if token.expired {
		return Err(Status::s403("Refresh token expired"))
	}
	
	let jwt = create_jwt(token.account_id as u32, token.profile_id as u32, &token.username, &token.email, &state)?;
	
	Ok(Json(RefreshTokenResponse {
		auth_token: jwt,
	}))
}

pub fn api_public_routes() -> Router<AppState> {
	Router::new()
		.routes(routes!(api_begin_signup, api_finish_signup))
		.routes(routes!(api_login))
		.routes(routes!(api_refresh_token))
}

// ---------- helpers ----------

fn new_token32() -> String {
	let mut raw = [0u8; 32];
	rand::fill(&mut raw);
	URL_SAFE_NO_PAD.encode(raw)
}

fn hash_token32(code: &[u8]) -> [u8; 32] {
	Sha256::digest(code).into()
}

async fn hash_password(password: String) -> Result<String, Status> {
	tokio::task::spawn_blocking(move || {
		Argon2::default()
			.hash_password(password.as_bytes())
			.map(|h| h.to_string())
	}).await
		.map_err(|e| {
			error!("hash task panicked: {e}");
			Status::s500()
		})?
		.map_err(|e| {
			error!("argon2 failure: {e}");
			Status::s500()
		})
}

fn client_ip(headers: &HeaderMap, peer: SocketAddr) -> IpNet {
	headers
		.get("x-forwarded-for")
		.and_then(|v| v.to_str().ok())
		// rightmost entry = the one appended by YOUR proxy (not client-controlled)
		.and_then(|v| v.rsplit(',').next())
		.and_then(|s| s.trim().parse::<IpAddr>().ok())
		.unwrap_or_else(|| peer.ip()) // fallback: direct connection
		.into()
}

fn email_validated(email: String) -> Result<String, Status> {
	const BAD_EMAIL: &str = "invalid email address";
	let email = email.trim();
	if email.len() > 255 || email.chars().any(|c| c.is_whitespace() || c.is_control()) {
		return Err(Status::s400(BAD_EMAIL));
	}
	let (local, domain) = email.split_once('@').ok_or_else(|| Status::s400(BAD_EMAIL))?;
	if local.is_empty()
		|| local.len() > 64
		|| domain.contains('@')
		|| !domain.contains('.')
		|| domain.starts_with('.')
		|| domain.ends_with('.')
		|| domain.contains("..")
	{
		return Err(Status::s400(BAD_EMAIL));
	}
	Ok(email.to_owned())
}

/// Must be the same function used wherever `profile_email.email_normalized`
/// is written (e.g. the signup-confirmation endpoint), or the checks above
/// won't line up.
fn email_normalized(email: &str) -> String {
	let lower = email.trim().to_lowercase();
	let (local, domain) = lower.split_once('@').unwrap_or((&lower, ""));
	let local = local.split('+').next().unwrap_or(local);
	if domain == "gmail.com" || domain == "googlemail.com" {
		format!("{}@gmail.com", local.replace('.', ""))
	} else {
		format!("{local}@{domain}")
	}
}

fn username_validated(username: String) -> Result<String, Status> {
	let username = username.trim();
	let len = username.chars().count();
	if !(3..=15).contains(&len) {
		return Err(Status::s400("username must be 3-15 characters"));
	}
	if !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
		return Err(Status::s400(
			"username may only contain letters, numbers and underscores",
		));
	}
	if !is_username_appropriate(username) {
		return Err(Status::s400("username is not allowed"));
	}
	Ok(username.to_owned())
}

/// Minimal placeholder: swap in a real profanity/impersonation list.
fn is_username_appropriate(username: &str) -> bool {
	todo!();
	const BLOCKED: &[&str] = &["admin", "administrator", "moderator", "mod", "staff", "support", "system"];
	// Undo common leetspeak so "adm1n" doesn't slip through.
	let folded: String = username
		.to_ascii_lowercase()
		.chars()
		.map(|c| match c {
			'0' => 'o',
			'1' | '!' => 'i',
			'3' => 'e',
			'4' | '@' => 'a',
			'5' | '$' => 's',
			'7' => 't',
			_ => c,
		})
		.filter(|c| *c != '_')
		.collect();
	!BLOCKED.iter().any(|b| folded.contains(b))
}

fn password_validated(password: String) -> Result<String, Status> {
	let password = password.trim();
	let len = password.chars().count();
	if len < 8 {
		return Err(Status::s400("password must be at least 8 characters"));
	}
	// Upper bound keeps argon2 from being used as a CPU/memory DoS vector.
	if len > 128 {
		return Err(Status::s400("password must be at most 128 characters"));
	}
	Ok(password.to_owned())
}