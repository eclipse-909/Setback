use {
	crate::{middleware::*, models::AppState},
	utoipa_axum::router::OpenApiRouter as Router};

// TODO we could make an admin dashboard and other things

pub fn api_admin_routes(state: AppState) -> Router<AppState> {
	Router::new()
		// .routes(routes!()) // TODO routes go here
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), jwt::require_admin))
}