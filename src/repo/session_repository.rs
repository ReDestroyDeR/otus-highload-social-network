use async_trait::async_trait;
use log::warn;
use sqlx::{Database, Postgres, Transaction};
use tap::TapFallible;

#[async_trait]
pub trait SessionRepository<DB: Database>
where
    Self: Send + Sync,
{
    async fn exists(&self, tx: &mut Transaction<'_, DB>, session_id: String) -> bool;

    async fn save(&self, tx: &mut Transaction<'_, DB>, session_id: String) -> ();
}

struct PgSessionRepository;

#[async_trait]
impl SessionRepository<Postgres> for PgSessionRepository {
    async fn exists(&self, tx: &mut Transaction<'_, Postgres>, session_id: String) -> bool {
        let count: i64 = sqlx::query_scalar!(
            r#"SELECT COALESCE(COUNT(1), 0) as "count!" FROM sessions WHERE sessions.session_id = $1"#,
            session_id.clone()
        ).fetch_one(&mut **tx)
            .await
            .tap_err(|err| warn!(session_id = session_id, err:err = *err; "Failed to check if session exists"))
            .unwrap_or(0);

        count > 0
    }

    async fn save(&self, tx: &mut Transaction<'_, Postgres>, session_id: String) -> () {
        let _ = sqlx::query!("INSERT INTO sessions(session_id) VALUES ($1)", session_id.clone())
            .fetch_one(&mut **tx)
            .await
            .tap_err(|err| warn!(session_id = session_id, err:err = *err; "Failed to check if session exists"));
    }
}
