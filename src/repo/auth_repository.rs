use crate::domain::user::{Credentials, User};
use crate::extensions::Unit;
use async_trait::async_trait;
use log::error;
use sqlx::{Database, Error, Postgres, Transaction};
use tap::TapFallible;
use uuid::Uuid;

#[async_trait]
pub trait AuthRepository<DB: Database>
where
    Self: Send + Sync,
{
    async fn find(&self, tx: &mut Transaction<'_, DB>, login: &str) -> Option<Credentials>;

    async fn save(
        &self,
        tx: &mut Transaction<'_, DB>,
        credentials: &Credentials,
        user: &User,
    ) -> Result<(), Error>;
}

pub struct PgAuthRepository;

#[async_trait]
impl AuthRepository<Postgres> for PgAuthRepository {
    async fn find(&self, tx: &mut Transaction<'_, Postgres>, login: &str) -> Option<Credentials> {
        sqlx::query_as!(
            Credentials,
            "SELECT auth.login, auth.password FROM auth WHERE auth.login = $1",
            login
        )
        .fetch_one(&mut **tx)
        .await
        .ok()
    }

    async fn save(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        credentials: &Credentials,
        user: &User,
    ) -> Result<(), Error> {
        sqlx::query!(
            r#"
            INSERT INTO auth(id, user_id, login, password)
            VALUES ($1, $2, $3, $4)
            "#,
            Uuid::new_v4(),
            &user.id,
            &credentials.login,
            &credentials.password,
        )
        .execute(&mut **tx)
        .await
        .tap_err(|err| error!(err:err = *err; "Failed to save credentials"))
        .unit()
    }
}
