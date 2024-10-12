use crate::auth::Session;
use crate::extensions::Unit;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use log::warn;
use sqlx::{Error, PgPool, Postgres, Transaction};
use tap::TapFallible;
use crate::pool::DatabasePool;

#[async_trait]
pub trait SessionRepository<Pool>
where
    Self: Send + Sync,
    Pool: DatabasePool,
{
    async fn not_expired(
        &self,
        tx: &mut Pool::Tx,
        session_id: &str,
    ) -> Option<DateTime<Utc>>;

    async fn save(&self, tx: &mut Pool::Tx, session: &Session) -> Result<(), Pool::Err>;
}

pub struct PgSessionRepository;

#[async_trait]
impl SessionRepository<PgPool> for PgSessionRepository {
    async fn not_expired(
        &self,
        tx: &mut Transaction<'static, Postgres>,
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
        tx: &mut Transaction<'static, Postgres>,
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
