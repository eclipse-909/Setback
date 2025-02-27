use {
	super::{
		ssp::{Ssp, Protocol},
		util::Error,
		game_items::{GameFilter, GameMeta, GameEvent}
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
		sync::mpsc::{UnboundedSender, unbounded_channel},
		io::{AsyncWriteExt, AsyncReadExt}
	},
	tokio_util::sync::CancellationToken,
	bincode::config::standard,
	serde::{Deserialize, Serialize},
	reqwest::{Client, StatusCode}
};

pub const SERVER_PORT: u16 = 42069;

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
	fn writer(&mut self) -> &mut UnboundedSender<Ssp>;
	#[inline(always)]
	fn send(&mut self, ssp: Ssp) {self.writer().send(ssp).unwrap();}
	fn write(write_half: &mut OwnedWriteHalf, ssp: Ssp) -> impl Future<Output = Result<(), Error>> {async move {
		let mut bytes: Vec<u8> = bincode::encode_to_vec(ssp, standard()).unwrap();
		//prefix with 2-byte length
		let mut message: Vec<u8> = (bytes.len() as u16).to_le_bytes().to_vec();
		message.append(&mut bytes);
		write_half.write_all(message.as_slice()).await.map_err(|_|Error::NetworkIO)
	}}
	fn read(read_half: &mut OwnedReadHalf, buffer: &mut Vec<u8>) -> impl Future<Output = Result<Ssp, Error>> {async move {
		//read the 2-byte length prefix
		let len: u16 = read_half.read_u16_le().await.map_err(|_|Error::NetworkIO)?;
		if len > buffer.len() as u16 {
			buffer.resize(len as usize, 0);
		}
		read_half.read_exact(&mut buffer[..len as usize]).await.map_err(|_|Error::NetworkIO)?;
		Ok(Ssp::decode(&mut buffer[..len as usize]).unwrap())
	}}
}

///Connection to the server, owned by the client.
pub struct ServerConnection {
	message_tx: UnboundedSender<Ssp>,
	pub cancellation_token: CancellationToken,
	api: Client,
	pub account_id: u32,//TODO I might just store this in the client's session cookies rather than in this struct
	pub ready: bool
}

impl ServerConnection {
	///Waits until it connects to the server. Creates a new TcpStream, begins listening to the server in a background task,
	///and returns (ServerConnection, account_id).
	///Use the [callback] to handle when a message is received or when there's an error.
	///The tcp stream stops listening when the callback is invoked with a ClientError, or when [Self::shutdown] is called.
	///Returning None means it couldn't connect to the server.
	pub async fn new_connection(username: String, password: String, ssp_callback: impl Fn(Option<Ssp>) + Send + Sync + 'static, on_write_error: impl Fn(Error) + Send + 'static) -> Result<Self, Error> {
		//set up reqwest client
		let api: Client = Client::builder()
			.cookie_store(true)
			.build()
			.unwrap();
		
		//login and get account_id
		let account_id: u32 = api
			//TODO fix the URL to match the updated API
			//TODO username and password need to be filtered/sanitized/verified
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
		
		//Read thread
		let (mut reader, mut writer) = TcpStream::connect(SERVER_SOCKET).await.map_err(|_|Error::NetworkIO)?.into_split();
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		tokio::spawn(async move {tokio::select! {
			_ = async move {
				let mut buffer: Vec<u8> = Vec::with_capacity(std::mem::size_of::<Ssp>());
				loop {
					ssp_callback(Self::read(&mut reader, &mut buffer).await.ok());
				}
			} => {}
			_ = read_cancel_token.cancelled() => {}
		}});

		//Write thread
		let (message_tx, mut message_rx) = unbounded_channel();
		let write_cancel_token: CancellationToken = cancellation_token.clone();
		tokio::spawn(async move {tokio::select! {
			_ = async move {
				while let Some(ssp) = message_rx.recv().await {
					if let Err(error) = Self::write(&mut writer, ssp).await {
						on_write_error(error);
					}
				}
				//TODO do I need to handle the post-error here? If so, how?
			} => {}
			_ = write_cancel_token.cancelled() => {}
		}});

		let mut server_connection: Self = Self {message_tx, cancellation_token, api, account_id, ready: false};
		server_connection.send(Ssp::new_with_src(Protocol::SyncID, account_id));
		Ok(server_connection)
	}

	pub fn send_error(&mut self, err: Error) {
		self.send(Ssp::new_with_src(Protocol::Err(err), self.account_id))
	}
	
	pub fn send_create_game(&mut self, public: bool) {
		self.send(Ssp::new_with_src(Protocol::CreateGame{public}, self.account_id))
	}
	
	pub fn send_join_game(&mut self, code: u32) {
		self.send(Ssp::new_with_src(Protocol::JoinGame{code}, self.account_id))
	}

	pub fn send_search_games(&mut self, filter: GameFilter) {
		self.send(Ssp::new_with_src(Protocol::SearchGames(filter), self.account_id))
	}

	///Host-only
	pub fn send_game_meta(&mut self, meta_data: GameMeta) {
		self.send(Ssp::new_with_src(Protocol::GameMeta(meta_data), self.account_id))
	}

	pub fn send_update_team(&mut self, index: u8, team: u8) {
		self.send(Ssp::new_with_src(Protocol::UpdatePlayerTeam{index, team}, self.account_id))
	}

	pub fn send_update_player_type(&mut self, index: u8, human: bool) {
		self.send(Ssp::new_with_src(Protocol::UpdatePlayerType{index, human}, self.account_id))
	}

	pub fn send_ready(&mut self, is_ready: bool, index: u8) {
		self.send(Ssp::new_with_src(Protocol::Ready{is_ready, index}, self.account_id))
	}

	pub fn send_game_event(&mut self, event: GameEvent) {
		self.send(Ssp::new_with_src(Protocol::GameEvent(event), self.account_id))
	}

	///Host-only
	pub fn send_add_player(&mut self) {
		self.send(Ssp::new_with_src(Protocol::AddPlayer, self.account_id))
	}

	///Host-only
	pub fn send_remove_player(&mut self, index: u8) {
		self.send(Ssp::new_with_src(Protocol::RemovePlayer{index}, self.account_id))
	}
	
	///Logs out of your account by clearing session cookies.
	pub fn logout(&mut self, runtime: tokio::runtime::Handle) {
		runtime.spawn(self.api.get(website!("api/account/logout")).send());
	}
}

impl SetbackConnection for ServerConnection {
	#[inline(always)]
	fn writer(&mut self) -> &mut UnboundedSender<Ssp> {&mut self.message_tx}
}

impl Drop for ServerConnection {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
	}
}

///Connection to the client, owned by the server.
pub struct ClientConnection {
	message_tx: UnboundedSender<Ssp>,
	cancellation_token: CancellationToken,
	pub account_id: u32,
	pub ready: bool
}

impl std::fmt::Debug for ClientConnection {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "ClientConnection")
	}
}

impl ClientConnection {
	///The TcpListener must accept a connection and then pass the TcpStream to this function.
	pub async fn new_connection(tcp_stream: TcpStream, ssp_callback: impl Fn(Option<Ssp>) + Send + Sync + 'static, on_write_error: impl Fn(Error) + Send + 'static) -> Option<Self> {
		let (mut reader, mut writer) = tcp_stream.into_split();
		//Wait for SyncID message
		let (account_id, mut reader, mut writer) = tokio::time::timeout(std::time::Duration::from_secs(10), async move {
			let mut buffer: Vec<u8> = Vec::with_capacity(std::mem::size_of::<Ssp>());
			let ssp: Ssp = Self::read(&mut reader, &mut buffer).await.ok()?;
			if matches!(ssp.protocol, Protocol::SyncID) {
				Self::write(&mut writer, Ssp::new(Protocol::SyncID)).await.ok()?;
				Some((ssp.src_id, reader, writer))
			} else {
				None
			}
		}).await.ok()??;

		//Read thread
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		tokio::spawn(async move {tokio::select! {
			_ = async move {
				let mut buffer: Vec<u8> = Vec::with_capacity(std::mem::size_of::<Ssp>());
				loop {
					ssp_callback(Self::read(&mut reader, &mut buffer).await.ok());
				}
			} => {}
			_ = read_cancel_token.cancelled() => {}
		}});

		//Write thread
		let (message_tx, mut message_rx) = unbounded_channel();
		let write_cancel_token: CancellationToken = cancellation_token.clone();
		tokio::spawn(async move {tokio::select! {
			_ = async move {
				while let Some(ssp) = message_rx.recv().await {
					if let Err(error) = Self::write(&mut writer, ssp).await {
						on_write_error(error);
					}
				}
				//TODO do I need to handle the post-error here? If so, how?
			} => {}
			_ = write_cancel_token.cancelled() => {}
		}});

		//construct the client connection
		Some(Self {message_tx, cancellation_token, account_id, ready: false})
	}

	//TODO make a function for each protocol for convenience
}

impl SetbackConnection for ClientConnection {
	#[inline(always)]
	fn writer(&mut self) -> &mut UnboundedSender<Ssp> {&mut self.message_tx}
}

impl Drop for ClientConnection {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountData {
	///alias for rowid in sqlite
	pub account_id: u32,
	pub username: String,
	pub password: String,
	pub last_active: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginCredentials {
	pub username: String,
	pub password: String,
}