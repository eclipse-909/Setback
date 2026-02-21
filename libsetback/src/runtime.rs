use {
	crate::*,
	rand::{Rng, rng, seq::SliceRandom},
	serde::{Deserialize, Serialize},
	std::{collections::HashMap, time::Duration},
	tokio_util::sync::CancellationToken
};

#[derive(Serialize, Deserialize, Debug)]
pub struct GameLoop {
	pub setup: GameSetup,
	pub state: GameState,
	pub teams: SmallVec<Team, MAX_PLAYERS>,
	pub deck: SmallVec<Card, DECK_SIZE>,
	pub pile: SmallVec<Card, PILE_SIZE>,
	pub trump: Suit,
	pub lead: Suit,
	///The index of the player who owns the pile. Played the highest_card.
	pub owner_idx: u8,
	///the card that was played to take ownership of the pile. Played by the pile_owner
	pub highest_card: Card,
	///index of the player who won the bid. Made the highes_bid.
	pub highest_bidder_idx: u8,
	///the highest bid so far
	pub highest_bid: Bid,
	///the index of the player whose turn it is
	pub turn_idx: u8,
	///0..6 excluding the bidding round
	pub round_num: u8,
	///the index of the player who led this turn
	pub lead_idx: u8,
	pub dealer_idx: u8,
	///A sequence is 6 rounds
	pub sequence_num: u8,
}

/// This enum isn't strictly necessary, but it makes the code a little more readable and reduces if statements.
#[repr(u8)]
#[derive(Serialize, Deserialize, Debug)]
pub enum GameState {
	DealingCards,
	Bidding,
	Playing,
	EndTurn,
	EndRound,
}

pub struct GameLoopHandle {
	pub cancel_token: CancellationToken,
	pub task: JoinHandle<SmallVec<ClientConnection, MAX_PLAYERS>>,
}

impl GameLoop {
	#[cfg(not(feature = "gpui"))]
	fn spawn_singleplayer(mut self) -> CancellationToken {
		let cancel_token = CancellationToken::new();
		let cancel_token_clone = cancel_token.clone();
		tokio::spawn(async move {
			tokio::select! {
				_ = self.start() => {}
				_ = cancel_token.cancelled() => {}
			}
		});
		cancel_token_clone
	}

	#[cfg(feature = "gpui")]
	fn spawn_singleplayer(mut self, app: gpui::AsyncApp) -> CancellationToken {
		let cancel_token = CancellationToken::new();
		let cancel_token_clone = cancel_token.clone();
		app.spawn(async move |app| {
			tokio::select! {
				_ = self.start(app) => {}
				_ = cancel_token.cancelled() => {}
			}
		});
		cancel_token_clone
	}

	#[cfg(not(feature = "client"))]
	fn spawn_server(mut self) -> GameLoopHandle {
		let cancel_token = CancellationToken::new();
		let cancel_token_clone = cancel_token.clone();
		GameLoopHandle {cancel_token, task: tokio::spawn(async move {
			tokio::select! {
				_ = self.start() => {}
				_ = cancel_token_clone.cancelled() => {}
			}
			self.setup
				.players
				.into_iter()
				.filter_map(|p| match p.player_type {
					PlayerType::HumanConnection {conn} => Some(conn),
					_ => None,
				})
				.collect()
		})}
	}

	async fn start(&mut self, #[cfg(feature = "gpui")] app: &mut gpui::AsyncApp) {
		loop {
			// Tip: `continue` is used to not increment the turn index
			match self.state {
				GameState::DealingCards => {
					//reset round-specific data
					self.turn_idx = self.dealer_idx;
					self.highest_card = Card {
						suit: Suit::Hearts,
						number: 0,
					};
					self.highest_bid = Bid::Pass;

					//collect cards and shuffle
					self.deck.append(&mut self.pile);
					for player in self.setup.players.iter_mut() {
						self.deck.append(&mut player.hand);
					}
					for team in self.teams.iter_mut() {
						self.deck.append(&mut team.pile);
					}
					self.deck.as_slice_mut().shuffle(&mut rand::rng());

					//deal cards
					for player in self.setup.players.iter_mut() {
						player.hand = self.deck.split();
					}
					debug_assert_eq!(self.deck.len(), DECK_SIZE as u8 - (self.setup.players.len() * CARDS_PER_PLAYER as u8));
					for i in ..self.setup.players.len() {
						_ = self.send_game_event(
							i as u8,
							GameEvent::DealCards {hand: self.setup.players[i].hand.clone()},
							#[cfg(feature = "gpui")] app
						).await;
					}
					self.state = GameState::Bidding;
				}
				GameState::Bidding => {
					// let amount = 'amount: {
					// 	for _ in 0..RETRY_ATTEMPTS {
					// 		if let Some(GameEvent::PlayerBid { amount, .. }) = self.send_game_event(
					// 			self.turn_idx,
					// 			GameEvent::PromptBid,
					// 			#[cfg(feature = "gpui")] app
					// 		).await {
					// 			break 'amount amount;
					// 		}
					// 	}
					// 	Bid::Pass
					// };
					// let Some(GameEvent::PlayerBid { amount, .. }) = self.send_game_event(
					// 	self.turn_idx,
					// 	GameEvent::PromptBid,
					// 	#[cfg(feature = "gpui")] app
					// ).await else {
					// 	#[cfg(debug_assertions)]
					// 	unreachable!("This function must return a bid response");
					// 	#[cfg(not(debug_assertions))]
					// 	std::hint::unreachable_unchecked();
					// };

					if self.turn_idx == self.dealer_idx {
						if self.highest_bid == Bid::Pass {
							self.state = GameState::StartRound;
						} else {
							self.state = GameState::Playing;
							self.turn_idx = self.highest_bidder_idx;
							self.lead_idx = self.highest_bidder_idx;
						}
						continue;
					}
				}
				GameState::Playing => {}
				GameState::EndTurn => {}
				GameState::EndRound => {}
			}
			self.turn_idx += 1;
			if self.turn_idx >= self.setup.players.len() {
				self.turn_idx = 0;
			}
		}
	}

	async fn send_all(
		&mut self,
		event: GameEvent,
		#[cfg(feature = "gpui")] app: &mut gpui::AsyncApp
	) -> SmallVec<Option<GameEvent>, MAX_PLAYERS> {
		let mut vec = SmallVec::new();
		for p in 0..self.setup.players.len() {
			vec.push(self.send_game_event(p, event.clone(), #[cfg(feature = "gpui")] app).await);
		}
		vec
	}

	/// Send to everyone but the player at the index.
	/// The returning vector will contain None at that index.
	async fn send_all_but(
		&mut self,
		player_idx: u8,
		event: GameEvent,
		#[cfg(feature = "gpui")] app: &mut gpui::AsyncApp
	) -> SmallVec<Option<GameEvent>, MAX_PLAYERS> {
		let mut vec = SmallVec::new();
		for p in 0..player_idx {
			vec.push(self.send_game_event(p, event.clone(), #[cfg(feature = "gpui")] app).await);
		}
		vec.push(None);
		for p in player_idx+1..self.setup.players.len() {
			vec.push(self.send_game_event(p, event.clone(), #[cfg(feature = "gpui")] app).await);
		}
		vec
	}

	async fn send_game_event(
		&mut self,
		player_idx: u8,
		event: GameEvent,
		#[cfg(feature = "gpui")] app: &mut gpui::AsyncApp,
	) -> Option<GameEvent> {
		match self.setup.players[player_idx].player_type {
			// TODO: #[cfg(not(feature = "client"))]
			PlayerType::HumanConnection {ref mut conn} => {
				match conn.send_game_event(event).await {
					Ok(res) => res,
					Err(err) => {
						todo!("Handle error from sending or receiving message")
					}
				}
			}
			#[cfg(feature = "client")]
			PlayerType::SingleplayerSelf {
				ref game_event_emitter,
				ref mut event_rx,
				..
			} => {
				#[cfg(feature = "gpui")]
				game_event_emitter.update(app, |_,cx| cx.emit(event));
				#[cfg(not(feature = "gpui"))]
				game_event_emitter.send(event).await.unwrap();

				event_rx.recv().await.unwrap()
			}
			PlayerType::Ai => {
				match event {
					GameEvent::PromptBid => {
						todo!("AI bid");
						Some(GameEvent::PlayerBid {index: 0, amount: Bid::Pass})
					}
					GameEvent::PromptCard => {
						todo!("AI play card");
						Some(GameEvent::PlayerPlayedCard {index: 0, card: Card {suit: Suit::Hearts, number: ACE}})
					}
					_ => None // AI doesn't care about other events
				}
			}
			_ => None // cannot emit events to non-players
		}
	}

	async fn retry<T>(&mut self, future: impl Future<Output=Option<T>>) -> Option<T> {
		for _ in RETRY_ATTEMPTS {
			if let Some(t) = future.await {return Some(t);}
		}
		None
	}

	async fn timeout<T>(&mut self, default: T, future: impl Future<Output=T>) -> T {
		if self.setup.turn_time == 0 {
			return future.await;
		}
		match tokio::time::timeout(Duration::from_secs(self.setup.turn_time as u64), future).await {
			Ok(t) => t,
			Err(_) => default,
		}
	}

	async fn handle_err(err: Error, player: &mut Player) -> Option<GameEvent> {
		let PlayerType::HumanConnection {conn} = &mut player.player_type else {
			#[cfg(debug_assertions)]
			unreachable!("This error handling is only for human players");
			#[cfg(not(debug_assertions))]
			std::hint::unreachable_unchecked();
		};
		todo!("if err is recoverable, retry one more time, otherwise just kick the client and replace with AI");
		match err {
			Error::Unknown => {},
			Error::NetworkError => {},
			Error::BadRequest => {},
			Error::BadVersion => {},
			Error::BadProtocol => {},
			Error::Unauthenticated => {},
			Error::TimedOut => {},
			Error::ServerError => {},
			Error::ServerDown => {},
			Error::Disconnected => {},
			Error::GamesAtCapacity => {},
			Error::GameAtCapacity => {},
			Error::GameNotFound => {},
			Error::BadBid => {},
			Error::BadCard => {},
			Error::IndexOutOfBounds => {},
			Error::BadPlayerIndex => {},
			Error::BadTeamNumber => {},
			Error::BadEncoding => todo!(),
			Error::DevError => todo!(),
		}
	}

	///Jumps to the next player's turn and increments turn and round data.
	///If Ok, the TurnResult represents what action the caller needs to take immediately after.
	///If Err, the caller needs to handle it, usually be terminating the game.
	pub fn next_turn(&mut self) -> TurnAction {
		if self.turn_num == 1 && self.pile.empty() {
			self.turn_idx = self.highest_bidder_idx;
			return TurnAction::NextTurn;
		}
		if self.turn_num == 0 && self.turn_idx == self.dealer_idx {
			return if self.highest_bid == Bid::Pass {
				//re-deal
				TurnAction::ReDeal
			} else {
				//carry on to the first turn
				self.turn_num = 1;
				self.turn_idx = self.highest_bidder_idx;
				TurnAction::NextTurn
			};
		}
		self.turn_idx += 1;
		if self.turn_idx >= self.setup.players.len() {
			//loop the iteration
			self.turn_idx = 0;
		}
		if self.pile.len() == self.setup.players.len() {
			//when the turn is over
			//give pile to owning team
			self.teams[self.setup.players[self.owner_idx].team_id]
				.pile
				.append(&mut self.pile);
			self.turn_idx = self.owner_idx;
			self.turn_num += 1;
			if self.turn_num == 7 {
				//if the round has ended
				let scores = match self.end_round() {
					Ok(s) => s,
					Err(e) => return TurnAction::Err(e),
				};
				//see if someone won
				let mut highest_score = 0;
				let mut second_highest_score = 0;
				for team in self.teams.iter() {
					if team.score >= highest_score {
						second_highest_score = highest_score;
						highest_score = team.score;
					}
				}
				// lossless cast to i16
				if highest_score as i16 >= self.setup.max_score as i16
					&& (highest_score > second_highest_score + 1
					|| (!self.setup.win_by_2 && highest_score > second_highest_score))
				{
					return TurnAction::GameOver(
						scores,
						self.teams.iter().map(|team| team.score).collect(),
					);
				}
				return TurnAction::EndRound(scores);
			}
		}
		TurnAction::NextTurn
	}

	///Caller must deal cards for next round after this function is called.
	///Use RoundScores to notify players the results of the round.
	pub fn end_round(&mut self) -> Result<RoundScores, Error> {
		//calculate scores
		let mut highest: u8 = 2;
		let mut highest_index: u8 = 0;
		let mut lowest: u8 = ACE;
		let mut lowest_index: u8 = 0;
		let mut jack_index: Option<u8> = None;
		let mut game: u8 = 0;
		let mut game_runner_up: u8 = 0;
		let mut game_index: Option<u8> = None;
		let mut team_game_scores: Vec<u8> = Vec::with_capacity(MAX_PLAYERS as usize);
		for (team_index, team) in self.teams.iter().enumerate() {
			let (h, l, j, g) = calc_points(&team.pile, self.trump);
			if h > highest {
				highest = h;
				highest_index = team_index as u8;
			}
			if l < lowest {
				lowest = l;
				lowest_index = team_index as u8;
			}
			if j {
				jack_index = Some(team_index as u8);
			}
			if g >= game {
				game_runner_up = game;
				game = g;
				game_index = Some(team_index as u8);
			}
			team_game_scores.push(g);
		}
		if game == game_runner_up {
			game_index = None;
		}
		let mut round_scores: HashMap<u8, i8> = HashMap::with_capacity(4);
		round_scores
			.entry(highest_index)
			.and_modify(|score| {
				*score += 1;
			})
			.or_insert(1);
		round_scores
			.entry(lowest_index)
			.and_modify(|score| {
				*score += 1;
			})
			.or_insert(1);
		if let Some(index) = jack_index {
			round_scores
				.entry(index)
				.and_modify(|score| {
					*score += 1;
				})
				.or_insert(1);
		}
		if let Some(index) = game_index {
			round_scores
				.entry(index)
				.and_modify(|score| {
					*score += 1;
				})
				.or_insert(1);
		}
		round_scores
			.entry(self.setup.players[self.highest_bidder_idx].team_id)
			.and_modify(|bidder_score| {
				if *bidder_score < self.highest_bid as i8 {
					*bidder_score = -(self.highest_bid as i8);
				}
			})
			.or_insert(-(self.highest_bid as i8));
		for (team_id, score) in round_scores {
			self.teams[team_id].score += score;
		}
		let mut team_scores = SmallVec::new();
		for team in self.teams.iter() {
			team_scores.push(team.score);
		}
		self.dealer_idx += 1;
		if self.dealer_idx >= self.setup.players.len() as u8 {
			self.dealer_idx = 0;
		}
		self.sequence_num += 1;
		Ok(RoundScores {
			high_team_id: highest_index,
			low_team_id: lowest_index,
			jack_team_id: jack_index,
			game_team_id: game_index,
			team_game_scores,
			team_scores,
		})
	}

	pub fn deal_cards(&mut self) {
		//reset game data
		self.highest_bid = Bid::Pass;
		self.round_num = 0;
		self.turn_idx = self.dealer_idx + 1;
		if self.turn_idx >= self.setup.players.len() {
			self.turn_idx = 0;
		}
		//collect cards and shuffle
		self.deck.append(&mut self.pile);
		for player in self.setup.players.iter_mut() {
			self.deck.append(&mut player.hand);
		}
		for team in self.teams.iter_mut() {
			self.deck.append(&mut team.pile);
		}
		shuffle(self.deck.as_slice_mut());
		//deal cards
		const ROUNDS: usize = 2;
		for _ in 0..ROUNDS {
			for player in self.setup.players.iter_mut() {
				for card in self.deck.drain(0..(CARDS_PER_PLAYER / ROUNDS)) {
					player.hand.push(card);
				}
			}
		}
	}

	///Which ever player's turn it currently is, this checks if that player is an AI.
	pub fn is_ai_turn(&self) -> bool {
		matches!(
			self.setup.players[self.turn_idx].player_type,
			PlayerType::Ai
		)
	}

	///Validates the bid and updates game state. Returns early if the bid was invalid. Call [Self::next_turn] after this.
	pub fn validate_bid(&mut self, bid: Bid) -> bool {
		if bid == Bid::Pass {
			return true;
		}
		if (bid == Bid::Override && !self.setup.dealer_override) || bid <= self.highest_bid {
			return false;
		}
		self.highest_bidder_idx = self.turn_idx;
		self.highest_bid = bid;
		if bid == Bid::Four {
			self.round_num = 1;
		}
		true
	}

	///Returns early if the card played is invalid. Call [Self::next_turn] after this.
	pub fn validate_card(&mut self, card: Card) -> bool {
		let player = &mut self.setup.players[self.turn_idx];
		let Some(card_idx) = player.hand.iter().position(|c| *c == card) else {
			return false;
		};
		//update game state
		if self.pile.len() == 0 {
			//the first card played of the turn
			self.owner_idx = self.turn_idx;
			self.lead_idx = self.turn_idx;
			self.highest_card = card;
			self.lead = card.suit;
			if self.round_num == 1 {
				//first card played of the round
				self.trump = card.suit;
			}
		} else {
			//verify legality of card
			if card.suit != self.trump && card.suit != self.lead && player.can_follow(self.lead) {
				return false;
			}
			//calculate if new owner and update game data
			if (card.suit == self.highest_card.suit && card.number > self.highest_card.number)
				|| (self.highest_card.suit != self.trump && card.suit == self.trump)
			{
				self.highest_card = card;
				self.owner_idx = self.turn_idx;
			}
		}
		//put card on pile
		self.pile.push(player.hand.remove(card_idx as u8));
		true
	}

	///Algorithm for deciding how the AI should bid
	fn ai_eval_bid(&self) -> (Suit, u8) {
		let player = &self.setup.players[self.turn_idx];
		let mut suit_points = HashMap::with_capacity(4);
		for suit in Suit::iter() {
			suit_points.insert(suit, 0);
		}
		for card in player.hand.iter() {
			let Some(points) = suit_points.get_mut(&card.suit) else {
				continue;
			};
			*points += match card.number {
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
				9 => 20, //9 through A can't add up to more than 255
				10 => 25,
				JACK => 40,
				QUEEN => 35,
				KING => 45,
				ACE => 90, //A should be more than any 2 other things combined
				_ => 0,
			}
		}
		let (suit, num) = suit_points
			.iter()
			.max_by(|(_, num1), (_, num2)| num1.cmp(num2))
			.unwrap();
		(*suit, *num)
	}

	///Uses a card-scoring system, the turn order, and the game score to determine the bid
	pub fn ai_bid(&mut self) -> u8 {
		let (_, points) = self.ai_eval_bid();
		let mut bid = match points {
			..=44 => 0,     //pass
			45..=99 => 1,   //minimum play out of desperation
			100..=139 => 2, //A + one more good card
			140..=179 => 3, //A + face card or two + maybe one more
			180.. => 4,     //A + K + J + one more good card
		};
		//if you are one of the first to bid, maybe bid higher either to force an opponent to outbid when they shouldn't
		//or to prevent opponents from bidding to try to be the highest bidder
		if bid != 0 && bid != 4 && ((90..=99).contains(&bid) || (125..=139).contains(&bid)) {
			for i in 0..self.setup.players.len() - 5 {
				let mut index = self.turn_idx + i;
				if index >= self.setup.players.len() {
					index -= self.setup.players.len();
				}
				if index == self.turn_idx {
					bid += 1;
					break;
				}
			}
		}
		//if opponents are about to win, make risky bids
		if let Some((index, lead_team)) = self
			.teams
			.iter()
			.enumerate()
			.max_by(|(_, team1), (_, team2)| team1.score.cmp(&team2.score))
		{
			let player: &Player = &self.setup.players[self.turn_idx];
			if player.team_id != index as u8
				&& lead_team.score >= self.setup.max_score - 4
				&& self.teams[player.team_id].score < self.setup.max_score - 4
				&& bid != 4
			{
				bid += 1;
			}
		}
		if bid <= 1 || bid <= self.highest_bidder_idx {
			bid = 0;
		}
		bid
	}

	///Plays the highest card if this player had the highest bid.
	///Currently picks a random valid card until a proper AI is implemented.
	pub fn ai_play_card(&mut self) -> Card {
		let player = &self.setup.players[self.turn_idx];
		//Highest bidder starts the round, and the leading card is a unique calculation
		if self.round_num == 1 && self.pile.len() == 0 {
			let (suit, _) = self.ai_eval_bid(); //Suit of highest bid should not have changed, so we will just recalculate it instead of storing it
			let highest = *player
				.hand
				.iter()
				.filter(|card| card.suit == suit)
				.max_by(|card1, card2| card1.number.cmp(&card2.number))
				.unwrap();
			return highest;
		}
		let playable_card = if !self.pile.len() == 0 && player.can_follow(self.lead) {
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
		let mut rng = rng();
		let index = rng.random_range(0..playable_card.len());

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

		let card = playable_card[index];
		card
	}
}
