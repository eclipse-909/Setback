use {
	bevy::{
		ecs::world::CommandQueue,
		prelude::*
	},
	libsetback::*,
	reqwest::Client,
	std::{
		collections::HashMap,
		sync::Arc
	},
	tokio::{
		runtime::Runtime,
		sync::{
			mpsc::{UnboundedReceiver, UnboundedSender},
			Mutex
		}
	},
	serde::{Deserialize, Serialize}
};

#[derive(Resource)]
pub struct ServerConn(pub Arc<Mutex<ServerConnection>>);

#[derive(Resource)]
pub struct ServerMessage(pub UnboundedReceiver<Result<Ssp, Error>>);

#[derive(Resource)]
pub struct WebsiteConn(pub Client);

#[derive(Resource, Default)]
pub struct CursorAtlas {
    pub texture: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub animations: HashMap<String, Vec<usize>>,
    pub frame_timer: Timer,
    /// Set to empty string if using static png instead of animation
    pub current_animation: &'static str,
    pub current_frame: usize
}

#[derive(Resource)]
pub struct TokioRuntime {
	pub rt: Runtime,
	pub tx: UnboundedSender<CommandQueue>
}

#[derive(Resource)]
pub struct TaskReceiver(pub UnboundedReceiver<CommandQueue>);

#[derive(Resource)]
pub enum ClientUpdate {
	UpToDate,
	UpdateAvailable,
	UpdateRequired
}

#[derive(Resource, Serialize, Deserialize)]
pub struct GameSettings {
	//audio
	pub master_volume: f64,
	pub music_volume: f64,
	pub effects_volume: f64,
	
	//keybinds
	pub bid_0: Vec<KeyCode>,
	pub bid_2: Vec<KeyCode>,
	pub bid_3: Vec<KeyCode>,
	pub bid_4: Vec<KeyCode>,
	pub play_card_1: Vec<KeyCode>,
	pub play_card_2: Vec<KeyCode>,
	pub play_card_3: Vec<KeyCode>,
	pub play_card_4: Vec<KeyCode>,
	pub play_card_5: Vec<KeyCode>,
	pub play_card_6: Vec<KeyCode>,
	pub toggle_settings: Vec<KeyCode>
}

impl Default for GameSettings {
	fn default() -> Self {Self {
		//audio
		master_volume: 100.,
		music_volume: 100.,
		effects_volume: 100.,
		
		//keybinds
		bid_0: vec![KeyCode::Digit0],
		bid_2: vec![KeyCode::Digit2],
		bid_3: vec![KeyCode::Digit3],
		bid_4: vec![KeyCode::Digit4],
		play_card_1: vec![KeyCode::Digit1],
		play_card_2: vec![KeyCode::Digit2],
		play_card_3: vec![KeyCode::Digit3],
		play_card_4: vec![KeyCode::Digit4],
		play_card_5: vec![KeyCode::Digit5],
		play_card_6: vec![KeyCode::Digit6],
		toggle_settings: vec![KeyCode::Escape]
	}}
}