//! Klarvo -- Tauri backend entry point.
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
//! the full pipeline automatically and emits `klarvo://state-changed` events
//! so the frontend can update the UI without being in the loop.

mod audio;
mod commands;
mod config;
mod dictionary;
mod fs;
mod history;
mod hotkey;
mod license;
pub mod llm;
mod paste;
mod pipeline;
mod stt;
mod sync;

#[cfg(desktop)]
mod vad;

#[cfg(desktop)]
mod voice_command;

#[cfg(target_os = "windows")]
mod native_pill;

#[cfg(test)]
mod test_helpers;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::AtomicBool;

use audio::AudioRecorder;
use config::{load_config_reporting, save_config, AppConfig, HotkeyMode};
use dictionary::{load_dictionary, Dictionary};
use license::{compute_cached_status, LicenseStatus};
use llm::{CleanupProvider, CleanupStyle};
use serde::{Deserialize, Serialize};
use stt::SttProvider;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WindowEvent};
#[cfg(target_os = "windows")]
use tauri::menu::{Menu, MenuItem};
#[cfg(target_os = "windows")]
use tauri::tray::TrayIconEvent;


// Re-export pipeline helpers so `commands/` modules can reach them.
pub use pipeline::{resolve_cleanup_provider, resolve_providers, resolve_stt_provider};
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
    /// Masked OpenRouter API key.
    pub openrouter_api_key_masked: String,
    /// Selected STT provider: "groq", "openai", or "local".
    pub stt_provider: String,
    /// Selected LLM cleanup provider: "deepseek", "openai", "anthropic", or "groq".
    pub llm_provider: String,
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
    /// GGML model variant for offline STT (e.g. `"base"`, `"tiny-q5_1"`).
    pub local_whisper_model: String,
    /// Whether GPU acceleration (CUDA) is enabled for local whisper.
    pub local_whisper_gpu: bool,
    /// Send Enter after pasting for hotkey slot 0.
    /// Replaces the deprecated global `insert_and_send` field.
    pub insert_and_send_slot1: bool,
    /// Send Enter after pasting for hotkey slot 1.
    pub insert_and_send_slot2: bool,
    /// Silence duration (seconds) before AutoStop mode triggers stop + pipeline.
    pub autostop_silence_secs: f32,
    /// Silence duration (seconds) before Auto mode triggers stop + pipeline.
    pub auto_mode_silence_secs: f32,
    /// Hotkey string for the optional second slot (empty = slot disabled).
    pub hotkey_slot2: String,
    /// Recording mode for the optional second slot.
    pub hotkey_mode_slot2: HotkeyMode,
    /// Recording mode for the Android floating bubble.
    /// Valid values: `"hold"`, `"toggle"`, `"autostop"`, `"auto"`.
    pub bubble_recording_mode: String,
    /// Recording mode for bubble single-tap gesture.
    pub bubble_tap_mode: String,
    /// Auto-send after paste for bubble tap gesture.
    pub bubble_tap_auto_send: bool,
    /// Silence duration (seconds) for auto-stop on bubble tap.
    pub bubble_tap_silence_secs: f32,
    /// Recording mode for bubble long-press gesture.
    pub bubble_long_press_mode: String,
    /// Auto-send after paste for bubble long-press gesture.
    pub bubble_long_press_auto_send: bool,
    /// Silence duration (seconds) for auto-stop on bubble long press.
    pub bubble_long_press_silence_secs: f32,
    /// Whether Voice Command Mode is enabled (persisted user preference).
    pub voice_command_enabled: bool,
    /// Webhook URL for in-app feedback submissions (plain, not a secret).
    /// Empty string = feedback feature disabled.
    pub feedback_webhook_url: String,
    /// Whether live-preview is enabled (opt-in, default false).
    pub live_preview_enabled: bool,
    /// Silence duration (seconds) that triggers a preview flush in Toggle/Hold mode.
    pub preview_pause_silence_secs: f32,
    /// Display form preset for the live-preview panel ("compact" | "comfortable" | "wide").
    pub preview_panel_form: String,
    // Story 6.6 preview appearance fields.
    /// Preview text color: CSS color string.
    pub preview_text_color: String,
    /// Preview background color: CSS color string.
    pub preview_bg_color: String,
    /// Preview backdrop-blur radius in px.
    pub preview_bg_blur: u8,
    /// Preview border color: CSS color string.
    pub preview_border_color: String,
    /// Preview border thickness in px.
    pub preview_border_width: u8,
    /// Preview corner radius in px (must match set_preview_shape radius, R11).
    pub preview_border_radius: u8,
    /// Preview font family: CSS font-family string.
    pub preview_font_family: String,
    /// Preview font size: "small" | "medium" | "large".
    pub preview_font_size: String,
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
    /// Serializes the read-modify-write+persist cycle for config.json.
    /// All callers must hold this lock across the entire cycle (read → modify →
    /// clone → disk write) so no concurrent saver can clobber a prior write.
    /// The `config` Mutex is dropped before the disk write; this lock stays held.
    pub config_disk_write: Mutex<()>,
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
    /// When `true`, the global hotkey handler ignores all key events.
    /// Set by the frontend when the ShortcutRecorder is active, so the user
    /// can press the current hotkey without triggering the pipeline.
    pub hotkey_paused: AtomicBool,
    /// Controls the Auto-Loop recording mode.
    ///
    /// Set to `true` when Auto mode is activated (first hotkey press).
    /// Set to `false` when the user presses the hotkey again to stop the loop.
    /// The pipeline itself never touches this flag -- only the hotkey handler
    /// controls the loop lifecycle. `SeqCst` ordering ensures visibility across
    /// the cpal OS-thread and the Tauri async runtime.
    pub auto_loop_active: AtomicBool,
    /// The `insert_and_send` flag from the slot that triggered the current
    /// (or most recent) recording. Written by the hotkey handler when a slot
    /// starts recording; read by `stop_and_process_pipeline` so the pipeline
    /// knows whether to send Enter after pasting.
    ///
    /// Default: `false`. Reusing an `AtomicBool` keeps this lock-free.
    pub active_insert_and_send: AtomicBool,
    /// Whether the Voice Command Mode monitor is currently running.
    ///
    /// Set to `true` by `start_voice_command_monitor`, cleared by
    /// `stop_voice_command_monitor`. Used as a guard to prevent double-start.
    pub voice_command_active: AtomicBool,
    /// Diagnostic metrics accumulated during normal use.
    ///
    /// Written at the end of each successful pipeline run and on every
    /// STT / LLM / paste error. Read by `get_feedback_metrics` when the user
    /// opens the feedback dialog so the payload carries fresh telemetry.
    pub feedback_metrics: Mutex<commands::feedback::FeedbackMetrics>,
    /// Native pill overlay window handle (Windows-only).
    /// Replaces the WebView2 "bar" window (Story 10-1).
    #[cfg(target_os = "windows")]
    pub native_pill: Mutex<Option<native_pill::NativePill>>,
}

// SAFETY: All fields are either `Arc<_>`, `Mutex<_>`, or `RwLock<_>`, which
// are `Send + Sync` when their inner types are `Send`.
// `AudioRecorder` carries its own `unsafe impl Send + Sync` in audio/mod.rs.
// The trait objects (`Arc<dyn SttProvider>` etc.) require `Send + Sync` bounds
// on the traits (both traits have those bounds).
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl AppState {
    /// The single sanctioned runtime path to persist `config.json`.
    ///
    /// Serializes the whole read-modify-write+persist cycle behind `config_disk_write` so
    /// concurrent savers cannot clobber each other (ROB-04): acquires the disk-write lock,
    /// applies `mutate` to the in-memory config under the `config` lock, drops the `config`
    /// lock, writes the resulting snapshot to disk under the still-held disk-write lock, and
    /// returns that snapshot. The `config` lock is never held across disk I/O. Callers that
    /// need the persisted config (e.g. to re-resolve providers) use the returned value.
    ///
    /// Every production config save goes through this. `crate::config::save_config` is
    /// `pub(crate)` and reserved for single-threaded boot (before this `AppState` exists);
    /// calling it elsewhere bypasses the ROB-04 serialization invariant.
    pub fn save_config_locked(
        &self,
        context: &str,
        mutate: impl FnOnce(&mut AppConfig),
    ) -> Result<AppConfig, String> {
        let _disk_guard = crate::lock!(self.config_disk_write)?;
        let snapshot = {
            let mut cfg = crate::lock!(self.config)?;
            mutate(&mut cfg);
            cfg.clone()
        };
        crate::config::save_config(&self.app_data_dir, &snapshot)
            .map_err(|e| format!("Failed to persist {context}: {e}"))?;
        Ok(snapshot)
    }

    pub fn new(
        cfg: AppConfig,
        dictionary: Dictionary,
        app_data_dir: PathBuf,
        history_db: rusqlite::Connection,
    ) -> Self {
        let (stt, cleanup) = resolve_providers(&cfg, &app_data_dir);

        // Compute the initial license status from the cached key + timestamp.
        // Shared with the Android JNI license bridge via `compute_cached_status`
        // so the two platforms can never diverge (ADR-0016).
        let initial_license_status = compute_cached_status(
            &cfg.license_key,
            &cfg.license_source,
            &cfg.ls_instance_id,
            cfg.ls_last_validated_at,
            cfg.license_validated_at,
            cfg.first_install_at,
        );

        log::info!("[license] Initial status: {initial_license_status:?}");

        AppState {
            recorder: Arc::new(AudioRecorder::new()),
            stt_provider: RwLock::new(stt),
            cleanup_provider: RwLock::new(cleanup),
            recording_start: Mutex::new(None),
            last_recording: Mutex::new(None),
            config: Mutex::new(cfg),
            config_disk_write: Mutex::new(()),
            dictionary: Mutex::new(dictionary),
            app_data_dir,
            prev_foreground_hwnd: Mutex::new(None),
            history_db: Mutex::new(history_db),
            prev_window_title: Mutex::new(None),
            command_mode_active: Mutex::new(false),
            command_mode_selected_text: Mutex::new(None),
            license_status: Mutex::new(initial_license_status),
            hotkey_paused: AtomicBool::new(false),
            auto_loop_active: AtomicBool::new(false),
            active_insert_and_send: AtomicBool::new(false),
            voice_command_active: AtomicBool::new(false),
            feedback_metrics: Mutex::new(commands::feedback::FeedbackMetrics::default()),
            #[cfg(target_os = "windows")]
            native_pill: Mutex::new(None),
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
        if !$crate::license::is_feature_allowed(&status, $feature) {
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
    let key = key.trim();
    if key.is_empty() {
        return String::new();
    }
    if key.len() <= 4 {
        return "****".to_string();
    }
    format!("****{}", &key[key.len() - 4..])
}

/// Updates the system tray icon tooltip to reflect the current pipeline state.
///
/// Tooltip strings per state:
/// - idle / done  → "Klarvo"
/// - recording    → "Klarvo — Recording..."
/// - transcribing → "Klarvo — Transcribing..."
/// - cleaning     → "Klarvo — Processing..."
/// - error        → "Klarvo — Error"
///
/// If the tray icon cannot be found, the failure is logged and ignored -- the
/// app must not crash because the tray tooltip failed to update.
#[cfg(desktop)]
pub fn update_tray_tooltip(handle: &AppHandle, state: &hotkey::PipelineState) {
    

    let tooltip = match state {
        hotkey::PipelineState::Idle | hotkey::PipelineState::Done => "Klarvo \u{00b7} Early Access",
        hotkey::PipelineState::Recording => "Klarvo \u{00b7} Early Access \u{2014} Recording...",
        hotkey::PipelineState::Transcribing => "Klarvo \u{00b7} Early Access \u{2014} Transcribing...",
        hotkey::PipelineState::Cleaning => "Klarvo \u{00b7} Early Access \u{2014} Processing...",
        hotkey::PipelineState::Error => "Klarvo \u{00b7} Early Access \u{2014} Error",
        hotkey::PipelineState::Warning => "Klarvo \u{00b7} Early Access",
    };

    match handle.tray_by_id("klarvo-tray") {
        Some(tray) => {
            if let Err(e) = tray.set_tooltip(Some(tooltip)) {
                log::warn!("[tray] Failed to set tooltip to {tooltip:?}: {e}");
            }
        }
        None => {
            log::debug!("[tray] Tray icon 'klarvo-tray' not found, skipping tooltip update");
        }
    }
}

/// Emits a pipeline state-changed event and updates the tray tooltip
/// to match the new state. Also drives the native pill overlay (Windows).
/// This is the single call site for all state transitions so tray and
/// frontend stay in sync automatically.
pub fn emit_pipeline_state(handle: &AppHandle, event: hotkey::PipelineEvent) {
    let pipeline_state = event.state.clone();
    // Drive native pill in-process (no JS round-trip needed — AC-3/ADR-0021 §4).
    #[cfg(target_os = "windows")]
    {
        let clipboard_only = event.clipboard_only.unwrap_or(false);
        if let Ok(guard) = handle.state::<AppState>().native_pill.lock() {
            if let Some(pill) = guard.as_ref() {
                pill.set_state(&pipeline_state, clipboard_only);
            }
        }
    }
    let _ = handle.emit(hotkey::EVENT_STATE_CHANGED, event);
    #[cfg(desktop)]
    update_tray_tooltip(handle, &pipeline_state);
}

/// Wraps an error message with a human-readable hint based on common HTTP/network
/// error patterns. The hint is appended after the raw error string.
pub fn friendly_error(context: &str, err: &str) -> String {
    let hint = if err.contains("401")
        || err.contains("Unauthorized")
        || err.contains("invalid_api_key")
    {
        " Check your API key in Settings."
    } else if err.contains("429") || err.contains("rate_limit") || err.contains("Rate limit") {
        " Groq Free-Tier-Limit erreicht \u{2014} warte einen Moment und versuche es erneut."
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
const EVENT_AUDIO_LEVEL: &str = "klarvo://audio-level";

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

/// Sets the window region to a rounded rectangle with an EXPLICIT corner radius
/// (physical px; caller applies the scale factor). Used for the live-preview panel
/// (Story 5.2), which is a tall rounded *card* — not a pill/stadium — so its corner
/// radius must be independent of the window height. The radius passed here MUST equal
/// the CSS `borderRadius` the WebView renders for the expanded bar, or the gap between
/// the OS region and the CSS shape shows as the "white line" artifact.
#[cfg(target_os = "windows")]
pub fn set_window_region_round_rect(hwnd: isize, width: i32, height: i32, radius: i32) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{CreateRoundRectRgn, SetWindowRgn};

    unsafe {
        // CreateRoundRectRgn's corner ellipse = 2*radius → corner radius = `radius`.
        // Clamp so the ellipse never exceeds the rect dimensions.
        let ell_w = (radius * 2).min(width);
        let ell_h = (radius * 2).min(height);
        let rgn = CreateRoundRectRgn(0, 0, width, height, ell_w, ell_h);
        if !rgn.is_invalid() {
            let _ = SetWindowRgn(HWND(hwnd as *mut _), Some(rgn), true);
        }
    }
}

#[cfg(desktop)]
/// Sets up the audio-level callback that emits events to the frontend
/// AND feeds the native pill overlay in-process (Windows, AC-3).
pub fn setup_audio_level_emitter(handle: &AppHandle) {
    let state = handle.state::<AppState>();
    let handle_clone = handle.clone();
    state.recorder.set_level_callback(Box::new(move |level| {
        // Still emit for the preview window (remains WebView2 until Story 10-2).
        let _ = handle_clone.emit(EVENT_AUDIO_LEVEL, serde_json::json!({ "level": level }));
        // Feed native pill directly — no JS round-trip (AC-3 / ADR-0021 §4).
        #[cfg(target_os = "windows")]
        if let Ok(guard) = handle_clone.state::<AppState>().native_pill.lock() {
            if let Some(pill) = guard.as_ref() {
                pill.feed_rms(level);
            }
        }
    }));
}

/// WebView2 browser arguments. MUST stay identical across ALL THREE windows —
/// `main` (set in `tauri.conf.json`), plus `bar` and `preview` (set on their
/// builders below). All webviews in the process share ONE WebView2 environment;
/// any divergence in browser args between windows fails environment reuse and
/// wedges startup silently (verified the hard way 2026-06-20: args on `main`
/// only → bar/preview kept Tauri's default → app launched to a dead tray icon,
/// no Rust error). The flags disable renderer/occluded-window/timer backgrounding
/// so the hidden overlays keep painting under the post-June-2026 WebView2 runtime
/// (149.0.4022.69+), while re-including wry's default `--disable-features=...`
/// (a set value REPLACES the default rather than extending it).
///
/// `CalculateNativeWinOcclusion` is disabled (2026-06-21): the transparent,
/// always-on-top bar/preview overlays went BLANK whenever another window (e.g. a
/// foreground terminal) covered their screen region — window stayed visible +
/// on-screen + correct geometry (measured via dumpsys-equivalent EnumWindows), but
/// Chromium's native occlusion detection marked the webview occluded and stopped
/// compositing it. `--disable-backgrounding-occluded-windows` alone does NOT cover
/// this: it stops the *backgrounding*, not the occlusion *detection* that gates the
/// paint. Disabling occlusion calculation makes the overlays composite regardless of
/// what is in front. (Repro: record with a maximized terminal foreground -> blank;
/// bring Klarvo's own window forward -> paints.)
#[cfg(desktop)]
const WEBVIEW2_BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,CalculateNativeWinOcclusion --disable-renderer-backgrounding --disable-backgrounding-occluded-windows --disable-background-timer-throttling";

#[cfg(desktop)]
/// Creates the floating bar window positioned above the taskbar.
///
/// `saved_x` / `saved_y`: persisted logical position from the config. When
/// present they are used as-is; the work-area calculation is skipped. On
/// first launch (both `None`) the position is derived from the Win32
/// `SPI_GETWORKAREA` work area on Windows, or from the monitor bounds on
/// other platforms.
///
/// Accepts any `Manager<R>` implementor (`App`, `AppHandle`, etc.) so this
/// function can be called from both the `setup` closure and from commands
/// that only have access to an `AppHandle`.
pub fn create_bar_window<M: tauri::Manager<tauri::Wry>>(
    app: &M,
    saved_x: Option<f64>,
    saved_y: Option<f64>,
) -> Result<(), Box<dyn std::error::Error>> {
    // The window is created at full pill size (200×36 logical px). The pill region
    // is set once here — the frontend never calls setSize or setBarShape.
    // Position calculations use bar_height directly (= pill height = 36).
    let bar_width = 200.0_f64;  // PILL_WIDTH
    let bar_height = 36.0_f64;  // PILL_HEIGHT

    #[allow(unused_mut)]
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
    .focused(false)
    // Must match `main` (tauri.conf.json) and `preview` exactly — see WEBVIEW2_BROWSER_ARGS.
    .additional_browser_args(WEBVIEW2_BROWSER_ARGS);

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

    // --- Position the bar ---
    //
    // Priority:
    //   1. Persisted config position  (user dragged the bar, we saved it)
    //   2. Win32 SPI_GETWORKAREA      (correct usable area, excludes taskbar)
    //   3. monitor.size() fallback    (60 px conservative taskbar estimate)
    //   4. Hard-coded fallback        (no monitor detected at all)

    if let (Some(cx), Some(cy)) = (saved_x, saved_y) {
        log::info!("[bar] Using saved config position ({cx}, {cy})");
        let _ = bar.set_position(tauri::LogicalPosition::new(cx, cy));
        return Ok(());
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::Foundation::RECT;
        use windows::Win32::UI::WindowsAndMessaging::{
            SystemParametersInfoW, SPI_GETWORKAREA, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
        };

        let mut work_area = RECT::default();
        // SAFETY: `work_area` is a valid stack-allocated RECT; the Win32 call
        // writes into it and we check the return value before using the data.
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&raw mut work_area as *mut _),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
        };

        match ok {
            Ok(()) => {
                // `SPI_GETWORKAREA` returns physical pixels; divide by the bar
                // window's scale factor to get logical coordinates that Tauri
                // expects for `set_position`.
                let scale = bar.scale_factor().unwrap_or(1.0);
                let work_w = (work_area.right - work_area.left) as f64 / scale;
                let work_bottom = work_area.bottom as f64 / scale;
                let work_left = work_area.left as f64 / scale;

                let x = work_left + (work_w - bar_width) / 2.0;
                let y = work_bottom - bar_height - 8.0;

                log::info!(
                    "[bar] SPI_GETWORKAREA -> work_area=({},{},{},{}) scale={scale:.2}, placing at ({x:.1}, {y:.1})",
                    work_area.left, work_area.top, work_area.right, work_area.bottom,
                );
                let _ = bar.set_position(tauri::LogicalPosition::new(x, y));
                return Ok(());
            }
            Err(e) => {
                log::warn!("[bar] SPI_GETWORKAREA failed ({e}), falling back to monitor size");
            }
        }
    }

    // Fallback: derive position from monitor bounds with a conservative
    // 60 px taskbar estimate.
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
            // 60 px: conservative estimate for taskbar height at common DPI settings.
            let y = offset_y + screen_h - bar_height - 60.0;
            log::info!(
                "[bar] monitor fallback: screen={screen_w}x{screen_h} scale={scale:.2} \
                 offset=({offset_x},{offset_y}), placing at ({x:.1}, {y:.1})"
            );
            let _ = bar.set_position(tauri::LogicalPosition::new(x, y));
        }
        _ => {
            log::warn!("[bar] No monitor detected, using hard-coded fallback position");
            let _ = bar.set_position(tauri::LogicalPosition::new(400.0, 10.0));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Preview window creation
// ---------------------------------------------------------------------------

/// Creates the standalone transparent, click-through `"preview"` window.
///
/// The window starts as a 1×1 hidden stub; Story 6.2 resizes it to the
/// computed `clampedMaxHeight × width` geometry on the first live-preview
/// chunk. No position is set here — 6.2 derives the position from the pill
/// anchor on first show.
///
/// Accepts any `Manager<R>` implementor (`App`, `AppHandle`, etc.) so this
/// function can be called from both the `setup` closure and from recovery
/// commands that only have access to an `AppHandle`.
#[cfg(desktop)]
pub fn create_preview_window<M: tauri::Manager<tauri::Wry>>(
    app: &M,
) -> Result<(), Box<dyn std::error::Error>> {
    #[allow(unused_mut)]
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        "preview",
        WebviewUrl::App("index.html".into()),
    )
    .title("")
    // Create the window VISIBLE at a real size.
    //   - NOT 1×1: a near-zero WebView2 surface never initializes the JS/event bridge
    //     (PreviewPanel never mounts, listeners never register).
    //   - Created visible (no `.visible(false)`): a window resident from startup is
    //     guaranteed to have mounted + registered its state-changed / live-preview-chunk
    //     listeners before the first recording, so the first chunk is never missed. It is
    //     fully transparent + click-through and renders nothing until there is preview
    //     text, so a resident empty window is invisible and non-interactive to the user.
    //
    // (History: the preview "received no events" bug was NOT a hidden-window throttle —
    //  it was a missing capability ("preview" absent from capabilities/default.json
    //  windows) that denied core:event:allow-listen, plus a tuple-vs-object serialization
    //  bug in get_bar_position that made runShowSequence compute NaN. Once both were
    //  fixed, a hidden window was observed to receive events fine across recording cycles.
    //  The window is left resident-visible because it is the verified-working config and
    //  reverting to `.visible(false)` would re-risk first-cycle webview-init timing.)
    //
    // runShowSequence sets the exact size/region/position on the first chunk.
    // Size is the max preset (wide 400 × maxHeight 600).
    .inner_size(400.0, 600.0)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .resizable(false)
    .skip_taskbar(true)
    .focused(false)
    // Must match `main` (tauri.conf.json) and `bar` exactly — see WEBVIEW2_BROWSER_ARGS.
    .additional_browser_args(WEBVIEW2_BROWSER_ARGS);

    // Remove window shadow so only the CSS-rendered content is visible.
    #[cfg(target_os = "windows")]
    {
        builder = builder.shadow(false);
    }

    let preview = builder.build()?;

    // Make the window click-through: cursor events pass to whatever is behind it.
    // Fail-soft: if the call fails the window still renders, just not click-through.
    if let Err(e) = preview.set_ignore_cursor_events(true) {
        log::warn!("[preview] set_ignore_cursor_events failed: {e}");
    }

    log::info!("[setup] preview window created (transparent, 400x600, shown on first chunk)");
    Ok(())
}

// ---------------------------------------------------------------------------
// Tauri entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let mut builder = tauri::Builder::default();

    // Structured logging: stdout/logcat + rotating log file in {app_log_dir}/klarvo.log
    // Defaults: 40KB max / KeepOne / UTC — we override for real-world debugging.
    builder = builder.plugin(
        tauri_plugin_log::Builder::new()
            .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
            .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
            .max_file_size(2_000_000) // 2MB per file
            .level(log::LevelFilter::Info)
            .build(),
    );

    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_opener::init())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_updater::Builder::new().build());
    }

    let mut builder = builder.setup(|app| {
        // Resolve the app-data directory (e.g. %APPDATA%\com.klarvo.voice on Windows).
        let app_data_dir = app
            .path()
            .app_data_dir()
            .expect("Tauri must provide an app-data directory");

        // Create the directory if it doesn't exist yet.
        std::fs::create_dir_all(&app_data_dir)?;

        // Check for early-adopter migration BEFORE loading config (we need to
        // know whether the license_key field was absent in the on-disk file).
        // Load persisted config (falls back to defaults + env vars on first run).
        // Use the reporting variant so a corrupt-config backup warning (ROB-02 /
        // ADR-0015) can be surfaced once the main window is up (emitted below).
        let mut config_warnings: Vec<String> = Vec::new();
        let mut cfg = load_config_reporting(&app_data_dir, &mut config_warnings);

        // Record first install timestamp on the very first launch.
        if cfg.first_install_at == 0 {
            cfg.first_install_at = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = save_config(&app_data_dir, &cfg);
            log::info!("[trial] First install detected, set first_install_at = {}", cfg.first_install_at);
        }

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

        // Extract bar position before `cfg` is moved into AppState.
        let _saved_bar_x = cfg.bar_x;
        let _saved_bar_y = cfg.bar_y;

        // Build and register the application state.
        let app_state = AppState::new(cfg, dictionary, app_data_dir, history_db);
        app.manage(app_state);

        // --- System tray (Windows only -- WSL2/Linux lacks proper tray support) ---
        #[cfg(target_os = "windows")]
        {
            let show_settings =
                MenuItem::with_id(app, "show_settings", "Settings", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show_settings, &quit])?;

            let tray_tooltip = format!("Klarvo \u{00b7} Early Access \u{2014} {hotkey_str}");
            let _tray = tauri::tray::TrayIconBuilder::with_id("klarvo-tray")
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

        // --- Native pill overlay (replaces WebView2 "bar" window, Story 10-1) ---
        #[cfg(target_os = "windows")]
        {
            match native_pill::NativePill::create(app.handle().clone(), _saved_bar_x, _saved_bar_y) {
                Ok(pill) => {
                    if let Ok(mut guard) = app.state::<AppState>().native_pill.lock() {
                        *guard = Some(pill);
                    }
                    log::info!("[setup] Native pill overlay created");
                }
                Err(e) => log::warn!("[setup] Could not create native pill overlay: {e}"),
            }
        }

        // --- Standalone preview window (transparent, click-through, always-on-top) ---
        #[cfg(target_os = "windows")]
        if let Err(e) = create_preview_window(app) {
            log::warn!("[setup] Could not create preview window: {e}");
        }

        // --- Desktop-only setup: audio level emitter + global hotkey ---
        #[cfg(desktop)]
        {
            let handle = app.handle().clone();
            setup_audio_level_emitter(&handle);

            log::info!("[setup] Registering hotkey slots from config");
            match register_hotkey(&handle) {
                Ok(()) => log::info!("[hotkey] Hotkey slots registered"),
                Err(e) => log::warn!(
                    "[hotkey] Could not register hotkey slots: {e}. Use the UI button instead."
                ),
            }

            // Auto-start Voice Command Monitor -- DISABLED (parked until SAPI implementation).
            // Code kept for when we re-enable with Windows Speech Recognition API.
            #[allow(unreachable_code)]
            if false {
                let state = app.state::<AppState>();
                let vc_enabled = state.config.lock().map(|c| c.voice_command_enabled).unwrap_or(false);
                log::debug!("[setup] vc_enabled={vc_enabled}");
                if vc_enabled {
                    log::info!("[setup] voice_command_enabled=true -- starting monitor");
                    if let Err(e) = voice_command::start_voice_command_monitor(&handle) {
                        log::warn!("[setup] Failed to auto-start voice command monitor: {e}");
                        // Reset config flag so the next launch does not attempt
                        // auto-start again (avoids a phantom-start loop).
                        // TODO(ROB-04): if this `if false` block is ever re-enabled, route the
                        // reset through `state.save_config_locked("voice command state", |c| ...)`
                        // instead of holding `config` across this direct `save_config` — the
                        // hand-written pattern below bypasses the disk-write serialization lock.
                        if let Ok(mut cfg) = state.config.lock() {
                            cfg.voice_command_enabled = false;
                            let dir = state.app_data_dir.clone();
                            if let Err(save_err) = save_config(&dir, &cfg) {
                                log::warn!("[setup] Could not persist voice_command_enabled=false: {save_err}");
                            } else {
                                log::info!("[setup] voice_command_enabled reset to false in config");
                            }
                        }
                    }
                }
            }
        }

        // Always show the main window on launch (desktop only).
        #[cfg(desktop)]
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.set_focus();
        }

        // Surface any boot-time config warnings (e.g. a corrupt config.json was
        // backed up to config.json.corrupt-<ts>). Best-effort, fire-and-forget:
        // emitted via the canonical state emitter AFTER the main window is shown so
        // the frontend listener has the best chance of catching it. Per D1 this
        // toast may still be lost to the boot race — the durable recovery surface is
        // the backup file itself, not this event; reliable pull-based delivery is a
        // deferred follow-up. No trailing `done`/`idle` emit: `warn` is message-only
        // and the frontend treats it as transient (recordingState stays idle), which
        // is correct at boot.
        #[cfg(desktop)]
        for warning in config_warnings {
            emit_pipeline_state(app.handle(), hotkey::PipelineEvent::warn(warning));
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
                // Preview window: always prevent close (native pill has no Tauri window).
                if label == "preview" {
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
            commands::settings::set_hotkey_paused,
            commands::settings::get_advanced_settings,
            commands::settings::save_advanced_settings,
            // Onboarding
            commands::settings::get_onboarding_state,
            commands::settings::set_onboarding_state,
            commands::settings::validate_api_key,
            commands::settings::clear_api_key,
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
            commands::history::is_tip_shown,
            commands::history::mark_tip_shown,
            // Feedback
            commands::feedback::send_feedback,
            commands::feedback::get_feedback_metrics,
            // Misc: profiles, snippets, sync, paste, UI helpers
            commands::misc::get_profiles,
            commands::misc::save_profiles,
            commands::misc::get_snippets,
            commands::misc::save_snippets,
            commands::misc::paste_snippet,
            commands::misc::sync_history,
            commands::recording::cancel_recording,
            commands::misc::set_bar_shape,
            commands::misc::set_preview_shape,
            commands::misc::frontend_log,
            commands::misc::save_bar_position,
            commands::misc::get_bar_position,
            #[cfg(desktop)]
            commands::misc::ensure_bar_window,
            #[cfg(desktop)]
            commands::misc::ensure_preview_window,
            commands::misc::get_log_dir_path,
            commands::misc::read_recent_logs,
            commands::misc::get_build_info,
            // License
            commands::license::validate_license,
            commands::license::get_license_status,
            commands::license::remove_license,
            commands::license::deactivate_license,
            commands::license::get_license_source,
            // Whisper model manager (Windows)
            #[cfg(target_os = "windows")]
            commands::whisper::windows::get_whisper_models,
            #[cfg(target_os = "windows")]
            commands::whisper::windows::download_whisper_model,
            #[cfg(target_os = "windows")]
            commands::whisper::windows::delete_whisper_model,
            // Whisper model manager + offline transcription (Android)
            #[cfg(target_os = "android")]
            commands::whisper::android::get_whisper_models,
            #[cfg(target_os = "android")]
            commands::whisper::android::download_whisper_model,
            #[cfg(target_os = "android")]
            commands::whisper::android::delete_whisper_model,
            #[cfg(target_os = "android")]
            commands::whisper::android::transcribe_local,
            // LLM model manager (Windows: GGUF for llama.cpp)
            #[cfg(target_os = "windows")]
            commands::llm_model::windows::get_llm_model_status,
            #[cfg(target_os = "windows")]
            commands::llm_model::windows::download_llm_model,
            #[cfg(target_os = "windows")]
            commands::llm_model::windows::delete_llm_model,
            // LLM model manager (Android: MNN model bundle)
            #[cfg(target_os = "android")]
            commands::llm_model::android::get_llm_model_status,
            #[cfg(target_os = "android")]
            commands::llm_model::android::download_llm_model,
            #[cfg(target_os = "android")]
            commands::llm_model::android::delete_llm_model,
            // Voice Command Mode (desktop only)
            #[cfg(desktop)]
            commands::voice_command::toggle_voice_command_mode,
            #[cfg(desktop)]
            commands::voice_command::get_voice_command_active,
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
            openrouter_api_key_masked: String::new(),
            stt_provider: "groq".to_string(),
            llm_provider: "deepseek".to_string(),
            output_language: String::new(),
            webhook_url: String::new(),
            turso_url: String::new(),
            turso_token_masked: String::new(),
            device_id: "test-device".to_string(),
            bubble_size: 1.0,
            bubble_opacity: 0.85,
            local_whisper_model: "base".to_string(),
            local_whisper_gpu: true,
            insert_and_send_slot1: false,
            insert_and_send_slot2: false,
            autostop_silence_secs: 2.0,
            auto_mode_silence_secs: 2.0,
            hotkey_slot2: String::new(),
            hotkey_mode_slot2: HotkeyMode::Hold,
            bubble_recording_mode: "hold".to_string(),
            bubble_tap_mode: "toggle".to_string(),
            bubble_tap_auto_send: false,
            bubble_tap_silence_secs: 2.0,
            bubble_long_press_mode: "hold".to_string(),
            bubble_long_press_auto_send: false,
            bubble_long_press_silence_secs: 2.0,
            voice_command_enabled: false,
            feedback_webhook_url: String::new(),
            live_preview_enabled: false,
            preview_pause_silence_secs: 2.0,
            preview_panel_form: "comfortable".to_string(),
            preview_text_color: "rgba(220,220,220,0.88)".to_string(),
            preview_bg_color: "rgba(25,25,25,0.96)".to_string(),
            preview_bg_blur: 12,
            preview_border_color: "rgba(42,195,168,0.25)".to_string(),
            preview_border_width: 1,
            preview_border_radius: 14,
            preview_font_family: "'Inter', system-ui, -apple-system, sans-serif".to_string(),
            preview_font_size: "small".to_string(),
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
            openrouter_api_key_masked: String::new(),
            stt_provider: "groq".to_string(),
            llm_provider: "deepseek".to_string(),
            output_language: String::new(),
            webhook_url: String::new(),
            turso_url: String::new(),
            turso_token_masked: String::new(),
            device_id: "test-device".to_string(),
            bubble_size: 1.0,
            bubble_opacity: 0.85,
            local_whisper_model: "base".to_string(),
            local_whisper_gpu: true,
            insert_and_send_slot1: false,
            insert_and_send_slot2: false,
            autostop_silence_secs: 2.0,
            auto_mode_silence_secs: 2.0,
            hotkey_slot2: String::new(),
            hotkey_mode_slot2: HotkeyMode::Hold,
            bubble_recording_mode: "hold".to_string(),
            bubble_tap_mode: "toggle".to_string(),
            bubble_tap_auto_send: false,
            bubble_tap_silence_secs: 2.0,
            bubble_long_press_mode: "hold".to_string(),
            bubble_long_press_auto_send: false,
            bubble_long_press_silence_secs: 2.0,
            voice_command_enabled: false,
            feedback_webhook_url: String::new(),
            live_preview_enabled: false,
            preview_pause_silence_secs: 2.0,
            preview_panel_form: "comfortable".to_string(),
            preview_text_color: "rgba(220,220,220,0.88)".to_string(),
            preview_bg_color: "rgba(25,25,25,0.96)".to_string(),
            preview_bg_blur: 12,
            preview_border_color: "rgba(42,195,168,0.25)".to_string(),
            preview_border_width: 1,
            preview_border_radius: 14,
            preview_font_family: "'Inter', system-ui, -apple-system, sans-serif".to_string(),
            preview_font_size: "small".to_string(),
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
            openrouter_api_key_masked: String::new(),
            stt_provider: "groq".to_string(),
            llm_provider: "deepseek".to_string(),
            output_language: String::new(),
            webhook_url: String::new(),
            turso_url: String::new(),
            turso_token_masked: String::new(),
            device_id: "test-device".to_string(),
            bubble_size: 1.0,
            bubble_opacity: 0.85,
            local_whisper_model: "base".to_string(),
            local_whisper_gpu: true,
            insert_and_send_slot1: false,
            insert_and_send_slot2: false,
            autostop_silence_secs: 2.0,
            auto_mode_silence_secs: 2.0,
            hotkey_slot2: String::new(),
            hotkey_mode_slot2: HotkeyMode::Hold,
            bubble_recording_mode: "hold".to_string(),
            bubble_tap_mode: "toggle".to_string(),
            bubble_tap_auto_send: false,
            bubble_tap_silence_secs: 2.0,
            bubble_long_press_mode: "hold".to_string(),
            bubble_long_press_auto_send: false,
            bubble_long_press_silence_secs: 2.0,
            voice_command_enabled: false,
            feedback_webhook_url: String::new(),
            live_preview_enabled: false,
            preview_pause_silence_secs: 2.0,
            preview_panel_form: "comfortable".to_string(),
            preview_text_color: "rgba(220,220,220,0.88)".to_string(),
            preview_bg_color: "rgba(25,25,25,0.96)".to_string(),
            preview_bg_blur: 12,
            preview_border_color: "rgba(42,195,168,0.25)".to_string(),
            preview_border_width: 1,
            preview_border_radius: 14,
            preview_font_family: "'Inter', system-ui, -apple-system, sans-serif".to_string(),
            preview_font_size: "small".to_string(),
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

    // --- Trial license logic ---

    #[test]
    fn test_trial_active() {
        let five_days_ago = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .saturating_sub(5 * 24 * 60 * 60);
        let dir = temp_dir();
        let cfg = AppConfig { first_install_at: five_days_ago, ..AppConfig::default() };
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let state = AppState::new(cfg, Dictionary::new(), dir.path().to_path_buf(), db);
        let status = state.license_status.lock().unwrap().clone();
        assert!(matches!(status, license::LicenseStatus::Trial { .. }), "expected Trial, got {status:?}");
    }

    #[test]
    fn test_trial_expired() {
        // 30 days ago — well beyond the 14-day trial window
        let past_trial = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .saturating_sub(30 * 24 * 60 * 60);
        let dir = temp_dir();
        let cfg = AppConfig { first_install_at: past_trial, ..AppConfig::default() };
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let state = AppState::new(cfg, Dictionary::new(), dir.path().to_path_buf(), db);
        let status = state.license_status.lock().unwrap().clone();
        assert!(matches!(status, license::LicenseStatus::Unlicensed), "expected Unlicensed, got {status:?}");
    }

    #[test]
    fn test_licensed_user_ignores_trial() {
        let five_days_ago = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .saturating_sub(5 * 24 * 60 * 60);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dir = temp_dir();
        let cfg = AppConfig {
            license_key: "test-key".to_string(),
            license_validated_at: now,
            first_install_at: five_days_ago,
            ..AppConfig::default()
        };
        let db = rusqlite::Connection::open_in_memory().unwrap();
        let state = AppState::new(cfg, Dictionary::new(), dir.path().to_path_buf(), db);
        let status = state.license_status.lock().unwrap().clone();
        assert!(!matches!(status, license::LicenseStatus::Trial { .. }), "licensed user got Trial: {status:?}");
    }
}
