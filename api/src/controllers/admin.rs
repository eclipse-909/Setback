use {
	crate::{middleware::*, models::AppState},
	std::{
		fs::{self, File},
		io::Write,
		path::PathBuf,
	},
	utoipa::OpenApi,
	utoipa_axum::router::OpenApiRouter as Router,
	utoipa_swagger_ui::SwaggerUi
};

#[derive(OpenApi)]
#[openapi(
	info(
		title="Setback API",
		description = "The public API documentation for the Setback game."
	),
	nest(
    	// (path="/api/account", api=AccountApiDoc),
    	// (path="/api/chat", api=ChatApiDoc),
    	// (path="/api/itinerary", api=ItineraryApiDoc)
	),
	servers(
    	(url="http://localhost:8000", description="Local host server for development"),
	)
)]
struct ApiDoc;

/// Merges swagger with the current routes
pub fn merge_swagger(router: Router<AppState>) -> axum::Router<AppState> {
	let doc = ApiDoc::openapi();
	let docs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("api/openapi");
	fs::create_dir_all(&docs_path).unwrap();
	let mut file = File::create(docs_path.join("openapi.json")).unwrap();
	file.write_all(doc.to_pretty_json().unwrap().as_bytes())
		.unwrap();
	let (router, api) = Router::with_openapi(doc)
		.merge(router)
		.split_for_parts();
	router.merge(SwaggerUi::new("/swagger").url("/api/openapi/openapi.json", api.clone()))
}

// TODO we could make an admin dashboard and use swagger and other things

pub fn api_admin_routes(state: AppState) -> Router<AppState> {
	Router::new()
		// TODO routes go here
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), jwt::require_admin))
}