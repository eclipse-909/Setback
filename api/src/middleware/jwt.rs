use {
	crate::models::*,
	jsonwebtoken::{DecodingKey, Validation, decode},
};

fn decode_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
	let validation = Validation::default(); // validates exp automatically
	decode::<Claims>(
		token,
		&DecodingKey::from_secret(secret.as_bytes()),
		&validation,
	).map(|data| data.claims)
}

use axum::{
	extract::{Request, State},
	http::StatusCode,
	middleware::Next,
	response::Response,
};

pub async fn require_jwt(
	State(state): State<AppState>,
	mut req: Request,
	next: Next,
) -> Result<Response, StatusCode> {
	let auth_header = req
		.headers()
		.get("authorization")
		.and_then(|v| v.to_str().ok())
		.ok_or(StatusCode::UNAUTHORIZED)?;
	
	let token = auth_header
		.strip_prefix("Bearer ")
		.ok_or(StatusCode::UNAUTHORIZED)?;
	
	let claims = decode_jwt(token, &state.jwt_secret)
		.map_err(|_| StatusCode::UNAUTHORIZED)?;
	
	req.extensions_mut().insert(claims);
	
	Ok(next.run(req).await)
}