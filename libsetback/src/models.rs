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

/// POST /api/report_player
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReportPlayerRequest {
	username: String,
	idx: u8,
	reason: ReportReason,
	description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReportReason {
	/// Stalling is automatically reported, and the player is kicked from the game if they don't play for 3 turns in a game.
	Stalling,
	/// A player is exploiting a bug or vulnerability in the game to give themselves or another player an unfair advantage.
	Hacking,
	/// A player is using external communication with teammates or opponents to influence the outcome of the game.
	Cheating,
}