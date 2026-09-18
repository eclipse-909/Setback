use {
	crate::*,
	serde::{Deserialize, Serialize},
};

///Setback Server Protocol
#[derive(Deserialize, Serialize, Debug)]
pub struct Ssp {
	/// The account id or game code that produced this message.
	/// Only the server uses this internally.
	#[serde(skip)]
	pub src_id: u32,
	/// Correlation is how the program knows which responses go to which requests
	/// and whether it needs to respond or expect a response.
	pub correlation_id: CorrelationId,
	/// The actual data being sent.
	pub payload: Payload,
}

impl Ssp {
	pub fn new(payload: impl Into<Payload>, correlation_id: CorrelationId) -> Self {
		Self {
			src_id: 0,
			correlation_id,
			payload: payload.into(),
		}
	}

	pub fn request(payload: impl Into<Payload>, correlation_id: u32) -> Self {
		Self::new(payload, CorrelationId::request(correlation_id))
	}
	pub fn response(payload: impl Into<Payload>, correlation_id: u32) -> Self {
		Self::new(payload, CorrelationId::response(correlation_id))
	}
	pub fn event(payload: impl Into<Payload>) -> Self {
		Self::new(payload, CorrelationId::event())
	}

	pub fn encode<E: Extend<u8>>(&self, e: E) -> Result<E, NetError> {
		postcard::to_extend(self, e).map_err(|_| NetError::BadRequest)
	}

	pub fn decode(bytes: &[u8]) -> Result<Self, NetError> {
		postcard::from_bytes(bytes).map_err(|_| NetError::BadRequest)
	}
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
pub struct CorrelationId {
	pub id: u32,
	pub end: bool,
}

impl CorrelationId {
	pub const fn request(id: u32) -> Self {
		Self { id, end: false }
	}
	pub const fn response(id: u32) -> Self {
		Self { id, end: true }
	}
	pub const fn event() -> Self {
		Self { id: 0, end: false }
	}
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Payload {
	/// Used to respond to any request to indicate the message was received successfully.
	Ack,
	/// Client ---Any--> Server
	///
	/// Client <--Err--- Server
	///
	/// Response only
	///
	/// OR
	///
	/// Client <--Err--- Server
	///
	/// Spontaneous
	///
	/// Any time the server has an error, it sends this to the client.
	/// The client does not send client-side errors to the server.
	Err(Error),
	/// Client ---Any--> Server
	///
	/// Client <--ErrRetry--- Server
	///
	/// Response only
	///
	/// Any time there is an error and the original payload needs to be resent.
	ErrRetry(Error, Box<Self>),
	/// Client ---Authenticate--> Server
	///
	/// Client <--Ack--- Server
	///
	/// Request only
	///
	/// Handshake only
	///
	/// Client must send this immediately after attempting to connect to the server.
	/// Client sends its account id as src_id and sends the auth token it got from logging into the API.
	/// Server responds with Ack if authentication was successful.
	/// Server responds with Err if authentication was unsuccessful.
	Authenticate { version: Version, auth_token: String },
	Server(ServerPayload),
	Lobby(LobbyPayload),
	Game(GamePayload),
}

impl Payload {
	pub fn try_server(self) -> Result<ServerPayload, NetError> {
		match self {
			Self::Server(p) => Ok(p),
			#[cfg(not(feature = "server"))]
			_ => Err(NetError::ServerError),
			#[cfg(feature = "server")]
			_ => Err(NetError::BadRequest),
		}
	}
	
	pub fn try_lobby(self) -> Result<LobbyPayload, NetError> {
		match self {
			Self::Lobby(p) => Ok(p),
			#[cfg(not(feature = "server"))]
			_ => Err(NetError::ServerError),
			#[cfg(feature = "server")]
			_ => Err(NetError::BadRequest),
		}
	}
	
	pub fn try_game(self) -> Result<GamePayload, NetError> {
		match self {
			Self::Game(p) => Ok(p),
			#[cfg(not(feature = "server"))]
			_ => Err(NetError::ServerError),
			#[cfg(feature = "server")]
			_ => Err(NetError::BadRequest),
		}
	}
}

impl From<ServerPayload> for Payload {
	fn from(value: ServerPayload) -> Self {
		Self::Server(value)
	}
}

impl From<LobbyPayload> for Payload {
	fn from(value: LobbyPayload) -> Self {
		Self::Lobby(value)
	}
}

impl From<GamePayload> for Payload {
	fn from(value: GamePayload) -> Self {
		Self::Game(value)
	}
}

// Documentation Key:
// Request - client sends message to server and gets a response.
// Response - server sends message to client in response to a request.
// Spontaneous - server sends message to client and client may or may not need to respond.

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ServerPayload {
	/// Client ---CreateGame--> Server
	///
	/// Client <--Lobby--- Server
	///
	/// Request only
	///
	/// Client creates game. An empty string password creates a public game. Anything else is a private game.
	CreateGame { password: String },
	/// Client ---JoinGame--> Server
	///
	/// Client <--Lobby--- Server
	///
	/// Request only
	///
	/// Client requests to join game.
	JoinGame { code: String, password: String },
	/// Client ---RejoinGame--> Server
	///
	/// Client <--Snapshot--- Server
	///
	/// Request only
	///
	/// Client requests to rejoin game.
	/// This is only used if the client has disconnected from an ongoing game and wants to rejoin.
	RejoinGame { code: String, password: String },
	/// Client ---SearchGames--> Server
	///
	/// Client <--PublicGames--- Server
	///
	/// Request only
	///
	/// Client sends a filter used for the server to send back a list of matching PublicGames.
	SearchGames(GameFilter),
	/// Client ---SearchGames--> Server
	///
	/// Client <--PublicGames--- Server
	///
	/// Response only
	///
	/// Client requests a list of public games and receives a list of PublicGames.
	PublicGames(Vec<PublicGame>),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum LobbyPayload {
	/// Client ---CreateGame--> Server
	///
	/// Client <--Lobby--- Server
	///
	/// OR
	///
	/// Client ---JoinGame--> Server
	///
	/// Client <--Lobby--- Server
	///
	/// Response only
	///
	/// Server sends game info when client creates or joins a game.
	Lobby {lobby: Box<LobbyState>, player_idx: u8},
	/// Host-Client ---LobbyRules--> Server
	///
	/// Host-Client <--Ack--- Server
	///
	/// Other-Clients <--LobbyRules--- Server
	///
	/// Host-request and spontaneous
	///
	/// Host can update the settings in the lobby which gets sent to the server.
	/// The server updates its state and propagates the message to all other clients.
	LobbyRules {rules: LobbyRules, player_idx: u8},
	/// Host-Client ---AddPlayer--> Server
	///
	/// Host-Client <--Ack--- Server
	///
	/// Other-Clients <--AddPlayer--- Server
	///
	/// Host-request and spontaneous
	///
	/// The host can add a player to the game.
	AddPlayer,
	/// Host-Client ---RemovePlayer--> Server
	///
	/// Host-Client <--Ack--- Server
	///
	/// Other-Clients <--RemovePlayer--- Server
	///
	/// Host-request and spontaneous
	///
	/// The host can remove a player from the game.
	RemovePlayer { player_idx: u8 },
	/// Host-Client ---SwapPlayers--> Server
	///
	/// Host-Client <--Ack--- Server
	///
	/// Other-Clients <--SwapPlayers--- Server
	///
	/// Host-request and spontaneous
	///
	/// The host can swap 2 players.
	SwapPlayers { player_idx_a: u8, player_idx_b: u8 },
	/// Host-Client ---ReinsertPlayer--> Server
	///
	/// Host-Client <--Ack--- Server
	///
	/// Other-Clients <--ReinsertPlayer--- Server
	///
	/// Host-request and spontaneous
	///
	/// The host can reinsert a player into a different position, shifting others if necessary.
	ReinsertPlayer { player_idx: u8, new_player_idx: u8 },
	/// Client ---UpdatePlayerTeam--> Server
	///
	/// Client <--Ack--- Server
	///
	/// Other-Clients <--UpdatePlayerTeam--- Server
	///
	/// Request and response
	///
	/// The host or a player can update the team of that player.
	UpdatePlayerTeam { player_idx: u8, team: u8 },
	/// Host-Client ---UpdatePlayerType--> Server
	///
	/// Host-Client <--Ack--- Server
	///
	/// Other-Clients <--UpdatePlayerType--- Server
	///
	/// Request and response
	///
	/// The host can update whether a player is a human or not.
	///
	/// When in the lobby this either results in turning a bot into an empty human slot
	/// or kicking a player.
	///
	/// When in the game, this is either due to a player disconnecting or rejoining.
	UpdatePlayerType { player_idx: u8, is_human: bool },
	/// Client <--PlayerJoined--- Server
	///
	/// Spontaneous
	///
	/// Informs you when another player joins and provides the index of that player.
	PlayerJoined { player_idx: u8, username: String },
	/// Client <--PlayerDisconnected--- Server
	///
	/// Spontaneous
	///
	/// Informs you when another player disconnects and provides the index of that player.
	/// The server internally uses NetError::Disconnected to track when players disconnect.
	PlayerDisconnected { player_idx: u8 },
	/// Client ---Ready--> Server
	///
	/// Client <--Ack--- Server
	///
	/// Other-Clients <--Ready--- Server
	///
	/// Request and spontaneous
	///
	/// When a client is ready, the server updates the state and propagates it to all other clients.
	/// Index doesn't need to be provided in request, but will be provided in response from server.
	/// The countdown starts when all players are ready.
	Ready { is_ready: bool, player_idx: u8 },
	/// Client <--Start--- Server
	///
	/// Spontaneous
	///
	/// Server tells clients when countdown ends.
	Start,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum GamePayload {
	/// Client <--PlayerDisconnected--- Server
	///
	/// Spontaneous
	///
	/// Informs you when another player disconnects and provides the index of that player.
	/// The server internally uses NetError::Disconnected to track when players disconnect.
	PlayerDisconnected { player_idx: u8 },
	/// Client <--PlayerRejoined--- Server
	///
	/// Spontaneous
	///
	/// Informs you when another player reconnects and provides the index and username of that player.
	PlayerRejoined { player_idx: u8, username: String },
	/// Client ---RejoinGame--> Server
	///
	/// Client <--Snapshot--- Server
	///
	/// Response only
	///
	/// Client requests to rejoin game.
	/// This is only used if the client has disconnected from an ongoing game and wants to rejoin.
	Snapshot {game_state: Box<GameState>, player_idx: u8},
	/// Client <--GameEvent--- Server
	///
	/// Client ---GameEvent--> Server
	///
	/// Client <--Ack--- Server
	///
	/// Server-request and client-response
	///
	/// The server emits these events to the client. The client sometimes needs to respond to these.
	/// If the client responds to a prompt with invalid data, the prompt will be resent either until
	/// the client provides valid data or there is an error.
	GameEvent(GameEvent),
	/// Client ---ReportPlayer--> Server
	///
	/// Client <--Ack--- Server
	///
	/// Request only
	///
	/// The player may report a human player in multiplayer.
	/// This is done in game rather than through the public API to prevent
	/// players from spam reporting other players.
	ReportPlayer(ReportPlayerRequest),
	/// Client ---LeaveGame--> Server
	///
	/// Client <--Ack--- Server
	///
	/// Request only
	///
	/// A client is intentionally leaving the game meaning they will not be allowed to rejoin.
	LeaveGame,
}