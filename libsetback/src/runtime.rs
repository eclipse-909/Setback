use {
	arrayvec::ArrayVec,
	crate::*,
	rand::seq::{IndexedRandom, SliceRandom},
	serde::{Deserialize, Serialize},
	tracing::debug,
	tokio::sync::mpsc::UnboundedSender,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameState {
	pub lobby_state: LobbyState,
	pub teams: ArrayVec<Team, MAX_PLAYERS>,
	pub deck: ArrayVec<Card, DECK_SIZE>,
	pub pile: ArrayVec<Card, MAX_PLAYERS>,
	pub trump: Suit,
	pub lead: Suit,
	/// The index of the player who owns the pile. Played the highest_card.
	pub owner_idx: u8,
	/// The card that was played to take ownership of the pile. Played by the pile_owner
	pub highest_card: Card,
	/// Index of the player who won the bid. Made the highes_bid.
	pub highest_bidder_idx: u8,
	/// The highest bid so far
	pub highest_bid: Bid,
	/// The index of the player whose turn it is
	pub turn_idx: u8,
	/// This turn is when everyone plays their turn. This will be 0..6 excluding the bidding turn.
	pub turn_num: u8,
	pub dealer_idx: u8,
	/// A round is 6 turns; when all cards have been played
	pub round_num: u8,
	/// Makes it easier to keep track of states in this state machine
	pub loop_state: GameLoopState,
}

pub type ErrQ = UnboundedSender<(u8, NetError)>;

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum GameLoopState {
	Init,
	DealCards,
	Bidding,
	Playing,
	GameOver,
	Terminated,
}

// Runtime-owned trait objects
/// The runtime emits events that the client subscribes to.
pub trait EventEmitter {
	fn send(&self, event: GameEvent) -> Result<(), Error>;
}
/// The runtime prompts clients and receives a response from the client
pub trait EventResponseReceiver {
	fn recv(&mut self) -> impl Future<Output=Result<GameEvent, Error>>;
}

// Client-owned trait objects
/// The client receives events from the event emitter
pub trait EventSubscriber {
	fn recv(&mut self) -> impl Future<Output=Result<GameEvent, Error>>;
}
/// The client sends events back to the runtime when prompted
pub trait PromptResponder {
	fn send(&self, event: GameEvent) -> Result<(), Error>;
}

#[cfg(not(feature = "server"))]
impl GameState {
	/// ## Safety
	/// No mutable pointers to player type must exist.
	#[cfg(not(feature = "test-libsetback"))]
	fn human_idx(&self) -> u8 {
		self.lobby_state
			.players
			.iter()
			.position(|p| matches!(&p.player_type, PlayerType::SingleplayerSelf { .. }))
			.unwrap() as u8
	}

	#[cfg(feature = "test-libsetback")]
	fn human_idx(&self) -> u8 {
		// there is no human player in tests
		u8::MAX
	}

	pub async fn run_singleplayer<
		#[cfg(not(feature = "test-libsetback"))] Ee: EventEmitter,
		#[cfg(not(feature = "test-libsetback"))] Err: EventResponseReceiver
	>(
		mut self,
		#[cfg(not(feature = "test-libsetback"))] event_emitter: Ee,
		#[cfg(not(feature = "test-libsetback"))] mut event_response_rx: Err,
	) {
		let human_idx = self.human_idx();
		loop {
			// Tip: `continue` can be used to not increment the turn index.
			match self.loop_state {
				GameLoopState::Init => {
					#[cfg(not(feature = "test-libsetback"))]
					event_emitter
						.send(GameEvent::Init { idx: human_idx })
						.unwrap();
					self.loop_state = GameLoopState::DealCards;
					self.increment_turn();
				}
				GameLoopState::DealCards => {
					self.deal_cards();
					#[cfg(not(feature = "test-libsetback"))]
					event_emitter
						.send(GameEvent::DealCards {
							hand: self.lobby_state.players[human_idx as usize].hand.clone(),
							idx: self.turn_idx,
						})
						.unwrap();
					self.loop_state = GameLoopState::Bidding;
					// it is currently the dealer's turn and we want the next player to start bidding.
					self.increment_turn();
				}
				GameLoopState::Bidding => {
					#[cfg(feature = "test-libsetback")]
					let bid = self.ai_bid();
					#[cfg(not(feature = "test-libsetback"))]
					let bid = if self.turn_idx == human_idx {
						let mut bid_err = None;
						loop {
							event_emitter.send(GameEvent::PromptBid(bid_err)).unwrap();
							match event_response_rx.recv().await {
								Ok(GameEvent::PlayerBid { bid, .. }) => {
									if self.bid_is_valid(bid) {
										break bid;
									}
									bid_err = Some(PromptError::Invalid);
								}
								Ok(_) | Err(_) => continue,
							}
						}
					} else {
						self.ai_bid()
					};
					// It feels redundant to let the player know what they just bid,
					// but this tells the player that their bid was valid.
					#[cfg(not(feature = "test-libsetback"))]
					event_emitter
						.send(GameEvent::PlayerBid {
							idx: self.turn_idx,
							bid,
						})
						.unwrap();
					self.player_bid(bid);
				}
				GameLoopState::Playing => {
					#[cfg(feature = "test-libsetback")]
					let card = self.ai_play_card();
					#[cfg(not(feature = "test-libsetback"))]
					let card = if self.turn_idx == human_idx {
						let mut card_err = None;
						loop {
							event_emitter.send(GameEvent::PromptCard(card_err)).unwrap();
							match event_response_rx.recv().await {
								Ok(GameEvent::PlayerPlayedCard { card, .. }) => {
									if self.card_is_valid(card) {
										break card;
									}
									card_err = Some(PromptError::Invalid);
								}
								Ok(_) | Err(_) => continue,
							}
						}
					} else {
						self.ai_play_card()
					};
					// It feels redundant to let the player know which card they played,
					// but this tells the player that their card was valid.
					#[cfg(not(feature = "test-libsetback"))]
					event_emitter
						.send(GameEvent::PlayerPlayedCard {
							idx: self.turn_idx,
							card,
						})
						.unwrap();
					match self.player_played_card(card) {
						PlayedCardResult::None => {}
						PlayedCardResult::RoundOver(round_scores) => {
							#[cfg(not(feature = "test-libsetback"))]
							event_emitter
								.send(GameEvent::RoundScores(round_scores))
								.unwrap();
							if self.team_won() {
								self.loop_state = GameLoopState::GameOver;
							} else {
								self.loop_state = GameLoopState::DealCards;
							}
						}
						PlayedCardResult::GameOver(round_scores) => {
							#[cfg(not(feature = "test-libsetback"))]
							event_emitter
								.send(GameEvent::RoundScores(round_scores))
								.unwrap();
							self.loop_state = GameLoopState::GameOver;
						}
					}
				}
				GameLoopState::GameOver => {
					#[cfg(not(feature = "test-libsetback"))]
					event_emitter.send(GameEvent::GameOver).unwrap();
					debug!(cx = %self.cx(), "Game over");
					return;
				}
				GameLoopState::Terminated => return,
			}
		}
	}
}

impl GameState {
	pub fn increment_turn(&mut self) {
		let turn_idx = self.turn_idx + 1;
		self.turn_idx = if turn_idx as usize == self.lobby_state.players.len() {
			0
		} else {
			turn_idx
		}
	}

	pub fn deal_cards(&mut self) {
		// reset round-specific data
		self.turn_idx = self.dealer_idx;
		self.highest_card = Card::zero();
		self.highest_bid = Bid::Pass;

		// collect cards and shuffle
		self.deck.extend(self.pile.drain(..));
		for player in self.lobby_state.players.iter_mut() {
			self.deck.extend(player.hand.drain(..));
		}
		for team in self.teams.iter_mut() {
			self.deck.extend(team.pile.drain(..));
		}
		self.deck.as_mut_slice().shuffle(&mut rand::rng());

		// deal cards
		for player in self.lobby_state.players.iter_mut() {
			player.hand.extend(self.deck.drain(..CARDS_PER_PLAYER));
		}
		debug!(cx = %self.cx(), "player {} dealt the cards", self.turn_idx);
		debug_assert_eq!(
			self.deck.len(),
			DECK_SIZE - (self.lobby_state.players.len() * CARDS_PER_PLAYER)
		);
	}

	/// Algorithm for deciding how the AI should bid
	/// Right now this is just an arbitrary mapping,
	/// and will eventually be replaced with a better AI
	fn ai_eval_bid(&self) -> (Suit, u8) {
		let player = &self.lobby_state.players[self.turn_idx as usize];
		// [Hearts, Clubs, Diamonds, Spades]
		let mut suit_points = [0u8; 4];
		for card in player.hand.iter() {
			suit_points[card.suit as usize] += match card.number {
				2 => 10,
				3 => {
					if self.lobby_state.players.len() == MAX_PLAYERS {
						4
					} else {
						8
					}
				}
				4 => 3,
				5 => 4,
				6 => 5,
				7 => 10,
				8 => 12,
				9 => 20, // 9 through A must not add up to more than 255
				10 => 25,
				JACK => 40,
				QUEEN => 35,
				KING => 45,
				ACE => 90, // A should be more than any 2 other things combined
				_n => unreachable!("card number should not exist: {_n}"),
			}
		}
		let (index, score) = suit_points
			.into_iter()
			.enumerate()
			.max_by(|(_, num1), (_, num2)| num1.cmp(num2))
			.unwrap();
		(
			// SAFETY:
			// Suit is repr(u8) and the array length is the same as the number of suits.
			unsafe { std::mem::transmute::<u8, Suit>(index as u8) },
			score,
		)
	}

	/// Uses a card-scoring system, the turn order, and the game score to determine the bid
	pub fn ai_bid(&self) -> Bid {
		let (_, points) = self.ai_eval_bid();
		let mut bid = match points {
			..=44 => 0,                               // pass
			45..=99 => 1,                             // minimum play out of desperation
			100..=139 => 2,                           // A + one more good card
			140..=179 => 3,                           // A + face card or two + maybe one more
			180..=219 => 4,                           // A + K + J + one more good card
			220.. if self.lobby_state.rules.dealer_override => 5, // A + K + Q + J + one more good card
			220.. => 4,                               // A + K + Q + J + one more good card
		};
		// if you are one of the first to bid, maybe bid higher either to force an opponent to outbid when they shouldn't
		// or to prevent opponents from bidding to try to be the highest bidder
		if bid != 0
			&& bid < 4
			&& ((90..=99).contains(&points) || (125..=139).contains(&points))
			&& (self.turn_idx as i8 - self.dealer_idx as i8).unsigned_abs() < 5
		{
			bid += 1;
		}
		// if opponents are about to win, make risky bids
		if let Some((index, lead_team)) = self
			.teams
			.iter()
			.enumerate()
			.max_by(|(_, team1), (_, team2)| team1.score.cmp(&team2.score))
		{
			let player = &self.lobby_state.players[self.turn_idx as usize];
			if player.team_id != index as u8
				&& lead_team.score as i16 >= self.lobby_state.rules.max_score as i16 - 4
				&& self.lobby_state.rules.max_score as i16 - 4 > self.teams[player.team_id as usize].score as i16
				&& bid != 4
			{
				bid += 1;
			}
		}
		if bid <= self.highest_bid.to_points() as u8 {
			bid = 0;
		}
		Bid::from(bid)
	}

	///Plays the highest card if this player had the highest bid.
	///Currently picks a random valid card until a proper AI is implemented.
	pub fn ai_play_card(&self) -> Card {
		let player = &self.lobby_state.players[self.turn_idx as usize];
		//Highest bidder starts the round, and the leading card is a unique calculation
		if self.turn_num == 0 && self.turn_idx == self.highest_bidder_idx {
			//Suit of highest bid should not have changed, so we will just recalculate it instead of storing it
			let (suit, _) = self.ai_eval_bid();
			let highest = *player
				.hand
				.iter()
				.filter(|card| card.suit == suit)
				.max_by(|card1, card2| card1.number.cmp(&card2.number))
				.unwrap();
			return highest;
		}
		let playable_cards = if !self.pile.is_empty() && player.can_follow(self.lead) {
			player
				.hand
				.iter()
				.filter(|card| card.suit == self.lead || card.suit == self.trump)
				.cloned()
				.collect()
		} else {
			player.hand.clone()
		};

		//For now I'm gonna get a random card
		*playable_cards.as_slice().choose(&mut rand::rng()).unwrap()

		//TODO make AI to choose card to play

		/*
		Factors:
			legal cards in hand
			number of players
			number of teams
			turn index between leader and last to play
			team number of players
			team that owns the pile
			other teams that could take the pile
			cards that have been played, particularly trump cards
			the highest trump card that hasn't been played
			who's following trump suit
			if the pile contains A, J, and/or 2 (or 3 if 2 hasn't been played yet)
			how many game points are in the pile

		Heuristics:
			if AI can follow, only allow follow suit or trump suit, else allow all cards
		*/
	}

	/// Returns true if valid, false if invalid. This function is idempotent.
	pub fn bid_is_valid(&self, bid: Bid) -> bool {
		bid == Bid::Pass
			|| (bid > self.highest_bid && (bid != Bid::Override || self.lobby_state.rules.dealer_override))
	}

	/// Returns true if valid, false if invalid. This function is idempotent.
	pub fn card_is_valid(&self, card: Card) -> bool {
		let player = &self.lobby_state.players[self.turn_idx as usize];
		let has_card = player.hand.contains(&card);
		let can_follow = player.can_follow(self.lead);
		let is_owner = self.owner_idx == self.turn_idx;
		let is_lead = card.suit == self.lead;
		let is_trump = card.suit == self.trump;
		has_card && (is_lead || is_trump || is_owner || !can_follow)
	}

	/// The bid must have already been validated because it will be considered the new highest bid.
	///
	/// Returns true if bidding should end, and the turn index should not be incremented following this function.
	/// Returns false if the next player needs to bid.
	pub fn player_bid(&mut self, bid: Bid) {
		debug_assert!(self.bid_is_valid(bid));
		debug!(cx = %self.cx(), "player {} bid {}", self.turn_idx, bid.to_points());
		if bid != Bid::Pass {
			self.highest_bid = bid;
			self.highest_bidder_idx = self.turn_idx;
			if (bid == Bid::Four && !self.lobby_state.rules.dealer_override)
				|| (bid == Bid::Override && self.lobby_state.rules.dealer_override)
			{
				self.loop_state = GameLoopState::Playing;
				return;
			}
		}
		// if all players have bid, end the turn...
		if self.turn_idx == self.dealer_idx {
			if self.highest_bid == Bid::Pass {
				self.loop_state = GameLoopState::DealCards;
			} else {
				self.loop_state = GameLoopState::Playing;
				self.turn_idx = self.highest_bidder_idx;
				self.owner_idx = self.highest_bidder_idx;
			}
		} else {
			self.increment_turn();
		}
	}

	/// The card must have already been validated because it will be taken from the player's hand and moved to the pile.
	///
	/// Returns Some if card playing for the round should end, and the turn index should not be incremented following this function.
	/// Returns None if the next player needs to play a card.
	pub fn player_played_card(&mut self, card: Card) -> PlayedCardResult {
		debug_assert!(self.card_is_valid(card));
		debug!(cx = %self.cx(), "player {} played the {card:?}", self.turn_idx);
		let hand = &mut self.lobby_state.players[self.turn_idx as usize].hand;
		let remove_idx = hand.iter().position(|c| *c == card).unwrap(); // validate_card already looks for the card in the hand
		hand.swap_remove(remove_idx);
		self.pile.push(card);

		// TODO: we can improve gameplay experience by ending a round early when a team is guaranteed to win.
		// When a team is 1 point away from winning, and they play a card that guarantees them a point at the end of the round,
		// we can stop the round early. The best example is when a team is 1 point away, and they play the ace.

		if self.owner_idx == self.turn_idx {
			// if this is the first card played this turn...
			self.highest_card = card;
			self.owner_idx = self.turn_idx;
			self.lead = card.suit;
			if self.turn_num == 0 {
				// if this is the first turn of the round...
				self.trump = card.suit;
			}
		} else if (card.suit == self.highest_card.suit && card.number > self.highest_card.number)
			|| (self.highest_card.suit != self.trump && card.suit == self.trump)
		{
			// if this card beats the current best card...
			self.highest_card = card;
			self.owner_idx = self.turn_idx;
		}
		// if this is the last card to be played this turn...
		if self.pile.len() == self.lobby_state.players.len() {
			self.turn_idx = self.owner_idx;
			self.turn_num += 1;
			// The winning team takes the pile after every turn.
			self.teams[self.lobby_state.players[self.owner_idx as usize].team_id as usize]
				.pile
				.extend(self.pile.drain(..));
			if self.turn_num == CARDS_PER_PLAYER as u8 {
				// if all cards have been played, end the round
				self.turn_num = 0;
				self.round_num += 1;
				self.dealer_idx += 1;
				if self.dealer_idx as usize == self.lobby_state.players.len() {
					self.dealer_idx = 0;
				}

				// calculate round scores
				let mut high_winner_team_id = 0u8;
				let mut low_winner_team_id = 0u8;
				let mut jack_winner_team_id = None;
				let mut game_winner_team_id = 0u8;
				let mut high = 0;
				let mut low = 255;
				let mut highest_game = 0u8;
				let mut second_highest_game = 0;
				let mut round_team_game_scores = ArrayVec::new();
				for (team_id, team) in self.teams.iter().enumerate() {
					let team_id = team_id as u8;
					let mut game = 0;
					for card in team.pile.iter() {
						game += match card.number {
							JACK => 1,
							QUEEN => 2,
							KING => 3,
							ACE => 4,
							10 => 10,
							_ => 0,
						};
						if card.suit != self.trump {
							continue;
						}
						if card.number > high {
							high_winner_team_id = team_id;
							high = card.number;
						} else if card.number < low {
							low_winner_team_id = team_id;
							low = card.number;
						}
						if card.number == JACK {
							debug_assert_eq!(jack_winner_team_id, None);
							jack_winner_team_id = Some(team_id);
						}
					}
					round_team_game_scores.push(game);
					if game > highest_game {
						game_winner_team_id = team_id;
						second_highest_game = highest_game;
						highest_game = game;
					}
				}
				let mut bidder_team_points = 0;
				let bidder_team_id = self.lobby_state.players[self.highest_bidder_idx as usize].team_id;
				if high_winner_team_id == bidder_team_id {
					bidder_team_points += 1;
				} else {
					self.teams[high_winner_team_id as usize].score += 1;
				}
				if low_winner_team_id == bidder_team_id {
					bidder_team_points += 1;
				} else {
					self.teams[low_winner_team_id as usize].score += 1;
				}
				if let Some(jack_team_id) = jack_winner_team_id {
					if jack_team_id == bidder_team_id {
						bidder_team_points += 1;
					} else {
						self.teams[jack_team_id as usize].score += 1;
					}
				}
				if game_winner_team_id == bidder_team_id {
					bidder_team_points += 1;
				} else {
					self.teams[game_winner_team_id as usize].score += 1;
				}
				if bidder_team_points >= self.highest_bid.to_points() {
					self.teams[bidder_team_id as usize].score += bidder_team_points;
				} else {
					self.teams[bidder_team_id as usize].score -= self.highest_bid.to_points();
				}
				let round_scores = RoundScores {
					high_winner_team_id,
					low_winner_team_id,
					jack_winner_team_id,
					game_winner_team_id: if highest_game == second_highest_game {
						None
					} else {
						Some(game_winner_team_id)
					},
					round_team_game_scores,
					total_team_scores: self.teams.iter().map(|team| team.score).collect(),
				};
				debug!(cx = %self.cx(), "Round ended - {round_scores:?}");
				return PlayedCardResult::RoundOver(round_scores);
			}
			return PlayedCardResult::None;
		}
		self.increment_turn();
		PlayedCardResult::None
	}

	pub fn team_won(&self) -> bool {
		let mut highest_score = 0;
		let mut second_highest_score = 0;
		for team in self.teams.iter() {
			if team.score >= highest_score {
				second_highest_score = highest_score;
				highest_score = team.score;
			}
		}
		highest_score as i16 > self.lobby_state.rules.max_score as i16
			&& (highest_score > second_highest_score + 1 || !self.lobby_state.rules.win_by_2)
	}
	
	pub fn cx(&self) -> LobbyLogCx<'_> {
		self.lobby_state.cx()
	}
}

#[derive(Clone)]
pub enum PlayedCardResult {
	None,
	RoundOver(RoundScores),
	GameOver(RoundScores),
}