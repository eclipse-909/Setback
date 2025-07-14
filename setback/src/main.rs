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
		winit::{WinitSettings, WinitWindows},
		log::{LogPlugin, BoxedLayer, tracing_subscriber::Layer}
	},
	winit::window::Icon,
	keyring::{set_default_credential_builder, mock::default_credential_builder},
	std::sync::OnceLock,
	tracing_appender::{rolling, non_blocking::WorkerGuard}
};

fn main() {
	if cfg!(debug_assertions) {
		set_default_credential_builder(default_credential_builder());
	}
	//TODO get settings here
	App::new()
		.add_plugins(
			DefaultPlugins.set(WindowPlugin {
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
			}).set(LogPlugin {
				custom_layer: log_layer,
				..default()
			})
		)
		.add_plugins(CursorIconPlugin)
		.add_plugins(AsyncTaskPlugin)
		// .add_plugins(DebugPlugin)
		.insert_resource(WinitSettings::game())
		.add_systems(Startup, startup)
		.add_plugins(bevy_framepace::FramepacePlugin)
		.add_plugins(MainMenuPlugin)
		.init_state::<UiScene>()
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

static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

fn log_layer(_app: &mut App) -> Option<BoxedLayer> {
	let file_appender = rolling::daily("logs", "app.log");
	let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
	let _ = LOG_GUARD.set(guard);
	Some(bevy::log::tracing_subscriber::fmt::layer()
		.with_writer(non_blocking)
		.with_file(true)
		.with_line_number(true)
		.boxed())
}