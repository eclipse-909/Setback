pub async fn health_check() -> i32 {
	let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
		let mut ws = crate::ws::ws_local_connect().await.ok()?;
		ws.close(1000, "healthcheck").await.ok()?;
		Some(())
	}).await;
	
	if matches!(result, Ok(Some(()))) { 0 } else { 1 }
}