use {
	crate::*,
	nanoid::nanoid,
	std::{
		time::Duration,
		sync::Arc,
		pin::Pin,
	},
	tracing::{debug, error},
	tokio::{
		sync::mpsc::{UnboundedReceiver, channel, unbounded_channel},
		time::{timeout, sleep, Sleep},
	},
};

impl Lobby {
	pub fn create_multiplayer(
		host: Box<ClientConnection>,
		password: String,
		state: Arc<State>,
	) -> (ClientSender, String) {
		let account_id = host.account_id;
		let host = Player::new_host(host, 0);
		
		// TODO we might need a different way to create game codes if there is a significant risk of collisions
		let code = nanoid!(GAME_CODE_LEN, &GAME_CODE_ALPHABET);
		
		let (join_tx, join_rx) = channel(MAX_PLAYERS);
		let (err_q_tx, err_q) = unbounded_channel();
		let mut lobby = Self::new(code.clone(), password, err_q_tx);
		lobby.lobby_state.players.push(host);
		for _ in 0..MAX_PLAYERS - 1 {
			lobby.add_ai();
		}
		
		debug!(cx = %lobby.cx(), host = account_id, "Creating multiplayer game");
		lobby.spawn_lobby_rt(state, err_q, join_rx);
		(join_tx, code)
	}
	
	pub fn spawn_lobby_rt(
		mut self,
		state: Arc<State>,
		mut err_q: UnboundedReceiver<(u8, NetError)>,
		mut join_rx: ClientReceiver,
	) {
		debug!(cx = %self.cx(), "Spawning lobby runtime");
		tokio::spawn(async move {
			let code = self.lobby_state.rules.code.clone();
			let password = self.lobby_state.rules.password.clone();
			let timeout_res = timeout(Duration::from_hours(1), async {
				let mut start_countdown = None;
				loop {
					tokio::select! {
						Some(message) = Player::try_recv_fallback(&mut self.lobby_state.players) => {
							self.handle_fallback(message, &mut start_countdown);
						}
						Some(conn) = join_rx.recv() => self.set_next_client(conn, state.clone()),
						Some(err) = err_q.recv() => self.handle_err(err),
						Some(()) = await_countdown(&mut start_countdown) => {
							self.lobby_state.notify_all(LobbyPayload::StartGame);
							if let Err((err, mut self_lobby)) = state.clone().start_game(self, err_q, join_rx).await {
								error!(cx = %self_lobby.cx(), %err, "Error starting game - terminating lobby");
								self_lobby.lobby_state.notify_all(Payload::Err(err));
								state.rm_lobby(
									self_lobby.lobby_state.rules.code,
									self_lobby.lobby_state.rules.password,
								).await;
							}
							return;
						}
						else => {
							debug!(cx = %self.cx(), "Terminating lobby");
							state.rm_lobby(
								self.lobby_state.rules.code,
								self.lobby_state.rules.password,
							).await;
							return;
						}
					}
				}
			}).await;
			if timeout_res.is_err() {
				let cx = LobbyLogCx { code: &code, public: password.is_empty() };
				debug!(%cx, "Terminating lobby - timed out");
				state.rm_lobby(code, password).await;
			}
		});
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
	
	fn handle_fallback(
		&mut self,
		(sender_idx, message): (u8, Result<Ssp, NetError>),
		start_countdown: &mut Option<Pin<Box<Sleep>>>,
	) {
		let ssp = match message {
			Ok(ssp) => ssp,
			Err(e) => {
				self.lobby_state.err_q.send((sender_idx, e)).unwrap();
				return;
			}
		};
		let id = ssp.correlation_id.id;
		match ssp.payload.try_lobby() {
			Ok(LobbyPayload::Ready {is_ready, ..}) => {
				if let PlayerType::HumanConnection {conn} = &mut self.lobby_state.players[sender_idx as usize].player_type {
					conn.ready = is_ready;
				}
				
				*start_countdown = if self.lobby_state
					.players
					.iter()
					.all(|p| match &p.player_type {
						PlayerType::HumanEmpty => false,
						PlayerType::HumanConnection {conn} => conn.ready,
						PlayerType::HumanPlayer {..} => unreachable!(),
						PlayerType::Ai {..} => true
					})
					&& self.can_start()
				{
					self.lobby_state.notify_all(LobbyPayload::ToggleCountdown {started: true});
					Some(Box::pin(sleep(Duration::from_secs(5))))
				} else {
					self.lobby_state.notify_all(LobbyPayload::ToggleCountdown {started: false});
					None
				};
				
				self.notify_lobby_update(sender_idx, LobbyPayload::Ready { is_ready, player_idx: sender_idx }, id);
			}
			Ok(mut lp) => {
				let PlayerType::HumanConnection {conn} = &mut self.lobby_state.players[sender_idx as usize].player_type else {
					return;
				};
				if !conn.host {
					if let Err(e) = conn.send_response(Payload::Err(Error::NotHost), id) {
						self.lobby_state.err_q.send((sender_idx, e)).unwrap();
					}
					return;
				}
				match &mut lp {
					LobbyPayload::LobbyRules {rules} => self.lobby_state.rules = rules.clone(),
					LobbyPayload::AddPlayer {is_human} => {
						if *is_human {
							self.lobby_state
								.players
								.push(Player::new_human_empty(self.lobby_state.players.len() as u8 & 1));
						} else {
							self.add_ai();
						}
					}
					LobbyPayload::RemovePlayer {player_idx} => self.remove_player(*player_idx),
					LobbyPayload::SwapPlayers {player_idx_a, player_idx_b} => self.swap_players(*player_idx_a, *player_idx_b),
					LobbyPayload::ReinsertPlayer {player_idx, new_player_idx} => self.reinsert_players(*player_idx, *new_player_idx),
					LobbyPayload::UpdatePlayerTeam {player_idx, team} => self.update_team(*player_idx, *team),
					LobbyPayload::UpdatePlayerType {player_idx, is_human} => {
						self.lobby_state.players[*player_idx as usize].player_type = if *is_human {
							PlayerType::HumanEmpty
						} else {
							PlayerType::Ai { account_id: 0 }
						};
					}
					_ => self.lobby_state.err_q.send((sender_idx, NetError::BadRequest)).unwrap(),
				}
				self.notify_lobby_update(sender_idx, lp, id);
			}
			Err(e) => self.lobby_state.err_q.send((sender_idx, e)).unwrap(),
		}
	}
	
	fn notify_lobby_update(&mut self, sender_idx: u8, lp: LobbyPayload, id: u32) {
		if let PlayerType::HumanConnection {conn} = &mut self.lobby_state.players[sender_idx as usize].player_type
			&& let Err(e) = conn.send_ack(id)
		{
			self.lobby_state.err_q.send((sender_idx, e)).unwrap();
		}
		self.lobby_state.notify_all_but(lp, sender_idx);
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

async fn await_countdown(start_countdown: &mut Option<Pin<Box<Sleep>>>) -> Option<()> {
	if let Some(c) = start_countdown {
		c.await;
		return Some(());
	}
	None
}