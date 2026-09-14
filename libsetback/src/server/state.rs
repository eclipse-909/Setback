use {
	crate::*,
	std::sync::Arc,
	scc::{HashMap, HashSet},
	tokio::sync::mpsc::unbounded_channel,
	tracing::error
};

#[derive(Hash, Eq, PartialEq)]
struct GameKey {
	code: String,
	password: String,
}

type ClientStore = HashSet<u32>;
type LobbyStore = HashMap<GameKey, Lobby>;
type RuntimeStore = HashMap<GameKey, GameLoopHandle>;

/// State for the setback server.
///
/// The client store holds clients that aren't in a game.
/// Clients in a game will be owned by that game in either
/// the lobby store or game runtime store.
pub struct State {
	client_store: ClientStore,
	lobby_store: LobbyStore,
	runtime_store: RuntimeStore,
	gsp_tx: GspTx,
}

impl State {
	pub fn new() -> (Arc<Self>, GspRx) {
		let (gsp_tx, rx) = unbounded_channel();
		(Arc::new(Self {
			client_store: HashSet::new(),
			lobby_store: HashMap::new(),
			runtime_store: HashMap::new(),
			gsp_tx,
		}), rx)
	}
	
	/// Returns true if the client could not be added.
	pub async fn add_client(&self, client: &mut ClientConnection) -> bool {
		if self.client_store.insert_async(client.account_id).await.is_err() {
			_ = client.send_err::<ServerPayload>(Error::AlreadyConnected);
			//TODO: if a client is trying to connect more than once,
			// it could either mean they are trying to cheat by being 2 players at once in a game,
			// or someone has hacked their account.
			// Either way we should do something with this.
			// Maybe send the client a message/notification/email that someone is trying to login to their account.
			// Suggest they should change their password.
			// Maybe have a "this was me" and "this wasn't me" response
			// which would confirm the cause.
			return true;
		}
		false
	}
	
	pub async fn rm_client(&self, account_id: u32) {
		_ = self.client_store.remove_async(&account_id).await;
	}
	
	pub async fn create_game(&self, host_client: ClientConnection,  password: String) -> Result<String, Error> {
		if self.lobby_store.len() + self.runtime_store.len() >= MAX_GAMES {
			return Err(Error::GamesAtCapacity);
		}
		let (lobby, code) = Lobby::create_multiplayer(Box::new(host_client), password.clone());
		let key = GameKey {code: code.clone(), password};
		if self.lobby_store.contains_async(&key).await {
			error!("Generated game code already exists");
			return Err(Error::GamesAtCapacity);
		}
		_ = self.lobby_store.insert_async(key, lobby).await;
		Ok(code)
	}
	
	pub async fn start_game(&self) {}
	
	pub async fn rm_game(&self) {}
	
	pub async fn join_game(&self, client: ClientConnection, code: String, password: String) -> JoinResult {
		let Some(mut entry) = self.lobby_store.get_async(&GameKey {code, password}).await else {
			return JoinResult::BadCodeOrPassword(client);
		};
		match entry.get_mut().set_next_client(client) {
			Ok(()) => JoinResult::Ok,
			Err(e) => JoinResult::GameIsFullOrEnded(e),
		}
	}
	
	pub async fn rejoin_game(&self, client: ClientConnection, code: String, password: String) -> JoinResult {
		let Some(mut entry) = self.runtime_store.get_async(&GameKey {code, password}).await else {
			return JoinResult::BadCodeOrPassword(client);
		};
		match entry.get_mut().rejoin_tx.send(Box::new(client)).await.map_err(|e| *e.0) {
			Ok(()) => JoinResult::Ok,
			Err(e) => JoinResult::GameIsFullOrEnded(e),
		}
	}
}

pub enum JoinResult {
	Ok,
	BadCodeOrPassword(ClientConnection),
	/// Full when joining a game lobby, ended when rejoining a running game that just ended
	GameIsFullOrEnded(ClientConnection),
}