use {
	crate::{
		client::Client,
		util::{ServerError, MAX_PLAYERS}
	},
	std::{
		sync::Arc,
		mem,
		array::IntoIter,
		fmt::{Display, Debug, Formatter}
	},
	tokio::sync::RwLock,
	bincode::{
		Decode, Encode, BorrowDecode,
		enc::Encoder,
		de::{Decoder, BorrowDecoder},
		error::{EncodeError, DecodeError}
	},
	godot::prelude::*
};

pub const J: u8 = 11;
pub const Q: u8 = 12;
pub const K: u8 = 13;
pub const A: u8 = 14;
pub const CARDS_PER_PLAYER: u8 = 6;
pub const MAX_GAME_METAS: u8 = 253;

#[derive(Encode, Decode, Clone)]
pub struct Player {
	pub player_type: PlayerType,
	pub team_id: u8,
	pub hand: Vec<Card>
}
impl Player {
	pub async fn new_client(client: Arc<RwLock<Client>>, host: bool, team_id: u8) -> Self {
		Self {
			player_type: PlayerType::Client(PlayerClient::new(client, host).await),
			team_id,
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize)
		}
	}
	pub fn new_ai(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::AI,
			team_id,
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize)
		}
	}
	pub fn new_human(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::Human,
			team_id,
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize)
		}
	}
	pub fn update_player(&mut self, other: &Self, update_client_info: bool) -> Result<Option<PlayerType>, ServerError> {
		self.team_id = other.team_id;
		self.hand.clone_from(&other.hand);
		if let PlayerType::Client(this_client) = &mut self.player_type {
			if let PlayerType::Client(other_client) = &other.player_type {
				if update_client_info {
					this_client.account_id = other_client.account_id;
					this_client.host = other_client.host;
					this_client.ready = other_client.ready;
				}
				Ok(None)
			} else {
				Ok(Some(mem::replace(&mut self.player_type, other.player_type.clone())))
			}
		} else if let PlayerType::Client(_) = &other.player_type {
			Err(ServerError::ClientNotFound)
		} else {
			self.player_type = other.player_type.clone();
			Ok(None)
		}
	}
	pub fn can_follow(&self, lead: Suit) -> bool {
		for card in self.hand.iter() {
			if card.suit == lead {
				return true;
			}
		}
		false
	}
}

//#[derive(Encode, Decode, Clone)] //implemented manually because of Arc
pub struct PlayerClient {
	pub client: Option<Arc<RwLock<Client>>>,
	pub account_id: u32,
	pub host: bool,
	pub ready: bool
}
impl PlayerClient {
	pub async fn new(client: Arc<RwLock<Client>>, host: bool) -> Self {
		Self {client: Some(client), account_id: 0, host, ready: false}
	}
}
impl Clone for PlayerClient {
	fn clone(&self) -> Self {
		Self {
			client: None,
			account_id: self.account_id,
			host: self.host,
			ready: self.ready
		}
	}
}
impl Encode for PlayerClient {
	fn encode<__E: Encoder>(&self, encoder: &mut __E) -> Result<(), EncodeError> {
		//Encode::encode(&self.client, encoder)?; //don't encode the pointer
		Encode::encode(&self.account_id, encoder)?;
		Encode::encode(&self.host, encoder)?;
		Encode::encode(&self.ready, encoder)?;
		Ok(())
	}
}
impl Decode for PlayerClient {
	fn decode<__D: Decoder>(decoder: &mut __D) -> Result<Self, DecodeError> {
		Ok(Self {
			client: None,//Decode::decode(decoder)?,
			account_id: Decode::decode(decoder)?,
			host: Decode::decode(decoder)?,
			ready: Decode::decode(decoder)?,
		})
	}
}
impl<'__de> BorrowDecode<'__de> for PlayerClient {
	fn borrow_decode<__D: BorrowDecoder<'__de>>(decoder: &mut __D) -> Result<Self, DecodeError> {
		Ok(Self {
			client: None,//BorrowDecode::borrow_decode(decoder)?,
			account_id: BorrowDecode::borrow_decode(decoder)?,
			host: BorrowDecode::borrow_decode(decoder)?,
			ready: BorrowDecode::borrow_decode(decoder)?,
		})
	}
}

#[repr(u8)]
#[derive(Encode, Decode, Clone)]
pub enum PlayerType {
	///Represents an empty slot in multiplayer, or the human in singleplayer
	Human,
	Client(PlayerClient),
	AI
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
#[derive(Encode, Decode, Clone, PartialEq, Eq, Hash, Debug, GodotConvert, Var)]
#[godot(via=u8)]
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
	pub max_players_cmp: Comparison,
	pub max_players_value: u8,
	pub std_team_mode: bool,
	pub max_score_cmp: Comparison,
	pub max_score_value: i8,
	pub win_by_2: bool,
	pub negative_scores: bool
}

#[repr(u8)]
#[derive(Encode, Decode, Debug, Clone, GodotConvert, Var)]
#[godot(via=u8)]
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