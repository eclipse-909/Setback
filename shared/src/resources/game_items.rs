use {
	super::{
		connection::{ServerConnection, SetbackConnection},
		util::{ServerError, ClientError, MAX_PLAYERS},
		ssp::{Protocol, Ssp}
	},
	std::{
		mem,
		array::IntoIter,
		fmt::{Display, Debug, Formatter},
		io::Error
	},
	tokio::sync::mpsc::{Sender, Receiver, channel, UnboundedSender},
	bincode::{
		Decode, Encode, BorrowDecode,
		enc::Encoder,
		de::{Decoder, BorrowDecoder},
		error::{EncodeError, DecodeError}
	}
};

pub const J: u8 = 11;
pub const Q: u8 = 12;
pub const K: u8 = 13;
pub const A: u8 = 14;
pub const CARDS_PER_PLAYER: u8 = 6;
pub const MAX_GAME_METAS: u8 = 253;

#[derive(Encode, Decode)]
pub struct PlayerData {
	pub player_type: PlayerType,
	pub username: String,
	pub hand: Vec<Card>,
	pub team_id: u8
}
impl PlayerData {
	fn new(player_type: PlayerType, username: String, hand: Vec<Card>, team_id: u8) -> Self {
		Self {player_type, username, hand, team_id}
	}
}

pub struct Player {
	pub player_data: PlayerData,
	pub bid_tx: Sender<u8>,
	pub bid_rx: Receiver<u8>,
	pub card_tx: Sender<Card>,
	pub card_rx: Receiver<Card>
}
impl Player {
	///Self in singleplayer, or another connected player in multiplayer.
	///username must come from server or be an empty string.
	pub fn new_human(username: String, team_id: u8) -> Self {
		let (bid_tx, bid_rx) = channel::<u8>(1);
		let (card_tx, card_rx) = channel::<Card>(1);
		Self {
			player_data: PlayerData::new(PlayerType::Human(None), username, Vec::with_capacity(CARDS_PER_PLAYER as usize), team_id),
			bid_tx,
			bid_rx,
			card_tx,
			card_rx
		}
	}

	///Self in multiplayer.
	///account_id and username must come from server.
	pub async fn new_self(ssp_callback: fn(message: Result<Ssp, ClientError>), team_id: u8, username: &str, password: &str) -> Option<Self> {
		let (bid_tx, bid_rx) = channel::<u8>(1);
		let (card_tx, card_rx) = channel::<Card>(1);
		let (server_connection, account_id) = ServerConnection::new_connection(username, password, ssp_callback).await?;
		Some(Self {
			player_data: PlayerData::new(PlayerType::Human(Some(Connection {server_connection: Some(server_connection), account_id, ready: false})), String::from(username), Vec::with_capacity(CARDS_PER_PLAYER as usize), team_id),
			bid_tx,
			bid_rx,
			card_tx,
			card_rx
		})
	}

	pub fn new_ai(team_id: u8) -> Self {
		let (bid_tx, bid_rx) = channel::<u8>(1);
		let (card_tx, card_rx) = channel::<Card>(1);
		Self {
			player_data: PlayerData::new(PlayerType::AI, String::from("Bot"), Vec::with_capacity(CARDS_PER_PLAYER as usize), team_id),
			bid_tx,
			bid_rx,
			card_tx,
			card_rx
		}
	}
	
	///When decoding a player, that player's player_type, bid_rx, and card_rx will be invalid,
	///so you need to use this function to move the values from the decoded player into this player to update it.
	///# Return
	///Ok(None) success
	///Ok(Some) success, and PlayerType is the old info of the player
	///Err(ServerError) error
	pub fn update_player(&mut self, data: PlayerData, update_client_info: bool) -> Result<Option<PlayerType>, ServerError> {
		self.player_data.username = data.username;
		self.player_data.hand = data.hand;
		self.player_data.team_id = data.team_id;
		match &mut self.player_data.player_type {
			PlayerType::Human(Some(this_client)) => {
				if let PlayerType::Human(Some(other_client)) = data.player_type {
					if update_client_info {
						this_client.account_id = other_client.account_id;
						this_client.ready = other_client.ready;
					}
					Ok(None)
				} else {
					Ok(Some(mem::replace(&mut self.player_data.player_type, data.player_type)))
				}
			}
			PlayerType::AI => {
				self.player_data.player_type = data.player_type;
				Ok(None)
			}
			PlayerType::Human(None) => {unreachable!("You should not be using the network in a singleplayer game")}
		}
	}
	
	pub fn can_follow(&self, lead: Suit) -> bool { self.player_data.hand.iter().any(|card| {card.suit == lead}) }
}

#[repr(u8)]
#[derive(Encode, Decode)]
pub enum PlayerType {
	///Human(None) = self in singleplayer or another connected player in multiplayer, Human(Some) = self in mutliplayer.
	///(ServerConnection, account_id, ready)
	Human(Option<Connection>),
	AI
}

pub struct Connection {
	server_connection: Option<ServerConnection>,
	account_id: u32,
	ready: bool
}

impl Connection {
	pub async fn send_client_error(&mut self, err: ClientError) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::ClientErr(err), self.account_id)).await
	}
	
	pub async fn send_create_game(&mut self) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::CreateGame, self.account_id)).await
	}
	
	pub async fn send_join_game(&mut self, code: u32) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::JoinGame(code), self.account_id)).await
	}
	
	pub async fn send_search_games(&mut self, filter: GameFilter) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::SearchGames(filter), self.account_id)).await
	}
	
	///Host-only
	pub async fn send_game_meta(&mut self, meta_data: GameMeta) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::GameMeta(meta_data), self.account_id)).await
	}
	
	///Host-only
	pub async fn send_player(&mut self, player_data: PlayerData, index: u8) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::Player(player_data, index), self.account_id)).await
	}
	
	///Host-only
	pub async fn send_players(&mut self, players: Vec<PlayerData>) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::Players(players), self.account_id)).await
	}
	
	pub async fn send_ready(&mut self, is_ready: bool) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::Ready(is_ready), self.account_id)).await
	}
	
	pub async fn send_bid(&mut self, amount: u8) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::Bid(amount), self.account_id)).await
	}
	
	pub async fn send_play_card(&mut self, card: Card) -> Result<(), Error> {
		self.server_connection.as_mut().unwrap().send(&Ssp::new_with_src(Protocol::PlayCard(card), self.account_id)).await
	}
}

impl Encode for Connection {
	fn encode<__E: Encoder>(&self, encoder: &mut __E) -> Result<(), EncodeError> {
		Encode::encode(&self.account_id, encoder)?;
		Encode::encode(&self.ready, encoder)?;
		Ok(())
	}
}
impl Decode for Connection {
	fn decode<__D: Decoder>(decoder: &mut __D) -> Result<Self, DecodeError> {
		Ok(Self {
			server_connection: None,
			account_id: Decode::decode(decoder)?,
			ready: Decode::decode(decoder)?
		})
	}
}
impl<'__de> BorrowDecode<'__de> for Connection {
	fn borrow_decode<__D: BorrowDecoder<'__de>>(decoder: &mut __D) -> Result<Self, DecodeError> {
		Ok(Self {
			server_connection: None,
			account_id: BorrowDecode::borrow_decode(decoder)?,
			ready: BorrowDecode::borrow_decode(decoder)?,
		})
	}
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Debug)]
pub struct Card {pub suit: Suit, pub number: u8}
impl Display for Card {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} of {:?}", match self.number {
			J => {"Jack".to_string()}
			Q => {"Queen".to_string()}
			K => {"King".to_string()}
			A => {"Ace".to_string()}
			n => {n.to_string()}
		}, self.suit)
	}
}

#[repr(u8)]
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Suit {Hearts, Clubs, Diamonds, Spades}
impl Suit {
	pub fn iter() -> IntoIter<Self, 4> {[Self::Hearts, Self::Clubs, Self::Diamonds, Self::Spades].into_iter()}
}

#[derive(Encode, Decode, Clone)]
pub struct GameMeta {
	pub std_team_mode: bool,
	pub max_score: i8,
	pub win_by_2: bool,
	pub negative_scores: bool
}
impl Default for GameMeta {
	fn default() -> Self {Self {std_team_mode: true, max_score: 21, win_by_2: false, negative_scores: true}}
}

#[derive(Encode, Decode)]
pub struct GameFilter {
	pub std_team_mode: bool,
	pub win_by_2: bool,
	pub negative_scores: bool,
	pub dealer_override: bool,
	pub max_score_cmp: Comparison,
	pub max_score_value: i8,
	pub max_players_cmp: Comparison,
	pub max_players_value: u8
}

#[repr(u8)]
#[derive(Encode, Decode, Debug, Clone)]
pub enum Comparison {Equal, LessOrEqual, GreaterOrEqual, Less, Greater, NotEqual}
impl Comparison {
	pub fn compare<T: PartialOrd>(&self, a: T, b: T) -> bool {
		match self {
			Comparison::Equal => {a == b}
			Comparison::LessOrEqual => {a <= b}
			Comparison::GreaterOrEqual => {a >= b}
			Comparison::Less => {a < b}
			Comparison::Greater => {a > b}
			Comparison::NotEqual => {a != b}
		}
	}
}

#[derive(Encode, Decode, Clone)]
pub struct Team {
	//the cards the team collects after winning a turn
	pub pile: Vec<Card>,
	//score of the game
	pub score: i8
}
impl Default for Team {
	fn default() -> Self {Self {pile: Vec::with_capacity((MAX_PLAYERS * CARDS_PER_PLAYER) as usize), score: 0}}
}

#[derive(Encode, Decode, Clone)]
pub struct GameData {
	pub teams: Vec<Team>,
	pub trump: Suit,
	pub lead: Suit,
	///the index of the player whose turn it is
	pub turn_index: u8,
	///the turn number in the round (0..=6 where turn 0 is the bidding turn)
	pub turn_number: u8,
	///the index of the player who led this turn
	pub lead_index: u8,
	pub dealer_index: u8,
	pub round_number: u8
}

impl Default for GameData {
	fn default() -> Self {
		Self {
			teams: Vec::with_capacity(MAX_PLAYERS as usize),
			trump: Suit::Hearts,
			lead: Suit::Hearts,
			turn_index: 0,
			turn_number: 0,
			lead_index: 0,
			dealer_index: 0,
			round_number: 0
		}
	}
}

#[derive(Encode, Decode, Debug)]
pub struct RoundScores {
	pub high_team_id: u8,
	pub low_team_id: u8,
	pub jack_team_id: Option<u8>,
	pub game_team_id: Option<u8>,
	pub team_game_scores: Vec<u8>,
	pub team_scores: Vec<i8>
}

///Events that can be sent from the game to the player.
///The game will emit these events in singleplayer.
///In multiplayer, the TCP stream will receive [Ssp] messages from the server, which may contain these events.
#[derive(Encode, Decode, Debug)]
#[repr(u8)]
pub enum GameEvent {
	///Index player bid the amount
	PlayerBid{index: u8, amount: u8},
	///Index player played the card
	PlayerPlayedCard{index: u8, card: Card},
	///It is now index player's turn. This should be sent to every player except the one whose turn it is.
	PlayerTurn{index: u8},
	///It is now index player's turn to bid. This should only be sent to the player whose turn it is.
	PromptBid{index: u8},
	///It is now index player's turn to play a card. This should only be sent to the player whose turn it is.
	PromptCard{index: u8},
	///The round is over, and the scores of the round are given for each team
	RoundScores(RoundScores),
	///The game is over, and the scores of the round are given for each team
	GameOver(RoundScores),
	///The game has encountered an error
	GameError(ServerError)
}