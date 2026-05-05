//! `klarvo-plugin-whisper-local` — Local Whisper.cpp STT-Provider.
//!
//! Implements `SttProvider` backed by `whisper-rs` (Rust FFI to `whisper.cpp`).
//!
//! # Substrate-Validation Role
//!
//! This is the second STT-Plugin alongside `klarvo-plugin-groq`. Where Groq uses
//! HTTPS-Cloud-API + Auth + RateLimit, Whisper-Local uses Disk-Loaded-Model +
//! In-Process-Inference. Both satisfy `SttProvider` without trait changes —
//! proving the trait carries both Cloud and Local implementations (ADR-0014).
//!
//! # Configuration
//!
//! Set the model path in Klarvo Settings (Plugin-Setting API):
//!   `plugins.whisper-local.model_path` = `/path/to/model.gguf`
//!
//! # Recommended Model Sizes (ADR-0014 D-2)
//!
//! | Model | Size | Quality | Note |
//! |-------|------|---------|------|
//! | `small` | ~500 MB | **Recommended** | Best quality/size tradeoff for German/English |
//! | `base` | ~150 MB | Low-Resource Fallback | Acceptable for short EN utterances |
//! | `tiny` | ~75 MB | Not recommended | Quality too low for production use (tested by Andy) |
//!
//! Models: download `.gguf` files from `ggerganov/whisper.cpp` releases on GitHub.
//!
//! # Thread Safety
//!
//! `WhisperContext` is wrapped in `Arc<Mutex<_>>` (ADR-0014 D-4).
//! `PipelineStage::process` dispatches inference to a blocking thread pool via
//! `tokio::task::spawn_blocking` — the async executor is never blocked.
//!
//! # Language Hint
//!
//! The `language` parameter (ADR-0014 D-5) is sourced from `settings.output_language()`
//! (i18n Axis 3 per `memory/project_i18n_three_axes`). Pass `None` only if no language
//! axis is configured.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use whisper_rs::WhisperContext;

use klarvo_core::audio::AudioBuffer;
use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::i18n;
use klarvo_core::pipeline::PipelineStage;
use klarvo_core::traits::SttProvider;

/// Plugin identifier — matches `plugin_id: "whisper-local"` in pipeline manifests.
pub const ID: &str = "whisper-local";

/// i18n error keys emitted by this plugin.
pub mod keys {
    pub const MODEL_NOT_FOUND: &str = "error.stt.local.model_not_found";
    pub const LOAD_FAILED: &str = "error.stt.local.load_failed";
    pub const INFERENCE_FAILED: &str = "error.stt.local.inference_failed";
}

/// Local Whisper.cpp-backed `SttProvider`. See module-level doc for scope and safety notes.
pub struct WhisperLocal {
    ctx: Arc<Mutex<WhisperContext>>,
    language: Option<String>,
}

impl WhisperLocal {
    /// Load a whisper.cpp model from `model_path`. `language` is an ISO-639-1 code
    /// (e.g. "de", "en") sourced from `settings.output_language()` (ADR-0014 D-5).
    /// Pass `None` only if language is genuinely unknown — whisper.cpp auto-detect
    /// is disabled in Klarvo per D-5 (brittleness on short utterances).
    ///
    /// # Errors
    ///
    /// - `error.stt.local.model_not_found` — `model_path` does not exist on disk.
    /// - `error.stt.local.load_failed` — whisper.cpp context creation failed
    ///   (corrupt file, unsupported format, OOM).
    pub fn load(model_path: &std::path::Path, language: Option<String>) -> Result<Self, AppError> {
        if !model_path.exists() {
            debug_assert!(i18n::is_key(keys::MODEL_NOT_FOUND));
            return Err(AppError {
                kind: AppErrorKind::Configuration,
                message: format!("whisper-local: model not found: {}", model_path.display()),
                user_message: Some(keys::MODEL_NOT_FOUND.to_string()),
                retryable: false,
            });
        }

        let path_str = model_path.to_str().ok_or_else(|| AppError {
            kind: AppErrorKind::Configuration,
            message: format!(
                "whisper-local: model path is not valid UTF-8: {}",
                model_path.display()
            ),
            user_message: Some(keys::LOAD_FAILED.to_string()),
            retryable: false,
        })?;

        let ctx = WhisperContext::new_with_params(
            path_str,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| {
            debug_assert!(i18n::is_key(keys::LOAD_FAILED));
            AppError {
                kind: AppErrorKind::Configuration,
                message: format!("whisper-local: context load failed: {e:?}"),
                user_message: Some(keys::LOAD_FAILED.to_string()),
                retryable: false,
            }
        })?;

        Ok(Self {
            ctx: Arc::new(Mutex::new(ctx)),
            language,
        })
    }
}

#[async_trait]
impl PipelineStage for WhisperLocal {
    type Input = AudioBuffer;
    type Output = String;

    async fn process(&self, audio: AudioBuffer) -> Result<String, AppError> {
        let ctx = Arc::clone(&self.ctx);
        let lang = self.language.clone();

        tokio::task::spawn_blocking(move || {
            let guard = ctx.lock().map_err(|_| AppError {
                kind: AppErrorKind::Internal,
                message: "whisper-local: mutex poisoned".to_string(),
                user_message: Some(keys::INFERENCE_FAILED.to_string()),
                retryable: false,
            })?;

            let mut state = guard.create_state().map_err(|e| {
                debug_assert!(i18n::is_key(keys::INFERENCE_FAILED));
                AppError {
                    kind: AppErrorKind::Internal,
                    message: format!("whisper-local: create_state failed: {e:?}"),
                    user_message: Some(keys::INFERENCE_FAILED.to_string()),
                    retryable: false,
                }
            })?;

            // FullParams has lifetime parameters — constructed inside the closure
            // so the closure remains 'static (ADR-0014 D-3 / AC-4 note).
            let mut params =
                whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 0 });
            if let Some(ref lang_code) = lang {
                params.set_language(Some(lang_code.as_str()));
            }
            // Suppress console output from whisper.cpp (noisy on some builds)
            params.set_print_special(false);
            params.set_print_progress(false);
            params.set_print_realtime(false);
            params.set_print_timestamps(false);

            state.full(params, &audio.samples).map_err(|e| AppError {
                kind: AppErrorKind::Internal,
                message: format!("whisper-local: inference failed: {e:?}"),
                user_message: Some(keys::INFERENCE_FAILED.to_string()),
                retryable: false,
            })?;

            let n_segments = state.full_n_segments().map_err(|e| AppError {
                kind: AppErrorKind::Internal,
                message: format!("whisper-local: full_n_segments failed: {e:?}"),
                user_message: Some(keys::INFERENCE_FAILED.to_string()),
                retryable: false,
            })?;

            let mut result = String::new();
            for i in 0..n_segments {
                match state.full_get_segment_text(i) {
                    Ok(seg) => {
                        let trimmed = seg.trim();
                        if !trimmed.is_empty() {
                            if !result.is_empty() {
                                result.push(' ');
                            }
                            result.push_str(trimmed);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "klarvo.stt.whisper_local",
                            segment = i,
                            error = ?e,
                            "segment text extraction failed; skipping"
                        );
                    }
                }
            }

            Ok::<String, AppError>(result)
        })
        .await
        .map_err(|join_err| AppError {
            kind: AppErrorKind::Internal,
            message: format!("whisper-local: spawn_blocking panic: {join_err}"),
            user_message: Some(keys::INFERENCE_FAILED.to_string()),
            retryable: false,
        })?
    }

    fn stage_type(&self) -> &'static str {
        "stt"
    }
}

#[async_trait]
impl SttProvider for WhisperLocal {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compile-time: WhisperLocal implements SttProvider + PipelineStage.
    /// If this test file compiles, the trait bounds are satisfied.
    #[allow(dead_code)]
    fn _assert_trait_bounds(plugin: WhisperLocal) {
        let _: &dyn klarvo_core::traits::SttProvider = &plugin;
    }

    #[test]
    fn load_rejects_nonexistent_model_path() {
        let result = WhisperLocal::load(
            std::path::Path::new("/does/not/exist/model.gguf"),
            Some("en".to_string()),
        );
        let err = match result {
            Ok(_) => panic!("must fail for nonexistent path"),
            Err(e) => e,
        };
        assert_eq!(err.user_message.as_deref(), Some(keys::MODEL_NOT_FOUND));
        assert!(matches!(
            err.kind,
            klarvo_core::error::AppErrorKind::Configuration
        ));
    }

    #[test]
    fn plugin_id_constant() {
        assert_eq!(ID, "whisper-local");
    }

    #[test]
    fn i18n_keys_are_valid() {
        klarvo_core::i18n::assert_is_key(keys::MODEL_NOT_FOUND);
        klarvo_core::i18n::assert_is_key(keys::LOAD_FAILED);
        klarvo_core::i18n::assert_is_key(keys::INFERENCE_FAILED);
    }
}
