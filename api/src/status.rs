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
	pub const fn s400(msg: &'static str) -> Self { Self(StatusCode::BAD_REQUEST, msg) }
	/// unauthorized
	pub const fn s401(msg: &'static str) -> Self { Self(StatusCode::UNAUTHORIZED, msg) }
	/// forbidden
	pub const fn s403(msg: &'static str) -> Self { Self(StatusCode::FORBIDDEN, msg) }
	/// not found
	pub const fn s404(msg: &'static str) -> Self { Self(StatusCode::NOT_FOUND, msg) }
	/// conflict
	pub const fn s409(msg: &'static str) -> Self { Self(StatusCode::CONFLICT, msg) }
	/// too many requests
	pub const fn s429(msg: &'static str) -> Self { Self(StatusCode::TOO_MANY_REQUESTS, msg) }
	/// internal server error
	pub const fn s500() -> Self { Self(StatusCode::INTERNAL_SERVER_ERROR, "") }
}

impl From<sqlx::Error> for Status {
	fn from(_value: sqlx::Error) -> Self {
		Self::s500()
	}
}

impl From<jsonwebtoken::errors::Error> for Status {
	fn from(_value: jsonwebtoken::errors::Error) -> Self {
		Self::s500()
	}
}