use {
	crate::{models::AppState, middleware::api_key},
	axum::Router,
};

// TODO we could make an admin dashboard and use swagger and other things

pub fn dev_routes(state: AppState) -> Router<AppState> {
	Router::new()
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), api_key::require_api_key))
}