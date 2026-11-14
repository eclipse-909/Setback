use {
	crate::{constants::*, models::*, status::*, util::*},
	argon2::{
		Argon2,
		password_hash::PasswordHasher,
	},
	axum::{Json, extract::{ConnectInfo, Query, State}},
	base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine},
	http::{HeaderMap, StatusCode},
	setback::*,
	sha2::{Digest, Sha256},
	sqlx::PgPool,
	std::{
		collections::HashMap,
		net::{IpAddr, SocketAddr},
	},
	tracing::error,
	utoipa_axum::{router::OpenApiRouter as Router, routes},
};

#[utoipa::path(
	post,
	path = "/signup",
	request_body = SignupRequest,
	responses((status = 200)),
	responses((status = 403)),
	responses((status = 409)),
	responses((status = 500)),
	tag = "public"
)]
async fn api_begin_signup(
	State(db): State<PgPool>,
	Json(creds): Json<SignupRequest>,
) -> Result<(), Status> {
	// 1. Validate request (cheap checks first, before any DB or hashing work)
	let email = creds.email.trim().to_owned();
	validate_email(&email)?;
	let username = creds.username.trim().to_owned();
	validate_username(&username)?;
	validate_password(&creds.password)?;
	let email_normalized = normalize_email(&email);
	
	// 2. Create temporary secure code; only its hash is stored
	let mut raw = [0u8; 32];
	rand::fill(&mut raw);
	let code = URL_SAFE_NO_PAD.encode(raw);
	let token_hash = Sha256::digest(code.as_bytes()).to_vec();
	
	// 3. Hash the password BEFORE opening the transaction, so the tx (and its locks)
	//    isn't held open during the expensive step.
	let password = creds.password;
	let password_hash = tokio::task::spawn_blocking(move || {
		Argon2::default()
			.hash_password(password.as_bytes())
			.map(|h| h.to_string())
	}).await
		.map_err(|e| {
			error!("hash task panicked: {e}");
			Status::s500("")
		})?
		.map_err(|e| {
			error!("argon2 failure: {e}");
			Status::s500("")
		})?;
	
	// 4. One transaction, two statements
	let mut tx = db.begin().await?;
	
	// 4a. Clear expired pending signups that would otherwise block this
	//     username/email. Kept as its own statement: inside the CTE below, the
	//     delete and the insert would share a snapshot, so the insert could
	//     still collide with the row being deleted.
	sqlx::query!(
		r#" delete from stage_account
			where expires_at < now()
				and (lower(username) = lower($1) or email_normalized = $2)
		"#,
		username,
		email_normalized
	).execute(&mut *tx).await?;
	
	// 4b. All checks + the upsert in a single statement (one snapshot).
	//     The insert only produces a row when every check passes; the final
	//     select reports which check (if any) failed.
	//     Explicit casts on the insert params are needed so Postgres/sqlx can
	//     infer their types (e.g. $5 must be bytea, not text).
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
		token_hash,
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
		"suspended" => return Err(Status::s403("this email cannot be used to sign up")),
		"username_taken" => return Err(Status::s409("username is already taken")),
		"limit" => return Err(Status::s403("account limit reached for this email")),
		other => {
			error!("unexpected signup outcome: {other}");
			return Err(Status::s500(""));
		}
	}
	tx.commit().await?;
	
	// 5. Send the email with the code (after commit, so the code is always valid)
	Email {
		to: email,
		subject: "Confirm your signup".to_owned(),
		body: format!(
			"Welcome, {username}!\n\n\
			 Your confirmation code is:\n\n{code}\n\n\
			 It expires in {SIGNUP_TOKEN_TTL_MINUTES} minutes. \
			 If you didn't request this, you can ignore this email."
		),
	}.send()
		.await
		.map_err(|e| {
			error!("failed to send signup email: {e}");
			Status::s500("")
		})?;
	
	Ok(())
}

#[utoipa::path(
	get,
	path = "/signup",
	params(("token" = String, Query)),
	responses((status = 201, body = LoginResponse)),
	tag = "public"
)]
async fn api_finish_signup(
	State(db): State<PgPool>,
	Query(params): Query<HashMap<String, String>>,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	headers: HeaderMap,
) -> Result<Json<LoginResponse>, StatusCode> {
	todo!()
	// remove from staging account table,
	// insert into account table and owner table if new email,
	// create new jwt and refresh token,
	// return response
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
	tag = "public"
)]
async fn api_refresh_token(
	State(db): State<PgPool>,
	Json(creds): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, StatusCode> {
	todo!()
	// validate refresh token in db,
	// return new auth token
}

pub fn api_public_routes() -> Router<AppState> {
	Router::new()
		.routes(routes!(api_begin_signup, api_finish_signup))
		.routes(routes!(api_login))
		.routes(routes!(api_refresh_token))
}

// ---------- helpers ----------

fn client_ip(headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
	headers
		.get("x-forwarded-for")
		.and_then(|v| v.to_str().ok())
		// rightmost entry = the one appended by YOUR proxy (not client-controlled)
		.and_then(|v| v.rsplit(',').next())
		.and_then(|s| s.trim().parse::<IpAddr>().ok())
		.unwrap_or_else(|| peer.ip()) // fallback: direct connection
}

fn validate_email(email: &str) -> Result<(), Status> {
	let bad = || Status::s400("invalid email address");
	if email.len() > 255 || email.chars().any(|c| c.is_whitespace() || c.is_control()) {
		return Err(bad());
	}
	let (local, domain) = email.split_once('@').ok_or_else(bad)?;
	if local.is_empty()
		|| local.len() > 64
		|| domain.contains('@')
		|| !domain.contains('.')
		|| domain.starts_with('.')
		|| domain.ends_with('.')
		|| domain.contains("..")
	{
		return Err(bad());
	}
	Ok(())
}

/// Must be the same function used wherever `profile_email.email_normalized`
/// is written (e.g. the signup-confirmation endpoint), or the checks above
/// won't line up.
fn normalize_email(email: &str) -> String {
	let lower = email.trim().to_lowercase();
	let (local, domain) = lower.split_once('@').unwrap_or((&lower, ""));
	let local = local.split('+').next().unwrap_or(local);
	if domain == "gmail.com" || domain == "googlemail.com" {
		format!("{}@gmail.com", local.replace('.', ""))
	} else {
		format!("{local}@{domain}")
	}
}

fn validate_username(username: &str) -> Result<(), Status> {
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
	Ok(())
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

fn validate_password(password: &str) -> Result<(), Status> {
	let len = password.chars().count();
	if len < 8 {
		return Err(Status::s400("password must be at least 8 characters"));
	}
	// Upper bound keeps argon2 from being used as a CPU/memory DoS vector.
	if len > 128 {
		return Err(Status::s400("password must be at most 128 characters"));
	}
	Ok(())
}