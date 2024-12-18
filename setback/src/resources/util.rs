use {
	bincode::{Decode, Encode},
	std::{
		fmt::{Debug, Display, Formatter},
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
#[derive(Encode, Decode, Clone, Debug)]
pub enum ServerError {
	Indeterminate,
	Deserialization,
	ListAtCapacity,
	GamesAtCapacity,
	InvalidProtocol,
	ClientNotFound,
	ProtocolCannotRelay,
	GameNotFound,
	GameAtCapacity,
	TcpSendFailed,
	HostOnlyProtocol,
	CouldNotCreatePlayer,
	CouldNotCreateGame,
	InvalidBid,
	InvalidCard,
	OutOfTurn
}
impl Display for ServerError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {write!(f, "{:?}", self)}
}
impl std::error::Error for ServerError {}

#[repr(u8)]
#[derive(Encode, Decode, Debug)]
pub enum ClientError {
	IO,
	Serialization,
	NotConnected,
	Runtime,
	EndOfStream,
	NullPtr,
	WrongVersion,
	InvalidUsernameOrPassword,
	BadRequest
}
impl Display for ClientError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {write!(f, "{:?}", self)}
}
impl std::error::Error for ClientError {}

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