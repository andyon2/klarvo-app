//! Settings persistence module.
//!
//! Loads and saves application configuration as a JSON file in the Tauri
//! app-data directory (`{app_data_dir}/config.json`).
//!
//! Design decisions:
//! - JSON over SQLite for MVP: simpler dependency graph, human-readable file.
//! - `AppConfig` is a flat struct -- no nesting needed for the current settings.
//! - API keys are stored on disk as plain text inside the user-owned app-data
//!   directory. A future improvement could use the system keystore (Windows
//!   Credential Manager). For now, the file is only readable by the current
//!   user (OS-level permissions on the app-data dir).
//! - Defaults are returned when the file does not exist (first run).
//! - `load_config` never fails: a missing or corrupt file yields defaults and
//!   logs a warning so the app can always start.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::llm::CleanupStyle;

// ---------------------------------------------------------------------------
// AdvancedSettings
// ---------------------------------------------------------------------------

/// Fine-grained controls for power users. Exposed via the "Advanced Settings"
/// tab in the UI. All fields have sensible defaults so existing config files
/// (without an `advanced` key) load without errors.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AdvancedSettings {
    // --- STT ---

    /// Whisper conditioning prompt for German input. When non-empty, overrides
    /// the built-in German hint that prepends dictionary terms.
    #[serde(default = "default_stt_prompt_de")]
    pub stt_prompt_de: String,

    /// Whisper conditioning prompt for English input.
    #[serde(default = "default_stt_prompt_en")]
    pub stt_prompt_en: String,

    /// Whisper conditioning prompt when language is set to auto-detect.
    #[serde(default = "default_stt_prompt_auto")]
    pub stt_prompt_auto: String,

    /// Whisper sampling temperature. 0.0 = deterministic (recommended for
    /// dictation). Higher values increase randomness.
    #[serde(default = "default_stt_temperature")]
    pub stt_temperature: f32,

    // --- LLM system prompts ---

    /// Custom system prompt for the Polished cleanup style.
    /// Empty string = use the built-in prompt.
    #[serde(default)]
    pub llm_system_prompt_polished: String,

    /// Custom system prompt for the Verbatim cleanup style.
    /// Empty string = use the built-in prompt.
    #[serde(default)]
    pub llm_system_prompt_verbatim: String,

    /// Custom system prompt for the Chat cleanup style.
    /// Empty string = use the built-in prompt.
    #[serde(default)]
    pub llm_system_prompt_chat: String,

    /// Custom system prompt for Command Mode.
    /// Empty string = use the built-in prompt.
    #[serde(default)]
    pub llm_command_mode_prompt: String,

    /// LLM sampling temperature. 0.0 = deterministic.
    #[serde(default = "default_llm_temperature")]
    pub llm_temperature: f32,

    /// Maximum output tokens for LLM calls.
    #[serde(default = "default_llm_max_tokens")]
    pub llm_max_tokens: u32,

    /// Model override for DeepSeek. Empty = use built-in default.
    #[serde(default)]
    pub llm_model_deepseek: String,

    /// Model override for OpenAI LLM. Empty = use built-in default.
    #[serde(default)]
    pub llm_model_openai: String,

    /// Model override for Anthropic. Empty = use built-in default.
    #[serde(default)]
    pub llm_model_anthropic: String,

    /// Model override for Groq LLM. Empty = use built-in default.
    #[serde(default)]
    pub llm_model_groq: String,

    /// Character count above which text is split into parallel chunks.
    #[serde(default = "default_chunk_threshold")]
    pub chunk_threshold: u32,

    /// Target character count per chunk.
    #[serde(default = "default_chunk_target_size")]
    pub chunk_target_size: u32,

    // --- Audio ---

    /// RMS silence detection threshold. Audio below this level is treated as
    /// silence and the pipeline is skipped. Default matches the hardcoded value.
    #[serde(default = "default_silence_threshold")]
    pub silence_threshold: f32,

    /// RMS threshold used in whisper mode (amplified audio). Should be lower
    /// than `silence_threshold` because the gain has already been applied.
    #[serde(default = "default_whisper_mode_threshold")]
    pub whisper_mode_threshold: f32,

    /// Minimum recording duration in milliseconds. Recordings shorter than this
    /// are discarded without calling STT.
    #[serde(default = "default_min_recording_ms")]
    pub min_recording_ms: u32,

    /// Audio gain multiplier applied when whisper mode is active.
    #[serde(default = "default_whisper_mode_gain")]
    pub whisper_mode_gain: f32,

    // --- Paste & behaviour ---

    /// When `false`, the pipeline transcribes and cleans up text but does NOT
    /// paste it into the target window. Useful for review-before-paste workflows.
    #[serde(default = "default_auto_paste")]
    pub auto_paste: bool,

    /// Milliseconds to wait after focusing the target window before pasting.
    #[serde(default = "default_paste_delay_ms")]
    pub paste_delay_ms: u32,

    /// Automatically capitalize the first letter of the cleaned text.
    #[serde(default = "default_auto_capitalize")]
    pub auto_capitalize: bool,

    // --- Webhook ---

    /// Custom HTTP headers to send with webhook POST requests, encoded as a
    /// JSON object string (e.g. `{"X-My-Header": "value"}`).
    /// Empty string = no extra headers.
    #[serde(default)]
    pub webhook_headers: String,

    /// Timeout in seconds for webhook HTTP requests.
    #[serde(default = "default_webhook_timeout_secs")]
    pub webhook_timeout_secs: u32,

    // --- System ---

    /// Log verbosity level. One of `"debug"`, `"info"`, `"warn"`, `"error"`.
    #[serde(default = "default_log_level")]
    pub log_level: String,

    // --- UI ---

    /// UI zoom level. One of `"small"`, `"medium"`, `"large"`.
    /// Default: `"medium"` (100%).
    #[serde(default = "default_ui_scale")]
    pub ui_scale: String,
}

fn default_stt_prompt_de() -> String {
    "Diktat auf Deutsch mit gelegentlichen englischen Fachbegriffen. Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion. ".to_string()
}

fn default_stt_prompt_en() -> String {
    "Voice dictation in English. Proper punctuation, capitalization, and spelling. ".to_string()
}

fn default_stt_prompt_auto() -> String {
    "Multilingual voice dictation. German and English with proper punctuation. ".to_string()
}

fn default_stt_temperature() -> f32 {
    0.0
}

fn default_llm_temperature() -> f32 {
    0.0
}

fn default_llm_max_tokens() -> u32 {
    4096
}

fn default_chunk_threshold() -> u32 {
    800
}

fn default_chunk_target_size() -> u32 {
    600
}

fn default_silence_threshold() -> f32 {
    0.005
}

fn default_whisper_mode_threshold() -> f32 {
    0.001
}

fn default_min_recording_ms() -> u32 {
    500
}

fn default_whisper_mode_gain() -> f32 {
    3.0
}

fn default_auto_paste() -> bool {
    true
}

fn default_paste_delay_ms() -> u32 {
    50
}

fn default_auto_capitalize() -> bool {
    true
}

fn default_webhook_timeout_secs() -> u32 {
    10
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_ui_scale() -> String {
    "medium".to_string()
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        AdvancedSettings {
            stt_prompt_de: default_stt_prompt_de(),
            stt_prompt_en: default_stt_prompt_en(),
            stt_prompt_auto: default_stt_prompt_auto(),
            stt_temperature: default_stt_temperature(),
            llm_system_prompt_polished: String::new(),
            llm_system_prompt_verbatim: String::new(),
            llm_system_prompt_chat: String::new(),
            llm_command_mode_prompt: String::new(),
            llm_temperature: default_llm_temperature(),
            llm_max_tokens: default_llm_max_tokens(),
            llm_model_deepseek: String::new(),
            llm_model_openai: String::new(),
            llm_model_anthropic: String::new(),
            llm_model_groq: String::new(),
            chunk_threshold: default_chunk_threshold(),
            chunk_target_size: default_chunk_target_size(),
            silence_threshold: default_silence_threshold(),
            whisper_mode_threshold: default_whisper_mode_threshold(),
            min_recording_ms: default_min_recording_ms(),
            whisper_mode_gain: default_whisper_mode_gain(),
            auto_paste: default_auto_paste(),
            paste_delay_ms: default_paste_delay_ms(),
            auto_capitalize: default_auto_capitalize(),
            webhook_headers: String::new(),
            webhook_timeout_secs: default_webhook_timeout_secs(),
            log_level: default_log_level(),
            ui_scale: default_ui_scale(),
        }
    }
}

// ---------------------------------------------------------------------------
// TextSnippet
// ---------------------------------------------------------------------------

/// A reusable text block the user can quickly paste anywhere.
///
/// Snippets are stored as a flat list in `AppConfig` and identified by their
/// short trigger `name` (e.g. `"sig"`, `"addr"`). The frontend can display
/// them in a panel or offer keyboard-driven selection.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TextSnippet {
    /// Short trigger name shown in the UI (e.g. `"sig"`, `"addr"`, `"greeting"`).
    pub name: String,
    /// The full text to insert when the snippet is activated.
    pub content: String,
}

// ---------------------------------------------------------------------------
// AppProfile
// ---------------------------------------------------------------------------

/// A per-application recording profile.
///
/// When recording starts, the foreground window title is matched against
/// `app_pattern` (case-insensitive substring). The first matching profile
/// overrides the global `cleanup_style`, `language`, and `custom_prompt`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppProfile {
    /// Human-readable name shown in the UI.
    pub name: String,
    /// Case-insensitive substring matched against the foreground window title.
    pub app_pattern: String,
    /// Cleanup style to use when this profile matches.
    pub cleanup_style: CleanupStyle,
    /// ISO-639-1 language code (e.g. `"de"`, `"en"`). Empty = auto-detect.
    pub language: String,
    /// Additional instructions appended to the LLM system prompt.
    pub custom_prompt: String,
}

// ---------------------------------------------------------------------------
// HotkeyMode
// ---------------------------------------------------------------------------

/// Controls how the global hotkey triggers recording.
///
/// - `Toggle`: one press starts recording, the next press stops and processes.
/// - `Hold`: hold the key to record; releasing triggers stop + pipeline.
///
/// Default is `Hold` -- this matches the Wispr Flow UX that users expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HotkeyMode {
    Toggle,
    Hold,
}

impl Default for HotkeyMode {
    fn default() -> Self {
        HotkeyMode::Hold
    }
}

// ---------------------------------------------------------------------------
// Configuration struct
// ---------------------------------------------------------------------------

/// Persisted application settings.
///
/// All fields have defaults via `Default` so a partially-written or
/// absent config file always yields a usable value.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// Groq API key (used for both STT and LLM providers).
    #[serde(default)]
    pub groq_api_key: String,

    /// DeepSeek API key for the LLM cleanup provider.
    #[serde(default)]
    pub deepseek_api_key: String,

    /// OpenAI API key (used for both STT and LLM providers).
    #[serde(default)]
    pub openai_api_key: String,

    /// Anthropic API key (used for LLM provider only).
    #[serde(default)]
    pub anthropic_api_key: String,

    /// Ordered list of STT provider IDs. The first provider with a configured
    /// API key is used. Falls back to the next in the list on missing key.
    /// Valid values: `"groq"`, `"openai"`.
    #[serde(default = "default_stt_priority")]
    pub stt_priority: Vec<String>,

    /// Ordered list of LLM provider IDs. The first provider with a configured
    /// API key is used. Falls back to the next in the list on missing key.
    /// Valid values: `"deepseek"`, `"openai"`, `"anthropic"`, `"groq"`.
    #[serde(default = "default_llm_priority")]
    pub llm_priority: Vec<String>,

    /// ISO-639-1 language code used for transcription (e.g. `"de"`, `"en"`).
    /// Empty string = auto-detect.
    #[serde(default = "default_language")]
    pub language: String,

    /// Cleanup aggressiveness for the LLM step.
    #[serde(default = "default_cleanup_style")]
    pub cleanup_style: CleanupStyle,

    /// Global hotkey string in Tauri shortcut format (e.g. `"ctrl+shift+d"`).
    #[serde(default = "default_hotkey")]
    pub hotkey: String,

    /// How the hotkey triggers recording: toggle (press/press) or hold (hold/release).
    #[serde(default = "default_hotkey_mode")]
    pub hotkey_mode: HotkeyMode,

    /// Name of the selected audio input device. `None` = system default.
    #[serde(default)]
    pub audio_device: Option<String>,

    /// Groq Whisper model variant to use for transcription.
    /// Defaults to `whisper-large-v3-turbo` (fast, cheap, good quality).
    #[serde(default = "default_stt_model")]
    pub stt_model: String,

    /// Additional instructions appended to the LLM system prompt.
    /// Allows the user to inject domain-specific rules (e.g. "always use
    /// formal German", "don't add line breaks").
    #[serde(default)]
    pub custom_prompt: String,

    /// Per-application recording profiles.
    /// The first profile whose `app_pattern` matches the foreground window
    /// title overrides the global settings for that recording session.
    #[serde(default)]
    pub profiles: Vec<AppProfile>,

    /// Launch Dikta automatically when the user logs in.
    /// On Windows this writes/removes a `HKCU\...\Run` registry entry.
    #[serde(default)]
    pub autostart: bool,

    /// Whisper mode: amplifies audio for quiet/whispered speech.
    /// When enabled, a 3x gain is applied before sending to STT,
    /// and the silence detection threshold is lowered.
    #[serde(default)]
    pub whisper_mode: bool,

    /// Hotkey for Command Mode (voice-edit selected text).
    /// Default: ctrl+shift+e
    #[serde(default = "default_command_hotkey")]
    pub command_hotkey: String,

    /// ISO-639-1 target language for translation.
    /// Empty string = no translation (output in the same language as input).
    /// E.g. `"en"` = translate to English, `"de"` = translate to German.
    #[serde(default = "default_output_language")]
    pub output_language: String,

    /// User-defined reusable text snippets.
    /// Stored as an ordered list -- order determines display in the UI.
    #[serde(default)]
    pub snippets: Vec<TextSnippet>,

    /// Hotkey string that triggers Voice Notes Mode instead of regular dictation.
    /// When pressed, the dictation result is saved as a note (not pasted into the
    /// active window). Empty string = Voice Notes Mode disabled.
    /// Example: `"ctrl+shift+n"`.
    #[serde(default = "default_voice_notes_hotkey")]
    pub voice_notes_hotkey: String,

    /// Webhook URL for HTTP POST notifications after each dictation.
    /// Empty string = webhook disabled.
    /// The backend sends a JSON POST to this URL after every successful pipeline run.
    #[serde(default)]
    pub webhook_url: String,

    /// Turso database URL for cross-device history sync.
    /// Format: `libsql://db-name.turso.io` — the sync module converts to HTTPS.
    /// Empty string = sync disabled.
    #[serde(default)]
    pub turso_url: String,

    /// Turso authentication token (JWT).
    /// Empty string = sync disabled.
    #[serde(default)]
    pub turso_token: String,

    /// Unique device identifier for sync deduplication.
    /// Auto-generated on first run (UUID v4 format).
    #[serde(default = "default_device_id")]
    pub device_id: String,

    /// Android floating bubble size multiplier.
    /// 0.5 = 50% of default, 1.0 = default (56 dp), 2.0 = double size.
    /// Only used on Android; ignored on desktop.
    #[serde(default = "default_bubble_size")]
    pub bubble_size: f32,

    /// Android floating bubble opacity when idle.
    /// Range: 0.3 (30%) to 1.0 (100%). Default: 0.85.
    /// Only used on Android; ignored on desktop.
    #[serde(default = "default_bubble_opacity")]
    pub bubble_opacity: f32,

    /// Fine-grained advanced settings for power users.
    /// Defaults to `AdvancedSettings::default()` so existing config files
    /// without this field load correctly.
    #[serde(default)]
    pub advanced: AdvancedSettings,

    // --- License ---

    /// Validated license key string. Empty = no license.
    /// Stored in config.json; the key itself is not secret but the HMAC
    /// embedded in it can only be forged with the compile-time secret.
    #[serde(default)]
    pub license_key: String,

    /// Unix timestamp (seconds) at which the license key was last validated.
    /// Used together with `license_key` to compute offline status.
    /// 0 = never validated.
    #[serde(default)]
    pub license_validated_at: u64,
}

fn default_stt_priority() -> Vec<String> {
    vec!["groq".to_string(), "openai".to_string()]
}

fn default_llm_priority() -> Vec<String> {
    vec![
        "deepseek".to_string(),
        "openai".to_string(),
        "anthropic".to_string(),
        "groq".to_string(),
    ]
}

fn default_language() -> String {
    String::new() // empty = auto-detect (Groq Whisper handles DE+EN mix)
}

fn default_stt_model() -> String {
    "whisper-large-v3-turbo".to_string()
}

fn default_cleanup_style() -> CleanupStyle {
    CleanupStyle::Polished
}

fn default_hotkey() -> String {
    "ctrl+shift+d".to_string()
}

pub fn default_hotkey_mode() -> HotkeyMode {
    HotkeyMode::Hold
}

fn default_command_hotkey() -> String {
    "ctrl+shift+e".to_string()
}

pub fn default_output_language() -> String {
    String::new() // empty = no translation
}

pub fn default_voice_notes_hotkey() -> String {
    String::new() // empty = Voice Notes Mode disabled
}

fn default_device_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn default_bubble_size() -> f32 {
    1.0
}

fn default_bubble_opacity() -> f32 {
    0.85
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            groq_api_key: String::new(),
            deepseek_api_key: String::new(),
            openai_api_key: String::new(),
            anthropic_api_key: String::new(),
            stt_priority: default_stt_priority(),
            llm_priority: default_llm_priority(),
            language: default_language(),
            cleanup_style: default_cleanup_style(),
            hotkey: default_hotkey(),
            hotkey_mode: default_hotkey_mode(),
            audio_device: None,
            stt_model: default_stt_model(),
            custom_prompt: String::new(),
            profiles: Vec::new(),
            autostart: false,
            whisper_mode: false,
            command_hotkey: default_command_hotkey(),
            output_language: default_output_language(),
            snippets: Vec::new(),
            voice_notes_hotkey: default_voice_notes_hotkey(),
            webhook_url: String::new(),
            turso_url: String::new(),
            turso_token: String::new(),
            device_id: default_device_id(),
            bubble_size: default_bubble_size(),
            bubble_opacity: default_bubble_opacity(),
            advanced: AdvancedSettings::default(),
            license_key: String::new(),
            license_validated_at: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// File name
// ---------------------------------------------------------------------------

const CONFIG_FILE: &str = "config.json";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Loads the configuration from `{app_data_dir}/config.json`.
///
/// Returns `AppConfig::default()` if the file does not exist or cannot be
/// parsed. This ensures the application always starts with a valid config.
///
/// Environment variable fallback: if the loaded config has empty API keys
/// and the corresponding env vars are set, they are used as values. This
/// allows `.env`-based development without touching the GUI.
pub fn load_config(app_data_dir: &Path) -> AppConfig {
    let path = app_data_dir.join(CONFIG_FILE);

    let mut config = match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str::<AppConfig>(&contents) {
            Ok(cfg) => cfg,
            Err(e) => {
                log::warn!("[config] Failed to parse config.json ({e}), using defaults");
                AppConfig::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log::info!("[config] config.json not found, using defaults");
            AppConfig::default()
        }
        Err(e) => {
            log::warn!("[config] Failed to read config.json ({e}), using defaults");
            AppConfig::default()
        }
    };

    // Env-var fallback: fill empty keys from process environment.
    // This allows developers to use a `.env` file / shell exports without
    // going through the settings UI.
    if config.groq_api_key.is_empty() {
        if let Ok(key) = std::env::var("GROQ_API_KEY") {
            if !key.is_empty() {
                log::info!("[config] groq_api_key loaded from GROQ_API_KEY env var");
                config.groq_api_key = key;
            }
        }
    }

    if config.deepseek_api_key.is_empty() {
        if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
            if !key.is_empty() {
                log::info!("[config] deepseek_api_key loaded from DEEPSEEK_API_KEY env var");
                config.deepseek_api_key = key;
            }
        }
    }

    if config.openai_api_key.is_empty() {
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            if !key.is_empty() {
                log::info!("[config] openai_api_key loaded from OPENAI_API_KEY env var");
                config.openai_api_key = key;
            }
        }
    }

    if config.anthropic_api_key.is_empty() {
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            if !key.is_empty() {
                log::info!("[config] anthropic_api_key loaded from ANTHROPIC_API_KEY env var");
                config.anthropic_api_key = key;
            }
        }
    }

    if config.turso_url.is_empty() {
        if let Ok(url) = std::env::var("TURSO_URL") {
            if !url.is_empty() {
                log::info!("[config] turso_url loaded from TURSO_URL env var");
                config.turso_url = url;
            }
        }
    }

    if config.turso_token.is_empty() {
        if let Ok(token) = std::env::var("TURSO_TOKEN") {
            if !token.is_empty() {
                log::info!("[config] turso_token loaded from TURSO_TOKEN env var");
                config.turso_token = token;
            }
        }
    }

    config
}

/// Returns `true` if the on-disk config.json contains a `licenseKey` field.
///
/// This is used for the early-adopter migration: existing users whose config
/// predates the license system will not have that field, and we grant them
/// a 60-day grace period automatically.
///
/// Returns `false` if the file does not exist, cannot be read, or lacks the
/// field.
pub fn config_file_has_license_field(app_data_dir: &Path) -> bool {
    let path = app_data_dir.join(CONFIG_FILE);
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            match serde_json::from_str::<serde_json::Value>(&contents) {
                Ok(v) => v.get("licenseKey").is_some(),
                Err(_) => false,
            }
        }
        Err(_) => false,
    }
}

/// Saves the configuration to `{app_data_dir}/config.json`.
///
/// Creates the directory if it does not exist.
///
/// # Errors
/// Returns an error if the directory cannot be created, the file cannot be
/// written, or serialization fails.
pub fn save_config(app_data_dir: &Path, config: &AppConfig) -> anyhow::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;

    let path = app_data_dir.join(CONFIG_FILE);
    let contents = serde_json::to_string_pretty(config)?;

    std::fs::write(&path, contents)?;

    log::debug!("[config] Saved config to {}", path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn temp_dir() -> TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    /// Default config has the expected field values.
    #[test]
    fn test_default_config_values() {
        let cfg = AppConfig::default();
        assert!(cfg.groq_api_key.is_empty());
        assert!(cfg.deepseek_api_key.is_empty());
        assert!(cfg.openai_api_key.is_empty());
        assert!(cfg.anthropic_api_key.is_empty());
        assert!(cfg.language.is_empty(), "default language should be empty (auto-detect)");
        assert_eq!(cfg.cleanup_style, CleanupStyle::Polished);
        assert_eq!(cfg.hotkey, "ctrl+shift+d");
        assert_eq!(cfg.hotkey_mode, HotkeyMode::Hold);
        assert_eq!(cfg.stt_priority, vec!["groq", "openai"]);
        assert_eq!(cfg.llm_priority, vec!["deepseek", "openai", "anthropic", "groq"]);
    }

    /// `HotkeyMode` serializes with lowercase variant names.
    #[test]
    fn test_hotkey_mode_serializes_lowercase() {
        let toggle = serde_json::to_string(&HotkeyMode::Toggle).unwrap();
        let hold = serde_json::to_string(&HotkeyMode::Hold).unwrap();
        assert_eq!(toggle, r#""toggle""#);
        assert_eq!(hold, r#""hold""#);
    }

    /// `HotkeyMode` deserializes from lowercase strings.
    #[test]
    fn test_hotkey_mode_deserializes_lowercase() {
        let toggle: HotkeyMode = serde_json::from_str(r#""toggle""#).unwrap();
        let hold: HotkeyMode = serde_json::from_str(r#""hold""#).unwrap();
        assert_eq!(toggle, HotkeyMode::Toggle);
        assert_eq!(hold, HotkeyMode::Hold);
    }

    /// Default `HotkeyMode` is `Hold`.
    #[test]
    fn test_hotkey_mode_default_is_hold() {
        assert_eq!(HotkeyMode::default(), HotkeyMode::Hold);
        assert_eq!(default_hotkey_mode(), HotkeyMode::Hold);
    }

    /// Loading from a non-existent directory returns defaults without panicking.
    #[test]
    fn test_load_config_missing_file_returns_defaults() {
        let dir = temp_dir();
        let cfg = load_config(dir.path());
        // device_id is a random UUID, so compare everything except that field.
        let mut expected = AppConfig::default();
        expected.device_id = cfg.device_id.clone();
        assert_eq!(cfg, expected);
    }

    /// Save then load round-trips the config correctly.
    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = temp_dir();

        let original = AppConfig {
            groq_api_key: "groq-test-key-abc".to_string(),
            deepseek_api_key: "ds-test-key-xyz".to_string(),
            openai_api_key: "sk-openai-test".to_string(),
            anthropic_api_key: "sk-ant-test".to_string(),
            stt_priority: vec!["openai".to_string(), "groq".to_string()],
            llm_priority: vec!["anthropic".to_string(), "openai".to_string()],
            language: "en".to_string(),
            cleanup_style: CleanupStyle::Chat,
            hotkey: "ctrl+alt+r".to_string(),
            hotkey_mode: HotkeyMode::Toggle,
            audio_device: Some("Test Mic".to_string()),
            stt_model: "whisper-large-v3".to_string(),
            custom_prompt: "Always use formal language.".to_string(),
            profiles: vec![AppProfile {
                name: "Terminal".to_string(),
                app_pattern: "powershell".to_string(),
                cleanup_style: CleanupStyle::Verbatim,
                language: "en".to_string(),
                custom_prompt: "No extra punctuation.".to_string(),
            }],
            autostart: true,
            whisper_mode: false,
            command_hotkey: "ctrl+shift+e".to_string(),
            output_language: "en".to_string(),
            snippets: vec![TextSnippet {
                name: "sig".to_string(),
                content: "Best regards,\nAndy".to_string(),
            }],
            voice_notes_hotkey: "ctrl+shift+n".to_string(),
            webhook_url: "https://example.com/webhook".to_string(),
            advanced: AdvancedSettings {
                stt_prompt_de: "Custom German prompt.".to_string(),
                stt_temperature: 0.2,
                llm_max_tokens: 2048,
                silence_threshold: 0.01,
                auto_paste: false,
                ..AdvancedSettings::default()
            },
            turso_url: String::new(),
            turso_token: String::new(),
            device_id: "test-device".to_string(),
            bubble_size: 1.0,
            bubble_opacity: 0.85,
            license_key: String::new(),
            license_validated_at: 0,
        };

        save_config(dir.path(), &original).expect("save should succeed");

        let loaded = load_config(dir.path());
        assert_eq!(loaded, original);
    }

    /// Both `HotkeyMode` variants survive a save/load round-trip.
    #[test]
    fn test_hotkey_mode_roundtrip() {
        for mode in [HotkeyMode::Toggle, HotkeyMode::Hold] {
            let dir = temp_dir();
            let cfg = AppConfig {
                hotkey_mode: mode,
                ..AppConfig::default()
            };
            save_config(dir.path(), &cfg).unwrap();
            let loaded = load_config(dir.path());
            assert_eq!(loaded.hotkey_mode, mode);
        }
    }

    /// Save creates the directory if it doesn't exist yet.
    #[test]
    fn test_save_creates_directory() {
        let dir = temp_dir();
        let nested = dir.path().join("nested").join("app_data");

        save_config(&nested, &AppConfig::default()).expect("save into nested dir should succeed");

        assert!(nested.join("config.json").exists());
    }

    /// A corrupt config.json falls back to defaults without panicking.
    #[test]
    fn test_load_corrupt_file_returns_defaults() {
        let dir = temp_dir();
        fs::write(dir.path().join("config.json"), b"not valid json!!!").unwrap();

        let cfg = load_config(dir.path());
        // Should not panic; returns defaults.
        assert!(cfg.language.is_empty(), "default language should be empty (auto-detect)");
    }

    /// Partial JSON (missing some fields) uses `serde` defaults for those fields.
    #[test]
    fn test_load_partial_json_fills_in_defaults() {
        let dir = temp_dir();
        // Only language is set; other fields should take their defaults.
        let partial = r#"{"language": "en"}"#;
        fs::write(dir.path().join("config.json"), partial.as_bytes()).unwrap();

        let cfg = load_config(dir.path());
        assert_eq!(cfg.language, "en");
        assert!(cfg.groq_api_key.is_empty());
        assert_eq!(cfg.cleanup_style, CleanupStyle::Polished);
        assert_eq!(cfg.hotkey, "ctrl+shift+d");
    }

    /// `AppConfig` serializes with camelCase keys (matches frontend expectations).
    #[test]
    fn test_config_serializes_with_camel_case() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("groqApiKey"), "expected camelCase 'groqApiKey'");
        assert!(json.contains("deepseekApiKey"), "expected camelCase 'deepseekApiKey'");
        assert!(json.contains("openaiApiKey"), "expected camelCase 'openaiApiKey'");
        assert!(json.contains("anthropicApiKey"), "expected camelCase 'anthropicApiKey'");
        assert!(json.contains("sttPriority"), "expected camelCase 'sttPriority'");
        assert!(json.contains("llmPriority"), "expected camelCase 'llmPriority'");
        assert!(json.contains("cleanupStyle"), "expected camelCase 'cleanupStyle'");
        assert!(json.contains("hotkeyMode"), "expected camelCase 'hotkeyMode'");
        assert!(json.contains("sttModel"), "expected camelCase 'sttModel'");
        assert!(json.contains("customPrompt"), "expected camelCase 'customPrompt'");
    }

    /// Default STT priority is groq > openai.
    #[test]
    fn test_default_stt_priority() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.stt_priority, vec!["groq", "openai"]);
    }

    /// Default LLM priority is deepseek > openai > anthropic > groq.
    #[test]
    fn test_default_llm_priority() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.llm_priority, vec!["deepseek", "openai", "anthropic", "groq"]);
    }

    /// Priority lists round-trip through save/load.
    #[test]
    fn test_priority_lists_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            stt_priority: vec!["openai".to_string()],
            llm_priority: vec!["anthropic".to_string(), "deepseek".to_string()],
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).unwrap();
        let loaded = load_config(dir.path());
        assert_eq!(loaded.stt_priority, cfg.stt_priority);
        assert_eq!(loaded.llm_priority, cfg.llm_priority);
    }

    /// Partial JSON with no priority fields fills in defaults.
    #[test]
    fn test_partial_json_fills_priority_defaults() {
        let dir = temp_dir();
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(cfg.stt_priority, vec!["groq", "openai"]);
        assert_eq!(cfg.llm_priority, vec!["deepseek", "openai", "anthropic", "groq"]);
    }

    /// Default STT model is whisper-large-v3-turbo.
    #[test]
    fn test_default_stt_model() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.stt_model, "whisper-large-v3-turbo");
    }

    /// Default custom_prompt is empty.
    #[test]
    fn test_default_custom_prompt_is_empty() {
        let cfg = AppConfig::default();
        assert!(cfg.custom_prompt.is_empty());
    }

    /// Default profiles list is empty.
    #[test]
    fn test_default_profiles_is_empty() {
        let cfg = AppConfig::default();
        assert!(cfg.profiles.is_empty());
    }

    /// Default autostart is false.
    #[test]
    fn test_default_autostart_is_false() {
        let cfg = AppConfig::default();
        assert!(!cfg.autostart);
    }

    /// AppProfile serializes with camelCase keys.
    #[test]
    fn test_app_profile_serializes_with_camel_case() {
        let profile = AppProfile {
            name: "Test".to_string(),
            app_pattern: "chrome".to_string(),
            cleanup_style: CleanupStyle::Chat,
            language: "en".to_string(),
            custom_prompt: "Be brief.".to_string(),
        };
        let json = serde_json::to_string(&profile).unwrap();
        assert!(json.contains("appPattern"), "expected camelCase 'appPattern'");
        assert!(json.contains("cleanupStyle"), "expected camelCase 'cleanupStyle'");
        assert!(json.contains("customPrompt"), "expected camelCase 'customPrompt'");
    }

    /// AppProfile round-trips through save/load.
    #[test]
    fn test_profiles_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            profiles: vec![
                AppProfile {
                    name: "Browser".to_string(),
                    app_pattern: "chrome".to_string(),
                    cleanup_style: CleanupStyle::Chat,
                    language: "en".to_string(),
                    custom_prompt: String::new(),
                },
                AppProfile {
                    name: "Terminal".to_string(),
                    app_pattern: "powershell".to_string(),
                    cleanup_style: CleanupStyle::Verbatim,
                    language: "de".to_string(),
                    custom_prompt: "No punctuation.".to_string(),
                },
            ],
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).unwrap();
        let loaded = load_config(dir.path());
        assert_eq!(loaded.profiles, cfg.profiles);
    }

    /// Default output_language is empty (no translation).
    #[test]
    fn test_default_output_language_is_empty() {
        let cfg = AppConfig::default();
        assert!(cfg.output_language.is_empty(), "default output_language should be empty (no translation)");
    }

    /// default_output_language() returns an empty string.
    #[test]
    fn test_default_output_language_fn_returns_empty() {
        assert!(default_output_language().is_empty());
    }

    /// output_language round-trips through save/load.
    #[test]
    fn test_output_language_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            output_language: "en".to_string(),
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).unwrap();
        let loaded = load_config(dir.path());
        assert_eq!(loaded.output_language, "en");
    }

    /// Config serializes output_language with camelCase.
    #[test]
    fn test_output_language_serializes_camel_case() {
        let cfg = AppConfig {
            output_language: "fr".to_string(),
            ..AppConfig::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("outputLanguage"), "expected camelCase 'outputLanguage'");
    }

    /// Partial JSON without output_language uses empty string default.
    #[test]
    fn test_partial_json_output_language_defaults_to_empty() {
        let dir = temp_dir();
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert!(cfg.output_language.is_empty());
    }

    /// Default snippets list is empty.
    #[test]
    fn test_default_snippets_is_empty() {
        let cfg = AppConfig::default();
        assert!(cfg.snippets.is_empty(), "default snippets should be an empty Vec");
    }

    /// Snippets round-trip through save/load without data loss.
    #[test]
    fn test_snippets_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            snippets: vec![
                TextSnippet {
                    name: "sig".to_string(),
                    content: "Best regards,\nAndy".to_string(),
                },
                TextSnippet {
                    name: "addr".to_string(),
                    content: "Musterstraße 1, 12345 Berlin".to_string(),
                },
            ],
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).expect("save should succeed");
        let loaded = load_config(dir.path());
        assert_eq!(loaded.snippets, cfg.snippets);
    }

    /// Partial JSON without snippets field deserializes to an empty Vec.
    #[test]
    fn test_partial_json_snippets_defaults_to_empty() {
        let dir = temp_dir();
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert!(cfg.snippets.is_empty(), "missing snippets field should default to empty Vec");
    }

    /// TextSnippet serializes with camelCase keys.
    #[test]
    fn test_text_snippet_serializes_camel_case() {
        let snippet = TextSnippet {
            name: "greeting".to_string(),
            content: "Hello!".to_string(),
        };
        let json = serde_json::to_string(&snippet).unwrap();
        assert!(json.contains("\"name\""), "expected 'name' key");
        assert!(json.contains("\"content\""), "expected 'content' key");
    }

    /// All three CleanupStyle variants round-trip through config serialization.
    #[test]
    fn test_cleanup_style_roundtrip() {
        for style in [CleanupStyle::Polished, CleanupStyle::Verbatim, CleanupStyle::Chat] {
            let dir = temp_dir();
            let cfg = AppConfig {
                cleanup_style: style,
                ..AppConfig::default()
            };
            save_config(dir.path(), &cfg).unwrap();
            let loaded = load_config(dir.path());
            assert_eq!(loaded.cleanup_style, style);
        }
    }

    /// Default webhook_url is empty (disabled).
    #[test]
    fn test_default_webhook_url_is_empty() {
        let cfg = AppConfig::default();
        assert!(cfg.webhook_url.is_empty(), "default webhook_url should be empty (disabled)");
    }

    /// webhook_url round-trips through save/load.
    #[test]
    fn test_webhook_url_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            webhook_url: "https://hooks.example.com/dikta".to_string(),
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).unwrap();
        let loaded = load_config(dir.path());
        assert_eq!(loaded.webhook_url, "https://hooks.example.com/dikta");
    }

    /// Partial JSON without webhook_url defaults to empty string.
    #[test]
    fn test_partial_json_webhook_url_defaults_to_empty() {
        let dir = temp_dir();
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert!(cfg.webhook_url.is_empty(), "missing webhookUrl should default to empty");
    }

    /// Config serializes webhook_url with camelCase key.
    #[test]
    fn test_webhook_url_serializes_camel_case() {
        let cfg = AppConfig {
            webhook_url: "https://example.com/wh".to_string(),
            ..AppConfig::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("webhookUrl"), "expected camelCase 'webhookUrl'");
    }

    // --- AdvancedSettings tests ---

    /// Default AdvancedSettings has the expected values for key fields.
    #[test]
    fn test_advanced_settings_defaults() {
        let adv = AdvancedSettings::default();
        // STT defaults match the hardcoded hints in stt/mod.rs
        assert!(adv.stt_prompt_de.contains("Deutsch"));
        assert!(adv.stt_prompt_en.contains("English"));
        assert!(adv.stt_prompt_auto.contains("Multilingual"));
        assert_eq!(adv.stt_temperature, 0.0);
        // LLM defaults
        assert_eq!(adv.llm_temperature, 0.0);
        assert_eq!(adv.llm_max_tokens, 4096);
        assert!(adv.llm_system_prompt_polished.is_empty());
        assert!(adv.llm_system_prompt_verbatim.is_empty());
        assert!(adv.llm_system_prompt_chat.is_empty());
        assert!(adv.llm_command_mode_prompt.is_empty());
        assert!(adv.llm_model_deepseek.is_empty());
        assert!(adv.llm_model_openai.is_empty());
        assert!(adv.llm_model_anthropic.is_empty());
        assert!(adv.llm_model_groq.is_empty());
        // Chunking
        assert_eq!(adv.chunk_threshold, 800);
        assert_eq!(adv.chunk_target_size, 600);
        // Audio
        assert_eq!(adv.silence_threshold, 0.005);
        assert_eq!(adv.whisper_mode_threshold, 0.001);
        assert_eq!(adv.min_recording_ms, 500);
        assert_eq!(adv.whisper_mode_gain, 3.0);
        // Paste
        assert!(adv.auto_paste);
        assert_eq!(adv.paste_delay_ms, 50);
        assert!(adv.auto_capitalize);
        // Webhook
        assert!(adv.webhook_headers.is_empty());
        assert_eq!(adv.webhook_timeout_secs, 10);
        // System
        assert_eq!(adv.log_level, "info");
    }

    /// AdvancedSettings serializes with camelCase keys.
    #[test]
    fn test_advanced_settings_camel_case() {
        let adv = AdvancedSettings::default();
        let json = serde_json::to_string(&adv).unwrap();
        assert!(json.contains("sttPromptDe"), "expected camelCase 'sttPromptDe'");
        assert!(json.contains("sttPromptEn"), "expected camelCase 'sttPromptEn'");
        assert!(json.contains("sttPromptAuto"), "expected camelCase 'sttPromptAuto'");
        assert!(json.contains("sttTemperature"), "expected camelCase 'sttTemperature'");
        assert!(json.contains("llmSystemPromptPolished"), "expected camelCase 'llmSystemPromptPolished'");
        assert!(json.contains("llmSystemPromptVerbatim"), "expected camelCase 'llmSystemPromptVerbatim'");
        assert!(json.contains("llmSystemPromptChat"), "expected camelCase 'llmSystemPromptChat'");
        assert!(json.contains("llmCommandModePrompt"), "expected camelCase 'llmCommandModePrompt'");
        assert!(json.contains("llmTemperature"), "expected camelCase 'llmTemperature'");
        assert!(json.contains("llmMaxTokens"), "expected camelCase 'llmMaxTokens'");
        assert!(json.contains("llmModelDeepseek"), "expected camelCase 'llmModelDeepseek'");
        assert!(json.contains("chunkThreshold"), "expected camelCase 'chunkThreshold'");
        assert!(json.contains("chunkTargetSize"), "expected camelCase 'chunkTargetSize'");
        assert!(json.contains("silenceThreshold"), "expected camelCase 'silenceThreshold'");
        assert!(json.contains("whisperModeThreshold"), "expected camelCase 'whisperModeThreshold'");
        assert!(json.contains("minRecordingMs"), "expected camelCase 'minRecordingMs'");
        assert!(json.contains("whisperModeGain"), "expected camelCase 'whisperModeGain'");
        assert!(json.contains("autoPaste"), "expected camelCase 'autoPaste'");
        assert!(json.contains("pasteDelayMs"), "expected camelCase 'pasteDelayMs'");
        assert!(json.contains("autoCapitalize"), "expected camelCase 'autoCapitalize'");
        assert!(json.contains("webhookHeaders"), "expected camelCase 'webhookHeaders'");
        assert!(json.contains("webhookTimeoutSecs"), "expected camelCase 'webhookTimeoutSecs'");
        assert!(json.contains("logLevel"), "expected camelCase 'logLevel'");
    }

    /// AdvancedSettings round-trips through save/load.
    #[test]
    fn test_advanced_settings_roundtrip() {
        let dir = temp_dir();
        let adv = AdvancedSettings {
            stt_prompt_de: "Benutzerdefinierter Prompt.".to_string(),
            stt_temperature: 0.3,
            llm_temperature: 0.5,
            llm_max_tokens: 2048,
            llm_system_prompt_polished: "Custom polished prompt.".to_string(),
            llm_model_deepseek: "deepseek-reasoner".to_string(),
            chunk_threshold: 1000,
            chunk_target_size: 800,
            silence_threshold: 0.01,
            whisper_mode_threshold: 0.002,
            min_recording_ms: 300,
            whisper_mode_gain: 5.0,
            auto_paste: false,
            paste_delay_ms: 100,
            auto_capitalize: false,
            webhook_headers: r#"{"X-API-Key": "secret"}"#.to_string(),
            webhook_timeout_secs: 30,
            log_level: "debug".to_string(),
            ..AdvancedSettings::default()
        };
        let cfg = AppConfig {
            advanced: adv.clone(),
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).expect("save should succeed");
        let loaded = load_config(dir.path());
        assert_eq!(loaded.advanced, adv);
    }

    /// Partial JSON without an `advanced` field deserializes to AdvancedSettings::default().
    #[test]
    fn test_partial_json_without_advanced_uses_defaults() {
        let dir = temp_dir();
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(cfg.advanced, AdvancedSettings::default(),
            "missing 'advanced' field should deserialize to AdvancedSettings::default()");
    }

    /// AppConfig serializes the `advanced` field with camelCase key.
    #[test]
    fn test_app_config_includes_advanced_field() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"advanced\""), "AppConfig should serialize an 'advanced' key");
    }
}
