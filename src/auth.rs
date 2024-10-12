use crate::auth::AuthenticationError::{InternalError, InvalidSessionId, NoSessionIdHeader};
use crate::config::AuthConfig;
use async_trait::async_trait;
use bcrypt::BcryptError;
use chrono::{DateTime, Duration, Utc};
use concurrent_queue::ConcurrentQueue;
use dashmap::mapref::one::Ref as DashMapRef;
use dashmap::DashMap;
use log::{error, info};
use serde::Serialize;
use std::fmt::Debug;
use std::marker::PhantomData;
use std::ops::Add;
use std::str::FromStr;
use std::sync::Arc;
use serde::ser::StdError;
use thiserror::Error;
use uuid::Uuid;
use warp::http::header::AUTHORIZATION;
use warp::http::StatusCode;
use warp::reject::Reject;
use warp::{reject, reply, Filter, Rejection, Reply};

use crate::domain::protocol::ToReply;
use crate::domain::user::{Credentials, User};
use crate::extensions::Unit;
use crate::pool::{DatabasePool, DbErrorOps};
use crate::repo::auth_repository::AuthRepository;
use crate::repo::session_repository::SessionRepository;

#[derive(Clone)]
pub struct AuthenticationFilter<Pool, IDP>
where
    Pool: DatabasePool,
    IDP: IDPContext<Pool>,
{
    pub pool: Arc<Pool>,
    pub idp: Arc<IDP>,
}

impl<IDP, Pool> AuthenticationFilter<Pool, IDP>
where
    Pool: DatabasePool,
    IDP: IDPContext<Pool> + Sync,
{
    pub fn with_session(
        self: Arc<Self>,
    ) -> impl Filter<Extract = (String,), Error = Rejection> + Clone {
        warp::header::optional(AUTHORIZATION.as_str()).and_then(move |token: Option<String>| {
            let inner_self = self.clone();
            async move {
                let session_id = token
                    .and_then(|raw| {
                        raw.strip_prefix("session-id ")
                            .map(|reference| reference.to_owned())
                    })
                    .ok_or(reject::custom(NoSessionIdHeader))?;

                match inner_self.pool.begin_tx().await {
                    Err(err) => {
                        error!(err:err = err; "Failed obtaining transaction for authentication");
                        Err(reject::custom(InternalError))
                    }
                    Ok(mut tx) => {
                        if inner_self
                            .idp
                            .is_valid(&mut tx, session_id.to_owned())
                            .await
                        {
                            Ok(session_id.to_owned())
                        } else {
                            Err(reject::custom(InvalidSessionId))
                        }
                    }
                }
            }
        })
    }
}

#[derive(Error, Serialize, Debug)]
pub enum AuthenticationError {
    #[error("Session id is missing. Expected header - Authorization: session-id [session_id]")]
    NoSessionIdHeader,
    #[error("Internal Error")]
    InternalError,
    #[error("Invalid session id")]
    InvalidSessionId,
}

impl Reject for AuthenticationError {}

pub struct Session {
    pub session_id: String,
    pub expires: DateTime<Utc>,
}

impl Session {
    fn to_cached(&self) -> CachedSession {
        CachedSession {
            invalid: false,
            expires: Some(self.expires.clone()),
        }
    }
}

#[async_trait]
pub trait IDPContext<Pool>
where
    Self: Send + Sync,
    Pool: DatabasePool
{
    async fn is_valid(&self, tx: &mut Pool::Tx, session_id: String) -> bool;
    async fn authenticate(
        &self,
        tx: &mut Pool::Tx,
        credentials: &Credentials,
    ) -> Result<Session, IDPError<Pool::Err>>;
    async fn add_user(
        &self,
        tx: &mut Pool::Tx,
        login: &str,
        password: &str,
        user: &User,
    ) -> Result<(), IDPError<Pool::Err>>;
}

pub struct PgIDPContext<Pool, SessionRepo, AuthRepo>
where
    Pool: DatabasePool,
    SessionRepo: SessionRepository<Pool>,
    AuthRepo: AuthRepository<Pool>,
{
    session_repo: Arc<SessionRepo>,
    session_cache: DashMap<String, CachedSession>,
    session_lifetime: Duration,
    invalid_sessions: ConcurrentQueue<String>,
    auth_repo: Arc<AuthRepo>,
    pool: PhantomData<Pool>,
}

#[derive(Clone)]
struct CachedSession {
    expires: Option<DateTime<Utc>>,
    invalid: bool,
}

impl CachedSession {
    pub fn valid(&self) -> bool {
        !self.invalid && self.expires.map(|time| time > Utc::now()).unwrap_or(false)
    }
}

impl<Pool, SessionRepo, AuthRepo> PgIDPContext<Pool, SessionRepo, AuthRepo>
where
    Pool: DatabasePool,
    SessionRepo: SessionRepository<Pool>,
    AuthRepo: AuthRepository<Pool>,
{
    pub fn new(
        session_repo: Arc<SessionRepo>,
        auth_repo: Arc<AuthRepo>,
        auth_config: &AuthConfig,
    ) -> Self {
        Self {
            session_repo,
            session_cache: DashMap::new(),
            session_lifetime: Duration::seconds(auth_config.session_lifetime_seconds as i64),
            invalid_sessions: ConcurrentQueue::bounded(auth_config.invalid_sessions_cache_limit),
            auth_repo,
            pool: PhantomData,
        }
    }

    fn invalidate_session(
        &self,
        from_cache: DashMapRef<String, CachedSession>,
        session_id: &str,
    ) -> () {
        drop(from_cache); // To eliminate deadlock

        self.session_cache
            .alter(session_id, |_, mut cached_session| {
                cached_session.invalid = true;
                cached_session
            });

        self.add_session_to_invalidation_queue(session_id);
    }

    fn cache_session(&self, session_id: String, session: CachedSession) {
        let invalid = session.invalid;
        self.session_cache.insert(session_id.clone(), session);

        if invalid {
            self.add_session_to_invalidation_queue(&session_id);
        }
    }

    fn add_session_to_invalidation_queue(&self, session_id: &str) {
        if self.invalid_sessions.is_full() {
            let _ = self.invalid_sessions.pop()
                .map_err(|err| error!(err:err = err; "Unexpected session invalidation invalidation error"))
                .map(|session_id| self.session_cache.remove(&session_id));
        }

        let _ = self.invalid_sessions.push(session_id.to_owned());

        info!(session_id = session_id; "Invalidated session");
    }

    fn is_valid_session_in_cache(&self, session_id: &str) -> Option<bool> {
        self.session_cache.get(session_id).map(|cached_session| {
            if cached_session.value().valid() {
                true
            } else if !cached_session.value().invalid {
                self.invalidate_session(cached_session, session_id);
                false
            } else {
                false
            }
        })
    }
}

#[async_trait]
impl<Pool, SessionRepo, AuthRepo> IDPContext<Pool> for PgIDPContext<Pool, SessionRepo, AuthRepo>
where
    Self: Sync,
    Pool: DatabasePool,
    SessionRepo: SessionRepository<Pool>,
    AuthRepo: AuthRepository<Pool>,
{
    async fn is_valid(&self, tx: &mut Pool::Tx, session_id: String) -> bool {
        if Uuid::from_str(&session_id).is_err() {
            false
        } else if let Some(cached) = self.is_valid_session_in_cache(&session_id) {
            cached
        } else {
            let from_db = self.session_repo.not_expired(tx, &session_id).await;

            let session = match from_db {
                Some(not_expired) if not_expired > Utc::now() => CachedSession {
                    expires: Some(not_expired),
                    invalid: false,
                },
                Some(expired) => CachedSession {
                    expires: Some(expired),
                    invalid: true,
                },
                None => CachedSession {
                    expires: None,
                    invalid: true,
                },
            };

            let is_valid = !session.invalid;
            self.cache_session(session_id, session);

            is_valid
        }
    }

    async fn authenticate(
        &self,
        tx: &mut Pool::Tx,
        credentials: &Credentials,
    ) -> Result<Session, IDPError<Pool::Err>> {
        let db_credentials = self
            .auth_repo
            .find(tx, &credentials.login)
            .await
            .ok_or(IDPError::AuthenticationFailed)?;

        if bcrypt::verify(&credentials.password, &db_credentials.password)
            .map_err(|err| IDPError::CryptoError(err))?
        {
            let session = Session {
                session_id: Uuid::new_v4().to_string(),
                expires: Utc::now().add(self.session_lifetime),
            };

            self.session_repo
                .save(tx, &session)
                .await
                .map_err(|err| IDPError::AuthenticationError(err))?;
            self.cache_session(session.session_id.clone(), session.to_cached());
            info!(login = credentials.login; "Authenticated user and generated new Session");

            Ok(session)
        } else {
            Err(IDPError::AuthenticationFailed)
        }
    }

    async fn add_user(
        &self,
        tx: &mut Pool::Tx,
        login: &str,
        password: &str,
        user: &User,
    ) -> Result<(), IDPError<Pool::Err>> {
        let encrypted_password = bcrypt::hash(password, bcrypt::DEFAULT_COST)
            .map_err(|err| IDPError::CryptoError(err))?;

        self.auth_repo
            .save(
                tx,
                &Credentials {
                    login: login.to_owned(),
                    password: encrypted_password,
                },
                user,
            )
            .await
            .map_err(|err| match err {
                error if error.is_unique_violation() => {
                    IDPError::UsernameTaken
                }
                _ => IDPError::RegistrationError(err),
            })
            .unit()
    }
}

#[derive(Error, Serialize, Debug)]
pub enum IDPError<PoolErr: Send + StdError + Sync + 'static> {
    #[error("Incorrect username or password")]
    AuthenticationFailed,
    #[error("Requested username is occupied")]
    UsernameTaken,
    #[error("Authentication error")]
    AuthenticationError(#[serde(skip)] PoolErr),
    #[error("Registration error")]
    RegistrationError(#[serde(skip)] PoolErr),
    #[error("Cryptographic error")]
    CryptoError(#[serde(skip)] BcryptError),
}

impl<T: Debug + Send + StdError + Sync + 'static> Reject for IDPError<T> {}

impl<T: Send + StdError + Sync + 'static> ToReply for IDPError<T> {
    fn into_reply(self) -> impl Reply {
        reply::with_status(reply::json(&self), StatusCode::UNAUTHORIZED)
    }
}
