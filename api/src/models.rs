use {
	serde::{Deserialize, Serialize},
	sqlx::MySqlPool,
};

#[derive(Clone)]
pub struct AppState {
	pub db: MySqlPool,
	pub jwt_secret: Vec<u8>,
	pub api_key_hash: [u8; 32]
}

impl axum::extract::FromRef<AppState> for MySqlPool {
	fn from_ref(state: &AppState) -> Self {
		state.db.clone()
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
	/// Expiration time (as UTC timestamp)
	pub exp: usize,
	/// Issued at (as UTC timestamp)
	pub iat: usize,
	/// Subject (whom token refers to - email:username)
	pub sub: String,
	pub account_id: u32,
	pub owner_id: u32,
}