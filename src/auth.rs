use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Database, Error as SQLXError, Transaction};
use thiserror::Error;
use warp::{reply, Reply};
use warp::http::StatusCode;

use crate::domain::protocol::ToReply;

pub struct Session {
    pub session_id: String,
    pub expires: DateTime<Utc>,
}

#[async_trait]
pub trait IDPContext<DB: Database>
where
    Self: Send + Sync,
{
    async fn authenticate(
        &self,
        tx: &mut Transaction<'_, DB>,
        login: &str,
        password: &str,
    ) -> Result<Session, IDPError>;
    async fn add_user(
        &self,
        tx: &mut Transaction<'_, DB>,
        login: &str,
        password: &str,
    ) -> Result<(), IDPError>;
}

#[derive(Clone)]
pub struct MockIDPContext;

#[async_trait]
impl<DB: Database> IDPContext<DB> for MockIDPContext {
    async fn authenticate(
        &self,
        _: &mut Transaction<'_, DB>,
        _: &str,
        _: &str,
    ) -> Result<Session, IDPError> {
        Ok(Session {
            session_id: "mock session id".to_owned(),
            expires: DateTime::<Utc>::MAX_UTC,
        })
    }

    async fn add_user(
        &self,
        _: &mut Transaction<'_, DB>,
        _: &str,
        _: &str,
    ) -> Result<(), IDPError> {
        Ok(())
    }
}

#[derive(Error, Serialize, Debug)]
pub enum IDPError {
    #[error("Incorrect username or password")]
    AuthenticationFailed,
    #[error("Requested username is occupied")]
    UsernameTaken,
    #[error("Authentication error")]
    AuthenticationError(#[serde(skip)] SQLXError),
    #[error("Registration error")]
    RegistrationError(#[serde(skip)] SQLXError),
}

impl ToReply for IDPError {
    fn into_reply(self) -> impl Reply {
        reply::with_status(reply::json(&self), StatusCode::UNAUTHORIZED)
    }
}
