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
mod unit_tests;

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
		log::{LogPlugin, BoxedLayer, tracing_subscriber::{self, Layer}}
	},
	winit::window::Icon,
	keyring::{set_default_credential_builder, mock::default_credential_builder},
	std::{
		sync::OnceLock,
		fs::{self, File},
		path::Path,
		io::Write
	},
	tracing_appender::non_blocking::WorkerGuard
};

fn main() {
	std::panic::set_hook(Box::new(|panic_info| {
		let mut file: File = File::create(CRASH_LOG).expect("Could not open crash log file");
		writeln!(file, "{panic_info}").expect("Could not write to crash log");
		let backtrace = std::backtrace::Backtrace::capture();
		writeln!(file, "stack backtrace:\n{backtrace}").expect("Could not write backtrace");
		writeln!(file, "Process finished with exit code 101").expect("Could not write exit code");
	}));
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
		.add_plugins(TextInputWrapperPlugin)
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
	let path = Path::new(LATEST_LOG);
	fs::create_dir_all(path.parent().unwrap()).unwrap();
	let (non_blocking, guard) = tracing_appender::non_blocking(File::create(path).unwrap());
	let _ = LOG_GUARD.set(guard);
	Some(tracing_subscriber::fmt::layer()
		.with_writer(non_blocking)
		.with_file(true)
		.with_line_number(true)
		.with_timer(tracing_subscriber::fmt::time::SystemTime)
		.boxed())
}