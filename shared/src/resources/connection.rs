use {
	super::{
		ssp::{Ssp, Protocol},
		util::Error,
		game_items::{GameFilter, GameMeta, PlayerData, GameEvent}
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
	bincode::{Decode, Encode, config::standard},
	serde::{Deserialize, Serialize},
	awc::Client
};

pub const SERVER_PORT: u16 = 42069;

const SERVER_SOCKET: SocketAddrV4 = if cfg!(debug_assertions) {
	SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), SERVER_PORT)
} else {
	SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), SERVER_PORT)//TODO change this to actual socket once server is deployed
};

const WEBSITE_URL: &str = if cfg!(debug_assertions) {
	"http://127.0.0.1:8080/"
} else {
	"http://127.0.0.1:8080/"//TODO change this to actual URL once website is deployed
};

///This standardizes the method of communication between the server and clients by putting the logic in one place.
trait SetbackConnection {
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
	cancellation_token: CancellationToken,
	api: Client,
	account_id: u32,//TODO I might just store this in the client's session cookies rather than in this struct
	pub ready: bool
}

impl ServerConnection {
	///Waits until it connects to the server. Creates a new TcpStream, begins listening to the server in a background task,
	///and returns (ServerConnection, account_id).
	///Use the [callback] to handle when a message is received or when there's an error.
	///The tcp stream stops listening when the callback is invoked with a ClientError, or when [Self::shutdown] is called.
	///Returning None means it couldn't connect to the server.
	pub async fn new_connection(username: &str, password: &str, ssp_callback: fn(message: Result<Ssp, Error>), on_write_error: impl Fn(Error) + Send + 'static) -> Option<Self> {
		//set up awc client
		let api: Client = Client::default();
		
		//login and get account_id
		let account_id: u32 = api
			//TODO fix the URL to match the updated API
			//TODO username and password need to be filtered/sanitized/verified
			.get(format!("{}api/login?username={}&password={}", WEBSITE_URL, username, password).as_str())
			.send()
			.await
			.ok()?
			.cookie("account_id")?
			.value()
			.parse::<u32>()
			.ok()?;
		
		//Read thread
		let (mut reader, mut writer) = TcpStream::connect(SERVER_SOCKET).await.ok()?.into_split();
		let cancellation_token: CancellationToken = CancellationToken::new();
		let read_cancel_token: CancellationToken = cancellation_token.clone();
		tokio::spawn(async move {tokio::select! {
			_ = async move {
				let mut buffer: Vec<u8> = Vec::with_capacity(std::mem::size_of::<Ssp>());
				loop {
					let res: Result<Ssp, Error> = Self::read(&mut reader, &mut buffer).await.map_err(|_|Error::NetworkIO);
					// let is_err: bool = res.is_err();
					ssp_callback(res);
					// if is_err {break;}
					//TODO how do I handle the error here?
				}
				//TODO do I need to handle the post-error here? If so, how?
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
		Some(server_connection)
	}

	pub async fn shutdown(self) {
		self.cancellation_token.cancel();
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

	///Host-only
	pub fn send_player(&mut self, player_data: PlayerData, index: u8) {
		self.send(Ssp::new_with_src(Protocol::Player{data: player_data, index}, self.account_id))
	}

	///Host-only
	pub fn send_players(&mut self, players: Vec<PlayerData>) {
		self.send(Ssp::new_with_src(Protocol::Players(players), self.account_id))
	}

	pub fn send_ready(&mut self, is_ready: bool) {
		self.send(Ssp::new_with_src(Protocol::Ready{is_ready}, self.account_id))
	}

	pub fn send_game_event(&mut self, event: GameEvent) {
		self.send(Ssp::new_with_src(Protocol::GameEvent(event), self.account_id))
	}
	
	///Logs out of your account by clearing session cookies.
	pub async fn logout(&mut self) {
		//TODO fix the URL for updated logout API
		let _ = self.api.get(format!("{}api/account/logout", WEBSITE_URL).as_str()).send().await;
	}
	
	///Fetches the account with the given ID.
	pub async fn get_account(&mut self, account_id: u32) -> Option<AccountData> {
		//TODO fix the URL for updated account API
		self.api.get(format!("{}api/user/{}", WEBSITE_URL, account_id).as_str()).send().await.ok()?.json::<AccountData>().await.ok()
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

impl ClientConnection {
	///The TcpListener must accept a connection and then pass the TcpStream to this function.
	pub async fn new_connection(tcp_stream: TcpStream) -> Self {
		todo!()
		//Read one message for authentication
		//construct the client connection
		//begin the message loop
		//being writing loop
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

#[derive(Encode, Decode, Serialize, Deserialize, Debug)]
pub struct AccountData {
	///alias for rowid in sqlite
	pub account_id: u32,
	pub username: String,
	pub password: String,
	pub last_active: String
}