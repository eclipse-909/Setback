pub async fn new_pool() -> sqlx::MySqlPool {
	sqlx::mysql::MySqlPoolOptions::new()
		.max_connections(5)
		.connect(std::env::var(crate::constants::ENV_DB_URL).unwrap().as_str())
		.await
		.unwrap()
}