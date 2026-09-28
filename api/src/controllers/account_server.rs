use {
	crate::models::*,
	axum::{Extension, Json, Router, routing::{get, post}},
	setback::{ReportPlayerRequest, UsernameResponse},
};

type Result<T> = core::result::Result<T, http::StatusCode>;

async fn api_report_player(
	Extension(claims): Extension<Claims>,
	Json(req): Json<ReportPlayerRequest>,
) -> Result<()> {
	todo!()
}

async fn api_username(Extension(claims): Extension<Claims>) -> Result<Json<UsernameResponse>> {
	todo!()
}

pub fn api_account_server_routes() -> Router<AppState> {
	Router::new()
		.route("/report-player", post(api_report_player))
		.route("/username", get(api_username))
}