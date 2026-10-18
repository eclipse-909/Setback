use {
	crate::{middleware::*, models::AppState},
	axum::{Json, extract::Path},
	http::StatusCode,
	setback::*,
	utoipa_axum::{router::OpenApiRouter as Router, routes},
};

type Result<T> = core::result::Result<T, StatusCode>;

#[utoipa::path(
	post,
	path = "/{account_id}/suspend",
	params(("account_id" = u32, Path, description = "Account id")),
	request_body = SuspendRequest,
	responses((status = 200)),
	tag = "admin"
)]
async fn api_suspend(
	Path(account_id): Path<u32>,
	Json(suspend): Json<SuspendRequest>,
) -> Result<()> {
	todo!()
	// delete refresh tokens
}

// TODO we could make an admin dashboard and other things

pub fn api_admin_routes(state: AppState) -> Router<AppState> {
	Router::new()
		.routes(routes!(api_suspend))
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), jwt::require_admin))
}