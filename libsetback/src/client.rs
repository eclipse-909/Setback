use {
	crate::{ws::ws_client::*, *},
	reqwest::Client,
	std::collections::HashMap,
	tokio::sync::mpsc::*,
	tracing::{info, warn},
};

#[cfg(target_family = "wasm")]
fn setback_api() -> Client {
	Client::builder().build().unwrap()
}

#[cfg(not(target_family = "wasm"))]
fn setback_api() -> Client {
	Client::builder().cookie_store(true).build().unwrap()
}

type CorrelationMap = HashMap<u32, UnboundedSender<Ssp>>;

/// Connection to the server, owned by the client.
pub struct SetbackClient {
	ws: Websocket,
	pending_reqs: CorrelationMap,
	api: Client,
	auth_token: String,
	renew_token: String,
	ready: bool,
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
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<LoginResponse>()
			.await
			.map_err(|_| Error::LogicError)?;
		info!("Logged into api");

		info!("Connecting to Setback server");
		let ws = ws_connect().await?;
		info!("Connected to Setback server");
		let mut client = Self {
			ws,
			pending_reqs: CorrelationMap::default(),
			api,
			auth_token: session.auth_token.clone(),
			renew_token: session.renew_token,
			ready: false,
		};

		// No need to use correlation yet because this handshake is required before arbitrary communication
		info!("Authenticating with Setback server");
		client
			.write(CorrelationId::request(0), Payload::Authenticate { auth_token: session.auth_token })
			.await?;
		let auth_res = ws_next(&mut client.ws).await?;
		if auth_res.correlation_id != CorrelationId::event()
			|| !matches!(auth_res.payload, Payload::Ack)
		{
			return Err(Error::ServerError);
		};
		info!("Authenticated with Setback server");

		Ok(client)
	}

	pub async fn logout(
		&self,
		#[cfg(not(feature = "test-dep"))] auth_token: &str,
	) -> Result<(), Error> {
		info!("Logging out");
		#[cfg(not(feature = "test-dep"))]
		self.api
			.get(URL_API_LOGOUT)
			.bearer_auth(auth_token)
			.send()
			.await
			.map_err(|_| Error::NetworkError)?
			.error_for_status()
			.map_err(|_| Error::ServerError)?;
		info!("Logged out");
		Ok(())
	}

	/// CorrelationId.end must be false. Removes the id from the map if Err is returned.
	pub async fn write(
		&mut self,
		correlation_id: CorrelationId,
		payload: Payload,
	) -> Result<(), Error> {
		debug_assert!(
			!correlation_id.end,
			"Any message sent to the server should expect a response"
		);
		ws_send(&mut self.ws, &Ssp::new(payload, correlation_id))
			.await
			.inspect_err(|e| {
				self.pending_reqs.remove(&correlation_id.id);
				warn!("Failed to write Ssp to websocket. Error: {e}");
			})
	}

	/// Returns the Ssp if it has no correlation id.
	/// Sends it to pending reqs map otherwise.
	pub async fn read(&mut self) -> Result<Option<Ssp>, Error> {
		let ssp = ws_next(&mut self.ws).await?;
		if ssp.correlation_id.id == 0 {
			return Ok(Some(ssp));
		}
		// if the transmitter isn't in the hash map,
		// the request probably timed out,
		// and it was removed from the hash map
		if ssp.correlation_id.end {
			if let Some(correlation_tx) = self.pending_reqs.remove(&ssp.correlation_id.id) {
				correlation_tx.send(ssp).unwrap();
			}
		} else if let Some(correlation_tx) = self.pending_reqs.get(&ssp.correlation_id.id) {
			correlation_tx.send(ssp).unwrap();
		}
		Ok(None)
	}

	/// Starts a message correlation where the correlation id is passed into all messages
	/// and the receiver is used to await responses from the server.
	fn start_correlation(&mut self) -> (CorrelationId, UnboundedReceiver<Ssp>) {
		//get new key
		let mut next_key = 0;
		//TODO: find a better way to make a unique key.
		// This loop will probably never run more than a few times, but still.
		let (tx, rx) = unbounded_channel();
		loop {
			if let std::collections::hash_map::Entry::Vacant(e) = self.pending_reqs.entry(next_key)
			{
				//create new channel
				e.insert_entry(tx);
				return (CorrelationId::request(next_key), rx);
			}
			next_key += 1;
		}
	}

	/// Send a message to the server and return its response
	async fn send_recv(&mut self, payload: Payload) -> Result<Payload, Error> {
		let (correlation_id, mut rx) = self.start_correlation();
		self.write(correlation_id, payload).await?;
		let ssp = rx.recv().await.ok_or({
			self.pending_reqs.remove(&correlation_id.id);
			Error::Disconnected
		})?;
		debug_assert_eq!(
			ssp.correlation_id,
			CorrelationId::response(correlation_id.id)
		);
		match ssp.payload {
			Payload::Err(err) => Err(err),
			_ => Ok(ssp.payload),
		}
	}

	async fn send_expect_game_setup(&mut self, payload: Payload) -> Result<Box<SetupState>, Error> {
		match self.send_recv(payload).await? {
			Payload::Game(game) => Ok(game),
			Payload::Err(err) => Err(err),
			_ => Err(Error::ServerError),
		}
	}

	pub async fn send_create_game(&mut self, password: String) -> Result<Box<SetupState>, Error> {
		self.send_expect_game_setup(Payload::CreateGame { password })
			.await
	}

	pub async fn send_join_game(
		&mut self,
		code: u32,
		password: String,
	) -> Result<Box<SetupState>, Error> {
		self.send_expect_game_setup(Payload::JoinGame { code, password })
			.await
	}

	pub async fn send_rejoin_game(
		&mut self,
		code: u32,
		password: String,
	) -> Result<Box<SetupState>, Error> {
		self.send_expect_game_setup(Payload::RejoinGame { code, password })
			.await
	}

	pub async fn send_search_games(
		&mut self,
		filter: GameFilter,
	) -> Result<Vec<PublicGame>, Error> {
		match self.send_recv(Payload::SearchGames(filter)).await? {
			Payload::PublicGames(games) => Ok(games),
			Payload::Err(err) => Err(err),
			_ => Err(Error::ServerError),
		}
	}

	async fn send_expect_ack(&mut self, payload: Payload) -> Result<(), Error> {
		match self.send_recv(payload).await? {
			Payload::Ack => Ok(()),
			Payload::Err(err) => Err(err),
			_ => Err(Error::ServerError),
		}
	}

	/// Host-only
	pub async fn send_game_setup(&mut self, setup_data: SetupState) -> Result<(), Error> {
		self.send_expect_ack(Payload::Game(Box::new(setup_data)))
			.await
	}

	pub async fn send_update_team(&mut self, player_idx: u8, team: u8) -> Result<(), Error> {
		self.send_expect_ack(Payload::UpdatePlayerTeam { player_idx, team })
			.await
	}

	/// Host-only
	pub async fn send_update_player_type(
		&mut self,
		player_idx: u8,
		is_human: bool,
	) -> Result<(), Error> {
		self.send_expect_ack(Payload::UpdatePlayerType {
			player_idx,
			is_human,
		})
		.await
	}

	pub async fn send_ready(&mut self, is_ready: bool) -> Result<(), Error> {
		self.send_expect_ack(Payload::Ready {
			is_ready,
			player_idx: 0,
		})
		.await
	}

	/// Host-only
	pub async fn send_add_player(&mut self) -> Result<(), Error> {
		self.send_expect_ack(Payload::AddPlayer).await
	}

	/// Host-only
	pub async fn send_remove_player(&mut self, player_idx: u8) -> Result<(), Error> {
		self.send_expect_ack(Payload::RemovePlayer { player_idx })
			.await
	}

	/// Host-only
	pub async fn send_swap_players(
		&mut self,
		player_idx_a: u8,
		player_idx_b: u8,
	) -> Result<(), Error> {
		self.send_expect_ack(Payload::SwapPlayers {
			player_idx_a,
			player_idx_b,
		})
		.await
	}

	/// Host-only
	pub async fn send_reinsert_player(
		&mut self,
		player_idx: u8,
		new_player_idx: u8,
	) -> Result<(), Error> {
		self.send_expect_ack(Payload::ReinsertPlayer {
			player_idx,
			new_player_idx,
		})
		.await
	}

	pub async fn send_game_event(&mut self, event: GameEvent) -> Result<(), Error> {
		self.send_expect_ack(Payload::GameEvent(event)).await
	}

	pub async fn send_report_player(&mut self, report: ReportPlayerRequest) -> Result<(), Error> {
		self.send_expect_ack(Payload::ReportPlayer(report)).await
	}
}

impl std::fmt::Debug for SetbackClient {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("ServerConnection")
			.field("ready", &self.ready)
			.finish()
	}
}
