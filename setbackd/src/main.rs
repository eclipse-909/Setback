use {
	libsetback::{ClientConnection, SETBACK_UNIX_SOCKET, SETBACKD_PORT, setback_api},
	std::{
		io::{ErrorKind, Write},
		net::{Ipv4Addr, SocketAddrV4},
		process::exit,
		sync::{Arc, atomic::{AtomicU16, Ordering}},
		time::Duration
	},
	systemd::daemon,
	tokio::{
		net::{TcpListener, UnixDatagram, unix::SocketAddr},
		signal::unix::{SignalKind, signal},
	},
	tokio_util::sync::CancellationToken,
	tracing::{error, info, warn}
};

unsafe extern "C" {
	// DO NOT CHANGE NAME
	#[unsafe(no_mangle)]
	static mut environ: *mut *mut std::ffi::c_char;
}

fn main() {
	// Setup logging, tokio runtime, SIGTERM handler, Unix socket from systemd, public API connection, and TCP listener.
	// Errors at this point are considered not recoverable because all these things are necessary.
	tracing_subscriber::fmt()
		.with_writer(std::io::stdout)
		.with_ansi(false)
		.with_level(true)
		.compact()
		.init();

	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.max_blocking_threads(8) // not sure how many blocking threads are used interally by tokio
		.worker_threads(1) // can increase if bottlenecked
		.build()
		.unwrap_or_else(crash!("Could not create async runtime. Error: {}", e));
	let _rt_guard = rt.enter();

	// Not sure if it's necessary to "convert" the signal into a cancel token,
	// but usage for Signal is a little weird so this is just easier to understand.
	let cancel_token = CancellationToken::new();
	let cancel_token1 = cancel_token.clone();
	rt.spawn(async move {
		signal(SignalKind::terminate())
			.unwrap_or_else(crash!("Could not create SIGTERM handler. Error: {}", e))
			.recv()
			.await;
		cancel_token1.cancel();
	});

	let sock = UnixDatagram::bind(SETBACK_UNIX_SOCKET)
		.unwrap_or_else(crash!("Could not bind unix socket. Error: {}", e));

	// TODO: it would be ideal to connect directly to the db rather than making http requests
	let api = setback_api()
		.unwrap_or_else(crash!("Could not connect to setback public api."));
	let tcp_stream = rt.block_on(async move {
		TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, SETBACKD_PORT))
			.await
			.unwrap_or_else(crash!("Could not bind tcp listener. Error: {}", e))
	});

	// After all that is setup, then we can fork-exec the server.
	// The server should immediately connect to the unix socket,
	// then it will bind a tcp listener, then send the bound port back here.
	let latest_server_tcp_port = Arc::new(AtomicU16::new(0));
	let latest_pid = spawn_server();
	if latest_pid == 0 {
		crash!("Could not spawn initial setback_server.");
	}
	if !daemon::notify(false, [(daemon::STATE_READY, "1")].iter()).unwrap_or(false) {
		crash!("Could not notify systemd of ready state.");
	}
	info!("setbackd started successfully");

	// It's all I/O operations from here with mostly recoverable errors.
	rt.block_on(async move {
		let cancel_token1 = cancel_token.clone();
		tokio::spawn(async {
			tokio::select! {
				_ = listen_tcp_socket(stream, api, latest_server_tcp_port.clone()) => {}
				_ = cancel_token1.cancelled() => {}
			}
		});
		let cancel_token1 = cancel_token.clone();
		let buf = todo!("create buffer of what size? Or make resizeable? Use tokio Buffer?");
		loop {
			tokio::select! {
				msg = sock.recv_from(buf) => {tokio::spawn(handle_unix_socket_msg(&sock, msg, &cancel_token1, buf));}
				_ = cancel_token.cancelled() => return
			}
		}
	});

	// background tasks should stop on their own quickly
	rt.shutdown_timeout(Duration::from_secs(32));
	sock.shutdown(std::net::Shutdown::Both);
	std::fs::remove_file(SETBACK_UNIX_SOCKET);
	info!("setbackd shutdown");
	std::io::stdout().flush();
}

/// Returns the PID of the spawned server, or 0 on failure.
fn spawn_server() -> i32 {
	match unsafe {libc::fork()} {
		0 => {
			// we are the child
			unsafe {libc::execve(
				c"/usr/bin/setback_server".as_ptr(),
				[c"setback_server".as_ptr(), std::ptr::null()].as_ptr(),
				environ as _
			)};
			// if execve was successful, the current process stops here
			// if execve failed, we are the current process still
			error!(
				"setback_server not spawned. execve failed. errno: {}",
				std::io::Error::last_os_error().raw_os_error().unwrap()
			);
			0
		}
		-1 => {
			// we are the parent and no child was spawned - check errno
			error!(
				"setback_server not spawned. fork failed. errno: {}",
				std::io::Error::last_os_error().raw_os_error().unwrap()
			);
			0
		}
		child_pid => {
			// we are the parent
			info!("setback_server spawned with PID: {child_pid}");
			child_pid
		}
	}
}

async fn handle_unix_socket_msg(
	sock: &UnixDatagram,
	msg: std::io::Result<(usize, SocketAddr)>,
	cancel_token: &CancellationToken,
	buf: &mut [u8]
) {
	let (buf_len, addr) = match msg {
		Ok((buf_len, addr)) => (buf_len, addr),
		Err(e) => {
			warn!("Error receiving message from unix socket. Error: {e}");
			return;
		}
	};
	//TODO
	// deserialize buf into Sdp
	// pattern match Sdp
	// if we are sending a response:
	//   construct and serialize the response into a buffer
	//   sock.send_to(res_buf, addr).await;
	todo!("pattern match req and use sock to send response if necessary")
}

async fn listen_tcp_socket(stream: TcpListener, api: Client, server_port: Arc<AtomicU16>) {
	const MAX_RETRIES: u8 = 2;
	let mut tries = 0;
	loop {
		match stream.accept().await {
			Ok((stream, _addr)) => {
				// TODO: we can use addr for security stuff like IP bans

				// the result is only relevant for setback_server
				_ = ClientConnection::authenticate(stream, &api, server_port.load(Ordering::Relaxed)).await;
				tries = 0;
			}
			Err(e) if e.kind() == ErrorKind::WouldBlock => warn!("TCP stream would block accepting a connection. Error: {e}"),
			Err(e) => {
				error!("Could not accept client. Error: {e}");
				if tries == MAX_RETRIES {
					crash!("Max retries reached for accepting a TCP client.");
				}
				tries += 1;
			}
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