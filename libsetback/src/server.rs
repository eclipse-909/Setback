use {
	crate::{ssp::*, *},
	futures::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}},
	reqwest::Client,
	scc::hash_map::Entry,
	std::{
		net::SocketAddr,
		sync::Arc,
		time::Duration,
	},
	tokio::
	net::TcpStream,
	tokio_tungstenite::{WebSocketStream, tungstenite::Message},
	tokio_util::{
		bytes::Bytes,
		sync::CancellationToken},
	tracing::{error, info, warn},
};

/// Connection to the client, owned by the server.
pub struct ClientConnection {
	reader: SplitStream<WebSocketStream<TcpStream>>,
	writer: SplitSink<WebSocketStream<TcpStream>, Message>,
	pub(crate) account_id: u32,
	pub(crate) username: String,
	pub(crate) ready: bool,
	pub(crate) host: bool,
	pub(crate) timeout_strikes: u8,
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
		// TODO: configure websocket for max message size and other stuff
		let ws = match tokio_tungstenite::accept_async(tcp_stream).await {
			Ok(ws) => ws,
			Err(e) => {
				error!("WebSocket handshake failed: {e}");
				return None;
			}
		};
		let (mut writer, mut reader) = ws.split();
		
		//TODO:
		// Use client_addr for security features like IP bans
		
		let res = async {
			// we can allow more time for authentication since this isn't delaying anything important
			let ssp: Ssp = tokio::time::timeout(
				Duration::from_secs(8),
				ws_next(&mut reader)
			).await.map_err(|_| Error::TimedOut)??;
			
			// validate version
			if !ssp.version.is_compatible(SSP_VERSION) {
				return Err(Error::BadVersion);
			}
			// get auth token
			let Payload::Authenticate {auth_token} = ssp.payload else {
				return Err(Error::BadProtocol);
			};
			// validate token and get username
			let UsernameResponse {username, account_id} = Self::get_username(api, auth_token.as_str()).await?;
			ws_send(&mut writer, Ssp::new(Payload::Ack, ChainId::EndOfChain(0))).await.map_err(Error::from)?;
			Ok((username, account_id))
		}.await;
		
		match res {
			Ok((username, account_id)) => Some(Self {
				writer,
				reader,
				account_id,
				username,
				ready: false,
				host: false,
				timeout_strikes: 0,
			}),
			Err(err) => {
				info!("Authentication failed for client - Error: {err}");
				ws_send(&mut writer, Ssp::new(Payload::Err(err), ChainId::EndOfChain(0))).await.ok()?;
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
			let (chain_id, res) = self.read().await;
			if matches!(res, Err(Error::Disconnected)) { return; }
			let ssp = match res {
				Ok(ssp) => ssp,
				Err(err) => {
					self.write(chain_id, Payload::Err(err)).await;
					continue;
				}
			};
			let ChainId::ExpectResponse(chain_id) = chain_id else {
				self.send_err(Error::BadRequest).await;
				continue;
			};
			let res = match ssp.payload {
				_ => todo!()
			};
			_ = self.write(ChainId::EndOfChain(chain_id), res).await;
		}
	}
	
	/// Wraps [read] and validates the and version.
	/// This should always be used in the server instead of [read] for security.
	/// The chain id is the chain id from the received ssp.
	/// It's returned here just in case version checking fails and you need to respond.
	async fn read(&mut self) -> (ChainId, Result<Ssp, Error>) {
		let mut ssp = match ws_next(&mut self.reader).await {
			Ok(ssp) => ssp,
			Err(err) => return (ChainId::None, Err(err)),
		};
		let chain_id = match ssp.chain_id {
			ChainId::ExpectResponse(id) => ChainId::EndOfChain(id),
			_ => ChainId::None
		};
		if !ssp.version.is_compatible(SSP_VERSION) {
			return (chain_id, Err(Error::BadVersion));
		}
		ssp.src_id = self.account_id;
		(chain_id, Ok(ssp))
	}
	
	/// Use ChainId::None when sending a new message. The client will assign their own id if they respond.
	///
	/// Use the id provided by the client when responding.
	async fn write(&mut self, chain_id: ChainId, payload: Payload) -> Result<(), Error> {
		ws_send(&mut self.writer, Ssp::new(payload, chain_id))
			.await
			.inspect_err(|e| {
				warn!("Failed to write data to stream. Error: {e}");
			})
	}
	
	/// Only use this when a client is in a game.
	/// This does not respond to the client; the client is still expecting a response.
	pub async fn send_prompt_bid(&mut self, chain_id: &mut u32) -> Result<Bid, Error> {
		self.write(ChainId::ExpectResponse(*chain_id), Payload::GameEvent(GameEvent::PromptBid(None))).await?;
		let (id, res) = self.read().await;
		let ChainId::ExpectResponse(i) = id else {
			return Err(Error::BadRequest);
		};
		if *chain_id != 0 && i != *chain_id {
			return Err(Error::BadRequest);
		}
		*chain_id = i;
		match res?.payload {
			Payload::GameEvent(GameEvent::PlayerBid {bid, ..}) => Ok(bid),
			Payload::Err(e) => Err(e),
			_ => Err(Error::BadProtocol),
		}
	}
	
	/// Only use this when a client is in a game.
	/// This does not respond to the client; the client is still expecting a response.
	pub async fn send_prompt_card(&mut self, chain_id: &mut u32) -> Result<Card, Error> {
		self.write(ChainId::ExpectResponse(*chain_id), Payload::GameEvent(GameEvent::PromptCard(None))).await?;
		let (id, res) = self.read().await;
		let ChainId::ExpectResponse(i) = id else {
			return Err(Error::BadRequest);
		};
		if *chain_id != 0 && i != *chain_id {
			return Err(Error::BadRequest);
		}
		*chain_id = i;
		match res?.payload {
			Payload::GameEvent(GameEvent::PlayerPlayedCard {card, ..}) => Ok(card),
			Payload::Err(e) => Err(e),
			_ => Err(Error::BadProtocol),
		}
	}
	
	/// Only use this when a client is in a game.
	/// This does not respond to the client; the client is not expecting a response.
	/// This function does retries if there is an error.
	pub async fn send_expect_ack(&mut self, chain_id: u32, payload: Payload, timeout_secs: u64) -> Result<(), Error> {
		let timeout_res = tokio::time::timeout(Duration::from_secs(timeout_secs), async {
			loop {
				if let Err(err) = self.write(ChainId::ExpectResponse(chain_id), payload.clone()).await {
					warn!("Failed to send message to client. Error: {err}");
					continue;
				}
				let (id, res) = self.read().await;
				let err = if let ChainId::EndOfChain(i) = id && chain_id != 0 && i == chain_id {
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
		self.write(ChainId::None, Payload::Err(err)).await
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

async fn ws_next(reader: &mut SplitStream<WebSocketStream<TcpStream>>) -> Result<Ssp, Error> {
	match reader.next().await
		.ok_or(Error::Disconnected)?
		.map_err(|_| Error::NetworkError)?
	{
		Message::Binary(bytes) => postcard::from_bytes(&bytes).map_err(|_| Error::BadEncoding),
		Message::Close(_) => Err(Error::Disconnected),
		_ => Err(Error::LogicError),
	}
}

/// Note: does not flush after sending
async fn ws_send(writer: &mut SplitSink<WebSocketStream<TcpStream>, Message>, ssp: Ssp) -> Result<(), Error> {
	//TODO: how do we avoid an allocation here?
	// Can we use some kind of memory pooling and deserialize to that?
	let bytes = postcard::to_stdvec(&ssp).map_err(|_| Error::BadEncoding)?;
	let bytes = Bytes::from(bytes);
	let msg = Message::Binary(bytes);
	writer.send(msg).await.map_err(|_| Error::NetworkError)
}