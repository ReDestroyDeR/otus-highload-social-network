use confique::Config;

#[derive(Config)]
pub struct ApplicationConfig {
    #[config(nested)]
    pub logger_config: LoggerConfig,
    #[config(nested)]
    pub pg_config: PgConfig,
    #[config(nested)]
    pub auth_config: AuthConfig,
}

#[derive(Config)]
pub struct LoggerConfig {
    pub level: String,
}

#[derive(Config)]
pub struct PgConfig {
    #[config(env = "PG_HOST")]
    pub host: String,
    #[config(env = "PG_PORT")]
    pub port: u16,
    #[config(env = "PG_DB")]
    pub database: String,
    pub schema: Option<String>,
    #[config(env = "PG_USER")]
    pub user: String,
    #[config(env = "PG_PASS")]
    pub password: String,
}

#[derive(Config)]
pub struct AuthConfig {
    pub session_lifetime_seconds: u32,
    pub invalid_sessions_cache_limit: usize,
}
