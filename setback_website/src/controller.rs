use {
	crate::{
		error::Result,
		middleware::User,
		models::*,
	},
	axum::{Extension, Json, extract::Multipart},
	libsetback::{LoginRequest, LoginResponse, UsernameResponse, RenewRequest, RenewResponse},
	sqlx::MySqlPool,
	tower_cookies::{Cookies, cookie::Key}
};

pub async fn api_login(
	cookies: Cookies,
	Extension(key): Extension<Key>,
	Extension(db): Extension<MySqlPool>,
	Json(req): Json<LoginRequest>,
) -> Result<LoginResponse> {
	todo!()
}

pub async fn api_signup(
	cookies: Cookies,
	Extension(key): Extension<Key>,
	Extension(db): Extension<MySqlPool>,
	Json(req): Json<LoginRequest>,
) -> Result<LoginResponse> {
	todo!()
}

pub async fn api_logout(
	cookies: Cookies,
	Extension(key): Extension<Key>,
	Extension(user): Extension<User>,
) -> Result<()> {
	todo!()
}

//TODO: only the server needs this so maybe we should restrict access and not make it public
pub async fn api_username(
	Extension(user): Extension<User>,
) -> Result<UsernameResponse> {
	todo!()
}

pub async fn api_refresh(
	cookies: Cookies,
	Extension(key): Extension<Key>,
	Extension(user): Extension<User>,
	Json(req): Json<RenewRequest>,
) -> Result<RenewResponse> {
	todo!()
}

pub async fn api_feedback(
	Extension(user): Extension<User>,
	multipart: Multipart,
) -> Result<()> {
	todo!()
}