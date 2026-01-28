#![feature(allocator_api)]

use {libsetback::*, reqwest::Client};

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
			auth_token: String::from(TEST_AUTH_TOKEN),
			refresh_token: String::from(TEST_AUTH_TOKEN),
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
