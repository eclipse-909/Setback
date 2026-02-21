mod api;
mod constants;
mod controller;
mod db;
mod error;
mod jwt;
mod middleware;
mod session;
mod sql;
mod models;

use {
	axum::{Extension, Router, routing::get_service},
	http::{Method, header::HeaderValue},
	std::{
		net::{IpAddr, SocketAddr},
		path::Path,
		str::FromStr
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
	dotenvy::dotenv().ok();

	let api_routes = Router::new()
	    .route("/login", post(api_login))
	    .route("/signup", post(api_signup))
	    .route("/logout", get(api_logout))
	    .route("/username", get(api_username))
	    .route("/refresh", post(api_refresh));

	let app = Router::new()
		.merge(Router::new().nest("/api", api_routes))
		.fallback_service(get_service(
			ServeDir::new(CLIENT_DIR)
				.fallback(ServeFile::new(Path::new(CLIENT_DIR).join("html/file_not_found.html"))),
		))
		.layer(Extension(db::new_pool()))
		.layer(Extension(Key::generate()))
		.layer(CookieManagerLayer::new())
		.layer(CorsLayer::new()
			.allow_origin(
				front_end_url
					.parse::<HeaderValue>()
					.expect("Invalid frontend_url format"),
			)
			.allow_credentials(true)
			.allow_methods([Method::GET, Method::POST, Method::DELETE])
			.allow_headers([
				http::header::CONTENT_TYPE,
				http::header::ACCEPT,
				http::header::AUTHORIZATION,
				http::header::HeaderName::from_static("x-requested-with"),
			])
		);

	let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8080);
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app.into_make_service()).await.unwrap();
}