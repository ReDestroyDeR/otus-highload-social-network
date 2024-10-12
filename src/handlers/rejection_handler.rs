use crate::auth::{AuthenticationError, IDPError};
use serde::Serialize;
use std::convert::Infallible;
use warp::http::StatusCode;
use warp::{reply, Rejection, Reply};
use crate::pool::DatabasePool;

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
}

pub async fn handle_rejections<Pool: DatabasePool>(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not found".to_owned();
    } else if let Some(e) = err.find::<IDPError<Pool::Err>>() {
        match e {
            IDPError::AuthenticationFailed => {
                code = StatusCode::UNAUTHORIZED;
                message = e.to_string();
            }
            IDPError::UsernameTaken => {
                code = StatusCode::BAD_REQUEST;
                message = e.to_string();
            }
            IDPError::AuthenticationError(_) => {
                code = StatusCode::UNAUTHORIZED;
                message = e.to_string();
            }
            IDPError::RegistrationError(_) => {
                code = StatusCode::UNAUTHORIZED;
                message = e.to_string();
            }
            IDPError::CryptoError(_) => {
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = e.to_string();
            }
        }
    } else if let Some(e) = err.find::<AuthenticationError>() {
        match e {
            AuthenticationError::InternalError => {
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = e.to_string();
            }
            AuthenticationError::NoSessionIdHeader => {
                code = StatusCode::UNAUTHORIZED;
                message = e.to_string();
            }
            AuthenticationError::InvalidSessionId => {
                code = StatusCode::UNAUTHORIZED;
                message = e.to_string();
            }
        }
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
