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
type LobbyStore = HashMap<GameKey, LobbyRtHandle>;
type RuntimeStore = HashMap<GameKey, GameRtHandle>;

/// State for the setback server.
///
/// The client store holds clients that aren't in a game.
/// Clients in a game will be owned by that game in either
/// the lobby store or game runtime store.
pub struct State {
	client_store: ClientStore,
	pub_lobby_store: LobbyStore,
	private_lobby_store: LobbyStore,
	runtime_store: RuntimeStore,
	gsp_tx: GspTx,
	lsp_tx: LspTx,
}

impl State {
	pub fn new() -> (Arc<Self>, GspRx, LspRx) {
		let (gsp_tx, gsp_rx) = unbounded_channel();
		let (lsp_tx, lsp_rx) = unbounded_channel();
		(Arc::new(Self {
			client_store: HashSet::new(),
			pub_lobby_store: HashMap::new(),
			private_lobby_store: HashMap::new(),
			runtime_store: HashMap::new(),
			gsp_tx,
			lsp_tx,
		}), gsp_rx, lsp_rx)
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
	
	pub async fn create_game(
		&self,
		host_client: ClientConnection,
		password: String,
		lsp_tx: LspTx,
	) -> Result<String, Error> {
		let num_games = self.pub_lobby_store.len() + self.private_lobby_store.len() + self.runtime_store.len();
		if num_games >= MAX_GAMES {
			return Err(Error::GamesAtCapacity);
		}
		let (handle, code) = Lobby::create_multiplayer(Box::new(host_client), password.clone(), lsp_tx);
		let lobby_store = self.lobby_store(&password);
		let key = GameKey {code: code.clone(), password};
		if lobby_store.contains_async(&key).await {
			error!("Generated game code already exists");
			return Err(Error::GamesAtCapacity);
		}
		_ = lobby_store.insert_async(key, handle).await;
		Ok(code)
	}
	
	pub async fn start_game(&self) {}
	
	pub async fn rm_game(&self) {}
	
	pub async fn join_game(&self, client: ClientConnection, code: String, password: String) -> JoinResult {
		let Some(mut entry) = self.lobby_store(&password).get_async(&GameKey {code, password}).await else {
			return JoinResult::BadCodeOrPassword(client);
		};
		match entry.get_mut().join_tx.try_send(Box::new(client)) {
			Ok(()) => JoinResult::Ok,
			Err(e) => JoinResult::GameIsFullOrEnded(*e.into_inner()),
		}
	}
	
	pub async fn rejoin_game(&self, client: ClientConnection, code: String, password: String) -> JoinResult {
		let Some(mut entry) = self.runtime_store.get_async(&GameKey {code, password}).await else {
			return JoinResult::BadCodeOrPassword(client);
		};
		match entry.get_mut().rejoin_tx.try_send(Box::new(client)) {
			Ok(()) => JoinResult::Ok,
			Err(e) => JoinResult::GameIsFullOrEnded(*e.into_inner()),
		}
	}
	
	fn lobby_store(&self, password: &str) -> &LobbyStore {
		if password == "" {
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