use serde::{Deserialize, Serialize};

/// POST /api/login and POST /api/signup
#[derive(Serialize)]
pub struct LoginRequest {
	pub username: String,
	pub password: String,
}

/// POST /api/login and POST /api/signup
#[derive(Deserialize)]
pub struct LoginResponse {
	pub auth_token: String,
	pub renew_token: String,
	pub username: String,
}

/// GET /api/username
#[derive(Deserialize)]
pub struct UsernameResponse {
	pub username: String,
	pub account_id: u32,
}

/// POST /api/renew
#[derive(Serialize)]
pub struct RenewRequest {
	pub renew_token: String,
}

/// POST /api/renew
#[derive(Deserialize)]
pub struct RenewResponse {
	pub auth_token: String,
}