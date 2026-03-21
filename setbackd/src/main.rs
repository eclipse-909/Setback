use {
	reqwest::Client,
	futures::{SinkExt, StreamExt},
	libsetback::{
		ClientConnection, Error,
		SDP_VERSION, SETBACKD_PORT,
		sdp::*, setback_api, SETBACKD_UNIX_SOCKET
	},
	std::{
		collections::HashMap,
		io::{ErrorKind, Write},
		net::{Ipv4Addr, SocketAddrV4},
		process::exit,
		sync::{Arc, atomic::{AtomicU16, Ordering}},
		time::Duration,
		path::Path,
		fs::File,
		ffi::c_char,
	},
	systemd::daemon,
	tokio::{
		net::{TcpListener, UnixDatagram, unix::SocketAddr},
		signal::unix::{SignalKind, signal},
	},
	tracing::{error, info, warn}
};

// DO NOT TOUCH THIS!
unsafe extern "C" {
	static mut environ: *mut *mut c_char;
}

/// File keeps track of all the running servers.
const PIDFILE: &str = "/var/lib/setback/pidfile";
const SETBACK_SERVER_BIN: *const c_char = c"/usr/local/bin/setback_server".as_ptr();

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
	// Setup logging, updater, tokio runtime, SIGTERM handler, Unix socket, public API connection, and TCP listener.
	// Errors at this point are considered not recoverable because all these things are necessary.
	tracing_subscriber::fmt()
		.with_writer(std::io::stdout)
		.with_level(true)
		.compact()
		.init();

	std::thread::spawn(cd_auto_update);

	let rt = tokio::runtime::Builder::new_multi_thread()
		.enable_all()
		.max_blocking_threads(2) // not sure how many blocking threads are used internally by tokio
		.worker_threads(1) // can increase if bottlenecked
		.build()
		.unwrap_or_else(crash!("Could not create async runtime. Error: {}", e));
	let _rt_guard = rt.enter();

	let sigterm0 = signal(SignalKind::terminate())
		.unwrap_or_else(crash!("Could not create SIGTERM handler. Error: {}", e));
	let sigterm1 = signal(SignalKind::terminate())
		.unwrap_or_else(crash!("Could not create SIGTERM handler. Error: {}", e));

	let sock = UnixDatagram::bind(SETBACKD_UNIX_SOCKET)
		.unwrap_or_else(crash!("Could not bind unix socket. Error: {}", e));

	let api = setback_api()
		.unwrap_or_else(crash!("Could not connect to setback public api."));

	rt.block_on(async move {
		let tcp_stream = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, SETBACKD_PORT))
			.await
			.unwrap_or_else(crash!("Could not bind tcp listener. Error: {}", e));

		// After all that is setup, then we can fork-exec the server.
		// The server should almost immediately connect to the unix socket,
		// then it will bind a tcp listener, then send the bound port back here.
		let buf = [0u8; codec::MAX_LEN].as_mut_slice();
		let latest_server_tcp_port = AtomicU16::new(0);
		let mut server_map = std::fs::OpenOptions::new()
			.read(true)
			.open(PIDFILE)
			.inspect_err(|e| error!("Could not open server_pidfile. Error: {e}"))
			.ok()
			.and_then(|file| postcard::from_io((file, buf))
				.inspect_err(|e| error!("Could not deserialize server_pidfile. Error: {e}"))
				.ok()
				.map(|t| t.0)
			)
			.unwrap_or(HashMap::with_capacity(1));
		if !spawn_server(&sock, buf, &latest_server_tcp_port, &mut server_map).await {
			crash!("Could not spawn initial setback_server.");
		}
		if !daemon::notify(false, [(daemon::STATE_READY, "1")].iter()).unwrap_or(false) {
			crash!("Could not notify systemd of ready state.");
		}
		info!("setbackd started successfully");

		// Then it's mostly I/O operations with recoverable errors
		tokio::spawn(async {
			tokio::select! {
				_ = listen_tcp_socket(stream, api, &latest_server_tcp_port) => unreachable_unchecked!("TCP socket listener should never return"),
				_ = sigterm0.recv() => {}
			}
		});
		tokio::select! {
			_ = listen_unix_socket(&sock, buf, &latest_server_tcp_port) => unreachable_unchecked!("Unix socket listener should never return"),
			_ = sigterm1.recv() => {}
		}
	});

	// background tasks should stop on their own quickly
	rt.shutdown_timeout(Duration::from_secs(4));
	info!("setbackd shutdown");
	std::io::stdout().flush();
}

/// Returns whether successfully spawned server.
/// Server is killed if something goes wrong.
async fn spawn_server(
	sock: &UnixDatagram,
	buf: &mut [u8],
	server_port: &AtomicU16,
	server_map: &mut HashMap<i32, StartInfo>,
) -> bool {
	match unsafe {libc::fork()} {
		0 => {
			// we are the child
			unsafe {libc::execve(
				SETBACK_SERVER_BIN,
				[c"setback_server".as_ptr(), std::ptr::null()].as_ptr(),
				environ as _
			)};
			// if execve was successful, the current process stops here
			// if execve failed, we are the current process still
			let e = std::io::Error::last_os_error().raw_os_error().unwrap();
			crash!("setback_server not spawned. execve failed. errno: {e}");
		}
		-1 => {
			// we are the parent and no child was spawned - check errno
			error!(
				"setback_server not spawned. fork failed. errno: {}",
				std::io::Error::last_os_error().raw_os_error().unwrap()
			);
			false
		}
		child_pid => {
			// we are the parent
			info!("setback_server spawned with PID: {child_pid}");
			const MAX_ATTEMPTS: u8 = 2;
			let mut addr = None;
			for _ in 0..MAX_ATTEMPTS {
				match sock.recv_from(buf).await {
					Ok(r) => {
						addr = Some(r.1);
						break;
					}
					Err(e) if e.kind() == ErrorKind::WouldBlock => {
						warn!("Unix socket would block receiving message. Error: {e}");
						tokio::time::sleep(Duration::from_millis(10));
						continue;
					}
					Err(e) => {
						error!("Error receiving message from unix socket. Error: {e}");
						continue;
					}
				}
			}
			let err = 'error: {
				let Some(addr) = addr else {break 'error Error::TimedOut};
				let sdp = match Sdp::deserialize(buf) {
					Ok(s) if !sdp.version.is_compatible(SDP_VERSION) => break 'error Error::BadVersion,
					Ok(s) => s,
					Err(e) => break 'error e,
				};
				match sdp.payload {
					Payload::NotifyStart(info) => {
						server_port.store(info.port, Ordering::Relaxed);
						server_map_insert(server_map, child_pid, info);
						return true;
					}
					_ => break 'error Error::BadProtocol,
				}
			};
			if unsafe {libc::kill(child_pid, libc::SIGKILL)} == 0 {
				error!("Could not connect unix socket to setback_server. setback_server process killed. Error: {err}");
			} else {
				error!(
					"Could not connect unix socket to setback_server. Error: {err}\nCould not kill setback_server. errno: {}",
					std::io::Error::last_os_error().raw_os_error().unwrap()
				);
			}
			false
		}
	}
}

/// Infinite loop to handle unix socket communication
async fn listen_unix_socket(
	sock: &UnixDatagram,
	buf: &mut [u8],
	server_port: &AtomicU16,
) -> ! {
	loop {
		let addr = match sock.recv_from(buf).await {
			Ok(r) => r.1,
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

/// Infinite loop to handle tcp socket communication.
/// Crashes if it fails to accept connections.
async fn listen_tcp_socket(stream: TcpListener, api: Client, server_port: &AtomicU16) -> ! {
	const MAX_RETRIES: u8 = 2;
	let mut tries = 0;
	loop {
		match stream.accept().await {
			Ok((stream, addr)) => {
				// the result is only relevant for setback_server
				tokio::spawn(ClientConnection::authenticate(stream, &api, server_port.load(Ordering::Relaxed), addr));
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

fn server_map_insert(server_map: &mut HashMap<i32, StartInfo>, pid: i32, info: StartInfo) {
	server_map.insert(pid, info);
	server_map_write(server_map);
}

fn server_map_remove(server_map: &mut HashMap<i32, StartInfo>, pid: i32) {
	server_map.remove(&pid);
	server_map_write(server_map);
}

fn server_map_write(server_map: &HashMap<i32, StartInfo>) {
	match std::fs::OpenOptions::new().write(true).create(true).open(PIDFILE) {
		Ok(file) => {postcard::to_io(server_map, file);}
		Err(e) => error!("Could not open pidfile. Error: {e}"),
	}
}

fn cd_auto_update() -> ! {
	todo!(
r#"Setup continuous deployment:
- inotify /var/lib/setback/stage/setback.tar.gz
- mkdir /var/lib/setback/v#.#.#
- unzip and move files
- update symlinks to point to latest version
- systemctl restart"#
	)
}