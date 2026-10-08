use {
	crate::{middleware::*, models::AppState},
	axum::Router,
};

// TODO we could make an admin dashboard and use swagger and other things

pub fn api_admin_routes(state: AppState) -> Router<AppState> {
	Router::new()
		// TODO routes go here
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), jwt::require_admin))
}