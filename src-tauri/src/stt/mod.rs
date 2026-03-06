//! Speech-to-text module.
//!
//! Defines the `SttProvider` trait and the `GroqWhisper` implementation that
//! calls the Groq Whisper API (OpenAI-compatible).
//!
//! API docs: <https://console.groq.com/docs/speech-text>
//!
//! ## `prompt` parameter
//!
//! The Groq Whisper API accepts an optional `prompt` field (max 224 tokens)
//! that acts as a transcription hint. We use it to inject dictionary terms so
//! rare technical words and names are recognised correctly.
//!
//! The `SttProvider` trait exposes this as `prompt: Option<&str>`. Backends
//! that don't support a prompt parameter (e.g. local whisper.cpp in a future
//! implementation) can simply ignore it.

use reqwest::multipart;
use serde::Deserialize;
use thiserror::Error;

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
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over speech-to-text backends (Groq, local whisper.cpp, etc.).
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
// Groq Whisper response types
// ---------------------------------------------------------------------------

/// Successful transcription response from Groq.
#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Error response returned by the Groq API.
#[derive(Debug, Deserialize)]
struct ApiErrorResponse {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    message: String,
}

// ---------------------------------------------------------------------------
// GroqWhisper
// ---------------------------------------------------------------------------

/// Groq Whisper API client.
///
/// Uses `whisper-large-v3-turbo` by default -- 3x cheaper than v3 with
/// negligible quality difference for dictation.
pub struct GroqWhisper {
    api_key: String,
    client: reqwest::Client,
    model: String,
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
            api_key: api_key.into(),
            client: reqwest::Client::new(),
            model: Self::DEFAULT_MODEL.to_string(),
        }
    }

    /// Override the Whisper model variant (e.g. `"whisper-large-v3"`).
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Builds the multipart form for the transcription request.
    ///
    /// Extracted to a separate method so it can be tested without a live
    /// HTTP connection.
    ///
    /// `prompt` is appended when non-empty (max 224 tokens per Groq docs).
    fn build_form(
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
            .text("response_format", "json");

        if !language.is_empty() {
            form = form.text("language", language.to_string());
        }

        // Inject dictionary terms as a transcription hint.
        // Groq uses this to improve recognition of rare technical words.
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
impl SttProvider for GroqWhisper {
    /// Sends audio to the Groq Whisper API and returns the transcribed text.
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
            .post(Self::BASE_URL)
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

        Ok(result.text)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_groq_whisper_new_stores_api_key() {
        let stt = GroqWhisper::new("test-key-12345");
        assert_eq!(stt.api_key, "test-key-12345");
        assert_eq!(stt.model, GroqWhisper::DEFAULT_MODEL);
    }

    #[test]
    fn test_groq_whisper_with_model_overrides_default() {
        let stt = GroqWhisper::new("key").with_model("whisper-large-v3");
        assert_eq!(stt.model, "whisper-large-v3");
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
}
