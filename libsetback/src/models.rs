use serde::{Deserialize, Serialize};

///Used for /api/login
#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
	pub username: String,
	pub password: String
}

///Used for /api/login
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoginResponse {
	pub auth_token: String,
	pub refresh_token: String,
	pub account_id: u32
}

///Used for /api/signup
pub type SignupRequest = LoginRequest;

///Used for /api/signup
pub type SignupResponse = LoginResponse;

///Used for /api/refresh
#[derive(Serialize, Deserialize)]
pub struct RefreshRequest {
	pub refresh_token: String
}

///Used for /api/refresh
#[derive(Serialize, Deserialize)]
pub struct RefreshResponse {
	pub auth_token: String
}

///Used for /api/id
#[derive(Serialize, Deserialize)]
pub struct IdResponse {
	pub account_id: u32
}

///Used for /api/change_username
#[derive(Serialize, Deserialize)]
pub struct ChangeUsernameRequest {
	pub username: String
}

///Used for /api/change_password
#[derive(Serialize, Deserialize)]
pub struct ChangePasswordRequest {
	pub password: String
}

///Used for /api/update
#[derive(Serialize, Deserialize)]
pub struct UpdateResponse {
	pub update_available: bool
}

///Used for database
#[derive(Serialize, Deserialize)]
pub struct AccountData {
	///alias for rowid in sqlite
	pub account_id: u32,
	pub username: String,
	pub password: String,
	pub last_active: String
}