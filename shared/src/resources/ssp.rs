use {
	super::{
		util::Error,
		game_items::{PlayerData, GameFilter, GameMeta, GameData, GameEvent}
	},
	bincode::{
		Decode,
		Encode,
		error::{EncodeError, DecodeError},
		config::{Configuration, standard}
	}
};

///Setback Server Protocol
#[derive(Encode, Decode)]
pub struct Ssp {
	pub version: Version,
	///0 = server/game, other = specific client_id
	pub src_id: u32,
	///the context of the data being sent (see enum Protocol)
	pub protocol: Protocol
	//TODO authentication token/key?
}

impl Ssp {
	pub fn new(protocol: Protocol) -> Self {Self {version: Version::LATEST, src_id: 0, protocol}}
	pub fn new_with_src(protocol: Protocol, src_id: u32) -> Self {Self {version: Version::LATEST, src_id, protocol}}
	
	//Server/Game -> Client protocols are implemented where needed and don't require separate functions.
	//You can find functions for each Client -> Server/Game protocol in the game code
	
	//Encodes the SSP into a Vec<u8>, which is prefixed with 2 bytes representing the length of THE REST of the vector in little-endian.
	pub fn encode(&self) -> Result<Vec<u8>, EncodeError> {
		let mut bytes: Vec<u8> = bincode::encode_to_vec(self, standard())?;
		//prefix with 2-byte length
		let mut message: Vec<u8> = (bytes.len() as u16).to_le_bytes().to_vec();
		message.append(&mut bytes);
		Ok(message)
	}
	//Decodes the SSP from a &mut [u8], which is NOT prefixed with the 2-byte length.
	pub fn decode(buffer: &mut [u8]) -> Result<Ssp, DecodeError> {
		bincode::decode_from_slice::<Ssp, Configuration>(buffer, standard()).map(|(ssp, _)| {ssp})
	}
}

///Version for SSP, not the game
#[derive(Encode, Decode, PartialEq)]
pub struct Version(u8, u8, u8);
impl Version {
	pub const LATEST: Version = Version(0, 1, 0);
}

///Request - client sends message to server/game.
///Response - server/game sends message to client.
///Spontaneous - server/game sends message to client without request.
///Internal - server/game sends message to itself.
///When a request is sent, the caller will not wait for a response.
///You must handle the response the same way you would handle a spontaneous message.
#[repr(u8)]
#[derive(Encode, Decode)]
pub enum Protocol {
	///Client ---SyncID--> Server<br>
	///Client <--SyncID--- Server<br>
	///Request and response<br><br>
	///Client sends its account_id to the server when client connects to server.
	///There is a small window between the client connecting and the client sending a SyncID message.
	///The client must send this to authenticate before attempting to send any other messages.
	///The server will disconnect the client if it doesn't send this message immediately after connecting.
	///The server will send a SyncID message to the client in response once the client is authenticated.
	SyncID,
	///Server ---Disconnect--> Self/Game<br>
	///Internal<br><br>
	///The server detects when the client disconnects and sends this to the owner server or game
	///so the client can be removed from its owning collection. Client never interacts with this protocol.
	Disconnect,
	///Client ---Any--> Server/Game<br>
	///Client <--Err--- Server/Game<br>
	///Response only<br><br>
	///OR<br><br>
	///Client <--Err--- Server/Game<br>
	///Spontaneous<br><br>
	///Any time the server has an error, it sends this to the client.
	///The client does not send client-side errors to the server.
	Err(Error),
	///Client ---CreateGame--> Server<br>
	///Client <--Game--- Server<br>
	///Request only<br><br>
	///Client creates game.
	CreateGame{public: bool},
	///Client ---JoinGame--> Server<br>
	///Client <--Game--- Server<br>
	///Request only<br><br>
	///Client requests to join game.
	JoinGame{code: u32},
	///Client ---CreateGame--> Server<br>
	///Client <--Game--- Server<br>
	///Response only<br><br>
	///OR<br><br>
	///Client ---JoinGame--> Server<br>
	///Client <--Game--- Server<br>
	///Response only<br><br>
	///Server sends game info when client creates or joins a game.
	Game(GameMeta, Vec<PlayerData>),
	///Client ---SearchGames--> Server<br>
	///Client <--GameMetas--- Server<br>
	///Request only<br><br>
	///Client sends a filter used for the server to send back a list of matching GameMetas.
	SearchGames(GameFilter),
	///Client ---SearchGames--> Server<br>
	///Client <--GameMetas--- Server<br>
	///Response only<br><br>
	///Client requests a list of public games and receives a list of GameMetas.
	GameMetas(Vec<GameMeta>),
	///Host-Client ---GameMeta--> Game<br>
	///All-Clients <--GameMeta--- Game<br>
	///Request and response<br>
	///Spontaneous<br><br>
	///Host can update the settings in the lobby which gets sent to the game.
	///The game updates its state and propagates the message to all other clients.
	GameMeta(GameMeta),
	///Client ---Player--> Game<br>
	///All-Clients <--Player--- Game<br>
	///Request and response<br>
	///Spontaneous<br><br>
	///The host or a player can update the player data of that player.
	///The game updates its state and propagates the message to all other clients.
	Player{data: PlayerData, index: u8},
	///Client ---Players--> Game<br>
	///All-Clients <--Players--- Game<br>
	///Request and response<br>
	///Spontaneous<br><br>
	///When information about multiple players changes or the number of players changes.
	///The game updates its state and propagates the message to all other clients.
	Players(Vec<PlayerData>),
	///Client ---Ready--> Game<br>
	///All-Clients <--Ready--- Game<br>
	///Request and response<br>
	///Spontaneous<br><br>
	///When a client ready-ups, the game updates the state and propagates it to all other clients.
	Ready{is_ready: bool},
	///Client <--Start--- Game<br>
	///Spontaneous<br><br>
	///Game tells clients when countdown ends.
	Start,
	///Client <--GameData--- Game<br>
	///Spontaneous<br><br>
	///Used to sync data between clients.
	GameData(GameData),
	///Client <--GameEvent--- Game<br>
	///Spontaneous<br><br>
	///OR<br><br>
	///Client <--GameEvent--- Game<br>
	///Client ---GameEvent--> Game<br>
	///Response and request<br>
	///Spontaneous<br><br>
	///The game emits these events to the client. Sometimes the client will send a message back to the game.
	GameEvent(GameEvent)
}