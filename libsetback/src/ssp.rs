use {
	std::fmt::{Display, Formatter},
	bincode::{
		Decode,
		Encode,
		config::{standard, Configuration},
		error::{DecodeError, EncodeError}
	},
	crate::{
		player::*,
		items::*,
		game::*,
		setup_data::*,
		constants::*
	},
	serde::{Deserialize, Serialize}
};

///Setback Server Protocol
#[derive(Encode, Decode, Debug, Clone)]
pub struct Ssp {
	pub version: Version,
	///0 = server/game, other = specific client_id
	pub src_id: u32,
	///the context of the data being sent
	pub protocol: Protocol
}

impl Ssp {
	pub fn new(protocol: Protocol) -> Self {Self {version: SETBACK_VERSION, src_id: 0, protocol}}
	pub fn new_with_src(protocol: Protocol, src_id: u32) -> Self {Self {version: SETBACK_VERSION, src_id, protocol}}

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

///(major, minor, patch)
#[derive(Encode, Decode, PartialEq, Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Version(pub u8, pub u8, pub u8);

impl Display for Version {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "Version {}.{}.{}", self.0, self.1, self.2)
	}
}

//Request - client sends message to server/game.
//Response - server/game sends message to client in response to a request.
//Spontaneous - server/game sends message to client without request.
//Internal - server/game sends message to itself.
///When the client sends a request, they won't wait for a response.
///The client must handle the response the same way it would handle a spontaneous message.
#[derive(Encode, Decode, Debug, Clone)]
pub enum Protocol {
	///Client ---Authenticate--> Server<br>
	///Client <--Connect--- Server<br>
	///Request only<br><br>
	///Client must send this immediately after attempting to connect to the server.
	///Client sends its account id as src_id and sends the auth token it got from logging into the website.
	///Server responds with Connect which carries the port number the client should reconnect to.
	Authenticate(String),
	///Client ---Authenticate--> Server<br>
	///Client <--Connect--- Server<br>
	///Response only<br><br>
	///Client must send Authenticate immediately after attempting to connect to the server.
	///Client sends its account id as src_id and sends the auth token it got from logging into the website.
	///Server responds with this which carries the port number the client should reconnect to.
	Connect(u16),
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
	///Client creates game. A password indicates a private game.
	CreateGame{password: Option<String>},
	///Client ---JoinGame--> Server<br>
	///Client <--Game--- Server<br>
	///Request only<br><br>
	///Client requests to join game. A password indicates a private game.
	JoinGame{code: String, password: Option<String>},
	///Client ---CreateGame--> Server<br>
	///Client <--Game--- Server<br>
	///Response only<br><br>
	///OR<br><br>
	///Client ---JoinGame--> Server<br>
	///Client <--Game--- Server<br>
	///Response only<br><br>
	///Server sends game info when client creates or joins a game.
	Game{setup_data: SetupData, is_host: bool},
	///Client ---SearchGames--> Server<br>
	///Client <--PublicGames--- Server<br>
	///Request only<br><br>
	///Client sends a filter used for the server to send back a list of matching PublicGames.
	SearchGames(GameFilter),
	///Client ---SearchGames--> Server<br>
	///Client <--PublicGames--- Server<br>
	///Response only<br><br>
	///Client requests a list of public games and receives a list of PublicGames.
	PublicGames(Vec<PublicGame>),
	///Host-Client ---GameMeta--> Game<br>
	///All-Clients <--GameMeta--- Game<br>
	///Request and response<br>
	///Spontaneous<br>
	///Host only on request<br><br>
	///Host can update the settings in the lobby which gets sent to the game.
	///The game updates its state and propagates the message to all other clients.
	SetupData(SetupData),
	///Client ---UpdatePlayerTeam--> Game<br>
	///All-Clients <--UpdatePlayerTeam--- Game<br>
	///Request and response<br>
	///The host or a player can update the team of that player.
	UpdatePlayerTeam{index: u8, team: u8},
	///Client ---UpdatePlayerType--> Game<br>
	///All-Clients <--UpdatePlayerType--- Game<br>
	///Request and response<br>
	///The host or a player can update the type of that player.
	UpdatePlayerType{index: u8, human: bool},
	///Host-Client ---AddPlayer--> Game<br>
	///All-Clients <--Players--- Game<br>
	///Response only<br>
	///Spontaneous<br><br>
	///OR<br><br>
	///Client ---RemovePlayer--> Game<br>
	///All-Clients <--Players--- Game<br>
	///Response only<br><br>
	///When information about multiple players changes or the number of players changes.
	///The game updates its state and propagates the message to all other clients.
	Players{players: Vec<Player>, local_player_index: u8},
	///Client ---Ready--> Game<br>
	///All-Clients <--Ready--- Game<br>
	///Request and response<br>
	///Spontaneous<br><br>
	///When a client ready-ups, the game updates the state and propagates it to all other clients.
	Ready{is_ready: bool, index: u8},
	///Client <--Start--- Game<br>
	///Response only<br>
	///Spontaneous<br><br>
	///Game tells clients when countdown ends.
	Start,
	///Client <--GameData--- Game<br>
	///Response only<br>
	///Spontaneous<br><br>
	///Used to sync data between clients.
	GameData(Game),
	///Client <--GameEvent--- Game<br>
	///Spontaneous<br><br>
	///OR<br><br>
	///Client <--GameEvent--- Game<br>
	///Client ---GameEvent--> Game<br>
	///Response and request<br>
	///Spontaneous<br><br>
	///The game emits these events to the client. Sometimes the client will send a message back to the game.
	GameEvent(GameEvent),
	///Host-Client ---AddPlayer--> Game<br>
	///All-Clients <--Players--- Game<br>
	///Request only<br>
	///Host only<br><br>
	///The host can add a player to the game.
	AddPlayer,
	///Host-Client ---RemovePlayer--> Game<br>
	///All-Clients <--Players--- Game<br>
	///Request only<br>
	///Host only<br><br>
	///The host can remove a player from the game.
	RemovePlayer{index: u8}
}

///Admins can use these messages to mess with the server with admin privileges.
#[derive(Encode, Decode, Debug)]
pub enum AdminMessage {
	///Admin ---Authorize--> Server<br>
	///Admin <--AuthorizeResponse--- Server<br>
	///Request only<br><br>
	///Client sends an authorize message to the server with their username, password, account_id, and admin_password.
	Authorize{admin_password: String},
	///Admin ---Any--> Server<br>
	///Admin <--AuthorizeResponse--- Server<br>
	///Authorization error response only<br><br>
	///Server sends an authorize response to the client to indicate if the client now has admin privileges.
	AuthorizeResponse(bool),
	///Admin ---DisableCreateGame--> Server<br>
	///Admin <--DisableCreateGame--- Server<br>
	///Request and response<br><br>
	///Disables the ability to create games.
	DisableCreateGame,
	///Admin ---EnableCreateGame--> Server<br>
	///Admin <--EnableCreateGame--- Server<br>
	///Request and response<br><br>
	///Enables the ability to create games.
	EnableCreateGame,
	///Admin ---NumActiveGames--> Server<br>
	///Admin <--NumActiveGames--- Server<br>
	///Request and response<br><br>
	///Server sends the number of active games to the admin.
	NumActiveGames(u8)
}