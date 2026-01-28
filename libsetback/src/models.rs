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
	pub refresh_token: String,
	pub username: String,
}

/// GET /api/username
#[derive(Deserialize)]
pub struct UsernameResponse {
	pub username: String,
}

/// POST /api/refresh
#[derive(Serialize)]
pub struct RefreshRequest {
	pub refresh_token: String,
}

/// POST /api/refresh
#[derive(Deserialize)]
pub struct RefreshResponse {
	pub auth_token: String,
}