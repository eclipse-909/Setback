mod game_items;
mod ssp;
mod util;
mod client;
mod game;

///Items used by the server. You can just use resouces::server_items::*
pub mod server_items {
	pub use crate::{
		client::Client,
		ssp::{Ssp, Protocol},
		util::{ServerError, ClientError, Version, spawn_async, spawn_sync, MAX_SERVER_CLIENTS, MAX_PLAYERS},
		game::{SPGame, TurnResult},
		game_items::{Player, PlayerType, PlayerClient, GameFilter, GameMeta, GameData, Team, Card, Suit, CARDS_PER_PLAYER, J, Q, K, A, MAX_GAME_METAS}
	};
}
///Items used by the client. You can just use resouces::client_items::*
pub mod client_items {
	pub use crate::{
		ssp::{Ssp, Protocol},
		util::{ServerError, ClientError, Version, MAX_PLAYERS, MIN_PLAYERS},
		game::{SPGame, TurnResult},
		game_items::{Player, PlayerType, GameFilter, Comparison, GameMeta, GameData, Team, Card, Suit, RoundScores, CARDS_PER_PLAYER},
		client::{AccountData, AccountResponse}
	};
}
///Items used by the website. You can just use resouces::website_items::*
pub mod website_items {
	pub use crate::client::{AccountData, AccountResponse};
}