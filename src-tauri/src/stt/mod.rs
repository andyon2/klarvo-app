//! Speech-to-text module.
//!
//! Defines the `SttProvider` trait and concrete implementations:
//! - `GroqWhisper`: Groq Whisper API (OpenAI-compatible, fast/cheap)
//! - `OpenAiWhisper`: OpenAI Whisper API (OpenAI-compatible, `whisper-1`)
//!
//! Both cloud providers use the same multipart/form-data format, so they share
//! the generic `WhisperStt` struct parameterized by base URL and model name.
//!
//! API docs:
//! - Groq: <https://console.groq.com/docs/speech-text>
//! - OpenAI: <https://platform.openai.com/docs/api-reference/audio/createTranscription>
//!
//! ## `prompt` parameter
//!
//! The Whisper API accepts an optional `prompt` field (max 224 tokens)
//! that acts as a transcription hint. We use it to inject dictionary terms so
//! rare technical words and names are recognised correctly.
//!
//! The `SttProvider` trait exposes this as `prompt: Option<&str>`. Backends
//! that don't support a prompt parameter (e.g. local whisper.cpp in a future
//! implementation) can simply ignore it.

use reqwest::multipart;
use serde::Deserialize;
use thiserror::Error;

// Sub-modules
pub mod local_whisper;
#[cfg(target_os = "windows")]
pub use local_whisper::LocalWhisperProvider;

pub mod model_manager;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during speech-to-text transcription.
#[derive(Debug, Error)]
pub enum SttError {
    #[error("HTTP request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("API error {status}: {message}")]
    ApiError { status: u16, message: String },

    #[error("Unexpected response format: {0}")]
    ResponseFormat(String),

    #[error("Audio data is empty")]
    EmptyAudio,

    /// Error from the local whisper.cpp backend (offline STT).
    /// The inner string contains the formatted `LocalWhisperError` message.
    #[error("Local whisper error: {0}")]
    LocalWhisper(String),
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over speech-to-text backends (Groq, OpenAI, local whisper.cpp, etc.).
///
/// Implementations receive raw WAV bytes and return the transcribed text.
///
/// Parameters:
/// - `audio`: raw WAV bytes.
/// - `language`: ISO-639-1 code (e.g. `"de"`, `"en"`). Empty string = auto-detect.
/// - `prompt`: optional hint for the STT model. Used to inject dictionary
///   terms so rare words are recognised correctly. Backends that do not
///   support a prompt can ignore this parameter.
#[async_trait::async_trait]
pub trait SttProvider: Send + Sync {
    async fn transcribe(
        &self,
        audio: Vec<u8>,
        language: &str,
        prompt: Option<&str>,
    ) -> Result<String, SttError>;
}

// ---------------------------------------------------------------------------
// Prompt builder
// ---------------------------------------------------------------------------

/// Builds the full Whisper `prompt` string from dictionary terms and language.
///
/// When `language` is `"de"`, a code-switching hint is prepended so Whisper
/// preserves embedded English words instead of germanising them.
///
/// If `custom_hint` is `Some(s)` and non-empty, it is used instead of the
/// built-in language hint. Dictionary terms are still appended after it.
///
/// Returns `None` when the resulting prompt would be empty.
pub fn build_stt_prompt(dict_terms: Option<&str>, language: &str) -> Option<String> {
    build_stt_prompt_with_hint(dict_terms, language, None)
}

/// Like `build_stt_prompt` but accepts an optional custom hint that overrides
/// the built-in language-specific conditioning text.
///
/// When `custom_hint` is `Some(s)` and non-empty, it replaces the default
/// language hint. Dictionary terms are still appended after the hint.
pub fn build_stt_prompt_with_hint(
    dict_terms: Option<&str>,
    language: &str,
    custom_hint: Option<&str>,
) -> Option<String> {
    // If the caller supplied a non-empty custom hint, use that instead of the
    // built-in language-specific conditioning text.
    let hint: &str = match custom_hint {
        Some(h) if !h.trim().is_empty() => h,
        _ => match language {
            "de" => "Diktat auf Deutsch mit gelegentlichen englischen Fachbegriffen. Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion. ",
            "en" => "Voice dictation in English. Proper punctuation, capitalization, and spelling. ",
            _ => "Multilingual voice dictation. German and English with proper punctuation. ",
        },
    };

    let terms = dict_terms.unwrap_or("");

    let combined = format!("{hint}{terms}");
    let trimmed = combined.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

// ---------------------------------------------------------------------------
// Shared response types (both Groq and OpenAI return identical JSON)
// ---------------------------------------------------------------------------

/// Successful transcription response (OpenAI-compatible format).
#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Error response returned by OpenAI-compatible APIs.
#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
}

// ---------------------------------------------------------------------------
// WhisperStt -- generic OpenAI-compatible Whisper client
// ---------------------------------------------------------------------------

/// Generic Whisper STT client for any OpenAI-compatible `/audio/transcriptions`
/// endpoint.
///
/// Both `GroqWhisper` and `OpenAiWhisper` are thin wrappers around this struct
/// with different base URLs and default models.
pub struct WhisperStt {
    api_key: String,
    client: reqwest::Client,
    base_url: String,
    model: String,
    /// Whisper sampling temperature. 0.0 = deterministic (default).
    temperature: f32,
}

impl WhisperStt {
    /// Creates a new `WhisperStt` client.
    ///
    /// - `api_key`: Bearer token for the API.
    /// - `base_url`: Full URL of the transcriptions endpoint
    ///   (e.g. `"https://api.groq.com/openai/v1/audio/transcriptions"`).
    /// - `model`: Model identifier (e.g. `"whisper-large-v3-turbo"`, `"whisper-1"`).
    pub fn new(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        WhisperStt {
            api_key: api_key.into(),
            client: reqwest::Client::new(),
            base_url: base_url.into(),
            model: model.into(),
            temperature: 0.0,
        }
    }

    /// Override the model variant.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Override the Whisper sampling temperature.
    ///
    /// 0.0 (the default) produces deterministic output; higher values increase
    /// randomness. Values outside `[0.0, 1.0]` are clamped by the Whisper API.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    /// Builds the multipart form for the transcription request.
    ///
    /// Extracted to a separate method so it can be tested without a live
    /// HTTP connection.
    ///
    /// `prompt` is appended when non-empty (max 224 tokens per Whisper docs).
    pub fn build_form(
        &self,
        audio: Vec<u8>,
        language: &str,
        prompt: Option<&str>,
    ) -> Result<multipart::Form, reqwest::Error> {
        let part = multipart::Part::bytes(audio)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .expect("audio/wav is a valid MIME type");

        let mut form = multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("response_format", "json")
            .text("temperature", self.temperature.to_string());

        if !language.is_empty() {
            form = form.text("language", language.to_string());
        }

        // Inject dictionary terms as a transcription hint.
        if let Some(p) = prompt {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                form = form.text("prompt", trimmed.to_string());
            }
        }

        Ok(form)
    }
}

#[async_trait::async_trait]
impl SttProvider for WhisperStt {
    /// Sends audio to the Whisper API endpoint and returns the transcribed text.
    ///
    /// # Errors
    /// - `SttError::EmptyAudio` -- `audio` is empty.
    /// - `SttError::Request` -- network or serialization failure.
    /// - `SttError::ApiError` -- the API returned a non-2xx status.
    /// - `SttError::ResponseFormat` -- the response JSON was unexpected.
    async fn transcribe(
        &self,
        audio: Vec<u8>,
        language: &str,
        prompt: Option<&str>,
    ) -> Result<String, SttError> {
        if audio.is_empty() {
            return Err(SttError::EmptyAudio);
        }

        let form = self.build_form(audio, language, prompt)?;

        let response = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let status_code = status.as_u16();
            // Try to extract the API error message; fall back to raw text.
            let body = response.text().await.unwrap_or_default();
            let message = serde_json::from_str::<ApiErrorResponse>(&body)
                .map(|e| e.error.message)
                .unwrap_or(body);
            return Err(SttError::ApiError {
                status: status_code,
                message,
            });
        }

        let result: TranscriptionResponse = response.json().await?;

        if result.text.is_empty() {
            return Err(SttError::ResponseFormat(
                "API returned empty text field".to_string(),
            ));
        }

        Ok(result.text.trim().to_string())
    }
}

// ---------------------------------------------------------------------------
// GroqWhisper -- thin wrapper around WhisperStt
// ---------------------------------------------------------------------------

/// Groq Whisper API client.
///
/// Uses `whisper-large-v3-turbo` by default -- 3x cheaper than v3 with
/// negligible quality difference for dictation.
pub struct GroqWhisper {
    inner: WhisperStt,
}

impl GroqWhisper {
    const BASE_URL: &'static str = "https://api.groq.com/openai/v1/audio/transcriptions";
    const DEFAULT_MODEL: &'static str = "whisper-large-v3-turbo";

    /// Creates a new `GroqWhisper` client with the given API key.
    ///
    /// The API key should come from the caller (environment variable or
    /// system keystore) -- never hard-coded.
    pub fn new(api_key: impl Into<String>) -> Self {
        GroqWhisper {
            inner: WhisperStt::new(api_key, Self::BASE_URL, Self::DEFAULT_MODEL),
        }
    }

    /// Override the Whisper model variant (e.g. `"whisper-large-v3"`).
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.with_model(model);
        self
    }

    /// Override the Whisper sampling temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.inner = self.inner.with_temperature(temperature);
        self
    }

    /// Returns the configured API key (for testing).
    #[cfg(test)]
    pub fn api_key(&self) -> &str {
        &self.inner.api_key
    }

    /// Returns the configured model (for testing).
    #[cfg(test)]
    pub fn model(&self) -> &str {
        &self.inner.model
    }

    /// Builds the multipart form (exposed for tests).
    #[cfg(test)]
    pub fn build_form(
        &self,
        audio: Vec<u8>,
        language: &str,
        prompt: Option<&str>,
    ) -> Result<multipart::Form, reqwest::Error> {
        self.inner.build_form(audio, language, prompt)
    }
}

#[async_trait::async_trait]
impl SttProvider for GroqWhisper {
    async fn transcribe(
        &self,
        audio: Vec<u8>,
        language: &str,
        prompt: Option<&str>,
    ) -> Result<String, SttError> {
        self.inner.transcribe(audio, language, prompt).await
    }
}

// ---------------------------------------------------------------------------
// OpenAiWhisper -- OpenAI Whisper API client
// ---------------------------------------------------------------------------

/// OpenAI Whisper API client (`whisper-1`).
///
/// Uses the same multipart/form-data format as Groq but against the OpenAI
/// endpoint. Good fallback when Groq rate limits are hit.
pub struct OpenAiWhisper {
    inner: WhisperStt,
}

impl OpenAiWhisper {
    const BASE_URL: &'static str = "https://api.openai.com/v1/audio/transcriptions";
    const DEFAULT_MODEL: &'static str = "whisper-1";

    /// Creates a new `OpenAiWhisper` client with the given API key.
    ///
    /// The API key should come from the caller (environment variable or
    /// system keystore) -- never hard-coded.
    pub fn new(api_key: impl Into<String>) -> Self {
        OpenAiWhisper {
            inner: WhisperStt::new(api_key, Self::BASE_URL, Self::DEFAULT_MODEL),
        }
    }

    /// Override the model variant.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.with_model(model);
        self
    }

    /// Override the Whisper sampling temperature.
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.inner = self.inner.with_temperature(temperature);
        self
    }

    /// Returns the configured API key (for testing).
    #[cfg(test)]
    pub fn api_key(&self) -> &str {
        &self.inner.api_key
    }

    /// Returns the configured model (for testing).
    #[cfg(test)]
    pub fn model(&self) -> &str {
        &self.inner.model
    }

    /// Builds the multipart form (exposed for tests).
    #[cfg(test)]
    pub fn build_form(
        &self,
        audio: Vec<u8>,
        language: &str,
        prompt: Option<&str>,
    ) -> Result<multipart::Form, reqwest::Error> {
        self.inner.build_form(audio, language, prompt)
    }
}

#[async_trait::async_trait]
impl SttProvider for OpenAiWhisper {
    async fn transcribe(
        &self,
        audio: Vec<u8>,
        language: &str,
        prompt: Option<&str>,
    ) -> Result<String, SttError> {
        self.inner.transcribe(audio, language, prompt).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- GroqWhisper tests ---

    #[test]
    fn test_groq_whisper_new_stores_api_key() {
        let stt = GroqWhisper::new("test-key-12345");
        assert_eq!(stt.api_key(), "test-key-12345");
        assert_eq!(stt.model(), GroqWhisper::DEFAULT_MODEL);
    }

    #[test]
    fn test_groq_whisper_with_model_overrides_default() {
        let stt = GroqWhisper::new("key").with_model("whisper-large-v3");
        assert_eq!(stt.model(), "whisper-large-v3");
    }

    /// Verifies that the form can be built without panicking for non-empty audio.
    #[test]
    fn test_build_form_with_language() {
        let stt = GroqWhisper::new("key");
        let dummy_audio = vec![0u8; 128];
        let form = stt.build_form(dummy_audio, "de", None);
        assert!(form.is_ok(), "build_form should succeed for valid input");
    }

    #[test]
    fn test_build_form_without_language() {
        let stt = GroqWhisper::new("key");
        let dummy_audio = vec![0u8; 128];
        let form = stt.build_form(dummy_audio, "", None);
        assert!(form.is_ok(), "build_form should succeed with empty language");
    }

    /// Verifies that build_form accepts a non-empty prompt without error.
    #[test]
    fn test_build_form_with_prompt() {
        let stt = GroqWhisper::new("key");
        let dummy_audio = vec![0u8; 128];
        let form = stt.build_form(dummy_audio, "de", Some("Kubernetes, TypeScript, Dikta"));
        assert!(form.is_ok(), "build_form should succeed with a prompt");
    }

    /// Empty prompt string is treated the same as None (not added to form).
    #[test]
    fn test_build_form_empty_prompt_is_ignored() {
        let stt = GroqWhisper::new("key");
        let dummy_audio = vec![0u8; 128];
        let form_none = stt.build_form(dummy_audio.clone(), "de", None);
        let form_empty = stt.build_form(dummy_audio, "de", Some(""));
        // Both should succeed; we can't inspect form internals but we verify
        // no error is returned.
        assert!(form_none.is_ok());
        assert!(form_empty.is_ok());
    }

    /// Verifies that empty audio is rejected before hitting the network.
    #[tokio::test]
    async fn test_transcribe_empty_audio_returns_error() {
        let stt = GroqWhisper::new("dummy-key");
        let result = stt.transcribe(vec![], "en", None).await;
        assert!(
            matches!(result, Err(SttError::EmptyAudio)),
            "expected EmptyAudio error, got: {result:?}"
        );
    }

    /// Empty audio is rejected even when a prompt is provided.
    #[tokio::test]
    async fn test_transcribe_empty_audio_with_prompt_returns_error() {
        let stt = GroqWhisper::new("dummy-key");
        let result = stt
            .transcribe(vec![], "de", Some("Kubernetes"))
            .await;
        assert!(
            matches!(result, Err(SttError::EmptyAudio)),
            "expected EmptyAudio error, got: {result:?}"
        );
    }

    // --- OpenAiWhisper tests ---

    #[test]
    fn test_openai_whisper_new_stores_api_key() {
        let stt = OpenAiWhisper::new("sk-openai-test-key");
        assert_eq!(stt.api_key(), "sk-openai-test-key");
        assert_eq!(stt.model(), OpenAiWhisper::DEFAULT_MODEL);
    }

    #[test]
    fn test_openai_whisper_default_model_is_whisper_1() {
        let stt = OpenAiWhisper::new("key");
        assert_eq!(stt.model(), "whisper-1");
    }

    #[test]
    fn test_openai_whisper_with_model_overrides_default() {
        let stt = OpenAiWhisper::new("key").with_model("whisper-2");
        assert_eq!(stt.model(), "whisper-2");
    }

    #[test]
    fn test_openai_whisper_build_form_succeeds() {
        let stt = OpenAiWhisper::new("key");
        let form = stt.build_form(vec![0u8; 128], "en", None);
        assert!(form.is_ok(), "build_form should succeed for OpenAiWhisper");
    }

    #[test]
    fn test_openai_whisper_build_form_with_prompt() {
        let stt = OpenAiWhisper::new("key");
        let form = stt.build_form(vec![0u8; 128], "de", Some("TypeScript, Kubernetes"));
        assert!(form.is_ok(), "build_form should accept a prompt");
    }

    #[tokio::test]
    async fn test_openai_whisper_empty_audio_returns_error() {
        let stt = OpenAiWhisper::new("dummy-key");
        let result = stt.transcribe(vec![], "en", None).await;
        assert!(
            matches!(result, Err(SttError::EmptyAudio)),
            "expected EmptyAudio error, got: {result:?}"
        );
    }

    // --- WhisperStt generic tests ---

    #[test]
    fn test_whisper_stt_with_custom_base_url() {
        let stt = WhisperStt::new("key", "https://custom.example.com/v1/audio/transcriptions", "my-model");
        assert_eq!(stt.base_url, "https://custom.example.com/v1/audio/transcriptions");
        assert_eq!(stt.model, "my-model");
    }

    #[tokio::test]
    async fn test_whisper_stt_empty_audio_returns_error() {
        let stt = WhisperStt::new("key", "https://api.groq.com/openai/v1/audio/transcriptions", "whisper-large-v3-turbo");
        let result = stt.transcribe(vec![], "en", None).await;
        assert!(matches!(result, Err(SttError::EmptyAudio)));
    }

    // --- build_stt_prompt tests ---

    #[test]
    fn test_build_stt_prompt_german_with_terms() {
        let result = build_stt_prompt(Some("Kubernetes, TypeScript"), "de");
        let prompt = result.expect("should produce a prompt");
        assert!(prompt.contains("Deutsch"), "should have German hint");
        assert!(prompt.contains("Kubernetes, TypeScript"), "should contain dictionary terms");
    }

    #[test]
    fn test_build_stt_prompt_german_without_terms() {
        let result = build_stt_prompt(None, "de");
        let prompt = result.expect("should produce a prompt for German even without terms");
        assert!(prompt.contains("Deutsch"));
    }

    #[test]
    fn test_build_stt_prompt_english_with_terms() {
        let result = build_stt_prompt(Some("Kubernetes"), "en");
        let prompt = result.expect("should produce a prompt");
        assert!(prompt.contains("Kubernetes"), "should contain dictionary terms");
        assert!(prompt.contains("English"), "should have English hint");
    }

    #[test]
    fn test_build_stt_prompt_english_without_terms() {
        let result = build_stt_prompt(None, "en");
        let prompt = result.expect("should produce a prompt for English");
        assert!(prompt.contains("English"));
    }

    #[test]
    fn test_build_stt_prompt_auto_detect_with_terms() {
        let result = build_stt_prompt(Some("Dikta"), "");
        let prompt = result.expect("should produce a prompt");
        assert!(prompt.contains("Dikta"), "should contain dictionary terms");
        assert!(prompt.contains("Multilingual"), "should have multilingual hint");
    }

    #[test]
    fn test_build_stt_prompt_auto_detect_without_terms() {
        let result = build_stt_prompt(None, "");
        let prompt = result.expect("should produce a prompt even without terms");
        assert!(prompt.contains("Multilingual"));
    }

    // --- build_stt_prompt_with_hint tests ---

    #[test]
    fn test_build_stt_prompt_with_custom_hint_overrides_default() {
        let result = build_stt_prompt_with_hint(None, "de", Some("Mein benutzerdefinierter Hint. "));
        let prompt = result.expect("should produce a prompt");
        assert!(prompt.contains("benutzerdefinierter"), "custom hint should be used");
        // Built-in German hint should NOT be present
        assert!(!prompt.contains("Diktat auf Deutsch"), "built-in hint should be replaced");
    }

    #[test]
    fn test_build_stt_prompt_with_custom_hint_and_terms() {
        let result = build_stt_prompt_with_hint(Some("Kubernetes, TypeScript"), "de", Some("Custom hint. "));
        let prompt = result.expect("should produce a prompt");
        assert!(prompt.contains("Custom hint"), "custom hint should appear");
        assert!(prompt.contains("Kubernetes"), "dictionary terms should still appear");
    }

    #[test]
    fn test_build_stt_prompt_empty_custom_hint_falls_back_to_default() {
        let result_none = build_stt_prompt_with_hint(None, "de", None);
        let result_empty = build_stt_prompt_with_hint(None, "de", Some(""));
        let result_whitespace = build_stt_prompt_with_hint(None, "de", Some("   "));
        // All three should fall back to the built-in German hint
        for result in [result_none, result_empty, result_whitespace] {
            let prompt = result.expect("should produce a prompt");
            assert!(prompt.contains("Deutsch"), "should fall back to German built-in hint");
        }
    }

    // --- with_temperature tests ---

    #[test]
    fn test_whisper_stt_default_temperature_is_zero() {
        let stt = WhisperStt::new("key", "https://api.groq.com/openai/v1/audio/transcriptions", "whisper-large-v3-turbo");
        assert_eq!(stt.temperature, 0.0);
    }

    #[test]
    fn test_groq_whisper_with_temperature() {
        let stt = GroqWhisper::new("key").with_temperature(0.5);
        // Build a form and verify it doesn't error out
        let form = stt.build_form(vec![0u8; 128], "de", None);
        assert!(form.is_ok(), "build_form should succeed with non-zero temperature");
    }

    #[test]
    fn test_openai_whisper_with_temperature() {
        let stt = OpenAiWhisper::new("key").with_temperature(0.3);
        let form = stt.build_form(vec![0u8; 128], "en", None);
        assert!(form.is_ok(), "build_form should succeed with non-zero temperature");
    }
}
