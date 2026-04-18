pub mod audio;
pub mod error;
pub mod manifest;
pub mod pipeline;
pub mod registry;
pub mod traits;
pub mod v1_import;

pub use error::{AppError, AppErrorKind, PluginError};
pub use registry::{PluginRegistry, bootstrap};
