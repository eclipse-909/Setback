use {
	crate::{models::*, util::*},
	axum::{
		RequestExt,
		extract::{ConnectInfo, Request, State},
		http::StatusCode,
		middleware::Next,
		response::Response,
	},
	chrono::{Duration, Utc},
	jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode},
};

pub fn create_jwt(
	account_id: i32,
	profile_id: i32,
	suspended: bool,
	username: &str,
	email: &str,
	state: &AppState,
) -> Result<String, jsonwebtoken::errors::Error> {
	let now = Utc::now();
	let claims = Claims {
		sub: format!("{email}:{username}"),
		account_id,
		profile_id,
		suspended,
		iat: now.timestamp() as usize,
		exp: (now + Duration::minutes(15)).timestamp() as usize,
	};
	
	encode(
		&Header::default(), // HS256 by default
		&claims,
		&EncodingKey::from_secret(&state.jwt_secret)?,
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
		&DecodingKey::from_secret(&state.jwt_secret)
			.map_err(|_| StatusCode::UNAUTHORIZED)?,
		&Validation::default(),
	).map(|data| data.claims)
		.map_err(|_| StatusCode::UNAUTHORIZED)?;
	
	let ConnectInfo(peer) = req.extract_parts()
		.await
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
	let ip_addr = client_ip(req.headers(), peer);
	
	update_account_activity(claims.account_id, claims.profile_id, ip_addr, &state.db)
		.await
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
		.ok_or(StatusCode::UNAUTHORIZED)?;
	
	// user is authenticated at this point
	
	if require_admin {
		sqlx::query_scalar!(
			r#" select 1 from account
				where account_id = $1 and is_admin
			"#,
			claims.account_id
		).fetch_one(&state.db)
			.await
			.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
			.ok_or(StatusCode::FORBIDDEN)?;
	}
	
	req.extensions_mut().insert(claims);
	Ok(next.run(req).await)
}