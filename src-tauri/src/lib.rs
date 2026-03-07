//! Dikta -- Tauri backend entry point.
//!
//! Wires together the audio, STT, LLM, paste, hotkey, config and dictionary
//! modules and exposes them to the React frontend via Tauri commands and events.
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
//!
//! ## Settings & Dictionary persistence
//!
//! On startup the app loads `config.json` and `dictionary.json` from the Tauri
//! app-data directory. Settings can be updated at runtime via `save_settings`;
//! dictionary terms via `add_dictionary_term` / `remove_dictionary_term`.
//! Both writes are synchronous and happen in the command handler -- no
//! background flush needed for these small files.

mod audio;
mod config;
mod dictionary;
mod hotkey;
mod llm;
mod paste;
mod stt;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use audio::AudioRecorder;
use config::{load_config, save_config, AppConfig, HotkeyMode};
use dictionary::{load_dictionary, save_dictionary, Dictionary};
use hotkey::{PipelineEvent, EVENT_STATE_CHANGED};
use llm::{CleanupProvider, CleanupStyle, DeepSeekCleanup};
use paste::{capture_foreground_window, create_paste_handler};
use serde::{Deserialize, Serialize};
use stt::{GroqWhisper, SttProvider};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WindowEvent};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconEvent;
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

/// Returned by `get_settings`. API keys are masked -- only the last 4
/// characters are visible so the frontend can show a "configured" indicator
/// without exposing the full secret.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SettingsView {
    /// Masked Groq API key, e.g. `"****abcd"`. Empty string if not set.
    pub groq_api_key_masked: String,
    /// Masked DeepSeek API key, e.g. `"****wxyz"`. Empty string if not set.
    pub deepseek_api_key_masked: String,
    /// ISO-639-1 language code, e.g. `"de"`.
    pub language: String,
    /// Current cleanup style.
    pub cleanup_style: CleanupStyle,
    /// Registered global hotkey string, e.g. `"ctrl+shift+d"`.
    pub hotkey: String,
    /// How the hotkey triggers recording: `Hold` or `Toggle`.
    pub hotkey_mode: HotkeyMode,
    /// Name of the selected audio input device. `None` = system default.
    pub audio_device: Option<String>,
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
/// `RwLock` around provider `Arc`s allows `save_settings` to swap out a
/// provider at runtime without restarting the application.
///
/// Tauri requires `State<T>: Send + Sync + 'static`.
pub struct AppState {
    recorder: Arc<AudioRecorder>,
    /// Wrapping in `RwLock` lets `save_settings` replace the provider.
    stt_provider: RwLock<Arc<dyn SttProvider>>,
    cleanup_provider: RwLock<Arc<dyn CleanupProvider>>,
    /// Timestamp set by `start_recording`, cleared by `stop_recording`.
    recording_start: Mutex<Option<std::time::Instant>>,
    /// WAV bytes from the most recent recording. Set by `stop_recording`,
    /// consumed (read, not cleared) by `transcribe_audio`.
    last_recording: Mutex<Option<Vec<u8>>>,
    /// Full persisted configuration (includes API keys).
    config: Mutex<AppConfig>,
    /// User's custom word list -- injected into STT prompt and LLM system prompt.
    dictionary: Mutex<Dictionary>,
    /// Path to the app-data directory for persisting config and dictionary.
    app_data_dir: PathBuf,
    /// Window handle (HWND) of the app that was focused when recording started.
    /// Used on Windows to restore focus before pasting.
    prev_foreground_hwnd: Mutex<Option<isize>>,
}

// SAFETY: All fields are either `Arc<_>`, `Mutex<_>`, or `RwLock<_>`, which
// are `Send + Sync` when their inner types are `Send`.
// `AudioRecorder` carries its own `unsafe impl Send + Sync` in audio/mod.rs.
// The trait objects (`Arc<dyn SttProvider>` etc.) require `Send + Sync` bounds
// on the traits (both traits have those bounds).
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl AppState {
    fn new(cfg: AppConfig, dictionary: Dictionary, app_data_dir: PathBuf) -> Self {
        AppState {
            recorder: Arc::new(AudioRecorder::new()),
            stt_provider: RwLock::new(Arc::new(GroqWhisper::new(cfg.groq_api_key.clone()))),
            cleanup_provider: RwLock::new(Arc::new(DeepSeekCleanup::new(
                cfg.deepseek_api_key.clone(),
            ))),
            recording_start: Mutex::new(None),
            last_recording: Mutex::new(None),
            config: Mutex::new(cfg),
            dictionary: Mutex::new(dictionary),
            app_data_dir,
            prev_foreground_hwnd: Mutex::new(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper -- lock/rwlock macros with descriptive error strings
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
// Helpers
// ---------------------------------------------------------------------------

/// Masks an API key for safe display in the frontend.
///
/// Returns an empty string if the key is empty.
/// Returns `"****{last4}"` for keys longer than 4 characters.
/// Returns `"****"` for keys with 4 or fewer characters (avoids leaking short keys).
fn mask_api_key(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    if key.len() <= 4 {
        return "****".to_string();
    }
    format!("****{}", &key[key.len() - 4..])
}

// ---------------------------------------------------------------------------
// Hotkey dictation pipeline
// ---------------------------------------------------------------------------

/// Starts recording audio and emits `state=recording`.
///
/// Does nothing (returns silently) if recording is already in progress.
/// Used by the hold-mode hotkey handler on key-press.
async fn start_recording_only(handle: AppHandle) {
    let state = handle.state::<AppState>();

    if state.recorder.is_recording() {
        return;
    }

    // Capture the foreground window BEFORE we start recording.
    // This is the window the user was typing in -- we'll restore focus to it
    // before pasting the result.
    if let Ok(mut guard) = state.prev_foreground_hwnd.lock() {
        *guard = capture_foreground_window();
    }

    // Re-install the audio level callback before each recording.
    setup_audio_level_emitter(&handle);

    let device_name = state.config.lock().ok().and_then(|c| c.audio_device.clone());
    if let Err(e) = state.recorder.start_recording(device_name.as_deref()) {
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
}

/// Stops the active recording and runs the full STT → LLM → paste pipeline.
///
/// Does nothing (returns silently) if no recording is active.
/// Used by the hold-mode hotkey handler on key-release, and called internally
/// by `run_dictation_pipeline` for the toggle case.
///
/// Dictionary terms are injected at both the STT step (as a Groq `prompt`
/// hint) and the LLM step (as `dictionary_terms` in the system prompt).
async fn stop_and_process_pipeline(handle: AppHandle) {
    let state = handle.state::<AppState>();

    if !state.recorder.is_recording() {
        // Not recording -- key released without a corresponding press (race condition or
        // hold mode released before recording started). Safe to ignore.
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

    log::debug!(
        "[pipeline] recording stopped after {duration_ms}ms, {len} WAV bytes",
        len = wav_bytes.len()
    );

    // --- Silence detection ---
    // If the recording is very short (<500ms) or essentially silent, skip the
    // STT/LLM pipeline. This matches Wispr Flow's "nothing said" behaviour.
    if duration_ms < 500 {
        log::info!("[pipeline] recording too short ({duration_ms}ms), skipping");
        let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::idle());
        return;
    }

    // Check RMS of the raw WAV samples. If the audio is near-silent, abort.
    if let Some(rms) = compute_wav_rms(&wav_bytes) {
        log::debug!("[pipeline] audio RMS = {rms:.5}");
        if rms < 0.005 {
            log::info!("[pipeline] audio is silent (rms={rms:.5}), skipping");
            let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::idle());
            return;
        }
    }

    // --- Collect config + dictionary (release locks before await points) ---
    let (language, stt_provider, cleanup_provider, dict_prompt) = {
        let cfg = match state.config.lock() {
            Ok(g) => g.clone(),
            Err(_) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned"),
                );
                return;
            }
        };

        let stt_prov = match state.stt_provider.read() {
            Ok(g) => g.clone(),
            Err(_) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned"),
                );
                return;
            }
        };

        let cleanup_prov = match state.cleanup_provider.read() {
            Ok(g) => g.clone(),
            Err(_) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned"),
                );
                return;
            }
        };

        let prompt = match state.dictionary.lock() {
            Ok(g) => {
                let p = g.terms_as_prompt();
                if p.is_empty() { None } else { Some(p) }
            }
            Err(_) => None,
        };

        (cfg.language.clone(), stt_prov, cleanup_prov, prompt)
    };

    // --- Transcribe ---
    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::transcribing());

    let raw_text = match stt_provider
        .transcribe(wav_bytes, &language, dict_prompt.as_deref())
        .await
    {
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
        match state.config.lock() {
            Ok(g) => g.cleanup_style,
            Err(_) => CleanupStyle::Polished,
        }
    };

    // Re-read dictionary for the LLM prompt (separate concern from STT prompt).
    let dict_list = match state.dictionary.lock() {
        Ok(g) => {
            let l = g.terms_as_list();
            if l.is_empty() { None } else { Some(l) }
        }
        Err(_) => None,
    };

    let cleaned_text = match cleanup_provider
        .cleanup(&raw_text, style, dict_list.as_deref())
        .await
    {
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
    let prev_hwnd = state.prev_foreground_hwnd.lock().ok().and_then(|g| *g);
    let paste_handler = create_paste_handler(prev_hwnd);
    if let Err(e) = paste_handler.paste(&cleaned_text) {
        log::warn!("[pipeline] paste failed: {e}. Text is still available.");
        // Do not return error -- text was produced, paste failure is non-fatal.
    }

    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::done(cleaned_text));
}

/// Toggle-mode hotkey handler: press once to start, press again to stop + process.
///
/// This is the legacy behaviour, kept for users who prefer toggle mode.
async fn run_dictation_pipeline(handle: AppHandle) {
    let state = handle.state::<AppState>();

    if !state.recorder.is_recording() {
        start_recording_only(handle).await;
    } else {
        stop_and_process_pipeline(handle).await;
    }
}

/// Registers the global shortcut with mode-aware handlers.
///
/// Unregisters all existing shortcuts first so this can be called to
/// re-register after a settings change.
///
/// - `Toggle`: Pressed fires `run_dictation_pipeline` (start or stop+process).
/// - `Hold`: Pressed fires `start_recording_only`; Released fires `stop_and_process_pipeline`.
fn register_hotkey(handle: &AppHandle, shortcut: Shortcut, mode: HotkeyMode) -> Result<(), String> {
    println!("[hotkey] Registering shortcut: {shortcut:?} mode={mode:?}");

    handle
        .global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {e}"))?;

    let handle_clone = handle.clone();
    handle
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            println!("[hotkey] Event: {event:?}");

            let h = handle_clone.clone();
            println!("[hotkey] mode={mode:?} state={:?}", event.state);
            match (mode, event.state) {
                (HotkeyMode::Toggle, ShortcutState::Pressed) => {
                    tauri::async_runtime::spawn(async move {
                        run_dictation_pipeline(h).await;
                    });
                }
                (HotkeyMode::Hold, ShortcutState::Pressed) => {
                    tauri::async_runtime::spawn(async move {
                        start_recording_only(h).await;
                    });
                }
                (HotkeyMode::Hold, ShortcutState::Released) => {
                    tauri::async_runtime::spawn(async move {
                        stop_and_process_pipeline(h).await;
                    });
                }
                // Toggle + Released: ignore (we only act on press).
                _ => {}
            }
        })
        .map_err(|e| format!("Failed to register shortcut: {e}"))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri commands -- Recording
// ---------------------------------------------------------------------------

/// Opens the default microphone and starts capturing audio.
///
/// Returns an error string if recording is already in progress or no
/// microphone is available.
#[tauri::command]
async fn start_recording(handle: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let inner = state.inner();

    // Re-install the audio level callback before recording.
    setup_audio_level_emitter(&handle);

    let device_name = lock!(inner.config)?.audio_device.clone();
    inner
        .recorder
        .start_recording(device_name.as_deref())
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
/// Dictionary terms are injected as a Groq `prompt` hint to improve accuracy
/// for technical vocabulary.
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

    // Read dictionary terms for the STT prompt hint.
    let dict_prompt = {
        let guard = lock!(inner.dictionary)?;
        let p = guard.terms_as_prompt();
        if p.is_empty() { None } else { Some(p) }
    };

    // Read the current provider (shared read lock -- no contention with other readers).
    let provider = read_lock!(inner.stt_provider)?.clone();

    provider
        .transcribe(wav_bytes, &language, dict_prompt.as_deref())
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
/// `dictionary_terms`: optional comma-separated list of terms to preserve
///   verbatim. If `None`, the current app dictionary is used automatically.
#[tauri::command]
async fn cleanup_text(
    state: State<'_, AppState>,
    raw_text: String,
    style: CleanupStyle,
    dictionary_terms: Option<String>,
) -> Result<String, String> {
    let inner = state.inner();
    let provider = read_lock!(inner.cleanup_provider)?.clone();

    // Use caller-supplied terms if provided; otherwise fall back to app dictionary.
    let terms = match dictionary_terms {
        Some(t) => {
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        }
        None => {
            let guard = lock!(inner.dictionary)?;
            let l = guard.terms_as_list();
            if l.is_empty() { None } else { Some(l) }
        }
    };

    provider
        .cleanup(&raw_text, style, terms.as_deref())
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

// ---------------------------------------------------------------------------
// Tauri commands -- Settings
// ---------------------------------------------------------------------------

/// Persists new settings and hot-reloads the affected providers.
///
/// After saving to disk:
/// - STT and LLM providers are replaced with new instances (API key changes
///   take effect immediately without a restart).
/// - The global shortcut is re-registered with the new hotkey string and mode.
///
/// Passing an empty string for an API key disables that provider (requests
/// will fail with an auth error from the API until a valid key is supplied).
#[tauri::command]
async fn save_settings(
    handle: AppHandle,
    state: State<'_, AppState>,
    groq_api_key: String,
    deepseek_api_key: String,
    language: String,
    cleanup_style: CleanupStyle,
    hotkey: String,
    hotkey_mode: HotkeyMode,
    audio_device: Option<String>,
) -> Result<(), String> {
    let inner = state.inner();

    // Validate the hotkey string before writing anything to disk.
    println!("[save_settings] hotkey={hotkey:?} mode={hotkey_mode:?}");
    let parsed_shortcut = hotkey
        .parse::<Shortcut>()
        .map_err(|e| {
            println!("[save_settings] Invalid shortcut: {e}");
            format!("Invalid shortcut string: {e}")
        })?;

    // Build updated config. Empty API key strings preserve the existing key
    // so the user can change other settings without re-entering keys.
    let existing = lock!(inner.config)?.clone();
    let new_cfg = AppConfig {
        groq_api_key: if groq_api_key.is_empty() { existing.groq_api_key } else { groq_api_key.clone() },
        deepseek_api_key: if deepseek_api_key.is_empty() { existing.deepseek_api_key } else { deepseek_api_key.clone() },
        language,
        cleanup_style,
        hotkey,
        hotkey_mode,
        audio_device,
    };
    let effective_groq = new_cfg.groq_api_key.clone();
    let effective_deepseek = new_cfg.deepseek_api_key.clone();

    // Persist to disk.
    save_config(&inner.app_data_dir, &new_cfg)
        .map_err(|e| format!("Failed to save settings: {e}"))?;

    // Update in-memory config.
    *lock!(inner.config)? = new_cfg;

    // Hot-reload providers with the effective API keys.
    *write_lock!(inner.stt_provider)? = Arc::new(GroqWhisper::new(effective_groq));
    *write_lock!(inner.cleanup_provider)? = Arc::new(DeepSeekCleanup::new(effective_deepseek));

    // Re-register the global shortcut with the (possibly new) hotkey + mode.
    register_hotkey(&handle, parsed_shortcut, hotkey_mode)?;

    Ok(())
}

/// Returns the current settings for display in the frontend.
///
/// API keys are masked (only last 4 characters visible) so this can be sent
/// to the frontend without exposing the full secrets.
#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Result<SettingsView, String> {
    let cfg = lock!(state.inner().config)?.clone();

    Ok(SettingsView {
        groq_api_key_masked: mask_api_key(&cfg.groq_api_key),
        deepseek_api_key_masked: mask_api_key(&cfg.deepseek_api_key),
        language: cfg.language,
        cleanup_style: cfg.cleanup_style,
        hotkey: cfg.hotkey,
        hotkey_mode: cfg.hotkey_mode,
        audio_device: cfg.audio_device,
    })
}

/// Returns which API keys are currently configured (non-empty).
///
/// Does NOT return the key values themselves -- only booleans indicating
/// presence. The frontend uses this to show configuration status.
#[tauri::command]
fn get_api_key_status(state: State<'_, AppState>) -> Result<ApiKeyStatus, String> {
    let cfg = lock!(state.inner().config)?.clone();

    Ok(ApiKeyStatus {
        groq_configured: !cfg.groq_api_key.is_empty(),
        deepseek_configured: !cfg.deepseek_api_key.is_empty(),
    })
}

/// Replaces the STT and/or LLM provider with a new instance using the supplied
/// API keys. Settings are also persisted to disk.
///
/// Passing `None` for a key leaves that provider unchanged.
/// Passing `Some("")` effectively disables the provider.
///
/// Kept for backward compatibility with existing frontend code.
/// New code should prefer `save_settings`.
#[tauri::command]
async fn update_api_keys(
    state: State<'_, AppState>,
    groq_api_key: Option<String>,
    deepseek_api_key: Option<String>,
) -> Result<(), String> {
    let inner = state.inner();

    {
        let mut cfg = lock!(inner.config)?;

        if let Some(ref key) = groq_api_key {
            cfg.groq_api_key = key.clone();
        }
        if let Some(ref key) = deepseek_api_key {
            cfg.deepseek_api_key = key.clone();
        }

        // Persist updated config.
        let cfg_clone = cfg.clone();
        drop(cfg); // release lock before I/O
        save_config(&inner.app_data_dir, &cfg_clone)
            .map_err(|e| format!("Failed to persist API keys: {e}"))?;
    }

    if let Some(key) = groq_api_key {
        *write_lock!(inner.stt_provider)? = Arc::new(GroqWhisper::new(key));
    }

    if let Some(key) = deepseek_api_key {
        *write_lock!(inner.cleanup_provider)? = Arc::new(DeepSeekCleanup::new(key));
    }

    Ok(())
}

/// Sets the language used by the hotkey pipeline and persists the change.
///
/// `language`: ISO-639-1 code, e.g. `"de"` or `"en"`. Empty string = auto-detect.
#[tauri::command]
fn set_language(state: State<'_, AppState>, language: String) -> Result<(), String> {
    let inner = state.inner();
    let mut cfg = lock!(inner.config)?;
    cfg.language = language;
    let cfg_clone = cfg.clone();
    drop(cfg);
    save_config(&inner.app_data_dir, &cfg_clone)
        .map_err(|e| format!("Failed to persist language setting: {e}"))
}

/// Sets the cleanup style used by the hotkey pipeline and persists the change.
#[tauri::command]
fn set_cleanup_style(state: State<'_, AppState>, style: CleanupStyle) -> Result<(), String> {
    let inner = state.inner();
    let mut cfg = lock!(inner.config)?;
    cfg.cleanup_style = style;
    let cfg_clone = cfg.clone();
    drop(cfg);
    save_config(&inner.app_data_dir, &cfg_clone)
        .map_err(|e| format!("Failed to persist cleanup style: {e}"))
}

/// Changes the registered global hotkey and/or mode at runtime.
///
/// `shortcut`: a Tauri shortcut string, e.g. `"ctrl+shift+d"`.
/// `mode`: `HotkeyMode::Hold` or `HotkeyMode::Toggle`.
///
/// Returns an error if the shortcut string is invalid or registration fails.
/// Persists both the new shortcut and mode to config.
#[tauri::command]
async fn set_hotkey(
    handle: AppHandle,
    state: State<'_, AppState>,
    shortcut: String,
    mode: HotkeyMode,
) -> Result<(), String> {
    // Validate the shortcut string before touching system APIs.
    let parsed = shortcut
        .parse::<Shortcut>()
        .map_err(|e| format!("Invalid shortcut string: {e}"))?;

    // Re-register with the new shortcut + mode.
    register_hotkey(&handle, parsed, mode)?;

    // Persist both fields to config.
    let inner = state.inner();
    let mut cfg = lock!(inner.config)?;
    cfg.hotkey = shortcut;
    cfg.hotkey_mode = mode;
    let cfg_clone = cfg.clone();
    drop(cfg);
    save_config(&inner.app_data_dir, &cfg_clone)
        .map_err(|e| format!("Failed to persist hotkey setting: {e}"))
}

// ---------------------------------------------------------------------------
// Tauri commands -- Audio devices
// ---------------------------------------------------------------------------

/// Returns the names of all available audio input devices.
#[tauri::command]
fn list_audio_devices() -> Vec<String> {
    audio::list_input_devices()
}

// ---------------------------------------------------------------------------
// Tauri commands -- Dictionary
// ---------------------------------------------------------------------------

/// Returns all terms in the custom dictionary.
#[tauri::command]
fn get_dictionary_terms(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let guard = lock!(state.inner().dictionary)?;
    Ok(guard.terms().to_vec())
}

/// Adds a term to the custom dictionary and persists the change.
///
/// Duplicate terms (case-insensitive) and empty strings are silently ignored.
#[tauri::command]
fn add_dictionary_term(state: State<'_, AppState>, term: String) -> Result<(), String> {
    let inner = state.inner();
    let mut dict = lock!(inner.dictionary)?;
    dict.add_term(term);
    let dict_clone = dict.clone();
    drop(dict);
    save_dictionary(&inner.app_data_dir, &dict_clone)
        .map_err(|e| format!("Failed to save dictionary: {e}"))
}

/// Removes a term from the custom dictionary and persists the change.
///
/// Does nothing if the term is not present.
#[tauri::command]
fn remove_dictionary_term(state: State<'_, AppState>, term: String) -> Result<(), String> {
    let inner = state.inner();
    let mut dict = lock!(inner.dictionary)?;
    dict.remove_term(&term);
    let dict_clone = dict.clone();
    drop(dict);
    save_dictionary(&inner.app_data_dir, &dict_clone)
        .map_err(|e| format!("Failed to save dictionary: {e}"))
}

// ---------------------------------------------------------------------------
// Silence detection helper
// ---------------------------------------------------------------------------

/// Parses a WAV byte buffer and computes the overall RMS of the audio samples.
///
/// Returns `None` if the WAV cannot be parsed (should not happen since we
/// encoded it ourselves, but we handle it gracefully).
fn compute_wav_rms(wav_bytes: &[u8]) -> Option<f32> {
    let cursor = std::io::Cursor::new(wav_bytes);
    let mut reader = match hound::WavReader::new(cursor) {
        Ok(r) => r,
        Err(_) => return None,
    };

    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .filter_map(|s| s.ok())
            .collect(),
        hound::SampleFormat::Int => {
            let max_val = (1_i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
    };

    if samples.is_empty() {
        return Some(0.0);
    }

    Some(audio::compute_rms(&samples))
}

// ---------------------------------------------------------------------------
// Default hotkey string
// ---------------------------------------------------------------------------

const DEFAULT_HOTKEY: &str = "ctrl+shift+d";

// ---------------------------------------------------------------------------
// Tauri entry point
// ---------------------------------------------------------------------------

/// Event name for real-time audio level updates sent to the floating bar.
const EVENT_AUDIO_LEVEL: &str = "dikta://audio-level";

/// Creates the floating bar window positioned above the taskbar.
fn create_bar_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Start as a tiny dot -- the frontend resizes dynamically based on state.
    let bar_width = 44.0_f64;
    let bar_height = 32.0_f64;

    let builder = tauri::WebviewWindowBuilder::new(
        app,
        "bar",
        WebviewUrl::App("index.html".into()),
    )
    .title("")
    .inner_size(bar_width, bar_height)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .resizable(false)
    .skip_taskbar(true)
    .focused(false);

    let bar = builder.build()?;

    // Position at bottom-center of the screen, above the taskbar (~60px margin).
    match bar.current_monitor() {
        Ok(Some(monitor)) => {
            let screen_size = monitor.size();
            let scale = monitor.scale_factor();
            let screen_w = screen_size.width as f64 / scale;
            let screen_h = screen_size.height as f64 / scale;
            let x = (screen_w - bar_width) / 2.0;
            let y = screen_h - bar_height - 60.0;
            log::info!("[bar] screen={screen_w}x{screen_h} scale={scale}, placing at ({x}, {y})");
            let _ = bar.set_position(tauri::LogicalPosition::new(x, y));
        }
        _ => {
            // Fallback: top-center (WSL2 may not report monitors correctly).
            log::warn!("[bar] No monitor detected, using fallback position");
            let _ = bar.set_position(tauri::LogicalPosition::new(400.0, 10.0));
        }
    }

    Ok(())
}

/// Sets up the audio-level callback that emits events to the frontend.
fn setup_audio_level_emitter(handle: &AppHandle) {
    let state = handle.state::<AppState>();
    let handle_clone = handle.clone();
    state.recorder.set_level_callback(Box::new(move |level| {
        let _ = handle_clone.emit(EVENT_AUDIO_LEVEL, serde_json::json!({ "level": level }));
    }));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            // Resolve the app-data directory (e.g. %APPDATA%\com.dikta.voice on Windows).
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Tauri must provide an app-data directory");

            // Create the directory if it doesn't exist yet.
            std::fs::create_dir_all(&app_data_dir)?;

            // Load persisted config (falls back to defaults + env vars on first run).
            let cfg = load_config(&app_data_dir);

            // Restore the hotkey from config (or fall back to the compile-time default).
            let hotkey_str = if cfg.hotkey.is_empty() {
                DEFAULT_HOTKEY.to_string()
            } else {
                cfg.hotkey.clone()
            };

            let hotkey_mode = cfg.hotkey_mode;

            // Load persisted dictionary.
            let dictionary = load_dictionary(&app_data_dir);

            log::info!(
                "[setup] Loaded config: language={}, style={:?}, hotkey={}, mode={:?}",
                cfg.language,
                cfg.cleanup_style,
                hotkey_str,
                hotkey_mode,
            );
            log::info!(
                "[setup] Loaded dictionary: {} terms",
                dictionary.len()
            );

            // Build and register the application state.
            let app_state = AppState::new(cfg, dictionary, app_data_dir);
            app.manage(app_state);

            // --- System tray (Windows only -- WSL2/Linux lacks proper tray support) ---
            #[cfg(target_os = "windows")]
            {
                let show_settings = MenuItem::with_id(app, "show_settings", "Settings", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_settings, &quit])?;

                let _tray = tauri::tray::TrayIconBuilder::with_id("dikta-tray")
                    .icon(app.default_window_icon().unwrap().clone())
                    .tooltip("Dikta")
                    .menu(&menu)
                    .on_menu_event(|app, event| {
                        match event.id.as_ref() {
                            "show_settings" => {
                                if let Some(w) = app.get_webview_window("main") {
                                    let _ = w.show();
                                    let _ = w.set_focus();
                                }
                            }
                            "quit" => {
                                app.exit(0);
                            }
                            _ => {}
                        }
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                            if let Some(w) = tray.app_handle().get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                    })
                    .build(app)?;
            }

            // --- Floating bar window ---
            // Only create on Windows; WSL2/Linux lacks decorations(false),
            // transparency, and proper always-on-top support.
            #[cfg(target_os = "windows")]
            if let Err(e) = create_bar_window(app) {
                log::warn!("[setup] Could not create floating bar: {e}");
            }

            // --- Audio level emitter ---
            let handle = app.handle().clone();
            setup_audio_level_emitter(&handle);

            // Register the global hotkey from config.
            println!("[setup] Parsing hotkey: {hotkey_str:?}");
            let shortcut = hotkey_str
                .parse::<Shortcut>()
                .unwrap_or_else(|e| {
                    log::warn!(
                        "[hotkey] Saved hotkey {:?} is invalid ({e}), falling back to default",
                        hotkey_str
                    );
                    DEFAULT_HOTKEY
                        .parse::<Shortcut>()
                        .expect("DEFAULT_HOTKEY must be a valid shortcut string")
                });

            match register_hotkey(&handle, shortcut, hotkey_mode) {
                Ok(()) => log::info!("[hotkey] Registered shortcut: {hotkey_str} (mode={hotkey_mode:?})"),
                Err(e) => log::warn!("[hotkey] Could not register shortcut: {e}. Use the UI button instead."),
            }
            Ok(())
        })
        // On Windows with a working system tray, we hide windows on close
        // instead of quitting. On Linux/WSL2 (no tray), closing main = quit.
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                // Bar window: always prevent close (it should always exist).
                if label == "bar" {
                    let _ = window.hide();
                    api.prevent_close();
                }
                // Main window: hide only if tray is available (Windows).
                // On Linux/WSL2, let it close normally (= quit the app).
                #[cfg(target_os = "windows")]
                if label == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Recording
            start_recording,
            stop_recording,
            transcribe_audio,
            cleanup_text,
            is_recording,
            // Settings
            save_settings,
            get_settings,
            get_api_key_status,
            update_api_keys,
            set_language,
            set_cleanup_style,
            set_hotkey,
            // Audio devices
            list_audio_devices,
            // Dictionary
            get_dictionary_terms,
            add_dictionary_term,
            remove_dictionary_term,
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
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    fn make_state(dir: &TempDir) -> AppState {
        AppState::new(AppConfig::default(), Dictionary::new(), dir.path().to_path_buf())
    }

    // --- AppState initial conditions ---

    #[test]
    fn test_initial_state_has_no_recording() {
        let dir = temp_dir();
        let state = make_state(&dir);
        assert!(!state.recorder.is_recording());
        assert!(state.last_recording.lock().unwrap().is_none());
        assert!(state.recording_start.lock().unwrap().is_none());
    }

    #[test]
    fn test_initial_config_defaults() {
        let dir = temp_dir();
        let state = make_state(&dir);
        let cfg = state.config.lock().unwrap();
        assert_eq!(cfg.language, "de");
        assert_eq!(cfg.cleanup_style, CleanupStyle::Polished);
        assert_eq!(cfg.hotkey, "ctrl+shift+d");
        assert_eq!(cfg.hotkey_mode, HotkeyMode::Hold);
    }

    #[test]
    fn test_initial_dictionary_is_empty() {
        let dir = temp_dir();
        let state = make_state(&dir);
        assert!(state.dictionary.lock().unwrap().is_empty());
    }

    // --- mask_api_key ---

    #[test]
    fn test_mask_api_key_empty() {
        assert_eq!(mask_api_key(""), "");
    }

    #[test]
    fn test_mask_api_key_short_key() {
        assert_eq!(mask_api_key("abc"), "****");
        assert_eq!(mask_api_key("abcd"), "****");
    }

    #[test]
    fn test_mask_api_key_long_key() {
        assert_eq!(mask_api_key("gsk_somereallylongapikey"), "****ikey");
        assert_eq!(mask_api_key("abcde"), "****bcde");
    }

    #[test]
    fn test_mask_api_key_exactly_five_chars() {
        // 5 chars: last 4 are "bcde", prefix is "a"
        assert_eq!(mask_api_key("abcde"), "****bcde");
    }

    // --- SettingsView serialization ---

    #[test]
    fn test_settings_view_camel_case_serialization() {
        let view = SettingsView {
            groq_api_key_masked: "****1234".to_string(),
            deepseek_api_key_masked: "****5678".to_string(),
            language: "de".to_string(),
            cleanup_style: CleanupStyle::Polished,
            hotkey: "ctrl+shift+d".to_string(),
            hotkey_mode: HotkeyMode::Hold,
            audio_device: None,
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("groqApiKeyMasked"), "expected camelCase key");
        assert!(json.contains("deepseekApiKeyMasked"), "expected camelCase key");
        assert!(json.contains("cleanupStyle"), "expected camelCase key");
        assert!(json.contains("hotkeyMode"), "expected camelCase 'hotkeyMode'");
    }

    // --- HotkeyMode via SettingsView ---

    #[test]
    fn test_settings_view_hotkey_mode_hold_serializes_lowercase() {
        let view = SettingsView {
            groq_api_key_masked: String::new(),
            deepseek_api_key_masked: String::new(),
            language: "de".to_string(),
            cleanup_style: CleanupStyle::Polished,
            hotkey: "ctrl+shift+d".to_string(),
            hotkey_mode: HotkeyMode::Hold,
            audio_device: None,
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains(r#""hotkeyMode":"hold""#), "hold variant must serialize as lowercase 'hold'");
    }

    #[test]
    fn test_settings_view_hotkey_mode_toggle_serializes_lowercase() {
        let view = SettingsView {
            groq_api_key_masked: String::new(),
            deepseek_api_key_masked: String::new(),
            language: "de".to_string(),
            cleanup_style: CleanupStyle::Polished,
            hotkey: "ctrl+shift+d".to_string(),
            hotkey_mode: HotkeyMode::Toggle,
            audio_device: None,
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains(r#""hotkeyMode":"toggle""#), "toggle variant must serialize as lowercase 'toggle'");
    }

    // --- ApiKeyStatus ---

    #[test]
    fn test_api_key_status_empty_keys() {
        let dir = temp_dir();
        let state = make_state(&dir);
        let cfg = state.config.lock().unwrap();
        assert!(!cfg.groq_api_key.is_empty() || cfg.groq_api_key.is_empty()); // tautology check
        assert!(cfg.groq_api_key.is_empty());
        assert!(cfg.deepseek_api_key.is_empty());
    }

    #[test]
    fn test_api_key_status_with_keys() {
        let dir = temp_dir();
        let cfg = AppConfig {
            groq_api_key: "groq-key-123".to_string(),
            deepseek_api_key: "ds-key-456".to_string(),
            ..AppConfig::default()
        };
        let state = AppState::new(cfg, Dictionary::new(), dir.path().to_path_buf());
        let locked = state.config.lock().unwrap();
        assert!(!locked.groq_api_key.is_empty());
        assert!(!locked.deepseek_api_key.is_empty());
    }

    // --- WAV roundtrip ---

    #[test]
    fn test_last_recording_roundtrip() {
        let dir = temp_dir();
        let state = make_state(&dir);
        let dummy_wav = vec![0u8, 1, 2, 3, 255];
        *state.last_recording.lock().unwrap() = Some(dummy_wav.clone());
        let retrieved = state.last_recording.lock().unwrap().clone().unwrap();
        assert_eq!(retrieved, dummy_wav);
    }

    // --- Serialization invariants ---

    #[test]
    fn test_recording_info_camel_case_serialization() {
        let info = RecordingInfo { duration_ms: 4200 };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("durationMs"), "expected camelCase 'durationMs'");
        assert!(!json.contains("duration_ms"), "snake_case must not appear");
    }

    #[test]
    fn test_api_key_status_camel_case_serialization() {
        let status = ApiKeyStatus {
            groq_configured: true,
            deepseek_configured: false,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("groqConfigured"));
        assert!(json.contains("deepseekConfigured"));
    }

    // --- Dictionary mutation (internal, without Tauri context) ---

    #[test]
    fn test_dictionary_add_and_remove_via_state() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut dict = state.dictionary.lock().unwrap();
            dict.add_term("Kubernetes".to_string());
            dict.add_term("TypeScript".to_string());
        }
        {
            let dict = state.dictionary.lock().unwrap();
            assert_eq!(dict.len(), 2);
            assert_eq!(dict.terms_as_prompt(), "Kubernetes, TypeScript");
        }
        {
            let mut dict = state.dictionary.lock().unwrap();
            dict.remove_term("Kubernetes");
        }
        {
            let dict = state.dictionary.lock().unwrap();
            assert_eq!(dict.len(), 1);
            assert_eq!(dict.terms()[0], "TypeScript");
        }
    }

    // --- Config mutation via state ---

    #[test]
    fn test_set_language_mutates_config() {
        let dir = temp_dir();
        let state = make_state(&dir);
        state.config.lock().unwrap().language = "en".to_string();
        assert_eq!(state.config.lock().unwrap().language, "en");
    }

    #[test]
    fn test_set_cleanup_style_mutates_config() {
        let dir = temp_dir();
        let state = make_state(&dir);
        state.config.lock().unwrap().cleanup_style = CleanupStyle::Chat;
        assert_eq!(state.config.lock().unwrap().cleanup_style, CleanupStyle::Chat);
    }

    // --- DEFAULT_HOTKEY constant ---

    #[test]
    fn test_default_hotkey_is_valid_string() {
        assert!(!DEFAULT_HOTKEY.is_empty());
        assert_eq!(DEFAULT_HOTKEY, "ctrl+shift+d");
    }
}
