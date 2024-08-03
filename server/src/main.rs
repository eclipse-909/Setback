use {
	crate::game::{MPGame, Games, MAX_GAMES},
	resources::server_items::*,
	std::{
		collections::HashMap,
		sync::Arc
	},
	tokio::{
		io::AsyncWriteExt,
		net::TcpListener,
		sync::{RwLock, RwLockWriteGuard, RwLockReadGuard, watch, mpsc},
		time::{self, Duration, Interval}
	},
	bincode::config::{Configuration, standard}
};

mod game;

type Clients = Arc<RwLock<HashMap<u32, Arc<RwLock<Client>>>>>;

///TODO change to "0.0.0.0" for production and use a different port
const SOCKET: &str = "127.0.0.1:42069";
const MIN_CLIENTS: usize = 20;

///main represents the server
#[tokio::main]
async fn main() {
	//create the server in main
	let listener: TcpListener = TcpListener::bind(SOCKET).await.unwrap_or_else(|_| panic!("Server could not be bound to {}", SOCKET));
	//these clients are owned by the server. other hashmaps will be owned by individual games, which are owned by the server
	let clients: Clients = Arc::new(RwLock::new(HashMap::new()));
	//the lobbies/games that are active
	let games: Games = Arc::new(RwLock::new(HashMap::new()));
	//sending will terminate all select! expressions with a receiver
	let (server_terminator_tx, server_terminator_rx) = watch::channel::<()>(());
	//communication between each client and the server
	let (client_tx, mut client_rx) = mpsc::unbounded_channel::<Ssp>();

	//when clients receive messages in their TCP streams, they will send messages to the server.
	//those messages go here, so they can be handled or relayed to other clients.
	let clients_clone: Clients = clients.clone();
	let server_term_tx: watch::Sender<()> = server_terminator_tx.clone();
	let server_term_rx: watch::Receiver<()> = server_terminator_rx.clone();
	spawn_async(server_terminator_rx.clone(), async move {
		loop {
			let Some(ssp) = client_rx.recv().await else {continue;};
			match &ssp.protocol {
				Protocol::Disconnect => {remove_player(&clients_clone, ssp.src_id).await;}
				Protocol::CreateGame => {
					if games.read().await.len() >= MAX_GAMES as usize {
						send_error_back(&clients_clone, ssp.src_id, ServerError::GamesAtCapacity).await;
						continue;
					}
					let Some(client) = clients_clone.write().await.remove(&ssp.src_id) else {continue;};
					//create the game and add the client to it
					let game: Arc<RwLock<MPGame>> = MPGame::create(games.clone(), server_term_tx.clone(), server_term_rx.clone()).await;
					let mut game_guard: RwLockWriteGuard<MPGame> = game.write().await;
					game_guard.add_client(client.clone(), true).await;
					//fill the rest with AIs
					while game_guard.can_add_player() {
						let team_id: u8 = (game_guard.sp_game.players.len() % 2) as u8;
						game_guard.sp_game.players.push(Player::new_ai(team_id));
					}
					//Send game info back to client, letting them know they've joined the lobby
					if client.write().await.message_client(&Ssp::new(Protocol::Game(game_guard.sp_game.meta_data.clone(), game_guard.sp_game.players.clone()))).await.is_err() {
						remove_player(&clients_clone, ssp.src_id).await;
					}
				}
				Protocol::JoinGame(code) => {
					let games_guard: RwLockReadGuard<HashMap<u32, Arc<RwLock<MPGame>>>> = games.read().await;
					let Some(game) = games_guard.get(code) else {
						send_error_back(&clients_clone, ssp.src_id, ServerError::GameNotFound).await;
						continue;
					};
					let mut game_guard: RwLockWriteGuard<MPGame> = game.write().await;
					if !game_guard.can_add_player() {
						send_error_back(&clients_clone, ssp.src_id, ServerError::GameAtCapacity).await;
						continue;
					}
					let Some(client) = clients_clone.write().await.remove(&ssp.src_id) else {continue;};
					game_guard.add_client(client.clone(), false).await;
					if client.write().await.message_client(&Ssp::new(Protocol::Game(game_guard.sp_game.meta_data.clone(), game_guard.sp_game.players.clone()))).await.is_err() {
						remove_player(&clients_clone, ssp.src_id).await;
					}
				}
				Protocol::SearchGames(filter) => {
					send_message(&clients_clone, &Ssp::new(Protocol::GameMetas(MPGame::get_game_metas(&games, filter).await)), ssp.src_id).await;
				}
				_ => {send_error_back(&clients_clone, ssp.src_id, ServerError::InvalidProtocol).await;}
			}
		}
	});

	//continuously try to accept clients
	spawn_sync(server_terminator_rx.clone(), async move {
		loop {
			//accept a new client
			let Ok((mut tcp_stream, _)) = listener.accept().await else {continue;};
			if clients.read().await.len() as u32 == MAX_SERVER_CLIENTS {
				let _ = tcp_stream.shutdown().await;
				continue;
			}
			
			//Wait for the client to send the account_id
			let tx: mpsc::UnboundedSender<Ssp> = client_tx.clone();
			let client: Arc<RwLock<Client>> = Arc::new(RwLock::new(Client::new(tcp_stream, tx)));
			//The client has 3 seconds to send a SyncID protocol before it is automatically disconnected.
			let mut interval: Interval = time::interval(Duration::from_secs(3));
			interval.tick().await;
			let mut buf: Vec<u8> = vec![0; 1024];
			let mut id: u32 = 0;
			let mut client_guard: RwLockWriteGuard<Client> = client.write().await;
			if !tokio::select! {
				_ = interval.tick() => {false}
				result = client_guard.read(&mut buf) => {
					match result {
						Ok(0) | Err(_) => {false}
						Ok(n) => {
							let Ok((Ssp{
								version,
								src_id,
								protocol: Protocol::SyncID
							},_)) = bincode::decode_from_slice::<Ssp, Configuration>(&buf[..n as usize], standard()) else {return false;};
							if version != Version::LATEST {return false;}
							id = src_id;
							true
						}
					}
				}
			} /*if the SyncID message cannot be properly received*/ {
				let _ = client_guard.shutdown().await;
				continue;
			}
			client_guard.account_id = id;
			if client_guard.message_client(&Ssp::new(Protocol::SyncID)).await.is_err() {continue;}
			drop(client_guard);

			//manage client in concurrent task
			let client_clone: Arc<RwLock<Client>> = client.clone();
			let client_clone1: Arc<RwLock<Client>> = client.clone();
			clients.write().await.insert(id, client);
			let server_term_rx: watch::Receiver<()> = server_terminator_rx.clone();
			let clients_clone: Clients = clients.clone();
			tokio::spawn(async move {
				//listen for messages and send them to the server/game
				spawn_sync(server_term_rx, async move {
					loop {
						let mut client_guard: RwLockWriteGuard<Client> = client_clone.write().await;
						match client_guard.read(&mut buf).await {
							Ok(0) | Err(_) => {break;}
							Ok(n) => {
								match bincode::decode_from_slice::<Ssp, Configuration>(&buf[..n as usize], standard()) {
									Ok((ssp, _)) => {
										if ssp.version != Version::LATEST {
											let _ = client_guard.message_client(&Ssp::new(Protocol::ClientErr(ClientError::WrongVersion))).await;
											break;
										}
										if client_guard.owner_tx.send(ssp).is_err() {break}
									}
									Err(_) => {
										if client_guard.message_client(&Ssp::new(Protocol::ServerErr(ServerError::Deserialization))).await.is_err() {break;}
									}//1
								}//2
							}//3
						}//4
					}//5
					//occasionally shrink HashMap capacity when client leaves to clean up unused memory
					let mut clients_guard: RwLockWriteGuard<HashMap<u32, Arc<RwLock<Client>>>> = clients_clone.write().await;
					if (clients_guard.len() as f32) < f32::max(clients_guard.capacity() as f32 * 0.1, MIN_CLIENTS as f32) {
						let num_clients: usize = clients_guard.len();
						clients_guard.shrink_to(usize::max(num_clients * 5, MIN_CLIENTS));
					}
				}).await;

				//remove the client from the HashMap before the task ends, and shutdown the TcpStream
				let mut client_guard1: RwLockWriteGuard<Client> = client_clone1.write().await;
				let _ = client_guard1.shutdown().await;
				let _ = client_guard1.owner_tx.send(Ssp::new_with_src(Protocol::Disconnect, client_guard1.account_id));
			});
		}
	}).await;
	//the server is shut down if this line is reached
}

async fn send_message(clients: &Clients, ssp: &Ssp, dest_id: u32) {
	let clients_guard: RwLockWriteGuard<HashMap<u32, Arc<RwLock<Client>>>> = clients.write().await;
	let Some(client) = clients_guard.get(&dest_id) else {return;};
	if client.write().await.message_client(ssp).await.is_err() {
		remove_player(clients, dest_id).await;
	}
}

async fn send_error_back(clients: &Clients, src_id: u32, err: ServerError) {
	let client_guard: RwLockWriteGuard<HashMap<u32, Arc<RwLock<Client>>>> = clients.write().await;
	let Some(client) = client_guard.get(&src_id) else {return;};
	if client.write().await.message_client(&Ssp::new(Protocol::ServerErr(err))).await.is_err() {
		remove_player(clients, src_id).await;
	}
}

///Removes a player from the hashmap of players, shuts down the TcpSteam, and drops it
async fn remove_player(clients: &Clients, id: u32) {
	if let Some(client) = clients.write().await.remove(&id) {
		let _ = client.write().await.shutdown().await;
	}
}