use crate::auth::Session;
use crate::extensions::Unit;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::warn;
use sqlx::{Database, Error, Postgres, Transaction};
use tap::TapFallible;

#[async_trait]
pub trait SessionRepository<DB: Database>
where
    Self: Send + Sync,
{
    async fn not_expired(
        &self,
        tx: &mut Transaction<'_, DB>,
        session_id: &str,
    ) -> Option<DateTime<Utc>>;

    async fn save(&self, tx: &mut Transaction<'_, DB>, session: &Session) -> Result<(), Error>;
}

pub struct PgSessionRepository;

#[async_trait]
impl SessionRepository<Postgres> for PgSessionRepository {
    async fn not_expired(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        session_id: &str,
    ) -> Option<DateTime<Utc>> {
        sqlx::query_scalar!(
            r#"SELECT expires as "expires!" FROM sessions WHERE session_id = $1"#,
            &session_id
        )
        .fetch_one(&mut **tx)
        .await
        .map(|expires| expires.and_utc())
        .ok()
    }

    async fn save(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        session: &Session,
    ) -> Result<(), Error> {
        sqlx::query!(
            "INSERT INTO sessions(session_id, expires) VALUES ($1, $2)",
            session.session_id.clone(),
            session.expires.clone().naive_utc(),
        )
        .execute(&mut **tx)
        .await
        .tap_err(
            |err| warn!(session_id = session.session_id, err:err = *err; "Failed to save session"),
        )
        .unit()
    }
}
