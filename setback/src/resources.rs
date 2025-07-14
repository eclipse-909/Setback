use {
	bevy::{
		prelude::*,
		tasks::Task,
		ecs::world::CommandQueue
	},
	futures::stream::FuturesUnordered,
	libsetback::*,
	std::{
		collections::HashMap,
		sync::{Arc, RwLock}
	},
	reqwest::Client
};

#[derive(Resource)]
pub struct ServerConn(pub Arc<RwLock<ServerConnection>>);

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

#[derive(Resource, Default)]
pub struct TaskSchedule(pub FuturesUnordered<Task<CommandQueue>>);