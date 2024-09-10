use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use confique::Config;
use log::{error, info};
use refinery::config::{Config as RefineryCfg, ConfigDbType};
use refinery::embed_migrations;
use sqlx::postgres::PgConnectOptions;
use sqlx::{PgPool, Postgres};
use structured_logger::async_json::new_writer;
use structured_logger::Builder;
use tap::TapFallible;
use warp::Filter;

use crate::auth::{AuthenticationFilter, IDPContext, PgIDPContext};
use crate::config::{ApplicationConfig, LoggerConfig, PgConfig};
use crate::handlers::user_handler::UserHandler;
use crate::handlers::RestHandler;
use crate::repo::auth_repository::{AuthRepository, PgAuthRepository};
use crate::repo::session_repository::{PgSessionRepository, SessionRepository};
use crate::repo::user_repository::{PgUserRepository, UserRepository};

mod auth;
mod config;
pub(crate) mod domain;
mod extensions;
mod handlers;
pub(crate) mod repo;

const CONFIG_ENV: &str = "CONFIG";
const DEFAULT_CONFIG_PATH: &str = "cfg/application.yml";

embed_migrations!("migrations");

#[tokio::main]
async fn main() {
    let env: HashMap<String, String> = env::vars().map(|(k, v)| (k.to_uppercase(), v)).collect();

    let config_file_path = env
        .get(CONFIG_ENV)
        .map(|file_name| file_name.as_str())
        .unwrap_or(DEFAULT_CONFIG_PATH);

    let config: ApplicationConfig =
        ApplicationConfig::from_file(config_file_path).expect(&format!(
            "Failed to load application config from {}",
            config_file_path
        ));

    initialize_logger(config.logger_config);

    let _ = migrate(&config.pg_config).await;
    let pool: Arc<PgPool> = connect_to_db(&config.pg_config).await;

    let session_repository: Arc<dyn SessionRepository<Postgres>> = Arc::new(PgSessionRepository);
    let auth_repository: Arc<dyn AuthRepository<Postgres>> = Arc::new(PgAuthRepository);
    let idp_context: Arc<dyn IDPContext<Postgres>> = Arc::new(PgIDPContext::new(
        session_repository,
        auth_repository,
        &config.auth_config,
    ));
    let auth_filter: Arc<AuthenticationFilter<Postgres>> = Arc::new(AuthenticationFilter {
        pool: pool.clone(),
        idp: idp_context.clone(),
    });
    let user_repository: Arc<dyn UserRepository<Postgres>> = Arc::new(PgUserRepository);
    let user_handler = Arc::new(UserHandler {
        pool: pool.clone(),
        authentication_filter: auth_filter.clone(),
        idp_context: idp_context.clone(),
        repository: user_repository,
    });

    let routes = user_handler
        .routes()
        .recover(handlers::rejection_handler::handle_rejections);

    warp::serve(routes).run(([127, 0, 0, 1], 8080)).await;
}

async fn connect_to_db(config: &PgConfig) -> Arc<PgPool> {
    Arc::new(
        PgPool::connect_with(
            PgConnectOptions::new()
                .host(&config.host)
                .port(config.port)
                .username(&config.user)
                .password(&config.password)
                .database(&config.database),
        )
        .await
        .expect("Failed to construct Database Pool"),
    )
}

async fn migrate(config: &PgConfig) -> () {
    let mut conn = RefineryCfg::new(ConfigDbType::Postgres)
        .set_db_user(&config.user)
        .set_db_pass(&config.password)
        .set_db_host(&config.host)
        .set_db_port(&config.port.to_string())
        .set_db_name(&config.database);

    info!("Starting database migrations");

    migrations::runner()
        .run_async(&mut conn)
        .await
        .tap_err(|err| error!(err:err = *err; "Failed to perform migrations"))
        .tap_ok(|report| info!(report:debug = report; "Successfully completed migrations"))
        .expect("Failed to perform migrations");
}

fn initialize_logger(cfg: LoggerConfig) {
    Builder::with_level(&cfg.level)
        .with_target_writer("*", new_writer(tokio::io::stdout()))
        .init();
}
