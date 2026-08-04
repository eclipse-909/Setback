use {
	crate::{ssp::*, *},
	dioxus::fullstack::{Websocket as Ws, body::Bytes, Encoding},
	reqwest::Client,
	std::{
		collections::HashMap,
		env,
		sync::Arc,
	},
	tracing::{error, info, warn},
	serde::{Serialize, de::DeserializeOwned}
};

pub fn setback_api() -> Option<Client> { Client::builder().cookie_store(true).build().ok() }

type ChainMap = HashMap<u32, UnboundedSender<Ssp>>;

pub type Websocket = Ws<Ssp, Ssp, PostcardEncoding>;
struct PostcardEncoding;
impl Encoding for PostcardEncoding {
	fn content_type() -> &'static str { APPLICATION_OCTET_STREAM }
	fn stream_content_type() -> &'static str { APPLICATION_OCTET_STREAM }
	fn encode(data: impl Serialize, buf: &mut Vec<u8>) -> Option<usize> {
		Some(postcard::to_io(&data, buf).ok()?.len())
	}
	fn decode<O: DeserializeOwned>(bytes: Bytes) -> Option<O> {
		postcard::from_bytes(&bytes).ok()
	}
}

/// Connection to the server, owned by the client.
///
/// Tokio runtime is required for all async operations.
/// However, this is already handled for you.
/// You do not need to spawn tasks on or enter the runtime.
/// The runtime will be entered and exited for each async operation defined here.
pub struct SetbackClient {
	writer: TlsWriter,
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
			.map_err(|_| Error::LogicError)?;
		info!("Logged into api");
		
		//Connect to server
		info!("Connecting to Setback server");
		let (mut reader, writer) = Self::make_tls_stream().await?;
		info!("Connected to Setback server");
		let chain_map = ChainMap::default();
		let chain_map_clone = chain_map.clone();
		let mut client = Self {
			writer,
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
						_ => unreachable!(),
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