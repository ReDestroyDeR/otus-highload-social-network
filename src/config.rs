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
    pub host: String,
    pub port: u16,
    pub database: String,
    pub schema: Option<String>,
    pub user: String,
    pub password: String,
}

#[derive(Config)]
pub struct AuthConfig {
    pub session_lifetime_seconds: u32,
    pub invalid_sessions_cache_limit: usize,
}
