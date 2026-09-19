use {
	arrayvec::ArrayVec,
	crate::*,
	futures::TryFutureExt,
	serde::{Deserialize, Serialize},
	tracing::{error, debug},
	tokio::sync::mpsc::UnboundedSender,
};

#[derive(Deserialize, Serialize, Debug)]
pub struct LobbyRules {
	/// Every other player is on a team. The alternative being custom teams.
	pub std_team_mode: bool,
	pub win_by_2: bool,
	pub negative_scores: bool,
	pub dealer_override: bool,
	pub max_score: u8,
	pub max_players: u8,
	/// In seconds. 0 means unlimited time. The maximum allowed time on the server is [MAX_TURN_TIME_SERVER] seconds.
	pub turn_time: u8,
	pub code: String,
	pub password: String,
}

impl Clone for LobbyRules {
	fn clone(&self) -> Self {
		Self {
			std_team_mode: self.std_team_mode,
			win_by_2: self.win_by_2,
			negative_scores: self.negative_scores,
			dealer_override: self.dealer_override,
			max_score: self.max_score,
			max_players: self.max_players,
			turn_time: self.turn_time,
			code: self.code.clone(),
			// password must never be cloned
			password: String::new(),
		}
	}
}

/// Setup data is created and modified when setting up a game.
/// The rules should be read-only when the game is being played.
/// See [Lobby] for the wrapper struct that should be used in the lobby.
#[derive(Deserialize, Serialize, Debug)]
pub struct LobbyState {
	pub rules: LobbyRules,
	pub players: ArrayVec<Player, MAX_PLAYERS>,
	#[cfg(feature = "server")]
	#[serde(skip_serializing)]
	#[serde(deserialize_with = "deserialize_err_q_panic")]
	pub err_q: ErrQ,
}

impl LobbyState {
	pub fn snapshot(&self) -> Self {
		Self {
			rules: self.rules.clone(),
			players: self.players.iter().map(|p| p.snapshot()).collect(),
			#[cfg(feature = "server")]
			err_q: self.err_q.clone(),
		}
	}
	
	pub fn cx(&self) -> LobbyLogCx<'_> {
		LobbyLogCx {
			code: &self.rules.code,
			public: self.rules.password.is_empty(),
		}
	}
}

#[cfg(feature = "server")]
impl LobbyState {
	/// Sends a payload to player at idx
	pub fn notify(&mut self, payload: impl Into<Payload> + Clone, idx: u8) {
		let err_q = self.err_q.clone();
		let PlayerType::HumanConnection {conn} = &mut self
			.players[idx as usize]
			.player_type
		else {return};
		let fut = conn.send_expect_ack(payload.into())
			.inspect_err(move |e| err_q.send((idx, *e)).unwrap());
		tokio::spawn(fut);
	}
	
	pub fn notify_all(&mut self, payload: impl Into<Payload> + Clone) {
		self.notify_all_but(payload, u8::MAX);
	}
	
	pub fn notify_all_but(&mut self, payload: impl Into<Payload> + Clone, exclude: u8) {
		self.notify_all_unique_but(move |_,_| payload.clone(), exclude);
	}
	
	pub fn notify_all_unique<P: Into<Payload> + Clone>(&mut self, payload_fn: impl FnMut(u8, &Player) -> P) {
		self.notify_all_unique_but(payload_fn, u8::MAX);
	}
	
	pub fn notify_all_unique_but<P: Into<Payload> + Clone>(&mut self, mut payload_fn: impl FnMut(u8, &Player) -> P, exclude: u8) {
		self.players
			.iter_mut()
			.enumerate()
			.filter(|(i,_)| *i != exclude as usize)
			.for_each(|(i, player)| {
				let payload = payload_fn(i as u8, player);
				let PlayerType::HumanConnection {conn} = &mut player.player_type else {return;};
				let err_q = self.err_q.clone();
				let fut = conn.send_expect_ack(payload.into())
					.inspect_err(move |e| err_q.send((i as u8, *e)).unwrap());
				tokio::spawn(fut);
			});
	}
}

impl Clone for LobbyState {
	fn clone(&self) -> Self {
		self.snapshot()
	}
}

/// This struct just exists to wrap [LobbyState] and provide methods for convenience while setting up a game.
/// These methods should not be called during a game, so it adds type safety to guarantee that they won't accidentally be used.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Lobby {
	pub lobby_state: LobbyState,
}

impl Lobby {
	/// Default rules for a game. These can be changed by the host in the lobby.
	pub const fn new(
		code: String,
		password: String,
		#[cfg(feature = "server")] err_q: ErrQ,
	) -> Self {
		Self {
			lobby_state: LobbyState {
				rules: LobbyRules {
					std_team_mode: true,
					win_by_2: false,
					negative_scores: true,
					dealer_override: false,
					max_score: MAX_SCORE,
					max_players: MAX_PLAYERS as u8,
					turn_time: DEFAULT_TURN_TIME,
					code,
					password,
				},
				players: ArrayVec::new_const(),
				#[cfg(feature = "server")] err_q,
			}
		}
	}
	
	#[cfg(not(feature = "server"))]
	pub fn create_ai_only() -> Self {
		debug!("Creating AI only game");
		let mut lobby = Self::new(String::new(), String::new());
		for _ in 0..MAX_PLAYERS {
			lobby.add_ai();
		}
		lobby
	}

	#[cfg(not(feature = "server"))]
	pub fn create_singleplayer(username: String) -> Self {
		debug!("Creating singleplayer game");
		let sp_self = Player::new_singleplayer_self(0, username);
		let mut lobby = Self::new(String::new(), String::new());
		lobby.lobby_state.players.push(sp_self);
		for _ in 0..MAX_PLAYERS - 1 {
			lobby.add_ai();
		}
		lobby
	}

	pub fn add_ai(&mut self) {
		self.lobby_state
			.players
			.push(Player::new_ai(self.lobby_state.players.len() as u8 & 1));
	}

	/// Updates a player's team.
	pub fn update_team(&mut self, index: usize, team_id: u8) {
		self.lobby_state.players[index].team_id = team_id;
	}

	/// Initializes a game loop that hasn't started yet.
	/// ## Error
	/// The lobby is in an invalid state
	/// like multiple humans in singleplayer or empty human slots in multiplayer
	pub fn finish(mut self) -> Result<GameState, Self> {
		debug!(cx = %self.cx(), "Attempting to initialize game");
		// verify that the players are in the correct state
		// Singleplayer
		// * cfg feature = client
		// * 1 SingleplayerSelf
		// * Remainder are Ai
		// Multiplayer
		// * cfg not feature = client
		// * at least 2 HumanConnection
		// * remainder are Ai or HumanConnection
		if self.lobby_state.players.len() < MIN_PLAYERS as usize {
			error!(
				cx = %self.cx(),
				"Not enough players - got {}, need {MIN_PLAYERS}",
				self.lobby_state.players.len()
			);
			return Err(self);
		}
		#[cfg(not(feature = "server"))]
		let mut num_singleplayer = 0;
		#[cfg(feature = "server")]
		let mut num_human_connection = 0;
		#[cfg(feature = "server")]
		let mut num_human_empty = 0;
		let mut num_ai = 0;
		self.lobby_state
			.players
			.iter_mut()
			.for_each(|p| match p.player_type {
				#[cfg(not(feature = "server"))]
				PlayerType::SingleplayerSelf { .. } => num_singleplayer += 1,
				#[cfg(feature = "server")]
				PlayerType::HumanConnection { .. } => num_human_connection += 1,
				#[cfg(feature = "server")]
				PlayerType::HumanEmpty => num_human_empty += 1,
				PlayerType::Ai { .. } => num_ai += 1,
				_ => unreachable!(
					"PlayerType should be one of SingleplayerSelf, HumanConnection, HumanEmpty, or Ai"
				),
			});
		#[cfg(all(not(feature = "server"), not(feature = "test-libsetback")))]
		if num_singleplayer != 1 || self.lobby_state.players.len() - 1 != num_ai {
			error!(cx = %self.cx(), "Incorrect player configuration - humans: {num_singleplayer}, AIs: {num_ai}");
			return Err(self);
		}
		#[cfg(all(not(feature = "server"), feature = "test-libsetback"))]
		if num_ai != self.lobby_state.players.len() {
			error!(cx = %self.cx(), "Incorrect player configuration - all players should be AI for tests");
			return Err(self);
		}
		#[cfg(feature = "server")]
		if num_human_connection < 2
			|| num_human_empty > 0
			|| self.lobby_state.players.len() - num_human_connection != num_ai
		{
			error!(
				cx = %self.cx(),
				"Incorrect player configuration - empty humans: {}, connected humans: {}, AIs: {}",
				num_human_empty, num_human_connection, num_ai
			);
			return Err(self);
		}

		// normalize team numbers so there are no gaps
		let mut team_ids: ArrayVec<u8, MAX_PLAYERS> = self
			.lobby_state
			.players
			.iter()
			.map(|p| p.team_id)
			.collect();
		team_ids.sort();
		team_ids.partition_dedup();
		let mut team_id_map = [0; MAX_PLAYERS];
		for (index, id) in team_ids.iter().enumerate() {
			team_id_map[*id as usize] = index as u8;
		}
		for player in self.lobby_state.players.iter_mut() {
			player.team_id = team_id_map[player.team_id as usize];
		}

		// create team objects in the game data
		let mut teams = ArrayVec::new();
		for _ in team_ids {
			teams.push(Team::new());
		}

		// don't allow infinite time on the server
		#[cfg(feature = "server")]
		if self.lobby_state.rules.turn_time == 0 || self.lobby_state.rules.turn_time > MAX_TURN_TIME_SERVER {
			self.lobby_state.rules.turn_time = MAX_TURN_TIME_SERVER;
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
			dealer_idx: rand::random_range(0..self.lobby_state.players.len() as u8),
			round_num: 0,
			lobby_state: self.lobby_state,
			loop_state: GameLoopState::Init,
		})
	}
	
	pub fn cx(&self) -> LobbyLogCx<'_> {
		self.lobby_state.cx()
	}
}

pub struct LobbyLogCx<'lobby> {
	code: &'lobby str,
	public: bool,
}

impl std::fmt::Display for LobbyLogCx<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "code:{},pub:{}", self.code, self.public)
	}
}

#[cfg(feature = "server")]
fn deserialize_err_q_panic<'de, D: serde::Deserializer<'de>>(_: D)
	-> Result<UnboundedSender<(u8, NetError)>, D::Error>
{
	Err(serde::de::Error::custom("unknown field `err_q`"))
}