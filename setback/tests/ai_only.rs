#![feature(panic_update_hook)]

use {
	std::{
		fs::{self, File},
		path::Path,
		io::Write,
		backtrace::Backtrace,
	},
	setback::*,
	tracing_subscriber::EnvFilter,
};

#[tokio::test]
async fn test_ai_only() {
	let path = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/logs/ai_only.log"));
	fs::create_dir_all(path.parent().unwrap()).unwrap();
	let log = File::create(path).unwrap();
	tracing_subscriber::fmt()
		.with_writer(log)
		.with_ansi(false)
		.with_level(true)
		.with_env_filter(EnvFilter::from_default_env())
		.compact()
		.init();
	
	std::panic::update_hook(move |prev, info| {
		let log_path = Path::new(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/logs/ai_only-backtrace.log"
		));
		fs::create_dir_all(log_path.parent().unwrap()).unwrap();
		let mut log_file = File::create(log_path).unwrap();
		_ = writeln!(log_file, "{info}");
		_ = writeln!(log_file, "{}", Backtrace::capture());
		prev(info);
	});

	Lobby::create_ai_only().finish()
		.run_ai_only()
		.await;
}