use {
	std::{
		array::IntoIter,
		fmt::{Display, Formatter},
		sync::{Arc, RwLock}
	},
	bincode::{Decode, Encode},
	crate::{
		constants::*,
		setup_data::*,
		game::*
	},
	tokio::{
		sync::{
			watch,
			mpsc::{Sender, UnboundedReceiver}
		},
		task::JoinHandle
	},
	tokio_util::sync::CancellationToken,
	rand::{
		prelude::ThreadRng,
		rng,
		Rng
	}
};

#[derive(Encode, Decode, Clone, Copy, PartialEq, Debug)]
pub struct Card {
	pub suit: Suit,
	pub number: u8
}

impl Display for Card {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} of {:?}", match self.number {
			JACK => {"Jack".to_string()}
			QUEEN => {"Queen".to_string()}
			KING => {"King".to_string()}
			ACE => {"Ace".to_string()}
			n => {n.to_string()}
		}, self.suit)
	}
}

#[repr(u8)]
#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Suit {
	Hearts,
	Clubs,
	Diamonds,
	Spades
}

impl Suit {
	pub fn iter() -> IntoIter<Self, 4> {[Self::Hearts, Self::Clubs, Self::Diamonds, Self::Spades].into_iter()}
}

#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
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
#[derive(Encode, Decode, Debug, Clone, PartialEq, Eq)]
pub enum Comparison {
	Equal,
	LessOrEqual,
	GreaterOrEqual,
	Less,
	Greater,
	NotEqual
}

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
///In multiplayer, the TCP stream will receive [ssp] messages from the server, which may contain these events.
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

#[repr(u8)]
pub enum TurnResult {
	NextTurn,
	///A re-deal is required after this one
	ReDeal,
	///A re-deal is required for this one too
	EndRound(RoundScores),
	///Round scores and team scores for whole game
	GameOver(RoundScores, Vec<i8>)
}

#[derive(Encode, Decode)]
pub struct PublicGame {
	pub join_code: u32,
	pub setup_data: SetupData
}

#[repr(u8)]
#[derive(Encode, Decode, Debug)]
pub enum Error {
	Indeterminate,
	Authentication,
	Authorization,
	NetworkIO,
	ServerError,
	IncompatibleVersion,
	InvalidProtocol,
	GamesAtCapacity,
	GameAtCapacity,
	GameNotFound,
	ClientNotFound,
	InvalidBid,
	InvalidCard,
	OutOfTurn
}

impl Display for Error {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {write!(f, "{}", match self {
		Error::Indeterminate => "Unexpected error - cause unknown",
		Error::Authentication => "Invalid username or password",
		Error::Authorization => "You do not have permission to perform this action",
		Error::NetworkIO => "Network error - try again later",
		Error::ServerError => "Server error - try again later",
		Error::IncompatibleVersion => "Incompatible version - please update the game",
		Error::InvalidProtocol => "Invalid protocol",
		Error::GamesAtCapacity => "Server is at capacity - please try to create a game later",
		Error::GameAtCapacity => "Game has maximum number of players - cannot join",
		Error::GameNotFound => "Game not found - the game code may be incorrect or the game may have ended",
		Error::ClientNotFound => "Client not found",
		Error::InvalidBid => "Invalid bid",
		Error::InvalidCard => "Invalid card - play a legal card",
		Error::OutOfTurn => "It is not your turn"
	})}
}

impl std::error::Error for Error {}

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

pub fn shuffle(deck: &mut [Card]) {
	let mut rng: ThreadRng = rng();
	for _ in 0..7 {
		for i in (1..deck.len()).rev() {
			let j: usize = rng.random_range(0..=i);
			deck.swap(i, j);
		}
	}
}

///returns (high, low, jack, game)
#[inline(always)]
pub fn calc_points(pile: &[Card], trump: Suit) -> (u8, u8, bool, u8) {
	let mut highest: u8 = 2;
	let mut lowest: u8 = ACE;
	let mut jack: bool = false;
	let mut game: u8 = 0;
	for card in pile.iter() {
		match card.number {
			10 => {game += 10;}
			JACK => {
				game += 1;
				if card.suit == trump {jack = true;}
			}
			QUEEN => {game += 2;}
			KING => {game += 3;}
			ACE => {game += 4}
			_ => {}
		}
		if card.suit != trump {continue;}
		if card.number > highest {
			highest = card.number;
		}
		if card.number < lowest {
			lowest = card.number;
		}
	}
	(highest, lowest, jack, game)
}