use {
	crate::*,
	serde::{Deserialize, Serialize, Serializer, ser::SerializeStructVariant},
	std::cell::SyncUnsafeCell,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
	#[serde(with = "serde_sync_unsafe_cell")]
	pub player_type: SyncUnsafeCell<PlayerType>,
	pub hand: SmallVec<Card, CARDS_PER_PLAYER>,
	/// This is some arbitrary integer during the setup because team numbers can be arbitrary and nonconsecutive.
	/// Once the game starts, team ids are normalized to be consecutive and zero-based,
	/// meaning they can be used as an index into an array of teams.
	pub team_id: u8,
}

#[repr(u8)]
#[derive(Deserialize, Debug)]
pub enum PlayerType {
	/// Yourself in singleplayer.
	#[serde(skip)]
	#[cfg(feature = "client")]
	SingleplayerSelf {
		/// Username may be empty if the username could not be read from the file.
		/// In that case, just don't display it.
		username: String,
	} = 0,
	/// A slot for a human player in a multiplayer game setup that doesn't have a connection yet.
	#[cfg(not(feature = "client"))]
	HumanEmpty = 1,
	/// A connected player in multiplayer.
	/// This gets serialized into HumanPlayer because the [ClientConnection] struct is not serializable.
	///
	/// Futures will take ownership of the inner ClientConnection and they leave behind HumanPlayer.
	/// When the future completes, the HumanConnection variant is restored.
	#[serde(skip_deserializing)]
	#[cfg(not(feature = "client"))]
	HumanConnection { conn: Box<ClientConnection> } = 2,
	/// This variant has 2 uses:
	/// 1. The server serializes a HumanConnection into this variant when sending data to clients.
	///    The clients only need to know the username and ready state.
	/// 2. A HumanConnection in a multiplayer game that has had its ClientConnection moved out and into a future,
	///    where it can be used for communication. The future will return the connection when done and turned back into HumanConnection.
	HumanPlayer { username: String, ready: bool } = 3,
	/// An NPC
	Ai {
		/// If a client disconnects, their player is turned into an AI.
		/// This account id keeps track of that account to allow the player to reconnect.
		account_id: u32,
	} = 4,
}

impl Player {
	/// The sender returned is used to respond to prompts
	#[cfg(feature = "client")]
	pub fn new_singleplayer_self(team_id: u8, username: String) -> Self {
		Self {
			player_type: SyncUnsafeCell::new(PlayerType::SingleplayerSelf { username }),
			hand: SmallVec::new(),
			team_id,
		}
	}

	///Empty slot for a human player in multiplayer.
	#[cfg(not(feature = "client"))]
	pub const fn new_human_empty(team_id: u8) -> Self {
		Self {
			player_type: SyncUnsafeCell::new(PlayerType::HumanEmpty),
			hand: SmallVec::new(),
			team_id,
		}
	}

	#[cfg(not(feature = "client"))]
	pub const fn new_host_connected(conn: Box<ClientConnection>, team_id: u8) -> Self {
		Self {
			player_type: SyncUnsafeCell::new(PlayerType::HumanConnection { conn }),
			hand: SmallVec::new(),
			team_id,
		}
	}

	pub const fn new_ai(team_id: u8) -> Self {
		Self {
			player_type: SyncUnsafeCell::new(PlayerType::Ai { account_id: 0 }),
			hand: SmallVec::new(),
			team_id,
		}
	}

	pub fn can_follow(&self, lead: Suit) -> bool {
		self.hand.iter().any(|card| card.suit == lead)
	}
}

impl Clone for Player {
	/// ## Notice
	/// Does not clone the player's hand, that is just an empty list.
	/// This function is only used when a player is trying to rejoin an ongoing game that they disconnected from
	/// and needs to get the game state.
	/// Do not use this function unless you know what it does and what you're doing.
	/// ## Safety
	/// Only HumanConnection, HumanPlayer, and Ai variants can be cloned. Others are undefined behavior.
	fn clone(&self) -> Self {
		Self {
			player_type: SyncUnsafeCell::new(
				// SAFETY:
				// We just need readonly access
				match unsafe { &*self.player_type.get() } {
					#[cfg(not(feature = "client"))]
					PlayerType::HumanConnection { conn } => PlayerType::HumanPlayer {
						username: conn.username.clone(),
						ready: conn.ready,
					},
					#[cfg(feature = "client")]
					PlayerType::HumanPlayer { username, ready } => PlayerType::HumanPlayer {
						username: username.clone(),
						ready: *ready,
					},
					PlayerType::Ai { .. } => PlayerType::Ai { account_id: 0 },
					_ => unreachable!("Variant should not be cloned: {:?}", self),
				},
			),
			hand: SmallVec::new(),
			team_id: self.team_id,
		}
	}
}

impl PlayerType {
	pub fn username(&self) -> &str {
		match self {
			#[cfg(feature = "client")]
			PlayerType::SingleplayerSelf { username, .. } => username.as_str(),
			PlayerType::HumanPlayer { username, .. } => username.as_str(),
			#[cfg(not(feature = "client"))]
			PlayerType::HumanConnection { conn } => conn.username.as_str(),
			#[cfg(not(feature = "client"))]
			PlayerType::HumanEmpty => "Waiting for player...",
			PlayerType::Ai { .. } => "CPU",
		}
	}
}

impl Serialize for PlayerType {
	fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		const PLAYER_TYPE: &str = "PlayerType";
		match self {
			#[cfg(not(feature = "client"))]
			PlayerType::HumanEmpty => {
				Serializer::serialize_unit_variant(serializer, PLAYER_TYPE, 1, "HumanEmpty")
			}
			#[cfg(not(feature = "client"))]
			PlayerType::HumanConnection { conn } => {
				let mut state = Serializer::serialize_struct_variant(
					serializer,
					PLAYER_TYPE,
					3,
					"HumanPlayer",
					2,
				)?;
				SerializeStructVariant::serialize_field(&mut state, "username", &conn.username)?;
				SerializeStructVariant::serialize_field(&mut state, "ready", &conn.ready)?;
				state.end()
			}
			#[cfg(feature = "client")]
			PlayerType::HumanPlayer { username, ready } => {
				let mut state = Serializer::serialize_struct_variant(
					serializer,
					PLAYER_TYPE,
					3,
					"HumanPlayer",
					2,
				)?;
				SerializeStructVariant::serialize_field(&mut state, "username", username)?;
				SerializeStructVariant::serialize_field(&mut state, "ready", ready)?;
				state.end()
			}
			PlayerType::Ai { .. } => {
				let mut state =
					Serializer::serialize_struct_variant(serializer, PLAYER_TYPE, 4, "Ai", 1)?;
				SerializeStructVariant::serialize_field(&mut state, "account_id", &0)?;
				state.end()
			}
			_ => Err(serde::ser::Error::custom(
				"variant should not be serialized: {:?}",
			)),
		}
	}
}

mod serde_sync_unsafe_cell {
	use {super::*, serde::Deserializer};

	pub fn serialize<S: Serializer>(
		this: &SyncUnsafeCell<PlayerType>,
		serializer: S,
	) -> Result<S::Ok, S::Error> {
		// SAFETY:
		// just need readonly access
		unsafe { &*this.get() }.serialize(serializer)
	}

	pub fn deserialize<'de, D: Deserializer<'de>>(
		deserializer: D,
	) -> Result<SyncUnsafeCell<PlayerType>, D::Error> {
		Ok(SyncUnsafeCell::new(PlayerType::deserialize(deserializer)?))
	}
}
