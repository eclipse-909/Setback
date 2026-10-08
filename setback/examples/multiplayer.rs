#![feature(panic_update_hook)]

use {
	arrayvec::ArrayVec,
	setback::*,
	std::{
		backtrace::Backtrace,
		io::Write,
		fs::{self, File},
		path::Path
	},
	tracing_subscriber::EnvFilter,
	tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};

#[tokio::main]
async fn main() {
	let log_path = Path::new(concat!(
		env!("CARGO_MANIFEST_DIR"),
		"/logs/multiplayer.log"
	));
	fs::create_dir_all(log_path.parent().unwrap()).unwrap();
	let log_file = File::create(log_path).unwrap();
	tracing_subscriber::fmt()
		.with_writer(log_file)
		.with_ansi(false)
		.with_level(true)
		.with_env_filter(EnvFilter::from_default_env())
		.compact()
		.init();
	
	std::panic::update_hook(move |prev, info| {
		let log_path = Path::new(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/logs/multiplayer-backtrace.log"
		));
		fs::create_dir_all(log_path.parent().unwrap()).unwrap();
		let mut log_file = File::create(log_path).unwrap();
		_ = write!(log_file, "{info}");
		_ = writeln!(log_file, "{}", Backtrace::capture());
		prev(info);
	});
	
	todo!("initialize game client and mimic the terminal game logic from the singleplayer example");
}

fn readln(buf: &mut String) {
	buf.clear();
	_ = std::io::stdin()
		.read_line(buf)
		.expect("Failed to read line from stdin");
	buf.trim().to_string().clone_into(buf);
}