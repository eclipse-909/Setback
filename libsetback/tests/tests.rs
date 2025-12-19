#![feature(allocator_api)]

use {
	libsetback::*,
	reqwest::Client,
	std::sync::atomic::{AtomicU8, Ordering},
	tokio::{
		io::BufWriter,
		net::tcp::OwnedWriteHalf,
		sync::mpsc::{UnboundedReceiver, UnboundedSender},
	},
	tokio_util::sync::CancellationToken,
};

#[test]
fn test_compile_error() {
	trybuild::TestCases::new().compile_fail("tests/comp_err/*.rs");
}

pub struct MockServer {
	tcp_writer: BufWriter<OwnedWriteHalf>,
	cancellation_token: CancellationToken,
	auth_token: String,
	refresh_token: String,
	pub ssp_rx: UnboundedReceiver<Result<Ssp, Error>>,
	pub account_id: u32,
	pub ready: bool,
}

impl SetbackConnection for MockServer {
	async fn new(
		username: String,
		password: String,
		api: &impl WebsiteConnection,
		mut ssp_tx: UnboundedSender<ServerEvent>,
	) -> Result<Self, Error> {
		todo!()
	}

	async fn send_create_game(&mut self, password: Option<String>) -> Result<(), Error> {
		todo!()
	}

	async fn send_join_game(
		&mut self,
		code: String,
		password: Option<String>,
	) -> Result<(), Error> {
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
	pub ready: bool,
}

impl Connection for MockClient {}

pub struct MockApi;

impl WebsiteConnection for MockApi {
	fn new_api() -> Self {
		Self
	}
	fn as_client(&self) -> &Client {
		panic!("Do not call this function from a unit test")
	}
	async fn login(&self, _credentials: &LoginRequest) -> Result<LoginResponse, Error> {
		Ok(LoginResponse {
			auth_token: String::new(),
			refresh_token: String::new(),
			account_id: 1,
		})
	}
	async fn logout(&self, _auth_token: &str) -> Result<(), Error> {
		Ok(())
	}
	async fn id(&self, _auth_token: &str) -> Result<u32, Error> {
		Ok(1)
	}
	async fn update(&self) -> Result<Version, Error> {
		Ok(SETBACK_VERSION)
	}
}

// #[test]
fn test_ai_only() {
	todo!()
}

// #[test]
fn test_mock_multiplayer() {
	todo!()
}

#[test]
fn test_default() {
	let arena = Arena::new();
	let one = arena.alloc(1);
	let two = Box::new_in(2, arena.allocator());
	assert_eq!(*one + *two, 3);
}

#[test]
fn test_many_allocations() {
	const ARR1_LEN: usize = 500;
	const ARR2_LEN: usize = 20;
	const ARR3_LEN: usize = 1024;
	let arena = Arena::new();
	let arr1 = arena.alloc([0; ARR1_LEN]);
	let arr2 = arena.alloc([0; ARR2_LEN]);
	let arr3 = arena.alloc([0; ARR3_LEN]);
	for (i, v) in arr1.iter_mut().enumerate() {
		*v = i;
	}
	for (i, v) in arr2.iter_mut().enumerate() {
		*v = i;
	}
	for (i, v) in arr3.iter_mut().enumerate() {
		*v = i;
	}
	for (i, v) in arr1.iter().enumerate() {
		assert_eq!(*v, i);
	}
	for (i, v) in arr2.iter().enumerate() {
		assert_eq!(*v, i)
	}
	for (i, v) in arr3.iter().enumerate() {
		assert_eq!(*v, i)
	}
	drop(arena);
}

#[test]
fn test_vec_in_arena() {
	let arena = Arena::new();
	let mut vec1 = Vec::new_in(arena.allocator());
	let mut vec2 = Vec::new_in(arena.allocator());
	let mut vec3 = Vec::new_in(arena.allocator());
	for i in 0..32 {
		vec1.push(i);
	}
	for i in 0..32 {
		vec2.push(i);
	}
	for i in 0..32 {
		vec3.push(i);
	}
	vec1.shrink_to_fit();
	vec2.shrink_to_fit();
	vec3.shrink_to_fit();
	for i in 0..32 {
		assert_eq!(vec1[i], i);
		assert_eq!(vec2[i], i);
		assert_eq!(vec3[i], i);
	}
}

static DROP_COUNT: AtomicU8 = AtomicU8::new(0);
struct DropStruct(usize);
impl Drop for DropStruct {
	fn drop(&mut self) {
		DROP_COUNT.fetch_add(1, Ordering::Relaxed);
	}
}

#[test]
fn test_dropping() {
	let arena: Arena = Arena::new();
	let vec_ref = arena.alloc(DropStruct(0));
	vec_ref.0 = 1;
	let mut vec_box = Box::new_in(DropStruct(0), arena.allocator());
	vec_box.0 = 1;
	drop(vec_box);
	drop(arena);
	assert_eq!(DROP_COUNT.load(Ordering::Relaxed), 2);
}
