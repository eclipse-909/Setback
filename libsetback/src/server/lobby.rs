use {
	crate::*,
	nanoid::nanoid,
	tracing::info,
};

impl Lobby {
	pub fn create_multiplayer(host: Box<ClientConnection>, password: String) -> (Self, String) {
		let host = Player::new_host_connected(host, 0);
		let username = host.username();
		let code = nanoid!(GAME_CODE_LEN, &GAME_CODE_ALPHABET);
		let mut lobby = Self::new(code.clone(), password);
		lobby.0.players.push(host);
		for _ in 0..MAX_PLAYERS - 1 {
			lobby.add_ai();
		}
		info!("Creating multiplayer game with code: {code} - host: {username}");
		(lobby, code)
	}
	
	pub fn add_human(&mut self) {
		info!("Adding human player");
		self.0
			.players
			.push(Player::new_human_empty(self.0.players.len() as u8 & 1));
	}
	
	/// Updates a player's status as AI or human.
	pub fn update_human(&mut self, index: usize, is_human: bool) {
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
	/// If all players are HumanConnection, this function errors.
	pub fn set_next_client(&mut self, conn: ClientConnection) -> Result<(), ClientConnection> {
		if let Some(player) = self
			.0
			.players
			.iter_mut()
			.find(|p| matches!(p.player_type, PlayerType::HumanEmpty))
		{
			player.player_type = PlayerType::HumanConnection {conn: Box::new(conn)};
			return Ok(());
		}
		if let Some(player) = self
			.0
			.players
			.iter_mut()
			.find(|p| matches!(p.player_type, PlayerType::Ai {..}))
		{
			player.player_type = PlayerType::HumanConnection {conn: Box::new(conn)};
			return Ok(());
		}
		Err(conn)
	}
}