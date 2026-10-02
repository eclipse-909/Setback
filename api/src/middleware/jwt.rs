use {
	crate::models::*,
	jsonwebtoken::{DecodingKey, Validation, decode},
	axum::{
		extract::{Request, State},
		http::StatusCode,
		middleware::Next,
		response::Response,
	},
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
	
	let claims = decode::<Claims>(
		token,
		&DecodingKey::from_secret(state.jwt_secret.as_bytes()),
		&Validation::default(),
	).map(|data| data.claims)
		.map_err(|_| StatusCode::UNAUTHORIZED)?;
	
	todo!("check to see if account is deleted or owner is suspended");
	todo!("update the last active column on the account and set it to now");
	
	req.extensions_mut().insert(claims);
	
	Ok(next.run(req).await)
}