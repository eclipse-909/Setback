use {libsetback::*, std::fs};

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

	GameSetup::new_ai_only()
		.init()
		.expect("Game cannot start in invalid state")
		.run_singleplayer(Empty, Empty)
		.await;
}

struct Empty;

impl EventEmitter for Empty {
	fn send(&self, _event: GameEvent) -> Result<(), Error> {
		Err(Error::LogicError)
	}
}
impl EventResponseReceiver for Empty {
	async fn recv(&mut self) -> Result<GameEvent, Error> {
		Err(Error::LogicError)
	}
}
