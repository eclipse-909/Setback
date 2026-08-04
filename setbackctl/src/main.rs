use {
	clap::{Parser, Subcommand},
	libsetback::{SDP_VERSION, sdp::*, SETBACKCTL_UNIX_SOCKET, SETBACK_WEBSITE_UNIX_SOCKET},
	std::os::unix::net::UnixDatagram
};

/// CLI controller for Setback processes (setbackd, setback_api, and setback_server)
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
	/// Lists all running processes
	Ps {
		/// Only show setback_server processes
		#[arg(short, default_value_t = false)]
		servers: bool,
	},
	/// Starts the service if not already running
	Start {
		/// Only show setback_server processes
		#[arg(short, long, default_value_t = String::from("latest"))]
		version: String,
		#[command(subcommand)]
		service: Service,
	},
	/// Gracefully shuts down the service if running
	Stop {
		#[command(subcommand)]
		service: Service,
	},
	/// Starts and stops the service if running
	Restart {
		#[command(subcommand)]
		service: Service,
	},
	/// Immediately kills the service if running.
	/// Do not use this unless the service is vulnerable to being exploited.
	Kill {
		#[command(subcommand)]
		service: Service,
	},
	/// Print the directory tree of relevant files
	Tree {
		/// Display the tree for all versions in /var/lib/setback
		#[arg(short, default_value_t = false)]
		all_versions: bool,
	},
}

#[derive(Subcommand)]
enum Service {
	/// The setback_api
	Api,
	/// The setbackd daemon
	Daemon,
	/// A setback_server
	Server {
		/// The process ID (PID) of the server
		#[arg(short, long)]
		pid: i32,
	},
}

fn main() {
	let cli = Cli::parse();

	todo!()
}