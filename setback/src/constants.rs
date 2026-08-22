use {
	once_cell::sync::Lazy,
	regex::Regex,
	std::path::PathBuf,
};

//Files

pub static mut CONFIG_DIR: PathBuf = PathBuf::new();
pub static mut DATA_DIR: PathBuf = PathBuf::new();
pub static mut LOG_DIR: PathBuf = PathBuf::new();

//Images

pub const BOT_ICON: &[u8] = include_bytes!("../assets/images/botIcon.png");
pub const CARDS_ICON: &[u8] = include_bytes!("../assets/images/cardsIcon.png");
pub const CHECKMARK: &[u8] = include_bytes!("../assets/images/checkmark.png");
pub const DROPDOWN_ARROW: &[u8] = include_bytes!("../assets/images/dropdown-arrow.png");
pub const GEAR_ICON: &[u8] = include_bytes!("../assets/images/gear.png");
pub const PERSON_ICON: &[u8] = include_bytes!("../assets/images/personIcon.png");
pub const VOLUME_HIGH: &[u8] = include_bytes!("../assets/images/volume-high.png");
pub const VOLUME_LOW: &[u8] = include_bytes!("../assets/images/volume-low.png");
pub const VOLUME_MUTE: &[u8] = include_bytes!("../assets/images/volume-mute.png");
pub const X_ICON: &[u8] = include_bytes!("../assets/images/x.png");

//Suits

pub const SUIT_CLUBS: &[u8] = include_bytes!("../assets/images/suits/clubs.png");
pub const SUIT_DIAMONDS: &[u8] = include_bytes!("../assets/images/suits/diamonds.png");
pub const SUIT_HEARTS: &[u8] = include_bytes!("../assets/images/suits/hearts.png");
pub const SUIT_SPADES: &[u8] = include_bytes!("../assets/images/suits/spades.png");

//Cards

macro_rules! card_images {
	($($num_ident:ident $num_lit:literal),*,$($royal_str:literal $royal_ident:ident),*) => {
		paste::paste! {
			$(
				pub const [<CARD_CLUBS $num_ident>]: &[u8] = include_bytes!(concat!("../assets/images/cards/", $num_lit, "_of_clubs.png"));
				pub const [<CARD_DIAMONDS $num_ident>]: &[u8] = include_bytes!(concat!("../assets/images/cards/", $num_lit, "_of_diamonds.png"));
				pub const [<CARD_HEARTS $num_ident>]: &[u8] = include_bytes!(concat!("../assets/images/cards/", $num_lit, "_of_hearts.png"));
				pub const [<CARD_SPADES $num_ident>]: &[u8] = include_bytes!(concat!("../assets/images/cards/", $num_lit, "_of_spades.png"));
			)*
			$(
				pub const [<CARD_CLUBS_ $royal_ident>]: &[u8] = include_bytes!(concat!("../assets/images/cards/", $royal_str, "_of_clubs.png"));
				pub const [<CARD_DIAMONDS_ $royal_ident>]: &[u8] = include_bytes!(concat!("../assets/images/cards/", $royal_str, "_of_diamonds.png"));
				pub const [<CARD_HEARTS_ $royal_ident>]: &[u8] = include_bytes!(concat!("../assets/images/cards/", $royal_str, "_of_hearts.png"));
				pub const [<CARD_SPADES_ $royal_ident>]: &[u8] = include_bytes!(concat!("../assets/images/cards/", $royal_str, "_of_spades.png"));
			)*
		}
	};
}

card_images!(_2 2, _3 3, _4 4, _5 5, _6 6, _7 7, _8 8, _9 9, _10 10, "jack" JACK, "queen" QUEEN, "king" KING, "ace" ACE);
pub const CARD_BACK: &[u8] = include_bytes!("../assets/images/cards/back.png");

//Gifs

pub const LOADING: &[u8] = include_bytes!("../assets/images/loading.gif");

//Fonts

pub const GEORGIA: &[u8] = include_bytes!("../assets/fonts/georgia.ttf");
pub const GEORGIA_BOLD: &[u8] = include_bytes!("../assets/fonts/georgiab.ttf");
pub const GEORGIA_ITALIC: &[u8] = include_bytes!("../assets/fonts/georgiai.ttf");
pub const TIMES_NEW_ROMAN: &[u8] = include_bytes!("../assets/fonts/times.ttf");