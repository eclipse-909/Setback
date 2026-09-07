use {
	arrayvec::ArrayVec,
	crate::*,
	serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
	pub player_type: PlayerType,
	pub hand: ArrayVec<Card, CARDS_PER_PLAYER>,
	/// This is some arbitrary integer during the setup because team numbers can be arbitrary and nonconsecutive.
	/// Once the game starts, team ids are normalized to be consecutive and zero-based,
	/// meaning they can be used as an index into an array of teams.
	pub team_id: u8,
}

/// Variant numbers must be explicit because some variants are feature toggled.
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
		conn: Box<ClientConnection>
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
			player_type: PlayerType::HumanConnection { conn },
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
				PlayerType::HumanConnection { conn } => {
					PlayerType::HumanPlayer {
						username: conn.username.clone(),
						ready: conn.ready,
					}
				},
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
	
	pub fn username(&self) -> String {
		match &self.player_type {
			#[cfg(feature = "client")]
			PlayerType::SingleplayerSelf { username, .. } => username.clone(),
			PlayerType::HumanPlayer { username, .. } => username.clone(),
			#[cfg(not(feature = "client"))]
			PlayerType::HumanConnection { conn } => conn.username.clone(),
			#[cfg(not(feature = "client"))]
			PlayerType::HumanEmpty => "Waiting for player...".to_string(),
			PlayerType::Ai { .. } => "CPU".to_string(),
		}
	}
	
	#[cfg(not(feature = "client"))]
	pub fn disconnect(&mut self, allow_reconnect: bool) {
		let PlayerType::HumanConnection {conn} = &mut self.player_type else {return;};
		let id = conn.account_id;
		let PlayerType::HumanConnection {mut conn} = std::mem::replace(
			&mut self.player_type,
			PlayerType::Ai { account_id: if allow_reconnect {id} else {0} }
		) else {return;};
		if allow_reconnect {
			_ = conn.send_err(NetError::Disconnected.into());
		} else {
			_ = conn.send_event(Payload::LeaveGame);
		}
	}
}

#[cfg(not(feature = "client"))]
fn serialize_conn_as_player<S>(conn: &ClientConnection, serializer: S) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	#[derive(Serialize)]
	#[serde(rename = "HumanPlayer")]
	struct HumanPlayerShim<'a> {
		username: &'a str,
		ready: bool,
	}
	HumanPlayerShim {
		username: &conn.username,
		ready: conn.ready,
	}.serialize(serializer)
}