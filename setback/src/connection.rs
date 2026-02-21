pub fn get_username() -> Option<String> {
	match std::fs::read_to_string(USR_CRD) {
		Ok(username) => Some(username),
		Err(err) => {
			info!("Could not get username from usr.crd - error: {}", err);
			None
		}
	}
}

pub fn get_credentials(username: String) -> Option<LoginRequest> {
	match keyring::Entry::new(
		KEYRING_SERVICE,
		username.as_str(),
	).and_then(|entry| entry.get_password()) {
		Ok(password) => {
			info!("Retrieved credentials");
			Some(LoginRequest {username, password})
		}
		Err(err) => {
			info!("Could not get password from keyring - error: {}", err);
			None
		}
	}
}