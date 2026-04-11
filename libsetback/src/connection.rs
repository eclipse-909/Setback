use {
	crate::{ssp::*, *},
	futures::{SinkExt, StreamExt},
	reqwest::Client,
	rustls::pki_types::ServerName,
	std::{
		collections::HashMap,
		sync::Arc,
		net::SocketAddr,
		time::Duration,
		env,
	},
	scc::hash_map::Entry,
	tokio::{
		io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
		net::TcpStream,
		sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
	},
	tokio_rustls::{
		TlsConnector,
		client::TlsStream,
		rustls::{ClientConfig, RootCertStore}
	},
	tokio_serde::{Framed, formats::Bincode},
	tokio_util::{
		sync::CancellationToken,
		codec::{FramedWrite, FramedRead, LengthDelimitedCodec}
	},
	tracing::{info, warn, error}
};

pub fn setback_api() -> Option<Client> { Client::builder().cookie_store(true).build().ok() }

type ChainMap = HashMap<u32, UnboundedSender<Ssp>>;

type SspWriter<W: AsyncWriteExt + Unpin> =
	Framed<FramedWrite<WriteHalf<W>, LengthDelimitedCodec>, (), Ssp, Bincode<(), Ssp>>;
type SspReader<R: AsyncReadExt + Unpin> =
	Framed<FramedRead<ReadHalf<R>, LengthDelimitedCodec>, Ssp, (), Bincode<Ssp, ()>>;

// The client needs a TlsStream to go through nginx
type TlsWriter = SspWriter<TlsStream<TcpStream>>;
type TlsReader = SspReader<TlsStream<TcpStream>>;

// The server just uses a regular TcpStream with nginx handling encryption
type TcpWriter = SspWriter<TcpStream>;
type TcpReader = SspReader<TcpStream>;

fn new_length_delimited_codec() -> LengthDelimitedCodec {
	type FrameLength = u16;
	LengthDelimitedCodec::builder()
		.max_frame_length(FrameLength::MAX as usize)
		.length_field_type::<FrameLength>()
		.length_field_length(size_of::<FrameLength>())
		.new_codec()
}

/// Connection to the server, owned by the client.
///
/// Tokio runtime is required for all async operations.
/// However, this is already handled for you.
/// You do not need to spawn tasks on or enter the runtime.
/// The runtime will be entered and exited for each async operation defined here.
pub struct SetbackClient {
	writer: TlsWriter,
	/// cancels the read loop
	cancel_token: CancellationToken,
	chain_map: ChainMap,
	api: Client,
	auth_token: String,
	renew_token: String,
	ready: bool,
}

impl SetbackClient {
	/// password must be plaintext.
	///
	/// The returned client is fully connected to the server or an error is returned.
	///
	/// The mpsc receiver that is returned is used to receive spontaneous/unprompted messages from the server.
	///
	/// If you send a message to the server that expects a response, you can just call the async function that sends the message,
	/// and awaiting that function will return the response from the server if all goes well.
	/// ## Panics
	/// If not in tokio runtime
	pub async fn new(credentials: LoginRequest) -> Result<(Self, UnboundedReceiver<Result<Ssp, Error>>), Error> {
		//login
		info!("Logging into api");
		let api = setback_api().unwrap();
		let session = api
			.post(URL_API_LOGIN)
			.json(&credentials)
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<LoginResponse>()
			.await
			.map_err(|_| Error::DevError)?;
		info!("Logged into api");

		//Connect to server
		info!("Connecting to Setback server");
		let (mut reader, writer) = Self::make_tls_stream().await?;
		info!("Connected to Setback server");
		let chain_map = ChainMap::default();
		let chain_map_clone = chain_map.clone();
		let mut client = Self {
			writer,
			cancel_token: CancellationToken::new(),
			chain_map,
			api,
			auth_token: session.auth_token.clone(),
			renew_token: session.renew_token,
			ready: false,
		};

		// Authenticate to get client port

		// No need to use chains yet because this handshake is required before arbitrary communication
		let auth_payload = Payload::Authenticate { auth_token: session.auth_token };
		info!("Authenticating with Setback server");
		client.write(ChainId::ExpectResponse(0), auth_payload.clone()).await?;
		let auth_res = reader.next().await.ok_or(Error::Disconnected)??;
		if !matches!(auth_res.chain_id, ChainId::EndOfChain(0)) {
			return Err(Error::ServerError);
		};
		if !matches!(auth_res.payload, Payload::Ack) {
			return Err(Error::ServerError);
		};
		info!("Authenticated with Setback server");

		// Listen for messages from server
		let (ssp_tx, ssp_rx) = unbounded_channel();
		tokio::spawn(Self::recv_ssps(reader, chain_map_clone, ssp_tx));

		Ok((client, ssp_rx))
	}

	async fn recv_ssps(
		mut reader: TlsReader,
		mut chain_map: ChainMap,
		ssp_tx: UnboundedSender<Result<Ssp, Error>>,
	) {
		info!("Listening for messages from server");
		loop {
			match reader.next().await {
				// Route to transmitter via chaining...
				Some(Ok(message)) if message.chain_id != ChainId::None => {
					match message.chain_id {
						ChainId::ExpectResponse(id) => {
							if let Some(chain_tx) = chain_map.get(&id) {
								chain_tx.send(message).unwrap();
							}
							// if the transmitter isn't in the hash map,
							// the request probably timed out,
							// and it was removed from the hash map
						}
						ChainId::EndOfChain(id) => {
							if let Some(chain_tx) = chain_map.remove(&id) {
								chain_tx.send(message).unwrap();
							}
							// if the transmitter isn't in the hash map,
							// the request probably timed out,
							// and it was removed from the hash map
						}
						_ => unreachable_unchecked!(),
					}
				}
				// ... or send to default transmitter
				Some(result) => ssp_tx.send(result.map_err(Error::from)).unwrap(),
				// disconnected
				None => break,
			}
		}
		info!("Stopped listening for messages from server");
	}

	async fn make_tls_stream() -> Result<(TlsReader, TlsWriter), Error> {
		let dns_name = match env::var("DNS_NAME") {
			Ok(name) => name,
			Err(env::VarError::NotPresent) => String::from(DEFAULT_SETBACK_DNS_NAME),
			Err(env::VarError::NotUnicode(var)) => {
				error!("Value for environment variable DNS_NAME is not a valid unicode string: {var:?}");
				return Err(Error::BadEnvVar);
			}
		};
		let mut root_cert_store = RootCertStore::empty();
		root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
		// Don't hover over or inspect       ^^^^^^^^^^^^^^^^
		// webpki_roots::TLS_SERVER_ROOTS. It's huge and will lag your IDE.
		let config = ClientConfig::builder()
			.with_root_certificates(root_cert_store)
			.with_no_client_auth();
		let connector = TlsConnector::from(Arc::new(config));
		let tcp_stream = TcpStream::connect(format!("{dns_name}:{SETBACK_SERVER_PORT}"))
			.await
			.map_err(|_| Error::NetworkError)?;
		let server_name = ServerName::try_from(dns_name).unwrap();
		let (r, w) = tokio::io::split(connector.connect(server_name, tcp_stream).await?);
		let codec = new_length_delimited_codec();
		Ok((
			Framed::new(FramedRead::new(r, codec.clone()), Bincode::default()),
			Framed::new(FramedWrite::new(w, codec.clone()), Bincode::default()),
		))
	}
	
	pub async fn shutdown(&mut self) {
		self.cancel_token.cancel();
		_ = self.writer.get_mut().get_mut().shutdown().await;
	}

	pub async fn logout(&self, #[cfg(not(feature = "test-dep"))] auth_token: &str) -> Result<(), Error> {
		info!("Logging out");
		#[cfg(not(feature = "test-dep"))]
		self.api
			.get(URL_API_LOGOUT)
			.bearer_auth(auth_token)
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(|_| Error::ServerError)?;
		info!("Logged out");
		Ok(())
	}

	/// Chain_id must not be ChainId::None. Removes the chain id from the chain map if Err is returned.
	async fn write(&mut self, chain_id: ChainId, payload: Payload) -> Result<(), Error> {
		self.writer.send(Ssp::new(payload, chain_id)).await.map_err(|e| {
			self.chain_map.remove(&chain_id.get_id());
			warn!("Failed to write Ssp to tcp socket. Error: {e}");
			Error::from(e)
		})
	}

	/// Starts a message chain where the chain id is passed into all messages
	/// and the receiver is used to await responses from the server.
	fn start_chain(&mut self) -> (ChainId, UnboundedReceiver<Ssp>) {
		//get new key
		let mut next_key = 0;
		//TODO: find a better way to make a unique key.
		// This loop will probably never run more than a few times, but still.
		let (tx, rx) = unbounded_channel();
		loop {
			if let std::collections::hash_map::Entry::Vacant(e) = self.chain_map.entry(next_key) {
				//create new channel
				e.insert_entry(tx);
				return (ChainId::ExpectResponse(next_key), rx);
			}
			next_key += 1;
		}
	}

	async fn send_recv(&mut self, payload: Payload) -> Result<Payload, Error> {
		let (chain_id, mut rx) = self.start_chain();
		self.write(chain_id, payload).await?;
		let ssp = rx.recv().await.ok_or({
			self.chain_map.remove(&chain_id.get_id());
			Error::Disconnected
		})?;
		debug_assert!(matches!(
			ssp.chain_id,
			ChainId::EndOfChain(i) if i == chain_id.get_id()
		));
		match ssp.payload {
			Payload::Err(err) => Err(err),
			_ => Ok(ssp.payload),
		}
	}

	pub async fn send_search_games(&mut self, filter: GameFilter) -> Result<Vec<PublicGame>, Error> {
		match self.send_recv(Payload::SearchGames(filter)).await? {
			Payload::PublicGames(games) => Ok(games),
			Payload::Err(err) => Err(err),
			_ => Err(Error::ServerError),
		}
	}

	async fn send_expect_game_setup(&mut self, payload: Payload) -> Result<Box<SetupState>, Error> {
		match self.send_recv(payload).await? {
			Payload::Game(game) => Ok(game),
			Payload::Err(err) => Err(err),
			_ => Err(Error::ServerError),
		}
	}

	pub async fn send_create_game(&mut self, password: String) -> Result<Box<SetupState>, Error> {
		self.send_expect_game_setup(Payload::CreateGame { password }).await
	}

	pub async fn send_join_game(&mut self, code: u32, password: String) -> Result<Box<SetupState>, Error> {
		self.send_expect_game_setup(Payload::JoinGame { code, password }).await
	}

	pub async fn send_rejoin_game(&mut self, code: u32, password: String) -> Result<Box<SetupState>, Error> {
		self.send_expect_game_setup(Payload::RejoinGame { code, password }).await
	}

	async fn send_expect_ack(&mut self, payload: Payload) -> Result<(), Error> {
		match self.send_recv(payload).await? {
			Payload::Ack => Ok(()),
			Payload::Err(err) => Err(err),
			_ => Err(Error::ServerError),
		}
	}

	/// Host-only
	pub async fn send_game_setup(&mut self, setup_data: SetupState) -> Result<(), Error> {
		self.send_expect_ack(Payload::Game(Box::new(setup_data))).await
	}

	pub async fn send_update_team(&mut self, player_idx: u8, team: u8) -> Result<(), Error> {
		self.send_expect_ack(Payload::UpdatePlayerTeam { player_idx, team }).await
	}

	/// Host-only
	pub async fn send_update_player_type(&mut self, player_idx: u8, is_human: bool) -> Result<(), Error> {
		self.send_expect_ack(Payload::UpdatePlayerType { player_idx, is_human }).await
	}

	pub async fn send_ready(&mut self, is_ready: bool) -> Result<(), Error> {
		self.send_expect_ack(Payload::Ready { is_ready, player_idx: 0 }).await
	}

	pub async fn send_game_event(&mut self, event: GameEvent) -> Result<(), Error> {
		self.send_expect_ack(Payload::GameEvent(event)).await
	}

	/// Host-only
	pub async fn send_add_player(&mut self) -> Result<(), Error> {
		self.send_expect_ack(Payload::AddPlayer).await
	}

	/// Host-only
	pub async fn send_remove_player(&mut self, player_idx: u8) -> Result<(), Error> {
		self.send_expect_ack(Payload::RemovePlayer { player_idx }).await
	}

	/// Host-only
	pub async fn send_swap_players(&mut self, player_idx_a: u8, player_idx_b: u8) -> Result<(), Error> {
		self.send_expect_ack(Payload::SwapPlayers { player_idx_a, player_idx_b }).await
	}

	/// Host-only
	pub async fn send_reinsert_player(&mut self, player_idx: u8, new_player_idx: u8) -> Result<(), Error> {
		self.send_expect_ack(Payload::ReinsertPlayer { player_idx, new_player_idx }).await
	}
}

impl Drop for SetbackClient {
	fn drop(&mut self) {
		self.cancel_token.cancel();
	}
}

impl std::fmt::Debug for SetbackClient {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ServerConnection")
			.field("ready", &self.ready)
			.finish()
	}
}

/// Connection to the client, owned by the server.
pub struct ClientConnection {
	writer: TcpWriter,
	reader: TcpReader,
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
		let (r, w) = tokio::io::split(tcp_stream);
		let codec = new_length_delimited_codec();
		let mut reader = Framed::new(
			FramedRead::new(r, codec.clone()),
			Bincode::default()
		);
		let mut writer = Framed::new(
			FramedWrite::new(w, codec.clone()),
			Bincode::default()
		);
		
		//TODO:
		// Use client_addr for security features like IP bans
		
		let res = async {
			// we can allow more time for authentication since this isn't delaying anything important
			let ssp: Ssp = tokio::time::timeout(Duration::from_secs(8), async {
				// get message
				match reader.next().await {
					Some(Ok(ssp)) => Ok(ssp),
					Some(Err(err)) => Err(Error::from(err)),
					None => Err(Error::Disconnected),
				}
			}).await.map_err(|_| Error::TimedOut)??;
			
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
			writer.send(Ssp::new(Payload::Ack, ChainId::EndOfChain(0))).await.map_err(Error::from)?;
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
				writer
					.send(Ssp::new(Payload::Err(err), ChainId::EndOfChain(0)))
					.await
					.ok()?;
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
		let mut ssp = match self.reader.next().await {
			Some(Ok(ssp)) => ssp,
			Some(Err(err)) => return (ChainId::None, Err(Error::from(err))),
			None => return (ChainId::None, Err(Error::Disconnected)),
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
		self.writer
			.send(Ssp::new(payload, chain_id))
			.await
			.map_err(|e| {
				warn!("Failed to write data to stream. Error: {e}");
				Error::from(e)
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