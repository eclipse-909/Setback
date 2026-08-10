use {
	reqwest::Client,
	std::{
		io::Write,
		net::{Ipv4Addr, SocketAddrV4},
		process::exit,
		sync::Arc,
	},
	tokio::net::TcpListener,
	tracing::{error, info, warn},
};

pub use libsetback::*;

const BIN_VERSION: Version = pkg_version!();

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
	// Setup logger, runtime, SIGTERM handler, API connection, and unix socket to setbackd
	tracing_subscriber::fmt()
		.with_writer(std::io::stdout)
		.with_level(true)
		.compact()
		.init();

	let rt = tokio::runtime::Builder::new_current_thread()
		.enable_all()
		.build()
		.unwrap();

	let api = setback_api().unwrap_or_else(|| crash!("Could not connect to setback public api."));

	// Create tcp listener, and setup communication loops for unix socket and tcp listener
	rt.block_on(async move {
		let tcp_stream = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
			.await
			.unwrap_or_else(crash!("Could not bind tcp listener. Error: {}", e));
		let port = tcp_stream
			.local_addr()
			.unwrap_or_else(crash!("Could not get tcp listener port. Error: {}", e))
			.port();

		let state = State::new();
		listen_tcp_socket(tcp_stream, api, state.clone()).await;
	});

	todo!("Find out how to block on all running tasks");
	info!("setback_server-{} shutdown", BIN_VERSION);
	_ = std::io::stdout().flush();
}

async fn listen_tcp_socket(listener: TcpListener, api: Client, state: Arc<State>) -> ! {
	loop {
		match listener.accept().await {
			Ok((stream, addr)) => {
				// Under normal usage, the client will have authenticated with setbackd prior to connecting here.
				// Since this port is open and accepting connections, we must assume that anyone can directly connect here.
				if let Some(client) = ClientConnection::new(stream, api.clone(), addr).await {
					client.spawn_read_loop_for_server(state.clone());
				}
			}
			Err(e) => warn!("Could not accept client. Error: {e}"),
		}
	}
}
