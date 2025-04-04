use {
	bincode::{Decode, Encode},
	std::{
		fmt::{Display, Formatter},
		future::Future
	},
	tokio::{
		sync::watch,
		task::JoinHandle
	}
};

///8 is the max number of players for each player to have 6 cards in a 52-card deck
pub const MAX_PLAYERS: u8 = 8;
///4 is the arbitrary min number of players that can play a game.
pub const MIN_PLAYERS: u8 = 4;

#[repr(u8)]
#[derive(Encode, Decode, Debug)]
pub enum Error {
	Indeterminate,
	Authentication,
	Authorization,
	NetworkIO,
	ServerError,
	IncompatibleVersion,
	InvalidProtocol,
	GamesAtCapacity,
	GameAtCapacity,
	GameNotFound,
	ClientNotFound,
	InvalidBid,
	InvalidCard,
	OutOfTurn
}
impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {write!(f, "{}", match self {
		Error::Indeterminate => "Unexpected error - cause unknown",
		Error::Authentication => "Invalid username or password",
		Error::Authorization => "You do not have permission to perform this action",
		Error::NetworkIO => "Network error - try again later",
		Error::ServerError => "Server error - try again later",
		Error::IncompatibleVersion => "Incompatible version - please update the game",
		Error::InvalidProtocol => "Invalid protocol",
		Error::GamesAtCapacity => "Server is at capacity - please try to create a game later",
		Error::GameAtCapacity => "Game has maximum number of players - cannot join",
		Error::GameNotFound => "Game not found - the game code may be incorrect or the game may have ended",
		Error::ClientNotFound => "Client not found",
		Error::InvalidBid => "Invalid bid",
		Error::InvalidCard => "Invalid card - play a legal card",
		Error::OutOfTurn => "It is not your turn"
	})}
}
impl std::error::Error for Error {}

///Spawns an async tasks that terminates if the terminator receives a message
pub fn spawn_async<F>(terminator: watch::Receiver<()>, future: F) -> JoinHandle<()> where F: Future + Send + 'static, F::Output: Send + 'static {
	tokio::spawn(async move {spawn_sync(terminator, future).await;})
}

///If awaited, runs a synchronous task that terminates if the terminator receives a message
pub async fn spawn_sync<F>(mut terminator: watch::Receiver<()>, future: F) where F: Future + Send + 'static, F::Output: Send + 'static {
	tokio::select! {
        _ = future => {},
        _ = terminator.changed() => {}
    }
}