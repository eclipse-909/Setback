use {
	setback::*,
	std::{
		env,
		io::Write,
		process::exit,
		sync::Arc,
		time::Instant,
	},
	sqlx::{Executor, postgres::PgPoolOptions},
	tokio::net::TcpListener,
	tracing::{error, info, warn},
};

fn main() {
	init_tracing!();

	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();
	
	rt.block_on(async move {
		if env::args().any(|a| a == "--health") {
			let result = tokio::time::timeout(std::time::Duration::from_secs(3), async {
				let mut ws = ws_local_connect().await.ok()?;
				ws.close(1000, "healthcheck").await.ok()?;
				Some(())
			}).await;
			
			let code = if matches!(result, Ok(Some(()))) { 0 } else { 1 };
			exit(code);
		}
		
		let db = PgPoolOptions::new()
			.max_connections(DB_MAX_CONNECTIONS)
			.after_connect(|conn, _meta| Box::pin(async move {
				conn.execute("set time_zone = '+00:00';").await?;
				Ok(())
			}))
			.connect(env::var(ENV_DB_URL).unwrap().as_str())
			.await
			.unwrap();
		
		let state = State::new(db);
		let api = Api::new();
		
		tokio::select! {
			res = tokio::signal::ctrl_c() => if let Err(e) = res {
				error!("Terminating via SIGINT - {e}")
			} else {
				info!("Terminating via SIGINT")
			},
			_ = tcp_listen(state, api) => unreachable!(),
		}
	});

	info!("Initiating shutdown");
	let timer = Instant::now();
	rt.shutdown_timeout(SHUTDOWN_TIMEOUT);
	info!("Shutdown complete after {} ms", timer.elapsed().as_millis());
	_ = std::io::stdout().flush();
}

async fn tcp_listen(state: Arc<State>, api: Api) -> ! {
	let addr = env::var(ENV_SERVER_BIND_ADDR).unwrap();
	let tcp_listener = TcpListener::bind(addr)
		.await
		.unwrap();
	loop {
		match tcp_listener.accept().await {
			Ok((stream, _)) => ClientConnection::try_accept(stream, state.clone(), &api).await,
			Err(e) => warn!("Could not accept client. Error: {e}"),
		}
	}
}