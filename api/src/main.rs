mod constants;
mod middleware;
mod controllers;
mod models;
mod util;

use {
	crate::{constants::*, controllers::*, middleware::*, models::*},
	axum::routing::get,
	base64::{Engine, engine::general_purpose::STANDARD},
	http::StatusCode,
	setback::*,
	sqlx::{mysql::MySqlPoolOptions, Executor},
	std::{env, fs, path::PathBuf, net::SocketAddr},
	utoipa::OpenApi,
	utoipa_axum::router::OpenApiRouter as Router,
	utoipa_swagger_ui::SwaggerUi,
};

#[tokio::main]
async fn main() {
	init_tracing!();
	
	let state = AppState {
		db: MySqlPoolOptions::new()
			.max_connections(8)
			.after_connect(|conn, _meta| Box::pin(async move {
				conn.execute("set time_zone = '+00:00';").await?;
				Ok(())
			}))
			.connect(env::var(ENV_DB_URL).unwrap().as_str())
			.await
			.unwrap(),
		jwt_secret: STANDARD.decode(env::var(ENV_JWT_SECRET).unwrap()).unwrap(),
		api_key_hash: api_key::sha256(&env::var(ENV_API_KEY).unwrap()),
	};
	
	sqlx::migrate!().run(&state.db).await.unwrap();
	
	let router = Router::new()
		.merge(api_public_routes())
		.nest("/account", api_account_routes(state.clone()))
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), rate_limit::require_rate_limiter))
    	.nest("/admin", api_admin_routes(state.clone()))
		.nest("/server", api_server_routes(state.clone()))
		.route_layer(http_trace_layer())
		.route("/healthcheck", get(StatusCode::OK));
	
	let router = Router::new()
		.nest("/api", router);
	
	let router = if env::var(ENV_SETBACK_ENV).is_ok() {
		router.into()
	} else {
		// merging swagger needs to be the last modification to the router
		merge_swagger(router)
	};
	
	let app = axum::Router::<AppState>::new()
    	.merge(router)
		.with_state(state);

	let addr = env::var(ENV_API_BIND_ADDR).unwrap();
	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
		.await
		.unwrap();
}

#[derive(OpenApi)]
#[openapi(
	info(
		title="Setback API",
		description = "The public API documentation for the Setback game."
	),
	servers((url="http://localhost:8000", description="Local host server for development"))
)]
struct ApiDoc;

/// Merges swagger with the current routes
fn merge_swagger(router: Router<AppState>) -> axum::Router<AppState> {
	let (router, api) = Router::with_openapi(ApiDoc::openapi())
		.merge(router)
		.split_for_parts();
	
	let docs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("api/openapi");
	fs::create_dir_all(&docs_path).unwrap();
	fs::write(docs_path.join("openapi.json"), api.to_pretty_json().unwrap()).unwrap();
	
	router.merge(SwaggerUi::new("/swagger").url("/api/openapi/openapi.json", api.clone()))
}