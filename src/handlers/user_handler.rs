use log::{error, info};
use std::sync::Arc;

use crate::auth::IDPError::AuthenticationError;
use crate::auth::{AuthenticationFilter, IDPContext, IDPError};
use crate::domain::protocol::ToResponse;
use sqlx::{Database, Pool};
use tap::TapFallible;
use uuid::Uuid;
use warp::filters::method;
use warp::{body, Filter, Rejection, Reply};

use crate::domain::user::{AuthenticationRequest, AuthenticationResponse, Credentials, RegistrationRequest, User};
use crate::handlers::RestHandler;
use crate::repo::user_repository::UserRepository;

#[derive(Clone)]
pub struct UserHandler<DB: Database> {
    pub pool: Arc<Pool<DB>>,
    pub repository: Arc<dyn UserRepository<DB>>,
    pub idp_context: Arc<dyn IDPContext<DB>>,
    pub authentication_filter: Arc<AuthenticationFilter<DB>>,
}

impl<DB: Database> UserHandler<DB>
where
    Self: Send + Sync,
{
    async fn login(
        &self,
        credentials: &Credentials
    ) -> Result<AuthenticationResponse, IDPError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| AuthenticationError(err))?;
        let response = self
            .idp_context
            .authenticate(&mut tx, &credentials)
            .await
            .map(|session| AuthenticationResponse {
                session_id: session.session_id,
            })?;
        tx.commit().await.map_err(|err| AuthenticationError(err))?;

        Ok(response)
    }

    async fn register(&self, request: RegistrationRequest) -> Result<User, IDPError> {
        let credentials = request.credentials;
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|err| IDPError::RegistrationError(err))?;

        let user = User {
            id: Uuid::new_v4(),
            first_name: request.first_name,
            last_name: request.last_name,
            birth_date: request.birth_date,
            gender: request.gender,
            interests: request.interests,
            city: request.city,
        };

        let _ = self
            .repository
            .save(&mut tx, user.clone())
            .await
            .map_err(|err| IDPError::RegistrationError(err))?;
        self.idp_context
            .add_user(&mut tx, &credentials.login, &credentials.password, &user)
            .await?;

        tx.commit()
            .await
            .map_err(|err| IDPError::RegistrationError(err))?;

        info!(user_id:display = user.id.clone(), login = credentials.login.clone(); "Registered new user");

        Ok(user)
    }

    async fn get(&self, user_id: Uuid) -> Option<User> {
        let mut tx = self.pool.begin().await.ok()?;
        let user = self.repository.find(&mut tx, user_id).await;
        let _ = tx
            .commit()
            .await
            .tap_err(|err| error!(err:err = *err; "Failed to find user"));
        user
    }
}

impl<DB: Database> RestHandler for Arc<UserHandler<DB>> {
    fn routes(self) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
        let login = {
            let handler = self.clone();
            warp::path!("login")
                .and(method::post())
                .and(body::json())
                .and_then(move |authentication: AuthenticationRequest| {
                    let inner_handler = handler.clone();
                    async move {
                        inner_handler
                            .login(&authentication.credentials)
                            .await
                            .into_response()
                    }
                })
        };

        let register = {
            let handler = self.clone();
            warp::path!("user" / "register")
                .and(method::post())
                .and(body::json())
                .and_then(move |registration_request| {
                    let inner_handler = handler.clone();
                    async move {
                        inner_handler
                            .register(registration_request)
                            .await
                            .into_response()
                    }
                })
        };

        let get = {
            let handler = self.clone();
            warp::path!("user" / "get" / Uuid)
                .and(handler.authentication_filter.clone().with_session())
                .and_then(move |user_id, _| {
                    let inner_handler = handler.clone();
                    async move { inner_handler.get(user_id).await.into_response() }
                })
        };

        login.or(register).or(get)
    }
}
