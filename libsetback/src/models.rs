use serde::{Deserialize, Serialize};

/// POST /api/login
#[derive(Serialize, Deserialize)]
pub struct LoginRequest {
	pub username_or_email: String,
	pub password: String,
}

/// POST /api/signup
#[derive(Serialize, Deserialize)]
pub struct SignupRequest {
	pub username: String,
	pub email: String,
	pub password: String,
}

/// POST /api/login and POST /api/signup
#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
	pub auth_token: String,
	pub renew_token: String,
	pub username: String,
}

/// GET /api/username
#[derive(Serialize, Deserialize)]
pub struct UsernameResponse {
	pub username: String,
	pub account_id: u32,
}

/// POST /api/report
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReportPlayerRequest {
	pub username: String,
	pub target_idx: u8,
	pub reason: ReportReason,
	pub description: String,
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

impl std::fmt::Display for ReportReason {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", match self {
			ReportReason::Stalling =>
				"Stalling is automatically reported, and the player is kicked from the game if they don't play for 3 turns in a game.",
			ReportReason::Hacking =>
				"A player is exploiting a bug or vulnerability in the game to give themselves or another player an unfair advantage.",
			ReportReason::Cheating =>
				"A player is using external communication with teammates or opponents to influence the outcome of the game.",
		})
	}
}

/// POST /api/feedback
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FeedbackRequest {
	description: String,
}
