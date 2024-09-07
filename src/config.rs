use confique::Config;

#[derive(Config)]
pub struct ApplicationConfig {
    #[config(nested)]
    pub logger_config: LoggerConfig,
    #[config(nested)]
    pub pg_config: PgConfig,
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

impl PgConfig {
    pub fn connection_url(&self) -> String {
        format!(
            "postgres://{user}:{pass}@{host}:{port}/{db}{schema}",
            user = self.user,
            pass = self.password,
            host = self.host,
            port = self.port,
            db = self.database,
            schema = self
                .schema
                .iter()
                .map(|schema| format!("?currentSchema={schema}"))
                .next()
                .unwrap_or("".to_owned())
        )
    }
}
