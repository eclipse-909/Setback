use {
	crate::Version,
	const_format::formatcp,
	std::{
		time::Duration,
		env,
	},
};

// lobby and rules

pub const AI_NAME          : &str  = "CPU";
pub const JACK             : u8    = 11;
pub const QUEEN            : u8    = 12;
pub const KING             : u8    = 13;
pub const ACE              : u8    = 14;
pub const DECK_SIZE        : usize = 52;
pub const CARDS_PER_PLAYER : usize = 6;
/// 8 is the max number of players for each player to have 6 cards in a 52-card deck
pub const MAX_PLAYERS      : usize = 8;
/// 4 is the arbitrary min number of players that can play a game.
pub const MIN_PLAYERS      : u8    = 4;
pub const MAX_SCORE        : u8    = 21;
pub const PILE_SIZE        : usize = const {
	let s = CARDS_PER_PLAYER * MAX_PLAYERS;
	assert!(s <= DECK_SIZE);
	s
};

// client

pub const TEST_AUTH_TOKEN          : &str = "test-auth-token";
pub const APPLICATION_OCTET_STREAM : &str = "application/octet-stream";
pub const URL_API_LOGIN            : &str = "/login";
pub const ENV_SETBACK_HOST         : &str = "SETBACK_HOST";
const ENV_PUBLIC_API_BASE_URL      : &str = "PUBLIC_API_BASE_URL";
pub const ENV_PUBLIC_SERVER_URL    : &str = "PUBLIC_SERVER_URL";

/// Creates the absolute path to the public api. Path must start with a `/`.
/// This reads the environment variable each time,
/// so don't use if you need to read the environment variable multiple times.
pub fn public_api(path: &'static str) -> String {
	let base_url = env::var(ENV_PUBLIC_API_BASE_URL).unwrap();
	format!("{base_url}/api{path}")
}

// server

pub const ENV_SERVER_BIND_ADDR   : &str  = "SERVER_BIND_ADDR";
pub const ENV_API_KEY            : &str  = "API_KEY";
pub const ENV_API_BASE_URL       : &str  = "API_BASE_URL";
pub const ENV_SETBACK_ENV        : &str  = "SETBACK_ENV";
pub const API_HEADER_API_KEY     : &str  = "x-api-key";
/// Maximum number of games to be sent in a request over the network
pub const MAX_GAME_LIST          : u8    = 127;
pub const NETWORK_TIMEOUT        : u64   = 4;
pub const DEFAULT_TURN_TIME      : u8    = 30;
pub const MAX_TURN_TIME_SERVER   : u8    = 120;
pub const PAGE_SIZE              : usize = 0x1000;
pub const MAX_CLIENTS            : usize = 10_000;
pub const MAX_GAMES              : usize = 10_000;
/// Time between receiving a SIGINT and process exiting.
pub const TIMEOUT_STRIKES        : u8    = 3;
pub const GAME_CODE_LEN          : usize = 6;
pub const SSP_VERSION            : Version = Version { major: 0, minor: 1, patch: 0 };
pub const SHUTDOWN_TIMEOUT       : Duration = Duration::from_hours(4);
pub const GAME_CODE_ALPHABET     : [char; 62] = [
	'0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
	'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm',
	'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z',
	'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M',
	'N', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];