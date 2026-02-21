mod constants;

use {
	crate::{
		constants::*,
		state::*,
	},
	libsetback::{*, sdp::*, stream::MemPool},
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
	tracing::{error, info}
};

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

	let sigterm = signal(SignalKind::terminate())
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
				_ = listen_unix_socket(sock, port) => unsafe {std::hint::unreachable_unchecked()},
				_ = cancel_token.cancelled() => {}
			}
		});

		tokio::select! {
			_ = listen_tcp_socket(tcp_stream, api, state.clone()) => unsafe {std::hint::unreachable_unchecked()},
			_ = master_cancel_token.cancelled() => {}
		}
	});

	todo!("Find out how to block on all running tasks");
	info!("setback_server-{} shutdown", BIN_VERION);
	std::io::stdout().flush();
}

async fn listen_unix_socket(sock: UnixDatagram, port: u16) -> ! {
	let buf = [0u8; codec::MAX_LEN].as_mut_slice();
	if let Ok(buf) = Sdp::new(Payload::NotifyStart(StartInfo {
		port,
		version: BIN_VERION,
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

async fn listen_tcp_socket(stream: TcpListener, api: Client, state: Arc<State>) -> ! {
	let mem_pool: FreeList::new();
	loop {
		match stream.accept().await {
			Ok((stream, addr)) => {
				// Under normal usage, the client will have authenticated with setbackd prior to connecting here.
				// Since this port is open and accepting connections, we must assume that anyone can directly connect here.
				tokio::spawn(async {
					let client = match ClientConnection::new(stream, api, addr, mem_pool.clone(), state.clone()).await {
						Ok(c) => c,
						Err(e) => {
							info!("Client failed to connect. Error: {e}");
							return;
						}
					};
					match state.client_store.entry_async(client.account_id).await {
						Entry::Vacant(e) => {_ = e.insert_entry(client);}
						Entry::Occupied(e) => {
							client.send_err(Error::AlreadyConnected).await;
							todo!("send Error::AlreadyConnected message to client and disconnect");
						}
					}
				});
			}
			Err(e) => warn!("Could not accept client. Error: {e}")
		}
	}
}

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