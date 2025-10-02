use {
	crate::keyboard::*,
	serde::{Deserialize, Serialize},
	std::{
		sync::Arc
	},
	vello::{
		util::RenderSurface
	},
	winit::{
		dpi::PhysicalSize,
		keyboard::{KeyCode, NamedKey},
		window::Window
	}
};

#[derive(Debug, Clone, Default)]
pub enum Scene {
	#[default]
	MainMenu,
	Multiplayer,
	Settings,
	Setup,
	Game
}

pub enum ConnState {
	Singleplayer,
	Host,
	Client
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum Level {
	Trace,
	Debug,
	Info,
	Warn,
	Error
}

impl From<tracing::Level> for Level {
	fn from(value: tracing::Level) -> Self {
		match value {
			tracing::Level::TRACE => Self::Trace,
			tracing::Level::DEBUG => Self::Debug,
			tracing::Level::INFO => Self::Info,
			tracing::Level::WARN => Self::Warn,
			tracing::Level::ERROR => Self::Error
		}
	}
}

impl From<Level> for tracing::Level {
	fn from(value: Level) -> Self {
		match value {
			Level::Trace => Self::TRACE,
			Level::Debug => Self::DEBUG,
			Level::Info => Self::INFO,
			Level::Warn => Self::WARN,
			Level::Error => Self::ERROR
		}
	}
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Settings {
	//video
	pub vsync: bool,
	pub max_fps: u32,
	pub antialiasing: bool,
	pub fullscreen: Fullscreen,
	pub monitor: Monitor,
	pub resolution: Option<PhysicalSize<u32>>,
	pub refresh_rate_mhz: Option<u32>,
	pub bit_depth: Option<u16>,
	pub dpi_scale: f64,
	
	//audio
	pub master_volume: f64,
	pub music_volume: f64,
	pub effects_volume: f64,
	
	//keybinds
	pub bid_0: Keybind,
	pub bid_2: Keybind,
	pub bid_3: Keybind,
	pub bid_4: Keybind,
	pub play_card_1: Keybind,
	pub play_card_2: Keybind,
	pub play_card_3: Keybind,
	pub play_card_4: Keybind,
	pub play_card_5: Keybind,
	pub play_card_6: Keybind,
	pub toggle_settings: Keybind,
	
	//account
	pub save_login_credentials: bool,
	
	//hidden
	pub log_level: Level
}

impl Default for Settings {
	fn default() -> Self {Self {
		//video
		vsync: true, //TODO I may need to disable vsync if performance looks bad
		max_fps: 60,
		antialiasing: true,
		fullscreen: if cfg!(debug_assertions) {Fullscreen::Windowed} else {Fullscreen::Borderless},
		monitor: Monitor::Current,
		resolution: None,
		refresh_rate_mhz: None,
		bit_depth: None,
		dpi_scale: 1.,
		
		//audio
		master_volume: 100.,
		music_volume: 100.,
		effects_volume: 100.,
		
		//keybinds
		bid_0: Keybind::character("0", KeyCode::Digit0),
		bid_2: Keybind::character("2", KeyCode::Digit2),
		bid_3: Keybind::character("3", KeyCode::Digit3),
		bid_4: Keybind::character("4", KeyCode::Digit4),
		play_card_1: Keybind::character("1", KeyCode::Digit1),
		play_card_2: Keybind::character("2", KeyCode::Digit2),
		play_card_3: Keybind::character("3", KeyCode::Digit3),
		play_card_4: Keybind::character("4", KeyCode::Digit4),
		play_card_5: Keybind::character("5", KeyCode::Digit5),
		play_card_6: Keybind::character("6", KeyCode::Digit6),
		toggle_settings: Keybind::named(NamedKey::Escape, KeyCode::Escape),
		
		//account
		save_login_credentials: true,
		
		//hidden
		log_level: if cfg!(debug_assertions) {Level::Debug} else {Level::Info}
	}}
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Fullscreen {
	Exclusive,
	Borderless,
	Windowed
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Monitor {
	Current,
	Primary,
	Index(usize)
}

pub enum RenderState {
	/// `RenderSurface` and `Window` for active rendering.
	Active {
		surface: RenderSurface<'static>,
		valid_surface: bool,
		window: Arc<Window>,
	},
	/// Cache a window so that it can be reused when the app is resumed after being suspended.
	Suspended(Option<Arc<Window>>),
}