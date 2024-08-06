mod ui;

use {
	crate::ui::ScenePlugin,
	bevy::{
		prelude::*,
		window::{PresentMode, WindowMode, EnabledButtons, WindowTheme},
		winit::WinitSettings
	},
	bevy_inspector_egui::quick::WorldInspectorPlugin,
	sickle_ui::SickleUiPlugin
};

fn main() {
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				present_mode: PresentMode::AutoVsync,
				mode: WindowMode::BorderlessFullscreen,
				resolution: (1920., 1080.).into(),
				title: "Setback".into(),
				name: Some("setback.app".into()),
				resizable: false,
				enabled_buttons: EnabledButtons {
					minimize: false,
					maximize: false,
					close: false
				},
				decorations: false,
				window_theme: Some(WindowTheme::Dark),
				..default()
			}),
			..default()
		}))
		.add_plugins(SickleUiPlugin)
		.add_plugins(WorldInspectorPlugin::default())//TODO remember to comment this line for release builds
		.add_plugins(ScenePlugin)
		.insert_resource(WinitSettings::game())
		.init_state::<SceneType>()
		.init_state::<HostType>()
		.add_systems(Startup, setup)
		.run();
}

fn setup(mut commands: Commands) {commands.spawn(Camera2dBundle::default());}

///Game state which stores what scene is currently active.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum SceneType {
	#[default]
	MainMenu,
	Setup,
	Multiplayer,
	Game,
	Settings
}

///Game state which stores the relation between the player and the type of game they're playing.
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
enum HostType {
	///Not in a game.
	#[default]
	None,
	Singleplayer,
	///Created a multiplayer game as the host.
	Host,
	///Joined a multiplayer game as a client.
	Client
}