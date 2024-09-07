use crate::domain::user::{Interest, User};
use async_trait::async_trait;
use log::warn;
use sqlx::{Database, Encode, Error, Postgres, Transaction};
use tap::TapFallible;
use uuid::Uuid;

#[async_trait]
pub trait UserRepository<DB: Database>
where
    Self: Send + Sync,
{
    async fn find(&self, tx: &mut Transaction<'_, DB>, id: Uuid) -> Option<User>;

    async fn save(&self, tx: &mut Transaction<'_, DB>, user: User) -> Result<(), Error>;
}

#[derive(Clone)]
pub(crate) struct PgUserRepository;

#[async_trait]
impl UserRepository<Postgres> for PgUserRepository {
    async fn find(&self, tx: &mut Transaction<'_, Postgres>, id: Uuid) -> Option<User> {
        sqlx::query_as!(
            User,
            r#"
            SELECT
                users.id,
                users.first_name,
                users.last_name,
                users.birth_date,
                users.gender,
                users.city,
                COALESCE(NULLIF(ARRAY_AGG((interest.name, interest.description)), '{NULL}'), '{}') AS "interests!: Vec<Interest>"
            FROM users
            LEFT JOIN interest ON interest.user_id = users.id
            WHERE users.id = $1
            GROUP BY
                users.id,
                users.first_name,
                users.last_name,
                users.birth_date,
                users.gender,
                users.city
            "#,
            &id
        )
            .fetch_one(&mut **tx)
            .await
            .tap_err(|err| warn!(id:display = id, err:err = *err; "Failed to fetch user"))
            .ok()
    }

    async fn save(&self, tx: &mut Transaction<'_, Postgres>, user: User) -> Result<(), Error> {
        let _ = sqlx::query!(
            r#"
            INSERT INTO users (
                id,
                first_name,
                last_name,
                birth_date,
                gender,
                city
            ) VALUES (
                $1,
                $2,
                $3,
                $4,
                $5,
                $6
            )
            "#,
            &user.id,
            &user.first_name,
            &user.last_name,
            &user.birth_date,
            Into::<String>::into(&user.gender),
            &user.city,
        )
        .execute(&mut **tx)
        .await
        .tap_err(|err| warn!(id:display = &user.id, err:err = *err; "Failed to save user"))?;

        let mut builder = sqlx::QueryBuilder::new(
            "INSERT INTO interest (
                        id,
                        user_id,
                        name,
                        description
                    )",
        );

        builder.push_values(user.interests, |mut b, interest| {
            b.push_bind(Uuid::new_v4())
                .push_bind(&user.id)
                .push_bind(interest.name)
                .push_bind(interest.description);
        });

        builder.build().execute(&mut **tx).await.tap_err(
            |err| warn!(id:display = &user.id, err:err = *err; "Failed to save user interests"),
        )?;

        Ok(())
    }
}
