use {
	std::{
		io::Write,
		net::{Ipv4Addr, SocketAddrV4},
		mem::MaybeUninit,
		process::exit,
		time::Instant,
	},
	reqwest::Client,
	tokio::net::TcpListener,
	tracing::{error, info, warn},
};

pub use setback::*;

const BIN_VERSION: Version = pkg_version!();
static mut SETBACK_API: MaybeUninit<Client> = MaybeUninit::uninit();

pub extern "Rust" fn setback_api() -> Client {
	// SAFETY:
	// Must link with libsetback.
	// SETBACK_API must never be mutated while this function is being called.
	unsafe {
		#[allow(static_mut_refs)]
		SETBACK_API.assume_init_ref().clone()
	}
}

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
	
	rt.block_on(async move {
		// SAFETY:
		// There are no other references to SETBACK_API.
		unsafe {
			#[allow(static_mut_refs)]
			SETBACK_API.write(Client::builder().cookie_store(true).build().unwrap());
		}
		
		tokio::select! {
			res = tokio::signal::ctrl_c() => if let Err(e) = res {
				error!("Terminating via SIGINT - {e}")
			} else {
				info!("Terminating via SIGINT")
			},
			_ = tcp_listen() => unreachable!(),
		}
	});
	info!("Initiating shutdown");
	let timer = Instant::now();
	rt.shutdown_timeout(SHUTDOWN_TIMEOUT);
	info!("Shutdown complete after {} ms", timer.elapsed().as_millis());
	_ = std::io::stdout().flush();
}

async fn tcp_listen() -> ! {
	let tcp_listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))
		.await
		.unwrap_or_else(crash!("Could not bind tcp listener. Error: {}", e));
	let state = State::new();
	loop {
		match tcp_listener.accept().await {
			Ok((stream, addr)) => {
				// ClientConnection::new will upgrade to ws and authenticate the client
				if let Some(client) = ClientConnection::new(stream, addr).await {
					state.clone().add_client(client).await;
				}
			}
			Err(e) => warn!("Could not accept client. Error: {e}"),
		}
	}
}