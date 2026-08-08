use {
	crate::{ssp::*, *},
	reqwest::Client,
	scc::hash_map::Entry,
	std::{
		net::SocketAddr,
		sync::Arc,
		time::Duration,
	},
	tokio::
	net::TcpStream,
	tokio_tungstenite::{MaybeTlsStream, tungstenite::protocol::WebSocketConfig},
	tokio_util::sync::CancellationToken,
	tracing::{error, info, warn},
};

/// Connection to the client, owned by the server.
pub struct ClientConnection {
	ws: Websocket,
	pub account_id: u32,
	pub username: String,
	pub ready: bool,
	pub host: bool,
	pub timeout_strikes: u8,
}

impl ClientConnection {
	/// This is used when someone first tries to connect to the server.
	/// Their token will be validated. They are sent Ack if successful or Err
	/// if unauthenticated.
	pub async fn new(
		tcp_stream: TcpStream,
		api: Client,
		client_addr: SocketAddr,
	) -> Option<Self> {
		let mut ws = match tokio_tungstenite::accept_async_with_config(
			MaybeTlsStream::Plain(tcp_stream),
			Some(
				// adjust these values as needed
				WebSocketConfig::default()
					.read_buffer_size(PAGE_SIZE)
					.write_buffer_size(PAGE_SIZE)
					.max_write_buffer_size(PAGE_SIZE * 2)
					.max_message_size(Some(PAGE_SIZE))
					.max_frame_size(Some(PAGE_SIZE))
			)
		).await {
			Ok(ws) => ws,
			Err(e) => {
				error!("WebSocket handshake failed: {e}");
				return None;
			}
		};
		
		//TODO:
		// Use client_addr for security features like IP bans
		
		let res = async {
			// we can allow more time for authentication since this isn't delaying anything important
			let ssp: Ssp = tokio::time::timeout(
				Duration::from_secs(8),
				ws_next(&mut ws)
			).await.map_err(|_| Error::TimedOut)??;
			
			// validate version
			ssp.version.check_compatible(SSP_VERSION)?;
			// get auth token
			let Payload::Authenticate {auth_token} = ssp.payload else {
				return Err(Error::BadProtocol);
			};
			// validate token and get username
			let UsernameResponse {username, account_id} = Self::get_username(api, auth_token.as_str()).await?;
			ws_send(&mut ws, &Ssp::new(Payload::Ack, CorrelationId::event())).await.map_err(Error::from)?;
			Ok((username, account_id))
		}.await;
		
		match res {
			Ok((username, account_id)) => Some(Self {
				ws,
				account_id,
				username,
				ready: false,
				host: false,
				timeout_strikes: 0,
			}),
			Err(err) => {
				info!("Authentication failed for client - Error: {err}");
				ws_send(&mut ws, &Ssp::new(Payload::Err(err), CorrelationId::event())).await.ok()?;
				None
			}
		}
	}
	
	/// Called after client connects to server. Not used when client is in a game.
	pub fn spawn_read_loop_for_server(mut self, state: Arc<State>) {
		tokio::spawn(async move {
			let cancel_token = CancellationToken::new();
			let cancel_token_clone = cancel_token.clone();
			match state.client_store.entry_async(self.account_id).await {
				Entry::Vacant(e) => {_ = e.insert_entry(cancel_token);}
				Entry::Occupied(_) => {
					_ = self.send_err(Error::AlreadyConnected).await;
					return;
				}
			}
			let account_id = self.account_id;
			tokio::select! {
				_ = self.handle_messages_for_server(state.clone()) => {}
				_ = cancel_token_clone.cancelled() => {}
			}
			_ = state.client_store.remove_async(&account_id).await;
		});
	}
	
	#[inline(always)]
	async fn handle_messages_for_server(mut self, state: Arc<State>) {
		loop {
			let (correlation_id, res) = self.read().await;
			if matches!(res, Err(Error::Disconnected)) { return; }
			let ssp = match res {
				Ok(ssp) => ssp,
				Err(err) => {
					_ = self.write(CorrelationId::response(correlation_id.id), Payload::Err(err)).await;
					continue;
				}
			};
			let res = match ssp.payload {
				_ => todo!("handle the message using the state")
			};
			_ = self.write(CorrelationId::response(correlation_id.id), res).await;
		}
	}
	
	/// Wraps [read] and validates the and version.
	/// This should always be used in the server instead of [read] for security.
	/// The chain id is the chain id from the received ssp.
	/// It's returned here just in case version checking fails and you need to respond.
	async fn read(&mut self) -> (CorrelationId, Result<Ssp, Error>) {
		let mut ssp = match ws_next(&mut self.ws).await {
			Ok(ssp) => ssp,
			Err(err) => return (CorrelationId::event(), Err(err)),
		};
		let correlation_id = if !ssp.correlation_id.end {
			CorrelationId::response(ssp.correlation_id.id)
		} else {
			CorrelationId::event()
		};
		if ssp.version.check_compatible(SSP_VERSION).is_err() {
			return (correlation_id, Err(Error::BadVersion));
		}
		ssp.src_id = self.account_id;
		(correlation_id, Ok(ssp))
	}
	
	/// Use CorrelationId::event() when sending a new message. The client will assign their own id if they respond.
	///
	/// Use the id provided by the client when responding.
	async fn write(&mut self, correlation_id: CorrelationId, payload: Payload) -> Result<(), Error> {
		ws_send(&mut self.ws, &Ssp::new(payload, correlation_id))
			.await
			.inspect_err(|e| {
				warn!("Failed to write data to stream. Error: {e}");
			})
	}
	
	/// Only use this when a client is in a game.
	/// This does not respond to the client; the client is still expecting a response.
	pub async fn send_prompt<T: PromptResult>(&mut self, correlation_id: &mut u32) -> Result<T, Error> {
		// send prompt
		self.write(CorrelationId::request(*correlation_id), Payload::GameEvent(T::PROMPT_EVENT)).await?;
		// get client response
		let (res_id, res) = self.read().await;
		// client should expect message from server
		if res_id.end || res_id.id == 0 || (res_id.id != *correlation_id && *correlation_id != 0) {
			return Err(Error::BadRequest);
		}
		*correlation_id = res_id.id;
		T::extract_response(res?.payload)
	}
	
	/// Only use this when a client is in a game.
	/// This does not respond to the client; the client is not expecting a response.
	/// This function does retries if there is an error.
	pub async fn send_expect_ack(&mut self, correlation_id: u32, payload: Payload, timeout_secs: u64) -> Result<(), Error> {
		let timeout_res = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
			loop {
				// send message to client
				if let Err(err) = self.write(CorrelationId::request(correlation_id), payload.clone()).await {
					warn!("Failed to send message to client. Error: {err}");
					continue;
				}
				// get response from client
				let (res_id, res) = self.read().await;
				// validate response
				let err = if res_id.end && res_id.id != 0 && res_id.id == correlation_id {
					match res?.payload {
						Payload::Ack => return Ok(()),
						Payload::Err(e) => e,
						_ => Error::BadProtocol,
					}
				} else {
					Error::BadRequest
				};
				warn!("Error receiving message from client. Error: {err}");
			}
		}).await;
		match timeout_res {
			Ok(Ok(())) => Ok(()),
			Ok(Err(err)) => Err(err),
			Err(_) => Err(Error::TimedOut),
		}
	}
	
	/// This is not responding to a specific client message and the client isn't responding to it.
	pub async fn send_err(&mut self, err: Error) -> Result<(), Error> {
		self.write(CorrelationId::event(), Payload::Err(err)).await
	}
	
	async fn get_username(api: Client, auth_token: &str) -> Result<UsernameResponse, Error> {
		api.get(URL_API_USERNAME)
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
	async fn report_player(api: Client, auth_token: &str, report: &ReportPlayerRequest) -> Result<(), Error> {
		api.post(URL_API_REPORT_PLAYER)
			.bearer_auth(auth_token)
			.json(report)
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?;
		Ok(())
	}
	
	pub fn as_human_player(&self) -> PlayerType {
		PlayerType::HumanPlayer { username: self.username.clone(), ready: self.ready }
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