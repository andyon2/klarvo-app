//! `Groq` is the Groq Whisper HTTPS-API-backed `SttProvider` implementation. HTTPS stack
//! (reqwest + rustls-tls-native-roots + wiremock-for-tests) is locked per ADR-0005 and is the
//! reference-pattern for all Phase-2+ Cloud-Provider-Plugins (DeepSeek, OpenAI, Anthropic,
//! OpenRouter).
//!
//! The companion reference-plugin `klarvo-plugin-verbatim` (Story 1B.3) demonstrates the
//! opposite pole of the plugin-complexity-spectrum — KeyStore-free, external-API-free, pure
//! identity-passthrough. Together, these two plugins exercise both ends of the
//! `PipelineStage`-contract-surface.
//!
//! Registry-registration and Manifest-driven instantiation are Epic 3 scope
//! (Shell-owned KeyStore-Resolution). 1B.5 E2E-Executor-Test uses Verbatim-based pipelines
//! only. Pipeline-level retry orchestration for `UpstreamUnavailable` errors is Epic 2 FR29
//! (not 1B.4). Config-driven endpoint/model/timeout-override is Phase-2+ scope.
//!
//! Phase-1 prefers accuracy over latency — `whisper-large-v3` is the default;
//! `whisper-large-v3-turbo` and config-driven model-override are Phase-2+ scope.
//!
//! WAV-encoding via `hound` is plugin-local; factor-out to `klarvo-core::audio::wav` deferred
//! until Phase-2+ Cloud-STT-Plugins (OpenAI Whisper, DeepSeek) prove duplication — then as
//! dedicated micro-refactor story, not as premature-abstraction in Phase 1.
//!
//! The `key_store: Arc<dyn KeyStore>` field holds the API-key handle. The key is fetched lazily
//! at `transcribe()`-call-time via `GROQ_API_KEY_ID`. `Debug`/`Display` on `AppError` never
//! leaks the raw key; Bearer-header construction uses `expose_secret()` only inside the
//! `transcribe` HTTPS-call-site. The Auth-Leak-Test in `tests/external_contract.rs` locks this
//! invariant against code-changes; factor-out to an `assert_no_secret_leak!` macro in
//! `klarvo-test-fixtures` is a Phase-2+ pattern-signal once additional API-Key-bearing plugins
//! exist.

use std::io::Cursor;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;

use klarvo_core::audio::AudioBuffer;
use klarvo_core::error::{AppError, AppErrorKind};
use klarvo_core::i18n;
use klarvo_core::keystore::KeyStore;
use klarvo_core::pipeline::PipelineStage;
use klarvo_core::traits::SttProvider;

/// i18n error keys emitted by this plugin.
///
/// Catch-all `upstream_4xx` covers 402/404/405/etc.; not exercised in integration-tests
/// as these are rarely encountered in practice, but code-path is reachable.
pub mod keys {
    pub const NETWORK: &str = "error.stt.network";
    pub const TIMEOUT: &str = "error.stt.timeout";
    pub const UPSTREAM_5XX: &str = "error.stt.upstream_5xx";
    pub const RATE_LIMITED: &str = "error.stt.rate_limited";
    pub const AUTH_FAILED: &str = "error.stt.auth_failed";
    pub const INVALID_AUDIO: &str = "error.stt.invalid_audio";
    pub const UPSTREAM_4XX: &str = "error.stt.upstream_4xx";
    /// i18n-key surfaced when `KeyStore::get(GROQ_API_KEY_ID)` returns `KeyMissing`.
    /// Re-mapped from `KeyMissing` to `UpstreamUnavailable` at `transcribe()` call-site:
    /// from pipeline-perspective, Groq-STT is 'unavailable' when the API-key is not
    /// configured. Shell-layer translates this key to a user-facing message prompting
    /// key-setup (Epic 4 / Config-Layer).
    pub const KEY_NOT_CONFIGURED: &str = "error.stt.key_not_configured";
}

/// Plugin identifier — matches `plugin_id: "groq"` in pipeline manifests.
pub const ID: &str = "groq";

/// Identifier for the Groq API-key in the `KeyStore`.
/// Naming-prefix-convention: `groq_` prefix namespaces this key from other plugins
/// (e.g. `deepseek_api_key`). Multi-Account support (multiple Groq API-keys per user)
/// is Phase-2+ scope — would require `key_id: &str` injection rather than this
/// hardcoded const.
pub const GROQ_API_KEY_ID: &str = "groq_api_key";

const DEFAULT_ENDPOINT: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const DEFAULT_MODEL: &str = "whisper-large-v3";

/// Groq Whisper HTTPS-backed `SttProvider`. See module-level doc for scope and safety notes.
pub struct Groq {
    client: reqwest::Client,
    key_store: Arc<dyn KeyStore>,
    endpoint: String,
    model: String,
}

#[derive(Deserialize)]
struct GroqTranscriptionResponse {
    text: String,
}

impl Groq {
    /// Primary constructor. Uses default 30s-timeout client, production endpoint, whisper-large-v3.
    ///
    /// `key_store` is held as `Arc<dyn KeyStore>` — key is fetched lazily at
    /// `transcribe()`-call-time via `GROQ_API_KEY_ID`. Constructor never fails with a
    /// key-error; `UpstreamUnavailable` surfaces only at `transcribe()` if the key is
    /// absent. Production: construct via `bootstrap()` in Epic 3 (Shell owns
    /// KeyStore-Resolution, not Core-bootstrap).
    pub fn new(key_store: Arc<dyn KeyStore>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest ClientBuilder default build");
        Self {
            client,
            key_store,
            endpoint: DEFAULT_ENDPOINT.to_string(),
            model: DEFAULT_MODEL.to_string(),
        }
    }

    /// Test-only constructor. `endpoint` overrides the default Groq-API-URL — pass
    /// wiremock-server URI for integration-tests. `client` allows short-timeout-injection
    /// for timeout-test-scenarios (see `tests/external_contract.rs` Test-Case 6).
    pub fn new_with_client(
        key_store: Arc<dyn KeyStore>,
        endpoint: impl Into<String>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            client,
            key_store,
            endpoint: endpoint.into(),
            model: DEFAULT_MODEL.to_string(),
        }
    }
}

#[async_trait]
impl PipelineStage for Groq {
    type Input = AudioBuffer;
    type Output = String;

    async fn process(&self, audio: AudioBuffer) -> Result<String, AppError> {
        let api_key: SecretString = self
            .key_store
            .get(GROQ_API_KEY_ID)
            .await
            .map_err(|e| {
                debug_assert!(i18n::is_key(keys::KEY_NOT_CONFIGURED));
                AppError {
                    kind: AppErrorKind::UpstreamUnavailable,
                    message: format!("groq: API key not configured: {}", e.message),
                    user_message: Some(keys::KEY_NOT_CONFIGURED.to_string()),
                    retryable: false,
                }
            })?;

        let wav_bytes = encode_wav(&audio)?;

        let part = reqwest::multipart::Part::bytes(wav_bytes)
            .file_name("audio.wav")
            .mime_str("audio/wav")
            .map_err(|e| AppError {
                kind: AppErrorKind::Internal,
                message: format!("groq transcribe: mime type error: {e}"),
                user_message: None,
                retryable: false,
            })?;
        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone())
            .text("response_format", "json");

        // expose_secret() is used exclusively here — never in intermediate variables or errors.
        let resp = self
            .client
            .post(&self.endpoint)
            .bearer_auth(api_key.expose_secret())
            .multipart(form)
            .send()
            .await
            .map_err(map_reqwest_error)?;

        let status = resp.status();
        if !status.is_success() {
            let body_excerpt: String =
                resp.text().await.unwrap_or_default().chars().take(200).collect();
            return Err(classify_http_error(status, &body_excerpt));
        }

        let groq_resp = resp
            .json::<GroqTranscriptionResponse>()
            .await
            .map_err(|e| AppError {
                kind: AppErrorKind::UpstreamUnavailable,
                message: format!("groq transcribe: response parse error: {e}"),
                user_message: Some(keys::UPSTREAM_5XX.to_string()),
                retryable: false,
            })?;

        Ok(groq_resp.text)
    }

    fn stage_type(&self) -> &'static str {
        "stt"
    }
}

#[async_trait]
impl SttProvider for Groq {}

fn encode_wav(audio: &AudioBuffer) -> Result<Vec<u8>, AppError> {
    let mut cursor = Cursor::new(Vec::<u8>::new());
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio.sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    // Pass &mut cursor so the cursor remains accessible after finalize() (hound 3.5
    // WavWriter::finalize returns Result<()>, not the inner writer).
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).map_err(|e| AppError {
            kind: AppErrorKind::Internal,
            message: format!("groq transcribe: wav init error: {e}"),
            user_message: None,
            retryable: false,
        })?;
        for &sample in &audio.samples {
            writer.write_sample(sample).map_err(|e| AppError {
                kind: AppErrorKind::Internal,
                message: format!("groq transcribe: wav write error: {e}"),
                user_message: None,
                retryable: false,
            })?;
        }
        writer.finalize().map_err(|e| AppError {
            kind: AppErrorKind::Internal,
            message: format!("groq transcribe: wav finalize error: {e}"),
            user_message: None,
            retryable: false,
        })?;
    }
    Ok(cursor.into_inner())
}

fn map_reqwest_error(e: reqwest::Error) -> AppError {
    let key = if e.is_timeout() {
        keys::TIMEOUT
    } else {
        // Covers: is_connect() (DNS/TCP/TLS), is_request() without status, other transport errors.
        keys::NETWORK
    };
    debug_assert!(i18n::is_key(key));
    AppError {
        kind: AppErrorKind::UpstreamUnavailable,
        message: format!("groq transcribe: {e}"),
        user_message: Some(key.to_string()),
        retryable: false,
    }
}

fn classify_http_error(status: reqwest::StatusCode, body_excerpt: &str) -> AppError {
    let key = match status.as_u16() {
        401 | 403 => keys::AUTH_FAILED,
        400 => keys::INVALID_AUDIO,
        429 => keys::RATE_LIMITED,
        500 | 502 | 503 | 504 => keys::UPSTREAM_5XX,
        c if (400..500).contains(&c) => keys::UPSTREAM_4XX,
        _ => keys::UPSTREAM_5XX,
    };
    debug_assert!(i18n::is_key(key));
    AppError {
        kind: AppErrorKind::UpstreamUnavailable,
        message: format!("groq transcribe: status={status}; body={body_excerpt}"),
        user_message: Some(key.to_string()),
        retryable: false,
    }
}
