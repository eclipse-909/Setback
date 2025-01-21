use {
	super::{
		connection::ServerConnection,
		util::{ServerError, MAX_PLAYERS}
	},
	std::{
		mem,
		array::IntoIter,
		fmt::{Display, Debug, Formatter}
	},
	tokio::sync::mpsc::{Sender, Receiver, channel},
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

pub struct Player {
	pub player_type: PlayerType,
	pub username: String,
	pub hand: Vec<Card>,
	pub bid_tx: Option<Sender<u8>>,
	pub bid_rx: Option<Receiver<u8>>,
	pub card_tx: Option<Sender<Card>>,
	pub card_rx: Option<Receiver<Card>>,
	pub team_id: u8
}
impl Player {
	///Self in singleplayer, or another connected player in multiplayer.
	///username must come from server or be an empty string.
	pub fn new_human(username: String, team_id: u8) -> Self {
		let (bid_tx, bid_rx) = channel::<u8>(1);
		let (card_tx, card_rx) = channel::<Card>(1);
		Self {
			player_type: PlayerType::Human(None),
			username,
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize),
			bid_tx: Some(bid_tx),
			bid_rx: Some(bid_rx),
			card_tx: Some(card_tx),
			card_rx: Some(card_rx),
			team_id
		}
	}
	///Self in multiplayer.
	///account_id and username must come from server.
	pub fn new_self(client: ServerConnection, account_id: u32, username: String, team_id: u8) -> Self {
		let (bid_tx, bid_rx) = channel::<u8>(1);
		let (card_tx, card_rx) = channel::<Card>(1);
		Self {
			player_type: PlayerType::Human(Some(PlayerClient {connection: Some(client), account_id, ready: false})),
			username,
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize),
			bid_tx: Some(bid_tx),
			bid_rx: Some(bid_rx),
			card_tx: Some(card_tx),
			card_rx: Some(card_rx),
			team_id
		}
	}
	pub fn new_ai(team_id: u8) -> Self {
		let (bid_tx, bid_rx) = channel::<u8>(1);
		let (card_tx, card_rx) = channel::<Card>(1);
		Self {
			player_type: PlayerType::AI,
			username: String::from("Bot"),//TODO come up with generic name for AI
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize),
			bid_tx: Some(bid_tx),
			bid_rx: Some(bid_rx),
			card_tx: Some(card_tx),
			card_rx: Some(card_rx),
			team_id
		}
	}
	
	///When decoding a player, that player's player_type, bid_rx, and card_rx will be invalid,
	///so you need to use this function to move the values from the decoded player into this player to update it.
	///# Return
	///Ok(None) success
	///Ok(Some) success, and PlayerType is the old info of the player
	///Err(ServerError) error
	pub fn update_player(&mut self, other: Self, update_client_info: bool) -> Result<Option<PlayerType>, ServerError> {
		self.username = other.username;
		self.hand = other.hand;
		self.team_id = other.team_id;
		match &mut self.player_type {
			PlayerType::Human(Some(this_client)) => {
				if let PlayerType::Human(Some(other_client)) = other.player_type {
					if update_client_info {
						this_client.account_id = other_client.account_id;
						this_client.ready = other_client.ready;
					}
					Ok(None)
				} else {
					Ok(Some(mem::replace(&mut self.player_type, other.player_type)))
				}
			}
			PlayerType::AI => {
				self.player_type = other.player_type;
				Ok(None)
			}
			PlayerType::Human(None) => {unreachable!("You should not be using the network in a singleplayer game")}
		}
	}
	
	pub fn can_follow(&self, lead: Suit) -> bool { self.hand.iter().any(|card| {card.suit == lead}) }
}

impl Encode for Player {
	fn encode<__E: Encoder>(&self, encoder: &mut __E) -> Result<(), EncodeError> {
		match &self.player_type {
			PlayerType::Human(Some(player_client)) => {
				Encode::encode(&0b01u8, encoder)?;
				Encode::encode(&player_client.account_id, encoder)?;
				Encode::encode(&player_client.ready, encoder)?;
			}
			PlayerType::Human(None) => {
				Encode::encode(&0b00u8, encoder)?;
			}
			PlayerType::AI => {
				Encode::encode(&0b10u8, encoder)?;
			}
		}
		Encode::encode(&self.username, encoder)?;
		Encode::encode(&self.hand, encoder)?;
		//Don't encode bid or card senders and receivers
		Encode::encode(&self.team_id, encoder)?;
		Ok(())
	}
}
impl Decode for Player {
	fn decode<__D: Decoder>(decoder: &mut __D) -> Result<Self, DecodeError> {
		let code: u8 = Decode::decode(decoder)?;
		Ok(Self {
			player_type: match code {
				0b01 => {PlayerType::Human(Some(PlayerClient {connection: None, account_id: Decode::decode(decoder)?, ready: Decode::decode(decoder)?}))}
				0b00 => {PlayerType::Human(None)}
				0b10 => {PlayerType::AI}
				_ => {unreachable!()}
			},
			username: Decode::decode(decoder)?,
			hand: Decode::decode(decoder)?,
			bid_tx: None,
			bid_rx: None,
			card_tx: None,
			card_rx: None,
			team_id: Decode::decode(decoder)?,
		})
	}
}
impl<'__de> BorrowDecode<'__de> for Player {
	fn borrow_decode<__D: BorrowDecoder<'__de>>(decoder: &mut __D) -> Result<Self, DecodeError> {
		let code: u8 = BorrowDecode::borrow_decode(decoder)?;
		Ok(Self {
			player_type: match code {
				0b01 => {PlayerType::Human(Some(PlayerClient {connection: None, account_id: BorrowDecode::borrow_decode(decoder)?, ready: BorrowDecode::borrow_decode(decoder)?}))}
				0b00 => {PlayerType::Human(None)}
				0b10 => {PlayerType::AI}
				_ => {unreachable!()}
			},
			username: BorrowDecode::borrow_decode(decoder)?,
			hand: BorrowDecode::borrow_decode(decoder)?,
			bid_tx: None,
			bid_rx: None,
			card_tx: None,
			card_rx: None,
			team_id: BorrowDecode::borrow_decode(decoder)?,
		})
	}
}

#[repr(u8)]
pub enum PlayerType {
	///Human(None) = self in singleplayer or another connected player in multiplayer, Human(Some) = self in mutliplayer.
	///(ServerConnection, account_id, ready)
	Human(Option<PlayerClient>),
	AI
}

pub struct PlayerClient {
	connection: Option<ServerConnection>,
	account_id: u32,
	ready: bool
}

#[derive(Encode, Decode, Clone, PartialEq, Debug)]
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
#[derive(Encode, Decode, Clone, PartialEq, Eq, Hash, Debug)]
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