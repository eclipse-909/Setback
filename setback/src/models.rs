use {
	bevy::prelude::*,
	serde::{Serialize, Deserialize}
};

/// This struct is the color pallet for the UI. It is intended to be used as a singleton, so don't create another pallet.
pub struct Pallet {
	pub primary_green: Color,
	pub secondary_green: Color,
	pub accent_gold: Color,
	pub light_gold: Color,
	pub dark_gold: Color,
	pub cream: Color,
	pub dark_brown: Color,
	pub medium_brown: Color,
	pub light_brown: Color,
	pub card_shadow: Color,
	pub text_shadow: Color,
	pub url: Color,
	pub border_radius: Val
}

#[derive(Deserialize)]
pub struct Frame {
    pub filename: String,
    pub frame: RectData,
    // other fields like rotated, trimmed, etc. are ignored for now
}

#[derive(Deserialize)]
pub struct RectData {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Deserialize)]
pub struct Meta {
    pub size: AtlasSize,
}

#[derive(Deserialize)]
pub struct AtlasSize {
    pub w: u32,
    pub h: u32,
}

#[derive(Deserialize)]
pub struct TextureAtlasJson {
    pub frames: Vec<Frame>,
    pub meta: Meta,
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
	//video
	pub vsync: bool,
	pub max_frame_rate: f64,
	pub fullscreen: bool,
	pub default_monitor: MonitorSelection,
	pub monitor_index: usize,
	
	//game settings
	pub fn game_settings: GameSettings,
	
	//account
	pub save_login_credentials: bool
}

impl Default for Settings {
	fn default() -> Self {Self {
		//video
		vsync: false,
		max_frame_rate: 60.,
		fullscreen: true,
		default_monitor: MonitorSelection::Primary,
		monitor_index: 0,
		
		//game settings
		game_settings: GameSettings::default(),
		
		//account
		save_login_credentials: true
	}}
}