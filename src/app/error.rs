use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to load settings: {0}")]
    Settings(#[from] config::ConfigError),

    #[error("failed to bind TCP listener: {0}")]
    Bind(#[source] std::io::Error),

    #[error("HTTP server failed: {0}")]
    Server(#[source] std::io::Error),
}
