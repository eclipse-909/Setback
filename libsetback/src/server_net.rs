use {
	crate::*,
	reqwest::Client,
	futures::TryFutureExt,
	std::{
		cell::LazyCell,
		net::SocketAddr,
		pin::Pin,
		sync::Arc,
		time::Duration
	},
	tokio::{
		net::TcpStream,
		sync::{
			mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
			oneshot::Sender
		},
		time::Sleep
	},
	tracing::{error, info},
};

// SAFETY:
// Must link with server
unsafe extern "Rust" {
	static SETBACK_API: LazyCell<Client>;
}

pub struct PromptState {
	pub correlation_id: u32,
	pub timeout: Pin<Box<Sleep>>,
}

trait PromptFut<T: PromptResult>: Future<Output=Result<(T, u32), Error>> + 'static {}
impl<T: PromptResult, F: Future<Output=Result<(T, u32), Error>> + 'static> PromptFut<T> for F {}
pub trait NotifyFut = Future<Output=Result<(), Error>> + 'static;

/// Connection to the client, owned by the server.
pub struct ClientConnection {
	ssp_tx: UnboundedSender<(Option<Sender<Result<Ssp, Error>>>, Ssp)>,
	pub fallback_rx: UnboundedReceiver<Result<Ssp, Error>>,
	correlation_id: u32,
	pub account_id: u32,
	pub username: String,
	pub ready: bool,
	pub host: bool,
	pub timeout_strikes: u8,
}

impl WebSocketeer for ClientConnection {
	fn ssp_tx(&self) -> &UnboundedSender<(Option<Sender<Result<Ssp, Error>>>, Ssp)> {&self.ssp_tx}
	fn fallback_rx(&mut self) -> &mut UnboundedReceiver<Result<Ssp, Error>> {&mut self.fallback_rx}
	fn correlation_id(&mut self) -> &mut u32 {&mut self.correlation_id}
}

impl ClientConnection {
	/// This is used when someone first tries to connect to the server.
	/// Their token will be validated. They are sent Ack if successful or Err
	/// if unauthenticated.
	pub async fn new(tcp_stream: TcpStream, client_addr: SocketAddr) -> Option<Self> {
		let (mut reader, mut writer) = ws::ws_accept(tcp_stream).await.ok()?;
		
		//TODO:
		// Use client_addr for security features like IP bans

		let res = async {
			// we can allow more time for authentication since this isn't delaying anything important
			let ssp: Ssp = tokio::time::timeout(Duration::from_secs(8), ws::ws_next(&mut reader))
				.await
				.map_err(|_| Error::TimedOut)??;

			// get version and auth token
			let Payload::Authenticate { version, auth_token } = ssp.payload else {
				return Err(Error::BadProtocol);
			};
			// validate version
			version.check_compatible(SSP_VERSION)?;
			// validate token and get username
			let UsernameResponse {
				username,
				account_id,
			} = Self::get_username(auth_token.as_str()).await?;
			ws::ws_send(&mut writer, &Ssp::new(Payload::Ack, CorrelationId::event()))
				.await
				.map_err(Error::from)?;
			Ok((username, account_id))
		}.await;
		
		let (username, account_id) = match res {
			Ok(r) => r,
			Err(err) => {
				info!("Authentication failed for client - Error: {err}");
				ws::ws_send(
					&mut writer,
					&Ssp::new(Payload::Err(err), CorrelationId::event()),
				).await.ok()?;
				return None;
			}
		};
		
		let (ssp_tx, ssp_rx) = unbounded_channel();
		let (fallback_tx, fallback_rx) = unbounded_channel();
		tokio::spawn(Self::run_rw_handler(reader, writer, ssp_rx, fallback_tx));
		
		Some(Self {
			ssp_tx,
			fallback_rx,
			correlation_id: 3, // DO NOT TOUCH THIS 3, see WebSocketeer::correlation_id
			account_id,
			username,
			ready: false,
			host: false,
			timeout_strikes: 0,
		})
	}

	/// Called after client connects to server. Not used when client is in a game.
	pub fn spawn_handler(mut self, state: Arc<State>) {
		tokio::spawn(async move {
			if state.client_store.insert_async(self.account_id).await.is_err() {
				_ = self.send_err(Error::AlreadyConnected);
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
			
			// DO NOT RETURN, USE BREAK INSTEAD
			// TODO maybe use defer semantics to guarantee the client is removed from the store
			loop {
				let Some(res) = self.fallback_rx().recv().await else { break };
				todo!("handle the message using the state");
				match res {
					Ok(ssp) => {}
					Err(Error::Disconnected) => break,
					Err(err) => {}
				};
			}
			_ = state.client_store.remove_async(&self.account_id).await;
		});
	}
	
	// these functions are not marked async because we don't want to move self into the futures they return
	
	/// Only use this when a client is in a game.
	/// This does not respond to the client; the client is still expecting a response.
	/// This function should be called in a loop to verify the response from the client,
	/// perform retries, and handle a timeout.
	/// Start the correlation id as 0. This function mutates it, then keep reusing it.
	pub fn send_prompt<T: PromptResult>(&mut self, correlation_id: u32) -> impl PromptFut<T> {
		// send prompt and get response
		let mut payload = Payload::GameEvent(T::PROMPT_EVENT);
		if correlation_id != 0 {
			payload = Payload::ErrRetry(T::INVALID_ERR, Box::new(payload));
		}
		self.send_request(payload, correlation_id)
			.and_then(|(response, id)| async move {
				// client should still expect message from server
				if id == 0 { return Err(Error::BadRequest) }
				T::extract_response(response).map(|t| (t, id))
			})
	}
	
	/// Only use this when a client is in a game.
	/// Sends a message to the client and expects ack in response.
	/// This does not send another message to the client; the client should not expect a response.
	/// It also has a timeout because it expects an immediate response.
	pub fn send_expect_ack(&mut self, payload: Payload) -> impl NotifyFut {
		let res_fut = self.send_new_request(payload);
		async move {
			tokio::time::timeout(
				Duration::from_secs(NETWORK_TIMEOUT),
				res_fut.and_then(|(response, correlation_id)| async move {
					if correlation_id != 0 {
						return Err(Error::BadRequest);
					}
					match response {
						Payload::Ack => Ok(()),
						Payload::Err(e) => Err(e),
						_ => Err(Error::BadProtocol),
					}
				})
			).await
				.map_err(|_| Error::TimedOut)
				.flatten()
		}
	}
	
	/// This is not responding to a specific client message and the client isn't responding to it.
	pub fn send_err(&mut self, err: Error) -> Result<(), Error> {
		self.send_event(Payload::Err(err))
	}

	async fn get_username(auth_token: &str) -> Result<UsernameResponse, Error> {
		// SAFETY:
		// Must link with server
		unsafe {&SETBACK_API}.get(URL_API_USERNAME)
			.bearer_auth(auth_token)
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<UsernameResponse>()
			.await
			.map_err(|_| Error::ServerError)
	}

	//TODO this will be a private API call
	async fn report_player(
		auth_token: &str,
		report: &ReportPlayerRequest,
	) -> Result<(), Error> {
		// SAFETY:
		// Must link with server
		unsafe {&SETBACK_API}.post(URL_API_REPORT_PLAYER)
			.bearer_auth(auth_token)
			.json(report)
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?;
		Ok(())
	}
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

#[cfg(not(feature = "client"))]
pub trait PromptResult: Copy {
	const PROMPT_EVENT: GameEvent;
	const INVALID_ERR: Error;
	fn validate(self, game_state: &GameState) -> bool;
	fn ai_play(game_state: &GameState) -> Self;
	fn extract_response(payload: Payload) -> Result<Self, Error>;
}

#[cfg(not(feature = "client"))]
impl PromptResult for Bid {
	const PROMPT_EVENT: GameEvent = GameEvent::PromptBid(None);
	const INVALID_ERR: Error = Error::BadBid;
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
	const INVALID_ERR: Error = Error::BadCard;
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
	const INVALID_ERR: Error = Error::ServerError;
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