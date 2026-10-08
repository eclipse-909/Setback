use {
	crate::{middleware::jwt, models::*},
	axum::{Extension, Json, Router, extract::State, routing::{delete, get, patch, post}},
	setback::*,
	sqlx::MySqlPool,
};

type Result<T> = core::result::Result<T, http::StatusCode>;

async fn api_refresh_token(
	Extension(claims): Extension<Claims>,
	State(db): State<MySqlPool>,
) -> Result<Json<RefreshTokenResponse>> {
	todo!()
	// validate refresh token in db,
	// return new auth token
}

async fn api_username(
	Extension(claims): Extension<Claims>,
) -> Result<Json<UsernameResponse>> {
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

async fn api_begin_reset_password(
	Extension(claims): Extension<Claims>,
) -> Result<()> {
	todo!()
	// create temporary secure code,
	// insert into stage password change table on duplicate key update,
	// send email with code
}

async fn api_finish_reset_password(
	Extension(claims): Extension<Claims>,
	Json(creds): Json<ChangePasswordRequest>,
) -> Result<()> {
	todo!()
	// validate code and old password,
	// change password,
	// delete refresh tokens for account
}

async fn api_delete_account(
	Extension(claims): Extension<Claims>,
) -> Result<()> {
	todo!()
	// mark account as deleted (don't actually delete),
	// set username to null,
	// actually delete all rows from refresh_token and staging_password_change where account_id matches
}

pub fn api_account_routes(state: AppState) -> Router<AppState> {
	Router::new()
		.route("/refresh-token", get(api_refresh_token))
		.route("/username", get(api_username))
		.route("/feedback", post(api_feedback))
		.route("/logout", get(api_logout))
		.route("/begin-reset-password", get(api_begin_reset_password))
		.route("/finish-reset-password", patch(api_finish_reset_password))
		.route("/delete-account", delete(api_delete_account))
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), jwt::require_auth))
}