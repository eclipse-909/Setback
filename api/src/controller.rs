use {
	crate::middleware::User,
	argon2::{
		Argon2,
		password_hash::{
			PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
		},
	},
	axum::{Extension, Json},
	libsetback::{
		FeedbackRequest, LoginRequest, LoginResponse, ReportPlayerRequest, SignupRequest,
		UsernameResponse,
	},
	sqlx::MySqlPool,
	tower_cookies::{Cookies, cookie::Key},
};

type Result<T> = core::result::Result<T, http::StatusCode>;

pub async fn api_signup(
	cookies: Cookies,
	Extension(key): Extension<Key>,
	Extension(db): Extension<MySqlPool>,
	Json(req): Json<SignupRequest>,
) -> Result<Json<LoginResponse>> {
	todo!()
}

pub async fn api_login(
	cookies: Cookies,
	Extension(key): Extension<Key>,
	Extension(db): Extension<MySqlPool>,
	Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>> {
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
pub async fn api_username(Extension(user): Extension<User>) -> Result<Json<UsernameResponse>> {
	todo!()
}

//TODO this endpoint should be private.
// Only the server should be able to call this.
pub async fn api_report(
	Extension(user): Extension<User>,
	Json(req): Json<ReportPlayerRequest>,
) -> Result<()> {
	todo!()
}

pub async fn api_feedback(
	Extension(user): Extension<User>,
	Json(req): Json<FeedbackRequest>,
) -> Result<()> {
	todo!()
}

pub async fn api_healthcheck() -> Result<()> {
	Ok(())
}
