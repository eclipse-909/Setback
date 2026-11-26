use {
	serde::{Deserialize, Serialize},
	sqlx::PgPool,
};

#[derive(Clone)]
pub struct AppState {
	pub db: PgPool,
	pub jwt_secret: Vec<u8>,
}

impl axum::extract::FromRef<AppState> for PgPool {
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
	pub account_id: i32,
	pub profile_id: i32,
	pub suspended: bool,
}