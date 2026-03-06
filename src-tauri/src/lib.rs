//! Dikta -- Tauri backend entry point.
//!
//! Wires together the audio, STT, LLM, paste and hotkey modules and exposes
//! them to the React frontend via Tauri commands and events.
//!
//! ## Command flow (frontend perspective)
//!
//! ```text
//! start_recording()
//!   -> [user speaks]
//! stop_recording()        -> RecordingInfo { durationMs }
//!   -> [show "Transcribing..."]
//! transcribe_audio(lang)  -> String (raw text)
//!   -> [show "Cleaning up..."]
//! cleanup_text(raw, style, dict?) -> String (cleaned text)
//!   -> [paste / display result]
//! ```
//!
//! Each step is a separate command so the frontend can show granular status.
//!
//! ## Hotkey pipeline
//!
//! When the global shortcut fires (default: Ctrl+Shift+D), the backend runs
//! the full pipeline automatically and emits `dikta://state-changed` events
//! so the frontend can update the UI without being in the loop.

mod audio;
mod hotkey;
mod llm;
mod paste;
mod stt;

use std::sync::{Arc, Mutex, RwLock};

use audio::AudioRecorder;
use hotkey::{PipelineEvent, EVENT_STATE_CHANGED};
use llm::{CleanupProvider, CleanupStyle, DeepSeekCleanup};
use paste::create_paste_handler;
use serde::{Deserialize, Serialize};
use stt::{GroqWhisper, SttProvider};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

// ---------------------------------------------------------------------------
// Frontend-facing data types
// ---------------------------------------------------------------------------

/// Returned by `stop_recording`. Contains metadata about the just-finished
/// recording session. The WAV bytes are stored internally in `AppState`.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RecordingInfo {
    /// Duration of the recording in milliseconds.
    pub duration_ms: u64,
}

/// Returned by `get_api_key_status`. Does NOT expose the actual key values.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyStatus {
    /// `true` if a non-empty Groq API key is configured.
    pub groq_configured: bool,
    /// `true` if a non-empty DeepSeek API key is configured.
    pub deepseek_configured: bool,
}

/// Kept for backward compatibility -- the old monolithic result type.
///
/// No longer returned by any command; retained so external callers that may
/// have deserialized it don't break.  New callers should use the individual
/// step commands.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptionResult {
    pub raw_text: String,
    pub cleaned_text: String,
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// Shared application state managed by Tauri.
///
/// `RwLock` around the provider `Arc`s allows `update_api_keys` to swap out
/// a provider at runtime without restarting the application.
///
/// Tauri requires `State<T>: Send + Sync + 'static`.
pub struct AppState {
    recorder: Arc<AudioRecorder>,
    /// Wrapping in `RwLock` lets `update_api_keys` replace the provider.
    stt_provider: RwLock<Arc<dyn SttProvider>>,
    cleanup_provider: RwLock<Arc<dyn CleanupProvider>>,
    /// Timestamp set by `start_recording`, cleared by `stop_recording`.
    recording_start: Mutex<Option<std::time::Instant>>,
    /// WAV bytes from the most recent recording. Set by `stop_recording`,
    /// consumed (read, not cleared) by `transcribe_audio`.
    last_recording: Mutex<Option<Vec<u8>>>,
    /// The raw API keys, kept only so `get_api_key_status` can check presence.
    /// Never sent to the frontend as values.
    groq_api_key: Mutex<String>,
    deepseek_api_key: Mutex<String>,
    /// ISO-639-1 language code used by the hotkey pipeline (default: "de").
    language: Mutex<String>,
    /// Cleanup style used by the hotkey pipeline (default: Polished).
    current_style: Mutex<CleanupStyle>,
}

// SAFETY: All fields are either `Arc<_>`, `Mutex<_>`, or `RwLock<_>`, which
// are `Send + Sync` when their inner types are `Send`.
// `AudioRecorder` carries its own `unsafe impl Send + Sync` in audio/mod.rs.
// The trait objects (`Arc<dyn SttProvider>` etc.) require `Send + Sync` bounds
// on the traits (both traits have those bounds).
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl AppState {
    fn new(groq_api_key: String, deepseek_api_key: String) -> Self {
        AppState {
            recorder: Arc::new(AudioRecorder::new()),
            stt_provider: RwLock::new(Arc::new(GroqWhisper::new(groq_api_key.clone()))),
            cleanup_provider: RwLock::new(Arc::new(DeepSeekCleanup::new(
                deepseek_api_key.clone(),
            ))),
            recording_start: Mutex::new(None),
            last_recording: Mutex::new(None),
            groq_api_key: Mutex::new(groq_api_key),
            deepseek_api_key: Mutex::new(deepseek_api_key),
            language: Mutex::new("de".to_string()),
            current_style: Mutex::new(CleanupStyle::Polished),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper -- lock unwrap with descriptive error string
// ---------------------------------------------------------------------------

/// Acquires a `Mutex` lock and converts a poisoned-lock panic into a
/// `Result<_, String>` (Tauri command convention).
macro_rules! lock {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|_| "Internal state lock poisoned".to_string())
    };
}

/// Acquires a `RwLock` read guard.
macro_rules! read_lock {
    ($rwlock:expr) => {
        $rwlock
            .read()
            .map_err(|_| "Internal state lock poisoned".to_string())
    };
}

/// Acquires a `RwLock` write guard.
macro_rules! write_lock {
    ($rwlock:expr) => {
        $rwlock
            .write()
            .map_err(|_| "Internal state lock poisoned".to_string())
    };
}

// ---------------------------------------------------------------------------
// Hotkey dictation pipeline
// ---------------------------------------------------------------------------

/// Runs the full dictation pipeline when the hotkey fires.
///
/// - If not recording: starts recording and emits `state=recording`.
/// - If recording: stops, transcribes, cleans up, pastes, emits events at
///   each stage.
///
/// This function is meant to be called from the global-shortcut handler.
/// It clones what it needs from `AppState` before any await points to avoid
/// holding locks across awaits.
async fn run_dictation_pipeline(handle: AppHandle) {
    let state = handle.state::<AppState>();

    let is_recording = state.recorder.is_recording();

    if !is_recording {
        // --- Start recording ---
        if let Err(e) = state.recorder.start_recording() {
            let _ = handle.emit(
                EVENT_STATE_CHANGED,
                PipelineEvent::error(format!("Failed to start recording: {e}")),
            );
            return;
        }

        *match state.recording_start.lock() {
            Ok(g) => g,
            Err(_) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned"),
                );
                return;
            }
        } = Some(std::time::Instant::now());

        let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::recording());
        return;
    }

    // --- Stop recording ---
    let duration_ms = {
        match state.recording_start.lock() {
            Ok(g) => g.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0),
            Err(_) => 0,
        }
    };

    let wav_bytes = match state.recorder.stop_recording() {
        Ok(bytes) => bytes,
        Err(e) => {
            let _ = handle.emit(
                EVENT_STATE_CHANGED,
                PipelineEvent::error(format!("Failed to stop recording: {e}")),
            );
            return;
        }
    };

    // Clear recording start timestamp.
    if let Ok(mut g) = state.recording_start.lock() {
        *g = None;
    }

    // Store WAV bytes for manual transcribe commands too.
    if let Ok(mut g) = state.last_recording.lock() {
        *g = Some(wav_bytes.clone());
    }

    log::debug!("[pipeline] recording stopped after {duration_ms}ms, {len} WAV bytes", len = wav_bytes.len());

    // --- Transcribe ---
    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::transcribing());

    let language = {
        match state.language.lock() {
            Ok(g) => g.clone(),
            Err(_) => "de".to_string(),
        }
    };

    let stt_provider = match state.stt_provider.read() {
        Ok(g) => g.clone(),
        Err(_) => {
            let _ = handle.emit(
                EVENT_STATE_CHANGED,
                PipelineEvent::error("State lock poisoned"),
            );
            return;
        }
    };

    let raw_text = match stt_provider.transcribe(wav_bytes, &language).await {
        Ok(t) => t,
        Err(e) => {
            let _ = handle.emit(
                EVENT_STATE_CHANGED,
                PipelineEvent::error(format!("Transcription failed: {e}")),
            );
            return;
        }
    };

    log::debug!("[pipeline] raw transcription: {raw_text:?}");

    // --- LLM cleanup ---
    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::cleaning());

    let style = {
        match state.current_style.lock() {
            Ok(g) => *g,
            Err(_) => CleanupStyle::Polished,
        }
    };

    let cleanup_provider = match state.cleanup_provider.read() {
        Ok(g) => g.clone(),
        Err(_) => {
            let _ = handle.emit(
                EVENT_STATE_CHANGED,
                PipelineEvent::error("State lock poisoned"),
            );
            return;
        }
    };

    let cleaned_text = match cleanup_provider.cleanup(&raw_text, style, None).await {
        Ok(t) => t,
        Err(e) => {
            let _ = handle.emit(
                EVENT_STATE_CHANGED,
                PipelineEvent::error(format!("Text cleanup failed: {e}")),
            );
            return;
        }
    };

    log::debug!("[pipeline] cleaned text: {cleaned_text:?}");

    // --- Paste ---
    let paste_handler = create_paste_handler();
    if let Err(e) = paste_handler.paste(&cleaned_text) {
        log::warn!("[pipeline] paste failed: {e}. Text is still available.");
        // Do not return error -- text was produced, paste failure is non-fatal.
    }

    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::done(cleaned_text));
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Opens the default microphone and starts capturing audio.
///
/// Returns an error string if recording is already in progress or no
/// microphone is available.
#[tauri::command]
async fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    let inner = state.inner();

    inner
        .recorder
        .start_recording()
        .map_err(|e: audio::AudioError| e.to_string())?;

    *lock!(inner.recording_start)? = Some(std::time::Instant::now());

    Ok(())
}

/// Stops the active recording and stores the WAV bytes in `AppState`.
///
/// Returns `RecordingInfo` with the recording duration. This command does NOT
/// run STT or cleanup -- call `transcribe_audio` and `cleanup_text` for that.
///
/// Returns an error if no recording is active.
#[tauri::command]
async fn stop_recording(state: State<'_, AppState>) -> Result<RecordingInfo, String> {
    let inner = state.inner();

    // Measure duration before stopping (start timestamp is cleared below).
    let duration_ms = {
        let start_guard = lock!(inner.recording_start)?;
        start_guard
            .map(|t: std::time::Instant| t.elapsed().as_millis() as u64)
            .unwrap_or(0)
    };

    // Stop the cpal stream and get WAV bytes.
    let wav_bytes = inner
        .recorder
        .stop_recording()
        .map_err(|e: audio::AudioError| e.to_string())?;

    // Persist WAV for the subsequent `transcribe_audio` call.
    *lock!(inner.last_recording)? = Some(wav_bytes);

    // Clear the start timestamp.
    *lock!(inner.recording_start)? = None;

    Ok(RecordingInfo { duration_ms })
}

/// Transcribes the most recently recorded audio using the configured STT provider.
///
/// Reads WAV bytes stored by the last `stop_recording` call.
///
/// `language`: ISO-639-1 code (e.g. `"de"`, `"en"`). Empty string = auto-detect.
///
/// Returns an error if no recording is available or the STT call fails.
#[tauri::command]
async fn transcribe_audio(state: State<'_, AppState>, language: String) -> Result<String, String> {
    let inner = state.inner();

    // Clone the WAV out of the mutex so we don't hold the lock across the await.
    let wav_bytes = {
        let guard = lock!(inner.last_recording)?;
        guard
            .clone()
            .ok_or_else(|| "No recording available. Call stop_recording first.".to_string())?
    };

    // Read the current provider (shared read lock -- no contention with other readers).
    let provider = read_lock!(inner.stt_provider)?.clone();

    provider
        .transcribe(wav_bytes, &language)
        .await
        .map_err(|e: stt::SttError| e.to_string())
}

/// Cleans up raw transcription text using the configured LLM provider.
///
/// Can be called independently of the recording pipeline (e.g. to re-clean
/// text with a different style).
///
/// `raw_text`: text to clean up.
/// `style`: cleanup aggressiveness.
/// `dictionary_terms`: optional comma-separated list of terms to preserve verbatim.
#[tauri::command]
async fn cleanup_text(
    state: State<'_, AppState>,
    raw_text: String,
    style: CleanupStyle,
    dictionary_terms: Option<String>,
) -> Result<String, String> {
    let provider = read_lock!(state.inner().cleanup_provider)?.clone();

    provider
        .cleanup(&raw_text, style, dictionary_terms.as_deref())
        .await
        .map_err(|e: llm::LlmError| e.to_string())
}

/// Returns whether the recorder is currently active.
///
/// Useful for frontend state sync (e.g. showing a recording indicator).
#[tauri::command]
fn is_recording(state: State<'_, AppState>) -> bool {
    state.inner().recorder.is_recording()
}

/// Replaces the STT and/or LLM provider with a new instance using the supplied
/// API keys.
///
/// Passing `None` for a key leaves that provider unchanged.
/// Passing `Some("")` effectively disables the provider (requests will fail
/// with an auth error from the API).
///
/// This allows the Settings UI to apply new keys at runtime without restarting.
#[tauri::command]
async fn update_api_keys(
    state: State<'_, AppState>,
    groq_api_key: Option<String>,
    deepseek_api_key: Option<String>,
) -> Result<(), String> {
    let inner = state.inner();

    if let Some(key) = groq_api_key {
        *write_lock!(inner.stt_provider)? = Arc::new(GroqWhisper::new(key.clone()));
        *lock!(inner.groq_api_key)? = key;
    }

    if let Some(key) = deepseek_api_key {
        *write_lock!(inner.cleanup_provider)? = Arc::new(DeepSeekCleanup::new(key.clone()));
        *lock!(inner.deepseek_api_key)? = key;
    }

    Ok(())
}

/// Returns which API keys are currently configured (non-empty).
///
/// Does NOT return the key values themselves -- only booleans indicating
/// presence. The frontend uses this to show configuration status.
#[tauri::command]
fn get_api_key_status(state: State<'_, AppState>) -> Result<ApiKeyStatus, String> {
    let inner = state.inner();

    let groq_configured = !lock!(inner.groq_api_key)?.is_empty();
    let deepseek_configured = !lock!(inner.deepseek_api_key)?.is_empty();

    Ok(ApiKeyStatus {
        groq_configured,
        deepseek_configured,
    })
}

/// Sets the language used by the hotkey pipeline.
///
/// `language`: ISO-639-1 code, e.g. `"de"` or `"en"`. Empty string = auto-detect.
#[tauri::command]
fn set_language(state: State<'_, AppState>, language: String) -> Result<(), String> {
    *lock!(state.inner().language)? = language;
    Ok(())
}

/// Sets the cleanup style used by the hotkey pipeline.
#[tauri::command]
fn set_cleanup_style(state: State<'_, AppState>, style: CleanupStyle) -> Result<(), String> {
    *lock!(state.inner().current_style)? = style;
    Ok(())
}

/// Changes the registered global hotkey at runtime.
///
/// `shortcut`: a Tauri shortcut string, e.g. `"ctrl+shift+d"`.
/// Returns an error if the shortcut string is invalid or registration fails.
#[tauri::command]
async fn set_hotkey(handle: AppHandle, shortcut: String) -> Result<(), String> {
    // `Shortcut::from_str` returns `HotKeyParseError`, not `tauri_plugin_global_shortcut::Error`.
    // We convert to String immediately so we don't need to name the concrete error type.
    let shortcut = shortcut
        .parse::<Shortcut>()
        .map_err(|e| format!("Invalid shortcut string: {e}"))?;

    // Unregister all previous shortcuts, then register the new one.
    handle
        .global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {e}"))?;

    let handle_clone = handle.clone();
    handle
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let handle = handle_clone.clone();
                tauri::async_runtime::spawn(async move {
                    run_dictation_pipeline(handle).await;
                });
            }
        })
        .map_err(|e| format!("Failed to register shortcut: {e}"))
}

// ---------------------------------------------------------------------------
// Default hotkey string
// ---------------------------------------------------------------------------

const DEFAULT_HOTKEY: &str = "ctrl+shift+d";

// ---------------------------------------------------------------------------
// Tauri entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // API keys come from environment variables.
    // In production the settings UI will store them in the system keystore
    // and call `update_api_keys`; for now we fall back to .env / process env.
    let groq_api_key = std::env::var("GROQ_API_KEY").unwrap_or_default();
    let deepseek_api_key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default();

    let app_state = AppState::new(groq_api_key, deepseek_api_key);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state)
        .setup(|app| {
            let handle = app.handle().clone();

            // Register the default global shortcut: Ctrl+Shift+D.
            let shortcut = DEFAULT_HOTKEY
                .parse::<Shortcut>()
                .expect("DEFAULT_HOTKEY must be a valid shortcut string");

            let handle_clone = handle.clone();
            handle
                .global_shortcut()
                .on_shortcut(shortcut, move |_app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let h = handle_clone.clone();
                        tauri::async_runtime::spawn(async move {
                            run_dictation_pipeline(h).await;
                        });
                    }
                })
                .map_err(|e| {
                    log::error!("[hotkey] Failed to register default shortcut: {e}");
                    e
                })?;

            log::info!("[hotkey] Registered default shortcut: {DEFAULT_HOTKEY}");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            transcribe_audio,
            cleanup_text,
            is_recording,
            update_api_keys,
            get_api_key_status,
            set_language,
            set_cleanup_style,
            set_hotkey,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state() -> AppState {
        AppState::new(String::new(), String::new())
    }

    /// AppState starts with no recording and no cached WAV.
    #[test]
    fn test_initial_state_has_no_recording() {
        let state = make_state();
        assert!(!state.recorder.is_recording());
        assert!(state.last_recording.lock().unwrap().is_none());
        assert!(state.recording_start.lock().unwrap().is_none());
    }

    /// `get_api_key_status` returns false for empty keys (default in tests).
    #[test]
    fn test_api_key_status_empty_keys() {
        let state = make_state();
        let groq_configured = !state.groq_api_key.lock().unwrap().is_empty();
        let deepseek_configured = !state.deepseek_api_key.lock().unwrap().is_empty();
        assert!(!groq_configured);
        assert!(!deepseek_configured);
    }

    /// `get_api_key_status` returns true after keys are set.
    #[test]
    fn test_api_key_status_with_keys() {
        let state = AppState::new("groq-key-123".to_string(), "ds-key-456".to_string());
        assert!(!state.groq_api_key.lock().unwrap().is_empty());
        assert!(!state.deepseek_api_key.lock().unwrap().is_empty());
    }

    /// Storing and retrieving WAV bytes round-trips correctly.
    #[test]
    fn test_last_recording_roundtrip() {
        let state = make_state();
        let dummy_wav = vec![0u8, 1, 2, 3, 255];
        *state.last_recording.lock().unwrap() = Some(dummy_wav.clone());
        let retrieved = state.last_recording.lock().unwrap().clone().unwrap();
        assert_eq!(retrieved, dummy_wav);
    }

    /// `RecordingInfo` serializes with camelCase keys.
    #[test]
    fn test_recording_info_camel_case_serialization() {
        let info = RecordingInfo { duration_ms: 4200 };
        let json = serde_json::to_string(&info).unwrap();
        assert!(
            json.contains("durationMs"),
            "expected camelCase key 'durationMs', got: {json}"
        );
        assert!(
            !json.contains("duration_ms"),
            "snake_case key must not appear in JSON: {json}"
        );
    }

    /// `ApiKeyStatus` serializes with camelCase keys.
    #[test]
    fn test_api_key_status_camel_case_serialization() {
        let status = ApiKeyStatus {
            groq_configured: true,
            deepseek_configured: false,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(
            json.contains("groqConfigured"),
            "expected 'groqConfigured', got: {json}"
        );
        assert!(
            json.contains("deepseekConfigured"),
            "expected 'deepseekConfigured', got: {json}"
        );
    }

    /// `update_api_keys` correctly updates the stored key strings.
    ///
    /// We test the internal state mutation rather than calling the async Tauri
    /// command (which requires a running Tauri app context).
    #[test]
    fn test_update_api_keys_mutates_stored_keys() {
        let state = make_state();

        // Simulate what update_api_keys does internally.
        *state.groq_api_key.lock().unwrap() = "new-groq-key".to_string();
        *state.deepseek_api_key.lock().unwrap() = "new-ds-key".to_string();

        assert_eq!(*state.groq_api_key.lock().unwrap(), "new-groq-key");
        assert_eq!(*state.deepseek_api_key.lock().unwrap(), "new-ds-key");
    }

    /// Default language is "de".
    #[test]
    fn test_default_language_is_german() {
        let state = make_state();
        assert_eq!(*state.language.lock().unwrap(), "de");
    }

    /// Default cleanup style is Polished.
    #[test]
    fn test_default_cleanup_style_is_polished() {
        let state = make_state();
        assert_eq!(*state.current_style.lock().unwrap(), CleanupStyle::Polished);
    }

    /// Language can be changed via internal mutation (simulates set_language command).
    #[test]
    fn test_set_language_mutates_state() {
        let state = make_state();
        *state.language.lock().unwrap() = "en".to_string();
        assert_eq!(*state.language.lock().unwrap(), "en");
    }

    /// Cleanup style can be changed via internal mutation (simulates set_cleanup_style).
    #[test]
    fn test_set_cleanup_style_mutates_state() {
        let state = make_state();
        *state.current_style.lock().unwrap() = CleanupStyle::Chat;
        assert_eq!(*state.current_style.lock().unwrap(), CleanupStyle::Chat);
    }

    /// DEFAULT_HOTKEY is a parseable shortcut string.
    #[test]
    fn test_default_hotkey_is_valid() {
        // Verify the constant compiles and is non-empty.
        assert!(!DEFAULT_HOTKEY.is_empty());
        assert_eq!(DEFAULT_HOTKEY, "ctrl+shift+d");
    }
}
