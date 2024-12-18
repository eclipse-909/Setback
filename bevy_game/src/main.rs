#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;
mod components;

use {
	crate::ui::UIPlugin,
	bevy::{
		prelude::*,
		window::{PresentMode, WindowMode, EnabledButtons, WindowTheme},
		winit::{WinitWindows, WinitSettings}
	},
	bevy_inspector_egui::quick::WorldInspectorPlugin,
	winit::window::Icon
};

fn main() {
	let mut app: &mut App = App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				present_mode: PresentMode::AutoVsync,
				mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
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
		}));
	
	#[cfg(feature = "dev")]
	app.add_plugins(WorldInspectorPlugin::default());
	
	app.add_plugins(UIPlugin)
		.insert_resource(WinitSettings::game())
		.init_state::<SceneType>()
		.init_state::<HostType>()
		.add_systems(Startup, (setup, set_window_icon))
		.run();
}

fn setup(mut commands: Commands) {commands.spawn(Camera2dBundle::default());}

fn set_window_icon(windows: NonSend<WinitWindows>) {
	let (icon_rgba, icon_width, icon_height) = {
		let image = image::open("icon.ico").expect("Failed to open icon path").into_rgba8();
		let (width, height) = image.dimensions();
		let rgba = image.into_raw();
		(rgba, width, height)
	};
	let icon = Icon::from_rgba(icon_rgba, icon_width, icon_height).unwrap();
	for window in windows.windows.values() {
		window.set_window_icon(Some(icon.clone()));
	}
}

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
	///Created a multiplayer bevy_game as the host.
	Host,
	///Joined a multiplayer bevy_game as a client.
	Client
}