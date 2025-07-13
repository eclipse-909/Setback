use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct AccountData {
	///alias for rowid in sqlite
	pub account_id: u32,
	pub username: String,
	pub password: String,
	pub last_active: String
}

#[derive(Serialize, Deserialize)]
pub struct LoginCredentials {
	pub username: String,
	pub password: String
}

#[derive(Serialize, Deserialize)]
pub struct LoggedIn {
	pub logged_in: bool
}