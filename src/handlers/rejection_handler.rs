use crate::auth::{AuthenticationError, IDPError};
use serde::Serialize;
use std::convert::Infallible;
use warp::http::StatusCode;
use warp::{reply, Rejection, Reply};

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
}

pub async fn handle_rejections(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not found".to_owned();
    } else if let Some(e @ IDPError::AuthenticationFailed) = err.find() {
        code = StatusCode::UNAUTHORIZED;
        message = e.to_string();
    } else if let Some(e @ IDPError::UsernameTaken) = err.find() {
        code = StatusCode::BAD_REQUEST;
        message = e.to_string();
    } else if let Some(e @ IDPError::AuthenticationError(_)) = err.find() {
        code = StatusCode::UNAUTHORIZED;
        message = e.to_string();
    } else if let Some(e @ IDPError::RegistrationError(_)) = err.find() {
        code = StatusCode::UNAUTHORIZED;
        message = e.to_string();
    } else if let Some(e @ IDPError::CryptoError(_)) = err.find() {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = e.to_string();
    } else if let Some(e @ AuthenticationError::InternalError) = err.find() {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = e.to_string();
    } else if let Some(e @ AuthenticationError::NoSessionIdHeader) = err.find() {
        code = StatusCode::UNAUTHORIZED;
        message = e.to_string();
    } else if let Some(e @ AuthenticationError::InvalidSessionId) = err.find() {
        code = StatusCode::UNAUTHORIZED;
        message = e.to_string();
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal Server Error".to_owned();
    }

    let json = reply::json(&ErrorResponse {
        code: code.as_u16(),
        message,
    });

    Ok(reply::with_status(json, code))
}
