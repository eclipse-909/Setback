use {
	crate::*,
	serde::{Deserialize, Serialize},
	std::sync::{Arc, RwLock, RwLockWriteGuard},
	tokio::sync::mpsc::{Receiver, Sender, channel, unbounded_channel},
	tokio_util::sync::CancellationToken,
};

/// Game is created modified when setting up a game.
/// It is read-only when the game is being played to handle the rules of the game.
#[derive(Deserialize, Serialize, Debug)]
pub struct GameSetup {
	/// Every other player is on a team. The alternative being custom teams.
	pub std_team_mode: bool,
	pub win_by_2: bool,
	pub negative_scores: bool,
	pub dealer_override: bool,
	pub max_score: u8,
	pub max_players: u8,
	/// In seconds. 0 means unlimited time. The maximum allowed time on the server is [MAX_TURN_TIME_SERVER] seconds.
	pub turn_time: u8,
	pub players: SmallVec<Player, MAX_PLAYERS>,
}

impl GameSetup {
	/// Default rules for a game. These can be changed by the host during setup.
	const fn new() -> Self {
		Self {
			std_team_mode: true,
			win_by_2: false,
			negative_scores: true,
			dealer_override: false,
			max_score: MAX_SCORE,
			max_players: MAX_PLAYERS as u8,
			turn_time: DEFAULT_TURN_TIME,
			players: SmallVec::new(),
		}
	}

	#[cfg(feature = "client")]
	pub fn new_singleplayer(username: String, game_event_emitter: GameEventEmitter) -> (Self, Sender<Option<GameEvent>>) {
		let (sp_self, tx) = Player::new_singleplayer_self(0, username, game_event_emitter);
		let mut setup_data = Self::new();
		setup_data.players.push(sp_self);
		for _ in 0..MAX_PLAYERS-1 {
			setup_data.add_ai();
		}
		(setup_data, tx)
	}

	pub fn new_multiplayer(host: ClientConnection) -> Self {
		let host = Player::new_host_connected(host, 0);
		let mut setup_data = Self::new();
		setup_data.players.push(host);
		for _ in 0..MAX_PLAYERS-1 {
			setup_data.add_ai();
		}
		setup_data
	}

	pub const fn add_ai(&mut self) {
		self.players.push(Player::new_ai(self.players.len() & 1));
	}

	pub const fn add_human(&mut self) {
		self.players.push(Player::new_human_empty(self.players.len() & 1));
	}

	/// Updates a player's status as AI or human.
	pub fn update_human(&mut self, index: u8, is_human: bool) {
		self.players[index].player_type = if is_human {
			PlayerType::HumanEmpty
		} else {
			PlayerType::Ai
		};
	}

	/// Updates a player's team.
	pub fn update_team(&mut self, index: u8, team_id: u8) {
		self.players[index].team_id = team_id;
	}

	/// Makes this player type a networked client.
	#[inline(always)]
	pub fn set_client(&mut self, index: u8, conn: ClientConnection) {
		debug_assert!(
			matches!(self.players[index].player_type, PlayerType::HumanEmpty),
			"Player must be an empty human slot before setting a client connection"
		);
		self.players[index].player_type = PlayerType::HumanConnection {conn};
	}

	/// Initializes a game loop that hasn't started yet.
	pub fn init(mut self) -> GameLoop {
		// normalize team numbers so there are no gaps
		let mut team_ids: SmallVec<u8, MAX_PLAYERS> = self
			.players
			.iter()
			.map(|p| p.team_id)
			.collect();
		team_ids.sort();
		team_ids.dedup();
		let mut team_id_map = [0; MAX_PLAYERS];
		for (index, id) in team_ids.iter().enumerate() {
			team_id_map[*id as usize] = index as u8;
		}
		for player in self.players.iter_mut() {
			player.team_id = team_id_map[player.team_id as usize];
		}

		// create team objects in the game data
		let mut teams = SmallVec::new();
		for _ in team_ids {
			teams.push(Team::new());
		}

		// don't allow infinite time on the server
		#[cfg(not(feature = "client"))]
		if self.turn_time == 0 || self.turn_time > MAX_TURN_TIME_SERVER {
			self.turn_time = MAX_TURN_TIME_SERVER;
		}

		GameLoop {
			setup: self,
			state: GameState::StartRound,
			teams,
			deck: Card::new_deck(),
			pile: SmallVec::new(),
			trump: Suit::Hearts,
			lead: Suit::Hearts,
			owner_idx: 0,
			highest_card: Card {
				suit: Suit::Hearts,
				number: 0,
			},
			highest_bidder_idx: 0,
			highest_bid: Bid::Pass,
			turn_idx: 0,
			round_num: 0,
			lead_idx: 0,
			dealer_idx: 0,
			sequence_num: 0,
		}
	}

	/// Initializes any remaining data and starts the game loop.
	/// Further communication between the game and the players is handled through the returned [GameRuntime].
	pub fn start_game(self) -> GameRuntime {
		let game: Arc<RwLock<GameSetup>> = Arc::new(RwLock::new(GameSetup::new(self)));
		let mut senders: Vec<(Sender<Bid>, Sender<Card>)> =
			Vec::with_capacity(MAX_PLAYERS as usize);
		let mut receivers: Vec<(Receiver<Bid>, Receiver<Card>)> =
			Vec::with_capacity(MAX_PLAYERS as usize);
		{
			let mut game_guard: RwLockWriteGuard<GameSetup> = game.write().unwrap();
			game_guard.deal_cards();
			for _ in 0..game_guard.setup_data.players.len() {
				let (bid_tx, bid_rx) = channel::<u8>(1);
				let (card_tx, card_rx) = channel::<Card>(1);
				senders.push((bid_tx, card_tx));
				receivers.push((bid_rx, card_rx));
			}
		}
		let token: CancellationToken = CancellationToken::new();
		let cloned_token: CancellationToken = token.clone();
		let (event_tx, event_rx) = unbounded_channel::<GameEvent>();
		let game_ref: Arc<RwLock<GameSetup>> = game.clone();
		tokio::spawn(async move {
			tokio::select! {
				_ = async move {
					let mut turn_result: Result<TurnAction, Error> = Ok(TurnAction::NextTurn);
					loop {
						match turn_result {
							Ok(TurnAction::NextTurn) => {
								let (index, ai, bidding) = {
									let game_guard: RwLockWriteGuard<GameSetup> = game.write().unwrap();
									(game_guard.turn_index, game_guard.is_ai_turn(), game_guard.turn_number == 0)
								};
								event_tx.send(GameEvent::PlayerTurn{index}).unwrap();
								if bidding {
									let bid: u8 = if ai {
										let mut game_guard: RwLockWriteGuard<GameSetup> = game.write().unwrap();
										let b: u8 = game_guard.ai_bid();
										assert!(game_guard.validate_bid(b));
										b
									} else {
										event_tx.send(GameEvent::PromptBid{index, hand: game.write().unwrap().setup_data.players[index as usize].hand.clone()}).unwrap();
										let mut b: u8 = receivers[index as usize].0.recv().await.unwrap();
										while !game.write().unwrap().validate_bid(b) {
											event_tx.send(GameEvent::GameError(Error::InvalidBid)).unwrap();
											event_tx.send(GameEvent::PromptBid{index, hand: game.write().unwrap().setup_data.players[index as usize].hand.clone()}).unwrap();
											b = receivers[index as usize].0.recv().await.unwrap();
										}
										b
									};
									let mut game_guard: RwLockWriteGuard<GameSetup> = game.write().unwrap();
									turn_result = game_guard.next_turn();
									event_tx.send(GameEvent::PlayerBid {index, amount: bid}).unwrap();
								} else {
									let card: Card = if ai {
										let mut game_guard: RwLockWriteGuard<GameSetup> = game.write().unwrap();
										let c: Card = game_guard.ai_play_card();
										assert!(game_guard.validate_card(c));
										c
									} else {
										event_tx.send(GameEvent::PromptCard{index, hand: game.write().unwrap().setup_data.players[index as usize].hand.clone()}).unwrap();
										let mut c: Card = receivers[index as usize].1.recv().await.unwrap();
										while !game.write().unwrap().validate_card(c) {
											event_tx.send(GameEvent::GameError(Error::InvalidCard)).unwrap();
											event_tx.send(GameEvent::PromptCard{index, hand: game.write().unwrap().setup_data.players[index as usize].hand.clone()}).unwrap();
											c = receivers[index as usize].1.recv().await.unwrap();
										}
										c
									};
									let mut game_guard: RwLockWriteGuard<GameSetup> = game.write().unwrap();
									turn_result = game_guard.next_turn();
									event_tx.send(GameEvent::PlayerPlayedCard{index, card}).unwrap();
								}
							}
							Ok(TurnAction::ReDeal) => {
								game.write().unwrap().deal_cards();
								event_tx.send(GameEvent::DealCards).unwrap();
								turn_result = Ok(TurnAction::NextTurn);
							}
							Ok(TurnAction::EndRound(scores)) => {
								game.write().unwrap().deal_cards();
								turn_result = Ok(TurnAction::NextTurn);
								event_tx.send(GameEvent::RoundScores(scores)).unwrap();
							}
							Ok(TurnAction::GameOver(round_scores, game_scores)) => {
								event_tx.send(GameEvent::RoundScores(round_scores)).unwrap();
								return event_tx.send(GameEvent::GameOver(game_scores)).unwrap();
							}
							Err(error) => {
								return event_tx.send(GameEvent::GameError(error)).unwrap();
							}
						}
					}
				} => {}
				_ = cloned_token.cancelled() => {}
			}
		});
		GameRuntime {
			game: game_ref,
			cancel_token: token,
			event_emitter: event_rx,
			prompt_senders: senders,
		}
	}
}
