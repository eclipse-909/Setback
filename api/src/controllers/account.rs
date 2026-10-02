use {
	crate::{middleware::jwt, models::*},
	axum::{Extension, Json, Router, extract::{Query, State}, routing::{delete, get, patch, post}},
	setback::{FeedbackRequest, LoginResponse},
	sqlx::MySqlPool,
	std::collections::HashMap,
};

type Result<T> = core::result::Result<T, http::StatusCode>;

async fn api_finish_signup(
	State(db): State<MySqlPool>,
	Query(params): Query<HashMap<String, String>>,
) -> Result<Json<LoginResponse>> {
	todo!()
}

async fn api_feedback(
	Extension(claims): Extension<Claims>,
	Json(req): Json<FeedbackRequest>,
) -> Result<()> {
	todo!()
}

async fn api_logout(
	Extension(claims): Extension<Claims>,
) -> Result<()> {
	todo!()
}

async fn api_reset_password(
	Extension(claims): Extension<Claims>,
) -> Result<()> {
	todo!()
}

async fn api_delete_account(
	Extension(claims): Extension<Claims>,
) -> Result<()> {
	todo!()
}

pub fn api_account_routes(state: AppState) -> Router<AppState> {
	Router::new()
		.route("/finish-signup", get(api_finish_signup))
		.route("/feedback", post(api_feedback))
		.route("/logout", get(api_logout))
		.route("/reset-password", patch(api_reset_password))
		.route("/delete-account", delete(api_delete_account))
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), jwt::require_jwt))
}