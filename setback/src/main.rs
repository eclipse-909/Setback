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
	libsetback::{Arena, LOG_DIR},
	std::{
		collections::HashSet,
		fs::{self, File},
		io::{BufWriter, Write},
		path::Path,
		sync::{Arc, OnceLock},
		time::{Duration, Instant},
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
