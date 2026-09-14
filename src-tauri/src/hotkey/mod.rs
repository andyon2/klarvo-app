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
//! `klarvo://state-changed` with a [`PipelineEvent`] payload.

use serde::Serialize;

use crate::overlay_message::{DegradeCause, OverlayMessage};

// ---------------------------------------------------------------------------
// Event payload
// ---------------------------------------------------------------------------

/// Pipeline state values sent to the frontend via `klarvo://state-changed`.
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
    /// A non-fatal issue occurred; the pipeline still produced output.
    /// The frontend should surface this as a warning (e.g. yellow toast)
    /// rather than a hard error.  Text was still pasted.
    Warning,
}

/// Payload for the `klarvo://state-changed` Tauri event.
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
    /// Human-readable warning message (only present when `state == Warning`).
    /// Unlike `error`, the pipeline completed successfully — text was pasted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    /// True when the text was written to the clipboard but Ctrl+V was NOT sent
    /// (focus verification failed). The frontend should show a "Text in clipboard"
    /// indicator so the user knows to paste manually.
    ///
    /// Only present (and `true`) when `state == Done` and focus-restore failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "clipboardOnly")]
    pub clipboard_only: Option<bool>,
    /// This event's message in the shape the native overlay card lays out
    /// (Story 7-10, AC8). **Never serialized** — the frontend keeps reading the
    /// flat `warning` / `error` strings, so no payload field and no event name
    /// changed (surface-smoke trap #5). This is an in-process carrier for
    /// `lib::emit_pipeline_state` → `native_preview`.
    #[serde(skip_serializing)]
    pub message: Option<OverlayMessage>,
}

impl PipelineEvent {
    pub fn idle() -> Self {
        PipelineEvent {
            state: PipelineState::Idle,
            text: None,
            raw_text: None,
            error: None,
            warning: None,
            clipboard_only: None,
            message: None,
        }
    }

    pub fn recording() -> Self {
        PipelineEvent {
            state: PipelineState::Recording,
            text: None,
            raw_text: None,
            error: None,
            warning: None,
            clipboard_only: None,
            message: None,
        }
    }

    pub fn transcribing() -> Self {
        PipelineEvent {
            state: PipelineState::Transcribing,
            text: None,
            raw_text: None,
            error: None,
            warning: None,
            clipboard_only: None,
            message: None,
        }
    }

    pub fn cleaning() -> Self {
        PipelineEvent {
            state: PipelineState::Cleaning,
            text: None,
            raw_text: None,
            error: None,
            warning: None,
            clipboard_only: None,
            message: None,
        }
    }

    /// Paste succeeded -- focus was verified, Ctrl+V was sent.
    pub fn done(text: String, raw_text: String) -> Self {
        PipelineEvent {
            state: PipelineState::Done,
            text: Some(text),
            raw_text: Some(raw_text),
            error: None,
            warning: None,
            clipboard_only: None,
            message: None,
        }
    }

    /// Paste not attempted -- the text is clipboard-only.
    ///
    /// Two causes reach this event:
    /// - focus verification failed (the paste target vanished), `warning` is `None`;
    /// - LLM cleanup failed and the pipeline deliberately withheld the paste
    ///   (Story 7-10 AC1), `warning` carries the degrade message.
    ///
    /// Story 7-10 (Q1) makes this the *only* terminal event on the degrade path:
    /// the pipeline no longer emits a separate `Warning` ahead of it, so the
    /// cause text cannot be cut short by the follow-up terminal event (7-9
    /// GATE-4 finding 3a). Every consumer must surface the cause when present.
    ///
    /// AC8 splits the two consumers off **one** [`DegradeCause`] instead of
    /// handing each a string: `warning` keeps the unchanged one-line wording for
    /// the main window (D1), `message` carries the laid-out card for the native
    /// preview. The flag that tells the two clipboard-only routes apart is
    /// `message.is_some()` — the focus-failure route passes `None` and looks
    /// exactly as it did before.
    pub fn done_with_clipboard_only(
        text: String,
        raw_text: String,
        cause: Option<DegradeCause>,
    ) -> Self {
        PipelineEvent {
            state: PipelineState::Done,
            text: Some(text),
            raw_text: Some(raw_text),
            error: None,
            warning: cause.as_ref().map(|c| c.status_line()),
            clipboard_only: Some(true),
            message: cause.as_ref().map(|c| c.card()),
        }
    }

    pub fn error(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        PipelineEvent {
            state: PipelineState::Error,
            text: None,
            raw_text: None,
            error: Some(msg.clone()),
            warning: None,
            clipboard_only: None,
            message: Some(OverlayMessage::error(msg)),
        }
    }

    /// Non-fatal warning: the pipeline completed and text was pasted, but
    /// something went wrong along the way (e.g. LLM cleanup failed and raw
    /// text was used as fallback).  The frontend should surface this as a
    /// yellow / amber indicator rather than a hard error.
    ///
    /// Note: this event is emitted *before* the done event so the frontend
    /// can briefly show the warning before transitioning to done state.
    ///
    /// Two producers remain since Story 7-10 (Q1): the STT fallback ladder
    /// (mid-run) and the boot-time config warnings in `lib::run`. Both now also
    /// carry an amber card (AC8) — the pill's `Warning` label is static.
    pub fn warn(msg: impl Into<String>) -> Self {
        let msg = msg.into();
        PipelineEvent {
            state: PipelineState::Warning,
            text: None,
            raw_text: None,
            error: None,
            warning: Some(msg.clone()),
            clipboard_only: None,
            message: Some(OverlayMessage::warning(msg)),
        }
    }

    /// Several warnings as ONE event (Story 7-10, AC8 review P8).
    ///
    /// `lib::run` surfaces the boot-time config warnings one per entry. Each was
    /// its own `warn` event, and since AC8 each one opened a card that replaced
    /// the previous card instantly — with two corrupt files only the second was
    /// ever readable. One event, one card, one line per warning.
    ///
    /// Returns `None` for an empty list, so the boot path has nothing to emit
    /// when nothing went wrong.
    pub fn warn_all(texts: Vec<String>) -> Option<Self> {
        let message = OverlayMessage::warnings(texts)?;
        Some(PipelineEvent {
            state: PipelineState::Warning,
            text: None,
            raw_text: None,
            error: None,
            warning: Some(message.cause.text()),
            clipboard_only: None,
            message: Some(message),
        })
    }
}

/// Event name emitted on the Tauri event bus.
pub const EVENT_STATE_CHANGED: &str = "klarvo://state-changed";

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
        // Normal done does NOT set clipboardOnly.
        assert!(!json.contains("clipboardOnly"));
    }

    /// PipelineEvent::done_with_clipboard_only sets the clipboardOnly flag.
    #[test]
    fn test_pipeline_event_done_clipboard_only() {
        let event = PipelineEvent::done_with_clipboard_only(
            "Hello world".to_string(),
            "uh hello world".to_string(),
            None,
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"state\":\"done\""));
        assert!(json.contains("\"text\":\"Hello world\""));
        assert!(json.contains("\"clipboardOnly\":true"));
        assert!(!json.contains("\"error\""));
        // No degrade cause -> no warning field (focus-failure route).
        assert!(!json.contains("\"warning\""));
    }

    /// Story 7-10 (Q1/AC3): the degrade route carries its cause text on the
    /// single terminal event, so the main window can still name the model ID
    /// after the run has ended.
    ///
    /// AC8 adds the second half: the same cause also produces the card the
    /// native preview renders, and the card is **not** serialized — the
    /// frontend payload is unchanged (surface-smoke trap #5: no new field, no
    /// new event name).
    #[test]
    fn test_pipeline_event_done_clipboard_only_carries_warning() {
        let event = PipelineEvent::done_with_clipboard_only(
            "raw text".to_string(),
            "raw text".to_string(),
            Some(DegradeCause::ModelNotFound { model: "deepseek-typo".to_string() }),
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"state\":\"done\""));
        assert!(json.contains("\"clipboardOnly\":true"));
        assert!(
            json.contains("Model 'deepseek-typo' not found"),
            "the degrade cause must ride on the terminal event: {json}"
        );
        assert!(
            !json.contains("CLEANUP FAILED"),
            "the card is in-process only — it must never reach the frontend payload: {json}"
        );

        let card = event.message.expect("AC8: the degrade event carries a card");
        assert_eq!(card.header, "CLEANUP FAILED");
        assert_eq!(card.cause.chip.as_deref(), Some("deepseek-typo"));
        assert!(card.next.is_some(), "the card tells the user where the text went");
    }

    /// AC8's gate: a clipboard-only run **without** a degrade cause (the
    /// focus-failure route) carries no card, so the pill keeps its static
    /// "In Clipboard" label and no card is shown. This is the discriminator the
    /// overlay boundary reads — `message.is_some()` — so it is pinned here.
    #[test]
    fn spec_focus_failure_clipboard_only_carries_no_card() {
        let event = PipelineEvent::done_with_clipboard_only(
            "Hello".to_string(),
            "Hello".to_string(),
            None,
        );
        assert_eq!(event.clipboard_only, Some(true));
        assert_eq!(event.warning, None);
        assert!(
            event.message.is_none(),
            "no cleanup failure, no card — otherwise the pill would say 'Cleanup failed' \
             for a vanished paste target"
        );
    }

    /// AC8: every other pipeline message rides the same card.
    #[test]
    fn spec_warning_and_error_events_carry_a_card() {
        let warn = PipelineEvent::warn("⚠ Groq am Limit → lokale Transkription");
        let card = warn.message.expect("a Warning must carry its card");
        assert_eq!(card.header, "WARNING");
        assert_eq!(card.cause.text(), "⚠ Groq am Limit → lokale Transkription");
        assert_eq!(warn.warning.as_deref(), Some("⚠ Groq am Limit → lokale Transkription"));

        let err = PipelineEvent::error("STT failed: timeout");
        let card = err.message.expect("an Error must carry its card");
        assert_eq!(card.header, "ERROR");
        assert_eq!(card.tone, crate::overlay_message::MessageTone::Error);
        assert_eq!(err.error.as_deref(), Some("STT failed: timeout"));
    }

    /// AC8 review, P8: `lib::run` used to emit one event per boot-time config
    /// warning, and each card replaced the previous one instantly — only the
    /// last was ever readable. They now travel as ONE event carrying ONE card.
    ///
    /// PINS the producer the real boot path calls; it does NOT pin that the card
    /// renders two lines (`native_preview.rs` is Windows-gated and uncompiled
    /// here — `wrap_text_lines` breaking on `\n` is read, not run).
    #[test]
    fn spec_boot_config_warnings_travel_as_one_event() {
        assert!(PipelineEvent::warn_all(Vec::new()).is_none(), "no warnings, no event");

        let event = PipelineEvent::warn_all(vec![
            "config.json was corrupt".to_string(),
            "dictionary.json was corrupt".to_string(),
        ])
        .expect("two warnings are one event");
        assert_eq!(event.state, PipelineState::Warning);
        let card = event.message.expect("the merged warning carries its card");
        assert_eq!(card.header, "WARNING");
        assert_eq!(
            card.cause.text(),
            "config.json was corrupt\ndictionary.json was corrupt"
        );
        assert_eq!(event.warning.as_deref(), Some(card.cause.text().as_str()));
    }

    /// Progress states are not messages — they carry no card.
    ///
    /// What the overlay does with that is the opposite of what this docstring
    /// claimed before the AC8 re-review: a message-less event does **not** take
    /// a live card down (`native_preview`'s `WM_PREVIEW_SET_MESSAGE` `None` arm
    /// ignores it inside the 4 s hold, P1). A card is dismissed by the recording
    /// state, by its own timer, or by a click on it.
    #[test]
    fn spec_progress_events_carry_no_card() {
        for event in [
            PipelineEvent::idle(),
            PipelineEvent::recording(),
            PipelineEvent::transcribing(),
            PipelineEvent::cleaning(),
            PipelineEvent::done("a".to_string(), "a".to_string()),
        ] {
            assert!(
                event.message.is_none(),
                "a progress/success state must not open a message card: {:?}",
                event.state
            );
        }
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
        assert_eq!(EVENT_STATE_CHANGED, "klarvo://state-changed");
    }
}
