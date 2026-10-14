use {
	crate::models::AppState,
	axum::{
		extract::{Request, State},
		middleware::Next,
		response::Response
	},
	http::StatusCode,
};

pub async fn require_rate_limiter(
	State(state): State<AppState>,
	req: Request,
	next: Next,
) -> Result<Response, StatusCode> {
	todo!()
}