use {
	super::{
		util::{Error, MAX_PLAYERS},
		connection::ClientConnection,
		game::Game
	},
	std::{
		array::IntoIter,
		fmt::{Display, Debug, Formatter},
		sync::{Arc, RwLock}
	},
	tokio::sync::mpsc::{Sender, UnboundedReceiver},
	tokio_util::sync::CancellationToken,
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
pub struct Player {
	pub player_type: PlayerType,
	pub username: String,
	pub hand: Vec<Card>,
	pub team_id: u8
}
impl Player {
	pub fn new_singleplayer_self(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::SingleplayerSelf,
			//TODO load username from disk
			username: String::from("Me"),
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize),
			team_id
		}
	}

	///Empty slot for a human player in multiplayer. This function should only be called by the server.
	pub fn new_human_empty(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::HumanEmpty,
			username: String::from("Waiting for player to connect..."),
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize),
			team_id
		}
	}

	pub fn new_ai(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::AI,
			username: String::from("Bot"),
			hand: Vec::with_capacity(CARDS_PER_PLAYER as usize),
			team_id
		}
	}
	
	pub fn can_follow(&self, lead: Suit) -> bool { self.hand.iter().any(|card| {card.suit == lead}) }
}

#[repr(u8)]
pub enum PlayerType {
	///Yourself in singleplayer.
	SingleplayerSelf,
	///A slot for a human player in multiplayer that doesn't have a connection yet. This is only used by the server.
	HumanEmpty,
	///A connected player in multiplayer. This is only used by the server.
	HumanConnection(ClientConnection),
	///The account id and ready status of a connected player in multiplayer.
	///The server will convert a HumanConnection to a MultiplayerHuman when serializing and sending it to clients.
	///The client doesn't store this, they just take the account id and ready status from the HumanConnection and update the UI accordingly.
	MultiplayerHuman{account_id: u32, ready: bool},
	AI
}

impl PlayerType {
	pub fn set(&mut self, other: Self) {
		if let Self::HumanConnection(connection) = self {
			if let Self::MultiplayerHuman{account_id, ready} = other {
				connection.account_id = account_id;
				connection.ready = ready;
				return;
			}
		}
		*self = other;
	}
}

impl Encode for PlayerType {
	fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), EncodeError> {
		match self {
			Self::SingleplayerSelf => {Encode::encode(&0, encoder)}
			Self::HumanEmpty => {Encode::encode(&1, encoder)}
			Self::HumanConnection(connection) => {
				//HumanConnection is encoded as MultiplayerHuman
				Encode::encode(&3, encoder)?;
				Encode::encode(&connection.account_id, encoder)?;
				Encode::encode(&connection.ready, encoder)
			}
			Self::MultiplayerHuman{account_id, ready} => {
				Encode::encode(&3, encoder)?;
				Encode::encode(&account_id, encoder)?;
				Encode::encode(&ready, encoder)
			}
			Self::AI => {Encode::encode(&4, encoder)}
		}
	}
}

impl<Context> Decode<Context> for PlayerType {
	fn decode<D: Decoder>(decoder: &mut D) -> Result<Self, DecodeError> {
		let player_type: u8 = Decode::decode(decoder)?;
		match player_type {
			0 => Ok(Self::SingleplayerSelf),
			1 => unreachable!("HumanConnection is encoded as MultiplayerHuman"),
			2 => Ok(Self::HumanEmpty),
			3 => Ok(Self::MultiplayerHuman{account_id: Decode::decode(decoder)?, ready: Decode::decode(decoder)?}),
			4 => Ok(Self::AI),
			_ => unreachable!("Invalid player type: {}", player_type)
		}
	}
}

impl<'de, Context> BorrowDecode<'de, Context> for PlayerType {
	fn borrow_decode<D: BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, DecodeError> {
		let player_type: u8 = Decode::decode(decoder)?;
		match player_type {
			0 => Ok(Self::SingleplayerSelf),
			1 => unreachable!("HumanConnection is encoded as MultiplayerHuman"),
			2 => Ok(Self::HumanEmpty),
			3 => Ok(Self::MultiplayerHuman{account_id: BorrowDecode::borrow_decode(decoder)?, ready: BorrowDecode::borrow_decode(decoder)?}),
			4 => Ok(Self::AI),
			_ => unreachable!("Invalid player type: {}", player_type)
		}
	}
}

#[derive(Encode, Decode, Clone, Copy, PartialEq)]
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

///A struct to handle communication between the game and the players.
///
///[GameBuilder::start_game] moves the game into the function and starts the event loop.
pub struct GameRuntime {
	pub game: Arc<RwLock<Game>>,
	pub cancel_token: CancellationToken,
	pub event_emitter: UnboundedReceiver<GameEvent>,
	pub prompt_senders: Vec<(Sender<u8>, Sender<Card>)>
}

impl Drop for GameRuntime {
	fn drop(&mut self) {
		self.cancel_token.cancel();
	}
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
#[derive(Encode, Decode)]
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

#[derive(Encode, Decode)]
pub struct Team {
	//the cards the team collects after winning a turn
	pub pile: Vec<Card>,
	//score of the game
	pub score: i8
}
impl Default for Team {
	fn default() -> Self {Self {pile: Vec::with_capacity((MAX_PLAYERS * CARDS_PER_PLAYER) as usize), score: 0}}
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
#[derive(Encode, Decode)]
#[repr(u8)]
pub enum GameEvent {
	///Index player bid the amount
	PlayerBid{index: u8, amount: u8},
	///Index player played the card
	PlayerPlayedCard{index: u8, card: Card},
	///It is now index player's turn. This should be sent to every player except the one whose turn it is.
	PlayerTurn{index: u8},
	///It is now index player's turn to bid. This should only be sent to the player whose turn it is.
	PromptBid{index: u8, hand: Vec<Card>},
	///It is now index player's turn to play a card. This should only be sent to the player whose turn it is.
	PromptCard{index: u8, hand: Vec<Card>},
	///Signals that cards are being dealt.
	DealCards,
	///The round is over, and the scores of the round are given for each team
	RoundScores(RoundScores),
	///The game is over, and the scores of the round are given for each team
	GameOver(Vec<i8>),
	///The game has encountered an error
	GameError(Error)
}