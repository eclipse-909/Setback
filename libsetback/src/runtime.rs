use {
	crate::*,
	rand::seq::{SliceRandom, IndexedRandom},
	serde::{Deserialize, Serialize},
	tokio::{
		sync::mpsc,
		task::JoinHandle
	},
	tokio_util::sync::CancellationToken,
	tracing::{info, warn, error},
	std::time::Duration,
};
#[cfg(feature = "client")]
use tokio::sync::watch;
#[cfg(not(feature = "client"))]
use crate::ssp::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GameState {
	pub(crate) setup: SetupState,
	pub(crate) teams: SmallVec<Team, MAX_PLAYERS>,
	pub(crate) deck: SmallVec<Card, DECK_SIZE>,
	pub(crate) pile: SmallVec<Card, MAX_PLAYERS>,
	pub(crate) trump: Suit,
	pub(crate) lead: Suit,
	/// The index of the player who owns the pile. Played the highest_card.
	pub(crate) owner_idx: u8,
	/// The card that was played to take ownership of the pile. Played by the pile_owner
	pub(crate) highest_card: Card,
	/// Index of the player who won the bid. Made the highes_bid.
	pub(crate) highest_bidder_idx: u8,
	/// The highest bid so far
	pub(crate) highest_bid: Bid,
	/// The index of the player whose turn it is
	pub(crate) turn_idx: u8,
	/// This turn is when everyone plays their turn. This will be 0..6 excluding the bidding turn.
	pub(crate) turn_num: u8,
	pub(crate) dealer_idx: u8,
	/// A round is 6 turns; when all cards have been played
	pub(crate) round_num: u8,
}

pub struct GameLoopHandle {
	pub cancel_token: CancellationToken,
	pub rejoin_tx: mpsc::Sender<ClientConnection>,
	pub task: JoinHandle<SmallVec<ClientConnection, MAX_PLAYERS>>,
}

#[repr(u8)]
#[derive(PartialEq, Clone, Copy)]
enum GameLoopState {
	DealCards,
	Bidding,
	Playing,
	RoundOver,
	GameOver,
}

impl GameState {
	/// Returns a cancel token, event subscriber, and prompt responder
	#[cfg(all(not(feature = "gpui"), feature = "client"))]
	pub fn spawn_singleplayer(self) -> (CancellationToken, mpsc::UnboundedReceiver<GameEvent>, watch::Sender<GameEvent>) {
		let cancel_token = CancellationToken::new();
		let cancel_token_clone = cancel_token.clone();
		// SAFETY:
		// we have exclusive access to self
		let human_idx = unsafe { self.human_idx() };
		let (event_emitter, event_subscriber) = mpsc::unbounded_channel();
		event_emitter.send(GameEvent::Init {idx: human_idx}).unwrap();
		let (prompt_responder, response_rx) = watch::channel(GameEvent::Init {idx: human_idx});
		tokio::spawn(async move {
			tokio::select! {
				_ = self.run_event_loop(human_idx, cancel_token.clone(), GameEventEmitter(event_emitter), response_rx) => {}
				_ = cancel_token.cancelled() => {}
			}
			
		});
		(cancel_token_clone, event_subscriber, prompt_responder)
	}
	
	/// Returns a cancel token and prompt responder
	#[cfg(feature = "gpui")]
	pub fn spawn_singleplayer_gpui(
		self,
		gpui_event_emitter: gpui::Entity<GpuiEventEmitter>,
		app: gpui::AsyncApp
	) -> (CancellationToken, watch::Sender<GameEvent>) {
		let cancel_token = CancellationToken::new();
		let cancel_token_clone = cancel_token.clone();
		// SAFETY:
		// we have exclusive access to self
		let human_idx = unsafe { self.human_idx() };
		let (prompt_responder, response_rx) = watch::channel(GameEvent::Init {idx: human_idx});
		app.spawn(async move |cx| {
			let mut event_emitter = GameEventEmitter(gpui_event_emitter, cx.clone());
			event_emitter.send(GameEvent::Init {idx: human_idx});
			tokio::select! {
				_ = self.run_event_loop(human_idx, cancel_token.clone(), event_emitter, response_rx) => {}
				_ = cancel_token.cancelled() => {}
			}
		}).detach();
		(cancel_token_clone, prompt_responder)
	}
	
	/// ## Safety
	/// No mutable pointers to player type must exist.
	#[cfg(all(feature = "client", not(feature = "test-dep")))]
	unsafe fn human_idx(&self) -> u8 {
		self
			.setup
			.players
			.iter()
			.position(|p| matches!(
				// SAFETY:
				// No mutable pointers to player type must exist.
				unsafe {&*p.player_type.get()},
				PlayerType::SingleplayerSelf {..})
			)
			.unwrap() as u8
	}
	
	/// ## Safety
	/// Not actually unsafe
	#[cfg(all(feature = "client", feature = "test-dep"))]
	unsafe fn human_idx(&self) -> u8 {
		// there is no human player in tests
		u8::MAX
	}
	
	#[cfg(feature = "client")]
	async fn run_event_loop(
		mut self,
		human_idx: u8,
		cancel_token: CancellationToken,
		#[cfg(feature = "gpui")]
		mut event_emitter: GameEventEmitter,
		#[cfg(not(feature = "gpui"))]
		event_emitter: GameEventEmitter,
		mut response_rx: watch::Receiver<GameEvent>
	) {
		let mut loop_state = GameLoopState::DealCards;
		loop {
			// Tip: `continue` can be used to not increment the turn index.
			match loop_state {
				GameLoopState::DealCards => {
					self.deal_cards();
					#[cfg(not(feature = "test-dep"))]
					event_emitter.send(GameEvent::DealCards {
						hand: self.setup.players[human_idx].hand.clone(),
						idx: self.turn_idx
					});
					loop_state = GameLoopState::Bidding;
					// no continue because it is currently the dealer's turn and we want the next player to start bidding.
				}
				GameLoopState::Bidding => {
					let mut bid_err = None;
					let bid = if self.turn_idx == human_idx {
						loop {
							event_emitter.send(GameEvent::PromptBid(bid_err));
							response_rx.changed().await.unwrap();
							let GameEvent::PlayerBid { bid, .. } = *response_rx.borrow_and_update() else { continue; };
							if self.validate_bid(bid) {
								break bid
							}
							bid_err = Some(Error::BadBid);
						}
					} else {
						self.ai_bid()
					};
					// It feels redundant to let the player know what they just bid,
					// but this tells the player that their bid was valid.
					#[cfg(not(feature = "test-dep"))]
					event_emitter.send(GameEvent::PlayerBid { idx: self.turn_idx, bid });
					if self.player_bid(bid, &mut loop_state) {
						continue;
					}
				}
				GameLoopState::Playing => {
					let mut card_err = None;
					let card = if self.turn_idx == human_idx {
						loop {
							event_emitter.send(GameEvent::PromptCard(card_err));
							response_rx.changed().await.unwrap();
							let GameEvent::PlayerPlayedCard { card, .. } = *response_rx.borrow_and_update() else { continue; };
							if self.validate_card(card) {
								break card
							}
							card_err = Some(Error::BadCard);
						}
					} else {
						self.ai_play_card()
					};
					// It feels redundant to let the player know which card they played,
					// but this tells the player that their card was valid.
					#[cfg(not(feature = "test-dep"))]
					event_emitter.send(GameEvent::PlayerPlayedCard { idx: self.turn_idx, card });
					match self.player_played_card(card) {
						PlayedCardResult::RoundOver(round_scores) => {
							#[cfg(not(feature = "test-dep"))]
							event_emitter.send(GameEvent::RoundScores(round_scores));
							loop_state = GameLoopState::RoundOver;
							continue;
						}
						PlayedCardResult::TurnOver => continue,
						PlayedCardResult::None => {}
					}
				}
				GameLoopState::RoundOver => {
					if self.team_won() {
						loop_state = GameLoopState::GameOver;
					} else {
						loop_state = GameLoopState::DealCards;
					}
					continue;
				}
				GameLoopState::GameOver => {
					#[cfg(not(feature = "test-dep"))]
					event_emitter.send(GameEvent::GameOver);
					cancel_token.cancel();
					info!("Game over");
					return;
				}
			}
			self.turn_idx = self.next_turn();
		}
	}
	
	fn next_turn(&self) -> u8 {
		let turn_idx = self.turn_idx + 1;
		if turn_idx == self.setup.players.len() {
			0
		} else {
			turn_idx
		}
	}
	
	fn deal_cards(&mut self) {
		// reset round-specific data
		self.turn_idx = self.dealer_idx;
		self.highest_card = Card::zero();
		self.highest_bid = Bid::Pass;
		
		// collect cards and shuffle
		self.deck.append(&mut self.pile);
		for player in self.setup.players.iter_mut() {
			self.deck.append(&mut player.hand);
		}
		for team in self.teams.iter_mut() {
			self.deck.append(&mut team.pile);
		}
		self.deck.as_slice_mut().shuffle(&mut rand::rng());
		
		// deal cards
		for player in self.setup.players.iter_mut() {
			player.hand = self.deck.split();
		}
		info!("player {} dealt the cards", self.turn_idx);
		debug_assert_eq!(self.deck.len(), DECK_SIZE as u8 - (self.setup.players.len() * CARDS_PER_PLAYER as u8));
	}
	
	/// Algorithm for deciding how the AI should bid
	/// Right now this is just an arbitrary mapping,
	/// and will eventually be replaced with a better AI
	fn ai_eval_bid(&self) -> (Suit, u8) {
		let player = &self.setup.players[self.turn_idx];
		// [Hearts, Clubs, Diamonds, Spades]
		let mut suit_points = [0u8; 4];
		for card in player.hand.iter() {
			suit_points[card.suit as usize] += match card.number {
				2 => 10,
				3 => {
					if self.setup.players.len() == MAX_PLAYERS as u8 {
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
				n => unreachable_unchecked!("card number should not exist: {n}"),
			}
		}
		let (index, score) = suit_points
			.into_iter()
			.enumerate()
			.max_by(|(_, num1), (_, num2)| num1.cmp(num2))
			.unwrap();
		// SAFETY:
		// Suit is repr(u8) and the array length is the same as the number of suits.
		(unsafe { std::mem::transmute::<u8, Suit>(index as u8) }, score)
	}
	
	/// Uses a card-scoring system, the turn order, and the game score to determine the bid
	fn ai_bid(&self) -> Bid {
		let (_, points) = self.ai_eval_bid();
		let mut bid = match points {
			..=44 => 0,     // pass
			45..=99 => 1,   // minimum play out of desperation
			100..=139 => 2, // A + one more good card
			140..=179 => 3, // A + face card or two + maybe one more
			180..=219 => 4, // A + K + J + one more good card
			220.. if self.setup.dealer_override => 5,  // A + K + Q + J + one more good card
			220.. => 4, // A + K + Q + J + one more good card
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
			let player = &self.setup.players[self.turn_idx];
			if player.team_id != index as u8
				&& lead_team.score as i16 >= self.setup.max_score as i16 - 4
				&& self.setup.max_score as i16 - 4 > self.teams[player.team_id].score as i16
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
	fn ai_play_card(&self) -> Card {
		let player = &self.setup.players[self.turn_idx];
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
	
	/// Returns true if valid, false if invalid
	fn validate_bid(&self, bid: Bid) -> bool {
		bid == Bid::Pass || (bid > self.highest_bid && (bid != Bid::Override || self.setup.dealer_override))
	}
	
	/// Returns true if valid, false if invalid
	fn validate_card(&self, card: Card) -> bool {
		let has_card = self.setup.players[self.turn_idx].hand.iter().any(|c| *c == card);
		let can_follow = self.setup.players[self.turn_idx].can_follow(self.lead);
		let is_owner = self.owner_idx == self.turn_idx;
		let is_lead = card.suit == self.lead;
		let is_trump = card.suit == self.trump;
		has_card && (is_lead || is_trump || is_owner || !can_follow)
	}
	
	/// The bid must have already been validated because it will be considered the new highest bid.
	///
	/// Returns true if bidding should end, and the turn index should not be incremented following this function.
	/// Returns false if the next player needs to bid.
	fn player_bid(&mut self, bid: Bid, loop_state: &mut GameLoopState) -> bool {
		debug_assert!(self.validate_bid(bid));
		info!("player {} bid {}", self.turn_idx, bid.to_points());
		if *loop_state == GameLoopState::GameOver { return true; }
		if bid != Bid::Pass {
			self.highest_bid = bid;
			self.highest_bidder_idx = self.turn_idx;
			if (bid == Bid::Four && !self.setup.dealer_override)
				|| (bid == Bid::Override && self.setup.dealer_override) { return true; }
		}
		// if all players have bid, end the turn...
		let should_end = self.turn_idx == self.dealer_idx;
		if should_end {
			if self.highest_bid == Bid::Pass {
				*loop_state = GameLoopState::DealCards;
			} else {
				*loop_state = GameLoopState::Playing;
				self.turn_idx = self.highest_bidder_idx;
				self.owner_idx = self.highest_bidder_idx;
			}
		}
		should_end
	}
	
	/// The card must have already been validated because it will be taken from the player's hand and moved to the pile.
	///
	/// Returns Some if card playing for the round should end, and the turn index should not be incremented following this function.
	/// Returns None if the next player needs to play a card.
	fn player_played_card(&mut self, card: Card) -> PlayedCardResult {
		debug_assert!(self.validate_card(card));
		info!("player {} played the {card:?}", self.turn_idx);
		self.setup.players[self.turn_idx].hand.remove_item(card);
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
		if self.pile.len() == self.setup.players.len() {
			self.turn_idx = self.owner_idx;
			self.turn_num += 1;
			// The winning team takes the pile after every turn.
			self.teams[self.setup.players[self.owner_idx].team_id].pile.append(&mut self.pile);
			if self.turn_num == CARDS_PER_PLAYER as u8 {
				// if all cards have been played, end the round
				self.turn_num = 0;
				self.round_num += 1;
				self.dealer_idx += 1;
				if self.dealer_idx == self.setup.players.len() {
					self.dealer_idx = 0;
				}
				
				// calculate round scores
				let mut high_winner_team_id = 0;
				let mut low_winner_team_id = 0;
				let mut jack_winner_team_id = None;
				let mut game_winner_team_id = 0;
				let mut high = 0;
				let mut low = 255;
				let mut highest_game = 0u8;
				let mut second_highest_game = 0;
				let mut round_team_game_scores = SmallVec::new();
				for (team_id, team) in self.teams.iter().enumerate() {
					let mut game = 0;
					for card in team.pile.iter() {
						game += match card.number {
							JACK => 1,
							QUEEN => 2,
							KING => 3,
							ACE => 4,
							10 => 10,
							_ => 0
						};
						if card.suit != self.trump { continue; }
						if card.number > high {
							high_winner_team_id = team_id as u8;
							high = card.number;
						} else if card.number < low {
							low_winner_team_id = team_id as u8;
							low = card.number;
						}
						if card.number == JACK {
							debug_assert_eq!(jack_winner_team_id, None);
							jack_winner_team_id = Some(team_id as u8);
						}
					}
					round_team_game_scores.push(game);
					if game > highest_game {
						game_winner_team_id = team_id as u8;
						second_highest_game = highest_game;
						highest_game = game;
					}
				}
				let mut bidder_team_points = 0;
				let bidder_team_id = self.setup.players[self.highest_bidder_idx].team_id;
				if high_winner_team_id == bidder_team_id {
					bidder_team_points += 1;
				} else {
					self.teams[high_winner_team_id].score += 1;
				}
				if low_winner_team_id == bidder_team_id {
					bidder_team_points += 1;
				} else {
					self.teams[low_winner_team_id].score += 1;
				}
				if let Some(jack_team_id) = jack_winner_team_id {
					if jack_team_id == bidder_team_id {
						bidder_team_points += 1;
					} else {
						self.teams[jack_team_id].score += 1;
					}
				}
				if game_winner_team_id == bidder_team_id {
					bidder_team_points += 1;
				} else {
					self.teams[game_winner_team_id].score += 1;
				}
				if bidder_team_points >= self.highest_bid.to_points() {
					self.teams[bidder_team_id].score += bidder_team_points;
				} else {
					self.teams[bidder_team_id].score -= self.highest_bid.to_points();
				}
				let round_scores = RoundScores {
					high_winner_team_id,
					low_winner_team_id,
					jack_winner_team_id,
					game_winner_team_id: if highest_game == second_highest_game { None } else { Some(game_winner_team_id) },
					round_team_game_scores,
					total_team_scores: self.teams.iter().map(|team| team.score).collect(),
				};
				info!("Round ended - {round_scores:?}");
				return PlayedCardResult::RoundOver(round_scores);
			}
			return PlayedCardResult::TurnOver;
		}
		PlayedCardResult::None
	}
	
	fn team_won(&self) -> bool {
		let mut highest_score = 0;
		let mut second_highest_score = 0;
		for team in self.teams.iter() {
			if team.score >= highest_score {
				second_highest_score = highest_score;
				highest_score = team.score;
			}
		}
		highest_score as i16 > self.setup.max_score as i16 && (highest_score > second_highest_score + 1 || !self.setup.win_by_2)
	}
}

#[cfg(not(feature = "client"))]
impl GameState {
	pub fn spawn_server(self, reject_tx: mpsc::UnboundedSender<ClientConnection>) -> GameLoopHandle {
		let cancel_token = CancellationToken::new();
		let cancel_token_clone = cancel_token.clone();
		let (rejoin_tx, rejoin_rx) = mpsc::channel(MAX_PLAYERS);
		GameLoopHandle {
			cancel_token,
			rejoin_tx,
			task: tokio::spawn(self.run_event_loop(cancel_token_clone, rejoin_rx, reject_tx))
		}
	}
	
	fn collect_players(self) -> SmallVec<ClientConnection, MAX_PLAYERS> {
		self.setup
			.players
			.into_iter()
			.filter_map(|mut p| match std::mem::replace(p.player_type.get_mut(), PlayerType::Ai {account_id: 0}) {
				PlayerType::HumanConnection {conn} => Some(conn),
				_ => None,
			})
			.collect()
	}
	
	/// Makes a copy of the game loop without sensitive data like the player's hand or the deck.
	/// This should be called after a player makes a bid or plays a card,
	/// but before the game state is updated with that bid or card.
	fn snapshot(&self) -> Self {
		Self {
			// setup and deck are not fully cloned to prevent the user from accessing protected data
			setup: self.setup.clone(),
			deck: SmallVec::new(),
			// the rest are fully cloned
			teams: self.teams.clone(),
			pile: self.pile.clone(),
			trump: self.trump,
			lead: self.lead,
			owner_idx: self.owner_idx,
			highest_card: self.highest_card,
			highest_bidder_idx: self.highest_bidder_idx,
			highest_bid: self.highest_bid,
			turn_idx: self.turn_idx,
			turn_num: self.turn_num,
			dealer_idx: self.dealer_idx,
			round_num: self.round_num,
		}
	}
	
	async fn run_event_loop(
		mut self,
		cancel_token: CancellationToken,
		mut rejoin_rx: mpsc::Receiver<ClientConnection>,
		reject_tx: mpsc::UnboundedSender<ClientConnection>,
	) -> SmallVec<ClientConnection, MAX_PLAYERS> {
		let mut loop_state = GameLoopState::DealCards;
		let mut idx_payloads = (0..self.setup.players.len())
			.map(|idx| Payload::GameEvent(GameEvent::Init {idx}))
			.collect();
		self.notify_all_unique(&mut loop_state, &mut rejoin_rx, &reject_tx, &mut idx_payloads).await;
		loop {
			if cancel_token.is_cancelled() {
				return self.collect_players();
			}
			// See if there are any players that have rejoined.
			// This is called twice: once here at the top of the loop,
			// and whenever we send any messages to a HumanConnection player.
			// It's done when sending messages because the player might take many seconds
			// or minutes to finish a turn, and we want the rejoining player to instantly join without delay.
			// It's also done here because it otherwise doesn't get run when it's the Ai's turn.
			let rejoined = rejoin_rx.try_recv().ok();
			if rejoined.is_some() {
				// SAFETY:
				// we have exclusive access to self
				let mut rejoin_prep = unsafe { self.prepare_rejoin() };
				// SAFETY:
				// We have exclusive access to self
				unsafe { self.rejoin_player(&mut rejoin_prep, &reject_tx, rejoined).await; }
				self.send_all_player_rejoined(rejoin_prep, &mut loop_state, &mut rejoin_rx, &reject_tx).await;
			}
			// Tip: `continue` can be used to not increment the turn index.
			match loop_state {
				GameLoopState::DealCards => {
					self.deal_cards();
					loop_state = GameLoopState::Bidding;
					let mut payloads = self
						.setup
						.players
						.iter()
						.map(|p|
							Payload::GameEvent(GameEvent::DealCards {hand: p.hand.clone(), idx: self.dealer_idx}),
						)
						.collect();
					self.notify_all_unique(&mut loop_state, &mut rejoin_rx, &reject_tx, &mut payloads).await;
					// no continue because it is currently the dealer's turn and we want the next player to start bidding.
				}
				GameLoopState::Bidding => {
					let bid = self.prompt(&mut loop_state, &mut rejoin_rx, &reject_tx).await;
					if self.player_bid(bid, &mut loop_state) {
						continue;
					}
				}
				GameLoopState::Playing => {
					let card = self.prompt(&mut loop_state, &mut rejoin_rx, &reject_tx).await;
					match self.player_played_card(card) {
						PlayedCardResult::RoundOver(round_scores) => {
							loop_state = GameLoopState::RoundOver;
							self.notify_all(
								&mut loop_state,
								&mut rejoin_rx,
								&reject_tx,
								Payload::GameEvent(GameEvent::RoundScores(round_scores))
							).await;
							continue;
						}
						PlayedCardResult::TurnOver => continue,
						PlayedCardResult::None => {}
					}
				}
				GameLoopState::RoundOver => {
					if self.team_won() {
						loop_state = GameLoopState::GameOver;
					} else {
						loop_state = GameLoopState::DealCards;
					}
					continue;
				}
				GameLoopState::GameOver => {
					self.notify_all(
						&mut loop_state,
						&mut rejoin_rx,
						&reject_tx,
						Payload::GameEvent(GameEvent::GameOver)
					).await;
					info!("Game over");
					return self.collect_players();
				}
			}
			self.turn_idx = self.next_turn();
		}
	}
	
	/// This function can be called for Ai and HumanConnection players
	async fn prompt<T: PromptResult>(
		&mut self,
		loop_state: &mut GameLoopState,
		join_rx: &mut mpsc::Receiver<ClientConnection>,
		reject_tx: &mpsc::UnboundedSender<ClientConnection>,
	) -> T {
		// SAFETY:
		// we have exclusive access to self
		if matches!(unsafe { &*self.setup.players[self.turn_idx].player_type.get() }, PlayerType::Ai{..}) {
			return T::ai_play(self);
		}
		// SAFETY:
		// Ai was checked so this is HumanConnection.
		// send_prompt must only push self.turn_idx to disconnected_idxs iff that player disconnects,
		// and it preserves the ClientConnection object in that player to be later properly disconnected.
		unsafe {self.send_with_join(
			loop_state,
			join_rx,
			reject_tx,
			async move |this, disconnected_idxs| this.send_prompt(disconnected_idxs).await
		).await}
	}
	
	async fn notify_all(
		&mut self,
		loop_state: &mut GameLoopState,
		join_rx: &mut mpsc::Receiver<ClientConnection>,
		reject_tx: &mpsc::UnboundedSender<ClientConnection>,
		payload: Payload,
	) {
		// u8::MAX will never be a valid index, so this just skips a player that doesn't exist anyway
		const {assert!(MAX_PLAYERS > u8::MAX as usize);}
		self.notify_all_but(u8::MAX, loop_state, join_rx, reject_tx, payload).await
	}
	
	async fn notify_all_but(
		&mut self,
		idx: u8,
		loop_state: &mut GameLoopState,
		join_rx: &mut mpsc::Receiver<ClientConnection>,
		reject_tx: &mpsc::UnboundedSender<ClientConnection>,
		payload: Payload,
	) {
		let payload_ref = &payload;
		// SAFETY:
		// Ais are skipped so these only send to HumanConnection players.
		// send_payload must only push player_idx to disconnected_idxs iff that player disconnects,
		// and it preserves the ClientConnection object in that player to be later properly disconnected.
		// Each player it sends to is a unique player in the array and disjoint in memory.
		unsafe {self.send_with_join(
			loop_state,
			join_rx,
			reject_tx,
			async move |this, disconnected_idxs| {
				futures::future::join_all(
					(0..this.setup.players.len())
						.filter_map(|i| if i != idx {
							Some(async move {
								// each future needs its own small vec because this future can't capture variables by mutable reference
								let mut disconn_idxs = SmallVec::new();
								this.send_payload(&mut disconn_idxs, i, payload_ref.clone()).await;
								disconn_idxs
							})
						} else { None })
				).await
					.into_iter()
					.for_each(|disconn_idxs|
						disconn_idxs.into_iter().for_each(|i|
							if !disconnected_idxs.iter().any(|ii| i == *ii) {
								debug_assert!(disconnected_idxs.len() < disconnected_idxs.cap());
								disconnected_idxs.push(i);
							}
						)
					);
			}
		).await}
	}
	
	async fn notify_all_unique(
		&mut self,
		loop_state: &mut GameLoopState,
		join_rx: &mut mpsc::Receiver<ClientConnection>,
		reject_tx: &mpsc::UnboundedSender<ClientConnection>,
		payloads: &mut SmallVec<Payload, MAX_PLAYERS>,
	) {
		// SAFETY:
		// Ais are skipped so these only send to HumanConnection players.
		// send_payload must only push player_idx to disconnected_idxs iff that player disconnects,
		// and it preserves the ClientConnection object in that player to be later properly disconnected.
		// Each player it sends to is a unique player in the array and disjoint in memory.
		unsafe {self.send_with_join(
			loop_state,
			join_rx,
			reject_tx,
			async move |this, disconnected_idxs| {
				futures::future::join_all(
					(0..this.setup.players.len())
						.map(|i| {
							let payload = std::mem::replace(&mut payloads[i], Payload::Ack);
							async move {
								// each future needs its own small vec because this future can't capture variables by mutable reference
								let mut disconn_idxs = SmallVec::new();
								this.send_payload(&mut disconn_idxs, i, payload).await;
								disconn_idxs
							}
						})
				).await
					.into_iter()
					.for_each(|disconn_idxs|
						disconn_idxs.into_iter().for_each(|i|
							if !disconnected_idxs.iter().any(|ii| i == *ii) {
								debug_assert!(disconnected_idxs.len() < disconnected_idxs.cap());
								disconnected_idxs.push(i);
							}
						)
					);
			}
		).await}
	}
	
	/// ## Notice
	/// This function can be recursive.
	/// If a player rejoins or disconnects, this function will notify all other players.
	/// If nobody rejoins or disconnects, or if all players have disconnected, then it doesn't.
	/// ## Safety
	/// When async_send_fn is called, a player might be disconnected.
	/// The index of that player needs to be pushed to the &mut SmallVec parameter.
	async unsafe fn send_with_join<T: PromptResult>(
		&mut self,
		loop_state: &mut GameLoopState,
		join_rx: &mut mpsc::Receiver<ClientConnection>,
		reject_tx: &mpsc::UnboundedSender<ClientConnection>,
		async_send_fn: impl AsyncFnOnce(&Self, &mut SmallVec<u8, MAX_PLAYERS>) -> T,
	) -> T {
		// SAFETY:
		// we have exclusive access to self
		let mut rejoin_prep = unsafe { self.prepare_rejoin() };
		let mut disconnected_idxs = SmallVec::new();
		
		// The following future is what actually sends the prompt,
		// verifies output, and implements retry logic and error handling for the response
		let output = {
			let future = async_send_fn(self, &mut disconnected_idxs);
			// pin the future so it doesn't get dropped in the select loop
			tokio::pin!(future);
			// by doing select in a loop, we can immediately handle a player trying to rejoin
			// without dropping the future. When a player rejoins, we just go to the next iteration and keep going.
			// When the future completes, we break from the loop with the result
			loop {
				tokio::select! {
					output = &mut future => break output,
					// SAFETY:
					// see safety of Self::rejoin_player
					join_res = join_rx.recv() => unsafe {self.rejoin_player(
						&mut rejoin_prep,
						reject_tx,
						join_res,
					).await},
				}
			}
		};
		self.send_all_player_rejoined(rejoin_prep, loop_state, join_rx, reject_tx).await;
		for i in disconnected_idxs.into_iter() {
			// SAFETY:
			// We have exclusive access to self.
			// disconnected_idxs determines which players must be disconnected.
			// The functions that populate it must guarantee that the players at these indices are HumanConnection.
			unsafe {
				self.disconnect_player(loop_state, join_rx, reject_tx, i).await
			}
		}
		output
	}
	
	async fn send_all_player_rejoined(
		&mut self,
		rejoin_prep: RejoinPrep,
		loop_state: &mut GameLoopState,
		join_rx: &mut mpsc::Receiver<ClientConnection>,
		reject_tx: &mpsc::UnboundedSender<ClientConnection>,
	) {
		for i in rejoin_prep.rejoined_idxs.into_iter() {
			Box::pin(self.notify_all(
				loop_state,
				join_rx,
				reject_tx,
				Payload::Reconnected {
					player_idx: i,
					// SAFETY:
					// We have exclusive access to self
					username: unsafe { &*self.setup.players[i].player_type.get() }.username().to_string()
				},
			)).await
		}
	}
	
	// ## Safety
	// The returned RejoinPrep is safe to use.
	// Don't hold mutable pointers to players while this function is called.
	unsafe fn prepare_rejoin(&self) -> RejoinPrep {
		RejoinPrep {
			// create copies of AI account ids and hands and game state snapshot
			// because they're cheaper than synchronizing shared state
			// SAFETY:
			// see above
			account_ids: self.setup.players.iter().map(|p| match unsafe { &*p.player_type.get() } {
				PlayerType::Ai {account_id} => *account_id,
				_ => 0,
			}).collect(),
			// SAFETY:
			// see above
			hands: self.setup.players.iter().map(|p| match unsafe { &*p.player_type.get() } {
				PlayerType::Ai {account_id} if *account_id != 0 => p.hand.clone(),
				#[cfg(debug_assertions)]
				_ => {
					let mut v = SmallVec::new();
					v.push(Card::zero()); // this tell us when this hand is invalid
					v
				}
				#[cfg(not(debug_assertions))]
				_ => SmallVec::new(),
			}).collect(),
			rejoined_idxs: SmallVec::new(),
			// The hand will be set correctly inside the rejoin_player function
			snapshot_payload: Payload::Snapshot(Box::new(self.snapshot()), SmallVec::new()),
		}
	}
	
	/// Attempts to let the client rejoin the game.
	/// If the rejoin is unsuccessful, the connection is rejected and sent back to the server.
	/// ## Safety
	/// account_ids must be 0 for each index where the player is HumanConnection.
	/// This function is called in a select loop with some other future that takes &Self.
	/// That future holds *mut PlayerType across .await boundaries for players that are PlayerType::HumanConnection,
	/// so this function must not in any way access HumanConnection players.
	/// This is why we have the account_ids array to tell us which players are guaranteed to be Ai
	/// (`if account_ids[i] == conn.account_id`).
	/// Since the pointers in this unsafe block are not held across .await boundaries,
	/// it is safe for the other future to mutably access any player_type, even Ai.
	async unsafe fn rejoin_player(
		&self,
		rejoin_prep: &mut RejoinPrep,
		reject_tx: &mpsc::UnboundedSender<ClientConnection>,
		conn: Option<ClientConnection>,
	) {
		let Some(mut conn) = conn else {
			error!("rejoin channel closed unexpectedly");
			return;
		};
		debug_assert_ne!(conn.account_id, 0);
		debug_assert_eq!(rejoin_prep.account_ids.len(), rejoin_prep.hands.len());
		let Some((i, account_id)) = rejoin_prep.account_ids.iter_mut().enumerate().find(|(_, id)| **id == conn.account_id) else {
			reject_tx.send(conn).unwrap();
			return;
		};
		let Payload::Snapshot(_, ref mut ssp_hand) = rejoin_prep.snapshot_payload else {
			unreachable!("snapshot_ssp should be a snapshot")
		};
		std::mem::swap(ssp_hand, &mut rejoin_prep.hands[i as u8]);
		debug_assert!(ssp_hand.len() != 1 || ssp_hand[0] != Card::zero());
		if let Err(err) = conn.send_expect_ack(0, rejoin_prep.snapshot_payload.clone(), NETWORK_TIMEOUT).await {
			warn!("Failed to rejoin player: {err}");
			if err != Error::Disconnected {
				reject_tx.send(conn).unwrap();
			}
			return;
		}
		*account_id = 0;
		debug_assert!(rejoin_prep.rejoined_idxs.len() < rejoin_prep.rejoined_idxs.cap());
		rejoin_prep.rejoined_idxs.push(i as u8);
		// SAFETY:
		// see above safety message
		// we can use ptr.write because the Ai variant doesn't need to call drop
		unsafe {
			let ptr = self.setup.players[i as u8].player_type.get();
			std::debug_assert_matches!(&*ptr, PlayerType::Ai{..});
			ptr.write(PlayerType::HumanConnection { conn });
		}
	}
	
	/// ## Safety
	/// Player at player_idx must be HumanConnection.
	async unsafe fn disconnect_player(
		&mut self,
		loop_state: &mut GameLoopState,
		join_rx: &mut mpsc::Receiver<ClientConnection>,
		reject_tx: &mpsc::UnboundedSender<ClientConnection>,
		player_idx: u8,
	) {
		warn!("player disconnected");
		{
			let human_conn = self.setup.players[player_idx].player_type.get_mut();
			let account_id = {
				let PlayerType::HumanConnection {conn} = human_conn else {
					unreachable_unchecked!("Player should be human")
				};
				conn.account_id
			};
			*human_conn = PlayerType::Ai {account_id};
		}
		// SAFETY:
		// &mut self gives exclusive access
		if self.setup.players.iter().all(|p| matches!(unsafe {&*p.player_type.get()}, PlayerType::Ai{..})) {
			// all humans have disconnected
			*loop_state = GameLoopState::GameOver;
			return;
		}
		Box::pin(self.notify_all(
			loop_state,
			join_rx,
			reject_tx,
			Payload::Disconnected {player_idx},
		)).await;
	}
	
	/// Sends a prompt to the player of the current turn.
	/// Returns the response from the player.
	/// If there is an error that requires the player to disconnect,
	/// this function returns true.
	/// Otherwise, it returns false.
	/// ## Safety
	/// Player at self.turn_idx must be HumanConnection.
	/// Player at self.turn_idx must not be accessed anywhere else.
	/// This function should be used as the future in send_with_join.
	async unsafe fn send_prompt<T: PromptResult>(&self, disconnected_idxs: &mut SmallVec<u8, MAX_PLAYERS>) -> T {
		const { assert!(size_of::<T>() != 0, "Prompt function shouldn't be used on `Unit`"); }
		debug_assert!(disconnected_idxs.len() < disconnected_idxs.cap());
		// SAFETY:
		// The future that receives rejoining players must not access any player_type that is HumanConnection
		let PlayerType::HumanConnection{conn} = (unsafe { &mut *self.setup.players[self.turn_idx].player_type.get() }) else {
			unreachable_unchecked!("Player should be human")
		};
		let mut chain_id = 0;
		let timeout_res = tokio::time::timeout(Duration::from_secs(self.setup.turn_time as u64), async {
			loop {
				let prompt_res = T::send_prompt(conn, &mut chain_id).await?;
				if prompt_res.validate(self) {
					return Ok(prompt_res);
				}
			}
		}).await;
		let (res, response) = match timeout_res {
			Ok(Ok(prompt_res)) => (prompt_res, Payload::Ack),
			Ok(Err(Error::Disconnected)) => {
				disconnected_idxs.push(self.turn_idx);
				return T::ai_play(self);
			}
			Ok(Err(err)) => (T::ai_play(self), Payload::Err(err)),
			Err(_) => {
				warn!("player response timed out");
				conn.timeout_strikes += 1;
				let prompt_res = T::ai_play(self);
				if conn.timeout_strikes == 3 {
					disconnected_idxs.push(self.turn_idx);
					return prompt_res;
				}
				(prompt_res, Payload::Err(Error::TimedOut))
			}
		};
		if let Err(err) = conn.send_expect_ack(0, response, NETWORK_TIMEOUT).await {
			if err == Error::Disconnected {
				disconnected_idxs.push(self.turn_idx);
			}
			warn!("Prompt ack failed. Error: {err}");
			return T::ai_play(self);
		}
		res
	}
	
	/// Sends a payload to the player at player_idx.
	/// If there is an error that requires the player to disconnect,
	/// this function returns true.
	/// Otherwise, it returns false.
	/// ## Safety
	/// Player at player_idx must be HumanConnection.
	/// Player at player_idx must not be accessed anywhere else.
	/// This function should be used as the future in send_with_join.
	async unsafe fn send_payload(&self, disconnected_idxs: &mut SmallVec<u8, MAX_PLAYERS>, player_idx: u8, payload: Payload) {
		debug_assert!(disconnected_idxs.len() < disconnected_idxs.cap());
		// SAFETY:
		// The future that receives rejoining players must not access any player_type that is HumanConnection
		let PlayerType::HumanConnection{conn} = (unsafe { &mut *self.setup.players[player_idx].player_type.get() }) else {
			unreachable_unchecked!("Player should be human");
		};
		if let Err(err) = conn.send_expect_ack(0, payload, NETWORK_TIMEOUT).await {
			if err == Error::Disconnected {
				disconnected_idxs.push(self.turn_idx);
			}
			warn!("Prompt ack failed. Error: {err}");
		}
	}
}

#[cfg(not(feature = "client"))]
trait PromptResult: Copy {
	async fn send_prompt(
		conn: &mut ClientConnection,
		chain_id: &mut u32,
	) -> Result<Self, Error>;
	fn validate(self, game_state: &GameState) -> bool;
	fn ai_play(game_state: &GameState) -> Self;
}

#[cfg(not(feature = "client"))]
impl PromptResult for Bid {
	#[inline(always)]
	async fn send_prompt(
		conn: &mut ClientConnection,
		chain_id: &mut u32,
	) -> Result<Self, Error> {
		conn.send_prompt_bid(chain_id).await
	}
	#[inline(always)]
	fn validate(self, game_state: &GameState) -> bool {
		game_state.validate_bid(self)
	}
	#[inline(always)]
	fn ai_play(game_state: &GameState) -> Self {
		game_state.ai_bid()
	}
}

#[cfg(not(feature = "client"))]
impl PromptResult for Card {
	#[inline(always)]
	async fn send_prompt(
		conn: &mut ClientConnection,
		chain_id: &mut u32,
	) -> Result<Self, Error> {
		conn.send_prompt_card(chain_id).await
	}
	#[inline(always)]
	fn validate(self, game_state: &GameState) -> bool {
		game_state.validate_card(self)
	}
	#[inline(always)]
	fn ai_play(game_state: &GameState) -> Self {
		game_state.ai_play_card()
	}
}

#[cfg(not(feature = "client"))]
impl PromptResult for () {
	#[inline(always)]
	async fn send_prompt(
		_conn: &mut ClientConnection,
		_chain_id: &mut u32,
	) -> Result<Self, Error> {Ok(())}
	#[inline(always)]
	fn validate(self, _game_state: &GameState) -> bool {true}
	#[inline(always)]
	fn ai_play(_game_state: &GameState) -> Self {}
}

#[cfg(not(feature = "client"))]
struct RejoinPrep {
	account_ids: SmallVec<u32, MAX_PLAYERS>,
	hands: SmallVec<SmallVec<Card, CARDS_PER_PLAYER>, MAX_PLAYERS>,
	rejoined_idxs: SmallVec<u8, MAX_PLAYERS>,
	snapshot_payload: Payload,
}

enum PlayedCardResult {
	RoundOver(RoundScores),
	TurnOver,
	None,
}