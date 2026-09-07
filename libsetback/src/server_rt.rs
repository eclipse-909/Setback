use {
	arrayvec::ArrayVec,
	crate::*,
	scc::{HashMap, HashSet},
	futures::{FutureExt, TryFutureExt},
	std::{
		sync::Arc,
		time::Duration,
		future::poll_fn,
		task::{Poll, Context},
		pin::Pin,
	},
	tokio::{
		sync::mpsc::*,
		task::JoinHandle,
		time::sleep
	},
};

type ConnArr = ArrayVec<Box<ClientConnection>, MAX_PLAYERS>;
type ErrIdxArr = ArrayVec<(u8, NetError), MAX_PLAYERS>;

pub trait TryPromptFut<T: PromptResult>: Future<Output=(Result<T, PromptError>, PromptState)> + Send + 'static {}
impl<T: PromptResult, F: Future<Output=(Result<T, PromptError>, PromptState)> + Send + 'static> TryPromptFut<T> for F {}
pub trait NotifyManyFut = Future<Output=ErrIdxArr> + Send + 'static;

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

pub struct GameLoopHandle {
	pub rejoin_tx: Sender<Box<ClientConnection>>,
	pub handle: JoinHandle<ConnArr>,
}

impl GameState {
	pub fn spawn_server(
		mut self,
		reject_tx: UnboundedSender<Box<ClientConnection>>,
		mut err_q: UnboundedReceiver<(u8, NetError)>,
	) -> GameLoopHandle {
		let (rejoin_tx, mut rejoin_rx) = channel(MAX_PLAYERS);
		let handle = tokio::spawn(async move {
			let mut client_io_fut = self.advance_game(ClientIOResult::None);
			loop {
				tokio::select! {
					client_io_res = &mut client_io_fut => {
						if matches!(client_io_res, ClientIOResult::GameOver) {
							return self.collect_players();
						}
						client_io_fut = self.advance_game(client_io_res);
					}
					message = self.recv_fallback() => self.handle_fallback(message),
					Some(conn) = rejoin_rx.recv() => self.rejoin_player(conn, &reject_tx),
					Some((idx, err)) = err_q.recv() => {
						match err {
							NetError::Disconnected => self.disconnect_player(idx, true),
							e => self.notify(Payload::Err(e.into())),
						}
					}
				}
			}
		});
		GameLoopHandle {rejoin_tx, handle}
	}
	
	fn advance_game(&mut self, io_res: ClientIOResult) -> ClientIOFut {
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
						self.notify_all(Payload::GameEvent(GameEvent::RoundScores(round_scores.clone())));
					}
					PlayedCardResult::GameOver(round_scores) => {
						self.loop_state = GameLoopState::GameOver;
						self.notify_all(Payload::GameEvent(GameEvent::RoundScores(round_scores.clone())));
					}
				}
			}
			ClientIOResult::GameOver => {
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
				self.notify_all_unique(|idx,_| Payload::GameEvent(GameEvent::Init { idx }));
				ClientIOFut::None
			}
			GameLoopState::DealCards => {
				self.deal_cards();
				self.loop_state = GameLoopState::Bidding;
				self.increment_turn();
				let dealer_idx = self.dealer_idx;
				self.notify_all_unique(|_, player| Payload::GameEvent(GameEvent::DealCards {
					hand: player.hand.clone(),
					idx: dealer_idx,
				}));
				ClientIOFut::None
			}
			GameLoopState::Bidding => self.prompt::<Bid>(io_res),
			GameLoopState::Playing => self.prompt::<Card>(io_res),
			GameLoopState::GameOver => {
				self.loop_state = GameLoopState::Terminated;
				self.notify_all(Payload::GameEvent(GameEvent::GameOver));
				ClientIOFut::None
			}
			GameLoopState::Terminated => ClientIOFut::GameOver,
		}
		
	}
	
	fn collect_players(self) -> ConnArr {
		self.setup
			.players
			.into_iter()
			.filter_map(|p| match p.player_type {
				PlayerType::HumanConnection {conn} => Some(conn),
				_ => None,
			})
			.collect()
	}

	/// Makes a copy of the game loop without sensitive data like the player's hand or the deck.
	fn snapshot(&self) -> Box<Self> {
		Box::new(Self {
			// setup and deck are not fully cloned to prevent the user from accessing protected data
			setup: self.setup.snapshot(),
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
			err_q: self.err_q.clone(),
		})
	}
	
	fn start_turn_timer(&self) -> PromptState {
		PromptState {
			correlation_id: 0,
			timeout: Box::pin(sleep(Duration::from_secs(self.setup.rules.turn_time as u64))),
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
								.setup
								.players[self.turn_idx as usize]
								.player_type
							{
								conn.timeout_strikes += 1;
								if conn.timeout_strikes == TIMEOUT_STRIKES {
									let username = conn.username.clone();
									let report = ReportPlayerRequest {
										username,
										target_idx: self.turn_idx,
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
						PromptError::Net(NetError::BadRequest)
						| PromptError::Invalid => self.try_prompt::<T>(prompt_state),
					}
				}
			}
			None => self.try_prompt::<T>(self.start_turn_timer()),
		}
	}
	
	fn try_prompt<T: PromptResult>(&mut self, mut prompt_state: PromptState) -> ClientIOFut {
		let conn = match self.setup.players[self.turn_idx as usize].player_type {
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
	
	/// Notifies the current turn player
	fn notify(&mut self, payload: Payload) {
		let err_q = self.err_q.clone();
		let idx = self.turn_idx;
		let PlayerType::HumanConnection {conn} = &mut self
			.setup.players[idx as usize]
			.player_type
		else {return};
		let fut = conn.send_expect_ack(payload).inspect_err(move |e| err_q.send((idx, *e)).unwrap());
		tokio::spawn(fut);
	}
	
	fn notify_all(&mut self, payload: Payload) {
		self.notify_all_but(payload, u8::MAX);
	}
	
	pub fn notify_all_but(&mut self, payload: Payload, exclude: u8) {
		self.notify_all_unique_but(move |_,_| payload.clone(), exclude);
	}
	
	fn notify_all_unique(&mut self, payload_fn: impl FnMut(u8, &Player) -> Payload) {
		self.notify_all_unique_but(payload_fn, u8::MAX);
	}
	
	fn notify_all_unique_but(&mut self, mut payload_fn: impl FnMut(u8, &Player) -> Payload, exclude: u8) {
		self.setup
			.players
			.iter_mut()
			.enumerate()
			.filter(|(i,_)| *i != exclude as usize)
			.for_each(|(i, player)| {
				let payload = payload_fn(i as u8, player);
				let PlayerType::HumanConnection {conn} = &mut player.player_type else {return;};
				let err_q = self.err_q.clone();
				let fut = conn.send_expect_ack(payload).inspect_err(move |e| err_q.send((i as u8, *e)).unwrap());
				tokio::spawn(fut);
			});
	}
	
	/// Attempts to let a player rejoin the game.
	fn rejoin_player(
		&mut self,
		conn: Box<ClientConnection>,
		reject_tx: &UnboundedSender<Box<ClientConnection>>
	) {
		if conn.account_id != 0 && let Some(player_idx) = self
			.setup
			.players
			.iter_mut()
			.position(|p| matches!(p.player_type, PlayerType::Ai {account_id} if conn.account_id == account_id))
		{
			let player = &mut self.setup.players[player_idx];
			player.player_type = PlayerType::HumanConnection {conn};
			let hand = player.hand.clone();
			let username = player.username();
			// send snapshot to rejoined player
			self.notify(Payload::Snapshot(self.snapshot(), hand));
			// send rejoin update to all other players
			let player_idx = player_idx as u8;
			self.notify_all_but(Payload::Reconnected {player_idx, username}, player_idx);
		} else {
			_ = reject_tx.send(conn);
		}
	}
	
	fn disconnect_player(&mut self, idx: u8, allow_reconnect: bool) {
		self.setup.players[idx as usize].disconnect(allow_reconnect);
		if !self
			.setup
			.players
			.iter()
			.any(|p| matches!(p.player_type, PlayerType::HumanConnection{..}))
		{
			self.loop_state = GameLoopState::Terminated;
		}
		self.notify_all(Payload::Disconnected {player_idx: idx});
	}
	
	/// Returns None if all channels are closed.
	async fn recv_fallback(&mut self) -> Option<(u8, Result<Ssp, NetError>)> {
		poll_fn(|cx| {
			let mut any_open = false;
			for (i, rx) in self
				.setup
				.players
				.iter_mut()
				.enumerate()
				.flat_map(|(i, p)| match &mut p.player_type {
					PlayerType::HumanConnection {conn} => Some((i, &mut conn.fallback_rx)),
					_ => None,
				})
			{
				match rx.poll_recv(cx) {
					Poll::Ready(Some(item)) => return Poll::Ready(Some((i as u8, item))),
					Poll::Ready(None) => return Poll::Ready(Some((i as u8, Err(NetError::Disconnected)))),
					Poll::Pending => any_open = true,
				}
			}
			if any_open {
				Poll::Pending
			} else {
				Poll::Ready(None)
			}
		}).await
	}
	
	fn handle_fallback(&mut self, message: Option<(u8, Result<Ssp, NetError>)>) {
		let Some((player_idx, message)) = message else {
			self.loop_state = GameLoopState::Terminated;
			return;
		};
		let ssp = match message {
			Ok(ssp) => ssp,
			Err(e) => return self.err_q.send((player_idx, e)).unwrap(),
		};
		match ssp.payload {
			Payload::ReportPlayer(report) => self.report_player(player_idx, report, ssp.correlation_id),
			Payload::LeaveGame => self.disconnect_player(player_idx, false),
			_ => {}
		}
	}
	
	fn report_player(&mut self, reporter_idx: u8, report: ReportPlayerRequest, correlation_id: CorrelationId) {
		let target_is_human = matches!(
			self.setup
				.players[report.target_idx as usize]
				.player_type,
			PlayerType::HumanConnection {..}
		);
		let err_q = self.err_q.clone();
		let PlayerType::HumanConnection {conn} = &mut self
			.setup
			.players[reporter_idx as usize]
			.player_type
		else {return};
		if !target_is_human && correlation_id.id != 0 && !correlation_id.end {
			if let Err(err) = conn.send_response(Payload::Err(Error::BadPlayerIndex), correlation_id.id) {
				err_q.send((reporter_idx, err)).unwrap();
			}
			return;
		}
		let auth_token = conn.auth_token.clone();
		let ssp_tx = conn.ssp_tx().clone();
		tokio::spawn(async move {
			let response = async move {
				// TODO this will be a private api call
				// SAFETY:
				// Must link with server
				unsafe {&SETBACK_API}.post(URL_API_REPORT_PLAYER)
					.bearer_auth(auth_token)
					.json(&report)
					.send()
					.await
					.map_err(|_| Error::ApiErr)?
					.error_for_status()
					.map_err(Error::from)?;
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
}

pub type ClientStore = HashSet<u32>;
pub type GameSetupStore = HashMap<u32, SetupState>;
pub type GameRuntimeStore = HashMap<u32, GameLoopHandle>;

/// State for the setback server.
///
/// The client store holds clients that aren't in a game.
/// Clients in a game will be owned by that game in either
/// the game setup store or game runtime store.
pub struct State {
	pub client_store: ClientStore,
	pub game_setup_store: GameSetupStore,
	pub game_runtime_store: GameRuntimeStore,
}

impl State {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			client_store: HashSet::new(),
			game_setup_store: HashMap::new(),
			game_runtime_store: HashMap::new(),
		})
	}
}