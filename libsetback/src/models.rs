use {
	serde::{Deserialize, Serialize},
	std::time::Duration,
	utoipa::ToSchema,
};

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SignupRequest {
	pub email: String,
	pub username: String,
	pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
	pub username: String,
	pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct LoginResponse {
	pub auth_token: String,
	pub refresh_token: String,
	pub username: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
	pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshTokenResponse {
	pub auth_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UsernameResponse {
	pub username: String,
	pub owner_id: u32,
	pub account_id: u32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
	token: String,
	old_password: String,
	new_password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ChangeEmailRequest {
	new_email: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct FeedbackRequest {
	description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct ReportPlayerRequest {
	pub game_code: String,
	pub username: String,
	pub target_idx: u8,
	pub target_account_id: u32,
	pub reason: ReportReason,
	pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub enum ReportReason {
	/// Stalling is automatically reported, and the player is kicked from the game if they don't play for 3 turns in a game.
	Stalling,
	/// A player is exploiting a bug or vulnerability in the game to give themselves or another player an unfair advantage.
	Hacking,
	/// A player is using external communication with teammates or opponents to influence the outcome of the game.
	Cheating,
	/// This player sucks/is mean and I just don't like them.
	DontLikePlayer,
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
			ReportReason::DontLikePlayer =>
				"This player sucks/is mean and I just don't like them.",
		})
	}
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct StatsRequest {
	is_singleplayer: bool,
	won_game: Option<bool>,
	total_bid: Option<u32>,
	bids_won: Option<u32>,
	pts_won: Option<u32>,
	pts_lost: Option<u32>,
	game_pts_won: Option<u32>,
	game_pts_lost: Option<u32>,
	num_hearts: Option<u32>,
	num_diamonds: Option<u32>,
	num_clubs: Option<u32>,
	num_spades: Option<u32>,
	total_card_num: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct SuspendRequest {
	action: SuspendAction,
	duration: Duration,
	reason: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub enum SuspendAction { Suspend, Reinstate, Warn }