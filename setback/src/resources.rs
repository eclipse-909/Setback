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
	}
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