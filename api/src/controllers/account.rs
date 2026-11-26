use {
	crate::{middleware::jwt, models::*, status::*},
	axum::{Extension, Json, extract::State},
	setback::*,
	sqlx::PgPool,
	utoipa_axum::{router::OpenApiRouter as Router, routes},
};

type Result<T> = core::result::Result<T, Status>;

#[utoipa::path(
	get,
	path = "/username",
	summary = "Get username for server",
	description = "The server uses this endpoint to authenticate clients and get their username.\
 This endpoint checks if the account is suspended and will return a 403",
	responses(
        (status = 200, body = UsernameResponse),
        (status = 401),
        (status = 403),
        (status = 500),
	),
	tag = "account"
)]
async fn api_username(
	State(db): State<PgPool>,
	Extension(claims): Extension<Claims>,
) -> Result<Json<UsernameResponse>> {
	if claims.suspended {
		return Err(Status::s403("Account suspended"));
	}
	let username = sqlx::query_scalar!(
		r#" select username as "username!" from account
			where account_id = $1 and deleted_at is not null
		"#,
		claims.account_id
	).fetch_optional(&db).await?;
	let Some(username) = username else {
		return Err(Status::s401("Username not found"));
	};
	Ok(Json(UsernameResponse {
		username,
		profile_id: claims.profile_id as u32,
		account_id: claims.account_id as u32,
	}))
}

#[utoipa::path(
	post,
	path = "/feedback",
	request_body = FeedbackRequest,
	responses((status = 200)),
	tag = "account"
)]
async fn api_feedback(
	State(db): State<PgPool>,
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
	State(db): State<PgPool>,
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
	State(db): State<PgPool>,
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
	State(db): State<PgPool>,
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
	State(db): State<PgPool>,
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
	State(db): State<PgPool>,
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
	State(db): State<PgPool>,
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