//! Dikta -- Tauri backend entry point.
//!
//! Wires together the audio, STT, LLM, paste, hotkey, config and dictionary
//! modules and exposes them to the React frontend via Tauri commands and events.
//!
//! ## Module layout
//!
//! ```text
//! lib.rs           -- AppState, run(), invoke_handler, shared helpers
//! pipeline.rs      -- hotkey pipeline: start_recording / stop_and_process
//! commands/
//!   recording.rs   -- start/stop/transcribe/cleanup Tauri commands
//!   settings.rs    -- save/get settings, API keys, hotkey, language
//!   dictionary.rs  -- get/add/remove dictionary terms
//!   history.rs     -- CRUD, search, notes, stats
//!   misc.rs        -- profiles, snippets, sync, paste, bar-shape
//! test_helpers.rs  -- shared helpers for unit tests
//! ```
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
mod commands;
mod config;
mod dictionary;
mod history;
mod hotkey;
mod license;
mod llm;
mod paste;
mod pipeline;
mod stt;
mod sync;

#[cfg(test)]
mod test_helpers;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use audio::AudioRecorder;
use config::{config_file_has_license_field, load_config, AppConfig, HotkeyMode};
use dictionary::{load_dictionary, Dictionary};
use license::{compute_status_from_cache, LicenseStatus, EARLY_ADOPTER_GRACE_SECS};
use llm::{CleanupProvider, CleanupStyle};
use serde::{Deserialize, Serialize};
use stt::SttProvider;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WindowEvent};

#[cfg(desktop)]
use tauri::menu::{Menu, MenuItem};
#[cfg(desktop)]
use tauri::tray::TrayIconEvent;
#[cfg(desktop)]
use tauri_plugin_global_shortcut::Shortcut;

// Re-export pipeline helpers so `commands/` modules can reach them.
pub use pipeline::{resolve_cleanup_provider, resolve_stt_provider};
#[cfg(desktop)]
pub use pipeline::register_hotkey;

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
    /// Output language for translation (empty = no translation).
    pub output_language: String,
    /// Webhook URL for HTTP POST after each dictation. Empty = disabled.
    pub webhook_url: String,
    /// Turso database URL (shown in full, not secret).
    pub turso_url: String,
    /// Masked Turso auth token.
    pub turso_token_masked: String,
    /// Device ID for sync.
    pub device_id: String,
    /// Android bubble size multiplier (0.5..2.0). Default: 1.0.
    pub bubble_size: f32,
    /// Android bubble opacity (0.3..1.0). Default: 0.85.
    pub bubble_opacity: f32,
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
    pub recorder: Arc<AudioRecorder>,
    /// Wrapping in `RwLock` lets `save_settings` replace the provider.
    pub stt_provider: RwLock<Arc<dyn SttProvider>>,
    pub cleanup_provider: RwLock<Arc<dyn CleanupProvider>>,
    /// Timestamp set by `start_recording`, cleared by `stop_recording`.
    pub recording_start: Mutex<Option<std::time::Instant>>,
    /// WAV bytes from the most recent recording. Set by `stop_recording`,
    /// consumed (read, not cleared) by `transcribe_audio`.
    pub last_recording: Mutex<Option<Vec<u8>>>,
    /// Full persisted configuration (includes API keys).
    pub config: Mutex<AppConfig>,
    /// User's custom word list -- injected into STT prompt and LLM system prompt.
    pub dictionary: Mutex<Dictionary>,
    /// Path to the app-data directory for persisting config and dictionary.
    pub app_data_dir: PathBuf,
    /// Window handle (HWND) of the app that was focused when recording started.
    /// Used on Windows to restore focus before pasting.
    pub prev_foreground_hwnd: Mutex<Option<isize>>,
    /// SQLite connection for dictation history.
    pub history_db: Mutex<rusqlite::Connection>,
    /// Window title of the app that was focused when recording started.
    /// Used for app-profile matching.
    pub prev_window_title: Mutex<Option<String>>,
    /// Whether the current recording is a Command Mode session.
    /// When true, the pipeline will rewrite selected text instead of dictating.
    pub command_mode_active: Mutex<bool>,
    /// The text that was selected when Command Mode was triggered (via Ctrl+C).
    pub command_mode_selected_text: Mutex<Option<String>>,
    /// Current license status, computed from config on startup and updated
    /// when the user validates or removes a key.
    pub license_status: Mutex<license::LicenseStatus>,
}

// SAFETY: All fields are either `Arc<_>`, `Mutex<_>`, or `RwLock<_>`, which
// are `Send + Sync` when their inner types are `Send`.
// `AudioRecorder` carries its own `unsafe impl Send + Sync` in audio/mod.rs.
// The trait objects (`Arc<dyn SttProvider>` etc.) require `Send + Sync` bounds
// on the traits (both traits have those bounds).
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl AppState {
    pub fn new(
        cfg: AppConfig,
        dictionary: Dictionary,
        app_data_dir: PathBuf,
        history_db: rusqlite::Connection,
        is_early_adopter: bool,
    ) -> Self {
        let stt = resolve_stt_provider(&cfg);
        let cleanup = resolve_cleanup_provider(&cfg);

        // Compute the initial license status from the cached key + timestamp.
        let initial_license_status = if is_early_adopter {
            // Existing user who predates the license system: grant 60-day grace period.
            let until = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                + EARLY_ADOPTER_GRACE_SECS;
            log::info!("[license] Early-adopter migration: 60-day grace period until {until}");
            LicenseStatus::GracePeriod { until }
        } else {
            compute_status_from_cache(&cfg.license_key, cfg.license_validated_at)
        };

        log::info!("[license] Initial status: {initial_license_status:?}");

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
            license_status: Mutex::new(initial_license_status),
        }
    }
}

// ---------------------------------------------------------------------------
// Lock/RwLock macros with descriptive error strings
// ---------------------------------------------------------------------------

/// Acquires a `Mutex` lock and converts a poisoned-lock panic into a
/// `Result<_, String>` (Tauri command convention).
#[macro_export]
macro_rules! lock {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|_| "Internal state lock poisoned".to_string())
    };
}

/// Acquires a `RwLock` read guard.
#[macro_export]
macro_rules! read_lock {
    ($rwlock:expr) => {
        $rwlock
            .read()
            .map_err(|_| "Internal state lock poisoned".to_string())
    };
}

/// Acquires a `RwLock` write guard.
#[macro_export]
macro_rules! write_lock {
    ($rwlock:expr) => {
        $rwlock
            .write()
            .map_err(|_| "Internal state lock poisoned".to_string())
    };
}

/// Guards a Tauri command behind a paid feature check.
///
/// Acquires the `license_status` lock from `$state` and calls
/// `license::is_feature_allowed`. If the feature is not allowed, the
/// enclosing function returns an `Err` with a machine-readable error string
/// that the frontend can parse to show the upgrade prompt.
///
/// Usage inside a `#[tauri::command]` that returns `Result<_, String>`:
/// ```ignore
/// require_license!(state, LicensedFeature::Sync);
/// ```
#[macro_export]
macro_rules! require_license {
    ($state:expr, $feature:expr) => {{
        let status = $state
            .license_status
            .lock()
            .map_err(|_| "license lock error".to_string())?;
        if !crate::license::is_feature_allowed(&status, $feature) {
            return Err(format!("feature_requires_license:{:?}", $feature));
        }
    }};
}

// ---------------------------------------------------------------------------
// Shared helper functions
// ---------------------------------------------------------------------------

/// Masks an API key for safe display in the frontend.
///
/// Returns an empty string if the key is empty.
/// Returns `"****{last4}"` for keys longer than 4 characters.
/// Returns `"****"` for keys with 4 or fewer characters (avoids leaking short keys).
pub fn mask_api_key(key: &str) -> String {
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
pub fn friendly_error(context: &str, err: &str) -> String {
    let hint = if err.contains("401")
        || err.contains("Unauthorized")
        || err.contains("invalid_api_key")
    {
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
// Default hotkey string
// ---------------------------------------------------------------------------

const DEFAULT_HOTKEY: &str = "ctrl+shift+d";

// ---------------------------------------------------------------------------
// Desktop-only window helpers
// ---------------------------------------------------------------------------

/// Event name for real-time audio level updates sent to the floating bar.
const EVENT_AUDIO_LEVEL: &str = "dikta://audio-level";

/// Sets the window region to an ellipse (circle when w==h) using Win32 API.
/// This clips the window shape at the OS level, hiding any WebView2 artifacts.
#[cfg(target_os = "windows")]
pub fn set_window_region_ellipse(hwnd: isize, width: i32, height: i32) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{CreateEllipticRgn, SetWindowRgn};

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
pub fn set_window_region_pill(hwnd: isize, width: i32, height: i32) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};

    unsafe {
        // Corner radius = height for a pill shape
        let rgn = CreateRoundRectRgn(0, 0, width, height, height, height);
        if !rgn.is_invalid() {
            let _ = SetWindowRgn(HWND(hwnd as *mut _), Some(rgn), true);
        }
    }
}

#[cfg(desktop)]
/// Sets up the audio-level callback that emits events to the frontend.
pub fn setup_audio_level_emitter(handle: &AppHandle) {
    let state = handle.state::<AppState>();
    let handle_clone = handle.clone();
    state.recorder.set_level_callback(Box::new(move |level| {
        let _ = handle_clone.emit(EVENT_AUDIO_LEVEL, serde_json::json!({ "level": level }));
    }));
}

#[cfg(desktop)]
/// Creates the floating bar window positioned above the taskbar.
fn create_bar_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // Start as a thin idle pill -- the frontend resizes dynamically based on state.
    let bar_width = 80.0_f64;
    let bar_height = 10.0_f64;

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

    // Set initial pill-shaped window region on Windows.
    #[cfg(target_os = "windows")]
    {
        if let Ok(hwnd) = bar.hwnd() {
            let scale = bar.scale_factor().unwrap_or(1.0);
            let pw = (bar_width * scale) as i32;
            let ph = (bar_height * scale) as i32;
            set_window_region_pill(hwnd.0 as isize, pw, ph);
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

// ---------------------------------------------------------------------------
// Tauri entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_updater::Builder::new().build());
    }

    let mut builder = builder.setup(|app| {
        // Resolve the app-data directory (e.g. %APPDATA%\com.dikta.voice on Windows).
        let app_data_dir = app
            .path()
            .app_data_dir()
            .expect("Tauri must provide an app-data directory");

        // Create the directory if it doesn't exist yet.
        std::fs::create_dir_all(&app_data_dir)?;

        // Check for early-adopter migration BEFORE loading config (we need to
        // know whether the license_key field was absent in the on-disk file).
        let is_early_adopter = !config_file_has_license_field(&app_data_dir);

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
        log::info!("[setup] Loaded dictionary: {} terms", dictionary.len());

        // Open history database.
        let history_db = history::open_db(&app_data_dir)
            .expect("Failed to open history database");

        // Apply autostart on launch: ensure registry entry matches config.
        commands::settings::apply_autostart(cfg.autostart);

        // Build and register the application state.
        let app_state = AppState::new(cfg, dictionary, app_data_dir, history_db, is_early_adopter);
        app.manage(app_state);

        // --- System tray (Windows only -- WSL2/Linux lacks proper tray support) ---
        #[cfg(target_os = "windows")]
        {
            let show_settings =
                MenuItem::with_id(app, "show_settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_settings, &quit])?;

            let tray_tooltip = format!("Dikta \u{2014} {hotkey_str}");
            let _tray = tauri::tray::TrayIconBuilder::with_id("dikta-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .tooltip(&tray_tooltip)
                .menu(&menu)
                .on_menu_event(|app, event| match event.id.as_ref() {
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
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        ..
                    } = event
                    {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;
        }

        // --- Floating bar window ---
        #[cfg(target_os = "windows")]
        if let Err(e) = create_bar_window(app) {
            log::warn!("[setup] Could not create floating bar: {e}");
        }

        // --- Desktop-only setup: audio level emitter + global hotkey ---
        #[cfg(desktop)]
        {
            let handle = app.handle().clone();
            setup_audio_level_emitter(&handle);

            println!("[setup] Parsing hotkey: {hotkey_str:?}");
            let shortcut = hotkey_str.parse::<Shortcut>().unwrap_or_else(|e| {
                log::warn!(
                    "[hotkey] Saved hotkey {:?} is invalid ({e}), falling back to default",
                    hotkey_str
                );
                DEFAULT_HOTKEY
                    .parse::<Shortcut>()
                    .expect("DEFAULT_HOTKEY must be a valid shortcut string")
            });

            match register_hotkey(&handle, shortcut, hotkey_mode) {
                Ok(()) => log::info!(
                    "[hotkey] Registered shortcut: {hotkey_str} (mode={hotkey_mode:?})"
                ),
                Err(e) => log::warn!(
                    "[hotkey] Could not register shortcut: {e}. Use the UI button instead."
                ),
            }
        }

        // Always show the main window on launch (desktop only).
        #[cfg(desktop)]
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.set_focus();
        }

        Ok(())
    });

    // On Windows with a working system tray, we hide windows on close
    // instead of quitting. On other platforms, closing main = quit.
    #[cfg(desktop)]
    {
        builder = builder.on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let label = window.label();
                // Bar window: always prevent close (it should always exist).
                if label == "bar" {
                    let _ = window.hide();
                    api.prevent_close();
                }
                // Main window: hide only if tray is available (Windows).
                #[cfg(target_os = "windows")]
                if label == "main" {
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        });
    }

    builder
        .invoke_handler(tauri::generate_handler![
            // Recording
            commands::recording::start_recording,
            commands::recording::stop_recording,
            commands::recording::transcribe_audio,
            commands::recording::transcribe_audio_bytes,
            commands::recording::cleanup_text,
            commands::recording::is_recording,
            commands::recording::list_audio_devices,
            commands::recording::transcribe_live_preview,
            // Settings
            commands::settings::save_settings,
            commands::settings::get_settings,
            commands::settings::get_api_key_status,
            commands::settings::update_api_keys,
            commands::settings::set_language,
            commands::settings::set_cleanup_style,
            commands::settings::set_output_language,
            commands::settings::set_hotkey,
            commands::settings::reformat_text,
            commands::settings::is_first_run,
            commands::settings::get_active_app,
            commands::settings::get_advanced_settings,
            commands::settings::save_advanced_settings,
            // Dictionary
            commands::dictionary::get_dictionary_terms,
            commands::dictionary::add_dictionary_term,
            commands::dictionary::remove_dictionary_term,
            // History
            commands::history::get_history,
            commands::history::search_history,
            commands::history::delete_history_entry,
            commands::history::clear_history,
            commands::history::add_history_entry,
            commands::history::get_usage_stats,
            commands::history::get_filler_stats,
            commands::history::get_notes,
            commands::history::save_note,
            // Misc: profiles, snippets, sync, paste, UI helpers
            commands::misc::get_profiles,
            commands::misc::save_profiles,
            commands::misc::get_snippets,
            commands::misc::save_snippets,
            commands::misc::paste_snippet,
            commands::misc::sync_history,
            commands::recording::cancel_recording,
            commands::misc::set_bar_shape,
            // License
            commands::license::validate_license,
            commands::license::get_license_status,
            commands::license::remove_license,
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
    use crate::test_helpers::{make_state, temp_dir};

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
            output_language: String::new(),
            webhook_url: String::new(),
            turso_url: String::new(),
            turso_token_masked: String::new(),
            device_id: "test-device".to_string(),
            bubble_size: 1.0,
            bubble_opacity: 0.85,
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(json.contains("groqApiKeyMasked"), "expected camelCase key");
        assert!(json.contains("deepseekApiKeyMasked"), "expected camelCase key");
        assert!(json.contains("cleanupStyle"), "expected camelCase key");
        assert!(
            json.contains("hotkeyMode"),
            "expected camelCase 'hotkeyMode'"
        );
        assert!(json.contains("webhookUrl"), "expected camelCase 'webhookUrl'");
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
            output_language: String::new(),
            webhook_url: String::new(),
            turso_url: String::new(),
            turso_token_masked: String::new(),
            device_id: "test-device".to_string(),
            bubble_size: 1.0,
            bubble_opacity: 0.85,
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(
            json.contains(r#""hotkeyMode":"hold""#),
            "hold variant must serialize as lowercase 'hold'"
        );
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
            output_language: String::new(),
            webhook_url: String::new(),
            turso_url: String::new(),
            turso_token_masked: String::new(),
            device_id: "test-device".to_string(),
            bubble_size: 1.0,
            bubble_opacity: 0.85,
        };
        let json = serde_json::to_string(&view).unwrap();
        assert!(
            json.contains(r#""hotkeyMode":"toggle""#),
            "toggle variant must serialize as lowercase 'toggle'"
        );
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
        let db = rusqlite::Connection::open_in_memory()
            .expect("in-memory SQLite must always open successfully");
        let state = AppState::new(cfg, Dictionary::new(), dir.path().to_path_buf(), db, false);
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
