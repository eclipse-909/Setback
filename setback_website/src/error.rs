use axum::http::StatusCode;

pub type Result<T> = Result<Json<T>, StatusCode>;