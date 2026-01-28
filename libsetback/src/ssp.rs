use {
	crate::*,
	serde::{Deserialize, Serialize},
	std::fmt::{Display, Formatter},
};

///Setback Server Protocol
#[derive(Deserialize, Serialize, Debug)]
pub struct Ssp {
	/// Must be the same major and minor version to be compatible.
	pub version: Version,
	/// Contains encrypted data to uniquely identify the user and prove their identity.
	pub auth_token: String,
	/// Chaining is how the program knows which responses go to which requests.
	pub chain_id: ChainId,
	/// The actual data being sent.
	pub payload: Payload,
}

impl Ssp {
	pub const fn new(payload: Payload, chain_id: ChainId) -> Self {
		Self {
			version: SETBACK_VERSION,
			auth_token: String::new(),
			chain_id,
			payload,
		}
	}
	pub const fn new_with_token(payload: Payload, chain_id: ChainId, auth_token: String) -> Self {
		Self {
			version: SETBACK_VERSION,
			auth_token,
			chain_id,
			payload,
		}
	}
}

///(major, minor, patch)
#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Copy)]
pub struct Version {
	pub major: u8,
	pub minor: u8,
	pub patch: u8,
}

impl Display for Version {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "Version {}.{}.{}", self.major, self.minor, self.patch)
	}
}

impl Version {
	pub const fn is_compatible(&self) -> bool {
		self.major == SETBACK_VERSION.major && self.minor == SETBACK_VERSION.minor
	}
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Clone, Copy)]
pub enum ChainId {
	/// The message isn't part of a chain
	None,
	/// The message is part of a chain but doesn't expect further messages
	EndOfChain(u32),
	/// The message expects you to respond to this chain
	ExpectResponse(u32),
}

impl ChainId {
	/// Gets the u32 id of the chain.
	/// ## PANICS
	/// If called on [Self::None]
	pub const fn id_unwrap(&self) -> u32 {
		match self {
			Self::None => panic!("Chain id should have an id"),
			Self::EndOfChain(id) => *id,
			Self::ExpectResponse(id) => *id,
		}
	}
}

// Request - client sends message to server.
// Response - server sends message to client in response to a request.
// Spontaneous - server sends message to client without request.
#[derive(Deserialize, Serialize, Debug)]
pub enum Payload {
	/// Client ---Authenticate--> Server<br>
	/// Client <--Connect--- Server<br>
	/// Request only<br><br>
	/// Client must send this immediately after attempting to connect to the server.
	/// Client sends its account id as src_id and sends the auth token it got from logging into the website.
	/// Server responds with Connect which carries the port number the client should reconnect to.
	Authenticate { auth_token: String },
	/// Client ---RefreshAuth--> Server<br>
	/// Client <--RefreshAuth--- Server<br>
	/// Request and response<br><br>
	/// Client sends refresh token to server to get a new auth token.
	/// Server responds with auth token if successful.
	RefreshAuth { token: String },
	/// Client ---Authenticate--> Server<br>
	/// Client <--Connect--- Server<br>
	/// Response only<br><br>
	/// Client must send Authenticate immediately after attempting to connect to the server.
	/// Client sends its account id as src_id and sends the auth token it got from logging into the website.
	/// Server responds with this which carries the port number the client should reconnect to.
	Connect { port: u16 },
	/// Client <--Disconnect--- Server<br>
	/// Spontaneous<br><br>
	/// Informs you when another player disconnects and provides the index of that player.
	Disconnect { player_idx: u8 },
	/// Client ---Any--> Server<br>
	/// Client <--Ack--- Server<br>
	/// Response only<br><br>
	/// If a request doesn't respond with useful data, the server will send back Ack as a signal that everything is ok.
	Ack,
	/// Client ---Any--> Server<br>
	/// Client <--Err--- Server<br>
	/// Response only<br><br>
	/// OR<br><br>
	/// Client <--Err--- Server<br>
	/// Spontaneous<br><br>
	/// Any time the server has an error, it sends this to the client.
	/// The client does not send client-side errors to the server.
	Err(Error),
	/// Client ---CreateGame--> Server<br>
	/// Client <--Game--- Server<br>
	/// Request only<br><br>
	/// Client creates game. An empty string password implies a public game. Anything else is a private game.
	CreateGame { password: String },
	/// Client ---JoinGame--> Server<br>
	/// Client <--Game--- Server<br>
	/// Request only<br><br>
	/// Client requests to join game. An empty string password implies a public game. Anything else is a private game.
	JoinGame {
		code: String,
		password: String,
	},
	/// Client ---CreateGame--> Server<br>
	/// Client <--Game--- Server<br>
	/// Response only<br><br>
	/// OR<br><br>
	/// Client ---JoinGame--> Server<br>
	/// Client <--Game--- Server<br>
	/// Response only<br><br>
	/// Server sends game info when client creates or joins a game.<br><br>
	/// OR<br><br>
	/// Host-Client ---Game--> Server<br>
	/// Host-Client <--Ack--- Server<br>
	/// Other-Clients <--Game--- Server<br>
	/// Host-request and spontaneous<br><br>
	/// Host can update the settings in the lobby which gets sent to the server.
	/// The server updates its state and propagates the message to all other clients.
	Game(GameSetup),
	/// Client ---SearchGames--> Server<br>
	/// Client <--PublicGames--- Server<br>
	/// Request only<br><br>
	/// Client sends a filter used for the server to send back a list of matching PublicGames.
	SearchGames(GameFilter),
	/// Client ---SearchGames--> Server<br>
	/// Client <--PublicGames--- Server<br>
	/// Response only<br><br>
	/// Client requests a list of public games and receives a list of PublicGames.
	PublicGames(Vec<PublicGame>),
	/// Client ---UpdatePlayerTeam--> Server<br>
	/// Client <--Ack--- Server<br>
	/// Other-Clients <--UpdatePlayerTeam--- Server<br>
	/// Request and response<br>
	/// The host or a player can update the team of that player.
	UpdatePlayerTeam { player_idx: u8, team: u8 },
	/// Host-Client ---UpdatePlayerType--> Server<br>
	/// Host-Client <--Ack--- Server<br>
	/// Other-Clients <--UpdatePlayerType--- Server<br>
	/// Request and response<br>
	/// The host can update whether a player is a human or not.
	/// This either results in turning a bot into an empty human slot
	/// or kicking a player
	UpdatePlayerType { player_idx: u8, is_human: bool },
	/// Client ---Ready--> Server<br>
	/// Client <--Ack--- Server<br>
	/// Other-Clients <--Ready--- Server<br>
	/// Request and spontaneous<br><br>
	/// When a client is ready, the server updates the state and propagates it to all other clients.
	/// Index doesn't need to be provided in request, but will be provided in response from server.
	Ready { is_ready: bool, player_idx: u8 },
	/// Host-Client ---AddPlayer--> Server<br>
	/// Host-Client <--Ack--- Server<br>
	/// Other-Clients <--AddPlayer--- Server<br>
	/// Host-request and spontaneous<br><br>
	/// The host can add a player to the game.
	AddPlayer,
	/// Host-Client ---RemovePlayer--> Server<br>
	/// Host-Client <--Ack--- Server<br>
	/// Other-Clients <--RemovePlayer--- Server<br>
	/// Host-request and spontaneous<br><br>
	/// The host can remove a player from the game.
	RemovePlayer { player_idx: u8 },
	/// Host-Client ---SwapPlayers--> Server<br>
	/// Host-Client <--Ack--- Server<br>
	/// Other-Clients <--SwapPlayers--- Server<br>
	/// Host-request and spontaneous<br><br>
	/// The host can swap 2 players.
	SwapPlayers { player_idx_a: u8, player_idx_b: u8 },
	/// Host-Client ---ReinsertPlayer--> Server<br>
	/// Host-Client <--Ack--- Server<br>
	/// Other-Clients <--ReinsertPlayer--- Server<br>
	/// Host-request and spontaneous<br><br>
	/// The host can reinsert a player into a different position, shifting others if necessary.
	ReinsertPlayer { player_idx: u8, new_player_idx: u8 },
	/// Client <--Start--- Server<br>
	/// Spontaneous<br><br>
	/// Server tells clients when countdown ends.
	Start,
	/// Client <--GameEvent--- Server<br>
	/// Client ---GameEvent--> Server<br>
	/// Client <--Ack--- Server<br>
	/// Server-request and client-response<br><br>
	/// The server emits these events to the client. The client usually needs to respond to these.
	GameEvent(GameEvent),
}