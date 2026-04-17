pub mod error;
pub mod registry;
pub mod traits;

pub use error::{AppError, AppErrorKind, PluginError};
pub use registry::{PluginRegistry, bootstrap};
