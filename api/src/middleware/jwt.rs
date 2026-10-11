use {
	crate::models::*,
	axum::{
		extract::{Request, State},
		http::StatusCode,
		middleware::Next,
		response::Response,
	},
	chrono::{Duration, Utc},
	jsonwebtoken::{DecodingKey, Validation, decode},
	jsonwebtoken::{EncodingKey, Header, encode},
};

pub fn create_jwt(
	account_id: u32,
	owner_id: u32,
	username: &str,
	email: &str,
	state: &AppState,
) -> Result<String, jsonwebtoken::errors::Error> {
	let now = Utc::now();
	let claims = Claims {
		sub: format!("{email}:{username}"),
		account_id,
		owner_id,
		iat: now.timestamp() as usize,
		exp: (now + Duration::minutes(15)).timestamp() as usize,
	};
	
	encode(
		&Header::default(), // HS256 by default
		&claims,
		&EncodingKey::from_secret(&state.jwt_secret),
	)
}

pub async fn require_auth(
	State(state): State<AppState>,
	req: Request,
	next: Next,
) -> Result<Response, StatusCode> {
	auth_middleware(state, req, next, false).await
}

pub async fn require_admin(
	State(state): State<AppState>,
	req: Request,
	next: Next,
) -> Result<Response, StatusCode> {
	auth_middleware(state, req, next, true).await
}

async fn auth_middleware(
	state: AppState,
	mut req: Request,
	next: Next,
	require_admin: bool,
) -> Result<Response, StatusCode> {
	let auth_header = req
		.headers()
		.get("authorization")
		.and_then(|v| v.to_str().ok())
		.ok_or(StatusCode::UNAUTHORIZED)?;
	
	let token = auth_header
		.strip_prefix("Bearer ")
		.ok_or(StatusCode::UNAUTHORIZED)?;
	
	let claims = decode::<Claims>(
		token,
		&DecodingKey::from_secret(&state.jwt_secret),
		&Validation::default(),
	).map(|data| data.claims)
		.map_err(|_| StatusCode::UNAUTHORIZED)?;
	
	todo!("update the last active column on the account and set it to now");
	todo!("check to see if account is deleted or owner is suspended");
	if require_admin {
		todo!("check to see if account is admin");
	}
	
	req.extensions_mut().insert(claims);
	
	Ok(next.run(req).await)
}