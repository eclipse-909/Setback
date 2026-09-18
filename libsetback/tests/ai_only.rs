use {setback::*, std::fs};

#[cfg(not(feature = "server"))]
#[tokio::test]
async fn test_ai_only() {
	let path = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/logs/ai_only.log"));
	fs::create_dir_all(path.parent().unwrap()).unwrap();
	let log = fs::File::create(path).unwrap();
	tracing_subscriber::fmt()
		.with_writer(log)
		.with_ansi(false)
		.with_level(true)
		.compact()
		.init();

	Lobby::create_ai_only()
		.finish()
		.expect("Game cannot start in invalid state")
		.run_singleplayer(Empty, Empty)
		.await;
}

struct Empty;

impl EventEmitter for Empty {
	fn send(&self, _event: GameEvent) -> Result<(), Error> {
		Ok(())
	}
}
impl EventResponseReceiver for Empty {
	async fn recv(&mut self) -> Result<GameEvent, Error> {
		Err(Error::LogicError)
	}
}
