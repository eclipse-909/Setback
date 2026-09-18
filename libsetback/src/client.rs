use {
	crate::*,
	reqwest::Client,
	tokio::sync::mpsc::unbounded_channel,
	tracing::{info, error},
};

#[cfg(target_family = "wasm")]
fn setback_api() -> Client {
	Client::builder().build().unwrap()
}

#[cfg(not(target_family = "wasm"))]
fn setback_api() -> Client {
	Client::builder().cookie_store(true).build().unwrap()
}

/// Connection to the server, owned by the client.
pub struct SetbackClient {
	ssp_tx: SspTx,
	fallback_rx: FallbackRx,
	correlation_id: u32,
	api: Client,
	auth_token: String,
	renew_token: String,
	ready: bool,
}

impl WebSocketeer for SetbackClient {
	fn ssp_tx(&self) -> &SspTx {&self.ssp_tx}
	fn correlation_id(&mut self) -> &mut u32 {&mut self.correlation_id}
}

impl SetbackClient {
	/// password must be plaintext.
	///
	/// The returned client is fully connected to the server or an error is returned.
	///
	/// If you send a message to the server that expects a response, you can just call the async function that sends the message,
	/// and awaiting that function will return the response from the server if all goes well.
	pub async fn new(credentials: LoginRequest) -> Result<Self, Error> {
		info!("Logging into api");
		let api = setback_api();
		let session = api
			.post(URL_API_LOGIN)
			.json(&credentials)
			.send()
			.await
			.map_err(|_| NetError::Disconnected)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<LoginResponse>()
			.await
			.map_err(|_| NetError::ServerError)?;
		info!("Logged into api");

		info!("Connecting to Setback server");
		let (mut reader, mut writer) = ws::ws_connect().await?;
		info!("Connected to Setback server");
		
		// No need to use correlation yet because this handshake is required before arbitrary communication
		info!("Authenticating with Setback server");
		ws::ws_send(&mut writer, &Ssp::new(
			Payload::Authenticate {
				version: SSP_VERSION,
				auth_token: session.auth_token.clone(),
			},
			CorrelationId::request(0)
		)).await.inspect_err(|e| {
			error!("Failed to authenticate: {e}");
		})?;
		let auth_res = ws::ws_next(&mut reader).await.inspect_err(|e| {
			error!("Failed to authenticate: {e}");
		})?;
		if auth_res.correlation_id != CorrelationId::event() {
			error!("Failed to authenticate");
			return Err(NetError::ServerError.into());
		};
		match auth_res.payload {
			Payload::Ack => {}
			Payload::Err(e) => {
				error!("Failed to authenticate");
				return Err(e);
			}
			_ => {
				error!("Failed to authenticate");
				return Err(NetError::ServerError.into());
			}
		}
		info!("Authenticated with Setback server");
		
		let (ssp_tx, ssp_rx) = unbounded_channel();
		let (fallback_tx, fallback_rx) = unbounded_channel();
		dioxus::prelude::spawn(Self::run_rw_handler(reader, writer, ssp_rx, fallback_tx));
		
		Ok(Self {
			ssp_tx,
			fallback_rx,
			correlation_id: 1, // DO NOT TOUCH THIS 1, see WebSocketeer::correlation_id
			api,
			auth_token: session.auth_token,
			renew_token: session.renew_token,
			ready: false,
		})
	}

	pub async fn logout(
		&self,
		auth_token: &str,
	) -> Result<(), Error> {
		info!("Logging out");
		#[cfg(not(feature = "test-libsetback"))]
		self.api
			.get(URL_API_LOGOUT)
			.bearer_auth(auth_token)
			.send()
			.await
			.map_err(|_| NetError::Disconnected)?
			.error_for_status()
			.map_err(Error::from)?;
		info!("Logged out");
		Ok(())
	}
	
	// Generic payloads
	
	async fn send_expect_ack(&mut self, payload: impl Into<Payload>) -> Result<(), NetError> {
		let (response, id) = self.send_new_request(payload).await?;
		if !matches!(response, Payload::Ack) {
			return Err(NetError::BadRequest);
		}
		self.send_response_ack(id)
	}
	
	// Server payloads

	async fn send_expect_lobby_state(&mut self, payload: ServerPayload) -> Result<(Box<LobbyState>, u8), Error> {
		let (response, id) = self.send_new_request(payload).await?;
		let lobby = match response {
			// switches protocols
			Payload::Lobby(LobbyPayload::Lobby {lobby, player_idx}) => Ok((lobby, player_idx)),
			Payload::Err(err) => Err(err),
			_ => Err(NetError::ServerError.into()),
		};
		self.send_response_ack(id)?;
		lobby
	}

	pub async fn send_create_game(&mut self, password: String) -> Result<(Box<LobbyState>, u8), Error> {
		self.send_expect_lobby_state(ServerPayload::CreateGame { password }).await
	}

	pub async fn send_join_game(
		&mut self,
		code: String,
		password: String,
	) -> Result<(Box<LobbyState>, u8), Error> {
		self.send_expect_lobby_state(ServerPayload::JoinGame { code, password }).await
	}

	pub async fn send_rejoin_game(
		&mut self,
		code: String,
		password: String,
	) -> Result<(Box<GameState>, u8), Error> {
		let payload = ServerPayload::RejoinGame { code, password };
		let (response, id) = self.send_new_request(payload).await?;
		let game = match response {
			// switches protocols
			Payload::Game(GamePayload::Snapshot {game_state, player_idx}) => Ok((game_state, player_idx)),
			Payload::Err(err) => Err(err),
			_ => Err(NetError::ServerError.into()),
		};
		self.send_response_ack(id)?;
		game
	}

	pub async fn send_search_games(
		&mut self,
		filter: GameFilter,
	) -> Result<Vec<PublicGame>, Error> {
		let (response, id) = self.send_new_request(ServerPayload::SearchGames(filter)).await?;
		let games = match response {
			Payload::Server(ServerPayload::PublicGames(games)) => Ok(games),
			Payload::Err(err) => Err(err),
			_ => Err(NetError::ServerError.into()),
		};
		self.send_response_ack(id)?;
		games
	}
	
	// Lobby payloads

	/// Host-only
	pub async fn send_lobby(&mut self, lobby: LobbyState) -> Result<(), NetError> {
		self.send_expect_ack(LobbyPayload::Lobby {lobby: Box::new(lobby), player_idx: 0}).await
	}

	pub async fn send_update_team(&mut self, player_idx: u8, team: u8) -> Result<(), NetError> {
		self.send_expect_ack(LobbyPayload::UpdatePlayerTeam { player_idx, team }).await
	}

	/// Host-only
	pub async fn send_update_player_type(
		&mut self,
		player_idx: u8,
		is_human: bool,
	) -> Result<(), NetError> {
		self.send_expect_ack(LobbyPayload::UpdatePlayerType {
			player_idx,
			is_human,
		}).await
	}

	pub async fn send_ready(&mut self, is_ready: bool) -> Result<(), NetError> {
		self.send_expect_ack(LobbyPayload::Ready {
			is_ready,
			player_idx: 0,
		}).await
	}

	/// Host-only
	pub async fn send_add_player(&mut self) -> Result<(), NetError> {
		self.send_expect_ack(LobbyPayload::AddPlayer).await
	}

	/// Host-only
	pub async fn send_remove_player(&mut self, player_idx: u8) -> Result<(), NetError> {
		self.send_expect_ack(LobbyPayload::RemovePlayer { player_idx }).await
	}

	/// Host-only
	pub async fn send_swap_players(
		&mut self,
		player_idx_a: u8,
		player_idx_b: u8,
	) -> Result<(), NetError> {
		self.send_expect_ack(LobbyPayload::SwapPlayers {
			player_idx_a,
			player_idx_b,
		}).await
	}

	/// Host-only
	pub async fn send_reinsert_player(
		&mut self,
		player_idx: u8,
		new_player_idx: u8,
	) -> Result<(), NetError> {
		self.send_expect_ack(LobbyPayload::ReinsertPlayer {
			player_idx,
			new_player_idx,
		}).await
	}
	
	// In-game payloads

	pub async fn send_game_event(&mut self, event: GameEvent) -> Result<(), NetError> {
		self.send_expect_ack(GamePayload::GameEvent(event)).await
	}

	pub async fn send_report_player(&mut self, report: ReportPlayerRequest) -> Result<(), NetError> {
		self.send_expect_ack(GamePayload::ReportPlayer(report)).await
	}
}

impl std::fmt::Debug for SetbackClient {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ServerConnection")
			.field("ready", &self.ready)
			.finish()
	}
}
