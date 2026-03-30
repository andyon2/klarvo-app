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

    let payload = FeedbackPayload {
        category,
        message,
        email,
        context_area,
        area,
        version: env!("CARGO_PKG_VERSION").to_string(),
        os: std::env::consts::OS.to_string(),
        license_status,
        platform: platform.to_string(),
        // Metrics
        stt_latency_ms: metrics.last_stt_latency_ms,
        llm_latency_ms: metrics.last_llm_latency_ms,
        total_latency_ms: metrics.last_total_latency_ms,
        last_target_app: metrics.last_target_app.clone(),
        last_dictation_at: metrics.last_dictation_at.clone(),
        stt_error_count: metrics.stt_error_count,
        llm_error_count: metrics.llm_error_count,
        paste_error_count: metrics.paste_error_count,
        // Opt-in dictation sample
        raw_text: if include_dictation { metrics.last_raw_text.clone() } else { None },
        cleaned_text: if include_dictation { metrics.last_cleaned_text.clone() } else { None },
    };

    let client = reqwest::Client::new();
    let resp = client
        .post(&webhook_url)
        .json(&payload)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to send feedback: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("Webhook returned status {}", resp.status()));
    }

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
    use super::{FeedbackMetrics, FeedbackPayload};

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

    /// When include_dictation is false, raw/cleaned text stay None.
    /// (Behaviour verified via the payload construction in send_feedback --
    /// this test checks the serialization side stays consistent.)
    #[test]
    fn test_payload_no_dictation_sample_when_not_requested() {
        let payload = FeedbackPayload {
            category: "question".to_string(),
            message: "How does X work?".to_string(),
            email: None,
            context_area: "home".to_string(),
            area: None,
            version: "0.5.0".to_string(),
            os: "windows".to_string(),
            license_status: "trial".to_string(),
            platform: "desktop".to_string(),
            stt_latency_ms: Some(400),
            llm_latency_ms: Some(800),
            total_latency_ms: Some(1300),
            last_target_app: Some("Notepad".to_string()),
            last_dictation_at: Some("2026-03-30T10:00:00Z".to_string()),
            stt_error_count: 0,
            llm_error_count: 0,
            paste_error_count: 0,
            // Simulates include_dictation == false
            raw_text: None,
            cleaned_text: None,
        };

        let json = serde_json::to_string(&payload).expect("serialization must not fail");
        assert!(json.contains("\"rawText\":null"));
        assert!(json.contains("\"cleanedText\":null"));
        // Metrics should still be present
        assert!(json.contains("\"sttLatencyMs\":400"));
    }
}
