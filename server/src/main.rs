use {
	reqwest::{Client, header::{HeaderMap, HeaderValue}},
	setback::*,
	std::{
		env,
		io::Write,
		mem::MaybeUninit,
		net::{Ipv4Addr, SocketAddrV4},
		process::exit,
		sync::Arc,
		time::Instant,
	},
	tokio::net::TcpListener,
	tracing::{error, info, warn}
};

macro_rules! crash {
	($fmt:expr) => {{
		error!($fmt);
		_ = std::io::stdout().flush();
		exit(1);
	}};
	($fmt:expr, $arg0:ident) => {
		|$arg0| {
			error!($fmt, $arg0);
			_ = std::io::stdout().flush();
			exit(1);
		}
	};
}

fn main() {
	init_tracing!();

	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();
	
	rt.block_on(async move {
		let state = State::new();
		
		// docker compose should already guarantee this, so this is a redundancy check
		state.api.healthcheck()
			.await
			.expect("Setback API healthcheck did not return 200");
		
		tokio::select! {
			res = tokio::signal::ctrl_c() => if let Err(e) = res {
				error!("Terminating via SIGINT - {e}")
			} else {
				info!("Terminating via SIGINT")
			},
			_ = tcp_listen(state) => unreachable!(),
		}
	});
	info!("Initiating shutdown");
	let timer = Instant::now();
	rt.shutdown_timeout(SHUTDOWN_TIMEOUT);
	info!("Shutdown complete after {} ms", timer.elapsed().as_millis());
	_ = std::io::stdout().flush();
}

async fn tcp_listen(state: Arc<State>) -> ! {
	let addr = env::var(ENV_SERVER_BIND_ADDR).unwrap();
	let tcp_listener = TcpListener::bind(addr)
		.await
		.unwrap_or_else(crash!("Could not bind tcp listener. Error: {}", e));
	loop {
		match tcp_listener.accept().await {
			Ok((stream, _)) => ClientConnection::try_accept(stream, state.clone()).await,
			Err(e) => warn!("Could not accept client. Error: {e}"),
		}
	}
}