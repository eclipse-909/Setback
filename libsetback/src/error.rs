use {
	serde::{Deserialize, Serialize},
	thiserror::Error,
};

#[repr(u8)]
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Error)]
pub enum NetError {
	#[error("Disconnected")]
	Disconnected,
	#[error("Bad Request")]
	BadRequest,
	#[error("Timed Out")]
	TimedOut,
	#[error("Internal server error")]
	ServerError,
}

#[repr(u8)]
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Error)]
pub enum PromptError {
	#[error(transparent)]
	Net(#[from] NetError),
	#[error("Invalid Prompt Response")]
	Invalid,
}

#[repr(u8)]
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Error)]
pub enum Error {
	#[error(transparent)]
	Net(#[from] NetError),
	#[error(transparent)]
	Prompt(#[from] PromptError),
	#[error("Failed to connect to API")]
	ApiErr,
	#[error("An unknown error occurred. Please report this")]
	Unknown,
	#[error("Environment variable is not set. Please set it and try again")]
	BadEnvVar,
	#[error("Incompatible version. Please update the game")]
	BadVersion,
	#[error("Unauthenticated. Please log in")]
	Unauthenticated,
	#[error("This account is already logged in. If this is a mistake, please change your password immediately")]
	AlreadyConnected,
	#[error("Logical error. Please report this")]
	LogicError,
	#[error("The server is down. Please try again later")]
	ServerDown,
	#[error("Cannot create a new game. Please try again later")]
	GamesAtCapacity,
	#[error("Game is full")]
	GameAtCapacity,
	#[error("Cannot rejoin a game you intentionally left")]
	CannotRejoin,
	#[error("Game code not found. Please check the code and try again")]
	GameNotFound,
	#[error("Bad player index")]
	BadPlayerIndex,
	#[error("Bad team number")]
	BadTeamNumber,
	#[error("Must be the host of the lobby to perform this action")]
	NotHost,
}

impl From<reqwest::Error> for Error {
	fn from(value: reqwest::Error) -> Self {
		match value.status().map(|o| o.as_u16()) {
			Some(401) => Error::Unauthenticated,
			_ => Error::LogicError,
		}
	}
}