use {
	super::{
		ssp::{Ssp, Protocol},
		util::Error,
		game_items::{GameFilter, GameEvent},
		setup_data::SetupData
	},
	std::{
		net::{Ipv4Addr, SocketAddrV4},
		future::Future
	},
	tokio::{
		net::{
			TcpStream,
			tcp::{OwnedWriteHalf, OwnedReadHalf}
		},
		sync::mpsc::{UnboundedReceiver, unbounded_channel},
		io::{AsyncWriteExt, AsyncReadExt}
	},
	tokio_util::sync::CancellationToken,
	bincode::config::standard,
	serde::{Deserialize, Serialize},
	reqwest::{Client, StatusCode}
};

pub const SERVER_PORT: u16 = 42069;
pub const PAGE_SIZE: usize = 4096;

const SERVER_SOCKET: SocketAddrV4 = if cfg!(debug_assertions) {
	SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), SERVER_PORT)
} else {
	SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), SERVER_PORT)//TODO change this to actual socket once server is deployed
};

const WEBSITE_URL: &str = if cfg!(debug_assertions) {
	"https://127.0.0.1/"
} else {
	"https://127.0.0.1/"//TODO change this to actual URL once website is deployed
};

macro_rules! website {
	($path:expr) => {format!("{}{}", WEBSITE_URL, $path)};
}

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
	api: Client,
	cancellation_token: CancellationToken,
	pub ssp_rx: UnboundedReceiver<Result<Ssp, Error>>,
	pub account_id: u32,
	pub ready: bool
}

impl ServerConnection {
	///Waits until it connects to the server. Creates a new TcpStream, begins listening to the server in a background task,
	///and returns (ServerConnection, account_id).
	///Use the [callback] to handle when a message is received or when there's an error.
	///The tcp stream stops listening when the callback is invoked with a ClientError, or when [Self::shutdown] is called.
	///Returning None means it couldn't connect to the server.
	pub async fn new_connection(username: String, password: String) -> Result<Self, Error> {
		//set up reqwest client
		let api: Client = Client::builder()
			.cookie_store(true)
			.build()
			.unwrap();
		
		//login and get account_id
		let account_id: u32 = api
			//TODO fix the URL to match the updated API
			.post(website!("api/login"))
			.json(&LoginCredentials {username, password})
			.send()
			.await
			.map_err(|_|Error::NetworkIO)?
			.error_for_status()
			.map_err(|err| {
				match err.status() {
					Some(StatusCode::UNAUTHORIZED) => {Error::Authentication}
					_ => {Error::ServerError}
				}
			})?
			.cookies()
			.find(|cookie| cookie.name() == "account_id")
			.ok_or(Error::ServerError)?
			.value()
			.parse::<u32>()
			.map_err(|_|Error::ServerError)?;

		//Spawn task to read messages from server
		let (mut reader, writer) = TcpStream::connect(SERVER_SOCKET).await.map_err(|_|Error::NetworkIO)?.into_split();
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		let (ssp_tx, ssp_rx) = unbounded_channel();
		tokio::spawn(async move {tokio::select! {
			_ = async move {
				let mut buffer: Vec<u8> = Vec::with_capacity(PAGE_SIZE);
				loop {
					ssp_tx.send(Self::read(&mut reader, &mut buffer).await).unwrap();
				}
			} => {}
			_ = read_cancel_token.cancelled() => {}
		}});

		let mut server_connection: Self = Self {
			tcp_writer: writer,
			api,
			cancellation_token,
			ssp_rx,
			account_id,
			ready: false
		};
		if !server_connection.write(Ssp::new_with_src(Protocol::SyncID, server_connection.account_id)).await {
			return Err(Error::NetworkIO);
		}
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
	
	///Logs out of your account by clearing session cookies.
	///Returns whether the write was successful.
	pub async fn logout(&mut self) -> bool {
		self.api.get(website!("api/account/logout")).send().await.is_ok()
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

///Connection to the client, owned by the server.
pub struct ClientConnection {
	tcp_writer: OwnedWriteHalf,
	pub ssp_rx: UnboundedReceiver<Result<Ssp, Error>>,
	cancellation_token: CancellationToken,
	pub account_id: u32,
	pub ready: bool
}

impl ClientConnection {
	///The TcpListener must accept a connection and then pass the TcpStream to this function.
	///If the connection is not valid, it returns None and the TcpStream is handled and closed in this function.
	///If the connection is valid, the returned [Self] contains the TcpStream's writer and a channel for receiving SSP messages.
	pub async fn new_connection(tcp_stream: TcpStream) -> Option<Self> {
		let (mut reader, writer) = tcp_stream.into_split();
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		let (ssp_tx, ssp_rx) = unbounded_channel();
		let mut client_connection: Self = Self {tcp_writer: writer, ssp_rx, cancellation_token, account_id: 0, ready: false};

		//Wait 5 seconds for SyncID message
		let (mut reader, mut buffer, client_connection) = tokio::time::timeout(std::time::Duration::from_secs(5), async move {
			let mut buffer: Vec<u8> = Vec::with_capacity(PAGE_SIZE);
			let ssp: Ssp = Self::read(&mut reader, &mut buffer).await.ok()?;
			if matches!(ssp.protocol, Protocol::SyncID) {
				client_connection.account_id = ssp.src_id;
				if !client_connection.write(Ssp::new(Protocol::SyncID)).await {
					return None;
				}
				Some((reader, buffer, client_connection))
			} else {
				None
			}
		}).await.ok()??;

		//Spawn task to read messages from TcpStream and send them to the channel
		tokio::spawn(async move {tokio::select! {
			_ = async move {loop {ssp_tx.send(Self::read(&mut reader, &mut buffer).await).unwrap();}} => {}
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

#[derive(Serialize, Deserialize)]
pub struct AccountData {
	///alias for rowid in sqlite
	pub account_id: u32,
	pub username: String,
	pub password: String,
	pub last_active: String
}

#[derive(Serialize, Deserialize)]
pub struct LoginCredentials {
	pub username: String,
	pub password: String,
}