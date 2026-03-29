//! Local LLM cleanup via llama.cpp (offline, desktop-only).
//!
//! Implements [`CleanupProvider`] using the `llama-cpp-2` crate for on-device
//! inference. No internet connection required. The model (GGUF format) must
//! be downloaded separately via the model manager.
//!
//! Currently desktop-only. Android uses KlarvoApi.kt for LLM calls instead.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;

use super::{CleanupProvider, CleanupResult, CleanupStyle, LlmError};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_MAX_TOKENS: u32 = 2048;
const DEFAULT_CONTEXT_SIZE: u32 = 4096;
const DEFAULT_TEMPERATURE: f32 = 0.3;

// ---------------------------------------------------------------------------
// Loaded model state
// ---------------------------------------------------------------------------

/// Holds the llama.cpp backend and loaded model.
///
/// The backend must outlive the model. Both are created once (lazy) and reused
/// across inference calls. A new context is created per inference call because
/// `LlamaContext` is not `Send`.
struct LoadedModel {
    backend: LlamaBackend,
    model: LlamaModel,
}

// SAFETY: LlamaBackend and LlamaModel are read-only after creation.
// Each inference call creates its own LlamaContext (which is !Send).
unsafe impl Send for LoadedModel {}
unsafe impl Sync for LoadedModel {}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Offline LLM cleanup provider using llama.cpp.
///
/// The model is loaded lazily on the first inference call. Subsequent calls
/// reuse the loaded model (only a fresh context is created each time).
///
/// The loaded model state is stored behind an `Arc<Mutex<_>>` so it can be
/// cheaply cloned into `spawn_blocking` closures without holding a `&self`
/// reference across the async boundary.
pub struct LocalLlmCleanup {
    model_path: PathBuf,
    // Arc so we can clone a handle into spawn_blocking without borrowing &self.
    state: Arc<Mutex<Option<LoadedModel>>>,
}

impl LocalLlmCleanup {
    /// Creates a new provider pointing at a GGUF model file.
    ///
    /// The model is NOT loaded yet — that happens on the first `cleanup()` call.
    pub fn new(model_path: PathBuf) -> Self {
        Self {
            model_path,
            state: Arc::new(Mutex::new(None)),
        }
    }

    /// Ensures the model is loaded, loading it if necessary.
    ///
    /// Takes `model_path` as a separate argument rather than `&self` so it can
    /// be called from a `spawn_blocking` closure.
    fn ensure_loaded_inner(
        state: &Arc<Mutex<Option<LoadedModel>>>,
        model_path: &std::path::Path,
    ) -> Result<(), LlmError> {
        let mut guard = state
            .lock()
            .map_err(|e| LlmError::InferenceError(format!("Lock poisoned: {e}")))?;

        if guard.is_some() {
            return Ok(());
        }

        if !model_path.exists() {
            return Err(LlmError::ModelNotFound(model_path.display().to_string()));
        }

        log::info!("Loading local LLM model: {}", model_path.display());

        let backend = LlamaBackend::init()
            .map_err(|e| LlmError::InferenceError(format!("Backend init failed: {e}")))?;

        let model_params = LlamaModelParams::default();
        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .map_err(|e| LlmError::InferenceError(format!("Model load failed: {e}")))?;

        log::info!("Local LLM model loaded successfully");
        *guard = Some(LoadedModel { backend, model });
        Ok(())
    }

    /// Runs inference with the given prompt text.
    ///
    /// Creates a fresh context for each call (contexts are cheap, models are expensive).
    /// Takes `state` and `model_path` by value so this can be called from a
    /// `spawn_blocking` closure without a `&self` lifetime.
    fn generate_inner(
        state: Arc<Mutex<Option<LoadedModel>>>,
        model_path: PathBuf,
        prompt: String,
    ) -> Result<String, LlmError> {
        Self::ensure_loaded_inner(&state, &model_path)?;

        let guard = state
            .lock()
            .map_err(|e| LlmError::InferenceError(format!("Lock poisoned: {e}")))?;
        let loaded = guard
            .as_ref()
            .expect("model must be loaded after ensure_loaded_inner");

        // Create a fresh context for this inference call.
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(DEFAULT_CONTEXT_SIZE));
        let mut ctx = loaded
            .model
            .new_context(&loaded.backend, ctx_params)
            .map_err(|e| LlmError::InferenceError(format!("Context creation failed: {e}")))?;

        // Tokenize the prompt.
        let tokens = loaded
            .model
            .str_to_token(&prompt, AddBos::Always)
            .map_err(|e| LlmError::InferenceError(format!("Tokenization failed: {e}")))?;

        if tokens.is_empty() {
            return Err(LlmError::EmptyInput);
        }

        // Feed prompt tokens into the model.
        let n_prompt = tokens.len();
        let mut batch = LlamaBatch::new(n_prompt.max(512), 1);
        let last_idx = n_prompt - 1;
        for (i, &token) in tokens.iter().enumerate() {
            let is_last = i == last_idx;
            batch
                .add(token, i as i32, &[0], is_last)
                .map_err(|e| LlmError::InferenceError(format!("Batch add failed: {e}")))?;
        }

        ctx.decode(&mut batch)
            .map_err(|e| LlmError::InferenceError(format!("Prompt decode failed: {e}")))?;

        // Sample tokens one by one.
        let mut sampler = LlamaSampler::chain_simple([
            LlamaSampler::temp(DEFAULT_TEMPERATURE),
            LlamaSampler::dist(42), // seed for reproducibility
        ]);

        // Decoder for token_to_piece (stateful: tracks multi-byte UTF-8 sequences).
        let mut decoder = encoding_rs::UTF_8.new_decoder();

        let mut output = String::new();
        let mut n_cur = batch.n_tokens();
        let max_total = n_prompt as i32 + DEFAULT_MAX_TOKENS as i32;

        loop {
            if n_cur >= max_total {
                log::warn!("Local LLM: max token limit reached");
                break;
            }

            let new_token = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(new_token);

            // Check for end-of-generation.
            if loaded.model.is_eog_token(new_token) {
                break;
            }

            // Decode token to text using the non-deprecated token_to_piece API.
            let piece = loaded
                .model
                .token_to_piece(new_token, &mut decoder, Special::Tokenize == Special::Tokenize, None)
                .map_err(|e| LlmError::InferenceError(format!("Token decode failed: {e}")))?;
            output.push_str(&piece);

            // Prepare next step.
            batch.clear();
            batch
                .add(new_token, n_cur, &[0], true)
                .map_err(|e| LlmError::InferenceError(format!("Batch add failed: {e}")))?;
            n_cur += 1;

            ctx.decode(&mut batch)
                .map_err(|e| LlmError::InferenceError(format!("Decode step failed: {e}")))?;
        }

        Ok(output.trim().to_string())
    }

    /// Builds a chat-formatted prompt for Qwen2.5-Instruct (ChatML format).
    fn build_prompt(system: &str, user: &str) -> String {
        format!(
            "<|im_start|>system\n{system}<|im_end|>\n\
             <|im_start|>user\n{user}<|im_end|>\n\
             <|im_start|>assistant\n"
        )
    }

    /// Spawns a blocking inference task and returns the result.
    ///
    /// Clones only the `Arc<Mutex<_>>` handle and the `PathBuf` (cheap) so the
    /// closure is `'static + Send` without borrowing `&self`.
    async fn run_inference(&self, prompt: String) -> Result<String, LlmError> {
        let state = Arc::clone(&self.state);
        let model_path = self.model_path.clone();

        tokio::task::spawn_blocking(move || Self::generate_inner(state, model_path, prompt))
            .await
            .map_err(|e| LlmError::InferenceError(format!("Task join failed: {e}")))?
    }
}

// ---------------------------------------------------------------------------
// CleanupProvider implementation
// ---------------------------------------------------------------------------

#[async_trait::async_trait]
impl CleanupProvider for LocalLlmCleanup {
    async fn cleanup(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
    ) -> Result<CleanupResult, LlmError> {
        if raw_text.trim().is_empty() {
            return Err(LlmError::EmptyInput);
        }

        let system = style.system_prompt(dictionary_terms, custom_prompt);
        let prompt = Self::build_prompt(&system, raw_text);

        let text = self.run_inference(prompt).await?;
        Ok(CleanupResult {
            text,
            prompt_tokens: None,
            completion_tokens: None,
        })
    }

    async fn cleanup_with_translation(
        &self,
        raw_text: &str,
        style: CleanupStyle,
        dictionary_terms: Option<&str>,
        custom_prompt: Option<&str>,
        output_language: Option<&str>,
    ) -> Result<CleanupResult, LlmError> {
        if raw_text.trim().is_empty() {
            return Err(LlmError::EmptyInput);
        }

        let system =
            style.system_prompt_with_translation(dictionary_terms, custom_prompt, output_language);
        let prompt = Self::build_prompt(&system, raw_text);

        let text = self.run_inference(prompt).await?;
        Ok(CleanupResult {
            text,
            prompt_tokens: None,
            completion_tokens: None,
        })
    }

    async fn rewrite(
        &self,
        selected_text: &str,
        voice_command: &str,
    ) -> Result<CleanupResult, LlmError> {
        let system = CleanupStyle::command_mode_system_prompt();
        let user = format!("Selected text:\n{selected_text}\n\nCommand: {voice_command}");
        let prompt = Self::build_prompt(system, &user);

        let text = self.run_inference(prompt).await?;
        Ok(CleanupResult {
            text,
            prompt_tokens: None,
            completion_tokens: None,
        })
    }

    async fn reformat(&self, text: &str, format: &str) -> Result<CleanupResult, LlmError> {
        let system = super::reformat_system_prompt(format);
        let prompt = Self::build_prompt(system, text);

        let result_text = self.run_inference(prompt).await?;
        Ok(CleanupResult {
            text: result_text,
            prompt_tokens: None,
            completion_tokens: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_format() {
        let prompt = LocalLlmCleanup::build_prompt("You are helpful.", "Hello");
        assert!(prompt.contains("<|im_start|>system\nYou are helpful.<|im_end|>"));
        assert!(prompt.contains("<|im_start|>user\nHello<|im_end|>"));
        assert!(prompt.ends_with("<|im_start|>assistant\n"));
    }

    #[test]
    fn new_does_not_load_model() {
        // Model path does not exist — loading should be deferred.
        let provider = LocalLlmCleanup::new(PathBuf::from("/nonexistent/model.gguf"));
        let guard = provider.state.lock().unwrap();
        assert!(guard.is_none(), "model should not be loaded eagerly");
    }

    #[test]
    fn model_not_found_error_on_missing_path() {
        let state = Arc::new(Mutex::new(None));
        let result =
            LocalLlmCleanup::ensure_loaded_inner(&state, std::path::Path::new("/no/such/file.gguf"));
        assert!(matches!(result, Err(LlmError::ModelNotFound(_))));
    }
}
