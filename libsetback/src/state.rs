use {
	crate::{
		constants::*,
		connection::*,
		setup::*,
		runtime::*,
	},
	std::sync::Arc,
	scc::HashMap
};

pub type ClientStore = HashMap<u32, ClientConnection>;
pub type GameSetupStore = HashMap<u32, GameSetup>;
pub type GameRuntimeStore = HashMap<u32, GameLoopHandle>;

/// State for the setback server.
///
/// The client store holds clients that aren't in a game.
/// Clients in a game will be owned by that game in either
/// the game setup store or game runtime store.
pub struct State {
	pub client_store: ClientStore,
	pub game_setup_store: GameSetupStore,
	pub game_runtime_store: GameRuntimeStore,
}

impl State {
	pub fn new() -> Arc<Self> {
		Arc::new(Self {
			client_store: HashMap::with_capacity(MAX_CLIENTS),
			game_setup_store: HashMap::with_capacity(MAX_GAMES),
			game_runtime_store: HashMap::with_capacity(MAX_GAMES),
		})
	}

	pub fn add_client() {}

	pub fn create_game() {}

	pub fn join_game() {}

	pub fn leave_game() {}
}