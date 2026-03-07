//! Hotkey and dictation pipeline module.
//!
//! Handles global shortcut registration and the end-to-end dictation pipeline:
//!
//! ```text
//! Hotkey pressed
//!   -> if idle:   start_recording()      emit state="recording"
//!   -> if active: stop_recording()       emit state="transcribing"
//!                 transcribe_audio()     emit state="cleaning"
//!                 cleanup_text()
//!                 paste to focused field  emit state="done"
//! ```
//!
//! All state changes are communicated to the frontend via a single Tauri event
//! `dikta://state-changed` with a [`PipelineEvent`] payload.

use serde::Serialize;

// ---------------------------------------------------------------------------
// Event payload
// ---------------------------------------------------------------------------

/// Pipeline state values sent to the frontend via `dikta://state-changed`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineState {
    /// No recording active, ready for next dictation.
    Idle,
    /// Microphone is open and capturing audio.
    Recording,
    /// Audio captured; sending to STT provider.
    Transcribing,
    /// Raw text received; sending to LLM for cleanup.
    Cleaning,
    /// Cleanup complete; text pasted into focused field.
    Done,
    /// An error occurred at some stage of the pipeline.
    Error,
}

/// Payload for the `dikta://state-changed` Tauri event.
#[derive(Debug, Clone, Serialize)]
pub struct PipelineEvent {
    /// Current pipeline state.
    pub state: PipelineState,
    /// Final cleaned text (only present when `state == Done`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Raw transcript before LLM cleanup (only present when `state == Done`).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "rawText")]
    pub raw_text: Option<String>,
    /// Human-readable error message (only present when `state == Error`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl PipelineEvent {
    pub fn idle() -> Self {
        PipelineEvent {
            state: PipelineState::Idle,
            text: None,
            raw_text: None,
            error: None,
        }
    }

    pub fn recording() -> Self {
        PipelineEvent {
            state: PipelineState::Recording,
            text: None,
            raw_text: None,
            error: None,
        }
    }

    pub fn transcribing() -> Self {
        PipelineEvent {
            state: PipelineState::Transcribing,
            text: None,
            raw_text: None,
            error: None,
        }
    }

    pub fn cleaning() -> Self {
        PipelineEvent {
            state: PipelineState::Cleaning,
            text: None,
            raw_text: None,
            error: None,
        }
    }

    pub fn done(text: String, raw_text: String) -> Self {
        PipelineEvent {
            state: PipelineState::Done,
            text: Some(text),
            raw_text: Some(raw_text),
            error: None,
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        PipelineEvent {
            state: PipelineState::Error,
            text: None,
            raw_text: None,
            error: Some(msg.into()),
        }
    }
}

/// Event name emitted on the Tauri event bus.
pub const EVENT_STATE_CHANGED: &str = "dikta://state-changed";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// PipelineEvent::recording serializes correctly.
    #[test]
    fn test_pipeline_event_recording_serialization() {
        let event = PipelineEvent::recording();
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"state\":\"recording\""));
        assert!(!json.contains("\"text\""));
        assert!(!json.contains("\"error\""));
    }

    /// PipelineEvent::done includes text and rawText fields.
    #[test]
    fn test_pipeline_event_done_includes_text() {
        let event = PipelineEvent::done("Hello world".to_string(), "uh hello world".to_string());
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"state\":\"done\""));
        assert!(json.contains("\"text\":\"Hello world\""));
        assert!(json.contains("\"rawText\":\"uh hello world\""));
        assert!(!json.contains("\"error\""));
    }

    /// PipelineEvent::error includes error field, no text.
    #[test]
    fn test_pipeline_event_error_has_message() {
        let event = PipelineEvent::error("STT failed: timeout");
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"state\":\"error\""));
        assert!(json.contains("\"error\":\"STT failed: timeout\""));
        assert!(!json.contains("\"text\""));
    }

    /// PipelineEvent::transcribing has no text or error.
    #[test]
    fn test_pipeline_event_transcribing_no_extras() {
        let event = PipelineEvent::transcribing();
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"state\":\"transcribing\""));
        assert!(!json.contains("\"text\""));
        assert!(!json.contains("\"error\""));
    }

    /// PipelineEvent::cleaning has no text or error.
    #[test]
    fn test_pipeline_event_cleaning_no_extras() {
        let event = PipelineEvent::cleaning();
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"state\":\"cleaning\""));
    }

    /// PipelineState serializes to lowercase strings.
    #[test]
    fn test_pipeline_state_lowercase_serialization() {
        assert_eq!(
            serde_json::to_string(&PipelineState::Recording).unwrap(),
            "\"recording\""
        );
        assert_eq!(
            serde_json::to_string(&PipelineState::Transcribing).unwrap(),
            "\"transcribing\""
        );
        assert_eq!(
            serde_json::to_string(&PipelineState::Cleaning).unwrap(),
            "\"cleaning\""
        );
        assert_eq!(
            serde_json::to_string(&PipelineState::Done).unwrap(),
            "\"done\""
        );
        assert_eq!(
            serde_json::to_string(&PipelineState::Error).unwrap(),
            "\"error\""
        );
    }

    /// EVENT_STATE_CHANGED has the correct event name.
    #[test]
    fn test_event_name_constant() {
        assert_eq!(EVENT_STATE_CHANGED, "dikta://state-changed");
    }
}
