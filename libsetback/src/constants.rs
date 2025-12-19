use {
	crate::ssp::Version,
	std::{
		net::{Ipv4Addr, SocketAddrV4},
		time::Duration,
	},
};

#[macro_export]
macro_rules! website {
	($path:expr) => {
		format!("{}{}", WEBSITE_URL, $path)
	};
}
pub(crate) use website;

///(major, minor, patch)
///
///Client will ignore the patch, but must upgrade to the latest major and minor version.
pub const SETBACK_VERSION: Version = {
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
				b'0' => 0,
				b'1' => 1,
				b'2' => 2,
				b'3' => 3,
				b'4' => 4,
				b'5' => 5,
				b'6' => 6,
				b'7' => 7,
				b'8' => 8,
				b'9' => 9,
				_ => panic!("invalid byte"),
			};
			let multiplier = 10u8.pow((i - (LEN - len)) as u32);
			nums[bufs_idx] += num * multiplier;
			i += 1;
		}
		bufs_idx += 1;
	}
	Version(nums[0], nums[1], nums[2])
};
pub const TIMEOUT: Duration = Duration::from_secs(10);
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

pub const fn server(port: u16) -> SocketAddrV4 {
	SocketAddrV4::new(
		if cfg!(debug_assertions) {
			Ipv4Addr::new(127, 0, 0, 1)
		} else {
			Ipv4Addr::new(127, 0, 0, 1) //TODO change this to actual socket once server is deployed
		},
		port,
	)
}

pub const WEBSITE_URL: &str = if cfg!(debug_assertions) {
	"http://127.0.0.1/"
} else {
	"https://127.0.0.1/" //TODO change this to actual URL once website is deployed
};

pub static URL_API_LOGIN: &str = "api/login";
pub static URL_API_LOGOUT: &str = "api/logout";
pub static URL_API_REFRESH: &str = "api/refresh";
pub static URL_API_ID: &str = "api/id";
pub static URL_API_UPDATE: &str = "api/update";
