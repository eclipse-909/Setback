use std::io::Write;
use std::path::Path;
use tokio_util::codec;
use {
	libsetback::{*, sdp::*},
	reqwest::Client,
	scc::{HashMap, hash_map::Entry},
	std::{
		net::{Ipv4Addr, SocketAddrV4, SocketAddr},
		process::exit,
		sync::{atomic::Ordering, Arc},
	},
	tokio::{
		net::{TcpListener, UnixDatagram, TcpStream},
		signal::unix::{SignalKind, signal},
		sync::mpsc::{self, UnboundedSender, UnboundedReceiver},
	},
	tokio_util::sync::CancellationToken,
	tracing::{error, info, warn}
};

const BIN_VERSION: Version = pkg_version!();

macro_rules! crash {
	($fmt:expr) => {{
		error!($fmt);
		std::io::stdout().flush();
		exit(1);
	}};
	($fmt:expr, $arg0:expr) => {
		|$arg0| {
			error!($fmt, $arg0);
			std::io::stdout().flush();
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
	let _rt_guard = rt.enter();

	let mut sigterm = signal(SignalKind::terminate())
		.unwrap_or_else(crash!("Could not create SIGTERM handler. Error: {}", e));
	let master_cancel_token = CancellationToken::new();
	let cancel_token = master_cancel_token.clone();
	rt.spawn(async move {
		sigterm.recv().await;
		cancel_token.cancel();
	});

	let api = setback_api()
		.unwrap_or_else(crash!("Could not connect to setback public api."));

	let sock = UnixDatagram::bind(setback_server_unix_socket(unsafe {libc::getpid()}))
		.unwrap_or_else(crash!("Could not bind unix socket. Error: {}", e));

	// Create tcp listener, and setup communication loops for unix socket and tcp listener
	rt.block_on(async move {
		let tcp_stream = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
			.await
			.unwrap_or_else(crash!("Could not bind tcp listener. Error: {}", e));
		let port = tcp_stream.local_addr()
			.unwrap_or_else(crash!("Could not get tcp listener port. Error: {}", e))
			.port();

		let state = State::new();
		let cancel_token = master_cancel_token.clone();
		tokio::spawn(async move {
			tokio::select! {
				_ = listen_unix_socket(sock, port) => unreachable_unchecked!("Unix socket listener should never return"),
				_ = cancel_token.cancelled() => {}
			}
		});

		tokio::select! {
			_ = listen_tcp_socket(tcp_stream, api, state.clone()) => unreachable_unchecked!("TCP socket listener should never return"),
			_ = master_cancel_token.cancelled() => {}
		}
	});

	todo!("Find out how to block on all running tasks");
	info!("setback_server-{} shutdown", BIN_VERSION);
	std::io::stdout().flush();
}

async fn listen_unix_socket(sock: UnixDatagram, port: u16) -> ! {
	let buf = [0u8; codec::MAX_LEN].as_mut_slice();
	if let Ok(buf) = Sdp::new(Payload::NotifyStart(StartInfo {
		port,
		version: BIN_VERSION,
		addr: setbackd_unix_socket().to_string_lossy().to_string(),
	})).serialize(buf) {
		sock.send_to(buf, setbackd_unix_socket()).await;
	}
	loop {
		let addr = match sock.recv_from(buf).await {
			Ok(r) => {
				if r.1.as_pathname().unwrap_or(Path::new("")) != setbackd_unix_socket() {
					warn!("Only setbackd is allowed to communicate with setback_server over this socket");
					continue;
				}
				r.1
			}
			Err(e) => {
				warn!("Error receiving message from unix socket. Error: {e}");
				continue;
			}
		};
		// Result<received payload to handle, payload to send back immediately>
		let payload = Sdp::deserialize(buf)
			.and_then(|sdp| if !sdp.version.is_compatible(SDP_VERSION) {Err(Error::BadVersion)} else {Ok(sdp.payload)});
		let payload = match payload {
			Ok(p) => {
				match p {
					Payload::Err(err) => {
						todo!("maybe implement retries?");
						continue;
					}
					Payload::PsReq => {
						todo!("send back PsRes")
					}
					_ => Payload::Err(Error::BadProtocol),
				}
			}
			Err(e) => Payload::Err(e),
		};
		if let Ok(buf) = Sdp::new(payload).serialize(buf) {
			sock.send_to(buf, addr).await;
		}
	}
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
			Err(e) => warn!("Could not accept client. Error: {e}")
		}
	}
}