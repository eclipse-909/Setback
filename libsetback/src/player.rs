use {
	arrayvec::ArrayVec,
	crate::*,
	serde::{Deserialize, Serialize, Serializer},
	tracing::error
};

const NONE_CONN_ERR: &str = "Player must own their connection when calling this function";

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
	pub player_type: PlayerType,
	pub hand: ArrayVec<Card, CARDS_PER_PLAYER>,
	/// This is some arbitrary integer during the setup because team numbers can be arbitrary and nonconsecutive.
	/// Once the game starts, team ids are normalized to be consecutive and zero-based,
	/// meaning they can be used as an index into an array of teams.
	pub team_id: u8,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Debug)]
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
	/// Futures will take ownership of the inner boxed ClientConnection and they leave behind None.
	/// When the future completes, the ClientConnection is restored.
	#[serde(skip_deserializing)]
	#[cfg(not(feature = "client"))]
	HumanConnection {
		/// See [PlayerType::HumanConnection]
		#[serde(serialize_with = "serialize_conn_as_player")]
		conn: Option<Box<ClientConnection>>
	} = 2,
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
			player_type: PlayerType::SingleplayerSelf { username },
			hand: ArrayVec::new(),
			team_id,
		}
	}

	///Empty slot for a human player in multiplayer.
	#[cfg(not(feature = "client"))]
	pub const fn new_human_empty(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::HumanEmpty,
			hand: ArrayVec::new_const(),
			team_id,
		}
	}

	#[cfg(not(feature = "client"))]
	pub const fn new_host_connected(conn: Box<ClientConnection>, team_id: u8) -> Self {
		Self {
			player_type: PlayerType::HumanConnection { conn: Some(conn) },
			hand: ArrayVec::new_const(),
			team_id,
		}
	}

	pub const fn new_ai(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::Ai { account_id: 0 },
			hand: ArrayVec::new_const(),
			team_id,
		}
	}

	pub fn can_follow(&self, lead: Suit) -> bool {
		self.hand.iter().any(|card| card.suit == lead)
	}
	
	/// Takes the connection from the player if they are a HumanConnection variant.
	/// Returns None if they are some other variant.
	/// ## Panics - debug_assertions only
	/// If this player is a HumanConnection and they don't have the connection.
	/// This implies take_conn was called twice without the connection being restored in between.
	/// Returns None in release mode and logs an error.
	pub fn take_conn(&mut self) -> Option<Box<ClientConnection>> {
		if let PlayerType::HumanConnection {conn} = &mut self.player_type {
			if conn.is_none() {
				error!("Logical error: a player had take_conn called with a missing ClientConnection");
			}
			debug_assert!(conn.is_some());
			return conn.take()
		}
		None
	}
	
	/// Gives the connection back to the player after calling [Self::take_conn].
	/// ## Panics - debug_assertions only
	/// ### Case 1
	/// If this player is a HumanConnection and they already have the connection.
	/// This implies restore_conn was called twice without the connection being taken in between.
	/// Logs an error in release mode.
	/// ### Case 2
	/// If the player is not a HumanConnection.
	/// The connection will just be dropped in release mode and logs an error.
	pub fn restore_conn(&mut self, connection: Box<ClientConnection>) {
		if let PlayerType::HumanConnection {conn} = &mut self.player_type {
			if conn.is_some() {
				error!("Logical error: a player had restore_conn called with an existing ClientConnection");
			}
			debug_assert!(conn.is_none());
			*conn = Some(connection);
		}
		const ERR_MSG: &str = "Logical error: restore_conn should only be called on HumanConnection players";
		error!("{ERR_MSG}");
		#[cfg(debug_assertions)]
		unreachable!("{ERR_MSG}");
	}
	
	/// ## Notice
	/// Does not clone the player's hand, that is just an empty list.
	/// This function is only used when a player is trying to rejoin an ongoing game that they disconnected from
	/// and needs to get the game state.
	/// Do not use this function unless you know what it does and what you're doing.
	/// Only HumanConnection, HumanPlayer, and Ai variants can be cloned. Others will panic.
	/// The server can send a snapshot to the players, and the host can send a snapshot to the server,
	/// so it needs to be supported from both the client and server side.
	pub fn snapshot(&self) -> Self {
		Self {
			player_type: match &self.player_type {
				#[cfg(not(feature = "client"))]
				PlayerType::HumanConnection { conn: Some(conn) } => {
					PlayerType::HumanPlayer {
						username: conn.username.clone(),
						ready: conn.ready,
					}
				},
				#[cfg(not(feature = "client"))]
				PlayerType::HumanConnection { conn: None } => unreachable!("{NONE_CONN_ERR}"),
				#[cfg(feature = "client")]
				PlayerType::HumanPlayer { username, ready } => PlayerType::HumanPlayer {
					username: username.clone(),
					ready: *ready,
				},
				PlayerType::Ai { .. } => PlayerType::Ai { account_id: 0 },
				_ => unreachable!("Variant should not be cloned: {:?}", self),
			},
			hand: ArrayVec::new_const(),
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
			PlayerType::HumanConnection { conn: Some(conn) } => conn.username.as_str(),
			#[cfg(not(feature = "client"))]
			PlayerType::HumanConnection { conn: None } => unreachable!("{NONE_CONN_ERR}"),
			#[cfg(not(feature = "client"))]
			PlayerType::HumanEmpty => "Waiting for player...",
			PlayerType::Ai { .. } => "CPU",
		}
	}
}

fn serialize_conn_as_player<S>(conn: &Option<Box<ClientConnection>>, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	#[derive(Serialize)]
	#[serde(rename = "HumanPlayer")]
	struct HumanPlayerShim<'a> {
		username: &'a str,
		ready: bool,
	}
	let conn = conn.as_ref().expect(NONE_CONN_ERR);
	HumanPlayerShim {
		username: &conn.username,
		ready: conn.ready,
	}.serialize(serializer)
}