use {
	crate::Version,
	std::{
		net::{Ipv4Addr, SocketAddrV4},
		time::Duration,
	},
};

pub const JACK: u8 = 11;
pub const QUEEN: u8 = 12;
pub const KING: u8 = 13;
pub const ACE: u8 = 14;
pub const CARDS_PER_PLAYER: usize = 6;
pub const DECK_SIZE: usize = 52;
pub const PILE_SIZE: usize = const {
	let s = CARDS_PER_PLAYER * MAX_PLAYERS;
	assert!(s <= DECK_SIZE);
	s
};
///Maximum number of games to be sent in a request over the network
pub const MAX_GAME_LIST: u8 = 253;
pub const MAX_SCORE: u8 = 21;
///8 is the max number of players for each player to have 6 cards in a 52-card deck
pub const MAX_PLAYERS: usize = 8;
///4 is the arbitrary min number of players that can play a game.
pub const MIN_PLAYERS: u8 = 4;
pub const PAGE_SIZE: usize = 4096;
/// This is the port that setbackd is bound to on the server. This is the port that clients must connect to initially.
pub const SETBACKD_PORT: u16 = 42069;
pub const NETWORK_TIMEOUT: Duration = Duration::from_secs(8);
pub const RETRY_ATTEMPTS: u8 = 3;
pub const DEFAULT_TURN_TIME: u8 = 30;
pub const MAX_TURN_TIME_SERVER: u8 = 120;

#[cfg(test)]
pub const TEST_AUTH_TOKEN: &str = "test";
pub const KEYRING_SERVICE: &str = "service-keyring-eclipse-setback";
/// `/run/setback` will be deleted by systemd when setbackd stops
pub const SETBACK_UNIX_SOCKET: &std::ffi::CStr = c"/run/setback/setback.sock";

#[cfg(debug_assertions)]
macro_rules! setback_path {
    ($path:literal) => {concat!("setback/", $path)};
}
#[cfg(not(debug_assertions))]
macro_rules! setback_path {
    ($path:literal) => {$path};
}

macro_rules! log_file {
	($file:expr) => {
		match std::str::from_utf8(&const {
			let mut buffer = [0; LOG_DIR.len() + 1 + $file.len()];
			let mut i = 0;
			while i < LOG_DIR.len() {
				buffer[i] = LOG_DIR.as_bytes()[i];
				i += 1;
			}
			buffer[LOG_DIR.len()] = b'/';
			i = 0;
			while i < $file.len() {
				buffer[i + LOG_DIR.len() + 1] = $file.as_bytes()[i];
				i += 1;
			}
			buffer
		}) {
			Ok(s) => s,
			Err(_) => panic!(),
		}
	};
}

pub const LOG_DIR: &str = setback_path!("logs");
pub const SETBACK_LOG: &str = "latest.log";
pub const DEP_LOG: &str = "dep.log";
pub const SETBACK_PATH: &str = log_file!(SETBACK_LOG);
pub const DEP_PATH: &str = log_file!(DEP_LOG);
pub const CRASH_PATH: &str = log_file!("crash.log");
pub const SETTINGS_JSON: &str = setback_path!("settings.json");
pub const USR_CRD: &str = setback_path!("usr.crd");

#[cfg(feature = "client")]
pub const fn server(port: u16) -> SocketAddrV4 {
	SocketAddrV4::new(
		#[cfg(debug_assertions)]
		Ipv4Addr::new(127, 0, 0, 1),
		#[cfg(not(debug_assertions))]
		Ipv4Addr::new(127, 0, 0, 1), //TODO change this to actual socket once server is deployed
		port,
	)
}

#[cfg(debug_assertions)]
macro_rules! website {
    ($path:literal) => {concat!("http://127.0.0.1/", $path)};
}
#[cfg(not(debug_assertions))]
macro_rules! website {
    ($path:literal) => {concat!("https://127.0.0.1/", $path)};
}

pub const ENDPOINT_LOGIN: &str = "/login";
pub const ENDPOINT_LOGOUT: &str = "/logout";
pub const ENDPOINT_USERNAME: &str = "/username";
pub const ENDPOINT_REFRESH: &str = "/refresh";

pub const URL_API_LOGIN: &str = website!("api/login");
pub const URL_API_LOGOUT: &str = website!("api/logout");
pub const URL_API_USERNAME: &str = website!("api/username");
pub const URL_API_REFRESH: &str = website!("api/refresh");

///Client will ignore the patch, but must upgrade to the latest major and minor version.
pub const SETBACK_VERSION: Version = {
	// Thanks Rust for not having 1st class support for compile time code execution.
	// Now I have to manually parse the string because str methods aren't const.

	const VERSION_STR: &str = env!("CARGO_PKG_VERSION");
	const LEN: usize = VERSION_STR.len();
	let mut buffers: [[u8; LEN]; 3] = [[0; LEN]; 3];
	let mut buf_lens: [usize; 3] = [0; 3];
	let mut nums: [u8; 3] = [0; 3];
	let mut bufs_idx = 0;
	let ptr = VERSION_STR.as_ptr();
	let mut i = 0;
	let mut buf_idx = LEN - 1;
	while i < LEN {
		let b = unsafe { *ptr.add(i) };
		match b {
			b'.' => {
				bufs_idx += 1;
				buf_idx = LEN - 1;
			}
			b'0'..=b'9' => {
				buffers[bufs_idx][buf_idx] = b;
				buf_lens[bufs_idx] += 1;
				buf_idx -= 1;
			}
			_ => panic!("invalid character in version string"),
		}
		i += 1;
	}
	bufs_idx = 0;
	while bufs_idx < 3 {
		let len = buf_lens[bufs_idx];
		i = LEN - len;
		loop {
			if i >= LEN {
				break;
			}
			let b = buffers[bufs_idx][i];
			let num = match b {
				b @ b'0'..=b'9' => b - b'0',
				_ => panic!("invalid byte"),
			};
			let multiplier = 10u8.pow((i - (LEN - len)) as u32);
			nums[bufs_idx] += num * multiplier;
			i += 1;
		}
		bufs_idx += 1;
	}
	Version { major: nums[0], minor: nums[1], patch: nums[2] }
};