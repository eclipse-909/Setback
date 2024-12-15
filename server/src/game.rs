use {
	resources::server_items::*,
	std::{
		sync::{Arc, RwLock, RwLockWriteGuard, RwLockReadGuard},
		collections::HashMap,
		mem
	},
	tokio::{
		time::{self, Duration, Interval},
		sync::{mpsc, watch}
	},
	rand::{
		rngs::ThreadRng,
		Rng, rng
	}
};

pub const MAX_GAMES: u32 = 100;
pub const MIN_GAMES: usize = 10;

pub type Games = Arc<RwLock<HashMap<u32, Arc<RwLock<MPGame>>>>>;

pub struct MPGame {
	///Receives messages from each client's sender
	client_rx: mpsc::UnboundedReceiver<Ssp>,
	///Sender to be copied and passed into each client that the Game adds to the HashMap. This will be used so clients can send data to the Game.
	game_tx: mpsc::UnboundedSender<Ssp>,
	game_terminator: watch::Sender<()>,
	server_terminator: watch::Sender<()>,
	countdown_signal: watch::Sender<bool>,
	client_id_to_index: HashMap<u32, u8>,
	pub sp_game: SPGame
}

impl MPGame {
	///Check if you can create a bevy_game before creating one
	pub async fn create(
		games: Games,
		server_terminator_tx: watch::Sender<()>,
		mut server_terminator_rx: watch::Receiver<()>
	) -> Arc<RwLock<Self>> {
		//create the bevy_game
		let (tx, rx) = mpsc::unbounded_channel::<Ssp>();
		let (terminator_tx, mut terminator_rx) = watch::channel::<()>(());
		let (countdown_signal, mut countdown_rx) = watch::channel::<bool>(false);
		let game: Arc<RwLock<Self>> = Arc::new(RwLock::new(Self {
			client_rx: rx,
			game_tx: tx,
			game_terminator: terminator_tx,
			server_terminator: server_terminator_tx,
			countdown_signal,
			client_id_to_index: HashMap::with_capacity(MAX_PLAYERS as usize),
			sp_game: SPGame::default()
		}));

		//create a join code and add the bevy_game to the hashmap
		let mut code: u32;
		{
			let games_guard: RwLockReadGuard<HashMap<u32, Arc<RwLock<Self>>>> = games.read().await;
			let mut rng: ThreadRng = rng();
			code = rng.random();
			
			//TODO come up with a better way to generate a bevy_game code
			
			while games_guard.contains_key(&code) {code = rng.random();}
		}//rng must be dropped before the next yield
		games.write().await.insert(code, game.clone());
		let return_game: Arc<RwLock<Self>> = game.clone();
		
		//create a task for the countdown
		let mut server_term_rx: watch::Receiver<()> = server_terminator_rx.clone();
		let mut term_rx: watch::Receiver<()> = terminator_rx.clone();
		let game_clone: Arc<RwLock<Self>> = game.clone();
		tokio::spawn(async move {
			tokio::select! {
				_ = async move {
					let mut interval: Interval = time::interval(Duration::from_secs(10));
					interval.tick().await;
					loop {
						match countdown_rx.changed().await {
							Ok(_) => {
								if *countdown_rx.borrow_and_update() {
									//start the countdown
									tokio::select! {
										result = countdown_rx.changed() => {
											//stop the countdown
											if result.is_err() {return;};
										}
										_ = interval.tick() => {
											//start the bevy_game
											//initialize remaining bevy_game data and tell clients to start
											let mut game_guard: RwLockWriteGuard<MPGame> = game_clone.write().await;
											if !game_guard.sp_game.start_game() {
												return game_guard.terminate();
											}
											if game_guard.deal_cards().await.is_err() {
												return game_guard.terminate();
											}
											game_guard.broadcast_message(&Ssp::new(Protocol::Start)).await;
											let players: Vec<Player> = game_guard.sp_game.players.clone();
											game_guard.broadcast_message(&Ssp::new(Protocol::Players(players))).await;
											game_guard.broadcast_message(&Ssp::new(Protocol::DealCards)).await;
											let is_ai: bool = game_guard.sp_game.is_ai();
											game_guard.ai_game_loop(Ok(TurnResult::IsAI(is_ai))).await;
											return;
										}
									}
								}
							}
							Err(_) => {return;}
						}
					}
				} => {}
				_ = server_term_rx.changed() => {}
				_ = term_rx.changed() => {}
			}
		});

		//have the bevy_game continuously listen for messages
		tokio::spawn(async move {
			tokio::select! {
				_ = async move {
					'listener: loop {
						let Some(ssp) = game.write().await.client_rx.recv().await else {break;};
						match &ssp.protocol {
							Protocol::Disconnect => {
								//handle client disconnection
								let mut game_guard: RwLockWriteGuard<Self> = game.write().await;
								let Some(player_client) = game_guard.remove_client(ssp.src_id) else {continue;};
								if player_client.host {
									for player in game_guard.sp_game.players.iter_mut() {
										if let PlayerType::Client(p) = &mut player.player_type {
											p.host = true;
											continue 'listener;
										}
									}
								}
								//shutdown bevy_game if nobody is left
								return;
							}
							Protocol::GameMeta(meta_data) => {
								//host-only
								let mut game_guard: RwLockWriteGuard<Self> = game.write().await;
								let Some((player_client, _)) = game_guard.get_player_client(ssp.src_id) else {continue;};
								if player_client.client.is_none() {
									game_guard.remove_client(ssp.src_id);
									continue;
								}
								if !player_client.host {
									game_guard.send_error_back(ssp.src_id, ServerError::HostOnlyProtocol).await;
									continue;
								}
								game_guard.sp_game.meta_data = meta_data.clone();
								let meta_data: GameMeta = game_guard.sp_game.meta_data.clone();
								game_guard.broadcast_message(&Ssp::new_with_src(Protocol::GameMeta(meta_data), ssp.src_id)).await;
							}
							Protocol::Player(player, index) => {
								//host-only
								let mut game_guard: RwLockWriteGuard<Self> = game.write().await;
								let Some((player_client, _)) = game_guard.get_player_client(ssp.src_id) else {continue;};
								if player_client.client.is_none() {
									game_guard.remove_client(ssp.src_id);
									continue;
								}
								if !player_client.host {
									game_guard.send_error_back(ssp.src_id, ServerError::HostOnlyProtocol).await;
									continue;
								}
								//update state and broadcast
								let mut subject_player: Player = game_guard.get_player(*index).clone();
								match subject_player.update_player(player, false) {
									Ok(None) => {}
									Ok(Some(player_type)) => {
										//disconnect client
										if let PlayerType::Client(player_client) = player_type {
											game_guard.client_id_to_index.remove(&player_client.account_id);
										}
									}
									Err(error) => {
										game_guard.send_error_back(ssp.src_id, error).await;
										game_guard.send_message(&Ssp::new(Protocol::Player(subject_player, *index)), ssp.src_id).await;
										continue;
									}
								}
								//subject_player was cloned, and the clone was updated. Now the update needs to be applied to the subject_player
								let _ = game_guard.get_player(*index).update_player(player, false);
								game_guard.broadcast_message(&Ssp::new_with_src(Protocol::Player(subject_player, *index), ssp.src_id)).await;
							}
							Protocol::Players(players) => {
								//host-only
								let mut game_guard: RwLockWriteGuard<Self> = game.write().await;
								let Some((player_client, _)) = game_guard.get_player_client(ssp.src_id) else {continue;};
								if player_client.client.is_none() {
									game_guard.remove_client(ssp.src_id);
									continue;
								}
								if !player_client.host {
									game_guard.send_error_back(ssp.src_id, ServerError::HostOnlyProtocol).await;
									continue;
								}
								//update state and broadcast
								//store client_ids and client pointers for later
								let mut client_arcs: HashMap<u32, Arc<RwLock<Client>>> = HashMap::with_capacity(MAX_PLAYERS as usize);
								for player in game_guard.sp_game.players.iter_mut() {
									let PlayerType::Client(player_client) = &mut player.player_type else {continue;};
									let Some(client) = player_client.client.take() else {continue;};
									client_arcs.insert(player_client.account_id, client);
								}
								//clone the players list to apply the updates. this will keep the actual players list intact until all updates are verified
								let mut list_clone: Vec<Player> = game_guard.sp_game.players.clone();
								for (index, player) in players.iter().enumerate() {
									//add player to list if index is bigger than list
									while index >= list_clone.len() {
										let ai: Player = Player::new_ai(0);
										list_clone.push(ai);
									}
									if let Err(error) = list_clone[index].update_player(player, true) {
										game_guard.send_error_back(ssp.src_id, error).await;
										let players: Vec<Player> = game_guard.sp_game.players.clone();
										game_guard.send_message(&Ssp::new(Protocol::Players(players)), ssp.src_id).await;
										continue 'listener;
									}
								}
								//re-insert client pointers
								for player in list_clone.iter_mut() {
									let PlayerType::Client(player_client) = &mut player.player_type else {continue;};
									let Some(client) = client_arcs.remove(&player_client.account_id) else {
										game_guard.send_error_back(ssp.src_id, ServerError::ClientNotFound).await;
										let players: Vec<Player> = game_guard.sp_game.players.clone();
										game_guard.send_message(&Ssp::new(Protocol::Players(players)), ssp.src_id).await;
										continue 'listener;
									};
									player_client.client = Some(client);
								}
								//any client pointers that weren't re-inserted will be freed when client_ptrs falls out of scope
								//update the actual players
								game_guard.sp_game.players = list_clone;
								//update client_id_to_index hashmap
								game_guard.client_id_to_index.clear();
								for i in 0..game_guard.sp_game.players.len() {
									let PlayerType::Client(player_client) = &game_guard.sp_game.players[i].player_type else {continue;};
									let account_id: u32 = player_client.account_id;
									game_guard.client_id_to_index.insert(account_id, i as u8);
								}
								//broadcast changes
								let players: Vec<Player> = game_guard.sp_game.players.clone();
								game_guard.broadcast_message(&Ssp::new_with_src(Protocol::Players(players), ssp.src_id)).await;
							}
							Protocol::Ready(is_ready) => {
								//update bevy_game state
								let mut game_guard: RwLockWriteGuard<Self> = game.write().await;
								let Some((player_client, _)) = game_guard.get_player_client(ssp.src_id) else {continue;};
								player_client.ready = *is_ready;
								game_guard.broadcast_message(&Ssp::new_with_src(Protocol::Ready(*is_ready), ssp.src_id)).await;
								//if everyone is ready, start a 10-second countdown
								//if the countdown is ticking, and someone un-readies, stop the countdown and reset it
								//if the countdown reaches 0, start the bevy_game
								if !is_ready {continue 'listener;}
								for i in 0..game_guard.sp_game.players.len() {
									match &game_guard.sp_game.players[i].player_type {
										PlayerType::Human => {
											if game_guard.countdown_signal.send(false).is_err() {return;}
											continue 'listener;
										}
										PlayerType::Client(player_client) => {
											if !player_client.ready {
												if game_guard.countdown_signal.send(false).is_err() {return;}
												continue 'listener;
											}
										}
										PlayerType::AI => {}
									}
								}
								if game_guard.countdown_signal.send(true).is_err() {return;}
							}
							Protocol::Bid(amount) => {
								let mut game_guard: RwLockWriteGuard<Self> = game.write().await;
								let Some(index) = game_guard.client_id_to_index.get(&ssp.src_id) else {return;};
								let index: u8 = *index;
								if game_guard.sp_game.game_data.turn_index != index {
									game_guard.send_error_back(ssp.src_id, ServerError::OutOfTurn).await;
									continue;
								}
								//verify valid amount
								if !game_guard.sp_game.validate_bid(*amount) {
									game_guard.send_error_back(ssp.src_id, ServerError::InvalidBid).await;
									continue;
								}
								//update bevy_game and player data
								//broadcast the bid and bevy_game data
								game_guard.broadcast_message(&Ssp::new_with_src(Protocol::Bid(*amount), ssp.src_id)).await;
								let game_data: GameData = game_guard.sp_game.game_data.clone();
								game_guard.broadcast_message(&Ssp::new(Protocol::GameData(game_data))).await;
								//do AI's turns and calculate their bids
								let next_turn: Result<TurnResult, ServerError> = game_guard.sp_game.next_turn();
								game_guard.ai_game_loop(next_turn).await;
							}
							Protocol::PlayCard(card) => {
								let mut game_guard: RwLockWriteGuard<Self> = game.write().await;
								//verify the player had this card
								let Some(client_index) = game_guard.client_id_to_index.get(&ssp.src_id) else {return;};
								let client_index: u8 = *client_index;
								if game_guard.sp_game.game_data.turn_index != client_index {
									game_guard.send_error_back(ssp.src_id, ServerError::OutOfTurn).await;
									continue;
								}
								match game_guard.sp_game.validate_card(card.to_owned()) {
									Ok(true) => {}
									Ok(false) => {
										game_guard.send_error_back(ssp.src_id, ServerError::InvalidCard).await;
										continue;
									}
									Err(_) => {return;}
								}
								//broadcast card played and bevy_game data
								game_guard.broadcast_message(&Ssp::new_with_src(Protocol::PlayCard(card.clone()), ssp.src_id)).await;
								let game_data: GameData = game_guard.sp_game.game_data.clone();
								game_guard.broadcast_message(&Ssp::new(Protocol::GameData(game_data))).await;
								//process AI turns
								let next_turn: Result<TurnResult, ServerError> = game_guard.sp_game.next_turn();
								game_guard.ai_game_loop(next_turn).await;
							}
							_ => {game.write().await.send_error_back(ssp.src_id, ServerError::InvalidProtocol).await;}
						}
					}
				} => {}
				_ = server_terminator_rx.changed() => {}
				_ = terminator_rx.changed() => {}
			}

			//The bevy_game shuts down here. Clean up the players and resources, and remove the bevy_game from the server's games hashmap
			let mut games_guard: RwLockWriteGuard<HashMap<u32, Arc<RwLock<MPGame>>>> = games.write().await;
			if let Some(this_game) = games_guard.remove(&code) {
				let mut game_guard: RwLockWriteGuard<Self> = this_game.write().await;
				for i in (0..game_guard.sp_game.players.len()).rev() {
					game_guard.remove_player(i as u8);
				}
			}
			//occasionally shrink HashMap capacity when bevy_game ends to clean up unused memory
			if (games_guard.len() as f32) < f32::max(games_guard.capacity() as f32 * 0.1, MIN_GAMES as f32) {
				let num_games: usize = games_guard.len();
				games_guard.shrink_to(usize::max(num_games * 5, MIN_GAMES));
			}
		});
		return_game
	}

	pub fn can_add_player(&self) -> bool {self.sp_game.players.len() < MAX_PLAYERS as usize }
	
	fn get_player(&mut self, index: u8) -> &mut Player {&mut self.sp_game.players[index as usize]}
	
	fn get_player_client(&mut self, account_id: u32) -> Option<(&mut PlayerClient, u8)> {
		let index: u8 = *self.client_id_to_index.get(&account_id)?;
		let Player{player_type: PlayerType::Client(player_client),..} = self.get_player(index) else {return None;};
		Some((player_client, index))
	}
	
	fn get_client(&mut self, account_id: u32) -> Option<(&mut Arc<RwLock<Client>>, u8)> {
		let (player_client, index) = self.get_player_client(account_id)?;
		Some((player_client.client.as_mut()?, index))
	}
	
	///To add a client to a bevy_game, the server must remove the client from its hashmap, and move it into this function.
	///Called when a player creates or joins a bevy_game.
	///Call [Self::can_add_player] before this function to determine if a client can be added.
	pub async fn add_client(&mut self, c: Arc<RwLock<Client>>, h: bool) {
		//Get the index of where this client will be added, and the client_id that this client will get
		let mut next_index: Option<u8> = None;
		for (index, player) in self.sp_game.players.iter_mut().enumerate() {
			if matches!(player.player_type, PlayerType::Human) && next_index.is_none() {
				next_index = Some(index as u8);
			}
		}
		//put the client in the list and update it's and Sender
		let mut client_guard: RwLockWriteGuard<Client> = c.write().await;
		client_guard.owner_tx = self.game_tx.clone();
		if let Some(index) = next_index {
			self.client_id_to_index.insert(client_guard.account_id, index);
			drop(client_guard);
			self.sp_game.players[index as usize].player_type = PlayerType::Client(PlayerClient::new(c, h).await);
		} else {
			self.client_id_to_index.insert(client_guard.account_id, self.sp_game.players.len() as u8);
			drop(client_guard);
			self.sp_game.players.push(Player::new_client(c, h, if self.sp_game.meta_data.std_team_mode { (self.sp_game.players.len() % 2 + 1) as u8 } else {0}).await);
		}
	}

	async fn send_message(&mut self, ssp: &Ssp, dest_id: u32) {
		if let Some((client, index)) = self.get_client(dest_id) {
			if client.write().await.message_client(ssp).await.is_err() {
				self.remove_player(index);
			}
		}
	}
	
	///Sends a message to everyone except the client that matches the given ssp's src_id.
	async fn broadcast_message(&mut self, ssp: &Ssp) {
		let mut remove: Vec<u8> = Vec::new();
		for (index, player) in self.sp_game.players.iter().enumerate() {
			let PlayerType::Client(PlayerClient{client, account_id,..}) = &player.player_type else {continue;};
			let Some(client) = client.as_ref() else {
				remove.push(index as u8);
				continue;
			};
			if *account_id == ssp.src_id {
				continue;
			}
			if client.write().await.message_client(ssp).await.is_err() {
				remove.push(index as u8);
				continue;
			}
		}
		for i in remove.iter().rev() {
			self.remove_player(*i);
		}
	}

	async fn send_error_back(&mut self, src_id: u32, err: ServerError) {
		let Some((client, index)) = self.get_client(src_id) else {return;};
		if client.write().await.message_client(&Ssp::new(Protocol::ServerErr(err))).await.is_err() {
			self.remove_player(index);
		}
	}
	
	///Removes a player from the list of players, shuts down the TcpSteam, and drops the client.
	fn remove_player(&mut self, index: u8) {
		let player: Player = self.sp_game.players.remove(index as usize);
		let PlayerType::Client(PlayerClient{account_id,..}) = player.player_type else {return;};
		self.client_id_to_index.remove(&account_id);
	}
	
	///Disconnects the client and turns it into an empty slot. Doesn't change the number of players in the list.
	fn remove_client(&mut self, account_id: u32) -> Option<PlayerClient> {
		let index = self.client_id_to_index.get(&account_id)?;
		let Player{player_type,..} = self.get_player(*index);
		let PlayerType::Client(client) = mem::replace(player_type, PlayerType::Human) else {return None;};
		self.client_id_to_index.remove(&account_id);
		Some(client)
	}
	
	///Tries to terminate this bevy_game only. If that fails, it tries to terminate the server. If that fails, it panics.
	pub fn terminate(&mut self) {
		if self.game_terminator.send(()).is_err() {
			self.server_terminator.send(()).expect("Could not gracefully shut down server.");
		}
	}
	
	async fn matches_filter(&self, filter: &GameFilter) -> bool {
		filter.max_players_cmp.compare(filter.max_players_value, self.sp_game.players.len() as u8)
			&& filter.std_team_mode == self.sp_game.meta_data.std_team_mode
			&& filter.max_score_cmp.compare(filter.max_score_value, self.sp_game.meta_data.max_score)
			&& filter.win_by_2 == self.sp_game.meta_data.win_by_2
			&& filter.negative_scores == self.sp_game.meta_data.negative_scores
	}
	
	pub async fn get_game_metas(games: &Games, filter: &GameFilter) -> Vec<GameMeta> {
		let mut game_metas: Vec<GameMeta> = Vec::new();
		for (_, game) in games.read().await.iter() {
			let game_guard: RwLockReadGuard<MPGame> = game.read().await;
			if game_guard.matches_filter(filter).await {
				game_metas.push(game_guard.sp_game.meta_data.clone());
				if game_metas.len() as u8 == MAX_GAME_METAS {break;}
			}
		}
		game_metas
	}
	
	async fn deal_cards(&mut self) -> Result<(), ServerError> {
		self.sp_game.deal_cards()?;
		//send Player to each player
		for i in 0..self.sp_game.players.len() {
			let PlayerType::Client(player_client) = &self.sp_game.players[i].player_type else { continue; };
			self.send_message(&Ssp::new(Protocol::Player(self.sp_game.players[i].clone(), i as u8)), player_client.account_id).await;
		}
		//broadcast bevy_game data
		self.broadcast_message(&Ssp::new(Protocol::GameData(self.sp_game.game_data.clone()))).await;
		Ok(())
	}
	
	///Processes the next AI turns until it's a human's turn.
	async fn ai_game_loop(&mut self, mut turn_result: Result<TurnResult, ServerError>) {
		loop {
			match turn_result {
				Ok(TurnResult::IsAI(ai)) => {
					if !ai {return;}
					if self.sp_game.game_data.turn_number == 0 {
						let bid: u8 = self.sp_game.ai_bid();
						turn_result = self.sp_game.next_turn();
						self.broadcast_message(&Ssp::new(Protocol::Bid(bid))).await;
						self.broadcast_message(&Ssp::new(Protocol::GameData(self.sp_game.game_data.clone()))).await;
					} else {
						let Some(card) = self.sp_game.ai_play_card() else {return self.terminate();};
						turn_result = self.sp_game.next_turn();
						self.broadcast_message(&Ssp::new(Protocol::PlayCard(card))).await;
						self.broadcast_message(&Ssp::new(Protocol::GameData(self.sp_game.game_data.clone()))).await;
					}
				}
				Ok(TurnResult::ReDeal) => {
					if self.deal_cards().await.is_err() {return self.terminate();}
					turn_result = Ok(TurnResult::IsAI(self.sp_game.is_ai()));
					self.broadcast_message(&Ssp::new(Protocol::DealCards)).await;
					self.broadcast_message(&Ssp::new(Protocol::GameData(self.sp_game.game_data.clone()))).await;
				}
				Ok(TurnResult::EndRound(scores)) => {
					if self.deal_cards().await.is_err() {return self.terminate();}
					turn_result = Ok(TurnResult::IsAI(self.sp_game.is_ai()));
					self.broadcast_message(&Ssp::new(Protocol::RoundScores(scores))).await;
					self.broadcast_message(&Ssp::new(Protocol::DealCards)).await;
					self.broadcast_message(&Ssp::new(Protocol::GameData(self.sp_game.game_data.clone()))).await;
				}
				Ok(TurnResult::GameOver(scores)) => {
					self.broadcast_message(&Ssp::new(Protocol::RoundScores(scores))).await;
					self.broadcast_message(&Ssp::new(Protocol::GameOver)).await;
					self.broadcast_message(&Ssp::new(Protocol::GameData(self.sp_game.game_data.clone()))).await;
					return self.terminate();
				}
				Err(_) => {return self.terminate();}
			}
		}
	}
}