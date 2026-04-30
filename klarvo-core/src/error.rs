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
    #[error("upstream unavailable: {0}")]
    UpstreamUnavailable(String),
    #[error("key missing for plugin: {plugin_id}")]
    KeyMissing { plugin_id: String },
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub kind: AppErrorKind,
    pub message: String,
    pub user_message: Option<String>,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AppErrorKind {
    /// Network-layer failure (TCP, DNS, TLS). Typical retryable=true.
    Network,
    /// Authentication/authorization rejection (401, 403). Typical retryable=false.
    Auth,
    /// Client-input validation failure. Distinct from `PipelineValidation`
    /// (boot-time manifest-strict-error). Typical retryable=false.
    Validation,
    /// Upstream rate-limit signal (429 + retry_after_ms). Typical retryable=true.
    RateLimit,
    /// Programmer-error, logic-bug, invariant-violation. Typical retryable=false.
    Internal,
    /// Upstream provider unavailable (5xx, timeout, connection-reset). Typical retryable=true.
    UpstreamUnavailable,
    /// OS configuration error (e.g., output target not found in registry). Typical retryable=false.
    Configuration,
    /// OS-level I/O error (e.g., clipboard write, file access). Typical retryable=false.
    Io,
    /// OS-level permission denied (e.g., microphone, accessibility-service).
    /// Typical retryable=false — requires user-action at OS-level.
    PermissionDenied,
    /// Manifest strict-validation error at boot-time (unknown stage-type, type-mismatch).
    /// Distinct from `Validation` (runtime client-input). Typical retryable=false.
    PipelineValidation,
    /// KeyStore-lookup miss during plugin-init. Plugin-identifier lands in `AppError.message`.
    /// Typical retryable=false.
    KeyMissing,
    /// Win32 `RegisterHotKey` rejected the combo — already claimed by another app.
    /// Typical retryable=false — user must choose a different combo.
    HotkeyConflict,
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
            PluginError::UpstreamUnavailable(msg) => AppError {
                kind: AppErrorKind::UpstreamUnavailable,
                message: msg,
                user_message: None,
                retryable: true,
            },
            PluginError::KeyMissing { plugin_id } => AppError {
                kind: AppErrorKind::KeyMissing,
                message: format!("key missing for plugin: {plugin_id}"),
                user_message: Some("error.keystore.key_missing".into()),
                retryable: false,
            },
        }
    }
}
