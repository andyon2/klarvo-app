//! Tauri commands for in-app feedback submissions and diagnostic metrics.
//!
//! The feedback feature is operator-controlled: it is only active when
//! `feedbackWebhookUrl` is set in `config.json`. End users cannot configure
//! this through the Settings UI.
//!
//! ## Metrics
//!
//! [`FeedbackMetrics`] is accumulated by the dictation pipeline and stored in
//! [`AppState`]. When the user opens the feedback dialog, the frontend calls
//! [`get_feedback_metrics`] to prefill the form with fresh telemetry. On a
//! successful [`send_feedback`] call the cumulative error counts are reset to
//! zero so the next submission starts clean.
//!
//! ## Privacy
//!
//! Raw transcription text and cleaned text are only included in the webhook
//! payload when the caller passes `include_dictation: true`. The field is
//! always `None` otherwise.

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::license::status_to_string;
use crate::AppState;

/// Filename for the feedback metrics JSON file written by Kotlin on Android.
/// On mobile the dictation pipeline runs in Kotlin, so metrics are passed to
/// Rust through this file (written to `app_data_dir`).
const METRICS_FILENAME: &str = "feedback_metrics.json";

// ---------------------------------------------------------------------------
// Metrics
// ---------------------------------------------------------------------------

/// Diagnostic metrics accumulated during the dictation pipeline.
///
/// Written after each successful dictation (latency fields) and on every
/// STT / LLM / paste error (error counters). Stored in `AppState` behind a
/// `Mutex` so pipeline code can update it from any async context.
///
/// All fields are `Option` or default-constructible so `Default::default()`
/// yields a zeroed-out struct that is safe to return before the first
/// dictation completes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackMetrics {
    // --- Last dictation latencies ---
    /// STT step duration for the most recent dictation (milliseconds).
    pub last_stt_latency_ms: Option<u64>,
    /// LLM cleanup step duration for the most recent dictation (milliseconds).
    /// `None` when the last dictation ran in offline mode (no LLM call).
    pub last_llm_latency_ms: Option<u64>,
    /// End-to-end pipeline duration for the most recent dictation (milliseconds).
    /// Measured from recording-stop to paste-complete.
    pub last_total_latency_ms: Option<u64>,

    // --- Last dictation context ---
    /// Window title of the app that was focused when the last dictation started.
    /// Populated on Windows (Win32 `GetForegroundWindow`). `None` on platforms
    /// where foreground-window capture is not implemented.
    pub last_target_app: Option<String>,
    /// ISO 8601 timestamp of the last completed dictation (UTC).
    pub last_dictation_at: Option<String>,

    // --- Last dictation text (opt-in only) ---
    /// Raw Whisper transcript of the last dictation.
    /// Only populated when `include_dictation: true` is passed to `send_feedback`.
    /// Stored here so the pipeline can always write it; the command decides
    /// whether to include it in the outbound payload.
    pub last_raw_text: Option<String>,
    /// LLM-cleaned text from the last dictation.
    /// Same opt-in semantics as `last_raw_text`.
    pub last_cleaned_text: Option<String>,

    // --- Cumulative error counters (since app start) ---
    /// Number of STT transcription errors since the app started.
    pub stt_error_count: u32,
    /// Number of LLM cleanup errors since the app started.
    pub llm_error_count: u32,
    /// Number of paste errors since the app started.
    pub paste_error_count: u32,
}

// ---------------------------------------------------------------------------
// Webhook payload
// ---------------------------------------------------------------------------

/// JSON body sent to the feedback webhook.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FeedbackPayload {
    /// Feedback category: `"problem"`, `"idea"`, `"question"`, or `"praise"`.
    category: String,
    /// Free-form feedback text entered by the user.
    message: String,
    /// Optional reply-to address provided by the user.
    email: Option<String>,
    /// UI area where feedback was triggered (e.g. `"home"`, `"settings"`,
    /// `"settings/sync"`). Helps triage reports without follow-up questions.
    context_area: String,
    /// User-selected feature area (e.g. `"Audio"`, `"Text Cleanup"`).
    /// `None` if the user did not select one.
    area: Option<String>,
    /// Klarvo version string from `Cargo.toml` (e.g. `"0.5.0"`).
    version: String,
    /// Operating system name as reported by `std::env::consts::OS`
    /// (e.g. `"windows"`, `"linux"`, `"android"`).
    os: String,
    /// License status string at the time of submission
    /// (e.g. `"licensed"`, `"trial"`, `"unlicensed"`).
    license_status: String,
    /// Runtime platform: `"desktop"` or `"mobile"`.
    platform: String,

    // --- Metrics ---
    /// STT step latency of the most recent dictation (milliseconds).
    stt_latency_ms: Option<u64>,
    /// LLM cleanup latency of the most recent dictation (milliseconds).
    llm_latency_ms: Option<u64>,
    /// End-to-end pipeline latency of the most recent dictation (milliseconds).
    total_latency_ms: Option<u64>,
    /// Window title of the app targeted by the last dictation.
    last_target_app: Option<String>,
    /// ISO 8601 timestamp of the last completed dictation.
    last_dictation_at: Option<String>,
    /// Cumulative STT error count since app start.
    stt_error_count: u32,
    /// Cumulative LLM error count since app start.
    llm_error_count: u32,
    /// Cumulative paste error count since app start.
    paste_error_count: u32,

    // --- Opt-in dictation sample ---
    /// Raw Whisper transcript of the last dictation.
    /// `None` unless `include_dictation: true` was requested by the caller.
    raw_text: Option<String>,
    /// LLM-cleaned text of the last dictation.
    /// `None` unless `include_dictation: true` was requested by the caller.
    cleaned_text: Option<String>,
}

// ---------------------------------------------------------------------------
// Payload builder (pure, no I/O — testable seam for the privacy gate)
// ---------------------------------------------------------------------------

/// Builds a [`FeedbackPayload`] from `include_dictation`, a metrics snapshot,
/// and the scalar form fields.
///
/// This is a pure data-transformation function: no network I/O, no lock
/// acquisition, no async. The `include_dictation` gate lives here so it can
/// be exercised in unit tests without spawning an HTTP client.
///
/// `version` and `os` are computed by the caller (usually from
/// `env!("CARGO_PKG_VERSION")` and `std::env::consts::OS`) so that this
/// function has no implicit dependencies.
#[allow(clippy::too_many_arguments)]
fn build_feedback_payload(
    include_dictation: bool,
    metrics: &FeedbackMetrics,
    category: String,
    message: String,
    email: Option<String>,
    context_area: String,
    area: Option<String>,
    version: String,
    os: String,
    license_status: String,
    platform: String,
) -> FeedbackPayload {
    FeedbackPayload {
        category,
        message,
        email,
        context_area,
        area,
        version,
        os,
        license_status,
        platform,
        // Metrics
        stt_latency_ms: metrics.last_stt_latency_ms,
        llm_latency_ms: metrics.last_llm_latency_ms,
        total_latency_ms: metrics.last_total_latency_ms,
        last_target_app: metrics.last_target_app.clone(),
        last_dictation_at: metrics.last_dictation_at.clone(),
        stt_error_count: metrics.stt_error_count,
        llm_error_count: metrics.llm_error_count,
        paste_error_count: metrics.paste_error_count,
        // Opt-in dictation sample — THE GATE
        raw_text: if include_dictation { metrics.last_raw_text.clone() } else { None },
        cleaned_text: if include_dictation { metrics.last_cleaned_text.clone() } else { None },
    }
}

// ---------------------------------------------------------------------------
// Network seam (injectable for tests)
// ---------------------------------------------------------------------------

/// POSTs a [`FeedbackPayload`] to the given URL using the provided client.
///
/// Extracted from `send_feedback` so that integration tests can supply a
/// `reqwest::Client` pointed at a `wiremock::MockServer` without touching a
/// live network. The `#[tauri::command]` wrapper remains a thin delegator.
///
/// Returns `Err` when the request fails (network error, timeout) or when the
/// server responds with a non-2xx status.
async fn post_feedback_to_url(
    client: &reqwest::Client,
    url: &str,
    payload: &FeedbackPayload,
) -> Result<(), String> {
    let resp = client
        .post(url)
        .json(payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to send feedback: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Webhook returned status {}", resp.status()));
    }

    Ok(())
}

/// Core of `send_feedback`: builds the payload and posts it to the webhook.
///
/// Extracted so that wire-path integration tests can drive the entire
/// gate→build→POST chain end-to-end without a Tauri `State`. The
/// `#[tauri::command]` reads `State`, builds the client, and delegates here.
///
/// A hardcoded `include_dictation` value or an arg-swap inside THIS function
/// will be caught by the wire tests, which pass the flag into this seam.
#[allow(clippy::too_many_arguments)]
async fn send_feedback_inner(
    client: &reqwest::Client,
    webhook_url: &str,
    include_dictation: bool,
    metrics: &FeedbackMetrics,
    category: String,
    message: String,
    email: Option<String>,
    context_area: String,
    area: Option<String>,
    license_status: String,
    platform: String,
) -> Result<(), String> {
    let payload = build_feedback_payload(
        include_dictation,
        metrics,
        category,
        message,
        email,
        context_area,
        area,
        env!("CARGO_PKG_VERSION").to_string(),
        std::env::consts::OS.to_string(),
        license_status,
        platform,
    );
    post_feedback_to_url(client, webhook_url, &payload).await
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Reads [`FeedbackMetrics`] from the JSON file written by Kotlin on Android.
///
/// Returns `Default` when the file does not exist or cannot be parsed —
/// the feedback dialog will simply show no metrics, which is fine for the
/// first use before any dictation has completed.
#[cfg(mobile)]
fn read_metrics_file(data_dir: &std::path::Path) -> FeedbackMetrics {
    let path = data_dir.join(METRICS_FILENAME);
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => FeedbackMetrics::default(),
    }
}

/// Writes [`FeedbackMetrics`] back to the JSON file (used to reset error
/// counts after a successful feedback submission on Android).
#[cfg(mobile)]
fn write_metrics_file(data_dir: &std::path::Path, metrics: &FeedbackMetrics) {
    let path = data_dir.join(METRICS_FILENAME);
    if let Ok(json) = serde_json::to_string(metrics) {
        let _ = std::fs::write(&path, json);
    }
}

/// Returns the current [`FeedbackMetrics`] snapshot.
///
/// Called by the frontend when the feedback dialog opens so it can display
/// the latest latency and error data to the user before they submit.
///
/// On desktop, reads from the in-memory `AppState` (populated by `pipeline.rs`).
/// On mobile, reads from `feedback_metrics.json` (written by Kotlin).
#[tauri::command]
pub async fn get_feedback_metrics(
    state: State<'_, AppState>,
) -> Result<FeedbackMetrics, String> {
    #[cfg(desktop)]
    {
        let metrics = crate::lock!(state.feedback_metrics)?;
        Ok(metrics.clone())
    }
    #[cfg(mobile)]
    {
        Ok(read_metrics_file(&state.app_data_dir))
    }
}

/// Sends user feedback to the configured webhook URL.
///
/// Returns `Err` when:
/// - No webhook URL is configured (`feedbackWebhookUrl` is empty in config).
/// - The HTTP request fails (network error, timeout).
/// - The webhook responds with a non-2xx status code.
///
/// On success, the cumulative error counters inside [`FeedbackMetrics`] are
/// reset to zero so the next feedback submission starts from a clean baseline.
///
/// # Arguments
/// * `category`           – `"problem"`, `"idea"`, `"question"`, or `"praise"`.
/// * `message`            – User's feedback text.
/// * `email`              – Optional reply address.
/// * `context_area`       – UI location that triggered the feedback dialog.
/// * `area`               – Optional user-selected feature area.
/// * `include_dictation`  – When `true`, attaches the last raw + cleaned text
///                          to the payload. The user must opt in explicitly.
#[tauri::command]
pub async fn send_feedback(
    category: String,
    message: String,
    email: Option<String>,
    context_area: String,
    area: Option<String>,
    include_dictation: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Read webhook URL from config (short lock, no await inside).
    let webhook_url = {
        let cfg = crate::lock!(state.config)?;
        cfg.feedback_webhook_url.clone()
    };

    if webhook_url.is_empty() {
        return Err(
            "Feedback is not configured. \
             Set a webhook URL in config.json (feedbackWebhookUrl)."
                .to_string(),
        );
    }

    // Read license status (short lock, no await inside).
    let license_status = {
        let status = crate::lock!(state.license_status)?;
        status_to_string(&status)
    };

    // Snapshot metrics.
    // Desktop: from in-memory AppState. Mobile: from JSON file written by Kotlin.
    let metrics = {
        #[cfg(desktop)]
        {
            let m = crate::lock!(state.feedback_metrics)?;
            m.clone()
        }
        #[cfg(mobile)]
        {
            read_metrics_file(&state.app_data_dir)
        }
    };

    let platform = if cfg!(mobile) { "mobile" } else { "desktop" };

    let client = reqwest::Client::new();
    send_feedback_inner(
        &client,
        &webhook_url,
        include_dictation,
        &metrics,
        category,
        message,
        email,
        context_area,
        area,
        license_status,
        platform.to_string(),
    )
    .await?;

    // Reset cumulative error counters on successful submission.
    // Desktop: in-memory. Mobile: rewrite the JSON file.
    {
        #[cfg(desktop)]
        {
            let mut m = crate::lock!(state.feedback_metrics)?;
            m.stt_error_count = 0;
            m.llm_error_count = 0;
            m.paste_error_count = 0;
        }
        #[cfg(mobile)]
        {
            let mut m = read_metrics_file(&state.app_data_dir);
            m.stt_error_count = 0;
            m.llm_error_count = 0;
            m.paste_error_count = 0;
            write_metrics_file(&state.app_data_dir, &m);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::{build_feedback_payload, FeedbackMetrics, FeedbackPayload};

    // --- FeedbackMetrics tests ---

    /// Default FeedbackMetrics has all Option fields as None and counters at 0.
    #[test]
    fn test_feedback_metrics_default() {
        let m = FeedbackMetrics::default();
        assert!(m.last_stt_latency_ms.is_none());
        assert!(m.last_llm_latency_ms.is_none());
        assert!(m.last_total_latency_ms.is_none());
        assert!(m.last_target_app.is_none());
        assert!(m.last_dictation_at.is_none());
        assert!(m.last_raw_text.is_none());
        assert!(m.last_cleaned_text.is_none());
        assert_eq!(m.stt_error_count, 0);
        assert_eq!(m.llm_error_count, 0);
        assert_eq!(m.paste_error_count, 0);
    }

    /// FeedbackMetrics serializes to camelCase JSON.
    #[test]
    fn test_feedback_metrics_serialization() {
        let m = FeedbackMetrics {
            last_stt_latency_ms: Some(450),
            last_llm_latency_ms: Some(1200),
            last_total_latency_ms: Some(1700),
            last_target_app: Some("Notepad".to_string()),
            last_dictation_at: Some("2026-03-30T10:00:00Z".to_string()),
            last_raw_text: Some("hello world".to_string()),
            last_cleaned_text: Some("Hello, world.".to_string()),
            stt_error_count: 2,
            llm_error_count: 1,
            paste_error_count: 0,
        };

        let json = serde_json::to_string(&m).expect("serialization must not fail");

        assert!(json.contains("\"lastSttLatencyMs\":450"));
        assert!(json.contains("\"lastLlmLatencyMs\":1200"));
        assert!(json.contains("\"lastTotalLatencyMs\":1700"));
        assert!(json.contains("\"lastTargetApp\":\"Notepad\""));
        assert!(json.contains("\"lastDictationAt\":\"2026-03-30T10:00:00Z\""));
        assert!(json.contains("\"lastRawText\":\"hello world\""));
        assert!(json.contains("\"lastCleanedText\":\"Hello, world.\""));
        assert!(json.contains("\"sttErrorCount\":2"));
        assert!(json.contains("\"llmErrorCount\":1"));
        assert!(json.contains("\"pasteErrorCount\":0"));
    }

    /// FeedbackMetrics with all None fields serializes Option fields as JSON null.
    #[test]
    fn test_feedback_metrics_none_fields() {
        let m = FeedbackMetrics::default();
        let json = serde_json::to_string(&m).expect("serialization must not fail");
        assert!(json.contains("\"lastSttLatencyMs\":null"));
        assert!(json.contains("\"lastRawText\":null"));
    }

    // --- FeedbackPayload tests ---

    /// FeedbackPayload serializes to camelCase JSON with all expected fields.
    #[test]
    fn test_feedback_payload_serialization() {
        let payload = FeedbackPayload {
            category: "problem".to_string(),
            message: "Something broke.".to_string(),
            email: Some("user@example.com".to_string()),
            context_area: "settings".to_string(),
            area: Some("Audio".to_string()),
            version: "0.5.0".to_string(),
            os: "windows".to_string(),
            license_status: "licensed".to_string(),
            platform: "desktop".to_string(),
            stt_latency_ms: Some(300),
            llm_latency_ms: Some(900),
            total_latency_ms: Some(1300),
            last_target_app: Some("VS Code".to_string()),
            last_dictation_at: Some("2026-03-30T10:00:00Z".to_string()),
            stt_error_count: 0,
            llm_error_count: 0,
            paste_error_count: 0,
            raw_text: Some("raw".to_string()),
            cleaned_text: Some("cleaned".to_string()),
        };

        let json = serde_json::to_string(&payload).expect("serialization must not fail");

        assert!(json.contains("\"category\":\"problem\""));
        assert!(json.contains("\"message\":\"Something broke.\""));
        assert!(json.contains("\"email\":\"user@example.com\""));
        assert!(json.contains("\"contextArea\":\"settings\""));
        assert!(json.contains("\"area\":\"Audio\""));
        assert!(json.contains("\"version\":\"0.5.0\""));
        assert!(json.contains("\"os\":\"windows\""));
        assert!(json.contains("\"licenseStatus\":\"licensed\""));
        assert!(json.contains("\"platform\":\"desktop\""));
        assert!(json.contains("\"sttLatencyMs\":300"));
        assert!(json.contains("\"llmLatencyMs\":900"));
        assert!(json.contains("\"totalLatencyMs\":1300"));
        assert!(json.contains("\"lastTargetApp\":\"VS Code\""));
        assert!(json.contains("\"sttErrorCount\":0"));
        assert!(json.contains("\"rawText\":\"raw\""));
        assert!(json.contains("\"cleanedText\":\"cleaned\""));
    }

    /// When email is None it serializes as JSON null (not missing).
    #[test]
    fn test_feedback_payload_email_none() {
        let payload = FeedbackPayload {
            category: "idea".to_string(),
            message: "Would be nice to have X.".to_string(),
            email: None,
            context_area: "home".to_string(),
            area: None,
            version: "0.5.0".to_string(),
            os: "linux".to_string(),
            license_status: "trial".to_string(),
            platform: "desktop".to_string(),
            stt_latency_ms: None,
            llm_latency_ms: None,
            total_latency_ms: None,
            last_target_app: None,
            last_dictation_at: None,
            stt_error_count: 0,
            llm_error_count: 0,
            paste_error_count: 0,
            raw_text: None,
            cleaned_text: None,
        };

        let json = serde_json::to_string(&payload).expect("serialization must not fail");
        assert!(json.contains("\"email\":null"));
        assert!(json.contains("\"rawText\":null"));
        assert!(json.contains("\"cleanedText\":null"));
    }

    // ---------------------------------------------------------------------------
    // Privacy-gate specs (TEST-02) — these call the REAL build_feedback_payload
    // so an inverted gate would cause a test failure, not a silent leak.
    // ---------------------------------------------------------------------------

    /// Helper: a FeedbackMetrics that contains real text in both text fields.
    fn metrics_with_text() -> FeedbackMetrics {
        FeedbackMetrics {
            last_raw_text: Some("hello world".to_string()),
            last_cleaned_text: Some("Hello, world.".to_string()),
            last_stt_latency_ms: Some(400),
            ..FeedbackMetrics::default()
        }
    }

    /// Helper: call build_feedback_payload with the given flag and the
    /// provided metrics — scalar args filled with test-appropriate defaults.
    fn call_gate(include_dictation: bool, metrics: &FeedbackMetrics) -> FeedbackPayload {
        build_feedback_payload(
            include_dictation,
            metrics,
            "question".to_string(),
            "How does X work?".to_string(),
            None,
            "home".to_string(),
            None,
            "0.5.0".to_string(),
            "linux".to_string(),
            "trial".to_string(),
            "desktop".to_string(),
        )
    }

    /// GATE OFF: when include_dictation is false, both text fields must be None
    /// even when metrics contains real text.
    ///
    /// Inversion guard: if the condition were `if !include_dictation { ... }`
    /// instead of `if include_dictation { ... }`, raw_text would be
    /// Some("hello world") and both asserts below would FAIL — the test
    /// actively detects the gate inversion.
    #[test]
    fn spec_privacy_gate_excludes_text_when_not_requested() {
        let metrics = metrics_with_text();
        let payload = call_gate(false, &metrics);
        assert!(
            payload.raw_text.is_none(),
            "raw_text must be None when include_dictation=false (gate not inverted)"
        );
        assert!(
            payload.cleaned_text.is_none(),
            "cleaned_text must be None when include_dictation=false (gate not inverted)"
        );
        // Sanity-check: non-text metrics still flow through the gate
        assert_eq!(payload.stt_latency_ms, Some(400));
    }

    /// GATE ON: when include_dictation is true, both text fields carry the
    /// exact values from the metrics struct.
    #[test]
    fn spec_privacy_gate_includes_text_when_requested() {
        let metrics = metrics_with_text();
        let payload = call_gate(true, &metrics);
        assert_eq!(
            payload.raw_text,
            Some("hello world".to_string()),
            "raw_text must equal metrics.last_raw_text when include_dictation=true"
        );
        assert_eq!(
            payload.cleaned_text,
            Some("Hello, world.".to_string()),
            "cleaned_text must equal metrics.last_cleaned_text when include_dictation=true"
        );
    }

    /// GATE ON with absent metrics: when include_dictation is true but metrics
    /// has no text, the payload fields stay None — no panic on missing data.
    #[test]
    fn spec_privacy_gate_excludes_text_when_metrics_has_none() {
        let metrics = FeedbackMetrics::default(); // last_raw_text / last_cleaned_text == None
        let payload = call_gate(true, &metrics);
        assert!(
            payload.raw_text.is_none(),
            "raw_text must be None when metrics.last_raw_text is None"
        );
        assert!(
            payload.cleaned_text.is_none(),
            "cleaned_text must be None when metrics.last_cleaned_text is None"
        );
    }

    // ---------------------------------------------------------------------------
    // Wire-path integration specs — drive send_feedback_inner end-to-end
    // against a wiremock server.
    //
    // CRITICAL: These tests pass include_dictation into send_feedback_inner
    // (the seam that also calls build_feedback_payload), NOT into
    // build_feedback_payload directly. A hardcoded flag or arg-swap INSIDE
    // send_feedback_inner (or in its call to build_feedback_payload) will
    // therefore cause the assertions to fail, closing Defer #1.
    // ---------------------------------------------------------------------------

    /// Wire spec: with include_dictation=false passed to send_feedback_inner,
    /// the POSTed JSON body must have rawText and cleanedText as JSON null,
    /// even when the metrics contain real text.
    ///
    /// Inversion guard: if send_feedback_inner were to hardcode `true` or
    /// swap the include_dictation arg, rawText/cleanedText would be non-null
    /// and both asserts below would FAIL.
    #[tokio::test]
    async fn spec_wire_gate_off_body_has_no_dictation_text() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/feedback"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let metrics = metrics_with_text();
        let client = reqwest::Client::new();
        let url = format!("{}/feedback", server.uri());

        // Pass the flag to the SEAM — not to build_feedback_payload directly.
        super::send_feedback_inner(
            &client,
            &url,
            false, // include_dictation = OFF
            &metrics,
            "problem".to_string(),
            "test message".to_string(),
            None,
            "home".to_string(),
            None,
            "trial".to_string(),
            "desktop".to_string(),
        )
        .await
        .expect("send_feedback_inner must succeed against mock server");

        // Capture what was actually POSTed to the wire
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "exactly one POST must have been made");
        let body: serde_json::Value =
            serde_json::from_slice(&requests[0].body).expect("body must be valid JSON");

        assert!(
            body["rawText"].is_null(),
            "rawText must be null on the wire when include_dictation=false, got: {}",
            body["rawText"]
        );
        assert!(
            body["cleanedText"].is_null(),
            "cleanedText must be null on the wire when include_dictation=false, got: {}",
            body["cleanedText"]
        );
    }

    /// Wire spec: with include_dictation=true passed to send_feedback_inner,
    /// the POSTed JSON body must carry the metrics' raw and cleaned text
    /// verbatim on the wire.
    #[tokio::test]
    async fn spec_wire_gate_on_body_carries_dictation_text() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/feedback"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let metrics = metrics_with_text();
        let client = reqwest::Client::new();
        let url = format!("{}/feedback", server.uri());

        // Pass the flag to the SEAM — not to build_feedback_payload directly.
        super::send_feedback_inner(
            &client,
            &url,
            true, // include_dictation = ON
            &metrics,
            "problem".to_string(),
            "test message".to_string(),
            None,
            "home".to_string(),
            None,
            "trial".to_string(),
            "desktop".to_string(),
        )
        .await
        .expect("send_feedback_inner must succeed against mock server");

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1, "exactly one POST must have been made");
        let body: serde_json::Value =
            serde_json::from_slice(&requests[0].body).expect("body must be valid JSON");

        assert_eq!(
            body["rawText"].as_str(),
            Some("hello world"),
            "rawText must carry metrics.last_raw_text on the wire when include_dictation=true"
        );
        assert_eq!(
            body["cleanedText"].as_str(),
            Some("Hello, world."),
            "cleanedText must carry metrics.last_cleaned_text on the wire when include_dictation=true"
        );
    }
}
