use {
	bincode::{Decode, Encode},
	std::{
		fmt::{Debug, Display, Formatter},
		future::Future
	},
	tokio::{
		sync::watch,
		task::JoinHandle
	},
	godot::prelude::*
};

///8 is the max number of players for each player to have 6 cards in a 52-card deck
pub const MAX_PLAYERS: u8 = 8;
///Not sure how many the server can handle, so I'll use a small number for now.
pub const MAX_SERVER_CLIENTS: u32 = 1000;

#[derive(Encode, Decode, PartialEq)]
pub struct Version(u8, u8, u8);
impl Version {
	pub const LATEST: Version = Version(0, 1, 0);
}

#[repr(u8)]
#[derive(Encode, Decode, Clone, Debug, GodotConvert, Var)]
#[godot(via=u8)]
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
#[derive(Encode, Decode, Debug, GodotConvert, Var)]
#[godot(via=u8)]
pub enum ClientError {IO, Serialization, NotConnected, Runtime, EndOfStream, NullPtr, WrongVersion, InvalidUsernameOrPassword, BadRequest}
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