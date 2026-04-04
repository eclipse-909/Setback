mod constants;
mod controller;
mod middleware;

use {
	crate::{
		constants::*,
		controller::*,
		middleware::*,
	},
	libsetback::DNS_NAME,
	axum::{Extension, Router, routing::{get_service, get, post}},
	http::{Method, header::HeaderValue},
	std::{
		net::{IpAddr, SocketAddr, Ipv4Addr},
		path::Path,
	},
	tower_cookies::{
		CookieManagerLayer,
		cookie::Key
	},
	tower_http::{
		cors::CorsLayer,
		services::{ServeDir, ServeFile},
	}
};

#[tokio::main]
async fn main() {
	let api_routes = Router::new()
	    .route("/logout", get(api_logout))
	    .route("/username", get(api_username))
	    .route("/report", post(api_report))
	    .route("/feedback", post(api_feedback))
		.route_layer(axum::middleware::from_fn(middleware_auth))
	    .route("/signup", post(api_signup))
	    .route("/login", post(api_login))
	    .route("/healthcheck", get(api_healthcheck));

	let app = Router::new()
		.merge(Router::new().nest("/api", api_routes))
		.fallback_service(get_service(
			ServeDir::new(FRONTEND_DIR)
				.fallback(ServeFile::new(Path::new(FRONTEND_DIR).join("html/404.html"))),
		))
		.layer(Extension(
			sqlx::mysql::MySqlPoolOptions::new()
				.max_connections(8)
				.connect(std::env::var(ENV_DB_URL).unwrap().as_str())
				.await
				.unwrap()
		))
		.layer(Extension(Key::generate()))
		.layer(CookieManagerLayer::new())
		.layer(CorsLayer::new()
			.allow_origin([
				"http://localhost:8080"
					.parse::<HeaderValue>()
					.unwrap(),
				"http://127.0.0.1:8080"
					.parse::<HeaderValue>()
					.unwrap(),
				format!("http://{DNS_NAME}:8080")
					.parse::<HeaderValue>()
					.unwrap(),
			])
			.allow_credentials(true)
			.allow_methods([Method::GET, Method::POST])
			.allow_headers([
				http::header::CONTENT_TYPE,
				http::header::ACCEPT,
				http::header::AUTHORIZATION,
				http::header::HeaderName::from_static("x-requested-with"),
			])
		);

	let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080);
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app.into_make_service()).await.unwrap();
}