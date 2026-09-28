use {
	crate::models::*,
	axum::{Extension, Json, Router, routing::{delete, get, patch, post}},
	setback::FeedbackRequest,
};

type Result<T> = core::result::Result<T, http::StatusCode>;

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

pub fn api_account_routes() -> Router<AppState> {
	Router::new()
		.route("/feedback", post(api_feedback))
		.route("/logout", get(api_logout))
		.route("/reset-password", patch(api_reset_password))
		.route("/delete-account", delete(api_delete_account))
}