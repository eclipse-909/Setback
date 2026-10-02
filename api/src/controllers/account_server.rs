use {
	crate::{middleware::{api_key, jwt}, models::*},
	axum::{Extension, Json, Router, extract::Query, routing::{get, post}},
	setback::{ReportPlayerRequest, UsernameResponse},
	std::collections::HashMap,
	tower::ServiceBuilder,
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

async fn api_stats(
	Extension(claims): Extension<Claims>,
	Query(params): Query<HashMap<String, String>>,
) -> Result<()> {
	todo!()
}

pub fn api_account_server_routes(state: AppState) -> Router<AppState> {
	Router::new()
		.route("/report-player", post(api_report_player))
		.route("/username", get(api_username))
		.route_layer(
			ServiceBuilder::new()
				.layer(axum::middleware::from_fn_with_state(state.clone(), jwt::require_jwt))
				.layer(axum::middleware::from_fn_with_state(state, api_key::require_api_key))
		)
}