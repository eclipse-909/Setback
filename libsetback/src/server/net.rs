use {
	crate::*,
	futures::{FutureExt, TryFutureExt},
	reqwest::Client,
	std::{
		pin::Pin,
		sync::Arc,
		time::Duration,
	},
	tokio::{
		net::TcpStream,
		sync::
		mpsc::unbounded_channel,
		time::Sleep,
	},
	tracing::debug,
};

// SAFETY:
// Must link with server
unsafe extern "Rust" {
	pub fn setback_api() -> &'static Client;
}

pub struct PromptState {
	pub correlation_id: u32,
	pub timeout: Pin<Box<Sleep>>,
}

pub trait TryPromptFut<T: PromptResult>: Future<Output=(Result<T, PromptError>, PromptState)> + Send + 'static {}
impl<T: PromptResult, F: Future<Output=(Result<T, PromptError>, PromptState)> + Send + 'static> TryPromptFut<T> for F {}

pub trait PromptFut<T: PromptResult>: Future<Output=Result<(T, u32), PromptError>> + Send +'static {}
impl<T: PromptResult, F: Future<Output=Result<(T, u32), PromptError>> + Send + 'static> PromptFut<T> for F {}

pub trait NotifyFut = Future<Output=Result<(), NetError>> + Send + 'static;

/// Connection to the client, owned by the server.
pub struct ClientConnection {
	state: Arc<State>,
	ssp_tx: SspTx,
	pub fallback_rx: FallbackRx,
	correlation_id: u32,
	pub owner_id: u32,
	pub account_id: u32,
	pub username: String,
	pub ready: bool,
	pub host: bool,
	pub timeout_strikes: u8,
}

impl WebSocketeer for ClientConnection {
	const INIT_CORRELATION_ID: u32 = 3; // DO NOT TOUCH
	fn ssp_tx(&self) -> &SspTx {&self.ssp_tx}
	fn correlation_id(&mut self) -> &mut u32 {&mut self.correlation_id}
}

impl ClientConnection {
	/// This is used when someone first tries to connect to the server.
	/// Their token will be validated. They are sent Ack if successful or Err
	/// if unauthenticated.
	pub async fn try_accept(tcp_stream: TcpStream, state: Arc<State>) {
		debug!("Client is connecting to server");
		let (mut reader, mut writer) = match ws::ws_accept(tcp_stream).await {
			Ok(s) => s,
			Err(err) => {
				debug!("Failed to accept tcp connection as websocket connection - Error: {err}");
				return;
			}
		};

		let res = async {
			// we can allow more time for authentication since this isn't delaying anything important
			let ssp: Ssp = tokio::time::timeout(Duration::from_secs(8), ws::ws_next(&mut reader))
				.await
				.map_err(|_| NetError::TimedOut)??;

			// get version and auth token
			let Payload::Authenticate { version, auth_token } = ssp.payload else {
				return Err(NetError::BadRequest.into());
			};
			// validate version
			version.check_compatible(SSP_VERSION)?;
			// validate token and get username
			let response = api_get_username(auth_token.as_str()).await?;
			ws::ws_send(&mut writer, &Ssp::new(Payload::Ack, CorrelationId::event())).await?;
			Ok(response)
		}.await;
		
		let UsernameResponse {username, owner_id, account_id} = match res {
			Ok(r) => r,
			Err(err) => {
				debug!("Authentication failed for client - Error: {err}");
				_ = ws::ws_send(
					&mut writer,
					&Ssp::new(Payload::Err(err), CorrelationId::event()),
				).await;
				return;
			}
		};
		
		let (ssp_tx, ssp_rx) = unbounded_channel();
		let (fallback_tx, fallback_rx) = unbounded_channel();
		tokio::spawn(Self::run_rw_handler(reader, writer, ssp_rx, fallback_tx));
		
		// This verifies the player isn't already connected to the server on any of their accounts
		if state.connected_account_owners.insert_sync(owner_id).is_err() {
			_ = Self::send_event_raw(&ssp_tx, Payload::Err(Error::AlreadyConnected));
			//TODO: if a client is trying to connect more than once,
			// it could either mean they are trying to cheat by being 2 players at once in a game,
			// or someone has hacked their account.
			// Either way we should do something with this.
			// Maybe send the client a message/notification/email that someone is trying to login to their account.
			// Suggest they should change their password.
			// Maybe have a "this was me" and "this wasn't me" response
			// which would confirm the cause.
			return;
		}
		
		let client = Self {
			ssp_tx,
			fallback_rx,
			correlation_id: Self::INIT_CORRELATION_ID,
			owner_id,
			account_id,
			username,
			ready: false,
			host: false,
			timeout_strikes: 0,
			state: state.clone(),
		};
		state.add_client(client).await;
	}

	/// Used when the client is connected to the server but not in a lobby or a game
	pub fn spawn_server_handler(mut self, state: Arc<State>) {
		tokio::spawn(async move {
			// DO NOT RETURN FROM THIS LOOP, USE BREAK INSTEAD
			loop {
				let Some(res) = self.fallback_rx.recv().await else { break };
				let ssp = match res {
					Ok(ssp) => ssp,
					Err(NetError::Disconnected | NetError::TimedOut) => break,
					Err(NetError::BadRequest | NetError::ServerError) => continue,
				};
				let payload = match ssp.payload.try_server() {
					Ok(p) => p,
					Err(e) => {
						todo!()
					}
				};
				match payload {
					ServerPayload::CreateGame {password} => {
					
					}
					ServerPayload::JoinGame {code, password} => {
					
					}
					ServerPayload::RejoinGame {code, password} => {
					
					}
					ServerPayload::SearchGames(filter) => {
					
					}
					_ => {}
				}
				todo!("handle the message using the state");
			}
			
			state.rm_client(self.account_id).await
		});
	}
	
	// these functions are not marked async because we don't want to move self into the futures they return
	
	// Generic payloads
	
	pub fn send_ack(&mut self, correlation_id: u32) -> Result<(), NetError> {
		self.send_response(Payload::Ack, correlation_id)
	}
	
	/// This is not responding to a specific client message and the client isn't responding to it.
	pub fn send_err(&mut self, err: Error) -> Result<(), NetError> {
		self.send_event(Payload::Err(err))
	}
	
	// In-game payloads
	
	/// Only use this when a client is in a game.
	/// This does not respond to the client; the client is still expecting a response.
	/// This function should be called in a loop to verify the response from the client,
	/// perform retries, and handle a timeout.
	/// Start the correlation id as 0. This function mutates it, then keep reusing it.
	pub fn send_prompt<T: PromptResult>(&mut self, correlation_id: u32) -> impl PromptFut<T> {
		// send prompt and get response
		let err = if correlation_id != 0 {
			Some(PromptError::Invalid)
		} else {
			None
		};
		let payload = GamePayload::GameEvent(T::prompt_event(err));
		self.send_request(payload, correlation_id)
			.map(|response| {
				let (response, id) = response?;
				// client should still expect message from server
				if id == 0 { return Err(NetError::BadRequest) }
				T::extract_response(response).map(|t| (t, id))
			})
			.map_err(PromptError::from)
	}
	
	// Lobby & in-game payloads
	
	/// Sends a message to the client and expects ack in response.
	/// This function does retries if there is an error,
	/// but otherwise sends no other messages after the client responds with Ack.
	/// It also has a timeout because it expects an immediate response.
	pub fn send_expect_ack(&mut self, payload: Payload) -> impl NotifyFut {
		let res_fut = self.send_new_request(payload.clone());
		let ssp_tx = self.ssp_tx.clone();
		tokio::time::timeout(
			Duration::from_secs(NETWORK_TIMEOUT),
			async move {
				let mut result = res_fut.await;
				loop {
					let (response, correlation_id) = result?;
					if correlation_id != 0 || !matches!(response, Payload::Ack) {
						result = Self::send_request_raw(
							&ssp_tx,
							Payload::ErrRetry(NetError::BadRequest.into(), Box::new(payload.clone())),
							correlation_id
						).await;
						continue;
					}
					return Ok(());
				}
			}
		).map(|res| res.map_err(|_| NetError::TimedOut).flatten())
	}
}

async fn api_get_username(auth_token: &str) -> Result<UsernameResponse, Error> {
	// SAFETY:
	// Must link with server
	unsafe {setback_api()}
		.get(URL_API_USERNAME)
		.bearer_auth(auth_token)
		.send()
		.await
		.map_err(|_| Error::ApiErr)?
		.error_for_status()
		.map_err(Error::from)?
		.json::<UsernameResponse>()
		.await
		.map_err(|_| NetError::ServerError.into())
}

pub async fn api_report_player(report: ReportPlayerRequest) -> Result<(), Error> {
	// SAFETY:
	// Must link with server
	unsafe {setback_api()}
		.post(url_api_report_player(report.target_account_id))
		.json(&report)
		.send()
		.await
		.map_err(|_| Error::ApiErr)?
		.error_for_status()
		.map_err(Error::from)?;
	Ok(())
}

pub async fn api_stats(stats: StatsRequest, account_id: u32) -> Result<(), Error> {
	// SAFETY:
	// Must link with server
	unsafe {setback_api()}
		.post(url_api_stats(account_id))
		.json(&stats)
		.send()
		.await
		.map_err(|_| Error::ApiErr)?
		.error_for_status()
		.map_err(Error::from)?;
	Ok(())
}

impl std::fmt::Debug for ClientConnection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ClientConnection")
			.field("username", &self.username)
			.field("ready", &self.ready)
			.field("host", &self.host)
			.finish()
	}
}

impl Drop for ClientConnection {
	fn drop(&mut self) {
		// allows the owner to connect with one of their accounts after they disconnect from this one
		self.state.connected_account_owners.remove_sync(&self.owner_id);
	}
}

#[cfg(feature = "server")]
pub trait PromptResult: Copy + Send {
	type PlayResult;
	fn prompt_event(err: Option<PromptError>) -> GameEvent;
	fn notify_prompt_payload(self, turn_idx: u8) -> GamePayload;
	fn is_valid(self, game_state: &GameState) -> bool;
	fn ai_play(game_state: &GameState) -> Self;
	fn extract_response(payload: Payload) -> Result<Self, NetError>;
	fn map_client_io_fut(future: impl TryPromptFut<Self>) -> ClientIOFut;
	fn notify_play(self, game_state: &mut GameState, correlation_id: u32) -> Self::PlayResult;
	fn match_io_res_prompt(io_res: ClientIOResult) -> Option<(Result<Self, PromptError>, PromptState)>;
	fn play_res_to_io_fut(play_res: Self::PlayResult) -> ClientIOFut;
	
	/// The correlation id and returned error are only used if it's a human's turn.
	fn notify_prompt_result(self, game_state: &mut GameState, correlation_id: u32) {
		if let PlayerType::HumanConnection {conn} = &mut game_state
			.lobby_state.players[game_state.turn_idx as usize]
			.player_type
			&& let Err(e) = conn.send_ack(correlation_id)
		{
			game_state.lobby_state.err_q.send((game_state.turn_idx, e)).unwrap();
		}
		game_state.lobby_state.notify_all_but(self.notify_prompt_payload(game_state.turn_idx), game_state.turn_idx);
	}
}

#[cfg(feature = "server")]
impl PromptResult for Bid {
	type PlayResult = ();
	fn prompt_event(err: Option<PromptError>) -> GameEvent {
		GameEvent::PromptBid(err)
	}
	fn notify_prompt_payload(self, turn_idx: u8) -> GamePayload {
		GamePayload::GameEvent(GameEvent::PlayerBid {bid: self, idx: turn_idx})
	}
	fn is_valid(self, game_state: &GameState) -> bool {
		game_state.bid_is_valid(self)
	}
	fn ai_play(game_state: &GameState) -> Self {
		game_state.ai_bid()
	}
	fn extract_response(payload: Payload) -> Result<Self, NetError> {
		match payload.try_game()? {
			GamePayload::GameEvent(GameEvent::PlayerBid { bid, .. }) => Ok(bid),
			_ => Err(NetError::BadRequest),
		}
	}
	fn map_client_io_fut(future: impl TryPromptFut<Self>) -> ClientIOFut {
		ClientIOFut::PromptBid(Box::pin(future))
	}
	fn notify_play(self, game_state: &mut GameState, correlation_id: u32)  -> Self::PlayResult {
		game_state.player_bid(self);
		self.notify_prompt_result(game_state, correlation_id);
	}
	fn match_io_res_prompt(io_res: ClientIOResult) -> Option<(Result<Self, PromptError>, PromptState)> {
		match io_res {
			ClientIOResult::PromptBid(r, s) => Some((r, s)),
			_ => None
		}
	}
	fn play_res_to_io_fut(_: Self::PlayResult) -> ClientIOFut { ClientIOFut::None }
}

#[cfg(feature = "server")]
impl PromptResult for Card {
	type PlayResult = PlayedCardResult;
	fn prompt_event(err: Option<PromptError>) -> GameEvent {
		GameEvent::PromptCard(err)
	}
	fn notify_prompt_payload(self, turn_idx: u8) -> GamePayload {
		GamePayload::GameEvent(GameEvent::PlayerPlayedCard {card: self, idx: turn_idx})
	}
	fn is_valid(self, game_state: &GameState) -> bool {
		game_state.card_is_valid(self)
	}
	fn ai_play(game_state: &GameState) -> Self {
		game_state.ai_play_card()
	}
	fn extract_response(payload: Payload) -> Result<Self, NetError> {
		match payload.try_game()? {
			GamePayload::GameEvent(GameEvent::PlayerPlayedCard { card, .. }) => Ok(card),
			_ => Err(NetError::BadRequest),
		}
	}
	fn map_client_io_fut(future: impl TryPromptFut<Self>) -> ClientIOFut {
		ClientIOFut::PromptCard(Box::pin(future))
	}
	fn notify_play(self, game_state: &mut GameState, correlation_id: u32) -> Self::PlayResult {
		let post_play = game_state.player_played_card(self);
		self.notify_prompt_result(game_state, correlation_id);
		post_play
	}
	fn match_io_res_prompt(io_res: ClientIOResult) -> Option<(Result<Self, PromptError>, PromptState)> {
		match io_res {
			ClientIOResult::PromptCard(r, s) => Some((r, s)),
			_ => None
		}
	}
	fn play_res_to_io_fut(play_res: Self::PlayResult) -> ClientIOFut { ClientIOFut::NotifyCardPlayed(play_res) }
}