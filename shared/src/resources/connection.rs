use {
	super::{
		ssp::{Ssp, Protocol},
		util::ClientError,
		game_items::{GameFilter, GameMeta, PlayerData, Card}
	},
	std::{
		net::{Ipv4Addr, SocketAddrV4},
		future::Future,
		io::Error
	},
	tokio::{
		net::{
			TcpStream,
			tcp::{OwnedWriteHalf, OwnedReadHalf}
		},
		io::{AsyncWriteExt, AsyncReadExt},
		task::JoinHandle
	},
	tokio_util::sync::CancellationToken,
	bincode::{Decode, Encode, config::standard},
	serde::{Deserialize, Serialize},
	awc::Client
};

const SERVER_SOCKET: SocketAddrV4 = if cfg!(debug_assertions) {
	SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 42069)
} else {
	SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 42069)//TODO change this to actual socket once server is deployed
};

const WEBSITE_URL: &str = if cfg!(debug_assertions) {
	"http://127.0.0.1:8080/"
} else {
	"http://127.0.0.1:8080/"//TODO change this to actual URL once website is deployed
};

///This standardizes the method of communication between the server and clients by putting the logic in one place.
pub trait SetbackConnection {
	const BUF_LEN: usize = size_of::<Ssp>();//TODO this might not be enough for protocols involving Vec<T>
	fn write_half(&mut self) -> &mut OwnedWriteHalf;
	fn send(&mut self, ssp: &Ssp) -> impl Future<Output = Result<(), Error>> {async move {
		let mut bytes: Vec<u8> = bincode::encode_to_vec(ssp, standard()).map_err(Error::other)?;
		//prefix with 2-byte length
		let mut message: Vec<u8> = (bytes.len() as u16).to_le_bytes().to_vec();
		message.append(&mut bytes);
		self.write_half().write_all(message.as_slice()).await
	}}
	fn read(read_half: &mut OwnedReadHalf, buffer: &mut [u8]) -> impl Future<Output = Result<Ssp, Error>> {async move {
		let len: u16 = read_half.read_u16_le().await?;
		debug_assert!(len <= Self::BUF_LEN as u16, "Buffer not big enough. Length of this message: {} bytes.", len);
		read_half.read_exact(&mut buffer[..len as usize]).await?;
		Ssp::decode(&mut buffer[..len as usize]).map_err(Error::other)
	}}
}

///Connection to the server, owned by the client.
pub struct ServerConnection {
	writer: OwnedWriteHalf,
	message_loop: Option<JoinHandle<()>>,
	cancellation_token: CancellationToken,
	api: Client,
	account_id: u32,
	pub ready: bool
}

impl ServerConnection {
	///Waits until it connects to the server. Creates a new TcpStream, begins listening to the server in a background task,
	///and returns (ServerConnection, account_id).
	///Use the [callback] to handle when a message is received or when there's an error.
	///The tcp stream stops listening when the callback is invoked with a ClientError, or when [Self::shutdown] is called.
	///Returning None means it couldn't connect to the server.
	pub async fn new_connection(username: &str, password: &str, ssp_callback: fn(message: Result<Ssp, ClientError>)) -> Option<Self> {
		//set up awc client
		let api: Client = Client::default();
		
		//login and get account_id
		let Ok(mut res) = api
			//TODO fix the URL to match the updated API
			//TODO username and password need to be filtered/sanitized/verified
			.get(format!("{}api/login?username={}&password={}", WEBSITE_URL, username, password).as_str())
			.send()
			.await else {return None;};
		let Ok(account_response) = res.json::<AccountResponse>().await else {return None;};
		let account_id: u32 = account_response.account_id;
		
		//Sync with server
		let (mut reader, writer) = TcpStream::connect(SERVER_SOCKET).await.ok()?.into_split();
		let cancellation_token: CancellationToken = CancellationToken::new();
		let cloned_token: CancellationToken = cancellation_token.clone();
		let message_loop: JoinHandle<()> = tokio::spawn(async move {tokio::select! {
			_ = async move {
				let mut buffer: [u8; <ServerConnection as SetbackConnection>::BUF_LEN] = [0; <ServerConnection as SetbackConnection>::BUF_LEN];
				loop {
					let res: Result<Ssp, ClientError> = Self::read(&mut reader, &mut buffer).await.map_err(|_|ClientError::IO);
					let is_err: bool = res.is_err();
					ssp_callback(res);
					if is_err {break;}
				}
				//TODO do I need to handle the post-error here? If so, how?
			} => {}
			_ = cloned_token.cancelled() => {}
		}});
		let mut server_connection: Self = Self {writer, message_loop: Some(message_loop), cancellation_token, api, account_id, ready: false};
		server_connection.send(&Ssp::new_with_src(Protocol::SyncID, account_id)).await.ok()?;
		Some(server_connection)
	}

	pub async fn shutdown(mut self) {
		self.cancellation_token.cancel();
		let _ = self.message_loop.take().unwrap().await;
	}

	pub async fn send_client_error(&mut self, err: ClientError) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::ClientErr(err), self.account_id)).await
	}
	
	pub async fn send_create_game(&mut self) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::CreateGame, self.account_id)).await
	}
	
	pub async fn send_join_game(&mut self, code: u32) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::JoinGame(code), self.account_id)).await
	}

	pub async fn send_search_games(&mut self, filter: GameFilter) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::SearchGames(filter), self.account_id)).await
	}

	///Host-only
	pub async fn send_game_meta(&mut self, meta_data: GameMeta) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::GameMeta(meta_data), self.account_id)).await
	}

	///Host-only
	pub async fn send_player(&mut self, player_data: PlayerData, index: u8) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::Player(player_data, index), self.account_id)).await
	}

	///Host-only
	pub async fn send_players(&mut self, players: Vec<PlayerData>) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::Players(players), self.account_id)).await
	}

	pub async fn send_ready(&mut self, is_ready: bool) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::Ready(is_ready), self.account_id)).await
	}

	pub async fn send_bid(&mut self, amount: u8) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::Bid(amount), self.account_id)).await
	}
	
	pub async fn send_play_card(&mut self, card: Card) -> Result<(), Error> {
		self.send(&Ssp::new_with_src(Protocol::PlayCard(card), self.account_id)).await
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
	fn write_half(&mut self) -> &mut OwnedWriteHalf {&mut self.writer}
}

impl Drop for ServerConnection {
	fn drop(&mut self) {
		self.cancellation_token.cancel();
	}
}

///Connection to the client, owned by the server.
pub struct ClientConnection {
	writer: OwnedWriteHalf,
	message_loop: Option<JoinHandle<()>>,
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
	}

	//TODO make a function for each protocol for convenience
}

impl SetbackConnection for ClientConnection {
	#[inline(always)]
	fn write_half(&mut self) -> &mut OwnedWriteHalf {&mut self.writer}
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

#[derive(Serialize, Deserialize)]
pub struct AccountResponse {pub account_id: u32}