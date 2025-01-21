use {
	super::resources::{
		game_items::{Card, GameFilter, GameMeta, Player},
		ssp::{Protocol, Ssp, Version},
		connection::{AccountData, AccountResponse, SERVER_SOCKET},
		util::ClientError,
	},
	awc::Client,
	tokio::{
		io::{AsyncReadExt, AsyncWriteExt},
		net::{tcp::OwnedWriteHalf, TcpStream},
		task::JoinHandle
	},
	tokio_util::sync::CancellationToken
};

///TODO change to website URL for production
const WEBSITE_URL: &str = "http://127.0.0.1:8080/";

///Used to hold state which is used in functions which handle communication with the Server and API.
pub struct GameClient {
	writer: OwnedWriteHalf,
	message_loop: JoinHandle<()>,
	terminator: CancellationToken,
	account_id: u32,
	api: Client//Does this use sessions or JWTs? I need authentication
}

impl GameClient {
	///Waits until it connects to the server. Creates a new TcpStream, begins listening to the server in a background task,
	///and returns the GameClient.
	///Use the [callback] to handle when a message is received or when there's an error.
	///The tcp stream stops listening when the callback is invoked with a ClientError, or when [Self::shutdown] is called.
	///The drop_connection must always be used in external code to clean up the resources, even if there's a ClientError. Failure to do so will cause a memory leak.
	pub async fn new_connection(username: &str, password: &str, callback: fn(message: Result<Ssp, ClientError>)) -> Option<Self> {
		//set up awc client
		let api: Client = Client::default();
		let mut account_id: u32 = 0;
		
		//login and get account_id
		let Ok(mut res) = api
			//TODO fix the URL to match the updated API
			//TODO username and password need to be sanitized and filtered to prevent URL injection attack
			.get(format!("{}api/login?username={}&password={}", WEBSITE_URL, username, password).as_str())
			.send()
			.await else {return None;};
		let Ok(account_response) = res.json::<AccountResponse>().await else {return None;};
		account_id = account_response.account_id;
		
		//Sync with server
		let Ok(stream) = TcpStream::connect(SERVER_SOCKET).await else {return None;};
		let (mut reader, mut writer) = stream.into_split();
		let Ok(bytes) = Ssp::new_with_src(Protocol::SyncID, account_response.account_id).encode() else {return None;};
		if writer.write_all(bytes.as_slice()).await.is_err() {return None;}
		
		//listen for messages from server
		let token: CancellationToken = CancellationToken::new();
		let cloned_token: CancellationToken = token.clone();
		let message_loop: JoinHandle<()> = tokio::spawn(async move {
			tokio::select! {
				_ = async move {
					let mut buffer: [u8; 1024] = [0; 1024];
					loop {
						//read length of message
						let Ok(n) = reader.read_u16_le().await else {
							return callback(Err(ClientError::IO));
						};
						//read message based on length
						match reader.read_exact(&mut buffer[..n as usize]).await {
							Ok(0) => {return callback(Err(ClientError::EndOfStream));}
							Err(_) => {return callback(Err(ClientError::IO));}
							Ok(_) => {
								let Ok(ssp) = Ssp::decode(&mut buffer) else {
									return callback(Err(ClientError::Serialization));
								};
								if ssp.version != Version::LATEST {
									return callback(Err(ClientError::WrongVersion));
								}
								callback(Ok(ssp));
							}
						}
					}
				} => {}
				_ = cloned_token.cancelled() => {}
			}
		});
		Some(GameClient {writer, message_loop, terminator: token, account_id, api})
	}
	
	pub async fn shutdown(self) {
		self.terminator.cancel();
		let _ = self.message_loop.await;
	}
	
	///send SSP to server
	async fn send(&mut self, ssp: Ssp) -> bool {
		let Ok(bytes) = ssp.encode() else {return false;};
		self.writer.write_all(bytes.as_slice()).await.is_ok()
	}
	
	pub async fn send_client_error(&mut self, err: ClientError) {
		self.send(Ssp::new_with_src(Protocol::ClientErr(err), self.account_id)).await;
	}
	
	pub async fn send_create_game(&mut self) {
		self.send(Ssp::new_with_src(Protocol::CreateGame, self.account_id)).await;
	}
	
	pub async fn send_join_game(&mut self, code: u32) {
		self.send(Ssp::new_with_src(Protocol::JoinGame(code), self.account_id)).await;
	}
	
	pub async fn send_search_games(&mut self, filter: GameFilter) {
		self.send(Ssp::new_with_src(Protocol::SearchGames(filter), self.account_id)).await;
	}
	
	///Host-only
	pub async fn send_game_meta(&mut self, meta_data: GameMeta) {
		self.send(Ssp::new_with_src(Protocol::GameMeta(meta_data), self.account_id)).await;
	}
	
	///Host-only
	pub async fn send_player(&mut self, player: Player, index: u8) {
		self.send(Ssp::new_with_src(Protocol::Player(player, index), self.account_id)).await;
	}
	
	///Host-only
	pub async fn send_players(&mut self, players: Vec<Player>) {
		self.send(Ssp::new_with_src(Protocol::Players(players), self.account_id)).await;
	}
	
	pub async fn send_ready(&mut self, is_ready: bool) {
		self.send(Ssp::new_with_src(Protocol::Ready(is_ready), self.account_id)).await;
	}
	
	pub async fn send_bid(&mut self, amount: u8) {
		self.send(Ssp::new_with_src(Protocol::Bid(amount), self.account_id)).await;
	}
	
	pub async fn send_play_card(&mut self, card: Card) {
		self.send(Ssp::new_with_src(Protocol::PlayCard(card), self.account_id)).await;
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