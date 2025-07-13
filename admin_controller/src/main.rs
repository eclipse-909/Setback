use {
	std::{
		io::stdin,
		fmt::{self, Display},
		sync::RwLock,
		pin::Pin
	},
	libsetback::{
		ClientConnection, SetbackConnection, ssp, Protocol, Error, AdminMessage
	},
	tokio::sync::mpsc::{Sender, Receiver, channel},
	tokio_util::sync::CancellationToken
};

struct AdminCommand {
	command: &'static str,
	description: &'static str,
	action: fn(&mut ClientConnection, &Sender<bool>, u32) -> Pin<Box<dyn Future<Output=()> + Send>>,
}

impl Display for AdminCommand {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}:\n  {}", self.command, self.description)
	}
}

const ADMIN_COMMANDS: &[AdminCommand] = &[
	AdminCommand{command: "exit", description: "Exit the admin console.", action: Box::pin(async |_,_,_| {
		println!("Disconnected");
		std::process::exit(0);
	})},
	AdminCommand{command: "help", description: "Show this help message.", action: async |_,tx,_| {
		println!("Available commands:");
		for cmd in ADMIN_COMMANDS {
			println!("{}", cmd);
		}
		tx.try_send(true).unwrap();
	}},
	AdminCommand{command: "login", description: "Login and authorize as an admin.", action: authorize},
	AdminCommand{command: "disable", description: "Disable the ability for users to create a new game.\n  Must be authorized.", action: Box::pin(async |conn, _, account_id| {
		conn.write(ssp::new_with_src(Protocol::Admin(AdminMessage::DisableCreateGame), account_id)).await;
	})},
	AdminCommand{command: "enable", description: "Enable the ability for users to create a new game.\n  Must be authorized.", action: Box::pin(async |conn, _, account_id| {
		conn.write(ssp::new_with_src(Protocol::Admin(AdminMessage::EnableCreateGame), account_id)).await;
	})},
	AdminCommand{command: "numgames", description: "Show the number of active games.\n  Must be authorized.", action: Box::pin(async |conn, _, account_id| {
		conn.write(ssp::new_with_src(Protocol::Admin(AdminMessage::NumActiveGames(0)), account_id)).await;
	})}
];

static PROMPT_STR: RwLock<String> = RwLock::new(String::new());

fn authorize(conn: &mut ClientConnection, _: &'static Sender<bool>, account_id: u32) -> Pin<Box<dyn Future<Output = ()> + Send>> {Box::pin(async move {
	let admin_password: String = rpassword::prompt_password("Enter the admin password: ").unwrap();
	conn.write(ssp::new_with_src(Protocol::Admin(AdminMessage::Authorize{admin_password}), account_id)).await;
})}

#[tokio::main]
async fn main() {



	//TODO once I create an admin account for myself, I can just bake the username, password, and admin_password into the program so I don't have to type it every time




	println!("Enter your username: ");
	let mut username: String = String::new();
	stdin().read_line(&mut username).unwrap();
	let password: String = rpassword::prompt_password("Enter your password: ").unwrap();
	let cancellation_token: CancellationToken = CancellationToken::new();
	let (ssp_waiter_tx, mut ssp_waiter_rx): (Sender<bool>, Receiver<bool>) = channel(1);
	println!("Connecting to server...");
	let mut connection: ClientConnection = match ClientConnection::new_connection(
		username.clone(),
		password,
		ssp_callback(cancellation_token.clone(), ssp_waiter_tx.clone(), username),
		on_write_error(cancellation_token.clone(), ssp_waiter_tx.clone())
	).await {
		Ok(connection) => connection,
		Err(error) => {
			println!("Error could not connect to server: {:?}", error);
			return;
		}
	};
	println!("Logged-in and connected to server");
	let account_id: u32 = connection.account_id;
	authorize(&mut connection, &ssp_waiter_tx, account_id).await;
	let mut command: String = String::new();
	*PROMPT_STR.write().unwrap() = String::from("Setback-User> ");
	loop {
		match ssp_waiter_rx.recv().await {
			Some(false) | None => break,
			Some(true) => {
				print!("{}", PROMPT_STR.read().unwrap());
				command.clear();
				stdin().read_line(&mut command).unwrap();
				match ADMIN_COMMANDS.iter().find(|cmd| cmd.command == command.as_str()) {
					Some(cmd) => (cmd.action)(&mut connection, &ssp_waiter_tx, account_id),
					None => {
						println!("Invalid command");
						ssp_waiter_tx.try_send(true).unwrap();
					}
				}
			}
		}
	}
	cancellation_token.cancelled().await;
	connection.cancellation_token.cancelled().await;
	println!("Disconnected");
}

fn ssp_callback(cancel_token: CancellationToken, ssp_waiter_tx: Sender<bool>, username: String) -> impl Fn(Option<Ssp>) + Send + Sync + 'static {
	move |message: Option<Ssp>| {
		match message {
			Some(ssp) => {
				match ssp.protocol {
					Protocol::SyncID => {}
					Protocol::Admin(admin_message) => {
						match admin_message {
							AdminMessage::Authorize{..} => unreachable!("Authorize message should not be sent to the admin"),
							AdminMessage::AuthorizeResponse(authorized) => {
								if authorized {
									println!("You are authorized with admin privileges");
									*PROMPT_STR.write().unwrap() = format!("{}-Admin> ", username);
								} else {
									println!("Not authorized. Type \"login\" to authorize.");
								}
							}
							AdminMessage::DisableCreateGame => {
								println!("Creating games is now disabled");
							}
							AdminMessage::EnableCreateGame => {
								println!("Creating games is now enabled");
							}
							AdminMessage::NumActiveGames(num_games) => {
								println!("There are {} active games", num_games);
							}
						}
					}
					_ => unreachable!("Invalid ssp protocol")
				}
				ssp_waiter_tx.try_send(true).unwrap();
			}
			None => {
				println!("Connection lost");
				cancel_token.cancel();
				ssp_waiter_tx.try_send(false).unwrap();
			}
		}
	}
}

fn on_write_error(cancel_token: CancellationToken, ssp_waiter_tx: Sender<bool>) -> impl Fn(Error) + Send + Sync + 'static {
	move |error| {
		println!("Error sending message to server: {:?}", error);
		cancel_token.cancel();
		ssp_waiter_tx.try_send(false).unwrap();
	}
}