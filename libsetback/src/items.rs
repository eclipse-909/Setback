use {
	crate::{constants::*, setup::*, SmallVec},
	serde::{Deserialize, Serialize},
	std::{
		fmt::{Display, Formatter},
		sync::{Arc, RwLock},
	},
	tokio::sync::mpsc::{Sender, UnboundedReceiver},
	tokio_util::sync::CancellationToken,
};

#[repr(u8)]
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, PartialOrd, Debug)]
pub enum Bid {
	Pass = 0,
	Two = 2,
	Three = 3,
	Four = 4,
	Override = 5,
}

impl Bid {
	pub const fn from(value: u8) -> Self {
		match value {
			2 => Bid::Two,
			3 => Bid::Three,
			4 => Bid::Four,
			5 => Bid::Override,
			_ => Bid::Pass,
		}
	}
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Debug)]
pub struct Card {
	pub suit: Suit,
	pub number: u8,
}

impl Card {
	pub const fn new_deck() -> SmallVec<Self, DECK_SIZE> {
		const {
			let mut deck = SmallVec::new();
			let mut number = 2;
			while number <= ACE {
				deck.push(Card { suit: Suit::Hearts, number });
				deck.push(Card { suit: Suit::Clubs, number });
				deck.push(Card { suit: Suit::Diamonds, number });
				deck.push(Card { suit: Suit::Spades, number });
				number += 1;
			}
			assert!(deck.len() == DECK_SIZE as u8);
			deck
		}
	}
}

impl Display for Card {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"{} of {:?}",
			match self.number {
				JACK => {
					"Jack".to_string()
				}
				QUEEN => {
					"Queen".to_string()
				}
				KING => {
					"King".to_string()
				}
				ACE => {
					"Ace".to_string()
				}
				n => {
					n.to_string()
				}
			},
			self.suit
		)
	}
}

#[repr(u8)]
#[derive(Deserialize, Serialize, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Suit {
	Hearts,
	Clubs,
	Diamonds,
	Spades,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct GameFilter {
	pub std_team_mode: bool,
	pub win_by_2: bool,
	pub negative_scores: bool,
	pub dealer_override: bool,
	pub max_score_cmp: Comparison,
	pub max_score_value: i8,
	pub min_players_cmp: Comparison,
	pub min_players_value: u8,
	pub max_players_cmp: Comparison,
	pub max_players_value: u8,
}

#[repr(u8)]
#[derive(Deserialize, Serialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
	None,
	Equal,
	LessOrEqual,
	GreaterOrEqual,
	Less,
	Greater,
	NotEqual,
}

impl Comparison {
	pub const fn compare_i8(&self, a: i8, b: i8) -> bool {
		match self {
			Comparison::None => true,
			Comparison::Equal => a == b,
			Comparison::LessOrEqual => a <= b,
			Comparison::GreaterOrEqual => a >= b,
			Comparison::Less => a < b,
			Comparison::Greater => a > b,
			Comparison::NotEqual => a != b,
		}
	}

	pub const fn compare_u8(&self, a: u8, b: u8) -> bool {
		match self {
			Comparison::None => true,
			Comparison::Equal => a == b,
			Comparison::LessOrEqual => a <= b,
			Comparison::GreaterOrEqual => a >= b,
			Comparison::Less => a < b,
			Comparison::Greater => a > b,
			Comparison::NotEqual => a != b,
		}
	}
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Team {
	//the cards the team collects after winning a turn
	pub pile: SmallVec<Card, PILE_SIZE>,
	//score of the game
	pub score: i8,
}

impl Team {
	pub const fn new() -> Self {
		Self {
			pile: SmallVec::new(),
			score: 0,
		}
	}
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RoundScores {
	pub high_team_id: u8,
	pub low_team_id: u8,
	pub jack_team_id: Option<u8>,
	pub game_team_id: Option<u8>,
	pub team_game_scores: SmallVec<u8, MAX_PLAYERS>,
	pub team_scores: SmallVec<i8, MAX_PLAYERS>,
}

/// Events that can be sent from the game to the player.
/// The game will emit these events in singleplayer.
/// In multiplayer, the TCP stream will receive [ssp] messages from the server, which may contain these events.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum GameEvent {
	///Index player bid the amount
	PlayerBid { index: u8, amount: Bid },
	///Index player played the card
	PlayerPlayedCard { index: u8, card: Card },
	///It is now index player's turn to bid. This should only be sent to the player whose turn it is.
	PromptBid,
	///It is now index player's turn to play a card. This should only be sent to the player whose turn it is.
	PromptCard,
	///Deal the hand to the player.
	DealCards { hand: SmallVec<Card, CARDS_PER_PLAYER> },
	///The round is over, and the scores of the round are given for each team
	RoundScores(RoundScores),
	///The game is over, and the scores of the round are given for each team
	GameOver(SmallVec<i8, MAX_PLAYERS>),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PublicGame {
	pub join_code: u32,
	pub setup_data: GameSetup,
}

///A struct to handle communication between the game and the players.
///
///[GameBuilder::start_game] moves the game into the function and starts the event loop.
pub struct GameRuntime {
	pub game: Arc<RwLock<GameSetup>>,
	pub cancel_token: CancellationToken,
	pub event_emitter: UnboundedReceiver<GameEvent>,
	pub prompt_senders: Vec<(Sender<Bid>, Sender<Card>)>,
}

impl Drop for GameRuntime {
	fn drop(&mut self) {
		self.cancel_token.cancel();
	}
}

///returns (high, low, jack, game)
#[inline(always)]
pub fn calc_points(pile: &[Card], trump: Suit) -> (u8, u8, bool, u8) {
	let mut highest = 2;
	let mut lowest = ACE;
	let mut jack = false;
	let mut game = 0;
	for card in pile.iter() {
		match card.number {
			10 => {
				game += 10;
			}
			JACK => {
				game += 1;
				if card.suit == trump {
					jack = true;
				}
			}
			QUEEN => {
				game += 2;
			}
			KING => {
				game += 3;
			}
			ACE => game += 4,
			_ => {}
		}
		if card.suit != trump {
			continue;
		}
		if card.number > highest {
			highest = card.number;
		}
		if card.number < lowest {
			lowest = card.number;
		}
	}
	(highest, lowest, jack, game)
}
