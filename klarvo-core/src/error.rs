use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PluginError {
    #[error("network: {0}")]
    Network(String),
    #[error("auth: {0}")]
    Auth(String),
    #[error("rate limited (retry after {retry_after_ms}ms)")]
    RateLimit { retry_after_ms: u64 },
    #[error("fatal: {0}")]
    Fatal(String),
    #[error("unavailable: {0}")]
    Unavailable(String),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: AppErrorKind,
    pub message: String,
    pub user_message: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AppErrorKind {
    Network,
    Auth,
    Validation,
    RateLimit,
    Internal,
    Unavailable,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<PluginError> for AppError {
    fn from(e: PluginError) -> Self {
        match e {
            PluginError::Network(msg) => AppError {
                kind: AppErrorKind::Network,
                message: msg,
                user_message: None,
                retryable: true,
            },
            PluginError::Auth(msg) => AppError {
                kind: AppErrorKind::Auth,
                message: msg,
                user_message: None,
                retryable: false,
            },
            PluginError::RateLimit { retry_after_ms } => AppError {
                kind: AppErrorKind::RateLimit,
                message: format!("rate limited, retry after {retry_after_ms}ms"),
                user_message: None,
                retryable: true,
            },
            PluginError::Fatal(msg) => AppError {
                kind: AppErrorKind::Internal,
                message: msg,
                user_message: None,
                retryable: false,
            },
            PluginError::Unavailable(msg) => AppError {
                kind: AppErrorKind::Unavailable,
                message: msg,
                user_message: None,
                retryable: true,
            },
        }
    }
}
