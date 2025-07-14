use {
	bincode::config::standard,
	reqwest::{Client, StatusCode},
	tokio::{
		net::{
			tcp::{OwnedReadHalf, OwnedWriteHalf},
			TcpStream
		},
		sync::mpsc::{unbounded_channel, UnboundedReceiver},
		io::{AsyncReadExt, AsyncWriteExt}
	},
	tokio_util::sync::CancellationToken,
	crate::{
		ssp::*,
		constants::*,
		items::*,
		models::*,
		setup_data::*
	}
};

///This standardizes the method of communication between the server and clients by putting the logic in one place.
pub trait SetbackConnection {
	fn writer(&mut self) -> &mut OwnedWriteHalf;
	///Returns whether the write was successful.
	fn write(&mut self, ssp: Ssp) -> impl Future<Output = bool> {async move {
		let bytes: Vec<u8> = bincode::encode_to_vec(ssp, standard()).unwrap();
		//prefix with 2-byte length
		self.writer().write_u16_le(bytes.len() as u16).await.is_ok() && self.writer().write_all(bytes.as_slice()).await.is_ok()
	}}
	fn read(reader: &mut OwnedReadHalf, buffer: &mut Vec<u8>) -> impl Future<Output = Result<Ssp, Error>> {async move {
		//read the 2-byte length prefix
		let len: u16 = reader.read_u16_le().await.map_err(|_|Error::NetworkIO)?;
		if len > buffer.len() as u16 {
			buffer.resize(len as usize, 0);
		}
		reader.read_exact(&mut buffer[..len as usize]).await.map_err(|_|Error::NetworkIO)?;
		Ok(Ssp::decode(&mut buffer[..len as usize]).unwrap())
	}}
}

///Connection to the server, owned by the client.
pub struct ServerConnection {
	tcp_writer: OwnedWriteHalf,
	cancellation_token: CancellationToken,
	pub ssp_rx: UnboundedReceiver<Result<Ssp, Error>>,
	pub account_id: u32,
	pub ready: bool
}

impl ServerConnection {
	///- Logs in via http request to api.
	///- Connects to Setback dedicated server on accept port.
	///- Reconnects to server on client port.
	///- Listens for messages from server in background task and sends them via mpsc channel.
	///
	///Possible errors are:
	///- [Error::NetworkIO]
	///- [Error::Authentication]
	///- [Error::ServerError]
	pub async fn new_connection(username: String, password: String, api: Client) -> Result<Self, Error> {
		//login and get account_id
		let account_id: u32 = api.login(username, password).await?;
		
		//Connect to server
		let (mut reader, writer) = TcpStream::connect(server(ACCEPT_PORT)).await.map_err(|_|Error::NetworkIO)?.into_split();
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		let (ssp_tx, ssp_rx) = unbounded_channel();
		let mut server_connection: Self = Self {
			tcp_writer: writer,
			cancellation_token,
			ssp_rx,
			account_id,
			ready: false
		};
		
		//authenticate, get client port, connect to client port
		if !server_connection.write(Ssp::new_with_src(Protocol::Connect(0), account_id)).await {
			return Err(Error::NetworkIO);
		}
		let mut buffer: Vec<u8> = Vec::with_capacity(PAGE_SIZE);
		let ssp = Self::read(&mut reader, &mut buffer).await?;
		let Protocol::Connect(client_port) = ssp.protocol else {
			return Err(Error::ServerError);
		};
		let (mut reader, writer) = TcpStream::connect(server(client_port)).await.map_err(|_|Error::NetworkIO)?.into_split();
		server_connection.tcp_writer = writer;
		
		//listen for messages from server
		tokio::spawn(async move {tokio::select! {
			_ = async move {
				loop {
					ssp_tx.send(Self::read(&mut reader, &mut buffer).await).unwrap();
				}
			} => {}
			_ = read_cancel_token.cancelled() => {}
		}});
		
		Ok(server_connection)
	}
	
	///Returns whether the write was successful.
	pub async fn send_create_game(&mut self, public: bool) -> bool {
		self.write(Ssp::new_with_src(Protocol::CreateGame{public}, self.account_id)).await
	}
	
	///Returns whether the write was successful.
	pub async fn send_join_game(&mut self, code: u32) -> bool {
		self.write(Ssp::new_with_src(Protocol::JoinGame{code}, self.account_id)).await
	}
	
	///Returns whether the write was successful.
	pub async fn send_search_games(&mut self, filter: GameFilter) -> bool {
		self.write(Ssp::new_with_src(Protocol::SearchGames(filter), self.account_id)).await
	}
	
	///Host-only
	///Returns whether the write was successful.
	pub async fn send_game_meta(&mut self, setup_data: SetupData) -> bool {
		self.write(Ssp::new_with_src(Protocol::SetupData(setup_data), self.account_id)).await
	}
	
	///Returns whether the write was successful.
	pub async fn send_update_team(&mut self, index: u8, team: u8) -> bool {
		self.write(Ssp::new_with_src(Protocol::UpdatePlayerTeam{index, team}, self.account_id)).await
	}
	
	///Returns whether the write was successful.
	pub async fn send_update_player_type(&mut self, index: u8, human: bool) -> bool {
		self.write(Ssp::new_with_src(Protocol::UpdatePlayerType{index, human}, self.account_id)).await
	}
	
	///Returns whether the write was successful.
	pub async fn send_ready(&mut self, is_ready: bool, index: u8) -> bool {
		self.write(Ssp::new_with_src(Protocol::Ready{is_ready, index}, self.account_id)).await
	}
	
	///Returns whether the write was successful.
	pub async fn send_game_event(&mut self, event: GameEvent) -> bool {
		self.write(Ssp::new_with_src(Protocol::GameEvent(event), self.account_id)).await
	}
	
	///Host-only
	///Returns whether the write was successful.
	pub async fn send_add_player(&mut self) -> bool {
		self.write(Ssp::new_with_src(Protocol::AddPlayer, self.account_id)).await
	}
	
	///Host-only
	///Returns whether the write was successful.
	pub async fn send_remove_player(&mut self, index: u8) -> bool {
		self.write(Ssp::new_with_src(Protocol::RemovePlayer{index}, self.account_id)).await
	}
}

impl SetbackConnection for ServerConnection {
	#[inline(always)]
	fn writer(&mut self) -> &mut OwnedWriteHalf {&mut self.tcp_writer}
}

impl Drop for ServerConnection {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
	}
}

pub trait WebsiteConnection {
	fn new_connection() -> Self;
	fn login(&self, username: String, password: String) -> impl Future<Output=Result<u32, Error>>;
	fn logout(&self) -> impl Future<Output=bool>;
}

impl WebsiteConnection for Client {
	fn new_connection() -> Self {
		Client::builder()
			.cookie_store(true)
			.build()
			.unwrap()
	}
	
	async fn login(&self, username: String, password: String) -> Result<u32, Error> {
		self
			.post(website!(URL_API_LOGIN))
			.json(&LoginCredentials {username, password})
			.send()
			.await
			.map_err(|_|Error::NetworkIO)?
			.error_for_status()
			.map_err(|err| {
				match err.status() {
					Some(StatusCode::UNAUTHORIZED) => Error::Authentication,
					//TODO handle other status codes
					_ => Error::ServerError
				}
			})?
			.cookies()
			.find(|cookie| cookie.name() == "account_id")
			.ok_or(Error::ServerError)?
			.value()
			.parse::<u32>()
			.map_err(|_|Error::ServerError)
	}
	
	///Logs out of your account by clearing session cookies.
	///Returns whether the request was successfully sent. The response code doesn't matter.
	async fn logout(&self) -> bool {
		self.get(website!(URL_API_LOGOUT)).send().await.is_ok()
	}
}

///Connection to the client, owned by the server.
pub struct ClientConnection {
	tcp_writer: OwnedWriteHalf,
	pub ssp_rx: UnboundedReceiver<Result<Ssp, Error>>,
	pub cancellation_token: CancellationToken,
	pub account_id: u32,
	pub ready: bool
}

impl ClientConnection {
	///The TcpListener must accept a connection and then pass the TcpStream to this function.
	///If the connection is not valid, it returns None and the TcpStream is handled and closed in this function.
	///If the connection is valid, the returned [Self] contains the TcpStream's writer and a channel for receiving SSP messages.
	pub async fn new_connection(tcp_stream: TcpStream, client_port: u16) -> Option<Self> {
		let (mut reader, writer) = tcp_stream.into_split();
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		let (ssp_tx, ssp_rx) = unbounded_channel();
		let mut client_connection: Self = Self {tcp_writer: writer, ssp_rx, cancellation_token, account_id: 0, ready: false};
		
		//Wait 5 seconds for SyncID message
		let (mut reader, mut buffer, client_connection) = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
			let mut buffer: Vec<u8> = Vec::with_capacity(PAGE_SIZE);
			let ssp: Ssp = Self::read(&mut reader, &mut buffer).await.ok()?;
			if matches!(ssp.protocol, Protocol::Connect(_)) {
				client_connection.account_id = ssp.src_id;
				if !client_connection.write(Ssp::new(Protocol::Connect(client_port))).await {
					return None;
				}
				Some((reader, buffer, client_connection))
			} else {
				None
			}
		}).await.ok()??;
		
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
		Some(client_connection)
	}
	
	//TODO make a function for each protocol for convenience
}

impl SetbackConnection for ClientConnection {
	#[inline(always)]
	fn writer(&mut self) -> &mut OwnedWriteHalf {&mut self.tcp_writer}
}

impl Drop for ClientConnection {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
	}
}