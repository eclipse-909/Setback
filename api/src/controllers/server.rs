use {
	crate::{
		middleware::api_key,
		models::*,
	},
	axum::{
		Json, Router,
		routing::{patch, post},
		extract::Path,
	},
	http::StatusCode,
	setback::*,
};

type Result<T> = core::result::Result<T, StatusCode>;

async fn api_report_player(
	Path(account_id): Path<u32>,
	Json(report): Json<ReportPlayerRequest>,
) -> Result<()> {
	todo!()
}

async fn api_stats(
	Path(account_id): Path<u32>,
	Json(stats): Json<StatsRequest>,
) -> Result<()> {
	todo!()
}

pub fn api_server_routes(state: AppState) -> Router<AppState> {
	Router::new()
		.route("/{account_id}/report-player", post(api_report_player))
		.route("/{account_id}/stats", patch(api_stats))
		.route_layer(axum::middleware::from_fn_with_state(state, api_key::require_api_key))
}