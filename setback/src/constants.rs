use {once_cell::sync::Lazy, regex::Regex, vello::peniko::Color};

macro_rules! setback_path {
	($path:literal) => {
		if cfg!(debug_assertions) {
			concat!("setback/", $path)
		} else {
			$path
		}
	};
	($path:expr) => {
		if cfg!(debug_assertions) {
			concat!("setback/", $path)
		} else {
			$path
		}
	};
}

//Files

pub const LOG_DIR: &str = setback_path!("logs");
pub const CRASH_LOG: &str = setback_path!("logs/crash.log");
pub const SETTINGS_JSON: &str = setback_path!("settings.json");
pub const USR_CRD: &str = setback_path!("usr.crd");

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

//Static cursors

/// Spade
pub const BJ_NORMAL_SELECT: &[u8] = include_bytes!("../assets/cursors/Blackjack Normal Select.png");
/// Club
pub const BJ_CLUB_SELECT: &[u8] = include_bytes!("../assets/cursors/Blackjack-Club Select.png");
/// Heart
pub const BJ_HEART_SELECT: &[u8] = include_bytes!("../assets/cursors/Blackjack Heart Select.png");
/// Diamond
pub const BJ_DIAMOND_SELECT: &[u8] =
	include_bytes!("../assets/cursors/Blackjack Diamond Select.png");
/// Card front
pub const BJ_CARD_FRONT: &[u8] = include_bytes!("../assets/cursors/Blackjack Card Front.png");
/// Card back
pub const BJ_CARD_BACK: &[u8] = include_bytes!("../assets/cursors/Blackjack Card Back.png");

//Animated cursor data

pub const CURSOR_ATLAS: &[u8] = include_bytes!("../assets/cursors/cursor_atlas.png");
pub const CURSOR_ATLAS_JSON: &str = include_str!("../assets/cursors/cursor_atlas.json");

//Gifs

pub const LOADING: &[u8] = include_bytes!("../assets/images/loading.gif");

//Fonts

pub const GEORGIA: &[u8] = include_bytes!("../assets/fonts/georgia.ttf");
pub const GEORGIA_BOLD: &[u8] = include_bytes!("../assets/fonts/georgiab.ttf");
pub const GEORGIA_ITALIC: &[u8] = include_bytes!("../assets/fonts/georgiai.ttf");
pub const TIMES_NEW_ROMAN: &[u8] = include_bytes!("../assets/fonts/times.ttf");

// Text

pub const SINGLE_COMPONENT_QUERY_ERROR: &str =
	"There should be exactly one component returned from the query";
pub const TASK_ERROR: &str = "Failed to send command queue following task completion";
pub const KEYRING_PASSWORD: &str = "service-keyring-setback-user-password";
pub const URL_SIGNUP: &str = "signup";
pub const GAME_CODE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9]*$").unwrap());

// Styles

pub const fn color_hex(hex: u32) -> Color {
	Color::from_rgba8(
		((hex & 0xFF_00_00_00) >> 24) as u8,
		((hex & 0x00_FF_00_00) >> 16) as u8,
		((hex & 0x00_00_FF_00) >> 8) as u8,
		(hex & 0x00_00_00_FF) as u8,
	)
}
pub const COLOR_PRIMARY_GREEN: Color = color_hex(0x0d5d2eff);
pub const COLOR_SECONDARY_GREEN: Color = color_hex(0x1a7a3eff);
pub const COLOR_ACCENT_GOLD: Color = color_hex(0xd4af37ff);
pub const COLOR_LIGHT_GOLD: Color = color_hex(0xf4e87cff);
pub const COLOR_DARK_GOLD: Color = color_hex(0xb8941fff);
pub const COLOR_CREAM: Color = color_hex(0xf5f5dcff);
pub const COLOR_DARK_BROWN: Color = color_hex(0x2c1810ff);
pub const COLOR_MEDIUM_BROWN: Color = color_hex(0x4a2c1aff);
pub const COLOR_LIGHT_BROWN: Color = color_hex(0x8b6f47ff);
pub const COLOR_CARD_SHADOW: Color = color_hex(0x0000004d);
pub const COLOR_TEXT_SHADOW: Color = color_hex(0x00000080);
pub const COLOR_URL: Color = color_hex(0xadc2fcff);
pub const COLOR_DISABLED_COLOR: Color = color_hex(0x666666ff);
pub const COLOR_DISABLED_BG: Color = color_hex(0x333333ff);
pub const COLOR_MODAL_OVERLAY: Color = color_hex(0x000000b3);
pub const BORDER_RADIUS: f32 = 12.;
pub const CURSOR_FPS: f32 = 15.;
pub const MOUSE_WHEEL_PIXELS_PER_LINE: f32 = 40.;
