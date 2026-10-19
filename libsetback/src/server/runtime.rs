use serde::{Deserialize, Serialize};
use {
	crate::*,
	arrayvec::ArrayVec,
	futures::FutureExt,
	sqlx::types::chrono::Utc,
	std::{
		pin::Pin,
		sync::Arc,
		task::{Context, Poll},
		time::Duration,
	},
	tokio::{
		sync::mpsc::*,
		time::sleep,
	},
	tracing::debug,
};

pub enum ClientIOFut {
	None,
	PromptBid(Pin<Box<dyn TryPromptFut<Bid>>>),
	PromptCard(Pin<Box<dyn TryPromptFut<Card>>>),
	NotifyCardPlayed(PlayedCardResult),
	GameOver,
}

impl Future for ClientIOFut {
	type Output = ClientIOResult;
	
	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		match self.get_mut() {
			ClientIOFut::None => Poll::Ready(ClientIOResult::None),
			ClientIOFut::PromptBid(f) => f.poll_unpin(cx).map(|(r, s)| ClientIOResult::PromptBid(r, s)),
			ClientIOFut::PromptCard(f) => f.poll_unpin(cx).map(|(r, s)| ClientIOResult::PromptCard(r, s)),
			ClientIOFut::NotifyCardPlayed(p) => Poll::Ready(ClientIOResult::NotifyCardPlayed(p.clone())),
			ClientIOFut::GameOver => Poll::Ready(ClientIOResult::GameOver),
		}
	}
}

pub enum ClientIOResult {
	None,
	PromptBid(Result<Bid, PromptError>, PromptState),
	PromptCard(Result<Card, PromptError>, PromptState),
	NotifyCardPlayed(PlayedCardResult),
	GameOver,
}

impl GameState {
	pub fn start_game(
		mut self,
		state: Arc<State>,
		mut err_q: UnboundedReceiver<(u8, NetError)>,
		mut rejoin_rx: ClientReceiver,
	) {
		debug!(cx = %self.cx(), "Starting game");
		tokio::spawn(async move {
			let mut log_buf = Vec::with_capacity(u8::MAX as usize);
			let mut client_io_fut = self.advance_game(ClientIOResult::None, &mut log_buf);
			loop {
				tokio::select! {
					client_io_res = &mut client_io_fut => {
						if matches!(client_io_res, ClientIOResult::GameOver) {
							debug!(cx = %self.cx(), "Terminating game");
							state.rm_game(
								self.lobby_state.rules.code.clone(),
								self.lobby_state.rules.password.clone(),
							).await;
							self.log_flush(&mut log_buf);
							self.lobby_state
								.players
								.into_iter()
								.filter_map(|p| match p.player_type {
									PlayerType::HumanConnection {conn} => Some(conn),
									_ => None,
								})
								.for_each(|conn| conn.spawn_server_handler());
							return;
						}
						client_io_fut = self.advance_game(client_io_res, &mut log_buf);
					}
					message = Player::try_recv_fallback(&mut self.lobby_state.players) => self.handle_fallback(message),
					Some(conn) = rejoin_rx.recv() => self.rejoin_player(conn),
					Some((idx, err)) = err_q.recv() => {
						match err {
							NetError::Disconnected => self.disconnect_player(idx, true),
							e => self.lobby_state.notify(Payload::Err(e.into()), self.turn_idx),
						}
					}
				}
			}
		});
	}
	
	fn advance_game(&mut self, io_res: ClientIOResult, log_buf: &mut Vec<u8>) -> ClientIOFut {
		match &io_res {
			ClientIOResult::NotifyCardPlayed(play_result) => {
				match play_result {
					PlayedCardResult::None => {}
					PlayedCardResult::RoundOver(round_scores) => {
						if self.team_won() {
							self.loop_state = GameLoopState::GameOver;
						} else {
							self.loop_state = GameLoopState::DealCards;
						}
						self.lobby_state.notify_all(GamePayload::GameEvent(GameEvent::RoundScores(round_scores.clone())));
					}
					PlayedCardResult::GameOver(round_scores) => {
						self.loop_state = GameLoopState::GameOver;
						self.lobby_state.notify_all(GamePayload::GameEvent(GameEvent::RoundScores(round_scores.clone())));
					}
				}
			}
			ClientIOResult::GameOver => {
				// this should be unreachable
				self.loop_state = GameLoopState::Terminated;
				return ClientIOFut::GameOver;
			}
			ClientIOResult::None
			// prompts will be handled when the loop state matches that prompt type
			| ClientIOResult::PromptBid(_, _)
			| ClientIOResult::PromptCard(_, _)
			=> {}
		}
		match self.loop_state {
			GameLoopState::Init => {
				self.loop_state = GameLoopState::DealCards;
				self.increment_turn();
				GameEvent::Init { idx: 0 }.log(log_buf);
				self.lobby_state.notify_all_unique(|idx,_| GamePayload::GameEvent(GameEvent::Init { idx }));
				ClientIOFut::None
			}
			GameLoopState::DealCards => {
				self.deal_cards();
				self.loop_state = GameLoopState::Bidding;
				self.increment_turn();
				let dealer_idx = self.dealer_idx;
				self.lobby_state.notify_all_unique(|_, player| {
					let event = GameEvent::DealCards {
						hand: player.hand.clone(),
						idx: dealer_idx,
					};
					event.log(log_buf);
					GamePayload::GameEvent(event)
				});
				ClientIOFut::None
			}
			GameLoopState::Bidding => self.prompt::<Bid>(io_res),
			GameLoopState::Playing => self.prompt::<Card>(io_res),
			GameLoopState::GameOver => {
				self.loop_state = GameLoopState::Terminated;
				self.lobby_state.notify_all(GamePayload::GameEvent(GameEvent::GameOver));
				ClientIOFut::GameOver
			}
			GameLoopState::Terminated => ClientIOFut::GameOver,
		}
	}

	/// Makes a copy of the game loop without sensitive data like the player's hand or the deck.
	fn snapshot(&self, player_idx: u8) -> Box<Self> {
		let mut snapshot = self.lobby_state.snapshot();
		snapshot.players[player_idx as usize].hand = self.lobby_state.players[player_idx as usize].hand.clone();
		Box::new(Self {
			// lobby and deck are not fully cloned to prevent the user from accessing protected data
			lobby_state: snapshot,
			deck: ArrayVec::new_const(),
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
			loop_state: self.loop_state,
		})
	}
	
	fn start_turn_timer(&self) -> PromptState {
		PromptState {
			correlation_id: 0,
			timeout: Box::pin(sleep(Duration::from_secs(self.lobby_state.rules.turn_time as u64))),
		}
	}
	
	// these functions are not marked async because we don't want to move self into the futures they return
	
	fn prompt<T: PromptResult>(&mut self, io_res: ClientIOResult) -> ClientIOFut {
		match T::match_io_res_prompt(io_res) {
			Some((prompt_result, prompt_state)) => match prompt_result {
				Ok(value) => {
					if value.is_valid(self) {
						let play_result = value.notify_play(self, prompt_state.correlation_id);
						T::play_res_to_io_fut(play_result)
					} else {
						self.try_prompt::<T>(prompt_state)
					}
				}
				Err(err) => {
					match err {
						PromptError::Net(NetError::Disconnected) => {
							self.disconnect_player(self.turn_idx, true);
							ClientIOFut::None
						}
						PromptError::Net(NetError::TimedOut) => {
							if let PlayerType::HumanConnection {conn} = &mut self
								.lobby_state
								.players[self.turn_idx as usize]
								.player_type
							{
								conn.timeout_strikes += 1;
								if conn.timeout_strikes == TIMEOUT_STRIKES {
									let username = conn.username.clone();
									let report = ReportPlayerRequest {
										game_code: self.lobby_state.rules.code.clone(),
										username,
										target_idx: self.turn_idx,
										target_account_id: conn.account_id,
										reason: ReportReason::Stalling,
										description: "Automatically reported for violating 3-strikes policy".to_string(),
									};
									// the player reports themselves which I think is very funny
									self.report_player(self.turn_idx, report, CorrelationId {id: 0, end: true});
									// yeet bozo
									self.disconnect_player(self.turn_idx, false);
								}
							}
							let play_result = T::ai_play(self).notify_play(self, 0);
							T::play_res_to_io_fut(play_result)
						}
						PromptError::Net(NetError::BadRequest | NetError::ServerError)
						| PromptError::Invalid
						=> self.try_prompt::<T>(prompt_state),
					}
				}
			}
			None => self.try_prompt::<T>(self.start_turn_timer()),
		}
	}
	
	fn try_prompt<T: PromptResult>(&mut self, mut prompt_state: PromptState) -> ClientIOFut {
		let conn = match self.lobby_state.players[self.turn_idx as usize].player_type {
			PlayerType::Ai {..} => {
				let play_result = T::ai_play(self).notify_play(self, prompt_state.correlation_id);
				return T::play_res_to_io_fut(play_result);
			}
			PlayerType::HumanConnection {ref mut conn} => conn,
			_ => unreachable!(),
		};
		let res_fut = conn.send_prompt(prompt_state.correlation_id);
		T::map_client_io_fut(async move {
			tokio::select! {
				result = res_fut => {
					match result {
						Ok((t, id)) => {
							prompt_state.correlation_id = id;
							(Ok(t), prompt_state)
						}
						Err(e) => (Err(e), prompt_state),
					}
				}
				_ = &mut prompt_state.timeout => (Err(NetError::TimedOut.into()), prompt_state),
			}
		})
	}
	
	/// Attempts to let a player rejoin the game.
	fn rejoin_player(
		&mut self,
		mut conn: Box<ClientConnection>,
	) {
		if conn.account_id != 0 && let Some(player_idx) = self
			.lobby_state
			.players
			.iter_mut()
			.position(|p| matches!(p.player_type, PlayerType::Ai {account_id} if conn.account_id == account_id))
		{
			let account_id = conn.account_id;
			debug!(cx = %self.cx(), account_id, "Player rejoined game at index {player_idx}");
			let player = &mut self.lobby_state.players[player_idx];
			let player_idx = player_idx as u8;
			player.player_type = PlayerType::HumanConnection {conn};
			let username = player.username();
			// send snapshot to rejoined player
			self.lobby_state.notify(GamePayload::Snapshot {game_state: self.snapshot(player_idx), player_idx}, player_idx);
			// send rejoin update to all other players
			self.lobby_state.notify_all_but(GamePayload::PlayerRejoined {player_idx, username}, player_idx);
			return;
		}
		_ = conn.send_err(Error::CannotRejoin);
		conn.spawn_server_handler();
	}
	
	fn disconnect_player(&mut self, idx: u8, allow_reconnect: bool) {
		debug!(cx = %self.cx(), "Player at index {idx} disconnected");
		self.lobby_state.players[idx as usize].disconnect(allow_reconnect);
		if !self
			.lobby_state
			.players
			.iter()
			.any(|p| matches!(p.player_type, PlayerType::HumanConnection{..}))
		{
			self.loop_state = GameLoopState::Terminated;
		}
		self.lobby_state.notify_all(GamePayload::PlayerDisconnected {player_idx: idx});
	}
	
	fn handle_fallback(&mut self, message: Option<(u8, Result<Ssp, NetError>)>) {
		let Some((player_idx, message)) = message else {
			self.loop_state = GameLoopState::Terminated;
			return;
		};
		let ssp = match message {
			Ok(ssp) => ssp,
			Err(e) => return self.lobby_state.err_q.send((player_idx, e)).unwrap(),
		};
		match ssp.payload.try_game() {
			Ok(GamePayload::ReportPlayer(report)) => self.report_player(player_idx, report, ssp.correlation_id),
			Ok(GamePayload::LeaveGame) => self.disconnect_player(player_idx, false),
			_ => {}
		}
	}
	
	fn report_player(
		&mut self,
		reporter_idx: u8,
		report: ReportPlayerRequest,
		correlation_id: CorrelationId,
	) {
		let target_is_human = matches!(
			self.lobby_state
				.players[report.target_idx as usize]
				.player_type,
			PlayerType::HumanConnection {..}
		);
		let err_q = self.lobby_state.err_q.clone();
		let PlayerType::HumanConnection {conn} = &mut self
			.lobby_state
			.players[reporter_idx as usize]
			.player_type
		else {return};
		if !target_is_human && correlation_id.id != 0 && !correlation_id.end {
			if let Err(err) = conn.send_response(Payload::Err(Error::BadPlayerIndex), correlation_id.id) {
				err_q.send((reporter_idx, err)).unwrap();
			}
			return;
		}
		let ssp_tx = conn.ssp_tx().clone();
		tokio::spawn(async move {
			let response = async move {
				// DontLikePlayer isn't a real report reason,
				// it's just to make players feel better that they think they can report people they don't like.
				if !matches!(report.reason, ReportReason::DontLikePlayer) {
					todo!("insert report into db");
				}
				Ok(())
			}.await;
			if correlation_id.id != 0 && !correlation_id.end {
				let payload = if let Err(err) = response {
					Payload::Err(err)
				} else {
					Payload::Ack
				};
				if let Err(err) = ClientConnection::send_response_raw(&ssp_tx, payload, correlation_id.id) {
					err_q.send((reporter_idx, err)).unwrap();
				}
			}
		});
	}
	
	fn log_flush(&mut self, buf: &mut Vec<u8>) {
		todo!("append buffer to game.events");
		buf.clear();
	}
	
	/// Don't use this, this is just to prevent errors in the ide
	pub async fn run_singleplayer(
		self,
		_event_emitter: impl EventEmitter,
		_event_response_rx: impl EventResponseReceiver,
	) {}
}

#[derive(Deserialize)]
pub struct LoggedGameEvent {
	pub event: GameEvent,
	pub timestamp_ms: i64
}

#[derive(Serialize)]
pub struct LoggedGameEventRef<'e> {
	pub event: &'e GameEvent,
	pub timestamp_ms: i64
}

impl LoggedGameEvent {
	pub fn decode_all(buf: &[u8]) -> Result<Vec<Self>, NetError> {
		let mut cursor = 0;
		let mut events = Vec::new();
		while cursor < buf.len() {
			let size = buf[cursor];
			cursor += 1;
			let end = cursor + size as usize;
			let event = postcard::from_bytes(&buf[cursor..end])
				.map_err(|_| NetError::ServerError)?;
			events.push(event);
			cursor = end;
		}
		Ok(events)
	}
}

impl GameEvent {
	fn log(&self, buf: &mut Vec<u8>) {
		let event = LoggedGameEventRef {
			event: self,
			timestamp_ms: Utc::now().timestamp_millis(),
		};
		let header_idx = buf.len();
		buf.push(0);
		*buf = postcard::to_extend(&event, std::mem::take(buf)).unwrap();
		let size = buf.len() - header_idx - 1;
		assert!(size < u8::MAX as usize);
		buf[header_idx] = size as u8;
	}
}