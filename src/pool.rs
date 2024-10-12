use async_trait::async_trait;
use serde::ser::StdError;

#[async_trait]
pub trait DatabasePool
where
    Self: Send + Sync
{
    type Tx: Send + TransactionOps<Self::Err>;
    type Err: 'static + Send + Sync + StdError + DbErrorOps;

    async fn begin_tx(&self) -> Result<Self::Tx, Self::Err>;
}

#[async_trait]
pub trait TransactionOps<Err>
where
    Self: Send,
    Err: 'static + Send + Sync + StdError
{
    async fn commit(mut self) -> Result<(), Err>;
}

pub trait DbErrorOps {
    fn is_unique_violation(&self) -> bool;
}

#[async_trait]
impl<DB: sqlx::Database> DatabasePool for sqlx::Pool<DB> {
    type Tx = sqlx::Transaction<'static, DB>;
    type Err = sqlx::Error;

    async fn begin_tx(&self) -> Result<Self::Tx, Self::Err> {
        self.begin().await
    }
}

#[async_trait]
impl<DB: sqlx::Database> TransactionOps<sqlx::Error> for sqlx::Transaction<'_, DB> {
    async fn commit(mut self) -> Result<(), sqlx::Error> {
        self.commit().await
    }
}

impl DbErrorOps for sqlx::Error {
    fn is_unique_violation(&self) -> bool {
        self.as_database_error().map_or(false, |err| err.is_unique_violation())
    }
}

#[cfg(test)]
pub struct MockPool {}

#[cfg(test)]
impl DatabasePool for MockPool {
    type Tx<'c> = ();
    type Err = ();

    async fn begin_tx(&self) -> Result<Self::Tx, Err> {
        Ok(())
    }
}
