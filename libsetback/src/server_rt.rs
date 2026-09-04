use {
	arrayvec::ArrayVec,
	crate::*,
	scc::{HashMap, HashSet},
	futures::FutureExt,
	std::{
		sync::Arc,
		time::Duration,
		future::poll_fn,
		task::Poll,
	},
	tokio::{
		sync::mpsc::*,
		task::{JoinHandle, JoinSet},
		time::sleep
	},
};

type ConnArr = ArrayVec<Box<ClientConnection>, MAX_PLAYERS>;
type ConnIdxArr = ArrayVec<(u8, Box<ClientConnection>), MAX_PLAYERS>;
type ErrIdxArr = ArrayVec<(u8, Error), MAX_PLAYERS>;

trait TryPromptFut<T: PromptResult>: Future<Output=(Result<T, Error>, PromptState)> + 'static {}
impl<T: PromptResult, F: Future<Output=(Result<T, Error>, PromptState)> + 'static> TryPromptFut<T> for F {}
trait NotifyManyFut = Future<Output=ErrIdxArr> + 'static;

enum ClientIOFut<B, C, N>
where
	B: TryPromptFut<Bid>,
	C: TryPromptFut<Card>,
	N: NotifyManyFut,
{
	None,
	PromptBid(B),
	PromptCard(C),
	NotifyMany(N),
}

impl<B, C, N> ClientIOFut<B, C, N>
where
	B: TryPromptFut<Bid>,
	C: TryPromptFut<Card>,
	N: NotifyManyFut,
{
	async fn result(self) -> ClientIOResult {
		match self {
			ClientIOFut::None => ClientIOResult::None,
			ClientIOFut::PromptBid(f) => {
				let (r, s) = f.await;
				ClientIOResult::PromptBid(r, s)
			}
			ClientIOFut::PromptCard(f) => {
				let (r, s) = f.await;
				ClientIOResult::PromptCard(r, s)
			}
			ClientIOFut::NotifyMany(f) => ClientIOResult::NotifyMany(f.await),
		}
	}
}

enum ClientIOResult {
	None,
	PromptBid(Result<Bid, Error>, PromptState),
	PromptCard(Result<Card, Error>, PromptState),
	NotifyMany(ErrIdxArr),
}

pub struct GameLoopHandle {
	pub rejoin_tx: Sender<Box<ClientConnection>>,
	pub task: JoinHandle<ConnArr>,
}

impl GameState {
	pub fn spawn_server(mut self, reject_tx: UnboundedSender<Box<ClientConnection>>) -> GameLoopHandle {
		let (rejoin_tx, mut rejoin_rx) = channel(MAX_PLAYERS);
		let task = tokio::spawn(async move {
			let mut client_io_fut = Box::pin(self.advance_game(ClientIOResult::None).result());
			loop {
				tokio::select! {
					Some(conn) = rejoin_rx.recv() => {
						if let Some(rejoin_fut) = self.rejoin_player(conn, &reject_tx) {
							rejoin_fut.await;
						}
					}
					client_io_res = &mut client_io_fut => {
						if matches!(client_io_res, ClientIOResult::None) {
							return self.collect_players(); // game over
						}
						client_io_fut = Box::pin(self.advance_game(client_io_res).result());
					}
					result = self.recv_fallback() => {
						// TODO
					}
				}
			}
		});
		GameLoopHandle {rejoin_tx, task}
	}
	
	fn advance_game(
		&mut self,
		io_res: ClientIOResult
	) -> ClientIOFut<impl TryPromptFut<Bid>, impl TryPromptFut<Card>, impl NotifyManyFut> {
		// TODO handle io_res
		loop {
			match self.loop_state {
				GameLoopState::Init => {
					self.loop_state = GameLoopState::DealCards;
					self.notify_all_unique(|idx,_| Payload::GameEvent(GameEvent::Init { idx })).await;
				}
				GameLoopState::DealCards => {
					self.deal_cards();
					self.loop_state = GameLoopState::Bidding;
					self.notify_all_unique(|_, player| Payload::GameEvent(GameEvent::DealCards {
						hand: player.hand.clone(),
						idx: self.dealer_idx,
					})).await;
					// no continue because it is currently the dealer's turn, and we want the next player to start bidding.
				}
				GameLoopState::Bidding => {
					let bid = self.try_prompt().await;
					if self.player_bid(bid, &mut self.loop_state) {
						continue;
					}
				}
				GameLoopState::Playing => {
					let card = self.try_prompt().await;
					match self.player_played_card(card) {
						PlayedCardResult::None => {}
						PlayedCardResult::TurnOver => continue,
						PlayedCardResult::RoundOver(round_scores) => {
							if self.team_won() {
								self.loop_state = GameLoopState::GameOver;
							} else {
								self.loop_state = GameLoopState::DealCards;
							}
							self.notify_all(Payload::GameEvent(GameEvent::RoundScores(round_scores))).await;
							continue;
						}
						PlayedCardResult::GameOver(round_scores) => {
							self.loop_state = GameLoopState::GameOver;
							self.notify_all(Payload::GameEvent(GameEvent::RoundScores(round_scores))).await;
							continue;
						}
					}
				}
				GameLoopState::GameOver => {
					self.loop_state = GameLoopState::Terminated;
					self.notify_all(Payload::GameEvent(GameEvent::GameOver)).await;
				}
				GameLoopState::Terminated => return ClientIOFut::None,
			}
			self.turn_idx = self.next_turn();
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
			loop_state: self.loop_state
		})
	}
	
	fn is_ai_turn(&self) -> bool {
		matches!(
			self.setup.players[self.turn_idx as usize].player_type,
			PlayerType::Ai { .. }
		)
	}
	
	fn start_turn_timer(&self) -> PromptState {
		PromptState {
			correlation_id: 0,
			timeout: Box::pin(sleep(Duration::from_secs(self.setup.rules.turn_time as u64))),
		}
	}
	
	// these functions are not marked async because we don't want to move self into the futures they return
	
	/// If it's an AI's turn, this function returns Ok(T) immediately.
	/// If it's a human's turn, this function returns Err(Future) immediately.
	/// The future will return with some information that must be validated and handled properly.
	/// This function should be called in a loop to reprompt the player until time runs out.
	fn try_prompt<T: PromptResult>(&mut self, mut state: PromptState) -> Result<T, impl TryPromptFut<T>> {
		let conn = match self.setup.players[self.turn_idx as usize].player_type {
			PlayerType::Ai {..} => return Ok(T::ai_play(self)),
			PlayerType::HumanConnection {ref mut conn} => conn,
			_ => unreachable!(),
		};
		let res_fut = conn.send_prompt(state.correlation_id);
		Err(async move {
			tokio::select! {
				result = res_fut => {
					match result {
						Ok((t, id)) => {
							state.correlation_id = id;
							(Ok(t), state)
						}
						Err(e) => (Err(e), state),
					}
				}
				_ = &mut state.timeout => (Err(Error::TimedOut), state),
			}
		})
	}
	
	fn notify(&mut self, payload: Payload) -> Option<impl NotifyFut> {
		let PlayerType::HumanConnection {conn} = &mut self
			.setup.players[self.turn_idx as usize]
			.player_type
		else {return None};
		Some(conn.send_expect_ack(payload))
	}
	
	fn notify_all(&mut self, payload: Payload) -> impl NotifyManyFut {
		self.notify_all_but(payload, u8::MAX)
	}
	
	fn notify_all_but(&mut self, payload: Payload, exclude: u8) -> impl NotifyManyFut {
		self.notify_all_unique_but(move |_,_| payload.clone(), exclude)
	}
	
	fn notify_all_unique(&mut self, payload_fn: impl FnMut(u8, &Player) -> Payload) -> impl NotifyManyFut {
		self.notify_all_unique_but(payload_fn, u8::MAX)
	}
	
	fn notify_all_unique_but(&mut self, mut payload_fn: impl FnMut(u8, &Player) -> Payload, exclude: u8) -> impl NotifyManyFut {
		let mut join_set = JoinSet::new();
		let mut idx_by_id = ArrayVec::<_, MAX_PLAYERS>::new_const();
		for (i, player) in self.setup.players.iter_mut().enumerate() {
			if i == exclude as usize {continue;}
			let payload = payload_fn(i as u8, player);
			let PlayerType::HumanConnection {conn} = &mut player.player_type else {continue;};
			let handle = join_set.spawn(
				conn.send_expect_ack(payload)
					.map(move |result| result.map_err(move |err| (i as u8, err)))
			);
			idx_by_id.push((handle.id(), i));
		}
		async move {
			let mut results = ArrayVec::new_const();
			while let Some(res) = join_set.join_next_with_id().await {
				match res {
					Ok((_, Ok(_))) => {}
					Ok((_, Err(err))) => results.push(err),
					Err(join_err) => {
						let idx = idx_by_id.iter().find(|(i, _)| *i == join_err.id()).unwrap().1;
						results.push((idx as u8, Error::LogicError))
					},
				}
			}
			results
		}
	}
	
	/// Attempts to let a player rejoin the game.
	fn rejoin_player(
		&mut self,
		conn: Box<ClientConnection>,
		reject_tx: &UnboundedSender<Box<ClientConnection>>
	) -> Option<impl NotifyManyFut> {
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
			let notify_snapshot_fut = self.notify(Payload::Snapshot(self.snapshot(), hand));
			// send rejoin update to all other players
			let player_idx = player_idx as u8;
			Some(self.notify_all_but(Payload::Reconnected {player_idx, username}, player_idx)
				.then(move |mut errors| async move {
					if let Some(notify_snapshot_fut) = notify_snapshot_fut && let Err(e) = notify_snapshot_fut.await {
						errors.push((player_idx, e));
					}
					errors
				}))
		} else {
			_ = reject_tx.send(conn);
			None
		}
	}
	
	fn disconnect_player(&mut self, idx: u8, allow_reconnect: bool) -> impl NotifyManyFut {
		self.setup.players[idx as usize].disconnect(allow_reconnect);
		self.check_human_remains();
		self.notify_player_disconnected(idx)
	}
	
	fn check_human_remains(&mut self) {
		if !self
			.setup
			.players
			.iter()
			.any(|p| matches!(p.player_type, PlayerType::HumanConnection{..}))
		{
			self.loop_state = GameLoopState::Terminated;
		}
	}
	
	fn notify_player_disconnected(&mut self, idx: u8) -> impl NotifyManyFut {
		self.notify_all(Payload::Disconnected {player_idx: idx})
	}
	
	fn handle_err(&mut self, err: Error) {
		todo!()
	}
	
	fn handle_errs(&mut self, errs: ErrIdxArr) {
		todo!()
	}
	
	/// Returns None if all channels are closed.
	async fn recv_fallback(&mut self) -> Option<(usize, Result<Ssp, Error>)> {
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
					Poll::Ready(Some(item)) => return Poll::Ready(Some((i, item))),
					Poll::Ready(None) => return Poll::Ready(Some((i, Err(Error::Disconnected)))),
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