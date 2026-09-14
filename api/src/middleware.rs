use {
	axum::{extract::Request, http::StatusCode, middleware::Next, response::IntoResponse},
	setback::{SETBACK_PROXY, SSP_VERSION, Version},
	serde::{Deserialize, Serialize},
	tower_cookies::{
		Cookie, Cookies,
		cookie::{
			Key, SameSite,
			time::{Duration, OffsetDateTime, UtcDateTime},
		},
	},
};

#[derive(Serialize, Deserialize)]
struct AuthToken {
	/// Expiration time (as UTC timestamp)
	expires: i64,
	user: User,
	version: Version,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
	account_id: u32,
	username: String,
}

impl AuthToken {
	pub fn new(account_id: u32, username: String) -> Self {
		Self {
			expires: (UtcDateTime::now() + Duration::days(7)).unix_timestamp(),
			user: User {
				account_id,
				username,
			},
			version: SSP_VERSION,
		}
	}

	pub fn insert(&self, cookies: &mut Cookies, key: &Key) -> Result<(), StatusCode> {
		let Ok(token) = serde_json::to_string(self) else {
			return Err(StatusCode::INTERNAL_SERVER_ERROR);
		};
		let cookie = Cookie::build((http::header::AUTHORIZATION.as_str(), token))
			.domain(SETBACK_PROXY)
			.path("/")
			.secure(true)
			.http_only(true)
			.same_site(SameSite::Strict)
			.expires(Some(
				OffsetDateTime::from_unix_timestamp(self.expires)
					.map_err(|_| StatusCode::UNAUTHORIZED)?,
			))
			.build();
		cookies.private(key).add(cookie);
		Ok(())
	}

	pub fn set_expired(&mut self) {
		self.expires = 0;
	}
}

/// Auth middleware for account routes
/// - Decrypts authorization private cookie using Key from extensions
/// - Validates embedded expiration
pub async fn middleware_auth(cookies: Cookies, mut req: Request, next: Next) -> impl IntoResponse {
	// Decrypt private cookie and extract token
	let key = match req.extensions().get::<Key>() {
		Some(k) => k,
		None => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
	};
	let decrypted = match cookies
		.private(key)
		.get(http::header::AUTHORIZATION.as_str())
	{
		Some(c) => c,
		None => return StatusCode::UNAUTHORIZED.into_response(),
	};
	let auth_token: AuthToken = match serde_json::from_str(decrypted.value()) {
		Ok(t) => t,
		Err(_) => return StatusCode::UNAUTHORIZED.into_response(),
	};

	if UtcDateTime::now().unix_timestamp() >= auth_token.expires {
		return StatusCode::UNAUTHORIZED.into_response();
	}

	req.extensions_mut().insert(auth_token.user);
	next.run(req).await
}
