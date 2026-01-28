use {
	crate::{
		error::Result,
		middleware::User
	},
	axum::{Extension, Json},
	libsetback::{LoginRequest, LoginResponse},
	sqlx::MySqlPool,
	tower_cookies::{Cookies, cookie::Key}
};

pub async fn api_login(
	cookies: Cookies,
	Extension(key): Extension<Key>,
	Extension(db): Extension<MySqlPool>,
	Json(req): Json<LoginRequest>
) -> Result<LoginResponse> {
	todo!()
}

pub async fn api_signup(
	cookies: Cookies,
	Extension(key): Extension<Key>,
	Extension(db): Extension<MySqlPool>,
	Json(req): Json<LoginRequest>
) -> Result<LoginResponse> {
	todo!()
}

pub async fn api_logout(
	cookies: Cookies,
	Extension(key): Extension<Key>,
	Extension(user): Extension<User>,
) -> Result<LoginResponse> {
	todo!()
}