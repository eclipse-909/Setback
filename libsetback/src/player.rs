use {
	crate::*,
	serde::{Deserialize, Serialize, Serializer, ser::SerializeStructVariant},
	tokio::sync::mpsc::{Receiver, Sender},
	tracing::info
};

#[derive(Debug)]
#[cfg(feature = "gpui")]
pub struct GpuiEventEmitter;
#[cfg(feature = "gpui")]
impl gpui::EventEmitter<GameEvent> for GpuiEventEmitter {}

#[cfg(feature = "gpui")]
pub type GameEventEmitter = gpui::Entity<GpuiEventEmitter>;
#[cfg(not(feature = "gpui"))]
pub type GameEventEmitter = tokio::sync::mpsc::UnboundedSender<GameEvent>;

pub fn get_username() -> Option<String> {
	match std::fs::read_to_string(USR_CRD) {
		Ok(username) => Some(username),
		Err(err) => {
			info!("Could not get username from usr.crd - error: {}", err);
			None
		}
	}
}

pub fn get_credentials() -> Option<LoginRequest> {
	let username = get_username()?;
	match keyring::Entry::new(
		KEYRING_SERVICE,
		username.as_str()
	).and_then(|entry| entry.get_password()) {
		Ok(password) => {
			info!("Retrieved credentials");
			Some(LoginRequest {username, password})
		}
		Err(err) => {
			info!("Could not get password from keyring - error: {}", err);
			None
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
	pub player_type: PlayerType,
	pub hand: SmallVec<Card, CARDS_PER_PLAYER>,
	pub team_id: u8,
}

impl Player {
	#[cfg(feature = "client")]
	pub fn new_singleplayer_self(team_id: u8, game_event_emitter: GameEventEmitter) -> (Self, Sender<Option<GameEvent>>) {
		let (tx, event_rx) = tokio::sync::mpsc::channel(1);
		(Self {
			player_type: PlayerType::SingleplayerSelf {
				username: get_username().unwrap_or_default(),
				game_event_emitter,
				event_rx,
			},
			hand: SmallVec::new(),
			team_id,
		}, tx)
	}

	///Empty slot for a human player in multiplayer.
	// TOOD: #[cfg(not(feature = "client"))]
	pub const fn new_human_empty(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::HumanEmpty,
			hand: SmallVec::new(),
			team_id,
		}
	}

	// TOOD: #[cfg(not(feature = "client"))]
	pub const fn new_host_connected(conn: ClientConnection, team_id: u8) -> Self {
		Self {
			player_type: PlayerType::HumanConnection {conn},
			hand: SmallVec::new(),
			team_id,
		}
	}

	pub const fn new_ai(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::Ai,
			hand: SmallVec::new(),
			team_id,
		}
	}

	pub fn can_follow(&self, lead: Suit) -> bool {
		self.hand.iter().any(|card| card.suit == lead)
	}

	pub const fn username(&self) -> &str {
		match &self.player_type {
			PlayerType::SingleplayerSelf { username, .. } => username.as_str(),
			PlayerType::HumanPlayer { username, .. } => username.as_str(),
			PlayerType::HumanConnection { conn: ClientConnection { username, .. } } => username.as_str(),
			PlayerType::HumanEmpty => "Waiting for player...",
			PlayerType::Ai => "CPU"
		}
	}
}

#[repr(u8)]
#[derive(Deserialize, Debug)]
pub enum PlayerType {
	/// Yourself in singleplayer.
	/// ## SAFETY
	/// It is undefined behavior to serialize this variant.
	#[serde(skip_deserializing)]
	#[cfg(feature = "client")]
	SingleplayerSelf {
		/// Username may be empty if the username could not be read from the file.
		/// In that case, just don't display it.
		username: String,
		game_event_emitter: GameEventEmitter,
		event_rx: Receiver<Option<GameEvent>>,
	} = 0,
	/// A slot for a human player in multiplayer that doesn't have a connection yet.
	// TODO: #[cfg(not(feature = "client"))]
	HumanEmpty = 1,
	/// A connected player in multiplayer.
	/// This gets serialized into HumanPlayer because the [ClientConnection] struct is not serializable.
	#[serde(skip_deserializing)]
	// TODO: #[cfg(not(feature = "client"))]
	HumanConnection { conn: ClientConnection } = 2,
	/// Same as HumanConnection
	HumanPlayer { username: String, ready: bool } = 3,
	Ai = 4,
}

impl Serialize for PlayerType {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		const PLAYER_TYPE: &str = "PlayerType";
		match self {
			PlayerType::HumanEmpty => Serializer::serialize_unit_variant(
				serializer,
				PLAYER_TYPE,
				1,
				"HumanEmpty",
			),
			PlayerType::HumanConnection { conn: ClientConnection { username, ready, ..} }
			| PlayerType::HumanPlayer { username, ready } => {
				let mut state = Serializer::serialize_struct_variant(
					serializer,
					PLAYER_TYPE,
					3,
					"HumanPlayer",
					2
				)?;
				SerializeStructVariant::serialize_field(&mut state, "username", username)?;
				SerializeStructVariant::serialize_field(&mut state, "ready", ready)?;
				state.end()
			}
			PlayerType::Ai => Serializer::serialize_unit_variant(
				serializer,
				PLAYER_TYPE,
				4,
				"AI",
			),
			_ => {
				#[cfg(debug_assertions)]
				unreachable!("Variant should not be serialized: {:?}", self);
				#[cfg(not(debug_assertions))]
				unsafe {std::hint::unreachable_unchecked();}
			}
		}
	}
}