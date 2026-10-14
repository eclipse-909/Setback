use {
	crate::{middleware::jwt, models::*},
	axum::{Extension, Json},
	setback::*,
	utoipa_axum::{router::OpenApiRouter as Router, routes},
};

type Result<T> = core::result::Result<T, http::StatusCode>;

#[utoipa::path(
	get,
	path = "/username",
	responses(
        (status = 200, body = UsernameResponse),
	),
	tag = "account"
)]
async fn api_username(
	Extension(claims): Extension<Claims>,
) -> Result<Json<UsernameResponse>> {
	todo!()
}

#[utoipa::path(
	post,
	path = "/feedback",
	request_body = FeedbackRequest,
	responses((status = 200)),
	tag = "account"
)]
async fn api_feedback(
	Extension(claims): Extension<Claims>,
	Json(req): Json<FeedbackRequest>,
) -> Result<()> {
	todo!()
}

#[utoipa::path(
	get,
	path = "/logout",
	responses((status = 200)),
	tag = "account"
)]
async fn api_logout(
	Extension(claims): Extension<Claims>,
) -> Result<()> {
	todo!()
}

#[utoipa::path(
	get,
	path = "/reset-password",
	responses((status = 200)),
	tag = "account"
)]
async fn api_begin_reset_password(
	Extension(claims): Extension<Claims>,
) -> Result<()> {
	todo!()
	// create temporary secure code,
	// insert into stage password change table on duplicate key update,
	// send email with code
}

#[utoipa::path(
	post,
	path = "/reset-password",
	request_body = ChangePasswordRequest,
	responses((status = 200)),
	tag = "account"
)]
async fn api_finish_reset_password(
	Extension(claims): Extension<Claims>,
	Json(creds): Json<ChangePasswordRequest>,
) -> Result<()> {
	todo!()
	// validate code and old password,
	// change password,
	// delete refresh tokens for account
}

#[utoipa::path(
	get,
	path = "/change-email",
	responses((status = 200)),
	tag = "account"
)]
async fn api_begin_change_email(
	Extension(claims): Extension<Claims>,
) -> Result<()> {
	todo!()
	// create temporary secure code,
	// insert into stage email change table on duplicate key update,
	// send email with code to new email
}

#[utoipa::path(
	post,
	path = "/change-email",
	request_body = ChangeEmailRequest,
	responses((status = 200)),
	tag = "account"
)]
async fn api_finish_change_email(
	Extension(claims): Extension<Claims>,
	Json(creds): Json<ChangeEmailRequest>,
) -> Result<()> {
	todo!()
	// validate code,
	// change email,
	// delete refresh tokens for account
}

#[utoipa::path(
	delete,
	path = "/delete-account",
	responses((status = 200)),
	tag = "account"
)]
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
		.routes(routes!(api_username))
		.routes(routes!(api_feedback))
		.routes(routes!(api_logout))
		.routes(routes!(api_begin_reset_password, api_finish_reset_password))
		.routes(routes!(api_begin_change_email, api_finish_change_email))
		.routes(routes!(api_delete_account))
		.route_layer(axum::middleware::from_fn_with_state(state.clone(), jwt::require_auth))
}