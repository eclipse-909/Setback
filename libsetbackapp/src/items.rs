use {
	crate::{SmallVec, constants::*, setup::*, Error},
	serde::{Deserialize, Serialize},
	std::fmt::{Display, Formatter},
};

/// Versions should try to follow SemVer and are considered compatible if the majors are the same
#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct Version {
	pub major: u8,
	pub minor: u8,
	pub patch: u8,
}

impl Display for Version {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
	}
}

impl Version {
	pub const fn is_compatible(&self, other: Self) -> bool {
		self.major == other.major
	}
}

/// This needs to be a macro rather than a const fn because `env!("CARGO_PKG_VERSION")` expands at compile time,
/// and would always refer to libsetback when we might want it to refer to dependent packages.
#[macro_export]
macro_rules! pkg_version {
    () => {
        const {
			// Thanks Rust for not having 1st class support for compile time code execution.
			// Now I have to manually parse the string because str methods aren't const.

			const VERSION_STR: &str = env!("CARGO_PKG_VERSION");
			const LEN: usize = VERSION_STR.len();
			let mut buffers = [[0; LEN]; 3];
			let mut buf_lens = [0; 3];
			let mut nums = [0; 3];
			let mut bufs_idx = 0;
			let ptr = VERSION_STR.as_ptr();
			let mut i = 0;
			let mut buf_idx = LEN - 1;
			while i < LEN {
				// Safety
				// while loop does bounds checking
				let b = unsafe { *ptr.add(i) };
				match b {
					b'.' => {
						bufs_idx += 1;
						buf_idx = LEN - 1;
					}
					b'0'..=b'9' => {
						buffers[bufs_idx][buf_idx] = b;
						buf_lens[bufs_idx] += 1;
						buf_idx -= 1;
					}
					_ => panic!("invalid character in version string"),
				}
				i += 1;
			}
			bufs_idx = 0;
			while bufs_idx < 3 {
				let len = buf_lens[bufs_idx];
				i = LEN - len;
				loop {
					if i >= LEN {
						break;
					}
					let b = buffers[bufs_idx][i];
					let num = match b {
						b @ b'0'..=b'9' => b - b'0',
						_ => panic!("invalid byte"),
					};
					let multiplier = 10u8.pow((i - (LEN - len)) as u32);
					nums[bufs_idx] += num * multiplier;
					i += 1;
				}
				bufs_idx += 1;
			}
			Version { major: nums[0], minor: nums[1], patch: nums[2] }
		}
    };
}

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

	pub const fn to_points(self) -> i8 {
		match self {
			Bid::Pass => 0,
			Bid::Two => 2,
			Bid::Three => 3,
			Bid::Four => 4,
			Bid::Override => 4,
		}
	}

	pub const BID_INPUT_FMT: &str =
r"
0..5, pass, override
";

	/// Parses a string into a bid.
	/// Returns None if the string is not a valid bid.
	/// Valid formats - see [Self::BID_INPUT_FMT].
	/// Case insensitive.
	pub fn parse_string(string: &str) -> Option<Self> {
		match string.trim().to_lowercase().as_str() {
			"pass" | "0" => Some(Bid::Pass),
			"2" => Some(Bid::Two),
			"3" => Some(Bid::Three),
			"4" => Some(Bid::Four),
			"override" | "5" => Some(Bid::Override),
			_ => None,
		}
	}
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct Card {
	pub suit: Suit,
	pub number: u8,
}

impl Card {
	pub(crate) const fn new_deck() -> SmallVec<Self, DECK_SIZE> {
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

	/// returns 0 of hearts, which is an invalid card that must not be used in the game.
	pub const fn zero() -> Self {
		Self {number: 0, suit: Suit::Hearts}
	}

	pub const CARD_INPUT_FMT: &str =
r"
<number> <suit>
<number> of <suit>
<index>
number: 2..=14, j, jack, q, queen, k, king, a, ace
suit: hearts, clubs, diamonds, spades
index: 0-based index of card in hand
";

	/// Parses a string into a card.
	/// Returns None if the string is not a valid card.
	/// Returns Ok if the card valid is given, or Err if the index was given.
	/// Err should not be treated as an error, it's just an enum with 2 options.
	/// Valid formats - see [Self::CARD_INPUT_FMT].
	/// Case insensitive.
	pub fn parse_string(string: &str) -> Option<Result<Self, u8>> {
		let string = string.to_lowercase();
		let strings: Vec<&str> = string.split(' ').collect();
		match strings.len() {
			1 => if let Ok(idx) = strings[1].parse::<u8>() {
				return Some(Err(idx));
			},
			2 => return Some(Ok(Card {
				number: Self::parse_num_string(strings[0])?,
				suit: Self::parse_suit_string(strings[1])?
			})),
			3 => {
				if strings[1].trim() != "of" {
					return None;
				}
				return Some(Ok(Card {
					number: Self::parse_num_string(strings[0])?,
					suit: Self::parse_suit_string(strings[2])?
				}));
			}
			_ => {}
		}
		None
	}

	fn parse_num_string(string: &str) -> Option<u8> {
		match string.trim() {
			"j" | "jack" => Some(JACK),
			"q" | "queen" => Some(QUEEN),
			"k" | "king" => Some(KING),
			"a" | "ace" => Some(ACE),
			_ => string.parse::<u8>().ok().and_then(|n| if !(2..=14).contains(&n) {None} else {Some(n)}),
		}
	}

	fn parse_suit_string(string: &str) -> Option<Suit> {
		match string.trim() {
			"hearts" => Some(Suit::Hearts),
			"clubs" => Some(Suit::Clubs),
			"diamonds" => Some(Suit::Diamonds),
			"spades" => Some(Suit::Spades),
			_ => None,
		}
	}
}

impl std::fmt::Debug for Card {
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
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
	pub fn compare<T: PartialEq + PartialOrd>(&self, a: T, b: T) -> bool {
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Team {
	//the cards the team collects after winning a turn
	pub pile: SmallVec<Card, PILE_SIZE>,
	//score of the game
	pub score: i8,
}

impl Team {
	pub(crate) const fn new() -> Self {
		Self {
			pile: SmallVec::new(),
			score: 0,
		}
	}
}

/// Resulting scores of a sequence, when all cards have been played.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RoundScores {
	pub high_winner_team_id: u8,
	pub low_winner_team_id: u8,
	pub jack_winner_team_id: Option<u8>,
	pub game_winner_team_id: Option<u8>,
	/// The number of points each team got from game (face cards and 10s) for this round
	pub round_team_game_scores: SmallVec<u8, MAX_PLAYERS>,
	/// The number of points each team now has in total (high, low, jack, and game) after this round
	pub total_team_scores: SmallVec<i8, MAX_PLAYERS>,
}

/// Events that can be sent from the game to the player.
/// The game will emit these events in singleplayer.
/// In multiplayer, the TCP stream will receive [ssp] messages from the server, which may contain these events.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum GameEvent {
	Init {idx: u8},
	/// Index player bid the amount
	PlayerBid { idx: u8, bid: Bid },
	/// Index player played the card
	PlayerPlayedCard { idx: u8, card: Card },
	/// This should only be sent to the player whose turn it is.
	PromptBid(Option<Error>),
	/// This should only be sent to the player whose turn it is.
	PromptCard(Option<Error>),
	/// Deal the hand to the player.
	DealCards { hand: SmallVec<Card, CARDS_PER_PLAYER>, idx: u8 },
	/// The round is over, and the scores of the round are given for each team
	RoundScores(RoundScores),
	GameOver,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PublicGame {
	pub join_code: u32,
	pub setup_data: SetupState,
}