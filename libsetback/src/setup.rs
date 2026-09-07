use {
	arrayvec::ArrayVec,
	crate::*,
	serde::{Deserialize, Serialize},
	tracing::{error, info, warn},
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SetupRules {
	/// Every other player is on a team. The alternative being custom teams.
	pub std_team_mode: bool,
	pub win_by_2: bool,
	pub negative_scores: bool,
	pub dealer_override: bool,
	pub max_score: u8,
	pub max_players: u8,
	/// In seconds. 0 means unlimited time. The maximum allowed time on the server is [MAX_TURN_TIME_SERVER] seconds.
	pub turn_time: u8,
}

/// Setup data is created and modified when setting up a game.
/// The rules should be read-only when the game is being played.
/// See [GameSetup] for the wrapper struct that should be used during setup.
#[derive(Deserialize, Serialize, Debug)]
pub struct SetupState {
	pub rules: SetupRules,
	pub players: ArrayVec<Player, MAX_PLAYERS>,
}

impl SetupState {
	pub fn snapshot(&self) -> Self {
		Self {
			rules: self.rules.clone(),
			players: self.players.iter().map(|p| p.snapshot()).collect(),
		}
	}
}

impl Clone for SetupState {
	fn clone(&self) -> Self {
		self.snapshot()
	}
}

/// This struct just exists to wrap [SetupState] and provide methods for convenience while setting up a game.
/// These methods should not be called during a game, so it adds type safety to guarantee that they won't accidentally be used.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[repr(transparent)]
pub struct GameSetup(SetupState);

impl GameSetup {
	/// Default rules for a game. These can be changed by the host during setup.
	const fn new() -> Self {
		Self(SetupState {
			rules: SetupRules {
				std_team_mode: true,
				win_by_2: false,
				negative_scores: true,
				dealer_override: false,
				max_score: MAX_SCORE,
				max_players: MAX_PLAYERS as u8,
				turn_time: DEFAULT_TURN_TIME,
			},
			players: ArrayVec::new_const(),
		})
	}

	pub fn new_ai_only() -> Self {
		info!("Creating AI only game");
		let mut setup_data = Self::new();
		for _ in 0..MAX_PLAYERS {
			setup_data.add_ai();
		}
		setup_data
	}

	#[cfg(feature = "client")]
	pub fn new_singleplayer(username: String) -> Self {
		info!("Creating singleplayer game");
		let sp_self = Player::new_singleplayer_self(0, username);
		let mut setup_data = Self::new();
		setup_data.0.players.push(sp_self);
		for _ in 0..MAX_PLAYERS - 1 {
			setup_data.add_ai();
		}
		setup_data
	}

	#[cfg(not(feature = "client"))]
	pub fn new_multiplayer(host: Box<ClientConnection>) -> Self {
		info!("Creating multiplayer game");
		let host = Player::new_host_connected(host, 0);
		let mut setup_data = Self::new();
		setup_data.0.players.push(host);
		for _ in 0..MAX_PLAYERS - 1 {
			setup_data.add_ai();
		}
		setup_data
	}

	pub fn add_ai(&mut self) {
		info!("Adding AI player");
		self.0
			.players
			.push(Player::new_ai(self.0.players.len() as u8 & 1));
	}

	#[cfg(not(feature = "client"))]
	pub fn add_human(&mut self) {
		info!("Adding human player");
		self.0
			.players
			.push(Player::new_human_empty(self.0.players.len() as u8 & 1));
	}

	/// Updates a player's status as AI or human.
	#[cfg(not(feature = "client"))]
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

	/// Updates a player's team.
	pub fn update_team(&mut self, index: usize, team_id: u8) {
		info!("Setting player at index {index} to team {team_id}");
		self.0.players[index].team_id = team_id;
	}

	/// Makes this player type a networked client.
	#[cfg(not(feature = "client"))]
	pub fn set_client(&mut self, index: usize, conn: Box<ClientConnection>) {
		info!("Setting empty human at index {index} to connected human client");
		let pt = &mut self.0.players[index].player_type;
		debug_assert!(
			matches!(pt, PlayerType::HumanEmpty),
			"Player must be an empty human slot before setting a client connection"
		);
		*pt = PlayerType::HumanConnection { conn };
	}

	/// Initializes a game loop that hasn't started yet.
	/// ## Error
	/// The setup is in an invalid state
	/// like multiple humans in singleplayer or empty human slots in multiplayer
	#[allow(clippy::result_large_err)]
	pub fn init(
		mut self,
		#[cfg(not(feature = "client"))] err_q: ErrQ,
	) -> Result<GameState, Self> {
		info!("Attempting to initialize game");
		// verify that the players are in the correct state
		// Singleplayer
		// * cfg feature = client
		// * 1 SingleplayerSelf
		// * Remainder are Ai
		// Multiplayer
		// * cfg not feature = client
		// * at least 2 HumanConnection
		// * remainder are Ai or HumanConnection
		if self.0.players.len() < MIN_PLAYERS as usize {
			warn!(
				"Not enough players - got {}, need {MIN_PLAYERS}",
				self.0.players.len()
			);
			return Err(self);
		}
		#[cfg(feature = "client")]
		let mut num_singleplayer = 0;
		#[cfg(not(feature = "client"))]
		let mut num_human_connection = 0;
		#[cfg(not(feature = "client"))]
		let mut num_human_empty = 0;
		let mut num_ai = 0;
		self.0
			.players
			.iter_mut()
			.for_each(|p| match p.player_type {
				#[cfg(feature = "client")]
				PlayerType::SingleplayerSelf { .. } => num_singleplayer += 1,
				#[cfg(not(feature = "client"))]
				PlayerType::HumanConnection { .. } => num_human_connection += 1,
				#[cfg(not(feature = "client"))]
				PlayerType::HumanEmpty => num_human_empty += 1,
				PlayerType::Ai { .. } => num_ai += 1,
				_ => unreachable!(
					"PlayerType should be one of SingleplayerSelf, HumanConnection, HumanEmpty, or Ai"
				),
			});
		#[cfg(all(feature = "client", not(feature = "test-libsetback")))]
		if num_singleplayer != 1 || self.0.players.len() - 1 != num_ai {
			error!("Incorrect player configuration - humans: {num_singleplayer}, AIs: {num_ai}");
			return Err(self);
		}
		#[cfg(all(feature = "client", feature = "test-libsetback"))]
		if num_ai != self.0.players.len() {
			warn!("Incorrect player configuration - all players should be AI for tests");
			return Err(self);
		}
		#[cfg(not(feature = "client"))]
		if num_human_connection < 2
			|| num_human_empty == 0
			|| self.0.players.len() - num_human_connection != num_ai
		{
			error!(
				"Incorrect player configuration - empty humans: {}, connected humans: {}, AIs: {}",
				num_human_empty, num_human_connection, num_ai
			);
			return Err(self);
		}

		// normalize team numbers so there are no gaps
		let mut team_ids: ArrayVec<u8, MAX_PLAYERS> =
			self.0.players.iter().map(|p| p.team_id).collect();
		team_ids.sort();
		team_ids.partition_dedup();
		let mut team_id_map = [0; MAX_PLAYERS];
		for (index, id) in team_ids.iter().enumerate() {
			team_id_map[*id as usize] = index as u8;
		}
		for player in self.0.players.iter_mut() {
			player.team_id = team_id_map[player.team_id as usize];
		}

		// create team objects in the game data
		let mut teams = ArrayVec::new();
		for _ in team_ids {
			teams.push(Team::new());
		}

		// don't allow infinite time on the server
		#[cfg(not(feature = "client"))]
		if self.0.rules.turn_time == 0 || self.0.rules.turn_time > MAX_TURN_TIME_SERVER {
			self.0.rules.turn_time = MAX_TURN_TIME_SERVER;
		}

		Ok(GameState {
			teams,
			deck: Card::new_deck(),
			pile: ArrayVec::new(),
			trump: Suit::Hearts,
			lead: Suit::Hearts,
			owner_idx: 0,
			highest_card: Card::zero(),
			highest_bidder_idx: 0,
			highest_bid: Bid::Pass,
			turn_idx: 0,
			turn_num: 0,
			dealer_idx: rand::random_range(0..self.0.players.len() as u8),
			round_num: 0,
			setup: self.0,
			loop_state: GameLoopState::Init,
			#[cfg(not(feature = "client"))] err_q,
		})
	}
}
