use {
	crate::models::*,
	axum::{
		extract::{Request, State},
		http::StatusCode,
		middleware::Next,
		response::Response,
	},
	setback::API_HEADER_API_KEY,
	sha2::{Digest, Sha256},
	subtle::ConstantTimeEq,
};

pub fn sha256(s: &str) -> [u8; 32] {
	Sha256::digest(s.as_bytes()).into()
}

/// Auth middleware: reads X-Api-Key header, validates it, and injects
/// the ApiKeyRow into request extensions for downstream handlers.
pub async fn require_api_key(
	State(state): State<AppState>,
	req: Request,
	next: Next,
) -> Result<Response, StatusCode> {
	let raw = req
		.headers()
		.get(API_HEADER_API_KEY)
		.and_then(|v| v.to_str().ok())
		.ok_or(StatusCode::UNAUTHORIZED)?;
	
	let valid: bool = sha256(raw).ct_eq(state.api_key_hash.as_slice()).into();
	if !valid {
		return Err(StatusCode::UNAUTHORIZED)
	}
	
	Ok(next.run(req).await)
}