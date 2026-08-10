use {
	crate::{ssp::Payload, *},
	scc::HashMap,
	std::{sync::Arc, time::Duration},
	tokio::{sync::mpsc::*, task::JoinHandle},
	tokio_util::sync::CancellationToken,
	tracing::{error, info, warn},
};

pub struct GameLoopHandle {
	pub cancel_token: CancellationToken,
	pub rejoin_tx: Sender<Box<ClientConnection>>,
	pub task: JoinHandle<SmallVec<Box<ClientConnection>, MAX_PLAYERS>>,
}

impl GameState {
	fn spawn_server(self, reject_tx: UnboundedSender<Box<ClientConnection>>) -> GameLoopHandle {
		let cancel_token = CancellationToken::new();
		let cancel_token_clone = cancel_token.clone();
		let (rejoin_tx, rejoin_rx) = channel(MAX_PLAYERS);
		GameLoopHandle {
			cancel_token,
			rejoin_tx,
			task: tokio::spawn(self.run_event_loop(cancel_token_clone, rejoin_rx, reject_tx)),
		}
	}

	fn collect_players(self) -> SmallVec<Box<ClientConnection>, MAX_PLAYERS> {
		self.setup
			.players
			.into_iter()
			.filter_map(|mut p| {
				match std::mem::replace(p.player_type.get_mut(), PlayerType::Ai { account_id: 0 }) {
					PlayerType::HumanConnection { conn } => Some(conn),
					_ => None,
				}
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
		mut rejoin_rx: Receiver<Box<ClientConnection>>,
		reject_tx: UnboundedSender<Box<ClientConnection>>,
	) -> SmallVec<Box<ClientConnection>, MAX_PLAYERS> {
		let mut loop_state = GameLoopState::DealCards;
		let mut idx_payloads = (0..self.setup.players.len())
			.map(|idx| Payload::GameEvent(GameEvent::Init { idx }))
			.collect();
		self.notify_all_unique(
			&mut loop_state,
			&mut rejoin_rx,
			&reject_tx,
			&mut idx_payloads,
		)
		.await;
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
				unsafe {
					self.rejoin_player(&mut rejoin_prep, &reject_tx, rejoined)
						.await;
				}
				self.send_all_player_rejoined(
					rejoin_prep,
					&mut loop_state,
					&mut rejoin_rx,
					&reject_tx,
				)
				.await;
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
						.map(|p| {
							Payload::GameEvent(GameEvent::DealCards {
								hand: p.hand.clone(),
								idx: self.dealer_idx,
							})
						})
						.collect();
					self.notify_all_unique(
						&mut loop_state,
						&mut rejoin_rx,
						&reject_tx,
						&mut payloads,
					)
					.await;
					// no continue because it is currently the dealer's turn and we want the next player to start bidding.
				}
				GameLoopState::Bidding => {
					let bid = self
						.prompt(&mut loop_state, &mut rejoin_rx, &reject_tx)
						.await;
					if self.player_bid(bid, &mut loop_state) {
						continue;
					}
				}
				GameLoopState::Playing => {
					let card = self
						.prompt(&mut loop_state, &mut rejoin_rx, &reject_tx)
						.await;
					match self.player_played_card(card) {
						PlayedCardResult::RoundOver(round_scores) => {
							loop_state = GameLoopState::RoundOver;
							self.notify_all(
								&mut loop_state,
								&mut rejoin_rx,
								&reject_tx,
								Payload::GameEvent(GameEvent::RoundScores(round_scores)),
							)
							.await;
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
						Payload::GameEvent(GameEvent::GameOver),
					)
					.await;
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
		join_rx: &mut Receiver<Box<ClientConnection>>,
		reject_tx: &UnboundedSender<Box<ClientConnection>>,
	) -> T {
		// SAFETY:
		// we have exclusive access to self
		if matches!(
			unsafe { &*self.setup.players[self.turn_idx].player_type.get() },
			PlayerType::Ai { .. }
		) {
			return T::ai_play(self);
		}
		// SAFETY:
		// Ai was checked so this is HumanConnection.
		// send_prompt must only push self.turn_idx to disconnected_idxs iff that player disconnects,
		// and it preserves the ClientConnection object in that player to be later properly disconnected.
		unsafe {
			self.send_with_join(
				loop_state,
				join_rx,
				reject_tx,
				async move |this, disconnected_idxs| this.send_prompt(disconnected_idxs).await,
			)
			.await
		}
	}

	async fn notify_all(
		&mut self,
		loop_state: &mut GameLoopState,
		join_rx: &mut Receiver<Box<ClientConnection>>,
		reject_tx: &UnboundedSender<Box<ClientConnection>>,
		payload: Payload,
	) {
		// u8::MAX will never be a valid index, so this just skips a player that doesn't exist anyway
		const {
			assert!(MAX_PLAYERS > u8::MAX as usize);
		}
		self.notify_all_but(u8::MAX, loop_state, join_rx, reject_tx, payload)
			.await
	}

	async fn notify_all_but(
		&mut self,
		idx: u8,
		loop_state: &mut GameLoopState,
		join_rx: &mut Receiver<Box<ClientConnection>>,
		reject_tx: &UnboundedSender<Box<ClientConnection>>,
		payload: Payload,
	) {
		let payload_ref = &payload;
		// SAFETY:
		// Ais are skipped so these only send to HumanConnection players.
		// send_payload must only push player_idx to disconnected_idxs iff that player disconnects,
		// and it preserves the ClientConnection object in that player to be later properly disconnected.
		// Each player it sends to is a unique player in the array and disjoint in memory.
		unsafe {
			self.send_with_join(
				loop_state,
				join_rx,
				reject_tx,
				async move |this, disconnected_idxs| {
					futures::future::join_all((0..this.setup.players.len()).filter_map(|i| {
						if i != idx {
							Some(async move {
								// each future needs its own small vec because this future can't capture variables by mutable reference
								let mut disconn_idxs = SmallVec::new();
								this.send_payload(&mut disconn_idxs, i, payload_ref.clone())
									.await;
								disconn_idxs
							})
						} else {
							None
						}
					}))
					.await
					.into_iter()
					.for_each(|disconn_idxs| {
						disconn_idxs.into_iter().for_each(|i| {
							if !disconnected_idxs.iter().any(|ii| i == *ii) {
								debug_assert!(disconnected_idxs.len() < disconnected_idxs.cap());
								disconnected_idxs.push(i);
							}
						})
					});
				},
			)
			.await
		}
	}

	async fn notify_all_unique(
		&mut self,
		loop_state: &mut GameLoopState,
		join_rx: &mut Receiver<Box<ClientConnection>>,
		reject_tx: &UnboundedSender<Box<ClientConnection>>,
		payloads: &mut SmallVec<Payload, MAX_PLAYERS>,
	) {
		// SAFETY:
		// Ais are skipped so these only send to HumanConnection players.
		// send_payload must only push player_idx to disconnected_idxs iff that player disconnects,
		// and it preserves the ClientConnection object in that player to be later properly disconnected.
		// Each player it sends to is a unique player in the array and disjoint in memory.
		unsafe {
			self.send_with_join(
				loop_state,
				join_rx,
				reject_tx,
				async move |this, disconnected_idxs| {
					futures::future::join_all((0..this.setup.players.len()).map(|i| {
						let payload = std::mem::replace(&mut payloads[i], Payload::Ack);
						async move {
							// each future needs its own small vec because this future can't capture variables by mutable reference
							let mut disconn_idxs = SmallVec::new();
							this.send_payload(&mut disconn_idxs, i, payload).await;
							disconn_idxs
						}
					}))
					.await
					.into_iter()
					.for_each(|disconn_idxs| {
						disconn_idxs.into_iter().for_each(|i| {
							if !disconnected_idxs.iter().any(|ii| i == *ii) {
								debug_assert!(disconnected_idxs.len() < disconnected_idxs.cap());
								disconnected_idxs.push(i);
							}
						})
					});
				},
			)
			.await
		}
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
		join_rx: &mut Receiver<Box<ClientConnection>>,
		reject_tx: &UnboundedSender<Box<ClientConnection>>,
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
		self.send_all_player_rejoined(rejoin_prep, loop_state, join_rx, reject_tx)
			.await;
		for i in disconnected_idxs.into_iter() {
			// SAFETY:
			// We have exclusive access to self.
			// disconnected_idxs determines which players must be disconnected.
			// The functions that populate it must guarantee that the players at these indices are HumanConnection.
			unsafe {
				self.disconnect_player(loop_state, join_rx, reject_tx, i)
					.await
			}
		}
		output
	}

	async fn send_all_player_rejoined(
		&mut self,
		rejoin_prep: RejoinPrep,
		loop_state: &mut GameLoopState,
		join_rx: &mut Receiver<Box<ClientConnection>>,
		reject_tx: &UnboundedSender<Box<ClientConnection>>,
	) {
		for i in rejoin_prep.rejoined_idxs.into_iter() {
			Box::pin(
				self.notify_all(
					loop_state,
					join_rx,
					reject_tx,
					Payload::Reconnected {
						player_idx: i,
						// SAFETY:
						// We have exclusive access to self
						username: unsafe { &*self.setup.players[i].player_type.get() }
							.username()
							.to_string(),
					},
				),
			)
			.await
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
			account_ids: self
				.setup
				.players
				.iter()
				.map(|p| match unsafe { &*p.player_type.get() } {
					PlayerType::Ai { account_id } => *account_id,
					_ => 0,
				})
				.collect(),
			// SAFETY:
			// see above
			hands: self
				.setup
				.players
				.iter()
				.map(|p| match unsafe { &*p.player_type.get() } {
					PlayerType::Ai { account_id } if *account_id != 0 => p.hand.clone(),
					#[cfg(debug_assertions)]
					_ => {
						let mut v = SmallVec::new();
						v.push(Card::zero()); // this tell us when this hand is invalid
						v
					}
					#[cfg(not(debug_assertions))]
					_ => SmallVec::new(),
				})
				.collect(),
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
		reject_tx: &UnboundedSender<Box<ClientConnection>>,
		conn: Option<Box<ClientConnection>>,
	) {
		let Some(mut conn) = conn else {
			error!("rejoin channel closed unexpectedly");
			return;
		};
		debug_assert_ne!(conn.account_id, 0);
		debug_assert_eq!(rejoin_prep.account_ids.len(), rejoin_prep.hands.len());
		let Some((i, account_id)) = rejoin_prep
			.account_ids
			.iter_mut()
			.enumerate()
			.find(|(_, id)| **id == conn.account_id)
		else {
			reject_tx.send(conn).unwrap();
			return;
		};
		let Payload::Snapshot(_, ref mut ssp_hand) = rejoin_prep.snapshot_payload else {
			unreachable!("snapshot_ssp should be a snapshot")
		};
		std::mem::swap(ssp_hand, &mut rejoin_prep.hands[i as u8]);
		debug_assert!(ssp_hand.len() != 1 || ssp_hand[0] != Card::zero());
		if let Err(err) = conn
			.send_expect_ack(0, rejoin_prep.snapshot_payload.clone(), NETWORK_TIMEOUT)
			.await
		{
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
			std::debug_assert_matches!(&*ptr, PlayerType::Ai { .. });
			ptr.write(PlayerType::HumanConnection { conn });
		}
	}

	/// ## Safety
	/// Player at player_idx must be HumanConnection.
	async unsafe fn disconnect_player(
		&mut self,
		loop_state: &mut GameLoopState,
		join_rx: &mut Receiver<Box<ClientConnection>>,
		reject_tx: &UnboundedSender<Box<ClientConnection>>,
		player_idx: u8,
	) {
		warn!("player disconnected");
		{
			let human_conn = self.setup.players[player_idx].player_type.get_mut();
			let account_id = {
				let PlayerType::HumanConnection { conn } = human_conn else {
					unreachable!("Player should be human")
				};
				conn.account_id
			};
			*human_conn = PlayerType::Ai { account_id };
		}
		// SAFETY:
		// &mut self gives exclusive access
		if self
			.setup
			.players
			.iter()
			.all(|p| matches!(unsafe { &*p.player_type.get() }, PlayerType::Ai { .. }))
		{
			// all humans have disconnected
			*loop_state = GameLoopState::GameOver;
			return;
		}
		Box::pin(self.notify_all(
			loop_state,
			join_rx,
			reject_tx,
			Payload::Disconnected { player_idx },
		))
		.await;
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
	async unsafe fn send_prompt<T: PromptResult>(
		&self,
		disconnected_idxs: &mut SmallVec<u8, MAX_PLAYERS>,
	) -> T {
		const {
			assert!(
				size_of::<T>() != 0,
				"Prompt function shouldn't be used on `Unit`"
			);
		}
		debug_assert!(disconnected_idxs.len() < disconnected_idxs.cap());
		// SAFETY:
		// The future that receives rejoining players must not access any player_type that is HumanConnection
		let PlayerType::HumanConnection { conn } =
			(unsafe { &mut *self.setup.players[self.turn_idx].player_type.get() })
		else {
			unreachable!("Player should be human");
		};
		let mut correlation_id = 0;
		let timeout_res =
			tokio::time::timeout(Duration::from_secs(self.setup.turn_time as u64), async {
				loop {
					let prompt_res = conn.send_prompt(&mut correlation_id).await?;
					if T::validate(prompt_res, self) {
						return Ok(prompt_res);
					}
				}
			})
			.await;
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
	async unsafe fn send_payload(
		&self,
		disconnected_idxs: &mut SmallVec<u8, MAX_PLAYERS>,
		player_idx: u8,
		payload: Payload,
	) {
		debug_assert!(disconnected_idxs.len() < disconnected_idxs.cap());
		// SAFETY:
		// The future that receives rejoining players must not access any player_type that is HumanConnection
		let PlayerType::HumanConnection { conn } =
			(unsafe { &mut *self.setup.players[player_idx].player_type.get() })
		else {
			unreachable!("Player should be human");
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
pub trait PromptResult: Copy {
	const PROMPT_EVENT: GameEvent;
	fn validate(self, game_state: &GameState) -> bool;
	fn ai_play(game_state: &GameState) -> Self;
	fn extract_response(payload: Payload) -> Result<Self, Error>;
}

#[cfg(not(feature = "client"))]
impl PromptResult for Bid {
	const PROMPT_EVENT: GameEvent = GameEvent::PromptBid(None);
	#[inline(always)]
	fn validate(self, game_state: &GameState) -> bool {
		game_state.validate_bid(self)
	}
	#[inline(always)]
	fn ai_play(game_state: &GameState) -> Self {
		game_state.ai_bid()
	}
	fn extract_response(payload: Payload) -> Result<Self, Error> {
		match payload {
			Payload::GameEvent(GameEvent::PlayerBid { bid, .. }) => Ok(bid),
			Payload::Err(e) => Err(e),
			_ => Err(Error::BadProtocol),
		}
	}
}

#[cfg(not(feature = "client"))]
impl PromptResult for Card {
	const PROMPT_EVENT: GameEvent = GameEvent::PromptCard(None);
	#[inline(always)]
	fn validate(self, game_state: &GameState) -> bool {
		game_state.validate_card(self)
	}
	#[inline(always)]
	fn ai_play(game_state: &GameState) -> Self {
		game_state.ai_play_card()
	}
	fn extract_response(payload: Payload) -> Result<Self, Error> {
		match payload {
			Payload::GameEvent(GameEvent::PlayerPlayedCard { card, .. }) => Ok(card),
			Payload::Err(e) => Err(e),
			_ => Err(Error::BadProtocol),
		}
	}
}

#[cfg(not(feature = "client"))]
impl PromptResult for () {
	const PROMPT_EVENT: GameEvent = GameEvent::GameOver;
	#[inline(always)]
	fn validate(self, _game_state: &GameState) -> bool {
		true
	}
	#[inline(always)]
	fn ai_play(_game_state: &GameState) -> Self {}
	fn extract_response(_payload: Payload) -> Result<Self, Error> {
		Ok(())
	}
}

#[cfg(not(feature = "client"))]
struct RejoinPrep {
	account_ids: SmallVec<u32, MAX_PLAYERS>,
	hands: SmallVec<SmallVec<Card, CARDS_PER_PLAYER>, MAX_PLAYERS>,
	rejoined_idxs: SmallVec<u8, MAX_PLAYERS>,
	snapshot_payload: Payload,
}

pub type ClientStore = HashMap<u32, CancellationToken>;
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
			client_store: HashMap::with_capacity(MAX_CLIENTS),
			game_setup_store: HashMap::with_capacity(MAX_GAMES),
			game_runtime_store: HashMap::with_capacity(MAX_GAMES),
		})
	}
}
