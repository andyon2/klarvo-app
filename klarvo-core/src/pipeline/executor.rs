use crate::error::{AppError, AppErrorKind};
use crate::manifest::PipelineManifest;
use crate::pipeline::stage::PipelineStageType;
use crate::registry::PluginRegistry;
use crate::traits::CleanupInput;

pub async fn run(
    manifest: &PipelineManifest,
    registry: &PluginRegistry,
    input: &str,
) -> Result<String, AppError> {
    let mut text = input.to_string();
    for stage in &manifest.pipeline.stages {
        match stage {
            #[cfg(feature = "stage-passthrough")]
            PipelineStageType::Passthrough => {
                // no-op: text passes through unchanged
            }
            #[cfg(feature = "stage-cleanup")]
            PipelineStageType::Cleanup { plugin_id } => {
                let impl_ = registry.cleanup(plugin_id).ok_or_else(|| AppError {
                    kind: AppErrorKind::PipelineValidation,
                    message: format!("cleanup plugin '{plugin_id}' not registered"),
                    user_message: None,
                    retryable: false,
                })?;
                let cleanup_input = CleanupInput::from_raw(text.clone());
                text = impl_.apply(cleanup_input).await?;
            }
            // Forcing-sentinel: Phase-0-Executor has no StageData / audio domain.
            // Story 1B.5 rewrites the full dispatch layer.
            #[cfg(feature = "stage-stt")]
            PipelineStageType::Stt { plugin_id } => {
                return Err(AppError {
                    kind: AppErrorKind::Internal,
                    message: format!(
                        "STT dispatch not yet wired in Phase-0-Executor \
                         (plugin_id: {plugin_id}) — Story 1B.5"
                    ),
                    user_message: None,
                    retryable: false,
                });
            }
        }
    }
    Ok(text)
}
