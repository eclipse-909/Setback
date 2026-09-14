use {
	std::{
		cell::LazyCell,
		io::Write,
		net::{Ipv4Addr, SocketAddrV4},
		process::exit,
		time::Instant,
		sync::Arc
	},
	reqwest::Client,
	tokio::net::TcpListener,
	tracing::{error, info, warn},
};

pub use setback::*;

const BIN_VERSION: Version = pkg_version!();
extern "Rust" static SETBACK_API: LazyCell<Client> = LazyCell::new(|| Client::builder().cookie_store(true).build().unwrap());

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
	tracing_subscriber::fmt()
		.with_writer(std::io::stdout)
		.with_level(true)
		.compact()
		.init();

	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.build()
		.unwrap();
	
	let (state, reject_rx) = State::new();
	
	rt.block_on(async move {
		tokio::select! {
			res = tokio::signal::ctrl_c() => if let Err(e) = res {
				error!("Terminating via SIGINT - {e}")
			} else {
				info!("Terminating via SIGINT")
			},
			_ = tcp_listen(state) => unreachable!(),
			_ = reject_listen(reject_rx, state) => unreachable!(),
		}
	});
	info!("Initiating shutdown");
	let timer = Instant::now();
	rt.shutdown_timeout(SHUTDOWN_TIMEOUT);
	info!("Shutdown complete after {} ms", timer.elapsed().as_millis());
	_ = std::io::stdout().flush();
}

async fn tcp_listen(state: Arc<State>) -> ! {
	let tcp_listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
		.await
		.unwrap_or_else(crash!("Could not bind tcp listener. Error: {}", e));
	loop {
		match tcp_listener.accept().await {
			Ok((stream, addr)) => {
				// ClientConnection::new will upgrade to ws and authenticate the client
				if let Some(client) = ClientConnection::new(stream, addr).await {
					client.spawn_server_handler(state.clone());
				}
			}
			Err(e) => warn!("Could not accept client. Error: {e}"),
		}
	}
}

/// This happens when a client tries to rejoin an ongoing game but fails,
/// so they are rejected from that game and their handler needs to be restarted.
async fn reject_listen(mut reject_rx: GspRx, state: Arc<State>) -> ! {
	loop {
		reject_rx.recv().await.unwrap().spawn_handler(state.clone());
	}
}