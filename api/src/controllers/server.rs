use {
	crate::{
		middleware::api_key,
		models::*,
	},
	axum::{Json, extract::Path},
	http::StatusCode,
	setback::*,
	utoipa_axum::{router::OpenApiRouter as Router, routes},
};

type Result<T> = core::result::Result<T, StatusCode>;

#[utoipa::path(
	post,
	path = "/{account_id}/report-player",
	params(("account_id" = u32, Path, description = "Account id")),
	request_body = ReportPlayerRequest,
	responses((status = 200)),
	tag = "server"
)]
async fn api_report_player(
	Path(account_id): Path<u32>,
	Json(report): Json<ReportPlayerRequest>,
) -> Result<()> {
	todo!()
}

#[utoipa::path(
	post,
	path = "/{account_id}/stats",
	params(("account_id" = u32, Path, description = "Account id")),
	request_body = StatsRequest,
	responses((status = 200)),
	tag = "server"
)]
async fn api_stats(
	Path(account_id): Path<u32>,
	Json(stats): Json<StatsRequest>,
) -> Result<()> {
	todo!()
}

#[utoipa::path(
	post,
	path = "/{account_id}/suspend",
	params(("account_id" = u32, Path, description = "Account id")),
	request_body = SuspendRequest,
	responses((status = 200)),
	tag = "server"
)]
async fn api_suspend(
	Path(account_id): Path<u32>,
	Json(suspend): Json<SuspendRequest>,
) -> Result<()> {
	todo!()
	// delete refresh tokens
}

pub fn api_server_routes(state: AppState) -> Router<AppState> {
	Router::new()
		.routes(routes!(api_report_player))
		.routes(routes!(api_stats))
		.routes(routes!(api_suspend))
		.route_layer(axum::middleware::from_fn_with_state(state, api_key::require_api_key))
}