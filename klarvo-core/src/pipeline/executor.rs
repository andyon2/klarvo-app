use crate::error::{AppError, AppErrorKind};
use crate::manifest::{Manifest, Stage};
use crate::registry::PluginRegistry;
use crate::traits::CleanupInput;

pub async fn run(
    manifest: &Manifest,
    registry: &PluginRegistry,
    input: &str,
) -> Result<String, AppError> {
    let mut text = input.to_string();
    for stage in &manifest.pipeline.stages {
        match stage {
            Stage::Cleanup { plugin } => {
                let impl_ = registry.cleanup(plugin).ok_or_else(|| AppError {
                    kind: AppErrorKind::Validation,
                    message: format!("cleanup plugin '{plugin}' not registered"),
                    user_message: None,
                    retryable: false,
                })?;
                let cleanup_input = CleanupInput::from_raw(text.clone());
                text = impl_.apply(cleanup_input).await?;
            }
        }
    }
    Ok(text)
}
