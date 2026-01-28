use {
	std::{
		fmt::{Display, Formatter},
		io::ErrorKind
	},
	serde::{Deserialize, Serialize},
};

#[repr(u8)]
#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub enum Error {
	// Unknown,
	NetworkError,
	BadEncoding,
	BadRequest,
	BadVersion,
	BadProtocol,
	Unauthenticated,
	// Unauthorized,
	TimedOut,
	ServerError,
	DevError,
	ServerDown,
	// NotFound,
	Disconnected,
	GamesAtCapacity,
	GameAtCapacity,
	GameNotFound,
	BadBid,
	BadCard,
	// OutOfTurn,
	IndexOutOfBounds,
	BadPlayerIndex,
	BadTeamNumber,
}

impl std::error::Error for Error {}
impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			f, "{}",
			match self {
				// Error::Unknown => "An unknown error occurred. Please report this.",
				Error::NetworkError => "Network error. Please try again later.",
				Error::BadEncoding => "Message is not in proper encoding.",
				Error::BadRequest => "Bad request.",
				Error::BadVersion => "Incompatible version. Please update the game.",
				Error::BadProtocol => "Unexpected protocol.",
				Error::Unauthenticated => "Unauthenticated. Please log in.",
				// Error::Unauthorized => "Unauthorized. Please log in as an admin.",
				Error::TimedOut => "Request timed out.",
				Error::ServerError => "Internal server error.",
				Error::DevError => "Local game error. Please report this.",
				Error::ServerDown => "The server is down. Please try again later.",
				// Error::NotFound => "Not found.",
				Error::Disconnected => "Disconnected. Please reconnect.",
				Error::GamesAtCapacity => "Cannot create a new game. Please try again later.",
				Error::GameAtCapacity => "Game is full, cannot join.",
				Error::GameNotFound => "Game code not found. Please check the code and try again.",
				Error::BadBid => "Invalid bid. Please try a different bid.",
				Error::BadCard => "Invalid card. Please try a different card.",
				// Error::OutOfTurn => "It is not your turn.",
				Error::IndexOutOfBounds => "Index out of bounds.",
				Error::BadPlayerIndex => "Bad player index.",
				Error::BadTeamNumber => "Bad team number.",
			}
		)
	}
}

impl From<reqwest::Error> for Error {
	fn from(value: reqwest::Error) -> Self {
		match value.status().map(|o| o.as_u16()) {
			Some(401) => Error::Unauthenticated,
			_ => Error::DevError,
		}
	}
}

impl From<std::io::Error> for Error {
	fn from(value: std::io::Error) -> Self {
		match value.kind() {
			ErrorKind::ConnectionAborted => Error::Disconnected,
			ErrorKind::TimedOut => Error::TimedOut,
			_ => Error::NetworkError
		}
	}
}