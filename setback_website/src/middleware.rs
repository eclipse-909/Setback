use {
	crate::constants::COOKIE_AUTH_TOKEN,
	axum::{
		extract::Request,
		http::StatusCode,
		middleware::Next,
		response::IntoResponse
	},
	chrono::{Duration, Utc},
	libsetback::{SETBACK_VERSION, Version},
	serde::{Deserialize, Serialize},
	sqlx::PgPool,
	tower_cookies::{
		Cookies,
		cookie::{
			Cookie, Key, SameSite,
			time::{Duration, OffsetDateTime},
		},
	}
};

#[derive(Serialize, Deserialize)]
struct AuthToken {
	user: User,
	max_age: Duration,
	expires: OffsetDateTime,
	version: Version,
}

#[derive(Serialize, Deserialize)]
pub struct User {
	pub id: u32,
	pub username: String,
}

impl AuthToken {
	pub fn new(user: User) -> Self {
		let max_age = Duration::hours(3);
		let expires = OffsetDateTime::now_utc() + age;
		Self {
			user,
			max_age,
			expires,
			version: SETBACK_VERSION
		}
	}

	pub fn new_refresh(user: User) -> Self {
		let max_age = Duration::weeks(1);
		let expires = OffsetDateTime::now_utc() + age;
		Self {
			user,
			max_age,
			expires,
			version: SETBACK_VERSION
		}
	}

	pub fn revoke(&mut self) {
		self.max_age = Duration::zero();
		self.expires = OffsetDateTime::UNIX_EPOCH;
	}
}

/// Auth middleware for account routes
/// - Decrypts `auth-token` private cookie using `Key` from extensions
/// - Validates embedded expiration and that the user exists in DB
/// - Inserts `User` into request extensions on success; otherwise 401
pub async fn middleware_auth(cookies: Cookies, mut req: Request, next: Next) -> impl IntoResponse {
	let key = match req.extensions().get::<Key>() {
		Some(k) => k,
		None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
	};
	let pool = match req.extensions().get::<PgPool>() {
		Some(p) => p,
		None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
	};

	// Decrypt private cookie and extract token
	let decrypted = match cookies.private(key).get(COOKIE_AUTH_TOKEN) {
		Some(c) => c,
		None => return StatusCode::UNAUTHORIZED.into_response(),
	};
	let auth_token: AuthToken = match serde_json::from_str(decrypted.value()) {
		Ok(t) => t,
		Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
	};

	let now = Utc::now().timestamp();
	if now > auth_token.expires {
		return StatusCode::UNAUTHORIZED.into_response();
	}

	// Ensure user exists
	// TODO: convert to macro query
	let exists_row = sqlx::query("SELECT EXISTS(SELECT 1 FROM accounts WHERE id = $1)")
		.bind(auth_token.user.id)
		.fetch_one(pool)
		.await
		.is_ok();

	if !exists_row {
		return StatusCode::UNAUTHORIZED.into_response();
	}

	// Attach user to request
	req.extensions_mut().insert(auth_token.user);

	next.run(req).await
}