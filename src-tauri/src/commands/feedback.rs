//! Tauri command for in-app feedback submissions.
//!
//! The feedback feature is operator-controlled: it is only active when
//! `feedbackWebhookUrl` is set in `config.json`. End users cannot configure
//! this through the Settings UI.
//!
//! The command POSTs a structured JSON payload to the configured webhook URL.
//! On success it returns `Ok(())`. On failure it returns a human-readable
//! error string that the frontend can display.

use serde::Serialize;
use tauri::State;

use crate::license::status_to_string;
use crate::AppState;

// ---------------------------------------------------------------------------
// Payload
// ---------------------------------------------------------------------------

/// JSON body sent to the feedback webhook.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FeedbackPayload {
    /// Feedback category: `"problem"` or `"idea"`.
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
}

// ---------------------------------------------------------------------------
// Command
// ---------------------------------------------------------------------------

/// Sends user feedback to the configured webhook URL.
///
/// Returns `Err` when:
/// - No webhook URL is configured (`feedbackWebhookUrl` is empty in config).
/// - The HTTP request fails (network error, timeout).
/// - The webhook responds with a non-2xx status code.
///
/// # Arguments
/// * `category`     – `"problem"`, `"idea"`, `"question"`, or `"praise"`.
/// * `message`      – User's feedback text.
/// * `email`        – Optional reply address.
/// * `context_area` – UI location that triggered the feedback dialog.
/// * `area`         – Optional user-selected feature area (e.g. `"Audio"`).
#[tauri::command]
pub async fn send_feedback(
    category: String,
    message: String,
    email: Option<String>,
    context_area: String,
    area: Option<String>,
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

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::FeedbackPayload;

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
        };

        let json = serde_json::to_string(&payload).expect("serialization must not fail");
        assert!(json.contains("\"email\":null"));
    }
}
