use {
	crate::*,
	scc::{HashMap, HashSet},
	sqlx::PgPool,
	std::sync::Arc,
	tokio::sync::mpsc::{Receiver, Sender, UnboundedReceiver},
	tracing::error,
};

#[derive(Hash, Eq, PartialEq)]
struct GameKey {
	code: String,
	password: String,
}

pub type ClientSender = Sender<Box<ClientConnection>>;
pub type ClientReceiver = Receiver<Box<ClientConnection>>;
type ClientStore = HashSet<u32>;
type GameStore = HashMap<GameKey, ClientSender>;

/// State for the setback server.
///
/// The client store holds clients that aren't in a game.
/// Clients in a game will be owned by that game in either
/// the lobby store or game runtime store.
/// Literally everything is owned by this state object,
/// so it's passed around with an Arc.
pub struct State {
	pub db: PgPool,
	pub owner_store: ClientStore,
	pub_lobby_store: GameStore,
	private_lobby_store: GameStore,
	runtime_store: GameStore,
}

impl State {
	/// You should only have one state object.
	/// It's just too much effort to use singleton pattern.
	pub fn new(db: PgPool) -> Arc<Self> {
		Arc::new(Self {
			db,
			owner_store: HashSet::new(),
			pub_lobby_store: HashMap::new(),
			private_lobby_store: HashMap::new(),
			runtime_store: HashMap::new(),
		})
	}
	
	/// Returns game code
	pub async fn create_lobby(
		self: Arc<Self>,
		host: ClientConnection,
		password: String,
	) -> Result<String, Error> {
		let num_games = self.pub_lobby_store.len() + self.private_lobby_store.len() + self.runtime_store.len();
		if num_games >= MAX_GAMES {
			return Err(Error::GamesAtCapacity);
		}
		let (handle, code) = Lobby::create_multiplayer(Box::new(host), password.clone(), self.clone());
		self.insert_lobby(handle, code, password).await
	}
	
	/// Returns game code
	async fn insert_lobby(
		&self,
		client_tx: ClientSender,
		code: String,
		password: String,
	) -> Result<String, Error> {
		let key = GameKey {code: code.clone(), password: password.clone()};
		self.lobby_store(&password)
			.insert_async(key, client_tx)
			.await
			.map(|_| code)
			.map_err(|_| {
				error!("Game code already exists");
				Error::GamesAtCapacity
			})
	}
	
	pub async fn start_game(
		self: Arc<Self>,
		lobby: Lobby,
		err_q: UnboundedReceiver<(u8, NetError)>,
		client_rx: ClientReceiver,
	) -> Result<(), (Error, Lobby)> {
		let key = GameKey {
			code: lobby.lobby_state.rules.code.clone(),
			password: lobby.lobby_state.rules.password.clone(),
		};
		let (_, client_tx) = self
			.lobby_store(&lobby.lobby_state.rules.password)
			.remove_async(&key)
			.await
			.unwrap();
		// TODO we should occasionally shrink the hashmap capacity to free up memory
		if self.runtime_store.insert_async(key, client_tx).await.is_err() {
			return Err((Error::GamesAtCapacity, lobby));
		}
		lobby.finish().start_game(self, err_q, client_rx);
		Ok(())
	}
	
	pub async fn rm_lobby(&self, code: String, password: String) {
		_ = self.lobby_store(&password).remove_async(&GameKey {code, password}).await;
		// TODO we should occasionally shrink the hashmap capacity to free up memory
	}
	
	pub async fn rm_game(&self, code: String, password: String) {
		_ = self.runtime_store.remove_async(&GameKey {code, password}).await;
		// TODO we should occasionally shrink the hashmap capacity to free up memory
	}
	
	pub async fn join_lobby(&self, client: ClientConnection, code: String, password: String) -> JoinResult {
		let Some(mut entry) = self.lobby_store(&password).get_async(&GameKey {code, password}).await else {
			return JoinResult::BadCodeOrPassword(client);
		};
		match entry.get_mut().try_send(Box::new(client)) {
			Ok(()) => JoinResult::Ok,
			Err(e) => JoinResult::GameIsFullOrEnded(*e.into_inner()),
		}
	}
	
	pub async fn rejoin_game(&self, client: ClientConnection, code: String, password: String) -> JoinResult {
		let Some(mut entry) = self.runtime_store.get_async(&GameKey {code, password}).await else {
			return JoinResult::BadCodeOrPassword(client);
		};
		match entry.get_mut().try_send(Box::new(client)) {
			Ok(()) => JoinResult::Ok,
			Err(e) => JoinResult::GameIsFullOrEnded(*e.into_inner()),
		}
	}
	
	fn lobby_store(&self, password: &str) -> &GameStore {
		if password.is_empty() {
			&self.pub_lobby_store
		} else {
			&self.private_lobby_store
		}
	}
}

pub enum JoinResult {
	Ok,
	BadCodeOrPassword(ClientConnection),
	/// Full when joining a game lobby, ended when rejoining a running game that just ended
	GameIsFullOrEnded(ClientConnection),
}