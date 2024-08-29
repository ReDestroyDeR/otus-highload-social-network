use std::collections::HashMap;
use std::env;

use confique::Config;
use structured_logger::async_json::new_writer;
use structured_logger::Builder;
use tap::TapFallible;
use warp::Filter;

use crate::config::{ApplicationConfig, LoggerConfig};

mod config;

const CONFIG_ENV: &str = "CONFIG";
const DEFAULT_CONFIG_PATH: &str = "cfg/application.yml";

#[tokio::main]
async fn main() {

    let env: HashMap<String, String> = env::vars()
        .map(|(k, v)| (k.to_uppercase(), v))
        .collect();

    let config_file_path = env.get(CONFIG_ENV)
        .map(|file_name| file_name.as_str())
        .unwrap_or(DEFAULT_CONFIG_PATH);

    let config: ApplicationConfig = ApplicationConfig::from_file(config_file_path)
        .expect(&format!("Failed to load application config from {}", config_file_path));

    initialize_logger(config.logger_config);

    // let routes =
    //
    // warp::serve(hello)
    //     .run(([127, 0, 0, 1], 8080))
    //     .await;
}

fn initialize_logger(cfg: LoggerConfig) {
    Builder::with_level(&cfg.level)
        .with_target_writer("*", new_writer(tokio::io::stdout()))
        .init();
}