use {
	crate::{*, codec::*},
	tracing::{error, info},
	reqwest::Client,
	std::{
		collections::{HashMap, hash_map::Entry},
		sync::{Arc, RwLock}
	},
	tokio::{
		net::{TcpStream, tcp::{OwnedReadHalf, OwnedWriteHalf}},
		runtime::Handle,
		sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel}
	},
	tokio_util::{
		sync::CancellationToken,
		codec::{FramedRead, FramedWrite}
	},
	futures::{SinkExt, StreamExt}
};

pub fn setback_api() -> Option<Client> {
	Client::builder().cookie_store(true).build().ok()
}

type ChainMap = Arc<RwLock<HashMap<u32, UnboundedSender<Ssp>>>>;

trait SspStream {
	fn chain_map(&self) -> &ChainMap;
	fn auth_token(&self) -> String;
	fn writer(&mut self) -> &mut FramedWrite<OwnedWriteHalf, SspEncoder>;

	/// Starts a message chain where the chain id is passed into all messages
	/// and the receiver is used to await responses from the server.
	fn start_chain(&self) -> (ChainId, UnboundedReceiver<Ssp>) {
		//get new key
		let mut next_key: u32 = 0;
		let mut chain_map = self.chain_map().write().unwrap();
		//TODO: find a better way to make a unique key.
		// This loop will probably never run more than a few times, but still.
		loop {
			if let Entry::Vacant(e) = chain_map.entry(next_key) {
				//create new channel
				let (tx, rx) = unbounded_channel();
				e.insert(tx);
				return (ChainId::ExpectResponse(next_key), rx);
			}
			next_key += 1;
		}
	}

	fn remove_chain(&self, id: u32) -> Option<UnboundedSender<Ssp>> {
		self.chain_map().write().unwrap().remove(&id)
	}

	async fn write(&mut self, chain_id: ChainId, protocol: Payload) -> Result<(), Error> {
		let auth_token = self.auth_token();
		self.writer().send(&Ssp::new_with_token(protocol, chain_id, auth_token)).await
	}
}

/// Connection to the server, owned by the client.
///
/// Tokio runtime is required for all async operations.
/// However, this is already handled for you.
/// You do not need to spawn tasks on or enter the runtime.
/// The runtime will be entered and exited for each async operation defined here.
pub struct SetbackClient {
	writer: FramedWrite<OwnedWriteHalf, SspEncoder>,
	rt: Handle,
	/// cancels the read loop
	cancellation_token: CancellationToken,
	chain_map: ChainMap,
	api: Client,
	auth_token: String,
	refresh_token: String,
	ready: bool,
}

impl SspStream for SetbackClient {
	fn chain_map(&self) -> &ChainMap { &self.chain_map }
	fn auth_token(&self) -> String { self.auth_token.clone() }
	fn writer(&mut self) -> &mut FramedWrite<OwnedWriteHalf, SspEncoder> { &mut self.writer }
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
	pub async fn new(credentials: LoginRequest) -> Result<(Self, UnboundedReceiver<Result<Ssp, Error>>), Error> {
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

		//Save username and password
		std::thread::spawn(move || {
			if let Err(err) = std::fs::write(USR_CRD, credentials.username.as_str()) {
				error!("Error creating usr.crd file: {}", err);
				return;
			}
			if let Err(err) = keyring::Entry::new(
				KEYRING_SERVICE,
				credentials.username.as_str()
			).and_then(|entry| entry.set_password(credentials.password.as_str())) {
				error!("Error setting password in keyring: {}", err);
				return;
			}
			info!("Saved credentials successfully");
		});

		//Connect to server
		let (reader, writer) = TcpStream::connect(server(SETBACKD_PORT))
			.await
			.map_err(|_| Error::ServerDown)?
			.into_split();
		let cancellation_token = CancellationToken::new();
		let read_cancel_token = cancellation_token.clone();
		let chain_map: ChainMap = Default::default();
		let chain_map_clone = chain_map.clone();
		let mut client = Self {
			writer: FramedWrite::with_capacity(writer, SspEncoder, codec::MAX_LEN),
			rt: tokio::runtime::Handle::current(),
			cancellation_token,
			chain_map,
			api,
			auth_token: session.auth_token.clone(),
			refresh_token: session.refresh_token,
			ready: false,
		};

		// Authenticate to get client port
		// No need to use chains yet because this handshake is required before arbitrary communication
		let auth_ssp = Ssp::new(
			Payload::Authenticate { auth_token: session.auth_token },
			ChainId::ExpectResponse(0),
		);
		client.writer.send(&auth_ssp).await?;
		let mut reader = FramedRead::with_capacity(reader, SspDecoder::new(), codec::MAX_LEN);
		let auth_res = reader.next().await.ok_or(Error::Disconnected)??;
		if !matches!(auth_res.chain_id, ChainId::EndOfChain(0)) {
			return Err(Error::ServerError);
		};
		let Payload::Connect {port} = auth_res.payload else {
			return Err(Error::ServerError);
		};

		// Then disconnect and reconnect to client port
		let (reader, writer) = TcpStream::connect(server(port))
			.await
			.map_err(|_| Error::ServerDown)?
			.into_split();

		// Reauthenticate - still don't need chaining
		let mut writer = FramedWrite::with_capacity(writer, SspEncoder, codec::MAX_LEN);
		writer.send(&auth_ssp).await?;
		if !matches!(auth_res.chain_id, ChainId::EndOfChain(0)) {
			return Err(Error::ServerError);
		};
		let mut reader = FramedRead::with_capacity(reader, SspDecoder::new(), codec::MAX_LEN);
		if !matches!(
			reader.next().await.ok_or(Error::Disconnected)??.payload,
			Payload::Connect {..}
		) {
			return Err(Error::ServerError);
		}

		// Listen for messages from server
		let (ssp_tx, ssp_rx) = unbounded_channel();
		tokio::spawn(async move {
			tokio::select! {
				_ = async move {
					loop {
						match reader.next().await.ok_or(Error::Disconnected).flatten() {
							// Route to transmitter via chaining...
							Ok(message @ Ssp {
								chain_id: ChainId::ExpectResponse(chain_id)
								| ChainId::EndOfChain(chain_id),
								..
							}) => {
								match message.chain_id {
									ChainId::ExpectResponse(_) => {
										if let Some(chain_tx) = chain_map_clone
											.read()
											.unwrap()
											.get(&chain_id)
											{chain_tx.send(message).unwrap();}
										// if the transmitter isn't in the hash map,
										// the request probably timed out,
										// and it was removed from the hash map
									}
									ChainId::EndOfChain(_) => {
										if let Some(chain_tx) = chain_map_clone
											.write()
											.unwrap()
											.remove(&chain_id)
											{chain_tx.send(message).unwrap();}
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
				} => {}
				_ = read_cancel_token.cancelled() => {}
			}
		});

		Ok((client, ssp_rx))
	}

	pub async fn refresh_token(&mut self) -> Result<(), Error> {
		//Client specifically doesn't have a refresh method because I want the request to go through the server.
		//This way the ServerConnection.auth_token and refresh_token get updated.
		//If the client tries to call refresh via reqwest and not through the server, the tokens will not be updated in ServerConnection.
		self.auth_token = self
			.api
			.post(URL_API_REFRESH)
			.bearer_auth(self.auth_token.clone())
			.json(&RefreshRequest {
				refresh_token: self.refresh_token.clone(),
			})
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<RefreshResponse>()
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

	#[cfg(test)]
	pub async fn server_version(&self) -> Result<Version, Error> {
		Ok(SETBACK_VERSION)
	}

	async fn send_expect_ack(&mut self, protocol: Payload) -> Result<(), Error> {
		let (chain_id, mut rx) = self.start_chain();
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
		let (chain_id, mut rx) = self.start_chain();
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

	pub async fn send_join_game(&mut self, code: String, password: String) -> Result<GameSetup, Error> {
		self.send_expect_game_setup(Payload::JoinGame { code, password }).await
	}

	pub async fn send_search_games(&mut self, filter: GameFilter) -> Result<Vec<PublicGame>, Error> {
		let (chain_id, mut rx) = self.start_chain();
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

	pub async fn send_refresh_token(&mut self) -> Result<(), Error> {
		let (chain_id, mut rx) = self.start_chain();
		self.write(chain_id, Payload::RefreshAuth {token: self.refresh_token.clone()}).await?;
		match tokio::time::timeout(NETWORK_TIMEOUT, rx.recv()).await {
			Ok(Some(res)) => {
				debug_assert!(matches!(
					res.chain_id,
					ChainId::EndOfChain(i) if i == chain_id.id_unwrap()
				));
				match res.payload {
					Payload::RefreshAuth {token} => {
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
	writer: FramedWrite<OwnedWriteHalf, SspEncoder>,
	ssp_rx: UnboundedReceiver<Ssp>,
	/// cancels the read loop
	cancellation_token: CancellationToken,
	chain_map: ChainMap,
	auth_token: Arc<RwLock<String>>,
	pub username: String,
	pub ready: bool,
	pub host: bool,
}

impl SspStream for ClientConnection {
	fn chain_map(&self) -> &ChainMap { &self.chain_map }
	fn auth_token(&self) -> String { self.auth_token.read().unwrap().clone() }
	fn writer(&mut self) -> &mut FramedWrite<OwnedWriteHalf, SspEncoder> { &mut self.writer }
}

struct AuthRes {
	reader: FramedRead<OwnedReadHalf, SspDecoder>,
	writer: FramedWrite<OwnedWriteHalf, SspEncoder>,
	auth_token: String,
	username: String,
}

impl ClientConnection {
	/// This is used when someone first tries to connect to the server.
	/// Their token will be validated and a port number will be sent back if validated.
	/// The client will disconnect from the server, and reconnect on that port number.
	pub async fn authenticate(
		tcp_stream: TcpStream,
		api: &Client,
		server_port: u16,
	) -> Result<AuthRes, Error> {
		let (reader, writer) = tcp_stream.into_split();
		let mut reader = FramedRead::with_capacity(reader, SspDecoder::new(), codec::MAX_LEN);

		let (protocol, auth_token, username) = tokio::time::timeout(NETWORK_TIMEOUT, async {
			// get message
			let ssp = match reader.next().await.ok_or(Error::Disconnected).flatten() {
				Ok(ssp) => ssp,
				Err(err) => return (Payload::Err(err), String::new(), String::new())
			};
			// validate version
			if !ssp.version.is_compatible() {
				return (Payload::Err(Error::BadVersion), String::new(), String::new());
			}
			// get auth token
			let Payload::Authenticate {auth_token} = ssp.payload else {
				return (Payload::Err(Error::BadProtocol), String::new(), String::new());
			};
			// validate token and get username
			match Self::get_username(api, auth_token.as_str()).await {
				Ok(username) => (Payload::Connect {port: server_port}, auth_token, username),
				Err(err) => (Payload::Err(err), String::new(), String::new()),
			}
		}).await.unwrap_or((Payload::Err(Error::TimedOut), String::new(), String::new()));

		// respond to client
		let ssp = Ssp::new(protocol, ChainId::EndOfChain(0));
		let mut writer = FramedWrite::with_capacity(writer, SspEncoder, codec::MAX_LEN);
		writer.send(&ssp).await?;
		match ssp.payload {
			Payload::Err(err) => Err(err),
			Payload::Connect {..} => Ok(AuthRes {reader, writer, auth_token, username}),
			_ => unsafe {std::hint::unreachable_unchecked()}
		}
	}

	/// The client has already been authenticated and is reconnecting to the server on the port they previously recieved.
	/// This function reauthenticates the client
	pub async fn new(tcp_stream: TcpStream, api: &Client) -> Result<Self, Error> {
		let AuthRes {
			mut reader,
			writer,
			auth_token,
			username,
		} = Self::authenticate(tcp_stream, api, 0).await?;
		let auth_token = Arc::new(RwLock::new(auth_token));
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		let (ssp_tx, ssp_rx) = unbounded_channel();
		let client_connection: Self = Self {
			writer,
			ssp_rx,
			cancellation_token,
			chain_map: Default::default(),
			auth_token: auth_token.clone(),
			username,
			ready: false,
			host: false, // this will be set to true after they create a game
		};
		//Spawn task to read messages from TcpStream and send them to the channel
		tokio::spawn(async move {
			tokio::select! {
				_ = async move {
					loop {
						ssp_tx.send(Self::read(&mut reader, &auth_token).await).unwrap();
					}
				} => {}
				_ = read_cancel_token.cancelled() => {}
			}
		});
		Ok(client_connection)
	}

	/// Wraps [read] and validates the auth token and version.
	/// This should always be used in the server instead of [read] for security.
	/// [Protocol::Err] is used if invalid.
	async fn read(reader: &mut FramedRead<OwnedReadHalf, SspDecoder>, auth_token: &Arc<RwLock<String>>) -> Ssp {
		let ssp = match reader.next().await.ok_or(Error::Disconnected).flatten() {
			Ok(ssp) => ssp,
			Err(err) => return Ssp::new(Payload::Err(err), ChainId::None)
		};

		// This method of validating the auth token every time a message is received is not very secure,
		// but it's much faster than making a request to the api every time.
		if ssp.auth_token != *auth_token.read().unwrap() {
			return Ssp::new(Payload::Err(Error::Unauthenticated), ssp.chain_id);
		}
		if !ssp.version.is_compatible() {
			return Ssp::new(Payload::Err(Error::BadVersion), ssp.chain_id);
		}
		ssp
	}

	async fn get_username(api: &Client, auth_token: &str) -> Result<String, Error> {
		Ok(api
			.get(URL_API_USERNAME)
			.bearer_auth(auth_token)
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<UsernameResponse>()
			.await
			.map_err(|_| Error::ServerError)?
			.username
		)
	}

	async fn refresh_token(api: &Client, auth_token: &str, refresh_token: String) -> Result<String, Error> {
		Ok(api
			.post(URL_API_REFRESH)
			.bearer_auth(auth_token)
			.json(&RefreshRequest {refresh_token})
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<RefreshResponse>()
			.await
			.map_err(|_| Error::ServerError)?
			.auth_token
		)
	}

	pub async fn send_game_event(&mut self, event: GameEvent) -> Result<Option<GameEvent>, Error> {
		let (chain_id, mut rx) = self.start_chain();
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

	pub async fn send_err(&mut self, err: Error) -> Result<Ssp, Error> {
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