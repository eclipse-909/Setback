use {
	crate::models::*,
	axum::{Json, Router, extract::State, routing::{get, post}},
	chrono::{Duration, Utc},
	http::StatusCode,
	jsonwebtoken::{EncodingKey, Header, encode},
	setback::{LoginRequest, LoginResponse, SignupRequest},
	sqlx::MySqlPool,
};

type Result<T> = core::result::Result<T, StatusCode>;

fn create_jwt(
	account_id: u32,
	owner_id: u32,
	username: &str,
	email: &str,
	secret: &str
) -> std::result::Result<String, jsonwebtoken::errors::Error> {
	let now = Utc::now();
	let claims = Claims {
		sub: format!("{email}:{username}"),
		account_id,
		owner_id,
		iat: now.timestamp() as usize,
		exp: (now + Duration::hours(24)).timestamp() as usize,
	};
	
	encode(
		&Header::default(), // HS256 by default
		&claims,
		&EncodingKey::from_secret(secret.as_bytes()),
	)
}

async fn api_begin_signup(
	State(db): State<MySqlPool>,
	Json(req): Json<SignupRequest>,
) -> Result<()> {
	todo!()
}

async fn api_login(
	State(db): State<MySqlPool>,
	Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
	todo!()
}

pub fn api_public_routes() -> Router<AppState> {
	Router::new()
		.route("/healthcheck", get(StatusCode::OK))
		.route("/begin-signup", post(api_begin_signup))
		.route("/login", post(api_login))
}