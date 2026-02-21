#![feature(allocator_api)]

mod connection;
mod constants;
mod game;
mod main_menu;
mod multiplayer;
mod settings;
mod setup;
mod state;
mod ui;

#[cfg(test)]
mod tests;

use {
	crate::{constants::*, settings::*, state::*, ui::*},
	gpui::{
		App, Application, Bounds, Context, SharedString, TitlebarOptions, Window, WindowBounds,
		WindowOptions, div, prelude::*, px, rgb, size,
	},
	libsetback::{Arena, CONFIG_DIR, DATA_DIR, LOG_DIR},
	std::{
		collections::HashSet,
		fs::{self, File},
		io::{BufWriter, Write},
		path::{Path, PathBuf},
		sync::{Arc, OnceLock},
		time::{Duration, Instant},
		env,
	},
	tokio::runtime::Builder,
	tracing_appender::{
		non_blocking::{NonBlocking, WorkerGuard},
		rolling,
	},
	tracing_subscriber::{
		EnvFilter, Layer, Registry,
		filter::Filtered,
		fmt::{
			self,
			format::{Format, Pretty},
		},
		layer::SubscriberExt,
		util::SubscriberInitExt,
	},
	log::{error, info, debug},
};

fn main() {
	/* Use per-platform FHS for all paths
	 * Linux:
	 * - binaries: /usr/local/bin/
	 * - settings: $XDG_CONFIG_HOME/setback/
	 * - data:     $XDG_DATA_HOME/setback/
	 * - logs:     $XDG_STATE_HOME/log/setback/
	 *
	 * Windows:
	 * - binaries: C:\Program Files\setback\
	 * - settings: %LOCALAPPDATA%\setback\config\
	 * - data:     %LOCALAPPDATA%\setback\data\
	 * - logs:     %LOCALAPPDATA%\setback\logs\
	 *
	 * MacOS:
	 * - binaries: /Applications/
	 * - settings: $HOME/Library/Preferences/setback/
	 * - data:     $HOME/Library/Application Support/setback/
	 * - logs:     $HOME/Library/Logs/setback/
	*/
	unsafe {
		if cfg!(target_os = "linux") {
			CONFIG_DIR = [env::var("XDG_CONFIG_HOME").unwrap().as_str(), "setback"].iter().collect();
			DATA_DIR = [env::var("XDG_DATA_HOME").unwrap().as_str(), "setback"].iter().collect();
			LOG_DIR = [env::var("XDG_STATE_HOME").unwrap().as_str(), "log/setback"].iter().collect();
		} else if cfg!(target_os = "windows") {
			let localappdata = env::var("LOCALAPPDATA").unwrap();
			CONFIG_DIR = [localappdata.as_str(), "setback/config"].iter().collect();
			DATA_DIR = [localappdata.as_str(), "setback/data"].iter().collect();
			LOG_DIR = [localappdata.as_str(), "setback/logs"].iter().collect();
		} else if cfg!(target_os = "macos") {
			let home = env::var("HOME").unwrap();
			CONFIG_DIR = [home.as_str(), "Library/Preferences/setback"].iter().collect();
			DATA_DIR = [home.as_str(), "Library/Application Support/setback"].iter().collect();
			LOG_DIR = [home.as_str(), "Library/Logs/setback"].iter().collect();
		} else {panic!("OS not supported")}
		fs::create_dir_all(CONFIG_DIR).unwrap();
		fs::create_dir_all(DATA_DIR).unwrap();
		fs::create_dir_all(LOG_DIR).unwrap();
	}

	// check for uninstall
	let mut args = std::env::args();
	_ = args.next();
	if let Some(arg1) = args.next() && arg1 == "--uninstall" {
		todo!("Uninstall setback from system");
	}

	// set up crash log
	unsafe {
		std::env::set_var("RUST_BACKTRACE", "full");
	}
	std::panic::set_hook(Box::new(|panic_info| {
		let panic_msg = format!(
			"Panic - see crash.log\nTime: {}\n{panic_info}\nBacktrace: {}\n",
			chrono::Local::now(),
			std::backtrace::Backtrace::capture(),
		);
		error!(panic_msg);
		eprintln!("{}", panic_msg);

		let path = Path::new(CRASH_LOG);
		const FS_ERR: &str = "Could create crash log";
		fs::create_dir_all(path.parent().expect(FS_ERR)).expect(FS_ERR);
		let file: File = File::create(path).expect("Could not create crash log file");
		const WRITE_ERR: &str = "Could not write to crash log";
		let mut writer = BufWriter::new(file);
		write!(writer, "{}", panic_msg).expect(WRITE_ERR);
		writer.flush().expect(WRITE_ERR);
	}));

	// load settings
	let settings = load_settings();

	// init logger
	_ = fs::remove_file(Path::new(SETBACK_PATH));
	_ = fs::remove_file(Path::new(DEP_PATH));
	let (latest_writer, latest_guard) = tracing_appender::non_blocking(rolling::never(LOG_DIR, SETBACK_LOG));
	let (dep_writer, dep_guard) = tracing_appender::non_blocking(rolling::never(LOG_DIR, DEP_LOG));
	Box::leak(Box::new((latest_guard, dep_guard)));

	fn log_layer(writer: NonBlocking, filter: EnvFilter) -> impl Layer<Registry> {
		fmt::layer()
			.with_timer(fmt::time::SystemTime)
			.with_ansi(false)
			.with_target(true)
			.with_file(true)
			.with_line_number(true)
			.with_level(true)
			.with_thread_names(true)
			.with_thread_ids(true)
			.pretty()
			.with_writer(writer)
			.with_filter(filter)
	}

	tracing_subscriber::registry()
		.with(log_layer(
			latest_writer,
			EnvFilter::builder()
				.with_default_directive(tracing::Level::from(settings.setback_log_level).into())
				.parse("setback,libsetback")
				.unwrap(),
		))
		.with(log_layer(
			dep_writer,
			EnvFilter::builder()
				//TODO figure out why filter isn't working
				.with_default_directive(tracing::Level::from(settings.dep_log_level).into())
				.parse("gpui")
				.unwrap(),
		))
		.init();

	info!("Initialized logger");
	debug!("{:#?}", settings);

	// sets mock keyring for debug builds
	#[cfg(debug_assertions)] {
		keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
		info!("Set mock keyring credential builder for development build");
	}
	#[cfg(not(debug_assertions))] {
		keyring::set_default_credential_builder(keyring::default::default_credential_builder());
		info!("Set persistent keyring credential builder for release build");
	}

	// create application
	Application::new().run(|app: &mut App| {
		let bounds = Bounds::maximized(None, app);
		app.set_global(Global::default());
		app.open_window(
			WindowOptions {
				window_bounds: Some(if settings.fullscreen {
					WindowBounds::Fullscreen(bounds)
				} else {
					WindowBounds::Maximized(bounds)
				}),
				titlebar: Some(TitlebarOptions {
					title: Some(SharedString::from("Setback")),
					..Default::default()
				}),
				..Default::default()
			},
			|_window, app| app.new(|_cx| RootUi::default()),
		)
		.unwrap();
		app.activate(true);
	})
}
