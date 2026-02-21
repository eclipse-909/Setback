use {
	crate::{*, stream::MemPool, ssp::*},
	reqwest::Client,
	rustls::pki_types::ServerName,
	scc::{HashMap, hash_map::Entry},
	std::{
		net::SocketAddr,
		sync::Arc
	},
	tokio::{
		io::{AsyncReadExt, AsyncWriteExt},
		net::TcpStream,
		runtime::Handle,
		sync::{
			RwLock, mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel}
		},
		task::JoinHandle
	},
	tokio_rustls::{
		TlsConnector,
		client::TlsStream,
		rustls::{ClientConfig, RootCertStore}
	},
	tokio_util::sync::CancellationToken
};

pub fn setback_api() -> Option<Client> { Client::builder().cookie_store(true).build().ok() }

type ChainMap = HashMap<u32, UnboundedSender<Ssp>>;
type SspWriter<W: AsyncWriteExt + Unpin> = stream::Writer<W>;
type SspReader<R: AsyncReadExt + Unpin> = stream::Reader<R>;
// The client needs a TlsStream to go through nginx
type TlsWriter = SspWriter<TlsStream<TcpStream>>;
type TlsReader = SspReader<TlsStream<TcpStream>>;
// The server just uses a regular TcpStream with nginx handling encryption
type TcpWriter = SspWriter<TcpStream>;
type TcpReader = SspReader<TcpStream>;

trait SspStream {
	fn chain_map(&self) -> &ChainMap;
	async fn write(&mut self, chain_id: ChainId, protocol: Payload) -> Result<(), Error>;

	/// Starts a message chain where the chain id is passed into all messages
	/// and the receiver is used to await responses from the server.
	async fn start_chain(&self) -> (ChainId, UnboundedReceiver<Ssp>) {
		//get new key
		let mut next_key: u32 = 0;
		//TODO: find a better way to make a unique key.
		// This loop will probably never run more than a few times, but still.
		let (tx, rx) = unbounded_channel();
		loop {
			if let Entry::Vacant(e) = self.chain_map().entry_async(next_key).await {
				//create new channel
				e.insert_entry(tx);
				return (ChainId::ExpectResponse(next_key), rx);
			}
			next_key += 1;
		}
	}

	async fn remove_chain(&self, id: u32) -> Option<UnboundedSender<Ssp>> {
		self.chain_map().remove_async(&id).await.map(|o|o.1)
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
	mem_pool: Arc<MemPool>,
	rt: Handle,
	/// cancels the read loop
	cancellation_token: CancellationToken,
	chain_map: ChainMap,
	api: Client,
	auth_token: String,
	renew_token: String,
	ready: bool,
}

impl SspStream for SetbackClient {
	fn chain_map(&self) -> &ChainMap { &self.chain_map }
	async fn write(&mut self, chain_id: ChainId, protocol: Payload) -> Result<(), Error> {
		let auth_token = self.auth_token.clone();
		self.writer.send(&Ssp::new_with_token(protocol, chain_id, auth_token), &self.mem_pool).await
	}
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
	/// ## PANICS
	/// If not in tokio runtime
	pub async fn new(credentials: LoginRequest, mem_pool: Arc<MemPool>) -> Result<(Self, UnboundedReceiver<Result<Ssp, Error>>), Error> {
		//login
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

		todo!("move this to setback workspace after this function returns Ok");
		// //Save username and password
		// std::thread::spawn(move || {
		// 	if let Err(err) = std::fs::write(USR_CRD, credentials.username.as_str()) {
		// 		error!("Error creating usr.crd file: {}", err);
		// 		return;
		// 	}
		// 	if let Err(err) = keyring::Entry::new(
		// 		KEYRING_SERVICE,
		// 		credentials.username.as_str()
		// 	).and_then(|entry| entry.set_password(credentials.password.as_str())) {
		// 		error!("Error setting password in keyring: {}", err);
		// 		return;
		// 	}
		// 	info!("Saved credentials successfully");
		// });

		//Connect to server
		let (reader, writer) = Self::make_tls_stream(SETBACKD_PORT).await?;
		let cancellation_token = CancellationToken::new();
		let read_cancel_token = cancellation_token.clone();
		let chain_map = ChainMap::default();
		let chain_map_clone = chain_map.clone();
		let mut client = Self {
			writer,
			mem_pool: mem_pool.clone(),
			rt: tokio::runtime::Handle::current(),
			cancellation_token,
			chain_map,
			api,
			auth_token: session.auth_token.clone(),
			renew_token: session.renew_token,
			ready: false,
		};

		// Authenticate to get client port
		// No need to use chains yet because this handshake is required before arbitrary communication
		let auth_ssp = Ssp::new(
			Payload::Authenticate { auth_token: session.auth_token },
			ChainId::ExpectResponse(0),
		);
		client.writer.send(&auth_ssp, &mem_pool).await?;
		let auth_res: Ssp = reader.recv(&mem_pool).await?;
		if !matches!(auth_res.chain_id, ChainId::EndOfChain(0)) {
			return Err(Error::ServerError);
		};
		let Payload::Connect {port} = auth_res.payload else {
			return Err(Error::ServerError);
		};

		// Then disconnect and reconnect to client port
		let (mut reader, writer) = Self::make_tls_stream(port).await?;

		// Reauthenticate - still don't need chaining
		writer.send(&auth_ssp, &mem_pool).await?;
		if !matches!(auth_res.chain_id, ChainId::EndOfChain(0)) {
			return Err(Error::ServerError);
		};
		if !matches!(reader.recv::<Ssp>(&mem_pool).await?.payload, Payload::Connect {..}) {
			return Err(Error::ServerError);
		}

		// Listen for messages from server
		let (ssp_tx, ssp_rx) = unbounded_channel();
		tokio::spawn(async move {
			tokio::select! {
				_ = Self::recv_ssps(reader, chain_map_clone, ssp_tx, mem_pool) => {}
				_ = read_cancel_token.cancelled() => {}
			}
		});

		Ok((client, ssp_rx))
	}

	async fn recv_ssps(mut reader: TlsReader, chain_map: ChainMap, ssp_tx: UnboundedSender<Result<Ssp, Error>>, mem_pool: Arc<MemPool>) {
		loop {
			match reader.recv(&mem_pool).await {
				// Route to transmitter via chaining...
				Ok(message @ Ssp {
					chain_id: ChainId::ExpectResponse(chain_id)
					| ChainId::EndOfChain(chain_id),
					..
				}) => {
					match message.chain_id {
						ChainId::ExpectResponse(_) => {
							if let Some(chain_tx) = chain_map.get_async(&chain_id).await {
								chain_tx.send(message).unwrap();
							}
							// if the transmitter isn't in the hash map,
							// the request probably timed out,
							// and it was removed from the hash map
						}
						ChainId::EndOfChain(_) => {
							if let Some((_, chain_tx)) = chain_map.remove_async(&chain_id).await {
								chain_tx.send(message).unwrap();
							}
							// if the transmitter isn't in the hash map,
							// the request probably timed out,
							// and it was removed from the hash map
						}
						_ => unsafe {std::hint::unreachable_unchecked()},
					}
				}
				// ... or send to default transmitter
				result => ssp_tx.send(result).unwrap(),
			}
		}
	}

	async fn make_tls_stream(port: u16) -> Result<(TlsReader, TlsWriter), Error> {
		// Don't hover over or inspect webpki_roots::TLS_SERVER_ROOTS.
		// It's huge and will lag your IDE.
		let mut root_cert_store = RootCertStore::empty();
		root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
		let config = ClientConfig::builder()
			.with_root_certificates(root_cert_store)
			.with_no_client_auth();
		let connector = TlsConnector::from(Arc::new(config));
		let dnsname = ServerName::try_from(DNS_NAME).unwrap();
		let tcp_stream = TcpStream::connect(server(port))
			.await
			.map_err(|_| Error::ServerDown)?;
		let tls_stream = connector.connect(dnsname, tcp_stream).await?;
		Ok(stream::new_stream(tls_stream))
	}

	pub async fn renew_token(&mut self) -> Result<(), Error> {
		//Client specifically doesn't have a renew method because I want the request to go through the server.
		//This way the ServerConnection.auth_token and renew_token get updated.
		//If the client tries to call renew via reqwest and not through the server, the tokens will not be updated in ServerConnection.
		self.auth_token = self
			.api
			.post(URL_API_RENEW)
			.bearer_auth(self.auth_token.clone())
			.json(&RenewRequest {
				renew_token: self.renew_token.clone(),
			})
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<RenewResponse>()
			.await
			.map_err(|_| Error::DevError)?
			.auth_token;
		Ok(())
	}

	pub async fn logout(&self, #[cfg(not(test))] auth_token: &str) -> Result<(), Error> {
		#[cfg(not(test))]
		self.api
			.get(URL_API_LOGOUT)
			.bearer_auth(auth_token)
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(|_| Error::ServerError)?;

		Ok(())
	}

	async fn send_expect_ack(&mut self, protocol: Payload) -> Result<(), Error> {
		let (chain_id, mut rx) = self.start_chain().await;
		self.write(chain_id, protocol).await?;
		match tokio::time::timeout(NETWORK_TIMEOUT, rx.recv()).await {
			Ok(Some(res)) => {
				debug_assert!(matches!(
					res.chain_id,
					ChainId::EndOfChain(i) if i == chain_id.id_unwrap()
				));
				match res.payload {
					Payload::Ack => Ok(()),
					Payload::Err(err) => Err(err),
					_ => Err(Error::ServerError),
				}
			}
			Ok(None) => Err(Error::Disconnected),
			Err(_) => {
				self.remove_chain(chain_id.id_unwrap());
				Err(Error::TimedOut)
			}
		}
	}

	async fn send_expect_game_setup(&mut self, protocol: Payload) -> Result<GameSetup, Error> {
		let (chain_id, mut rx) = self.start_chain().await;
		self.write(chain_id, protocol).await?;
		match tokio::time::timeout(NETWORK_TIMEOUT, rx.recv()).await {
			Ok(Some(res)) => {
				debug_assert!(matches!(
					res.chain_id,
					ChainId::EndOfChain(i) if i == chain_id.id_unwrap()
				));
				match res.payload {
					Payload::Game(game) => Ok(game),
					Payload::Err(err) => Err(err),
					_ => Err(Error::ServerError),
				}
			}
			Ok(None) => Err(Error::Disconnected),
			Err(_) => {
				self.remove_chain(chain_id.id_unwrap());
				Err(Error::TimedOut)
			}
		}
	}

	pub async fn send_create_game(&mut self, password: String) -> Result<GameSetup, Error> {
		self.send_expect_game_setup(Payload::CreateGame { password }).await
	}

	pub async fn send_join_game(&mut self, code: u32, password: String) -> Result<GameSetup, Error> {
		self.send_expect_game_setup(Payload::JoinGame { code, password }).await
	}

	pub async fn send_search_games(&mut self, filter: GameFilter) -> Result<Vec<PublicGame>, Error> {
		let (chain_id, mut rx) = self.start_chain().await;
		self.write(chain_id, Payload::SearchGames(filter)).await?;
		match tokio::time::timeout(NETWORK_TIMEOUT, rx.recv()).await {
			Ok(Some(res)) => {
				debug_assert!(matches!(
					res.chain_id,
					ChainId::EndOfChain(i) if i == chain_id.id_unwrap()
				));
				match res.payload {
					Payload::PublicGames(games) => Ok(games),
					Payload::Err(err) => Err(err),
					_ => Err(Error::ServerError),
				}
			}
			Ok(None) => Err(Error::Disconnected),
			Err(_) => {
				self.remove_chain(chain_id.id_unwrap());
				Err(Error::TimedOut)
			}
		}
	}

	/// Host-only
	pub async fn send_game_setup(&mut self, setup_data: GameSetup) -> Result<(), Error> {
		self.send_expect_ack(Payload::Game(setup_data)).await
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

	pub async fn send_renew_token(&mut self) -> Result<(), Error> {
		let (chain_id, mut rx) = self.start_chain().await;
		self.write(chain_id, Payload::RenewAuth {token: self.renew_token.clone()}).await?;
		match tokio::time::timeout(NETWORK_TIMEOUT, rx.recv()).await {
			Ok(Some(res)) => {
				debug_assert!(matches!(
					res.chain_id,
					ChainId::EndOfChain(i) if i == chain_id.id_unwrap()
				));
				match res.payload {
					Payload::RenewAuth {token} => {
						self.auth_token = token;
						Ok(())
					},
					Payload::Err(err) => Err(err),
					_ => Err(Error::ServerError),
				}
			}
			Ok(None) => Err(Error::Disconnected),
			Err(_) => {
				self.remove_chain(chain_id.id_unwrap());
				Err(Error::TimedOut)
			}
		}
	}
}

impl Drop for SetbackClient {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
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
// TODO: #[cfg(not(feature = "client"))]
pub struct ClientConnection {
	writer: TcpWriter,
	reader_handle: JoinHandle<TcpReader>,
	mem_pool: Arc<MemPool>,
	/// cancels the read loop
	cancellation_token: CancellationToken,
	chain_map: ChainMap,
	pub account_id: u32,
	auth_token: Arc<RwLock<String>>,
	pub username: String,
	pub ready: bool,
	pub host: bool,
}

impl SspStream for ClientConnection {
	fn chain_map(&self) -> &ChainMap { &self.chain_map }
	async fn write(&mut self, chain_id: ChainId, protocol: Payload) -> Result<(), Error> {
		let auth_token = self.auth_token.read().await.clone();
		self.writer.send(&Ssp::new_with_token(protocol, chain_id, auth_token), &self.mem_pool).await
	}
}

struct AuthRes {
	reader: TcpReader,
	writer: TcpWriter,
	auth_token: String,
	username: String,
	account_id: u32,
}

impl ClientConnection {
	/// This is used when someone first tries to connect to setbackd.
	/// Their token will be validated and a port number will be sent back if validated.
	/// The client will disconnect from the server, and reconnect on that port number.
	/// If not authenticated, this function will send a message to the client letting them know.
	pub async fn authenticate(
		tcp_stream: TcpStream,
		api: Client,
		server_port: u16,
		client_addr: SocketAddr,
		mem_pool: &MemPool,
	) -> Result<AuthRes, Error> {
		let (mut reader, mut writer) = stream::new_stream(tcp_stream);

		//TODO:
		// Use client_addr for security features like IP bans

		let (protocol, auth_token, username, account_id) = tokio::time::timeout(NETWORK_TIMEOUT, async {
			// get message
			let ssp: Ssp = match reader.recv(mem_pool).await {
				Ok(ssp) => ssp,
				Err(err) => return (Payload::Err(err), String::new(), String::new(), 0)
			};
			// validate version
			if !ssp.version.is_compatible(SSP_VERSION) {
				return (Payload::Err(Error::BadVersion), String::new(), String::new(), 0);
			}
			// get auth token
			let Payload::Authenticate {auth_token} = ssp.payload else {
				return (Payload::Err(Error::BadProtocol), String::new(), String::new(), 0);
			};
			// validate token and get username
			match Self::get_username(api, auth_token.as_str()).await {
				Ok(UsernameResponse {username, account_id}) => (Payload::Connect {port: server_port}, auth_token, username, account_id),
				Err(err) => (Payload::Err(err), String::new(), String::new(), 0),
			}
		}).await.unwrap_or((Payload::Err(Error::TimedOut), String::new(), String::new(), 0));

		// respond to client
		let ssp = Ssp::new(protocol, ChainId::EndOfChain(0));
		writer.send(&ssp, mem_pool).await?;
		match ssp.payload {
			Payload::Err(err) => Err(err),
			Payload::Connect {..} => Ok(AuthRes {reader, writer, auth_token, username, account_id}),
			_ => unsafe {std::hint::unreachable_unchecked()}
		}
	}

	/// The client has already been authenticated and is reconnecting to the server on the port they previously recieved.
	/// This function reauthenticates the client
	pub async fn new(
		tcp_stream: TcpStream,
		api: Client,
		client_addr: SocketAddr,
		mem_pool: Arc<MemPool>,
		state: Arc<State>,
	) -> Result<Self, Error> {
		let AuthRes {
			reader,
			writer,
			auth_token,
			username,
			account_id
		} = Self::authenticate(tcp_stream, api, 0, client_addr, &mem_pool).await?;
		let auth_token = Arc::new(RwLock::new(auth_token));
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();

		let client_connection = Self {
			writer,
			reader_handle: Self::spawn_reader(
				reader,
				auth_token.clone(),
				account_id,
				mem_pool.clone(),
				state,
				read_cancel_token
			),
			mem_pool,
			cancellation_token,
			chain_map: Default::default(),
			account_id,
			auth_token,
			username,
			ready: false,
			host: false,
		};

		Ok(client_connection)
	}

	fn spawn_reader(
		mut reader: TcpReader,
		auth_token: Arc<RwLock<String>>,
		account_id: u32,
		mem_pool: Arc<MemPool>,
		state: Arc<State>,
		cancel_token: CancellationToken,
	) -> JoinHandle<TcpReader> {
		tokio::spawn(async move {
			tokio::select! {
				_ = Self::handle_messages(&mut reader, auth_token.as_ref(), account_id, &mem_pool, state) => {}
				_ = cancel_token.cancelled() => {}
			}
			reader
		})
	}

	/// Cancels the current reader loop task, spawns a new one, and returns the old one.
	fn replace_reader(&mut self, reader: TcpReader, state: Arc<State>) -> JoinHandle<TcpReader> {
		let reader_loop = Self::spawn_reader(
			reader,
			self.auth_token.clone(),
			self.account_id,
			self.mem_pool.clone(),
			state,
			self.cancellation_token.clone(),
		);
		std::mem::replace(&mut self.reader_handle, reader_loop)
	}

	/// Wraps [read] and validates the auth token and version.
	/// This should always be used in the server instead of [read] for security.
	/// [Protocol::Err] is used if invalid.
	async fn read(reader: &mut TcpReader, auth_token: &RwLock<String>, src_id: u32, mem_pool: &MemPool) -> Ssp {
		let mut ssp: Ssp = match reader.recv(mem_pool).await {
			Ok(ssp) => ssp,
			Err(err) => return Ssp::new(Payload::Err(err), ChainId::None)
		};

		// This method of validating the auth token every time a message is received is not very secure,
		// but it's much faster than making a request to the api every time.
		if ssp.auth_token != *auth_token.read().await {
			return Ssp::new(Payload::Err(Error::Unauthenticated), ssp.chain_id);
		}
		if !ssp.version.is_compatible(SSP_VERSION) {
			return Ssp::new(Payload::Err(Error::BadVersion), ssp.chain_id);
		}
		ssp.src_id = src_id;
		ssp
	}

	async fn handle_messages(
		reader: &mut TcpReader,
		auth_token: &RwLock<String>,
		account_id: u32,
		mem_pool: &MemPool,
		state: Arc<State>, // TODO how do we differentiate between being in the server vs in a game?
	) -> ! {
		loop {
			match Self::read(reader, auth_token, account_id, mem_pool).await.payload {
				_ => todo!()
			}
		}
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

	async fn renew_token(api: Client, auth_token: &str, renew_token: String) -> Result<String, Error> {
		Ok(api
			.post(URL_API_RENEW)
			.bearer_auth(auth_token)
			.json(&RenewRequest {renew_token})
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<RenewResponse>()
			.await
			.map_err(|_| Error::ServerError)?
			.auth_token
		)
	}

	pub async fn send_game_event(&mut self, event: GameEvent) -> Result<Option<GameEvent>, Error> {
		let (chain_id, mut rx) = self.start_chain().await;
		self.write(chain_id, Payload::GameEvent(event)).await?;
		#[cfg(debug_assertions)]
		let mut res_chain_id = None;
		let res = match rx.recv().await {
			Some(res) => {
				#[cfg(debug_assertions)] {
					res_chain_id = Some(res.chain_id);
				}
				match res.payload {
					Payload::GameEvent(event) => Ok(Some(event)),
					Payload::Ack => Ok(None),
					Payload::Err(err) => Err(err),
					_ => Err(Error::BadProtocol),
				}
			}
			None => Err(Error::Disconnected),
		};
		#[cfg(debug_assertions)]
		if let Some(res_chain_id) = res_chain_id {
			assert_eq!(res_chain_id, chain_id);
		}
		self.write(
			ChainId::EndOfChain(chain_id.id_unwrap()),
			match res {
				Ok(Some(_)) => Payload::Ack,
				Ok(None) => return Ok(None),
				Err(err) => Payload::Err(err),
			}
		).await?;
		res
	}

	pub async fn send_err(&mut self, err: Error) -> Result<Option<Ssp>, Error> {
		todo!()
	}
}

impl Drop for ClientConnection {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
	}
}

impl std::fmt::Debug for ClientConnection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ClientConnection")
			.field("auth_token", &self.auth_token)
			.field("username", &self.username)
			.field("ready", &self.ready)
			.field("host", &self.host)
			.finish()
	}
}