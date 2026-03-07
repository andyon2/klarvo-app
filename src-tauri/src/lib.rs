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
mod history;
mod paste;
mod stt;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use audio::AudioRecorder;
use config::{load_config, save_config, AppConfig, HotkeyMode};
use dictionary::{load_dictionary, save_dictionary, Dictionary};
use hotkey::{PipelineEvent, EVENT_STATE_CHANGED};
use history::UsageSummary;
use llm::{
    chunked_cleanup, AnthropicCleanup, CleanupProvider, CleanupStyle, DeepSeekCleanup,
    GroqCleanup, OpenAiCleanup,
};
use paste::{capture_foreground_window, capture_foreground_window_title, create_paste_handler};
use serde::{Deserialize, Serialize};
use stt::{build_stt_prompt, GroqWhisper, OpenAiWhisper, SttProvider};
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
    /// Groq Whisper model variant.
    pub stt_model: String,
    /// Custom prompt for the LLM.
    pub custom_prompt: String,
    /// Launch on login.
    pub autostart: bool,
    /// Whisper mode (amplified mic for quiet speech).
    pub whisper_mode: bool,
    /// Masked OpenAI API key.
    pub openai_api_key_masked: String,
    /// Masked Anthropic API key.
    pub anthropic_api_key_masked: String,
    /// Ordered list of STT provider IDs (first with a key wins).
    pub stt_priority: Vec<String>,
    /// Ordered list of LLM provider IDs (first with a key wins).
    pub llm_priority: Vec<String>,
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
    /// SQLite connection for dictation history.
    history_db: Mutex<rusqlite::Connection>,
    /// Window title of the app that was focused when recording started.
    /// Used for app-profile matching.
    prev_window_title: Mutex<Option<String>>,
    /// Whether the current recording is a Command Mode session.
    /// When true, the pipeline will rewrite selected text instead of dictating.
    command_mode_active: Mutex<bool>,
    /// The text that was selected when Command Mode was triggered (via Ctrl+C).
    command_mode_selected_text: Mutex<Option<String>>,
}

// SAFETY: All fields are either `Arc<_>`, `Mutex<_>`, or `RwLock<_>`, which
// are `Send + Sync` when their inner types are `Send`.
// `AudioRecorder` carries its own `unsafe impl Send + Sync` in audio/mod.rs.
// The trait objects (`Arc<dyn SttProvider>` etc.) require `Send + Sync` bounds
// on the traits (both traits have those bounds).
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl AppState {
    fn new(cfg: AppConfig, dictionary: Dictionary, app_data_dir: PathBuf, history_db: rusqlite::Connection) -> Self {
        let stt = resolve_stt_provider(&cfg);
        let cleanup = resolve_cleanup_provider(&cfg);
        AppState {
            recorder: Arc::new(AudioRecorder::new()),
            stt_provider: RwLock::new(stt),
            cleanup_provider: RwLock::new(cleanup),
            recording_start: Mutex::new(None),
            last_recording: Mutex::new(None),
            config: Mutex::new(cfg),
            dictionary: Mutex::new(dictionary),
            app_data_dir,
            prev_foreground_hwnd: Mutex::new(None),
            history_db: Mutex::new(history_db),
            prev_window_title: Mutex::new(None),
            command_mode_active: Mutex::new(false),
            command_mode_selected_text: Mutex::new(None),
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

/// Wraps an error message with a human-readable hint based on common HTTP/network
/// error patterns. The hint is appended after the raw error string.
///
/// Keeps error messages actionable for non-technical users without losing the
/// original diagnostic information.
fn friendly_error(context: &str, err: &str) -> String {
    let hint = if err.contains("401") || err.contains("Unauthorized") || err.contains("invalid_api_key") {
        " Check your API key in Settings."
    } else if err.contains("429") || err.contains("rate_limit") {
        " Rate limit reached \u{2014} wait a moment and try again."
    } else if err.contains("timeout") || err.contains("timed out") {
        " Request timed out \u{2014} check your internet connection."
    } else if err.contains("connection") || err.contains("ConnectError") {
        " No internet connection."
    } else {
        ""
    };
    format!("{context}: {err}{hint}")
}

// ---------------------------------------------------------------------------
// Hotkey dictation pipeline
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Provider resolution from priority lists
// ---------------------------------------------------------------------------

fn resolve_stt_provider(cfg: &AppConfig) -> Arc<dyn SttProvider> {
    for id in &cfg.stt_priority {
        match id.as_str() {
            "groq" if !cfg.groq_api_key.is_empty() => {
                return Arc::new(GroqWhisper::new(&cfg.groq_api_key).with_model(cfg.stt_model.clone()));
            }
            "openai" if !cfg.openai_api_key.is_empty() => {
                return Arc::new(OpenAiWhisper::new(&cfg.openai_api_key));
            }
            _ => continue,
        }
    }
    Arc::new(GroqWhisper::new(&cfg.groq_api_key).with_model(cfg.stt_model.clone()))
}

fn resolve_cleanup_provider(cfg: &AppConfig) -> Arc<dyn CleanupProvider> {
    for id in &cfg.llm_priority {
        match id.as_str() {
            "deepseek" if !cfg.deepseek_api_key.is_empty() => {
                return Arc::new(DeepSeekCleanup::new(&cfg.deepseek_api_key));
            }
            "openai" if !cfg.openai_api_key.is_empty() => {
                return Arc::new(OpenAiCleanup::new(&cfg.openai_api_key));
            }
            "anthropic" if !cfg.anthropic_api_key.is_empty() => {
                return Arc::new(AnthropicCleanup::new(&cfg.anthropic_api_key));
            }
            "groq" if !cfg.groq_api_key.is_empty() => {
                return Arc::new(GroqCleanup::new(&cfg.groq_api_key));
            }
            _ => continue,
        }
    }
    Arc::new(DeepSeekCleanup::new(&cfg.deepseek_api_key))
}

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
    if let Ok(mut guard) = state.prev_window_title.lock() {
        *guard = capture_foreground_window_title();
        log::debug!("[hotkey] foreground window title: {:?}", *guard);
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

/// Starts Command Mode: copies selected text via Ctrl+C, then starts recording.
///
/// The voice command will be transcribed and used to rewrite the selected text.
async fn start_command_mode(handle: AppHandle) {
    let state = handle.state::<AppState>();

    if state.recorder.is_recording() {
        return;
    }

    // Capture foreground window
    if let Ok(mut guard) = state.prev_foreground_hwnd.lock() {
        *guard = capture_foreground_window();
    }

    // Copy selected text via clipboard
    #[cfg(target_os = "windows")]
    {
        // Simulate Ctrl+C to copy selected text
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
            KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_C,
        };

        unsafe {
            let inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_CONTROL,
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(VK_C.0),
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(VK_C.0),
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_CONTROL,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ];
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }

        // Wait for clipboard to populate
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Read clipboard
    let selected_text = arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok())
        .unwrap_or_default();

    log::info!("[command-mode] selected text: {:?}", &selected_text[..selected_text.len().min(100)]);

    if let Ok(mut guard) = state.command_mode_selected_text.lock() {
        *guard = if selected_text.is_empty() { None } else { Some(selected_text) };
    }
    if let Ok(mut guard) = state.command_mode_active.lock() {
        *guard = true;
    }

    // Start recording the voice command
    setup_audio_level_emitter(&handle);

    let device_name = state.config.lock().ok().and_then(|c| c.audio_device.clone());
    if let Err(e) = state.recorder.start_recording(device_name.as_deref()) {
        let _ = handle.emit(
            EVENT_STATE_CHANGED,
            PipelineEvent::error(format!("Failed to start recording: {e}")),
        );
        if let Ok(mut guard) = state.command_mode_active.lock() {
            *guard = false;
        }
        return;
    }

    *match state.recording_start.lock() {
        Ok(g) => g,
        Err(_) => {
            let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::error("State lock poisoned"));
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

    let whisper_mode = state.config.lock().ok().map(|c| c.whisper_mode).unwrap_or(false);
    let gain = if whisper_mode { 3.0 } else { 1.0 };

    let wav_bytes = match state.recorder.stop_recording_with_gain(gain) {
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
    // Whisper mode uses a lower threshold since the audio has been amplified.
    let silence_threshold = if whisper_mode { 0.001 } else { 0.005 };
    if let Some(rms) = compute_wav_rms(&wav_bytes) {
        log::debug!("[pipeline] audio RMS = {rms:.5} (threshold={silence_threshold})");
        if rms < silence_threshold {
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

        let dict_terms = match state.dictionary.lock() {
            Ok(g) => {
                let p = g.terms_as_prompt();
                if p.is_empty() { None } else { Some(p) }
            }
            Err(_) => None,
        };

        let prompt = build_stt_prompt(dict_terms.as_deref(), &cfg.language);

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
                PipelineEvent::error(friendly_error("Transcription failed", &e.to_string())),
            );
            return;
        }
    };

    log::debug!("[pipeline] raw transcription: {raw_text:?}");

    // --- Check Command Mode ---
    let is_command_mode = state.command_mode_active.lock().ok().map(|g| *g).unwrap_or(false);
    let selected_text = if is_command_mode {
        // Reset command mode flags
        if let Ok(mut guard) = state.command_mode_active.lock() { *guard = false; }
        state.command_mode_selected_text.lock().ok().and_then(|mut g| g.take())
    } else {
        None
    };

    // --- LLM step ---
    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::cleaning());

    let cleanup_result = if let Some(ref sel_text) = selected_text {
        // Command Mode: rewrite selected text using the voice command
        log::info!("[pipeline] command mode: rewriting with voice command");

        match cleanup_provider.rewrite(sel_text, &raw_text).await {
            Ok(r) => r,
            Err(e) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error(format!("Command mode failed: {e}")),
                );
                return;
            }
        }
    } else {
        // Normal dictation: cleanup raw transcription
        let (style, custom_prompt) = {
            match state.config.lock() {
                Ok(g) => {
                    let prev_title = state.prev_window_title.lock().ok().and_then(|t| t.clone());
                    let matched = prev_title.as_deref().and_then(|title| {
                        let title_lower = title.to_lowercase();
                        g.profiles.iter().find(|p| {
                            !p.app_pattern.is_empty() && title_lower.contains(&p.app_pattern.to_lowercase())
                        })
                    });
                    if let Some(profile) = matched {
                        log::info!("[pipeline] profile matched: {:?}", profile.name);
                        let prompt = if profile.custom_prompt.is_empty() {
                            let p = g.custom_prompt.clone();
                            if p.is_empty() { None } else { Some(p) }
                        } else {
                            Some(profile.custom_prompt.clone())
                        };
                        (profile.cleanup_style, prompt)
                    } else {
                        (g.cleanup_style, {
                            let p = g.custom_prompt.clone();
                            if p.is_empty() { None } else { Some(p) }
                        })
                    }
                }
                Err(_) => (CleanupStyle::Polished, None),
            }
        };

        let dict_list = match state.dictionary.lock() {
            Ok(g) => {
                let l = g.terms_as_list();
                if l.is_empty() { None } else { Some(l) }
            }
            Err(_) => None,
        };

        match chunked_cleanup(
            cleanup_provider.as_ref(), &raw_text, style,
            dict_list.as_deref(), custom_prompt.as_deref(),
        ).await {
            Ok(r) => r,
            Err(e) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error(friendly_error("Text cleanup failed", &e.to_string())),
                );
                return;
            }
        }
    };

    let is_command = selected_text.is_some();
    let cleaned_text = cleanup_result.text;
    log::debug!("[pipeline] cleaned text: {cleaned_text:?}");

    // --- Record usage ---
    if let Ok(db) = state.history_db.lock() {
        // STT cost per audio hour depends on the model
        let stt_rate = match state.config.lock().ok().as_ref().map(|c| c.stt_model.as_str()) {
            Some("whisper-large-v3") => 0.111,
            Some("distil-whisper-large-v3-en") => 0.02,
            _ => 0.04, // whisper-large-v3-turbo (default)
        };
        let stt_cost = duration_ms as f64 / 3_600_000.0 * stt_rate;
        if let Err(e) = history::record_usage(&db, "groq_stt", Some(duration_ms as i64), None, None, stt_cost) {
            log::warn!("[pipeline] Failed to record STT usage: {e}");
        }
        // LLM cost: DeepSeek input=$0.27/1M, output=$1.10/1M tokens
        let llm_cost = (cleanup_result.prompt_tokens.unwrap_or(0) as f64 * 0.27
            + cleanup_result.completion_tokens.unwrap_or(0) as f64 * 1.10)
            / 1_000_000.0;
        if let Err(e) = history::record_usage(
            &db, "deepseek_cleanup", None,
            cleanup_result.prompt_tokens, cleanup_result.completion_tokens, llm_cost,
        ) {
            log::warn!("[pipeline] Failed to record LLM usage: {e}");
        }
    }

    // --- Paste ---
    let prev_hwnd = state.prev_foreground_hwnd.lock().ok().and_then(|g| *g);
    let paste_handler = create_paste_handler(prev_hwnd);
    if let Err(e) = paste_handler.paste(&cleaned_text) {
        log::warn!("[pipeline] paste failed: {e}. Text is still available.");
    }

    // --- Save to history ---
    {
        let style_str = if is_command { "command".to_string() } else {
            state.config.lock().ok()
                .map(|c| serde_json::to_string(&c.cleanup_style).unwrap_or_default().replace('"', ""))
                .unwrap_or_else(|| "polished".to_string())
        };
        if let Ok(db) = state.history_db.lock() {
            if let Err(e) = history::add_entry(&db, &cleaned_text, Some(&raw_text), &style_str, &language) {
                log::warn!("[pipeline] Failed to save to history: {e}");
            }
        }
    }

    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::done(cleaned_text, raw_text));
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

    // --- Dictation hotkey ---
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
                _ => {}
            }
        })
        .map_err(|e| format!("Failed to register shortcut: {e}"))?;

    // --- Command Mode hotkey ---
    let cmd_shortcut_str = handle
        .state::<AppState>()
        .config
        .lock()
        .ok()
        .map(|c| c.command_hotkey.clone())
        .unwrap_or_else(|| "ctrl+shift+e".to_string());

    if let Ok(cmd_shortcut) = cmd_shortcut_str.parse::<Shortcut>() {
        let handle_clone2 = handle.clone();
        let _ = handle
            .global_shortcut()
            .on_shortcut(cmd_shortcut, move |_app, _shortcut, event| {
                let h = handle_clone2.clone();
                match event.state {
                    ShortcutState::Pressed => {
                        tauri::async_runtime::spawn(async move {
                            start_command_mode(h).await;
                        });
                    }
                    ShortcutState::Released => {
                        tauri::async_runtime::spawn(async move {
                            stop_and_process_pipeline(h).await;
                        });
                    }
                }
            });
    }

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
        let terms = guard.terms_as_prompt();
        let terms_opt = if terms.is_empty() { None } else { Some(terms) };
        build_stt_prompt(terms_opt.as_deref(), &language)
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

    let custom_prompt = match state.inner().config.lock() {
        Ok(g) => {
            let p = g.custom_prompt.clone();
            if p.is_empty() { None } else { Some(p) }
        }
        Err(_) => None,
    };

    chunked_cleanup(provider.as_ref(), &raw_text, style, terms.as_deref(), custom_prompt.as_deref())
        .await
        .map(|r| r.text)
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
    stt_model: Option<String>,
    custom_prompt: Option<String>,
    autostart: Option<bool>,
    whisper_mode: Option<bool>,
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    stt_priority: Option<Vec<String>>,
    llm_priority: Option<Vec<String>>,
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
        stt_model: stt_model.unwrap_or(existing.stt_model),
        custom_prompt: custom_prompt.unwrap_or(existing.custom_prompt),
        profiles: existing.profiles,
        autostart: autostart.unwrap_or(existing.autostart),
        whisper_mode: whisper_mode.unwrap_or(existing.whisper_mode),
        command_hotkey: existing.command_hotkey,
        openai_api_key: match openai_api_key {
            Some(ref k) if !k.is_empty() => k.clone(),
            _ => existing.openai_api_key,
        },
        anthropic_api_key: match anthropic_api_key {
            Some(ref k) if !k.is_empty() => k.clone(),
            _ => existing.anthropic_api_key,
        },
        stt_priority: stt_priority.unwrap_or(existing.stt_priority),
        llm_priority: llm_priority.unwrap_or(existing.llm_priority),
    };

    // Resolve providers from the new config before persisting.
    let new_stt = resolve_stt_provider(&new_cfg);
    let new_cleanup = resolve_cleanup_provider(&new_cfg);

    // Persist to disk.
    save_config(&inner.app_data_dir, &new_cfg)
        .map_err(|e| format!("Failed to save settings: {e}"))?;

    // Update in-memory config.
    *lock!(inner.config)? = new_cfg;

    // Hot-reload providers based on priority lists.
    *write_lock!(inner.stt_provider)? = new_stt;
    *write_lock!(inner.cleanup_provider)? = new_cleanup;

    // Re-register the global shortcut with the (possibly new) hotkey + mode.
    register_hotkey(&handle, parsed_shortcut, hotkey_mode)?;

    // Apply autostart: write or remove the OS startup entry.
    let autostart_enabled = lock!(inner.config)?.autostart;
    apply_autostart(autostart_enabled);

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
        stt_model: cfg.stt_model,
        custom_prompt: cfg.custom_prompt,
        autostart: cfg.autostart,
        whisper_mode: cfg.whisper_mode,
        openai_api_key_masked: mask_api_key(&cfg.openai_api_key),
        anthropic_api_key_masked: mask_api_key(&cfg.anthropic_api_key),
        stt_priority: cfg.stt_priority,
        llm_priority: cfg.llm_priority,
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
        let model = lock!(inner.config)?.stt_model.clone();
        *write_lock!(inner.stt_provider)? = Arc::new(GroqWhisper::new(key).with_model(model));
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
// Tauri commands -- History
// ---------------------------------------------------------------------------

/// Returns the most recent history entries.
#[tauri::command]
fn get_history(state: State<'_, AppState>, limit: Option<u32>) -> Result<Vec<history::HistoryEntry>, String> {
    let db = lock!(state.inner().history_db)?;
    history::get_entries(&db, limit.unwrap_or(50))
        .map_err(|e| format!("Failed to load history: {e}"))
}

/// Searches history entries by text content.
#[tauri::command]
fn search_history(state: State<'_, AppState>, query: String, limit: Option<u32>) -> Result<Vec<history::HistoryEntry>, String> {
    let db = lock!(state.inner().history_db)?;
    history::search_entries(&db, &query, limit.unwrap_or(50))
        .map_err(|e| format!("Failed to search history: {e}"))
}

/// Deletes a single history entry.
#[tauri::command]
fn delete_history_entry(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let db = lock!(state.inner().history_db)?;
    history::delete_entry(&db, id)
        .map_err(|e| format!("Failed to delete history entry: {e}"))?;
    Ok(())
}

/// Deletes all history entries.
#[tauri::command]
fn clear_history(state: State<'_, AppState>) -> Result<u64, String> {
    let db = lock!(state.inner().history_db)?;
    history::clear_history(&db)
        .map_err(|e| format!("Failed to clear history: {e}"))
}

/// Saves a dictation result to history (used by the frontend manual flow).
#[tauri::command]
fn add_history_entry(
    state: State<'_, AppState>,
    text: String,
    raw_text: Option<String>,
    style: String,
    language: String,
) -> Result<i64, String> {
    let db = lock!(state.inner().history_db)?;
    history::add_entry(&db, &text, raw_text.as_deref(), &style, &language)
        .map_err(|e| format!("Failed to save history entry: {e}"))
}

/// Updates the floating bar window shape (circle when idle, pill when expanded).
/// Called by the frontend whenever the bar state changes.
#[tauri::command]
fn set_bar_shape(handle: AppHandle, shape: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use tauri::Manager;
        if let Some(bar) = handle.get_webview_window("bar") {
            let scale = bar.scale_factor().unwrap_or(1.0);
            if let Ok(hwnd) = bar.hwnd() {
                let h = hwnd.0 as isize;
                if shape == "circle" {
                    let s = (28.0 * scale) as i32;
                    set_window_region_ellipse(h, s, s);
                } else {
                    let w = (260.0 * scale) as i32;
                    let ht = (40.0 * scale) as i32;
                    set_window_region_pill(h, w, ht);
                }
            }
        }
    }
    let _ = (handle, shape); // suppress unused warnings on non-Windows
    Ok(())
}

/// Takes a snapshot of the current audio buffer and transcribes it for live preview.
/// Returns the partial transcription text, or empty string if nothing recorded yet.
#[tauri::command]
async fn transcribe_live_preview(state: State<'_, AppState>) -> Result<String, String> {
    let inner = state.inner();

    // Only preview while actually recording
    if !inner.recorder.is_recording() {
        return Ok(String::new());
    }

    let wav_bytes = match inner.recorder.snapshot_wav() {
        Some(b) if b.len() > 44 => b, // 44 = WAV header only (no audio data)
        _ => return Ok(String::new()),
    };

    let (language, stt_provider, dict_prompt) = {
        let cfg = lock!(inner.config)?;
        let lang = cfg.language.clone();
        let stt = inner.stt_provider.read()
            .map_err(|e| format!("Lock poisoned: {e}"))?
            .clone();
        let dict_terms = match inner.dictionary.lock() {
            Ok(g) => {
                let p = g.terms_as_prompt();
                if p.is_empty() { None } else { Some(p) }
            }
            Err(_) => None,
        };
        let prompt = build_stt_prompt(dict_terms.as_deref(), &lang);
        (lang, stt, prompt)
    };

    match stt_provider.transcribe(wav_bytes, &language, dict_prompt.as_deref()).await {
        Ok(text) => Ok(text),
        Err(e) => {
            log::warn!("[live-preview] transcription failed: {e}");
            Ok(String::new()) // Don't error out, just return empty
        }
    }
}

/// Returns aggregated usage statistics (cost tracker + dictation stats).
#[tauri::command]
fn get_usage_stats(state: State<'_, AppState>) -> Result<UsageSummary, String> {
    let db = lock!(state.inner().history_db)?;
    history::get_usage_summary(&db)
        .map_err(|e| format!("Failed to get usage stats: {e}"))
}

// ---------------------------------------------------------------------------
// Tauri commands -- Profiles
// ---------------------------------------------------------------------------

/// Returns the app-specific profiles list.
#[tauri::command]
fn get_profiles(state: State<'_, AppState>) -> Result<Vec<config::AppProfile>, String> {
    let cfg = lock!(state.inner().config)?;
    Ok(cfg.profiles.clone())
}

/// Replaces the full profiles list and persists to disk.
#[tauri::command]
fn save_profiles(
    state: State<'_, AppState>,
    profiles: Vec<config::AppProfile>,
) -> Result<(), String> {
    let inner = state.inner();
    let mut cfg = lock!(inner.config)?;
    cfg.profiles = profiles;
    let cfg_clone = cfg.clone();
    drop(cfg);
    save_config(&inner.app_data_dir, &cfg_clone)
        .map_err(|e| format!("Failed to persist profiles: {e}"))
}

// ---------------------------------------------------------------------------
// Tauri commands -- Misc / Onboarding
// ---------------------------------------------------------------------------

/// Returns `true` if no API keys have been configured yet.
///
/// Used by the frontend to decide whether to show the onboarding wizard on
/// startup. Treated as "first run" when all provider keys are empty.
#[tauri::command]
fn is_first_run(state: State<'_, AppState>) -> bool {
    let inner = state.inner();
    match inner.config.lock() {
        Ok(g) => {
            g.groq_api_key.is_empty()
                && g.deepseek_api_key.is_empty()
                && g.openai_api_key.is_empty()
                && g.anthropic_api_key.is_empty()
        }
        Err(_) => true,
    }
}

// ---------------------------------------------------------------------------
// Autostart helper (Windows only)
// ---------------------------------------------------------------------------

/// Writes or removes the autostart registry entry under
/// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
///
/// On non-Windows platforms this is a no-op (the config field is still
/// persisted, but OS-level startup is not wired up).
#[cfg(target_os = "windows")]
fn apply_autostart(enabled: bool) {
    use windows::Win32::System::Registry::{
        RegCreateKeyExW, RegDeleteValueW, RegSetValueExW, RegCloseKey,
        HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, REG_OPTION_NON_VOLATILE,
    };
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::core::PCWSTR;

    // Encode the registry key path as a null-terminated wide string.
    let key_path: Vec<u16> = "Software\\Microsoft\\Windows\\CurrentVersion\\Run\0"
        .encode_utf16()
        .collect();
    let value_name: Vec<u16> = "Dikta\0".encode_utf16().collect();

    unsafe {
        let mut hkey = windows::Win32::System::Registry::HKEY::default();
        let result = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_path.as_ptr()),
            Some(0),
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut hkey,
            None,
        );

        if result != ERROR_SUCCESS {
            log::warn!("[autostart] Failed to open registry key: {:?}", result);
            return;
        }

        if enabled {
            // Determine path to the current executable.
            match std::env::current_exe() {
                Ok(exe_path) => {
                    let exe_str = exe_path.to_string_lossy();
                    // Quote the path in case it contains spaces.
                    let quoted = format!("\"{exe_str}\"\0");
                    let wide: Vec<u16> = quoted.encode_utf16().collect();
                    let byte_len = (wide.len() * 2) as u32;
                    let bytes = std::slice::from_raw_parts(wide.as_ptr() as *const u8, byte_len as usize);

                    let set_result = RegSetValueExW(
                        hkey,
                        PCWSTR(value_name.as_ptr()),
                        Some(0),
                        REG_SZ,
                        Some(bytes),
                    );
                    if set_result != ERROR_SUCCESS {
                        log::warn!("[autostart] Failed to write registry value: {:?}", set_result);
                    } else {
                        log::info!("[autostart] Autostart enabled: {exe_str}");
                    }
                }
                Err(e) => {
                    log::warn!("[autostart] Could not determine exe path: {e}");
                }
            }
        } else {
            // Delete the value (ignore error if it doesn't exist).
            let _ = RegDeleteValueW(hkey, PCWSTR(value_name.as_ptr()));
            log::info!("[autostart] Autostart disabled (registry entry removed)");
        }

        let _ = RegCloseKey(hkey);
    }
}

/// No-op stub for non-Windows platforms.
#[cfg(not(target_os = "windows"))]
fn apply_autostart(_enabled: bool) {}

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
    // Start as a tiny circle -- the frontend resizes dynamically based on state.
    let bar_width = 28.0_f64;
    let bar_height = 28.0_f64;

    let mut builder = tauri::WebviewWindowBuilder::new(
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

    // Remove window shadow so only the CSS-rendered content is visible.
    #[cfg(target_os = "windows")]
    {
        builder = builder.shadow(false);
    }

    let bar = builder.build()?;

    // Set initial elliptic (circle) window region on Windows.
    // This clips the window at the OS level so WebView2 artifacts are invisible.
    #[cfg(target_os = "windows")]
    {
        if let Ok(hwnd) = bar.hwnd() {
            let scale = bar.scale_factor().unwrap_or(1.0);
            let pw = (bar_width * scale) as i32;
            let ph = (bar_height * scale) as i32;
            set_window_region_ellipse(hwnd.0 as isize, pw, ph);
        }
    }

    // Position at bottom-center of the current monitor, above the taskbar.
    match bar.current_monitor() {
        Ok(Some(monitor)) => {
            let screen_size = monitor.size();
            let monitor_pos = monitor.position();
            let scale = monitor.scale_factor();
            let screen_w = screen_size.width as f64 / scale;
            let screen_h = screen_size.height as f64 / scale;
            let offset_x = monitor_pos.x as f64 / scale;
            let offset_y = monitor_pos.y as f64 / scale;
            let x = offset_x + (screen_w - bar_width) / 2.0;
            let y = offset_y + screen_h - bar_height - 52.0;
            log::info!(
                "[bar] screen={screen_w}x{screen_h} scale={scale} offset=({offset_x},{offset_y}), placing at ({x}, {y})"
            );
            let _ = bar.set_position(tauri::LogicalPosition::new(x, y));
        }
        _ => {
            log::warn!("[bar] No monitor detected, using fallback position");
            let _ = bar.set_position(tauri::LogicalPosition::new(400.0, 10.0));
        }
    }

    Ok(())
}

/// Sets the window region to an ellipse (circle when w==h) using Win32 API.
/// This clips the window shape at the OS level, hiding any WebView2 artifacts.
#[cfg(target_os = "windows")]
fn set_window_region_ellipse(hwnd: isize, width: i32, height: i32) {
    use windows::Win32::Graphics::Gdi::{CreateEllipticRgn, SetWindowRgn};
    use windows::Win32::Foundation::HWND;

    unsafe {
        let rgn = CreateEllipticRgn(0, 0, width, height);
        if !rgn.is_invalid() {
            let _ = SetWindowRgn(HWND(hwnd as *mut _), Some(rgn), true);
            // Note: after SetWindowRgn the system owns the region, do NOT delete it.
        }
    }
}

/// Sets the window region to a rounded rectangle using Win32 API.
#[cfg(target_os = "windows")]
fn set_window_region_pill(hwnd: isize, width: i32, height: i32) {
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};
    use windows::Win32::Foundation::HWND;

    unsafe {
        // Corner radius = height for a pill shape
        let rgn = CreateRoundRectRgn(0, 0, width, height, height, height);
        if !rgn.is_invalid() {
            let _ = SetWindowRgn(HWND(hwnd as *mut _), Some(rgn), true);
        }
    }
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

            // Open history database.
            let history_db = history::open_db(&app_data_dir)
                .expect("Failed to open history database");

            // Detect first run: no API keys configured means the user hasn't gone
            // through setup yet. We'll show the main window in this case even though
            // tauri.conf.json has visible:false (tray-first for returning users).
            let is_first_run = cfg.groq_api_key.is_empty()
                && cfg.deepseek_api_key.is_empty()
                && cfg.openai_api_key.is_empty()
                && cfg.anthropic_api_key.is_empty();

            // Apply autostart on launch: ensure registry entry matches config.
            apply_autostart(cfg.autostart);

            // Build and register the application state.
            let app_state = AppState::new(cfg, dictionary, app_data_dir, history_db);
            app.manage(app_state);

            // --- System tray (Windows only -- WSL2/Linux lacks proper tray support) ---
            #[cfg(target_os = "windows")]
            {
                let show_settings = MenuItem::with_id(app, "show_settings", "Settings", true, None::<&str>)?;
                let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
                let menu = Menu::with_items(app, &[&show_settings, &quit])?;

                let tray_tooltip = format!("Dikta \u{2014} {hotkey_str}");
                let _tray = tauri::tray::TrayIconBuilder::with_id("dikta-tray")
                    .icon(app.default_window_icon().unwrap().clone())
                    .tooltip(&tray_tooltip)
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

            // Show main window on first run so the onboarding wizard is visible.
            // On subsequent launches the window stays hidden (tray-first).
            if is_first_run {
                log::info!("[setup] First run detected -- showing main window for onboarding");
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
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
            // History
            get_history,
            search_history,
            delete_history_entry,
            clear_history,
            add_history_entry,
            // Stats
            get_usage_stats,
            // Profiles
            get_profiles,
            save_profiles,
            // Bar shape
            set_bar_shape,
            // Live preview
            transcribe_live_preview,
            // Onboarding
            is_first_run,
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
        let db = rusqlite::Connection::open_in_memory().unwrap();
        AppState::new(AppConfig::default(), Dictionary::new(), dir.path().to_path_buf(), db)
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
        assert!(cfg.language.is_empty());
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
            stt_model: "whisper-large-v3-turbo".to_string(),
            custom_prompt: String::new(),
            autostart: false,
            whisper_mode: false,
            openai_api_key_masked: String::new(),
            anthropic_api_key_masked: String::new(),
            stt_priority: vec!["groq".to_string(), "openai".to_string()],
            llm_priority: vec!["deepseek".to_string(), "openai".to_string()],
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
            stt_model: "whisper-large-v3-turbo".to_string(),
            custom_prompt: String::new(),
            autostart: false,
            whisper_mode: false,
            openai_api_key_masked: String::new(),
            anthropic_api_key_masked: String::new(),
            stt_priority: vec!["groq".to_string(), "openai".to_string()],
            llm_priority: vec!["deepseek".to_string(), "openai".to_string()],
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
            stt_model: "whisper-large-v3-turbo".to_string(),
            custom_prompt: String::new(),
            autostart: false,
            whisper_mode: false,
            openai_api_key_masked: String::new(),
            anthropic_api_key_masked: String::new(),
            stt_priority: vec!["groq".to_string(), "openai".to_string()],
            llm_priority: vec!["deepseek".to_string(), "openai".to_string()],
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
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let state = AppState::new(cfg, Dictionary::new(), dir.path().to_path_buf(), db);
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
