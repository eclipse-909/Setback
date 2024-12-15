use {
	resources::client_items::*,
	tokio::{
		net::{
			TcpStream,
			tcp::OwnedWriteHalf
		},
		runtime::{Runtime, Builder},
		io::{AsyncWriteExt, AsyncReadExt},
		sync::{
			RwLock,
			oneshot::{Sender, channel}
		}
	},
	std::{
		ptr::null_mut,
		time::Duration,
		ffi::{c_char, c_void, CString},
		sync::{
			Arc,
			atomic::{AtomicU32, Ordering}
		}
	},
	bincode::config::{Configuration, standard},
	awc::Client
};

///TODO change to server IP and port for production
const SERVER_SOCKET: &str = "127.0.0.1:42069";
///TODO change to website URL for production
const WEBSITE_URL: &str = "http://127.0.0.1:8080";

///Opaque pointer for external code. Used to hold state which is used in functions which handle communication with the Server and API.
pub struct GameClient {
	runtime: Runtime,
	writer: Arc<RwLock<OwnedWriteHalf>>,
	terminator: Sender<()>,
	account_id: Arc<AtomicU32>,
	api: Arc<RwLock<Client>>
}

///Blocks until it connects to the server. Creates a new TcpStream, begins listening to the server in a background task,
///and returns the GameClient pointer. The pointer is null if unsuccessful. Use the callback to handle when a message is received or when there's an error.
///The tcp stream stops listening when the callback is invoked with a ClientError, or when the connection is dropped via the drop_connection function.
///The drop_connection must always be used in external code to clean up the resources, even if there's a ClientError. Failure to do so will cause a memory leak.
///# Safety
///[username] and [password] must be cleaned up in external code.
unsafe fn new_connection(
	username: *mut c_char,
	password: *mut c_char,
	c_client: *mut c_void,
	callback: unsafe extern "C" fn(c_client: *mut c_void, message: Result<Ssp, ClientError>)
) -> *mut GameClient {
	if c_client.is_null() {return null_mut();}
	let c_client: usize = c_client as usize;
	let Ok(api) = Client::builder()
		.cookie_store(true)
		.timeout(Duration::from_secs(5))
		.build() else {return null_mut();};
	let api: Arc<RwLock<Client>> = Arc::new(RwLock::new(api));
	let Ok(runtime) = Builder::new_multi_thread().enable_all().build() else {return null_mut();};
	let mut account_id: u32 = 0;
	let id: &mut u32 = &mut account_id;
	let Ok(username) = CString::from_raw(username).into_string() else {return null_mut();};
	let Ok(password) = CString::from_raw(password).into_string() else {return null_mut();};
	let api_clone: Arc<RwLock<Client>> = api.clone();
	let Some((writer, mut reader)) = runtime.block_on(async move {
		//login and get account_id
		let Ok(res) = api_clone.read().await.get(format!("{}/api/login?username={}&password={}", WEBSITE_URL, username, password).as_str()).send().await else {return None;};
		let Ok(account_response) = res.json::<AccountResponse>().await else {return None;};
		*id = account_response.account_id;
		
		//Sync with server
		let Ok(stream) = TcpStream::connect(SERVER_SOCKET).await else {return None;};
		let (reader, writer) = stream.into_split();
		let writer: Arc<RwLock<OwnedWriteHalf>> = Arc::new(RwLock::new(writer));
		if !write(writer.clone(), Ssp::new_with_src(Protocol::SyncID, account_response.account_id)).await {return None;}
		Some((writer, reader))
	}) else {return null_mut();};
	
	//listen for messages from server
	let (terminator, terminator_rx) = channel::<()>();
	runtime.spawn(async move {
		tokio::select! {
	        _ = async move {
				let mut buffer: [u8; 1024] = [0; 1024];
				loop {
			        //read length of message
					let Ok(n) = reader.read_u16_le().await else {
						return callback(c_client as *mut c_void, Err(ClientError::IO));
					};
			        //read message based on length
					match reader.read_exact(&mut buffer[..n as usize]).await {
						Ok(0) => {return callback(c_client as *mut c_void, Err(ClientError::EndOfStream));}
						Err(_) => {return callback(c_client as *mut c_void, Err(ClientError::IO));}
						Ok(_) => {
					        let Ok((ssp, _)) = bincode::decode_from_slice::<Ssp, Configuration>(&buffer[..n as usize], standard()) else {
						        return callback(c_client as *mut c_void, Err(ClientError::Serialization));
					        };
							if ssp.version != Version::LATEST {
								return callback(c_client as *mut c_void, Err(ClientError::WrongVersion));
							}
							callback(c_client as *mut c_void, Ok(ssp));
						}
					}
				}
			} => {}
	        _ = terminator_rx => {}
	    }
	});
	Box::into_raw(Box::new(GameClient {runtime, writer, terminator, account_id: Arc::new(AtomicU32::new(account_id)), api}))
}

///TcpClient must be dropped by calling this function.
///# Safety
///This function immediately returns if the given pointer is null.
unsafe fn drop_connection(client: *mut GameClient) {
	if client.is_null() {return;};
	let _ = Box::from_raw(client).terminator.send(());
}

async fn write(writer: Arc<RwLock<OwnedWriteHalf>>, ssp: Ssp) -> bool {
	let Ok(bytes) = bincode::encode_to_vec(ssp, standard()) else {return false;};
	writer.write().await.write_all(bytes.len().to_le_bytes().as_slice()).await.is_ok() && writer.write().await.write_all(bytes.as_slice()).await.is_ok()
}

///Callback is invoked if the TCP send was unsuccessful.
///# Safety
///This function doesn't check for null pointers. You must check beforehand.
unsafe fn send(client: *mut GameClient, ssp: Ssp, c_client: usize, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	let writer: Arc<RwLock<OwnedWriteHalf>> = (*client).writer.clone();
	(*client).runtime.spawn(async move {
		if !write(writer, ssp).await {callback(c_client as *mut c_void);}
	});
}

///# Safety
///Callback is immediately invoked if client is null.
unsafe fn send_client_error(client: *mut GameClient, err: ClientError, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::ClientErr(err), (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
}

///# Safety
///Callback is immediately invoked if client is null.
unsafe fn send_create_game(client: *mut GameClient, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::CreateGame, (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
}

///# Safety
///Callback is immediately invoked if client is null.
unsafe fn send_join_game(client: *mut GameClient, code: u32, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::JoinGame(code), (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
}

///# Safety
///Callback is immediately invoked if client is null.
unsafe fn send_search_games(client: *mut GameClient, filter: GameFilter, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::SearchGames(filter), (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
}

///Host-only
///# Safety
///Callback is immediately invoked if client is null.
unsafe fn send_game_meta(client: *mut GameClient, meta_data: GameMeta, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::GameMeta(meta_data), (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
}

///Host-only
///# Safety
///Callback is immediately invoked if client is null.
unsafe fn send_player(client: *mut GameClient, player: Player, index: u8, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::Player(player, index), (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
}

///Host-only
///# Safety
///Callback is immediately invoked if client is null.
///Deallocates the FixedList, so don't use the pointer after passing it in.
unsafe fn send_players(client: *mut GameClient, players: Vec<Player>, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::Players(players), (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
	
}

///# Safety
///Callback is immediately invoked if client is null.
unsafe fn send_ready(client: *mut GameClient, is_ready: bool, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::Ready(is_ready), (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
}

///# Safety
///Callback is immediately invoked if client is null.
unsafe fn send_bid(client: *mut GameClient, amount: u8, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::Bid(amount), (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
}

///# Safety
///Callback is immediately invoked if client is null.
unsafe fn send_play_card(client: *mut GameClient, card: Card, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void)) {
	if client.is_null() || c_client.is_null() {return callback(c_client);}
	let c_client: usize = c_client as usize;
	send(client, Ssp::new_with_src(Protocol::PlayCard(card), (*client).account_id.load(Ordering::Relaxed)), c_client, callback)
}

///Logs out of your account by clearing session cookies.
///# Safety
///Immediately returns if client is null.
unsafe fn logout(client: *mut GameClient) {
	if client.is_null() {return;}
	let api: Arc<RwLock<Client>> = (*client).api.clone();
	(*client).runtime.spawn(async move {
		let _ = api.read().await.get(format!("{}/api/account/logout", WEBSITE_URL).as_str()).send().await;
	});
}

///Fetches the account with the given ID.
///# Safety
///Callback is immediately invoked with None if client is null or if unsuccessful.
unsafe fn get_account(client: *mut GameClient, account_id: u32, c_client: *mut c_void, callback: unsafe extern "C" fn(c_client: *mut c_void, result: Option<AccountData>)) {
	if client.is_null() || c_client.is_null() {return callback(c_client, None);}
	let c_client: usize = c_client as usize;
	let api: Arc<RwLock<Client>> = (*client).api.clone();
	(*client).runtime.spawn(async move {
		let Ok(res) = api.read().await.get(format!("{}/api/user/{}", WEBSITE_URL, account_id).as_str()).send().await else {return callback(c_client as *mut c_void, None);};
		let Ok(account) = res.json::<AccountData>().await else {return callback(c_client as *mut c_void, None);};
		callback(c_client as *mut c_void, Some(account));
	});
}