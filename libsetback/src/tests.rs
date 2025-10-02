#[cfg(test)]
pub mod testing {
	use {
		crate::{
			connection::*,
			constants::*,
			items::*,
			models::*,
			setup_data::*,
			ssp::*
		},
		reqwest::Client,
		tokio::{
			io::BufWriter,
			net::tcp::OwnedWriteHalf,
			sync::mpsc::{UnboundedSender, UnboundedReceiver}
		},
		tokio_util::sync::CancellationToken
	};

	pub struct MockServer {
		tcp_writer: BufWriter<OwnedWriteHalf>,
		cancellation_token: CancellationToken,
		auth_token: String,
		refresh_token: String,
		pub ssp_rx: UnboundedReceiver<Result<Ssp, Error>>,
		pub account_id: u32,
		pub ready: bool
	}

	impl SetbackConnection for MockServer {
		async fn new(
			username: String,
			password: String,
			api: &impl WebsiteConnection,
			mut ssp_tx: UnboundedSender<ServerEvent>
		) -> Result<Self, Error> {
			todo!()
		}

		async fn send_create_game(&mut self, password: Option<String>) -> Result<(), Error> {
			todo!()
		}

		async fn send_join_game(&mut self, code: String, password: Option<String>) -> Result<(), Error> {
			todo!()
		}

		async fn send_search_games(&mut self, filter: GameFilter) -> Result<(), Error> {
			todo!()
		}

		async fn send_game_meta(&mut self, setup_data: SetupData) -> Result<(), Error> {
			todo!()
		}

		async fn send_update_team(&mut self, index: u8, team: u8) -> Result<(), Error> {
			todo!()
		}

		async fn send_update_player_type(&mut self, index: u8, human: bool) -> Result<(), Error> {
			todo!()
		}

		async fn send_ready(&mut self, is_ready: bool, index: u8) -> Result<(), Error> {
			todo!()
		}

		async fn send_game_event(&mut self, event: GameEvent) -> Result<(), Error> {
			todo!()
		}

		async fn send_add_player(&mut self) -> Result<(), Error> {
			todo!()
		}

		async fn send_remove_player(&mut self, index: u8) -> Result<(), Error> {
			todo!()
		}

		async fn refresh_token(&mut self, api: &impl WebsiteConnection) -> Result<(), Error> {
			todo!()
		}
	}

	impl Connection for MockServer {}

	pub struct MockClient {
		tcp_writer: BufWriter<OwnedWriteHalf>,
		pub ssp_rx: UnboundedReceiver<Result<Ssp, Error>>,
		pub cancellation_token: CancellationToken,
		pub account_id: u32,
		pub ready: bool
	}

	impl Connection for MockClient {}

	pub struct MockApi;

	impl WebsiteConnection for MockApi {
		fn new_api() -> Self {Self}
		fn as_client(&self) -> &Client {panic!("Do not call this function from a unit test")}
		async fn login(&self, _credentials: &LoginRequest) -> Result<LoginResponse, Error> {
			Ok(LoginResponse {
				auth_token: String::new(),
				refresh_token: String::new(),
				account_id: 1,
			})
		}
		async fn logout(&self, _auth_token: &str) -> Result<(), Error> {Ok(())}
		async fn id(&self, _auth_token: &str) -> Result<u32, Error> {Ok(1)}
		async fn update(&self) -> Result<Version, Error> {Ok(SETBACK_VERSION)}
	}
	
	#[test]
	fn ai_only() {
		todo!()
	}
	
	#[test]
	fn mock_multiplayer() {
		todo!()
	}
}