use {
	crate::*,
	nanoid::nanoid,
	tracing::info,
	tokio::{
		sync::mpsc::{UnboundedReceiver, UnboundedSender, Sender, channel},
		task::JoinHandle,
	},
};

pub type LspTx = UnboundedSender<Lsp>;
pub type LspRx = UnboundedReceiver<Lsp>;

/// Lobby-Server Protocol
pub enum Lsp {
	StartGame {code: String, password: String},
	TerminateGame {code: String, password: String},
	RemoveClient(Box<ClientConnection>),
}

pub struct LobbyRtHandle {
	pub join_tx: Sender<Box<ClientConnection>>,
	pub handle: JoinHandle<Lobby>,
}

impl Lobby {
	pub fn create_multiplayer(
		host: Box<ClientConnection>,
		password: String,
		lsp_tx: LspTx,
	) -> (LobbyRtHandle, String) {
		let host = Player::new_host(host, 0);
		let username = host.username();
		
		// TODO we might need a different way to create game codes if there is a risk of collisions
		let code = nanoid!(GAME_CODE_LEN, &GAME_CODE_ALPHABET);
		let mut lobby = Self::new(code.clone(), password.clone());
		lobby.0.players.push(host);
		for _ in 0..MAX_PLAYERS - 1 {
			lobby.add_ai();
		}
		
		info!("Creating multiplayer game with code: {code} - host: {username}");
		let cd = code.clone();
		let (join_tx, mut join_rx) = channel(MAX_PLAYERS);
		let handle = tokio::spawn(async move {
			loop {
				tokio::select! {
					Some(message) = Player::try_recv_fallback(&mut lobby.0.players) => {
						if lobby.handle_fallback(message) {
							break;
						}
					}
					Some(conn) = join_rx.recv() => lobby.set_next_client(conn, &lsp_tx),
					else => {
						_ = lsp_tx.send(Lsp::TerminateGame {code, password});
						break;
					}
				}
			}
			lobby
		});
		(LobbyRtHandle {join_tx, handle}, cd)
	}
	
	fn add_human(&mut self) {
		info!("Adding human player");
		self.0
			.players
			.push(Player::new_human_empty(self.0.players.len() as u8 & 1));
	}
	
	/// Updates a player's status as AI or human.
	fn update_human(&mut self, index: usize, is_human: bool) {
		info!(
			"Setting player at index {index} to {}",
			if is_human { "empty human" } else { "AI" }
		);
		self.0.players[index].player_type = if is_human {
			PlayerType::HumanEmpty
		} else {
			PlayerType::Ai { account_id: 0 }
		};
	}
	
	/// If there are any HumanEmpty slots, the conn will take up the next slot.
	/// If there are no HumanEmpty slots, the conn will replace an AI.
	/// If there are no available slots, the conn will be sent back to the server.
	fn set_next_client(&mut self, conn: Box<ClientConnection>, lsp_tx: &LspTx) {
		if let Some(player) = self
			.0
			.players
			.iter_mut()
			.find(|p| matches!(p.player_type, PlayerType::HumanEmpty))
		{
			player.player_type = PlayerType::HumanConnection {conn};
			return;
		}
		if let Some(player) = self
			.0
			.players
			.iter_mut()
			.find(|p| matches!(p.player_type, PlayerType::Ai {..}))
		{
			player.player_type = PlayerType::HumanConnection {conn};
			return;
		}
		_ = lsp_tx.send(Lsp::RemoveClient(conn));
	}
	
	/// Returns true if the game is starting or terminating
	fn handle_fallback(&mut self, (player_idx, message): (u8, Result<Ssp, NetError>)) -> bool {
		let ssp = match message {
			Ok(ssp) => ssp,
			Err(e) => {
				todo!();
				return false;
			}
		};
		match ssp.payload.try_lobby() {
			Ok(_) => {}
			_ => {}
		}
		todo!()
	}
}