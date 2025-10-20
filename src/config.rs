use std::env;

#[derive(Debug, Default)]
pub struct Config {
    pub port: String,
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
        let port = env::var("PORT").unwrap_or_else(|_| ConfigError::InvalidPort.to_string());

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| ConfigError::InvalidRedisUrl.to_string());

        let mongodb_url =
            env::var("MONGODB_URL").unwrap_or_else(|_| ConfigError::InvalidDBUrl.to_string());

        Self {
            port,
            redis_url,
            mongodb_url,
        }
    }
}
