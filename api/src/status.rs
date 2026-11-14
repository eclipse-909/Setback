use {
	axum::response::{IntoResponse, Response},
	http::StatusCode,
};

pub struct Status(StatusCode, &'static str);

impl IntoResponse for Status {
	fn into_response(self) -> Response {
		(self.0, self.1).into_response()
	}
}

impl Status {
	/// bad request
	pub fn s400(msg: &'static str) -> Self { Self(StatusCode::BAD_REQUEST, msg) }
	/// unauthorized
	pub fn s401(msg: &'static str) -> Self { Self(StatusCode::UNAUTHORIZED, msg) }
	/// forbidden
	pub fn s403(msg: &'static str) -> Self { Self(StatusCode::FORBIDDEN, msg) }
	/// not found
	pub fn s404(msg: &'static str) -> Self { Self(StatusCode::NOT_FOUND, msg) }
	/// conflict
	pub fn s409(msg: &'static str) -> Self { Self(StatusCode::CONFLICT, msg) }
	/// too many requests
	pub fn s429(msg: &'static str) -> Self { Self(StatusCode::TOO_MANY_REQUESTS, msg) }
	/// internal server error
	pub fn s500(msg: &'static str) -> Self { Self(StatusCode::INTERNAL_SERVER_ERROR, msg) }
}

impl From<sqlx::Error> for Status {
	fn from(_value: sqlx::Error) -> Self {
		Self::s500("")
	}
}