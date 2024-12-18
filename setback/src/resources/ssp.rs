use {
	super::{
		util::{ServerError, ClientError},
		game_items::{Card, Player, GameFilter, GameMeta, GameData, RoundScores}
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
		bincode::decode_from_slice::<Ssp, Configuration>(&buffer, standard()).map(|(ssp, _)| {ssp})
	}
}

///Version for SSP, not the game
#[derive(Encode, Decode, PartialEq)]
pub struct Version(u8, u8, u8);
impl Version {
	pub const LATEST: Version = Version(0, 1, 0);
}

#[repr(u8)]
#[derive(Encode, Decode)]
pub enum Protocol {
	///Client <-> Server<br><br>Client sends its account_id to the server when client connects to server.
	///There is a small window between the client connecting and the client sending a SyncID message.
	///If it sends, the server sends one back to verify a successful connection.
	SyncID,
	///Server -> Self/Game<br><br>The server detects when the client disconnects and sends this to the owner server or bevy_game
	///so the client can be removed from its owning collection. Client never interacts with this protocol.
	Disconnect,
	///Server/Game -> Client<br><br>Any time the server has an error. Used as response only.
	ServerErr(ServerError),
	///Client -> Self<br><br>Any time the client has an error. Not sent over internet. Used to notify client of problem and disconnect.
	ClientErr(ClientError),
	///Client -> Server<br><br>Client creates bevy_game. Responds with Game if successful or Error otherwise.
	CreateGame,
	///Client -> Server<br><br>Client joins bevy_game. Responds with Game if successful or Error otherwise.
	JoinGame(u32),
	///Client -> Server<br><br>Client sends a filter used for the server to send back a list of matching GameMetas.
	///Responds with GameMetas if successful or Error otherwise.
	SearchGames(GameFilter),
	///Server -> Client<br><br>Server sends bevy_game info when client joins.
	///Used as response only.
	Game(GameMeta, Vec<Player>),
	///Host-Client -> Game -> Client<br><br>Host can update the settings in the lobby which gets sent to the bevy_game.
	///The bevy_game updates its state and propagates the message to all other clients. No response. May receive spontaneously.
	GameMeta(GameMeta),
	///Server -> Client<br><br>Server sends a list of games. Max list length is 253. Used as response only.
	GameMetas(Vec<GameMeta>),
	///Host-Client -> Game -> Client<br><br>When the host changes information about a particular player.
	///This sends the updated player info and its index. No response. May receive spontaneously.
	Player(Player, u8),
	///Host-Client -> Game -> Client<br><br>When information about multiple players changes or the number of players changes.
	///No response. May receive spontaneously.
	Players(Vec<Player>),
	///Client <-> Game<br><br>The clients need to ready-up. The bevy_game updates the state and propagates it to all other clients.
	///No response. May receive spontaneously.
	Ready(bool),
	///Client <-> Game<br><br>The clients bid before the rounds. The bevy_game updates the state and propagates it to all other clients.
	///No response. May receive spontaneously.
	Bid(u8),
	///Client <-> Game<br><br>The client plays a card on their turn. The bevy_game updates the state and propagates it to all other clients.
	///No response. May receive spontaneously.
	PlayCard(Card),
	///Game -> Client<br><br>Game tells clients when countdown ends. No response.
	Start,
	///Game -> Client<br><br>Used to sync data between clients. No response. May receive spontaneously.
	GameData(GameData),
	///Game -> Client<br><br>Lets the clients know when cards are being dealt. No response.
	///Receive when cards are dealt at the start of the round, or when nobody bids and a re-deal is required.
	DealCards,
	///Game -> Client<br><br>Tells clients which teams won which points in the round. No response. Receive every time a round ends.
	RoundScores(RoundScores),
	///Game -> Client<br><br>Tells clients when the bevy_game is over. No response. Receive when the bevy_game is over and terminated.
	GameOver
}

#[cfg(test)]
mod tests {
	use {
		super::*,
		super::super::{
			util::MAX_PLAYERS,
			game_items::{Suit, MAX_GAME_METAS, CARDS_PER_PLAYER}
		},
		bincode::config::standard
	};
	
	#[test]
	fn test_ssp_sizes() {
		const MAX_SIZE: usize = 1022;
		
		let mut players: Vec<Player> = Vec::with_capacity(MAX_PLAYERS as usize);
		while players.len() < MAX_PLAYERS as usize {
			let mut player: Player = Player::new_ai(0);
			while player.hand.len() < CARDS_PER_PLAYER as usize {
				player.hand.push(Card {suit: Suit::Hearts, number: 0});
			}
			players.push(player);
		}
		let ssp: Ssp = Ssp::new(Protocol::Game(GameMeta::default(), players));
		let bytes: Vec<u8> = bincode::encode_to_vec(ssp, standard()).unwrap();
		assert!(bytes.len() <= MAX_SIZE);
		
		let mut game_metas: Vec<GameMeta> = Vec::with_capacity(MAX_GAME_METAS as usize);
		while game_metas.len() < MAX_GAME_METAS as usize {
			game_metas.push(GameMeta::default());
		}
		let ssp: Ssp = Ssp::new(Protocol::GameMetas(game_metas));
		let bytes: Vec<u8> = bincode::encode_to_vec(ssp, standard()).unwrap();
		assert!(bytes.len() <= MAX_SIZE);
	}
}