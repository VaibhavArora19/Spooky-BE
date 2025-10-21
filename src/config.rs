use std::env;

#[derive(Debug, Default)]
pub struct Config {
    pub http_port: String,
    pub ws_port: String,
    pub redis_url: String,
    pub mongodb_url: String,
}

#[derive(thiserror::Error, Debug)]
enum ConfigError {
    #[error("Error: Invalid port number")]
    InvalidPort,
    #[error("Error: Invalid Redis URL")]
    InvalidRedisUrl,
    #[error("Error: Invalid MongoDB URL")]
    InvalidDBUrl,
}

impl Config {
    pub fn get_config() -> Self {
        let http_port =
            env::var("HTTP_PORT").unwrap_or_else(|_| ConfigError::InvalidPort.to_string());

        let ws_port = env::var("WS_PORT").unwrap_or_else(|_| ConfigError::InvalidPort.to_string());

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| ConfigError::InvalidRedisUrl.to_string());

        let mongodb_url =
            env::var("MONGODB_URL").unwrap_or_else(|_| ConfigError::InvalidDBUrl.to_string());


        Self {
            http_port,
            ws_port,
            redis_url,
            mongodb_url,
        }
    }
}
