use std::net::{Ipv4Addr, SocketAddrV4};

pub const JACK: u8 = 11;
pub const QUEEN: u8 = 12;
pub const KING: u8 = 13;
pub const ACE: u8 = 14;
pub const CARDS_PER_PLAYER: u8 = 6;
///Maximum number of game metas to be sent in a request over the network
pub const MAX_GAME_METAS: u8 = 253;
///8 is the max number of players for each player to have 6 cards in a 52-card deck
pub const MAX_PLAYERS: u8 = 8;
///4 is the arbitrary min number of players that can play a game.
pub const MIN_PLAYERS: u8 = 4;
pub const PAGE_SIZE: usize = 4096;
pub const ACCEPT_PORT: u16 = 42069;

pub const SERVER_SOCKET: SocketAddrV4 = SocketAddrV4::new(
	if cfg!(debug_assertions) {
		Ipv4Addr::new(127, 0, 0, 1)
	} else {
		Ipv4Addr::new(127, 0, 0, 1)//TODO change this to actual socket once server is deployed
	},
	ACCEPT_PORT
);

pub const WEBSITE_URL: &str = if cfg!(debug_assertions) {
	"http://127.0.0.1/"
} else {
	"https://127.0.0.1/"//TODO change this to actual URL once website is deployed
};

pub static URL_API_LOGIN: &str = "/api/login";
pub static URL_API_SIGNUP: &str = "/api/signup";
pub static URL_API_LOGOUT: &str = "/api/logout";
pub static URL_API_LOGGED_IN: &str = "/api/logged_in";
pub static URL_API_CHANGE_USERNAME: &str = "/api/change_username";
pub static URL_API_CHANGE_PASSWORD: &str = "/api/change_password";
pub static URL_API_DELETE_ACCOUNT: &str = "/api/delete_account";
pub static URL_API_KEEP_ALIVE: &str = "/api/keep_alive";

#[macro_export]
macro_rules! website {
	($path:expr) => {format!("{}{}", WEBSITE_URL, $path)};
}
pub(crate) use website;