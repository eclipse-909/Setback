mod main_menu;
mod settings;
mod setup;
mod multiplayer;
mod game;
mod components;
mod constants;
mod plugins;
mod resources;
mod states;
mod models;

use {
	crate::{
		constants::*,
		plugins::*,
		states::*,
		main_menu::MainMenuPlugin
	},
	bevy::{
		prelude::*,
		window::{EnabledButtons, PresentMode, WindowMode, WindowTheme},
		winit::{WinitSettings, WinitWindows}
	},
	winit::window::Icon,
	keyring::{set_default_credential_builder, mock::default_credential_builder}
};

fn main() {
	if cfg!(debug_assertions) {
		set_default_credential_builder(default_credential_builder());
	}
	//TODO get settings here
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				present_mode: PresentMode::AutoNoVsync,
				mode: if cfg!(debug_assertions) {//TODO these should be options in the settings and loaded before the app starts
					WindowMode::Windowed
				} else {
					WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current)
				},
				title: "Setback".into(),
				name: Some("Setback.app".into()),
				resizable: cfg!(debug_assertions),
				enabled_buttons: EnabledButtons {
					minimize: cfg!(debug_assertions),
					maximize: cfg!(debug_assertions),
					close: cfg!(debug_assertions)
				},
				decorations: cfg!(debug_assertions),
				window_theme: Some(WindowTheme::Dark),
				..default()
			}),
			..default()
		}))
		.add_plugins(CursorIconPlugin)
		// .add_plugins(DebugPlugin)
		.insert_resource(WinitSettings::game())
		.add_systems(Startup, startup)
		.add_plugins(bevy_framepace::FramepacePlugin)
		.add_plugins(MainMenuPlugin)
		.init_state::<Scene>()
		.init_state::<ConnectionType>()
		.run();
}

fn startup(
	mut commands: Commands,
	windows: NonSend<WinitWindows>,
	mut framepace_settings: ResMut<bevy_framepace::FramepaceSettings>
) {
	// Set window icon
	let image = image::load_from_memory_with_format(CARDS_ICON, image::ImageFormat::Png).unwrap().into_rgba8();
	let (width, height) = image.dimensions();
	let rgba = image.into_raw();
	let icon = Icon::from_rgba(rgba, width, height).unwrap();
	for window in windows.windows.values() {
		window.set_window_icon(Some(icon.clone()));
	}

	// Spawn camera
	commands.spawn(Camera2d);

	// Set max framerate
	framepace_settings.limiter = bevy_framepace::Limiter::from_framerate(60.0);
}