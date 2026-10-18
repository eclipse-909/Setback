use {
	crate::*,
	reqwest::Client,
	std::env,
};

pub struct Api {
	client: Client,
	url_username: String,
}

impl Api {
	pub fn new() -> Self {
		Self {
			client: Client::new(),
			url_username: format!("{}/api/account/username", env::var(ENV_API_BASE_URL).unwrap()),
		}
	}
	
	pub async fn get_username(&self, auth_token: &str) -> Result<UsernameResponse, Error> {
		self.client
			.get(&self.url_username)
			.bearer_auth(auth_token)
			.send()
			.await
			.map_err(|_| Error::ApiErr)?
			.error_for_status()
			.map_err(Error::from)?
			.json::<UsernameResponse>()
			.await
			.map_err(|_| NetError::ServerError.into())
	}
}