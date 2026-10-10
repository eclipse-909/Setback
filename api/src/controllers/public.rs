use {
	crate::models::*,
	axum::{
		Json, Router,
		extract::{Query, State},
		routing::{get, post},
	},
	http::StatusCode,
	setback::{LoginRequest, LoginResponse, SignupRequest},
	sqlx::MySqlPool,
	std::collections::HashMap,
};

type Result<T> = core::result::Result<T, StatusCode>;

async fn api_begin_signup(
	State(db): State<MySqlPool>,
	Json(creds): Json<SignupRequest>,
) -> Result<()> {
	todo!()
	// validate request,
	// make sure email is not suspended,
	// make sure username is appropriate and doesn't exist in account,
	// make sure owner has fewer than 3 accounts,
	// create temporary secure code,
	// insert into stage account table on duplicate key update,
	// send email with code
}

async fn api_finish_signup(
	State(db): State<MySqlPool>,
	Query(params): Query<HashMap<String, String>>,
) -> Result<Json<LoginResponse>> {
	todo!()
	// remove from staging account table,
	// insert into account table and owner table if new email,
	// create new jwt and refresh token,
	// return response
}

async fn api_login(
	State(db): State<MySqlPool>,
	Json(creds): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
	todo!()
	// validate credentials
	// create new jwt and refresh token,
	// return response
}

pub fn api_public_routes() -> Router<AppState> {
	Router::new()
		.route("/begin-signup", post(api_begin_signup))
		.route("/finish-signup", get(api_finish_signup))
		.route("/login", post(api_login))
}