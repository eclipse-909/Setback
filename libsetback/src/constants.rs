use {
	crate::Version,
	std::net::{Ipv4Addr, SocketAddrV4}
};

// setup

pub const JACK: u8 = 11;
pub const QUEEN: u8 = 12;
pub const KING: u8 = 13;
pub const ACE: u8 = 14;
pub const DECK_SIZE: usize = 52;
pub const CARDS_PER_PLAYER: usize = 6;
/// 8 is the max number of players for each player to have 6 cards in a 52-card deck
pub const MAX_PLAYERS: usize = 8;
/// 4 is the arbitrary min number of players that can play a game.
pub const MIN_PLAYERS: u8 = 4;
pub const MAX_SCORE: u8 = 21;
pub const PILE_SIZE: usize = const {
	let s = CARDS_PER_PLAYER * MAX_PLAYERS;
	assert!(s <= DECK_SIZE);
	s
};

// api

macro_rules! api { ($path:literal) => {concat!("http://127.0.0.1:8080/api/", $path)}; }
pub const URL_API_LOGIN: &str = api!("login");
pub const URL_API_LOGOUT: &str = api!("logout");
pub const URL_API_USERNAME: &str = api!("username");
pub const URL_API_RENEW: &str = api!("renew");
pub const URL_API_REPORT_PLAYER: &str = api!("report_player");
#[cfg(test)]
pub const TEST_AUTH_TOKEN: &str = "test";

// networking

pub const PAGE_SIZE: usize = 4096;
pub const SDP_VERSION: Version = Version {major: 0, minor: 1, patch: 0};
pub const SSP_VERSION: Version = Version {major: 0, minor: 1, patch: 0};
/// Maximum number of games to be sent in a request over the network
pub const MAX_GAME_LIST: u8 = 127;
/// This is the port that setbackd is bound to on the server. This is the port that clients must connect to initially.
pub const SETBACKD_PORT: u16 = 42069;
pub const NETWORK_TIMEOUT: u64 = 4;
pub const DEFAULT_TURN_TIME: u8 = 30;
pub const MAX_TURN_TIME_SERVER: u8 = 120;
pub const DNS_NAME: &str = "www.setback.com";
pub const MAX_CLIENTS: usize = 10_000;
pub const MAX_GAMES: usize = 10_000;
pub const fn server(port: u16) -> SocketAddrV4 {
	SocketAddrV4::new(
		Ipv4Addr::new(127, 0, 0, 1), //TODO change this to actual address once server is deployed
		port,
	)
}

// files

macro_rules! runtime_dir {($path:literal) => {concat!("/run/setback/", $path)};}
// `run/setback/daemon` and `run/setback/website` will be deleted by systemd when those services stop.
pub const SETBACKD_UNIX_SOCKET: &str = runtime_dir!("daemon/setbackd.sock");
pub const SETBACK_WEBSITE_UNIX_SOCKET: &str = runtime_dir!("website/setback_website.sock");
pub const SETBACKCTL_UNIX_SOCKET: &str = runtime_dir!("setbackctl.sock");
pub fn setback_server_unix_socket(pid: i32) -> String { format!(runtime_dir!("setback_server-{}.sock"), pid) }
pub const KEYRING_SERVICE: &str = "service-keyring-eclipse-909-setback";