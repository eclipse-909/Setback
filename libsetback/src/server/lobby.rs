use {
	crate::*,
	nanoid::nanoid,
	std::{
		time::Duration,
		sync::Arc,
	},
	tracing::{debug, error},
	tokio::{
		sync::mpsc::{UnboundedReceiver, Sender, channel, unbounded_channel},
		task::JoinHandle,
		time::timeout,
	},
};

pub struct LobbyRtHandle {
	pub join_tx: Sender<Box<ClientConnection>>,
	pub handle: JoinHandle<(Lobby, UnboundedReceiver<(u8, NetError)>)>,
}

impl Lobby {
	pub fn create_multiplayer(
		host: Box<ClientConnection>,
		password: String,
		state: Arc<State>,
	) -> (LobbyRtHandle, String) {
		let account_id = host.account_id;
		let host = Player::new_host(host, 0);
		
		// TODO we might need a different way to create game codes if there is a significant risk of collisions
		let code = nanoid!(GAME_CODE_LEN, &GAME_CODE_ALPHABET);
		
		let (err_q_tx, err_q) = unbounded_channel();
		let mut lobby = Self::new(code.clone(), password, err_q_tx);
		lobby.lobby_state.players.push(host);
		for _ in 0..MAX_PLAYERS - 1 {
			lobby.add_ai();
		}
		
		debug!(cx = %lobby.cx(), host = account_id, "Creating multiplayer game");
		(lobby.spawn_lobby_rt(state, err_q), code)
	}
	
	pub fn spawn_lobby_rt(
		mut self,
		state: Arc<State>,
		mut err_q: UnboundedReceiver<(u8, NetError)>,
	) -> LobbyRtHandle {
		debug!(cx = %self.cx(), "Spawning lobby runtime");
		let (join_tx, mut join_rx) = channel(MAX_PLAYERS);
		let handle = tokio::spawn(async move {
			let timeout_res = timeout(Duration::from_hours(1), async {
				loop {
					tokio::select! {
						Some(message) = Player::try_recv_fallback(&mut self.lobby_state.players) => self.handle_fallback(message),
						Some(conn) = join_rx.recv() => self.set_next_client(conn, state.clone()),
						Some(err) = err_q.recv() => self.handle_err(err),
						else => {
							debug!(cx = %self.cx(), "Terminating lobby");
							state.rm_lobby(
								self.lobby_state.rules.code.clone(),
								self.lobby_state.rules.password.clone()
							).await;
							break;
						}
					}
				}
			}).await;
			if timeout_res.is_err() {
				debug!(cx = %self.cx(), "Terminating lobby");
				state.rm_lobby(
					self.lobby_state.rules.code.clone(),
					self.lobby_state.rules.password.clone()
				).await;
			}
			(self, err_q)
		});
		LobbyRtHandle {join_tx, handle}
	}
	
	fn add_human(&mut self) {
		self.lobby_state
			.players
			.push(Player::new_human_empty(self.lobby_state.players.len() as u8 & 1));
	}
	
	/// Updates a player's status as AI or human.
	fn update_human(&mut self, index: usize, is_human: bool) {
		self.lobby_state.players[index].player_type = if is_human {
			PlayerType::HumanEmpty
		} else {
			PlayerType::Ai { account_id: 0 }
		};
	}
	
	/// If there are any HumanEmpty slots, the conn will take up the next slot.
	/// If there are no HumanEmpty slots, the conn will replace an AI.
	/// If there are no available slots, the conn will be sent back to the server.
	fn set_next_client(&mut self, mut conn: Box<ClientConnection>, state: Arc<State>) {
		let mut next_idx = usize::MAX;
		for (i, player) in self.lobby_state.players.iter_mut().enumerate() {
			match player.player_type {
				PlayerType::HumanEmpty => {
					next_idx = i;
					break;
				}
				PlayerType::Ai {..} if next_idx == usize::MAX => next_idx = i,
				_ => {}
			}
		}
		if next_idx != usize::MAX {
			let account_id = conn.account_id;
			debug!(cx = %self.cx(), account_id, "Player joined lobby at index {next_idx}");
			let player = &mut self.lobby_state.players[next_idx];
			let player_idx = next_idx as u8;
			player.player_type = PlayerType::HumanConnection {conn};
			let username = player.username();
			// send snapshot to rejoined player
			self.lobby_state.notify(LobbyPayload::Lobby {lobby: Box::new(self.lobby_state.snapshot()), player_idx}, player_idx);
			// send rejoin update to all other players
			self.lobby_state.notify_all_but(LobbyPayload::PlayerJoined {player_idx, username}, player_idx);
			return;
		}
		error!(cx = %self.cx(), "A player tried joining but the game was at capacity");
		_ = conn.send_err(Error::GameAtCapacity);
		conn.spawn_server_handler(state);
	}
	
	/// Returns true if the game is starting or terminating
	fn handle_fallback(&mut self, (player_idx, message): (u8, Result<Ssp, NetError>)) {
		let ssp = match message {
			Ok(ssp) => ssp,
			Err(e) => {
				self.lobby_state.err_q.send((player_idx, e)).unwrap();
				return;
			}
		};
		match ssp.payload.try_lobby() {
			Ok(LobbyPayload::LobbyRules {player_idx, rules}) => {
			
			}
			Ok(LobbyPayload::AddPlayer) => {
			
			}
			Ok(LobbyPayload::RemovePlayer {player_idx}) => {
			
			}
			Ok(LobbyPayload::SwapPlayers {player_idx_a, player_idx_b}) => {
			
			}
			Ok(LobbyPayload::ReinsertPlayer {player_idx, new_player_idx}) => {
			
			}
			Ok(LobbyPayload::UpdatePlayerTeam {player_idx, team}) => {
			
			}
			Ok(LobbyPayload::UpdatePlayerType {player_idx, is_human}) => {
			
			}
			Ok(LobbyPayload::Ready {player_idx, is_ready}) => {
			
			}
			Ok(_) => self.lobby_state.err_q.send((player_idx, NetError::BadRequest)).unwrap(),
			Err(e) => self.lobby_state.err_q.send((player_idx, e)).unwrap(),
		}
		todo!()
	}
	
	fn is_host(&self, idx: u8) -> bool {
		let PlayerType::HumanConnection {conn} = &self.lobby_state.players[idx as usize].player_type else {
			return false;
		};
		conn.host
	}
	
	fn handle_err(&mut self, (player_idx, err): (u8, NetError)) {
		match err {
			NetError::Disconnected => {
				debug!(cx = %self.cx(), "Player at index {player_idx} disconnected");
				self.lobby_state.players[player_idx as usize].player_type = PlayerType::HumanEmpty;
				self.lobby_state.notify_all(LobbyPayload::PlayerDisconnected {player_idx});
			}
			e => self.lobby_state.notify(Payload::Err(e.into()), player_idx),
		}
	}
}