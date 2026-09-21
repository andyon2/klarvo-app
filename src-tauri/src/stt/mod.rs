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
pub mod hallucination;
pub use hallucination::{is_hallucination, strip_stockphrase_ghosts};

pub mod local_whisper;
#[cfg(any(target_os = "windows", target_os = "android"))]
pub use local_whisper::LocalWhisperProvider;

pub mod model_manager;

#[cfg(target_os = "android")]
pub mod jni_bridge;

// Not `#[cfg(target_os = "android")]` as a whole: the JNI entry points inside are
// android-only and carry their own gate, but the provider choice they make
// (`groq_jni::select_stt_provider`) is plain Rust and must compile — and be
// tested — on the platform the `cargo test --lib` gate runs on. Story 13-1: an
// android-gated selector is a selector no executing test can reach.
pub mod groq_jni;

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
/// - `audio`: raw WAV bytes, borrowed — callers that need to preserve the
///   audio for a fallback/error path (finding G) keep ownership instead of
///   paying for a defensive clone before every attempt.
/// - `language`: ISO-639-1 code (e.g. `"de"`, `"en"`). Empty string = auto-detect.
/// - `prompt`: optional hint for the STT model. Used to inject dictionary
///   terms so rare words are recognised correctly. Backends that do not
///   support a prompt can ignore this parameter.
#[async_trait::async_trait]
pub trait SttProvider: Send + Sync {
    async fn transcribe(
        &self,
        audio: &[u8],
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

/// Successful transcription response (`response_format=json`).
#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    text: String,
}

/// Successful transcription response (`response_format=verbose_json`).
///
/// Groq returns the `text` field plus an optional `segments` array with
/// per-segment metadata. Fields that may not be present are `Option` so
/// the parser tolerates both legacy and extended responses (AC6 fail-soft).
#[derive(Debug, Deserialize)]
struct VerboseTranscriptionResponse {
    text: String,
    #[serde(default)]
    segments: Vec<TranscriptionSegment>,
}

/// A single segment in a `verbose_json` response.
///
/// All confidence fields are `Option` — the API may omit them (or add new ones
/// in future). A missing field is treated as "not low confidence" (fail-open).
#[derive(Debug, Deserialize)]
struct TranscriptionSegment {
    /// Segment text content.
    text: String,
    /// Probability of no speech. High (>0.6) → likely silence/noise → drop.
    #[serde(default)]
    no_speech_prob: Option<f64>,
    /// Ratio of output token length to input audio length. Very low (<0.1) or
    /// very high (>2.4) can indicate hallucination loops.
    #[serde(default)]
    compression_ratio: Option<f64>,
    /// Average log-probability of tokens. Very low (<-1.0) → low confidence → drop.
    #[serde(default)]
    avg_logprob: Option<f64>,
}

impl TranscriptionSegment {
    /// Returns `true` if this segment should be dropped based on confidence
    /// thresholds (AC6).
    ///
    /// ## Thresholds (golden-vector seeds, locked by 7.7 parity net)
    ///
    /// - `no_speech_prob > 0.6`: segment is more likely silence than speech.
    /// - `compression_ratio < 0.1`: near-empty output for audio length (silence drop).
    /// - `avg_logprob < -1.0`: very low token confidence.
    ///
    /// Missing fields are treated as "not low confidence" (fail-open: do NOT drop).
    /// This is intentional: unknown fields preserve the segment, not discard it.
    pub fn should_drop(&self) -> bool {
        if let Some(nsp) = self.no_speech_prob {
            if nsp > 0.6 {
                return true;
            }
        }
        if let Some(cr) = self.compression_ratio {
            if cr < 0.1 {
                return true;
            }
        }
        if let Some(alp) = self.avg_logprob {
            if alp < -1.0 {
                return true;
            }
        }
        false
    }
}

/// Extracts the transcribed text from a `verbose_json` response, dropping
/// low-confidence segments (AC6).
///
/// Segments are joined by a single space. If all segments are dropped, returns
/// an empty string (pipeline will treat it as empty and skip paste).
fn extract_verbose_text(resp: VerboseTranscriptionResponse) -> String {
    if resp.segments.is_empty() {
        // No segments — fall back to the top-level `text` field (fail-soft).
        return resp.text.trim().to_string();
    }
    resp.segments
        .into_iter()
        .filter(|s| !s.should_drop())
        .map(|s| s.text.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
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
            client: reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(15))
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: base_url.into(),
            model: model.into(),
            temperature: 0.0,
        }
    }

    /// Override the model variant.
    #[allow(dead_code)] // builder API, used by GroqWhisper/OpenAiWhisper wrappers
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Override the Whisper sampling temperature.
    ///
    /// 0.0 (the default) produces deterministic output; higher values increase
    /// randomness. Values outside `[0.0, 1.0]` are clamped by the Whisper API.
    #[allow(dead_code)] // builder API for future use
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
            // AC6: verbose_json returns per-segment confidence metadata used for
            // segment drop (no_speech_prob / compression_ratio / avg_logprob).
            .text("response_format", "verbose_json")
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
        audio: &[u8],
        language: &str,
        prompt: Option<&str>,
    ) -> Result<String, SttError> {
        if audio.is_empty() {
            return Err(SttError::EmptyAudio);
        }

        // `build_form` needs an owned buffer (multipart::Part::bytes requires
        // `'static` ownership) — this copy is intrinsic to constructing the
        // HTTP request body, unlike the eager defensive clone removed from
        // the pipeline caller (finding G).
        let form = self.build_form(audio.to_vec(), language, prompt)?;

        let response = self
            .client
            .post(&self.base_url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        map_transcription_http_response(response).await
    }
}

// ---------------------------------------------------------------------------
// Response → Result mapping (shared by WhisperStt and DebugStt)
// ---------------------------------------------------------------------------

/// Maps a transcription HTTP response to the transcript text.
///
/// Extracted **verbatim** from `WhisperStt::transcribe` (story 13-1,
/// behaviour-preserving) so [`DebugStt`] can drive the same mapping from a
/// synthesised response. Every `SttError` variant the live path produced it
/// still produces.
///
/// ⚠ Note the asymmetry with the LLM twin ([`crate::llm::map_chat_http_response`]),
/// which is real, pre-existing and deliberately NOT normalised away here: this
/// path reads `bytes()` and parses with `serde_json::from_slice`, so an
/// undecodable body becomes `SttError::ResponseFormat` — **non-retryable** —
/// whereas the LLM path's `response.json()` yields a retryable
/// `LlmError::Request`. The debug provider reports whatever each twin's own
/// mapping does; it does not harmonise them.
///
/// AC6: parses the `verbose_json` response with both-shape tolerance. It
/// attempts `verbose_json` first (the format we request). If the `segments`
/// field is absent (plain json response or future API change), serde still
/// succeeds because `segments` has `#[serde(default)]` → empty Vec. Both shapes
/// have a top-level `text` field which is the final fallback.
pub(crate) async fn map_transcription_http_response(
    response: reqwest::Response,
) -> Result<String, SttError> {
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

    let body = response.bytes().await?;

    let text = if let Ok(verbose) = serde_json::from_slice::<VerboseTranscriptionResponse>(&body) {
        // Verbose response (or plain json that deserializes with empty segments).
        let t = extract_verbose_text(verbose);
        if t.is_empty() {
            // All segments dropped or empty text field — treat as no speech.
            log::debug!("[stt] all segments dropped or empty after verbose_json parse");
            return Err(SttError::ResponseFormat(
                "API returned empty text after segment confidence filter".to_string(),
            ));
        }
        t
    } else {
        // Fallback: try legacy plain json shape (future-compat / fail-soft).
        match serde_json::from_slice::<TranscriptionResponse>(&body) {
            Ok(r) => {
                if r.text.is_empty() {
                    return Err(SttError::ResponseFormat(
                        "API returned empty text field".to_string(),
                    ));
                }
                r.text.trim().to_string()
            }
            Err(e) => {
                return Err(SttError::ResponseFormat(format!(
                    "Cannot parse STT response (tried verbose_json and json): {e}"
                )));
            }
        }
    };

    Ok(text)
}

// ---------------------------------------------------------------------------
// DebugStt -- canned wire responses for reproducing STT anomalies
// (Story 13-1, the H+ enabler for drift row D9/D-M5+D-M6)
// ---------------------------------------------------------------------------

/// The canned `(status, body)` pair for a debug STT scenario.
///
/// `None` means "no response at all" — the caller performs a real request to
/// [`crate::llm::DEBUG_TRANSPORT_URL`] (the loopback discard port) instead.
///
/// The scenario set is the LLM set **minus `truncated`**: [`SttError`] has no
/// truncation variant, so there is nothing for a truncated STT answer to map
/// to. An unrecognised scenario resolves like `"ok"`.
pub(crate) fn debug_stt_canned_wire(scenario: &str) -> Option<(u16, &'static str)> {
    match scenario {
        // 200 with an empty transcript: the verbose parse succeeds and
        // `extract_verbose_text` yields "" → ResponseFormat (drift row D-M5).
        "empty" => Some((200, r#"{"text":"","segments":[]}"#)),
        // 200 with a body that deserializes as neither shape. Unlike the LLM
        // twin this is NON-retryable: the real STT path reads `bytes()` and
        // parses with `serde_json::from_slice`, so the failure surfaces as
        // `SttError::ResponseFormat`, not as the `reqwest` decode error
        // (`SttError::Request`) that `response.json()` would have produced. The
        // asymmetry is real and pre-existing; it is pinned as a finding, not
        // normalised away.
        "malformed" => Some((200, r#"{"unexpected":"debug provider malformed body"}"#)),
        "http429" => Some((
            429,
            r#"{"error":{"message":"Debug provider: simulated rate limit"}}"#,
        )),
        "http5xx" => Some((
            503,
            r#"{"error":{"message":"Debug provider: simulated server error"}}"#,
        )),
        "transport" => None,
        // "ok" and any unrecognised value (including "truncated", which has no
        // SttError counterpart and is therefore not offered in the STT picker)
        _ => Some((
            200,
            r#"{"text":"Debug provider canned transcript.","segments":[]}"#,
        )),
    }
}

/// An STT provider that never talks to a real API: it yields the canned wire
/// response selected by `advanced.debugSttScenario` and lets the *real* mapping
/// ([`map_transcription_http_response`]) decide the outcome.
///
/// Lives in Rust only — ADR-0017 makes STT shared core, so Android consumes
/// this same provider through `stt::groq_jni::nativeTranscribe` and the
/// `__ERROR_*` sentinels it already emits. A Kotlin-side canned transcript would
/// *forge* those sentinels rather than produce them.
///
/// The provider ignores `audio`, `language` and `prompt` entirely — the
/// scenario alone decides the answer, and in particular there is **no
/// `EmptyAudio` guard**. That is load-bearing: it lets a Rust unit test drive
/// `groq_jni::select_stt_provider("debug", …)` with empty audio and still reach
/// the canned mapping, while the same call with `"groq"` stops at `EmptyAudio`
/// before any socket is opened.
pub struct DebugStt {
    scenario: String,
}

impl DebugStt {
    pub fn new(scenario: impl Into<String>) -> Self {
        DebugStt {
            scenario: scenario.into(),
        }
    }
}

#[async_trait::async_trait]
impl SttProvider for DebugStt {
    async fn transcribe(
        &self,
        _audio: &[u8],
        _language: &str,
        _prompt: Option<&str>,
    ) -> Result<String, SttError> {
        log::info!(
            "[stt] DEBUG STT provider active: scenario={}",
            self.scenario
        );
        match debug_stt_canned_wire(&self.scenario) {
            Some((status, body)) => match crate::llm::debug_canned_response(status, body) {
                // The synthesised response goes through the REAL mapping, so the
                // outcome is whatever the live path would produce for these bytes.
                Some(response) => map_transcription_http_response(response).await,
                None => Err(SttError::ResponseFormat(format!(
                    "Debug provider: invalid canned status {status}"
                ))),
            },
            None => {
                // `transport`: a real request that cannot succeed. The `?` turns
                // the connection failure into `SttError::Request`, the same
                // variant a real transport failure produces.
                //
                // Same builder as `WhisperStt::new` above, so a host that DROPs
                // rather than REFUSEs loopback:1 fails in 15s instead of hanging
                // the pipeline (and `cargo test --lib`) forever. Plus
                // `.no_proxy()`: with `HTTP_PROXY` set, reqwest would otherwise
                // route this probe through the proxy and the "no byte leaves the
                // device" claim on `DEBUG_TRANSPORT_URL` would be false.
                let response = reqwest::Client::builder()
                    .connect_timeout(std::time::Duration::from_secs(15))
                    .timeout(std::time::Duration::from_secs(30))
                    .no_proxy()
                    .build()
                    .unwrap_or_else(|_| reqwest::Client::new())
                    .post(crate::llm::DEBUG_TRANSPORT_URL)
                    .send()
                    .await?;
                map_transcription_http_response(response).await
            }
        }
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
    #[allow(dead_code)] // builder API for future use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.with_model(model);
        self
    }

    /// Override the Whisper sampling temperature.
    #[allow(dead_code)] // builder API for future use
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
        audio: &[u8],
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
    #[allow(dead_code)] // builder API for future use
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.inner = self.inner.with_model(model);
        self
    }

    /// Override the Whisper sampling temperature.
    #[allow(dead_code)] // builder API for future use
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
        audio: &[u8],
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
        let form = stt.build_form(dummy_audio, "de", Some("Kubernetes, TypeScript, Klarvo"));
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
        let result = stt.transcribe(&[], "en", None).await;
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
            .transcribe(&[], "de", Some("Kubernetes"))
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
        let result = stt.transcribe(&[], "en", None).await;
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
        let result = stt.transcribe(&[], "en", None).await;
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
        let result = build_stt_prompt(Some("Klarvo"), "");
        let prompt = result.expect("should produce a prompt");
        assert!(prompt.contains("Klarvo"), "should contain dictionary terms");
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

    // --- AC6: verbose_json + confidence-drop golden-vector fixtures ---
    // These are SEEDS for the 7.7 parity net. The thresholds (no_speech_prob > 0.6,
    // compression_ratio < 0.1, avg_logprob < -1.0) are defined in TranscriptionSegment::should_drop.

    #[test]
    fn test_ac6_segment_drop_high_no_speech_prob() {
        // no_speech_prob > 0.6 → segment is likely silence/noise → drop.
        let seg = TranscriptionSegment {
            text: "ZDF 2020".to_string(),
            no_speech_prob: Some(0.85),
            compression_ratio: None,
            avg_logprob: None,
        };
        assert!(seg.should_drop(), "high no_speech_prob must trigger drop");
    }

    #[test]
    fn test_ac6_segment_drop_low_compression_ratio() {
        // compression_ratio < 0.1 → near-empty output → drop.
        let seg = TranscriptionSegment {
            text: "...".to_string(),
            no_speech_prob: None,
            compression_ratio: Some(0.05),
            avg_logprob: None,
        };
        assert!(seg.should_drop(), "low compression_ratio must trigger drop");
    }

    #[test]
    fn test_ac6_segment_drop_low_avg_logprob() {
        // avg_logprob < -1.0 → very low token confidence → drop.
        let seg = TranscriptionSegment {
            text: "amara.org".to_string(),
            no_speech_prob: None,
            compression_ratio: None,
            avg_logprob: Some(-1.5),
        };
        assert!(seg.should_drop(), "low avg_logprob must trigger drop");
    }

    #[test]
    fn test_ac6_segment_keep_good_confidence() {
        // All confidence values within acceptable range → keep.
        let seg = TranscriptionSegment {
            text: "Ich brauche die Unterlagen bis Freitag.".to_string(),
            no_speech_prob: Some(0.05),
            compression_ratio: Some(1.5),
            avg_logprob: Some(-0.3),
        };
        assert!(!seg.should_drop(), "good confidence segment must be kept");
    }

    #[test]
    fn test_ac6_segment_missing_fields_fail_open() {
        // Missing confidence fields → do NOT drop (fail-open: unknown = keep).
        let seg = TranscriptionSegment {
            text: "Bitte send mir die Unterlagen.".to_string(),
            no_speech_prob: None,
            compression_ratio: None,
            avg_logprob: None,
        };
        assert!(!seg.should_drop(), "missing confidence fields must not cause drop");
    }

    #[test]
    fn test_ac6_extract_verbose_text_drops_low_confidence() {
        // A response with one good segment and one bad segment → only good text returned.
        let resp = VerboseTranscriptionResponse {
            text: "Bitte schick mir die Datei ZDF".to_string(),
            segments: vec![
                TranscriptionSegment {
                    text: "Bitte schick mir die Datei".to_string(),
                    no_speech_prob: Some(0.05),
                    compression_ratio: Some(1.2),
                    avg_logprob: Some(-0.2),
                },
                TranscriptionSegment {
                    text: "ZDF".to_string(),
                    no_speech_prob: Some(0.9),  // high → drop
                    compression_ratio: None,
                    avg_logprob: None,
                },
            ],
        };
        let result = extract_verbose_text(resp);
        assert_eq!(result, "Bitte schick mir die Datei", "low-confidence segment must be dropped");
    }

    #[test]
    fn test_ac6_extract_verbose_text_both_shapes_tolerated() {
        // A plain-json-like verbose response (no segments) falls back to top-level text.
        let resp = VerboseTranscriptionResponse {
            text: "Fallback text".to_string(),
            segments: vec![],  // empty segments = plain json shape
        };
        let result = extract_verbose_text(resp);
        assert_eq!(result, "Fallback text", "empty segments must fall back to top-level text");
    }

    #[test]
    fn test_ac6_no_speech_prob_boundary_exactly_06_is_kept() {
        // Boundary: exactly 0.6 is NOT dropped (> 0.6, not >=).
        let seg = TranscriptionSegment {
            text: "Grenzfall".to_string(),
            no_speech_prob: Some(0.6),
            compression_ratio: None,
            avg_logprob: None,
        };
        assert!(!seg.should_drop(), "exactly 0.6 no_speech_prob must be kept (boundary check)");
    }

    #[test]
    fn test_ac6_avg_logprob_boundary_exactly_minus1_is_kept() {
        // Boundary: exactly -1.0 is NOT dropped (< -1.0, not <=).
        let seg = TranscriptionSegment {
            text: "Grenzfall".to_string(),
            no_speech_prob: None,
            compression_ratio: None,
            avg_logprob: Some(-1.0),
        };
        assert!(!seg.should_drop(), "exactly -1.0 avg_logprob must be kept (boundary check)");
    }

    // -----------------------------------------------------------------------
    // Story 13-1 — debug test provider (STT half, shared core per ADR-0017)
    //
    // Driven by the SAME test-fixtures/debug-provider-scenario-vectors.json the
    // LLM half in llm/mod.rs and the Kotlin twin's DebugProviderScenarioTest
    // read. Every assertion compares a PRODUCTION seam
    // (`debug_stt_canned_wire`, `DebugStt::transcribe`) against the FIXTURE
    // literal, never against another production symbol.
    //
    // There is no Kotlin twin for these vectors by construction: ADR-0017 makes
    // the STT request and its guards shared Rust core, consumed on Android over
    // `stt::groq_jni::nativeTranscribe`. The Kotlin column of every `stt` vector
    // says so, and the Kotlin test skips them by surface.
    // -----------------------------------------------------------------------

    fn load_debug_vectors() -> Vec<serde_json::Value> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let path = std::path::Path::new(&manifest_dir)
            .parent()
            .expect("workspace root")
            .join("test-fixtures/debug-provider-scenario-vectors.json");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));
        serde_json::from_str(&content)
            .expect("debug-provider-scenario-vectors.json must be a JSON array")
    }

    /// Throwing lookup — a missing id must fail loudly, never skip the assertion.
    fn debug_vector(vectors: &[serde_json::Value], id: &str) -> serde_json::Value {
        vectors
            .iter()
            .find(|v| v["id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("fixture has no vector with id={id}"))
            .clone()
    }

    /// The fixture's symbolic error name for an `SttError`. Exhaustive on
    /// purpose: a new variant must be named here, not silently bucketed.
    fn stt_error_name(e: &SttError) -> &'static str {
        match e {
            SttError::Request(_) => "Request",
            SttError::ApiError { .. } => "ApiError",
            SttError::ResponseFormat(_) => "ResponseFormat",
            SttError::EmptyAudio => "EmptyAudio",
            SttError::LocalWhisper(_) => "LocalWhisper",
        }
    }

    /// Drives the REAL provider for one fixture vector and asserts both halves:
    /// the canned bytes it puts on the wire, and the verdict the real mapping
    /// returns for them.
    async fn assert_stt_vector(id: &str) {
        let vectors = load_debug_vectors();
        let v = debug_vector(&vectors, id);
        assert_eq!(v["surface"].as_str(), Some("stt"), "{id} is not an stt vector");
        assert_eq!(
            v["kotlin"]["outcome"].as_str(),
            Some("n/a"),
            "{id}: an STT vector has no Kotlin twin (ADR-0017) and must say so"
        );
        let scenario = v["scenario"].as_str().expect("scenario");

        // (a) the canned wire bytes, against the fixture literal.
        let wire = &v["wire"];
        match wire["kind"].as_str().expect("wire.kind") {
            "canned" => {
                let (status, body) = debug_stt_canned_wire(scenario)
                    .unwrap_or_else(|| panic!("{id}: {scenario} must produce a canned wire"));
                assert_eq!(
                    u64::from(status),
                    wire["status"].as_u64().expect("wire.status"),
                    "{id}: canned status must match the fixture"
                );
                assert_eq!(
                    body,
                    wire["body"].as_str().expect("wire.body"),
                    "{id}: canned body must match the fixture"
                );
            }
            "loopback" => {
                assert!(
                    debug_stt_canned_wire(scenario).is_none(),
                    "{id}: {scenario} must have NO canned wire (it performs a real loopback request)"
                );
                assert_eq!(
                    crate::llm::DEBUG_TRANSPORT_URL,
                    wire["url"].as_str().expect("wire.url"),
                    "{id}: loopback URL must match the fixture"
                );
            }
            // `truncated` is an expected-absence vector: it is not offered in
            // the STT picker, so the provider must treat it like any other
            // unknown string (fail-soft to `ok`), not invent an outcome.
            "not_offered" => {
                let ok_wire = debug_stt_canned_wire("ok").expect("ok must have a canned wire");
                assert_eq!(
                    debug_stt_canned_wire(scenario),
                    Some(ok_wire),
                    "{id}: a not-offered scenario must fall back to the ok wire"
                );
            }
            other => panic!("{id}: unknown wire.kind {other}"),
        }

        // (b) the verdict the REAL mapping returns, against the fixture literal.
        let provider = DebugStt::new(scenario);
        let result = provider.transcribe(b"not-a-real-wav", "de", None).await;
        let expected = &v["rust"];
        match expected["outcome"].as_str().expect("rust.outcome") {
            "ok" => {
                let text = result.unwrap_or_else(|e| panic!("{id}: expected Ok, got {e:?}"));
                assert_eq!(
                    text,
                    expected["text"].as_str().expect("rust.text"),
                    "{id}: canned transcript must match the fixture"
                );
            }
            "error" => {
                let e = match result {
                    Err(e) => e,
                    Ok(t) => panic!("{id}: expected an error, got Ok({t:?})"),
                };
                assert_eq!(
                    stt_error_name(&e),
                    expected["error"].as_str().expect("rust.error"),
                    "{id}: error variant must match the fixture (got {e:?})"
                );
                if let Some(status) = expected["error_status"].as_u64() {
                    match &e {
                        SttError::ApiError { status: got, .. } => {
                            assert_eq!(u64::from(*got), status, "{id}: HTTP status must match")
                        }
                        other => panic!("{id}: error_status stated but variant is {other:?}"),
                    }
                }
                if let Some(needle) = expected["message_contains"].as_str() {
                    let msg = e.to_string();
                    assert!(
                        msg.contains(needle),
                        "{id}: error message {msg:?} must contain {needle:?}"
                    );
                }
                // (c) whether the production retry fires for this shape, from
                // the real predicate. `DEBUG-STT-MALFORMED-001` pins `false`
                // here while its LLM twin pins `true` — the LLM↔STT
                // retryability asymmetry, recorded rather than normalised away.
                if let Some(want) = expected["retryable"].as_bool() {
                    assert_eq!(
                        crate::pipeline::is_retryable_stt_error(&e),
                        want,
                        "{id}: is_retryable_stt_error must say {want} for {e:?}"
                    );
                }
            }
            other => panic!("{id}: unknown rust.outcome {other}"),
        }
    }

    /// The six STT scenarios the Advanced → System picker offers are the LLM set
    /// MINUS `truncated` — `SttError` has no truncation variant, so there would
    /// be nothing for it to map to.
    #[test]
    fn spec_debug_stt_scenario_set_is_the_llm_set_minus_truncated() {
        let vectors = load_debug_vectors();
        let llm: Vec<&str> = vectors
            .iter()
            .filter(|v| v["surface"].as_str() == Some("llm"))
            .map(|v| v["scenario"].as_str().expect("scenario"))
            .collect();
        let offered: Vec<&str> = vectors
            .iter()
            .filter(|v| {
                v["surface"].as_str() == Some("stt")
                    && v["wire"]["kind"].as_str() != Some("not_offered")
            })
            .map(|v| v["scenario"].as_str().expect("scenario"))
            .collect();
        let expected: Vec<&str> = llm.into_iter().filter(|s| *s != "truncated").collect();
        assert_eq!(
            offered, expected,
            "the STT scenario set is the contract with AdvancedSettingsPanel's option list"
        );
    }

    #[tokio::test]
    async fn spec_debug_stt_ok() {
        assert_stt_vector("DEBUG-STT-OK-001").await;
    }

    #[tokio::test]
    async fn spec_debug_stt_empty() {
        assert_stt_vector("DEBUG-STT-EMPTY-001").await;
    }

    #[tokio::test]
    async fn spec_debug_stt_malformed() {
        assert_stt_vector("DEBUG-STT-MALFORMED-001").await;
    }

    #[tokio::test]
    async fn spec_debug_stt_http429() {
        assert_stt_vector("DEBUG-STT-HTTP429-001").await;
    }

    #[tokio::test]
    async fn spec_debug_stt_http5xx() {
        assert_stt_vector("DEBUG-STT-HTTP5XX-001").await;
    }

    /// Performs a REAL request to the loopback discard port. No byte leaves the
    /// device and nothing listens there, so this is hermetic and fast.
    #[tokio::test]
    async fn spec_debug_stt_transport() {
        assert_stt_vector("DEBUG-STT-TRANSPORT-001").await;
    }

    /// `truncated` is not offered on the STT side; the provider must fail soft
    /// to `ok` rather than panic or invent an outcome.
    #[tokio::test]
    async fn spec_debug_stt_truncated_is_not_offered() {
        assert_stt_vector("DEBUG-STT-NO-TRUNCATED-001").await;
    }

    /// Synthesises the response the same way `DebugStt` does.
    fn canned(status: u16, body: &str) -> reqwest::Response {
        crate::llm::debug_canned_response(status, body).expect("canned status must be valid")
    }

    /// The extraction that made the debug provider possible must not have moved
    /// the real mapping: the same wire bytes `transcribe` would see yield the
    /// same verdicts they yielded before.
    #[tokio::test]
    async fn spec_map_transcription_http_response_preserves_the_real_mapping() {
        // verbose_json with segments → low-confidence segments are dropped.
        let t = map_transcription_http_response(canned(
            200,
            r#"{"text":"a b","segments":[{"text":"a","no_speech_prob":0.05},{"text":"b","no_speech_prob":0.9}]}"#,
        ))
        .await
        .expect("well-formed verbose body must map to Ok");
        assert_eq!(t, "a");

        // plain json shape (no segments) → top-level text.
        let t = map_transcription_http_response(canned(200, r#"{"text":"  hello  "}"#))
            .await
            .expect("plain json must map to Ok");
        assert_eq!(t, "hello");

        // A non-2xx prefers the API's own error message …
        match map_transcription_http_response(canned(401, r#"{"error":{"message":"bad key"}}"#))
            .await
        {
            Err(SttError::ApiError { status, message }) => {
                assert_eq!(status, 401);
                assert_eq!(message, "bad key");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
        // … and falls back to the raw body when there is none.
        match map_transcription_http_response(canned(500, "upstream exploded")).await {
            Err(SttError::ApiError { status, message }) => {
                assert_eq!(status, 500);
                assert_eq!(message, "upstream exploded");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Story 13-1 — the ANDROID debug branch (groq_jni::select_stt_provider)
    //
    // The review's one unfixed `high` was that no executing test reached the
    // Android debug branch: it sat inside an `extern "system"` fn, on a target
    // the test gate never builds. The selector is now a plain function, and
    // these two tests are the pair that makes inverting its comparison fail.
    //
    // The discriminator is EMPTY AUDIO: `DebugStt` ignores the audio entirely,
    // so it reaches its canned mapping, while `WhisperStt::transcribe` returns
    // `EmptyAudio` before it builds a request. Neither opens a socket.
    // -----------------------------------------------------------------------

    /// `select_stt_provider("debug", …)` must hand back the debug provider.
    ///
    /// Inversion (verified RED at writing time): flipping the comparison in
    /// `select_stt_provider` to `!=` makes this return `EmptyAudio` instead of
    /// the canned transcript.
    #[tokio::test]
    async fn spec_android_select_stt_provider_reaches_the_debug_branch() {
        let provider = crate::stt::groq_jni::select_stt_provider(
            crate::llm::DEBUG_PROVIDER_NAME,
            "ok",
            "", // no API key: a debug run must not need one
            "whisper-large-v3-turbo",
            0.0,
        );
        let text = provider
            .transcribe(b"", "de", None)
            .await
            .expect("the debug provider ignores the audio and answers from its canned wire");
        let vectors = load_debug_vectors();
        let ok = debug_vector(&vectors, "DEBUG-STT-OK-001");
        assert_eq!(text, ok["rust"]["text"].as_str().expect("rust.text"));
    }

    /// The other half: every other provider name keeps the production Groq path,
    /// which stops at `EmptyAudio` before any network call. Without this the
    /// test above could pass with a selector that always returns `DebugStt`.
    #[tokio::test]
    async fn spec_android_select_stt_provider_keeps_groq_for_every_other_name() {
        for name in ["groq", "openai", "", "Debug", "debugx"] {
            let provider = crate::stt::groq_jni::select_stt_provider(
                name,
                "ok",
                "gsk-not-a-real-key",
                "whisper-large-v3-turbo",
                0.0,
            );
            let err = provider
                .transcribe(b"", "de", None)
                .await
                .expect_err("the Groq path must refuse empty audio before it opens a socket");
            assert!(
                matches!(err, SttError::EmptyAudio),
                "provider name {name:?} must keep the Groq path, got {err:?}"
            );
        }
    }

    /// The scenario argument is carried through to the provider, not ignored —
    /// otherwise an Android debug run would always replay `ok`.
    #[tokio::test]
    async fn spec_android_select_stt_provider_carries_the_scenario() {
        let provider = crate::stt::groq_jni::select_stt_provider(
            crate::llm::DEBUG_PROVIDER_NAME,
            "http429",
            "",
            "whisper-large-v3-turbo",
            0.0,
        );
        let err = provider
            .transcribe(b"", "de", None)
            .await
            .expect_err("http429 must be an error");
        assert!(
            matches!(err, SttError::ApiError { status: 429, .. }),
            "the selected scenario must reach the provider, got {err:?}"
        );
    }
}
