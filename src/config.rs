use std::env;

#[derive(Debug, Default)]
pub struct Config {
    pub port: String,
}

#[derive(thiserror::Error, Debug)]
enum ConfigError {
    #[error("Error: Invalid port number")]
    InvalidPort,
}

impl Config {
    pub fn get_config() -> Self {
        let port = env::var("PORT").unwrap_or_else(|_| ConfigError::InvalidPort.to_string());

        // Placeholder for actual config retrieval logic
        Self { port }
    }
}
