#![feature(panic_update_hook)]

use {
	std::{
		fs::{self, File},
		path::Path,
	},
	setback::*,
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
		.compact()
		.init();
	
	std::panic::update_hook(move |prev, info| {
		let log_path = Path::new(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/logs/ai_only-backtrace.log"
		));
		fs::create_dir_all(log_path.parent().unwrap()).unwrap();
		let mut log_file = File::create(log_path).unwrap();
		_ = write!(log_file, "{info}");
		prev(info);
	});

	Lobby::create_ai_only()
		.finish()
		.expect("Game cannot start in invalid state")
		.run_singleplayer()
		.await;
}