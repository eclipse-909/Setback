use std::fs;

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
	
	libsetback::GameSetup::new_ai_only()
		.init()
		.expect("Game cannot start in invalid state")
		.run_singleplayer().await;
}