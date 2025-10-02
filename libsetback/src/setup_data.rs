use {
	std::sync::{Arc, RwLock, RwLockWriteGuard},
	bincode::{Decode, Encode},
	crate::{
		constants::*,
		player::*,
		connection::*,
		items::*,
		game::*
	},
	tokio::sync::mpsc::{channel, unbounded_channel, Receiver, Sender},
	tokio_util::sync::CancellationToken
};

/// Setup Data is written to during the setup of the game.
/// It is read from when the game is being played to handle the rules of the game.
#[derive(Encode, Decode, Debug, Clone)]
pub struct SetupData {
	pub players: Vec<Player>,
	pub std_team_mode: bool,
	pub win_by_2: bool,
	pub negative_scores: bool,
	pub dealer_override: bool,
	pub max_score: i8,
	pub max_players: u8
}
impl Default for SetupData {
	fn default() -> Self {Self {
		players: Vec::with_capacity(MAX_PLAYERS as usize),
		std_team_mode: true,
		win_by_2: false,
		negative_scores: true,
		dealer_override: false,
		max_score: 21,
		max_players: 8
	}}
}

impl SetupData {
	pub fn new_singleplayer() -> Self {
		let mut setup_data: Self = Self::default();
		setup_data.players.push(Player::new_singleplayer_self(0));
		for _ in 1..8 {
			setup_data.add_ai();
		}
		setup_data
	}

	pub fn add_ai(&mut self) {
		self.players.push(Player::new_ai(self.players.len() as u8));
	}

	///Updates a player's status as AI or human. This should only be called by the server.
	pub fn update_human(&mut self, index: u8, human: bool) {
		self.players[index as usize].player_type.set(if human {PlayerType::HumanEmpty} else {PlayerType::AI});
	}

	///Updates a player's team.
	pub fn update_team(&mut self, index: u8, team_id: u8) {
		self.players[index as usize].team_id = team_id;
	}

	///Makes this player type a networked client. This should only be called by the server.
	pub fn set_client(&mut self, index: u8, connection: ClientConnection) {
		self.players[index as usize].player_type.set(PlayerType::HumanConnection(connection));
	}

	///Removes a player from the list, and reinserts it at the given index, sliding the remaining players if necessary.
	///Do not call this after starting the game.
	pub fn re_insert_player(&mut self, init_index: u8, new_index: u8) {
		let player: Player = self.players.remove(init_index as usize);
		self.players.insert(new_index as usize, player);
	}

	///Initializes any remaining data and starts the game loop.
	///Further communication between the game and the players is handled through the returned [GameRuntime].
	pub fn start_game(self) -> GameRuntime {
		let game: Arc<RwLock<Game>> = Arc::new(RwLock::new(Game::new(self)));
		let mut senders: Vec<(Sender<u8>, Sender<Card>)> = Vec::with_capacity(MAX_PLAYERS as usize);
		let mut receivers: Vec<(Receiver<u8>, Receiver<Card>)> = Vec::with_capacity(MAX_PLAYERS as usize);
		{
			let mut game_guard: RwLockWriteGuard<Game> = game.write().unwrap();
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
		let game_ref: Arc<RwLock<Game>> = game.clone();
		tokio::spawn(async move {tokio::select! {
			_ = async move {
				let mut turn_result: Result<TurnResult, Error> = Ok(TurnResult::NextTurn);
				loop {
					match turn_result {
						Ok(TurnResult::NextTurn) => {
							let (index, ai, bidding) = {
								let game_guard: RwLockWriteGuard<Game> = game.write().unwrap();
								(game_guard.turn_index, game_guard.is_ai_turn(), game_guard.turn_number == 0)
							};
							event_tx.send(GameEvent::PlayerTurn{index}).unwrap();
							if bidding {
								let bid: u8 = if ai {
									let mut game_guard: RwLockWriteGuard<Game> = game.write().unwrap();
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
								let mut game_guard: RwLockWriteGuard<Game> = game.write().unwrap();
								turn_result = game_guard.next_turn();
								event_tx.send(GameEvent::PlayerBid {index, amount: bid}).unwrap();
							} else {
								let card: Card = if ai {
									let mut game_guard: RwLockWriteGuard<Game> = game.write().unwrap();
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
								let mut game_guard: RwLockWriteGuard<Game> = game.write().unwrap();
								turn_result = game_guard.next_turn();
								event_tx.send(GameEvent::PlayerPlayedCard{index, card}).unwrap();
							}
						}
						Ok(TurnResult::ReDeal) => {
							game.write().unwrap().deal_cards();
							event_tx.send(GameEvent::DealCards).unwrap();
							turn_result = Ok(TurnResult::NextTurn);
						}
						Ok(TurnResult::EndRound(scores)) => {
							game.write().unwrap().deal_cards();
							turn_result = Ok(TurnResult::NextTurn);
							event_tx.send(GameEvent::RoundScores(scores)).unwrap();
						}
						Ok(TurnResult::GameOver(round_scores, game_scores)) => {
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
		}});
		GameRuntime {game: game_ref, cancel_token: token, event_emitter: event_rx, prompt_senders: senders}
	}
}