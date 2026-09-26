use {
	arrayvec::ArrayVec,
	crate::*,
	serde::{Deserialize, Serialize},
};

#[cfg(feature = "server")]
use std::{
	future::poll_fn,
	task::Poll,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Player {
	pub player_type: PlayerType,
	pub hand: ArrayVec<Card, CARDS_PER_PLAYER>,
	/// This is some arbitrary integer while in the lobby because team numbers can be arbitrary and nonconsecutive.
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
	#[cfg(not(feature = "server"))]
	SingleplayerSelf {
		/// Username may be empty if the username could not be read from the file.
		/// In that case, just don't display it.
		username: String,
	} = 0,
	/// A slot for a human player in a multiplayer lobby that doesn't have a connection yet.
	#[cfg(feature = "server")]
	HumanEmpty = 1,
	/// A connected player in multiplayer.
	/// This gets serialized into HumanPlayer because the [ClientConnection] struct is not serializable.
	///
	/// Futures will take ownership of the inner boxed ClientConnection and they leave behind None.
	/// When the future completes, the ClientConnection is restored.
	#[serde(skip_deserializing)]
	#[cfg(feature = "server")]
	HumanConnection {
		/// See [PlayerType::HumanConnection]
		#[serde(serialize_with = "serialize_conn_as_player")]
		conn: Box<ClientConnection>
	} = 2,
	/// The server serializes a HumanConnection into this variant when sending data to clients.
	/// The clients only need to know the username and ready state.
	/// This variant is never actually used in the runtime.
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
	#[cfg(not(feature = "server"))]
	pub fn new_singleplayer_self(team_id: u8, username: String) -> Self {
		Self {
			player_type: PlayerType::SingleplayerSelf { username },
			hand: ArrayVec::new(),
			team_id,
		}
	}

	///Empty slot for a human player in multiplayer.
	#[cfg(feature = "server")]
	pub const fn new_human_empty(team_id: u8) -> Self {
		Self {
			player_type: PlayerType::HumanEmpty,
			hand: ArrayVec::new_const(),
			team_id,
		}
	}

	#[cfg(feature = "server")]
	pub const fn new_host(conn: Box<ClientConnection>, team_id: u8) -> Self {
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
				#[cfg(feature = "server")]
				PlayerType::HumanConnection { conn } => {
					PlayerType::HumanPlayer {
						username: conn.username.clone(),
						ready: conn.ready,
					}
				},
				#[cfg(not(feature = "server"))]
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
			#[cfg(not(feature = "server"))]
			PlayerType::SingleplayerSelf { username, .. } => username.clone(),
			PlayerType::HumanPlayer { username, .. } => username.clone(),
			#[cfg(feature = "server")]
			PlayerType::HumanConnection { conn } => conn.username.clone(),
			#[cfg(feature = "server")]
			PlayerType::HumanEmpty => "Waiting for player...".to_string(),
			PlayerType::Ai { .. } => AI_NAME.to_string(),
		}
	}
	
	#[cfg(feature = "server")]
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
			_ = conn.send_event(GamePayload::LeaveGame);
		}
	}
	
	/// Returns None if all channels are closed.
	#[cfg(feature = "server")]
	pub async fn try_recv_fallback(players: &mut ArrayVec<Self, MAX_PLAYERS>) -> Option<(u8, Result<Ssp, NetError>)> {
		poll_fn(|cx| {
			let mut any_open = false;
			for (i, rx) in players
				.iter_mut()
				.enumerate()
				.flat_map(|(i, p)| match &mut p.player_type {
					PlayerType::HumanConnection {conn} => Some((i, &mut conn.fallback_rx)),
					_ => None,
				})
			{
				match rx.poll_recv(cx) {
					Poll::Ready(Some(item)) => return Poll::Ready(Some((i as u8, item))),
					Poll::Ready(None) => return Poll::Ready(Some((i as u8, Err(NetError::Disconnected)))),
					Poll::Pending => any_open = true,
				}
			}
			if any_open {
				Poll::Pending
			} else {
				Poll::Ready(None)
			}
		}).await
	}
}

#[cfg(feature = "server")]
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