use crate::error::{AppError, AppErrorKind};
use crate::manifest::{Manifest, Stage};
use crate::registry::PluginRegistry;

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
                text = impl_.apply(&text).await?;
            }
        }
    }
    Ok(text)
}
