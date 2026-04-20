pub mod audio;
pub mod error;
pub mod i18n;
pub mod keystore;
pub mod manifest;
pub mod output;
pub mod pipeline;
pub mod registry;
pub mod traits;
pub mod v1_import;

pub use error::{AppError, AppErrorKind, PluginError};
pub use registry::{PluginRegistry, bootstrap};
