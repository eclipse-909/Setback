use {
	std::env,
	reqwest::{Client, header::{HeaderMap, HeaderValue}},
	crate::*,
};

pub struct Api {
	client: Client,
	base_url: String,
	url_healthcheck: String,
	url_username: String,
}

impl Api {
	pub fn new() -> Self {
		let api_key = env::var(ENV_API_KEY).unwrap();
		let mut headers = HeaderMap::new();
		let mut header_api_key = HeaderValue::from_str(&api_key).unwrap();
		header_api_key.set_sensitive(true);
		headers.insert(API_HEADER_API_KEY, header_api_key);
		
		let client = Client::builder()
			.default_headers(headers)
			.build()
			.unwrap();
		
		let base_url = env::var(ENV_API_BASE_URL).unwrap();
		let url_healthcheck = format!("{base_url}/api/healthcheck");
		let url_username = format!("{base_url}/api/account/username");
		
		Self {
			client,
			base_url,
			url_healthcheck,
			url_username,
		}
	}
	
	pub async fn healthcheck(&self) -> Result<(), Error> {
		self.client
			.get(&self.url_healthcheck)
			.send()
			.await
			.map_err(|_| Error::ApiErr)?
			.status()
			.is_success()
			.ok_or(Error::ApiErr)
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
	
	pub async fn report_player(&self, report: ReportPlayerRequest) -> Result<(), Error> {
		self.client
			.post(format!("{}/api/server/{}/report-player", self.base_url, report.target_account_id))
			.json(&report)
			.send()
			.await
			.map_err(|_| Error::ApiErr)?
			.error_for_status()
			.map_err(Error::from)?;
		Ok(())
	}
	
	pub async fn stats(&self, stats: StatsRequest, account_id: u32) -> Result<(), Error> {
		self.client
			.post(format!("{}/api/server/{}/stats", self.base_url, account_id))
			.json(&stats)
			.send()
			.await
			.map_err(|_| Error::ApiErr)?
			.error_for_status()
			.map_err(Error::from)?;
		Ok(())
	}
}