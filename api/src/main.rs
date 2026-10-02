mod constants;
mod middleware;
mod controllers;
mod models;

use {
	crate::{constants::*, controllers::*, middleware::*, models::*},
	axum::Router,
	http::{Method, header::HeaderValue},
	setback::{API_HEADER_API_KEY, SETBACK_PROXY, ENV_API_KEY},
	sqlx::mysql::MySqlPoolOptions,
	std::net::{IpAddr, Ipv4Addr, SocketAddr},
	tower_http::cors::CorsLayer};

#[tokio::main]
async fn main() {
	let state = AppState {
		db: MySqlPoolOptions::new()
			.max_connections(8)
			.connect(std::env::var(ENV_DB_URL).unwrap().as_str())
			.await
			.unwrap(),
		jwt_secret: std::env::var(ENV_JWT_SECRET).unwrap(),
		api_key_hash: api_key::sha256(&std::env::var(ENV_API_KEY).unwrap()),
	};
	sqlx::migrate!().run(&state.db).await.unwrap();
	
	let api_routes = Router::new()
		.merge(api_public_routes())
		.nest("/account", api_account_routes(state.clone()))
		.nest("/account/server", api_account_server_routes(state.clone()));
	
	let app = Router::<AppState>::new()
    	.nest("/api", api_routes)
    	.nest("/dev", dev_routes(state.clone()))
		.route_layer(
			CorsLayer::new()
				.allow_origin([
					"http://localhost:8080".parse::<HeaderValue>().unwrap(),
					"http://127.0.0.1:8080".parse::<HeaderValue>().unwrap(),
					format!("http://{SETBACK_PROXY}:8080")
						.parse::<HeaderValue>()
						.unwrap(),
				])
				.allow_credentials(true)
				.allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
				.allow_headers([
					http::header::CONTENT_TYPE,
					http::header::ACCEPT,
					http::header::AUTHORIZATION,
					http::header::HeaderName::from_static("x-requested-with"),
					http::header::HeaderName::from_static(API_HEADER_API_KEY),
				]),
		)
		.with_state(state);

	let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app.into_make_service())
		.await
		.unwrap();
}