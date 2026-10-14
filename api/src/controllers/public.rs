use {
	crate::models::*,
	axum::{Json, extract::{Query, State, ConnectInfo}},
	http::{StatusCode, HeaderMap},
	setback::*,
	sqlx::MySqlPool,
	std::{
		collections::HashMap,
		net::{IpAddr, SocketAddr},
	},
	utoipa_axum::{router::OpenApiRouter as Router, routes},
};

type Result<T> = core::result::Result<T, StatusCode>;

fn client_ip(headers: &HeaderMap, peer: SocketAddr) -> IpAddr {
	headers
		.get("x-forwarded-for")
		.and_then(|v| v.to_str().ok())
		// rightmost entry = the one appended by YOUR proxy (not client-controlled)
		.and_then(|v| v.rsplit(',').next())
		.and_then(|s| s.trim().parse::<IpAddr>().ok())
		.unwrap_or_else(|| peer.ip()) // fallback: direct connection
}

#[utoipa::path(
	post,
	path = "/signup",
	request_body = SignupRequest,
	responses((status = 200)),
	tag = "public"
)]
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

#[utoipa::path(
	get,
	path = "/signup",
	params(("token" = String, Query)),
	responses((status = 201, body = LoginResponse)),
	tag = "public"
)]
async fn api_finish_signup(
	State(db): State<MySqlPool>,
	Query(params): Query<HashMap<String, String>>,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	headers: HeaderMap,
) -> Result<Json<LoginResponse>> {
	todo!()
	// remove from staging account table,
	// insert into account table and owner table if new email,
	// create new jwt and refresh token,
	// return response
}

#[utoipa::path(
	post,
	path = "/login",
	request_body = LoginRequest,
	responses((status = 200, body = LoginResponse)),
	tag = "public"
)]
async fn api_login(
	State(db): State<MySqlPool>,
	ConnectInfo(peer): ConnectInfo<SocketAddr>,
	headers: HeaderMap,
	Json(creds): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
	todo!()
	// validate credentials
	// create new jwt and refresh token,
	// return response
}

#[utoipa::path(
	post,
	path = "/refresh-token",
	request_body = RefreshTokenRequest,
	responses((status = 200, body = RefreshTokenResponse)),
	tag = "public"
)]
async fn api_refresh_token(
	State(db): State<MySqlPool>,
	Json(creds): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>> {
	todo!()
	// validate refresh token in db,
	// return new auth token
}

pub fn api_public_routes() -> Router<AppState> {
	Router::new()
		.routes(routes!(api_begin_signup, api_finish_signup))
		.routes(routes!(api_login))
		.routes(routes!(api_refresh_token))
}