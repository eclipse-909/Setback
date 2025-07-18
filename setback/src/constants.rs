use {
	crate::models::*,
	bevy::prelude::*
};

pub static CARDS_ICON: &[u8] = include_bytes!("../assets/images/cardsIcon.png");
pub static CURSOR_ATLAS: &str = "setback/assets/cursors/cursor_atlas.json";//TODO these paths may need to be changed for release
pub static USR_CRD: &str = "setback/usr/usr.crd";
pub static LATEST_LOG: &str = "setback/logs/latest.log";
pub static CRASH_LOG: &str = "setback/logs/crash.log";
pub static CURSOR_ERROR: &str = "There should be exactly one cursor at all times";
pub static BTN_CONTAINER_ERR: &str = "There should be exactly one ButtonContainer";
pub static KEYRING_PASSWORD: &str = "service-keyring-setback-user-password";
pub static URL_INDEX: &str = "";
pub static URL_404: &str = "file_not_found";
pub static URL_DOWNLOAD: &str = "download";
pub static URL_LOGIN: &str = "login";
pub static URL_ACCOUNT: &str = "account";
pub static GEORGIA: &str = "fonts/georgia.ttf";
pub static GEORGIA_BOLD: &str = "fonts/georgiab.ttf";
pub static GEORGIA_ITALIC: &str = "fonts/georgiai.ttf";
pub static TIMES_NEW_ROMAN: &str = "fonts/times.ttf";

/// Singleton for [Pallet] struct
pub static PALLET: Pallet = Pallet {
	primary_green: Color::srgb_u8(0x0d, 0x5d, 0x2e),
	secondary_green: Color::srgb_u8(0x1a, 0x7a, 0x3e),
	accent_gold: Color::srgb_u8(0xd4, 0xaf, 0x37),
	light_gold: Color::srgb_u8(0xf4, 0xe8, 0x7c),
	dark_gold: Color::srgb_u8(0xb8, 0x94, 0x1f),
	cream: Color::srgb_u8(0xf5, 0xf5, 0xdc),
	dark_brown: Color::srgb_u8(0x2c, 0x18, 0x10),
	medium_brown: Color::srgb_u8(0x4a, 0x2c, 0x1a),
	light_brown: Color::srgb_u8(0x8b, 0x6f, 0x47),
	card_shadow: Color::srgba(0., 0., 0., 0.3),
	text_shadow: Color::srgba(0., 0., 0., 0.5),
	url: Color::srgb_u8(173, 194, 252),
	border_radius: Val::Px(12.)
};

pub const CURSOR_FPS: f32 = 15.;

/// Spade
pub static BJ_NORMAL_SELECT: &str = "cursors/Blackjack Normal Select.png";
/// Club
pub static BJ_CLUB_SELECT: &str = "cursors/Blackjack-Club Select.png";
/// Heart
pub static BJ_HEART_SELECT: &str = "cursors/Blackjack HEART Select.png";
/// Diamond
pub static BJ_DIAMOND_SELECT: &str = "cursors/Blackjack Diamond Select.png";
/// Card front
pub static BJ_CARD_FRONT: &str = "cursors/Blackjack Card Front.png";
/// Card back
pub static BJ_CARD_BACK: &str = "cursors/Blackjack Card Back.png";

/// Card flips over
pub static BJ_CARD_ALT_PAGE: &str = "Blackjack Card Alternate_page";
/// Card flat spin
pub static BJ_CARD_BUSY_PAGE: &str = "Blackjack Card Busy_page";
/// Card flips over
pub static BJ_CARD_BUSY_2_PAGE: &str = "Blackjack Card Busy 2_page";
/// Spade morphs into straight line
pub static BJ_HANDWRITING_PAGE: &str = "Blackjack Handwriting_page";
/// Rotating heart
pub static BJ_HEART_BUSY_PAGE: &str = "Blackjack Heart Busy_page";
/// Spade with red and white flashing
pub static BJ_HELP_SELECT_PAGE: &str = "Blackjack Help Select_page";
/// Bleeding spade
pub static BJ_HELP_SELECT_2_PAGE: &str = "Blackjack Help Select 2_page";
/// Spade with little rotating heart
pub static BJ_WORKING_IN_BG_PAGE: &str = "Blackjack Working In Background_page";
/// Spade morphs into little spade with parallel lines
pub static BJ_PREC_SELECT_PAGE: &str = "Blackjack Precision Select_page";
/// Spade morphs into standard text-select cursor
pub static BJ_TEXT_SELECT_PAGE: &str = "Blackjack Text Select_page";