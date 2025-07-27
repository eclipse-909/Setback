use {
	crate::{
		constants::*,
		items::*,
		models::*,
		setup_data::*,
		ssp::*
	},
	reqwest::Client,
	tokio::{
		io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
		net::{
			tcp::OwnedWriteHalf,
			TcpStream
		},
		sync::mpsc::{unbounded_channel, UnboundedReceiver}
	},
	tokio_util::sync::CancellationToken
};

pub trait Connection {
	fn write(writer: &mut BufWriter<impl AsyncWriteExt + Unpin>, ssp: &Ssp) -> impl Future<Output=Result<(), Error>> {async move {
		let bytes: Vec<u8> = bincode::encode_to_vec(ssp, bincode::config::standard()).unwrap();
		//prefix with 4-byte length
		writer
			.write_u32_le(bytes.len() as u32)
			.await
			.map_err(|_| Error::IoError)?;
		writer
			.write_all(bytes.as_slice())
			.await
			.map_err(|_| Error::IoError)?;
		writer
			.flush()
			.await
			.map_err(|_| Error::IoError)
	}}
	
	fn read(reader: &mut BufReader<impl AsyncReadExt + Unpin>, buffer: &mut Vec<u8>) -> impl Future<Output=Result<Ssp, Error>> {async move {
		//read the 4-byte length prefix
		let len: u32 = reader.read_u32_le().await.map_err(|_| Error::BadRequest)?;
		if len > buffer.len() as u32 {
			buffer.resize(len as usize, 0);
		}
		reader.read_exact(&mut buffer[..len as usize]).await.map_err(|_| Error::BadRequest)?;
		Ok(Ssp::decode(&mut buffer[..len as usize]).unwrap())
	}}
}

pub trait SetbackConnection: Connection + Sized {
	///- Logs in via http request to api.
	///- Connects to Setback dedicated server on accept port.
	///- Reconnects to server on client port.
	///- Listens for messages from server in background task and sends them via mpsc channel.
	fn new(username: String, password: String, api: &impl WebsiteConnection) -> impl Future<Output=Result<(Self, UnboundedReceiver<Result<Ssp, Error>>), Error>>;
	fn send_create_game(&mut self, password: Option<String>) -> impl Future<Output=Result<(), Error>>;
	fn send_join_game(&mut self, code: String, password: Option<String>) -> impl Future<Output=Result<(), Error>>;
	fn send_search_games(&mut self, filter: GameFilter) -> impl Future<Output=Result<(), Error>>;
	///Host-only
	fn send_game_meta(&mut self, setup_data: SetupData) -> impl Future<Output=Result<(), Error>>;
	fn send_update_team(&mut self, index: u8, team: u8) -> impl Future<Output=Result<(), Error>>;
	fn send_update_player_type(&mut self, index: u8, human: bool) -> impl Future<Output=Result<(), Error>>;
	fn send_ready(&mut self, is_ready: bool, index: u8) -> impl Future<Output=Result<(), Error>>;
	fn send_game_event(&mut self, event: GameEvent) -> impl Future<Output=Result<(), Error>>;
	///Host-only
	fn send_add_player(&mut self) -> impl Future<Output=Result<(), Error>>;
	///Host-only
	fn send_remove_player(&mut self, index: u8) -> impl Future<Output=Result<(), Error>>;
	fn refresh_token(&mut self, api: &impl WebsiteConnection) -> impl Future<Output=Result<(), Error>>;
}

pub trait WebsiteConnection {
	fn new_api() -> Self;
	fn as_client(&self) -> &Client;
	fn login(&self, credentials: &LoginRequest) -> impl Future<Output=Result<LoginResponse, Error>>;
	fn logout(&self, auth_token: &str) -> impl Future<Output=Result<(), Error>>;
	fn id(&self, auth_token: &str) -> impl Future<Output=Result<u32, Error>>;
	fn update(&self) -> impl Future<Output=Result<Version, Error>>;
}

///Connection to the server, owned by the client.
pub struct ServerConnection {
	tcp_writer: BufWriter<OwnedWriteHalf>,
	cancellation_token: CancellationToken,
	pub auth_token: String,
	pub refresh_token: String,
	pub account_id: u32,
	pub ready: bool
}

impl SetbackConnection for ServerConnection {
	async fn new(username: String, password: String, api: &impl WebsiteConnection) -> Result<(Self, UnboundedReceiver<Result<Ssp, Error>>), Error> {
		//login and get account_id
		let session = api.login(&LoginRequest{username, password}).await?;
		
		//Connect to server
		let (reader, writer) = TcpStream::connect(server(ACCEPT_PORT)).await.map_err(|_|Error::NotFound)?.into_split();
		let mut reader = BufReader::new(reader);
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		let (ssp_tx, ssp_rx) = unbounded_channel();
		let mut server_connection: Self = Self {
			tcp_writer: BufWriter::new(writer),
			cancellation_token,
			auth_token: session.auth_token.clone(),
			refresh_token: session.refresh_token,
			account_id: session.account_id,
			ready: false
		};
		
		//authenticate, get client port, connect to client port
		let auth_ssp = Ssp::new_with_src(Protocol::Authenticate(session.auth_token), session.account_id);
		Self::write(&mut server_connection.tcp_writer, &auth_ssp).await?;
		let mut buffer: Vec<u8> = Vec::with_capacity(PAGE_SIZE);
		let Protocol::Connect(client_port) = Self::read(&mut reader, &mut buffer).await?.protocol else {
			return Err(Error::BadRequest);
		};
		let (reader, writer) = TcpStream::connect(server(client_port)).await.map_err(|_|Error::NotFound)?.into_split();
		let mut reader = BufReader::new(reader);
		server_connection.tcp_writer = BufWriter::new(writer);
		//reauthenticate
		Self::write(&mut server_connection.tcp_writer, &auth_ssp).await?;
		let Protocol::Connect(_) = Self::read(&mut reader, &mut buffer).await?.protocol else {
			return Err(Error::BadRequest);
		};
		
		//listen for messages from server
		tokio::spawn(async move {tokio::select! {
			_ = async move {
				loop {
					ssp_tx.send(Self::read(&mut reader, &mut buffer).await).unwrap();
				}
			} => {}
			_ = read_cancel_token.cancelled() => {}
		}});
		
		Ok((server_connection, ssp_rx))
	}
	
	async fn send_create_game(&mut self, password: Option<String>) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::CreateGame{password}, self.account_id)).await
	}
	
	async fn send_join_game(&mut self, code: String, password: Option<String>) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::JoinGame{code, password}, self.account_id)).await
	}
	
	async fn send_search_games(&mut self, filter: GameFilter) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::SearchGames(filter), self.account_id)).await
	}
	
	async fn send_game_meta(&mut self, setup_data: SetupData) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::SetupData(setup_data), self.account_id)).await
	}
	
	async fn send_update_team(&mut self, index: u8, team: u8) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::UpdatePlayerTeam{index, team}, self.account_id)).await
	}
	
	async fn send_update_player_type(&mut self, index: u8, human: bool) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::UpdatePlayerType{index, human}, self.account_id)).await
	}
	
	async fn send_ready(&mut self, is_ready: bool, index: u8) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::Ready{is_ready, index}, self.account_id)).await
	}
	
	async fn send_game_event(&mut self, event: GameEvent) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::GameEvent(event), self.account_id)).await
	}
	
	async fn send_add_player(&mut self) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::AddPlayer, self.account_id)).await
	}
	
	async fn send_remove_player(&mut self, index: u8) -> Result<(), Error> {
		Self::write(&mut self.tcp_writer, &Ssp::new_with_src(Protocol::RemovePlayer{index}, self.account_id)).await
	}
	
	async fn refresh_token(&mut self, api: &impl WebsiteConnection) -> Result<(), Error> {
		//Client specifically doesn't have a refresh method because I want the request to go through the server.
		//This way the ServerConnection.auth_token and refresh_token get updated.
		//If the client tries to call refresh via reqwest and not through the server, the tokens will not be updated in ServerConnection.
		self.auth_token = api
			.as_client()
			.post(website!(URL_API_REFRESH))
			.bearer_auth(self.auth_token.clone())
			.json(&RefreshRequest {refresh_token: self.refresh_token.clone()})
			.send()
			.await
			.map_err(|_| Error::IoError)?
			.error_for_status()
			.map_err(|err| Error::from(err.status().unwrap()))?
			.json::<RefreshResponse>()
			.await
			.map_err(|_| Error::BadRequest)?
			.auth_token;
		Ok(())
	}
}

impl Connection for ServerConnection {}

impl Drop for ServerConnection {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
	}
}

///Connection to the client, owned by the server.
pub struct ClientConnection {
	tcp_writer: BufWriter<OwnedWriteHalf>,
	pub ssp_rx: UnboundedReceiver<Result<Ssp, Error>>,
	pub cancellation_token: CancellationToken,
	pub account_id: u32,
	pub ready: bool
}

impl ClientConnection {
	pub async fn accept(tcp_stream: TcpStream, client_port: u16, api: &Client) -> Result<(), Error> {
		let (reader, writer) = tcp_stream.into_split();
		let mut reader = BufReader::new(reader);
		let mut writer = BufWriter::new(writer);
		
		let response = tokio::time::timeout(TIMEOUT, async move {
			let mut buffer: Vec<u8> = Vec::with_capacity(PAGE_SIZE);
			let ssp: Ssp = Self::read(&mut reader, &mut buffer).await.map_err(|_| Error::ServerError)?;
			if let Protocol::Authenticate(auth_token) = ssp.protocol {
				let account_id: u32 = api.id(auth_token.as_str()).await.map_err(|_| Error::ServerError)?;
				if account_id != ssp.src_id {
					return Err(Error::Forbidden);
				}
				Ok(Ssp::new(Protocol::Connect(client_port)))
			} else {
				Err(Error::BadRequest)
			}
		}).await.map_err(|_| Error::Indeterminate)?;
		match response {
			Ok(ssp) => Self::write(&mut writer, &ssp).await,
			Err(err) => Self::write(&mut writer, &Ssp::new(Protocol::Err(err))).await
		}
	}
	
	pub async fn new(tcp_stream: TcpStream, api: &Client) -> Result<Self, Error> {
		let (reader, writer) = tcp_stream.into_split();
		let mut reader = BufReader::new(reader);
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		let (ssp_tx, ssp_rx) = unbounded_channel();
		let mut client_connection: Self = Self {tcp_writer: BufWriter::new(writer), ssp_rx, cancellation_token, account_id: 0, ready: false};
		
		let response = tokio::time::timeout(TIMEOUT, async move {
			let mut buffer: Vec<u8> = Vec::with_capacity(PAGE_SIZE);
			let ssp: Ssp = Self::read(&mut reader, &mut buffer).await.map_err(|_| Error::ServerError)?;
			if let Protocol::Authenticate(auth_token) = ssp.protocol {
				let account_id: u32 = api.id(auth_token.as_str()).await.map_err(|_| Error::ServerError)?;
				if account_id != ssp.src_id {
					return Err(Error::Forbidden);
				}
				Ok((reader, buffer, Ssp::new(Protocol::Connect(0))))
			} else {
				Err(Error::BadRequest)
			}
		}).await.map_err(|_| Error::Indeterminate)?;
		match response {
			Ok((mut reader, mut buffer, ref ssp)) => {
				Self::write(&mut client_connection.tcp_writer, ssp).await?;
				//Spawn task to read messages from TcpStream and send them to the channel
				tokio::spawn(async move {tokio::select! {
					_ = async move {
						loop {
							ssp_tx.send(Self::read(&mut reader, &mut buffer).await).unwrap();
						}
					} => {}
					_ = read_cancel_token.cancelled() => {}
				}});
				//construct the client connection
				Ok(client_connection)
			}
			Err(err) => {
				Self::write(&mut client_connection.tcp_writer, &Ssp::new(Protocol::Err(err))).await?;
				Err(err)
			}
		}
	}
	
	//TODO make a function for each protocol for convenience
}

impl Connection for ClientConnection {}

impl Drop for ClientConnection {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
	}
}

impl WebsiteConnection for Client {
	fn new_api() -> Self {
		Client::builder()
			.cookie_store(true)
			.build()
			.unwrap()
	}
	
	fn as_client(&self) -> &Self { self }
	
	async fn login(&self, credentials: &LoginRequest) -> Result<LoginResponse, Error> {
		self
			.post(website!(URL_API_LOGIN))
			.json(credentials)
			.send()
			.await
			.map_err(|_| Error::IoError)?
			.error_for_status()
			.map_err(|err| Error::from(err.status().unwrap()))?
			.json::<LoginResponse>()
			.await
			.map_err(|_| Error::BadRequest)
	}
	
	async fn logout(&self, auth_token: &str) -> Result<(), Error> {
		self
			.get(website!(URL_API_LOGOUT))
			.bearer_auth(auth_token)
			.send()
			.await
			.map_err(|_| Error::IoError)?
			.error_for_status()
			.map_err(|err| Error::from(err.status().unwrap()))?;
		Ok(())
	}
	
	async fn id(&self, auth_token: &str) -> Result<u32, Error> {
		Ok(self
			.get(website!(URL_API_ID))
			.bearer_auth(auth_token)
			.send()
			.await
			.map_err(|_| Error::IoError)?
			.error_for_status()
			.map_err(|err| Error::from(err.status().unwrap()))?
			.json::<IdResponse>()
			.await
			.map_err(|_| Error::BadRequest)?
			.account_id
		)
	}
	
	async fn update(&self) -> Result<Version, Error> {
		self
			.get(website!(URL_API_UPDATE))
			.send()
			.await
			.map_err(|_| Error::IoError)?
			.error_for_status()
			.map_err(|err| Error::from(err.status().unwrap()))?
			.json::<Version>()
			.await
			.map_err(|_| Error::BadRequest)
	}
}