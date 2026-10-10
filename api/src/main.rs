mod constants;
mod middleware;
mod controllers;
mod models;
mod util;

use {
	crate::{constants::*, controllers::*, middleware::*, models::*},
	axum::{Router, routing::get},
	http::StatusCode,
	setback::*,
	sqlx::mysql::MySqlPoolOptions,
	std::env,
};

#[tokio::main]
async fn main() {
	init_tracing!();
	
	let state = AppState {
		db: MySqlPoolOptions::new()
			.max_connections(8)
			.connect(env::var(ENV_DB_URL).unwrap().as_str())
			.await
			.unwrap(),
		jwt_secret: env::var(ENV_JWT_SECRET).unwrap(),
		api_key_hash: api_key::sha256(&env::var(ENV_API_KEY).unwrap()),
	};
	sqlx::migrate!().run(&state.db).await.unwrap();
	
	let api_routes = Router::new()
		.merge(api_public_routes())
		.nest("/account", api_account_routes(state.clone()))
    	.nest("/admin", api_admin_routes(state.clone()))
		.nest("/server", api_server_routes(state.clone()))
		.route_layer(http_trace_layer())
		.route("/healthcheck", get(StatusCode::OK));
	
	let app = Router::<AppState>::new()
    	.nest("/api", api_routes)
		.with_state(state);

	let addr = env::var(ENV_API_BIND_ADDR).unwrap();
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app.into_make_service())
		.await
		.unwrap();
}