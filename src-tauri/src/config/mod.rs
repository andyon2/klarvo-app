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
/// - `AutoStop`: press once to start; recording stops automatically on silence.
/// - `Auto`: like AutoStop but loops continuously -- press again to exit.
///
/// Default is `Hold` -- this matches the Wispr Flow UX that users expect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum HotkeyMode {
    Toggle,
    #[default]
    Hold,
    AutoStop,
    Auto,
}


impl std::str::FromStr for HotkeyMode {
    type Err = String;

    /// Parses a case-insensitive mode string as produced by the frontend:
    /// `"hold"`, `"toggle"`, `"autostop"` / `"autoStop"`, `"auto"`.
    ///
    /// Returns `Err` for unknown strings.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "hold" => Ok(HotkeyMode::Hold),
            "toggle" => Ok(HotkeyMode::Toggle),
            "autostop" => Ok(HotkeyMode::AutoStop),
            "auto" => Ok(HotkeyMode::Auto),
            other => Err(format!("Unknown HotkeyMode: {other:?}")),
        }
    }
}

// ---------------------------------------------------------------------------
// HotkeySlot
// ---------------------------------------------------------------------------

/// One configurable hotkey binding with its own recording mode.
///
/// `AppConfig` holds two slots (`hotkey_slots`). Slot 0 is the primary
/// dictation hotkey; slot 1 is an optional secondary binding (e.g. for a
/// different mode or language). An empty `hotkey` string means the slot is
/// disabled.
///
/// # Migration note
/// The old flat `hotkey` / `hotkey_mode` fields on `AppConfig` are preserved
/// as deprecated fallbacks. `load_config` migrates them into slot 0 when
/// `hotkey_slots` is absent from an old config file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotkeySlot {
    /// Tauri shortcut string (e.g. `"ctrl+shift+d"`). Empty = slot disabled.
    pub hotkey: String,
    /// How this slot triggers recording.
    pub mode: HotkeyMode,
    /// When `true`, the pipeline sends a Return key press after pasting text.
    /// Useful for chat apps where Enter submits the message. Defaults to `false`
    /// so existing configs are unaffected.
    #[serde(default)]
    pub insert_and_send: bool,
}

impl HotkeySlot {
    /// Returns `true` if this slot has a non-empty hotkey string.
    pub fn is_enabled(&self) -> bool {
        !self.hotkey.is_empty()
    }
}

fn default_hotkey_slots() -> Vec<HotkeySlot> {
    vec![
        HotkeySlot {
            hotkey: default_hotkey(),
            mode: HotkeyMode::Hold,
            insert_and_send: false,
        },
        HotkeySlot {
            hotkey: String::new(), // slot 2 disabled by default
            mode: HotkeyMode::Hold,
            insert_and_send: false,
        },
    ]
}

// ---------------------------------------------------------------------------
// Configuration struct
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// OnboardingState
// ---------------------------------------------------------------------------

/// Tracks the user's progress through the first-run onboarding wizard.
///
/// Persisted as part of `AppConfig` so the wizard survives app restarts.
/// All fields default to "not started" via `Default`.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingState {
    /// `true` when the user has completed the wizard successfully.
    pub completed: bool,
    /// `true` when the user explicitly clicked "Skip" on the wizard.
    pub skipped: bool,
    /// The last step the user reached. 0 = not started.
    pub current_step: u8,
    /// Chosen operating mode. One of `"cloud"`, `"offline"`, or `""` (not chosen yet).
    pub mode: String,
    /// Chosen input language. ISO-639-1 code (e.g. `"de"`, `"en"`).
    /// Empty string = not chosen yet.
    pub language: String,
    /// Chosen onboarding track for the STT key setup.
    /// One of `"expert"`, `"beginner"`, or `""` (not chosen yet).
    #[serde(default)]
    pub track: String,
}

// ---------------------------------------------------------------------------
// AppConfig
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

    /// OpenRouter API key (OpenAI-compatible gateway for multi-provider routing).
    #[serde(default, rename = "openrouterApiKey")]
    pub openrouter_api_key: String,

    /// Selected STT provider.
    /// Valid values: `"groq"`, `"openai"`, `"local"`.
    /// Default: `"groq"`.
    #[serde(default = "default_stt_provider")]
    pub stt_provider: String,

    /// Selected LLM cleanup provider.
    /// Valid values: `"deepseek"`, `"openai"`, `"anthropic"`, `"groq"`.
    /// Default: `"deepseek"`.
    #[serde(default = "default_llm_provider")]
    pub llm_provider: String,

    // deprecated, ignored -- kept for config.json backwards compat
    #[serde(default)]
    pub stt_priority: Vec<String>,

    // deprecated, ignored -- kept for config.json backwards compat
    #[serde(default)]
    pub llm_priority: Vec<String>,

    /// ISO-639-1 language code used for transcription (e.g. `"de"`, `"en"`).
    /// Empty string = auto-detect.
    #[serde(default = "default_language")]
    pub language: String,

    /// Cleanup aggressiveness for the LLM step.
    #[serde(default = "default_cleanup_style")]
    pub cleanup_style: CleanupStyle,

    /// Global hotkey string in Tauri shortcut format (e.g. `"ctrl+shift+d"`).
    ///
    /// Deprecated: superseded by `hotkey_slots`. Kept as a migration fallback
    /// so that old config.json files load without data loss. New code should
    /// read from `hotkey_slots[0]` instead.
    #[serde(default = "default_hotkey")]
    pub hotkey: String,

    /// How the hotkey triggers recording: toggle (press/press) or hold (hold/release).
    ///
    /// Deprecated: superseded by `hotkey_slots`. Kept as a migration fallback.
    /// New code should read `hotkey_slots[0].mode` instead.
    #[serde(default = "default_hotkey_mode")]
    pub hotkey_mode: HotkeyMode,

    /// Dual hotkey slots. Each slot has its own key binding and recording mode.
    ///
    /// - Slot 0 (`hotkey_slots[0]`): primary dictation hotkey.
    /// - Slot 1 (`hotkey_slots[1]`): optional secondary binding. An empty
    ///   `hotkey` string means the slot is disabled.
    ///
    /// When absent from an old config.json the migration in `load_config`
    /// populates slot 0 from the deprecated `hotkey` / `hotkey_mode` fields
    /// (or from the compiled-in defaults) and sets slot 1 to disabled.
    #[serde(default)]
    pub hotkey_slots: Vec<HotkeySlot>,

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

    /// Launch Klarvo automatically when the user logs in.
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

    // --- Local Whisper (offline STT) ---

    /// GGML model variant for offline transcription.
    ///
    /// The provider resolves the actual file path as:
    /// `{app_data_dir}/models/ggml-{local_whisper_model}.bin`
    ///
    /// Supported values: `"tiny"`, `"tiny-q5_1"`, `"base"` (default),
    /// `"base-q5_1"`, `"small"`, etc. See the whisper.cpp model list.
    #[serde(default = "default_local_whisper_model")]
    pub local_whisper_model: String,

    /// Whether to enable GPU acceleration (CUDA) for local whisper inference.
    ///
    /// Has no effect unless the binary was compiled with the `cuda` feature.
    /// Default is `true` so users with a GPU benefit automatically once they
    /// enable the CUDA build.
    #[serde(default = "default_local_whisper_gpu")]
    pub local_whisper_gpu: bool,

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

    /// Which validation path was used: `"hmac"` or `"lemon_squeezy"`.
    /// Empty string = not yet validated.
    #[serde(default)]
    pub license_source: String,

    /// Instance ID (UUID) returned by Lemon Squeezy on activation.
    /// Needed for deactivation and re-validation.
    /// Empty string = not activated via Lemon Squeezy.
    #[serde(default)]
    pub ls_instance_id: String,

    /// Unix timestamp (seconds) when the Lemon Squeezy license was last
    /// validated online. 0 = never re-validated after initial activation.
    #[serde(default)]
    pub ls_last_validated_at: u64,

    // --- Floating bar position ---

    /// Deprecated: superseded by `HotkeySlot::insert_and_send`.
    ///
    /// Kept as a migration tombstone so that old config.json files load
    /// without data loss. `load_config` propagates this value to all slots
    /// when the slots do not yet carry their own `insertAndSend` flag
    /// (i.e. when the slot was serialised by an older binary).
    ///
    /// New code must NOT write to this field -- write to the slot instead.
    #[serde(default)]
    pub insert_and_send: bool,

    /// Silence duration (seconds) before AutoStop mode triggers stop + pipeline.
    /// Default: 2.0 seconds.
    #[serde(default = "default_autostop_silence_secs")]
    pub autostop_silence_secs: f32,

    /// Silence duration (seconds) before Auto mode triggers stop + pipeline
    /// (and then restarts listening). Default: 2.0 seconds.
    #[serde(default = "default_auto_mode_silence_secs")]
    pub auto_mode_silence_secs: f32,

    /// Last saved X position of the floating bar window (logical pixels).
    /// `None` = no saved position; the app will use the default placement
    /// (bottom-center of the primary monitor above the taskbar).
    #[serde(default)]
    pub bar_x: Option<f64>,

    /// Last saved Y position of the floating bar window (logical pixels).
    /// `None` = no saved position; paired with `bar_x` -- both are set or
    /// neither is set.
    #[serde(default)]
    pub bar_y: Option<f64>,

    /// Recording mode for the Android floating bubble.
    ///
    /// Valid values: `"hold"`, `"toggle"`, `"autostop"`, `"auto"`.
    /// Default: `"hold"`.
    ///
    /// Kotlin reads this field directly from config.json, so it must remain a
    /// plain String (not a Rust enum) to avoid deserialization coupling between
    /// the two runtimes.
    ///
    /// Note: the desktop equivalent is `HotkeyMode` inside `hotkey_slots`.
    /// This field is Android-only; desktop code should ignore it.
    #[serde(default = "default_bubble_recording_mode")]
    pub bubble_recording_mode: String,

    // --- Android bubble per-gesture controls ---
    //
    // The six fields below replace the single `bubble_recording_mode` field
    // with per-gesture configuration. `bubble_recording_mode` is kept for
    // backwards compatibility with existing config files and Kotlin code that
    // has not yet been updated to read the new fields.

    /// Recording mode triggered by a single tap on the Android bubble.
    /// Valid values: `"hold"`, `"toggle"`, `"autostop"`, `"auto"`.
    /// Default: `"toggle"`.
    #[serde(default = "default_bubble_tap_mode")]
    pub bubble_tap_mode: String,

    /// When `true`, the pipeline automatically sends (presses Enter) after
    /// pasting for the bubble tap gesture. Default: `false`.
    #[serde(default)]
    pub bubble_tap_auto_send: bool,

    /// Silence duration (seconds) before AutoStop / Auto mode stops recording
    /// when triggered by a bubble tap. Default: 2.0 seconds.
    #[serde(default = "default_bubble_silence_secs")]
    pub bubble_tap_silence_secs: f32,

    /// Recording mode triggered by a long press on the Android bubble.
    /// Valid values: `"hold"`, `"toggle"`, `"autostop"`, `"auto"`.
    /// Default: `"hold"`.
    #[serde(default = "default_bubble_long_press_mode")]
    pub bubble_long_press_mode: String,

    /// When `true`, the pipeline automatically sends (presses Enter) after
    /// pasting for the bubble long-press gesture. Default: `false`.
    #[serde(default)]
    pub bubble_long_press_auto_send: bool,

    /// Silence duration (seconds) before AutoStop / Auto mode stops recording
    /// when triggered by a bubble long press. Default: 2.0 seconds.
    #[serde(default = "default_bubble_silence_secs")]
    pub bubble_long_press_silence_secs: f32,

    /// Onboarding wizard state.
    ///
    /// Tracks whether the user has completed, skipped, or is partway through
    /// the first-run setup wizard. Uses `#[serde(default)]` so old config
    /// files (without this field) load correctly -- they get `OnboardingState::default()`.
    #[serde(default)]
    pub onboarding: OnboardingState,

    /// Whether the Voice Command Mode monitor is enabled (user preference).
    ///
    /// This is the persisted preference -- "does the user want voice commands
    /// to be active?" -- distinct from `AppState::voice_command_active` which
    /// reflects the live runtime state.
    ///
    /// When `true` on startup the monitor is automatically started.
    /// Default: `false` (opt-in feature).
    #[serde(default)]
    pub voice_command_enabled: bool,

    /// Unix timestamp (seconds) of the very first app launch.
    ///
    /// Written once on first run and never overwritten. Used to compute the
    /// 14-day trial window. 0 = not yet recorded (pre-trial builds or corrupt
    /// config); treated as Unlicensed.
    #[serde(default, rename = "firstInstallAt")]
    pub first_install_at: u64,

    /// Webhook URL for in-app feedback submissions.
    ///
    /// When non-empty, the `send_feedback` command POSTs a JSON payload to
    /// this URL. Empty string = feedback feature disabled (default).
    ///
    /// This is an operator-controlled field: it is set in `config.json` by
    /// whoever deploys/distributes Klarvo. End users do not see or change it
    /// through the Settings UI.
    #[serde(default, rename = "feedbackWebhookUrl")]
    pub feedback_webhook_url: String,
}

fn default_stt_provider() -> String {
    "groq".to_string()
}

fn default_llm_provider() -> String {
    "deepseek".to_string()
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

fn default_local_whisper_model() -> String {
    "tiny-german-1224-q8_0".to_string()
}

fn default_local_whisper_gpu() -> bool {
    true
}

fn default_bubble_size() -> f32 {
    1.0
}

fn default_bubble_opacity() -> f32 {
    0.85
}

fn default_autostop_silence_secs() -> f32 {
    2.0
}

fn default_auto_mode_silence_secs() -> f32 {
    2.0
}

fn default_bubble_recording_mode() -> String {
    "hold".to_string()
}

fn default_bubble_tap_mode() -> String {
    "toggle".to_string()
}

fn default_bubble_long_press_mode() -> String {
    "hold".to_string()
}

/// Shared default silence duration (seconds) for bubble gesture auto-stop.
/// Used by both `bubble_tap_silence_secs` and `bubble_long_press_silence_secs`.
fn default_bubble_silence_secs() -> f32 {
    2.0
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            groq_api_key: String::new(),
            deepseek_api_key: String::new(),
            openai_api_key: String::new(),
            anthropic_api_key: String::new(),
            openrouter_api_key: String::new(),
            stt_provider: default_stt_provider(),
            llm_provider: default_llm_provider(),
            stt_priority: Vec::new(),
            llm_priority: Vec::new(),
            language: default_language(),
            cleanup_style: default_cleanup_style(),
            hotkey: default_hotkey(),
            hotkey_mode: default_hotkey_mode(),
            hotkey_slots: default_hotkey_slots(),
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
            local_whisper_model: default_local_whisper_model(),
            local_whisper_gpu: default_local_whisper_gpu(),
            license_key: String::new(),
            license_validated_at: 0,
            license_source: String::new(),
            ls_instance_id: String::new(),
            ls_last_validated_at: 0,
            insert_and_send: false,
            autostop_silence_secs: default_autostop_silence_secs(),
            auto_mode_silence_secs: default_auto_mode_silence_secs(),
            bar_x: None,
            bar_y: None,
            bubble_recording_mode: default_bubble_recording_mode(),
            bubble_tap_mode: default_bubble_tap_mode(),
            bubble_tap_auto_send: false,
            bubble_tap_silence_secs: default_bubble_silence_secs(),
            bubble_long_press_mode: default_bubble_long_press_mode(),
            bubble_long_press_auto_send: false,
            bubble_long_press_silence_secs: default_bubble_silence_secs(),
            onboarding: OnboardingState::default(),
            voice_command_enabled: false,
            first_install_at: 0,
            feedback_webhook_url: String::new(),
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

/// Best-effort backup of a corrupt or unreadable `config.json` before
/// `load_config` falls back to defaults (ROB-02 / ADR-0015).
///
/// Copies the raw on-disk bytes (`raw`) to a uniquely timestamped sibling
/// `config.json.corrupt-<unix_ts>` via the shared atomic writer
/// (`crate::fs::save_atomic`), so the user's keys/license/snippets stay
/// recoverable instead of being silently overwritten on the next boot — the
/// first-install guard in `lib.rs` would otherwise persist a fresh default over
/// the corrupt file. The timestamped name (built by string, NOT
/// `with_extension`) guarantees a prior backup is never overwritten.
///
/// Infallible by contract: a failed backup write is logged and downgraded to a
/// warning — it must never block application startup. A human-readable warning
/// is pushed onto `warnings` for the shell to surface (see D1 in the story).
fn backup_corrupt_config(path: &Path, raw: &[u8], warnings: &mut Vec<String>) {
    // Same unix-seconds pattern as the first-install stamp in lib.rs.
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_name = format!("{CONFIG_FILE}.corrupt-{ts}");
    let backup_path = match path.parent() {
        Some(dir) => dir.join(&backup_name),
        None => std::path::PathBuf::from(&backup_name),
    };

    match crate::fs::save_atomic(&backup_path, raw) {
        Ok(()) => {
            log::warn!("[config] Corrupt config.json backed up to {}", backup_path.display());
            warnings.push(format!(
                "Your settings file was unreadable and has been backed up to {backup_name}. \
                 Settings were reset to defaults — your previous keys/license can be recovered from that file."
            ));
        }
        Err(e) => {
            // Best-effort: the original corrupt file still remains on disk; we just
            // could not duplicate it. Never error out — boot must continue.
            log::warn!(
                "[config] Failed to back up corrupt config.json to {} ({e}); continuing with defaults",
                backup_path.display()
            );
            warnings.push(format!(
                "Your settings file was unreadable and could not be backed up ({e}). Settings were reset to defaults."
            ));
        }
    }
}

/// Writes a best-effort backup of the current on-disk `config.json` to
/// `config.json.pre-migration-<unix_ts>-<migration>` before a schema migration
/// mutates and re-persists the config (ROB-05 / ADR-0015 §4).
///
/// Returns the backup filename on success so callers can embed it in
/// user-facing warning messages. Returns `None` if the backup could not be
/// written (logged at `warn!` but never blocks migration).
///
/// The caller must invoke this BEFORE calling `save_config` for the
/// migration write. The migration name is embedded in the filename to ensure
/// uniqueness when multiple migrations fire on the same boot.
fn backup_pre_migration_config(app_data_dir: &Path, migration_name: &str) -> Option<String> {
    let path = app_data_dir.join(CONFIG_FILE);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let safe_name = migration_name.replace('/', "-");
    let backup_name = format!("{CONFIG_FILE}.pre-migration-{ts}-{safe_name}");
    let backup_path = match path.parent() {
        Some(dir) => dir.join(&backup_name),
        None => app_data_dir.join(&backup_name),
    };
    match std::fs::read(&path) {
        Ok(raw) => match crate::fs::save_atomic(&backup_path, &raw) {
            Ok(()) => {
                log::info!(
                    "[config] Pre-migration backup written to {} (migration: {migration_name})",
                    backup_path.display()
                );
                Some(backup_name)
            }
            Err(e) => {
                log::warn!(
                    "[config] Failed to write pre-migration backup to {} ({e}); \
                     continuing with migration (migration: {migration_name})",
                    backup_path.display()
                );
                None
            }
        },
        Err(e) => {
            log::warn!(
                "[config] Could not read config.json for pre-migration backup ({e}); \
                 continuing with migration (migration: {migration_name})"
            );
            None
        }
    }
}

/// Builds the user-facing warning shown when a migration's `save_config` fails.
///
/// `backup_file` is the filename returned by [`backup_pre_migration_config`]
/// (`None` if the pre-migration backup itself could not be written). The
/// message deliberately says "a backup … before the migration" rather than
/// "your original config": when several migrations chain on a single boot,
/// each backup captures the on-disk state immediately before *that* migration,
/// so only the first backup of the chain is the literal pre-upgrade file.
/// Keys and license survive in every backup regardless — they live in no
/// migrated field — which is what the reassurance refers to.
fn migration_save_warning(
    migration_label: &str,
    error: &impl std::fmt::Display,
    backup_file: Option<String>,
) -> String {
    let location = backup_file
        .map(|f| format!("`{f}` in your app data directory"))
        .unwrap_or_else(|| {
            "your app data directory (look for config.json.pre-migration-* files)".to_string()
        });
    format!(
        "Config migration ({migration_label}) could not be saved: {error}. \
         A backup of your config was saved to {location} before the migration — \
         your keys and license are intact."
    )
}

/// Loads the configuration from `{app_data_dir}/config.json`.
///
/// Returns `AppConfig::default()` if the file does not exist or cannot be
/// parsed. This ensures the application always starts with a valid config.
///
/// Environment variable fallback: if the loaded config has empty API keys
/// and the corresponding env vars are set, they are used as values. This
/// allows `.env`-based development without touching the GUI.
///
/// Thin wrapper over [`load_config_reporting`] for the many call sites (~55
/// tests) that don't consume boot warnings. The single production caller
/// (`lib.rs` setup) uses [`load_config_reporting`] directly to surface them, so
/// in a non-test build this wrapper has no caller by design — hence the
/// `not(test)` dead-code allowance; under `cfg(test)` it is heavily used.
#[cfg_attr(not(test), allow(dead_code))]
pub fn load_config(app_data_dir: &Path) -> AppConfig {
    let mut warnings = Vec::new();
    load_config_reporting(app_data_dir, &mut warnings)
}

/// Reporting variant of [`load_config`]: identical behaviour, but pushes any
/// boot-time warnings (e.g. "a corrupt config was backed up") onto `warnings`
/// so the caller can surface them to the user.
///
/// Kept as a separate entry point so the public `load_config(&Path) ->
/// AppConfig` signature stays intact for existing callers/tests — structural
/// decoupling of this body is fenced to the DEPTH-config work (ADR-0015 §5).
pub fn load_config_reporting(app_data_dir: &Path, warnings: &mut Vec<String>) -> AppConfig {
    let path = app_data_dir.join(CONFIG_FILE);

    let mut config = match std::fs::read_to_string(&path) {
        Ok(contents) => match serde_json::from_str::<AppConfig>(&contents) {
            Ok(cfg) => cfg,
            Err(e) => {
                log::warn!("[config] Failed to parse config.json ({e}), using defaults");
                // Preserve the corrupt file BEFORE returning a default that the
                // first-install guard may persist over it (AC#1/#2, ROB-02).
                // `contents` already holds the file bytes — zero extra read.
                backup_corrupt_config(&path, contents.as_bytes(), warnings);
                AppConfig::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // Absent file is NOT corruption: no backup, no warning (AC#3).
            log::info!("[config] config.json not found, using defaults");
            AppConfig::default()
        }
        Err(e) => {
            log::warn!("[config] Failed to read config.json ({e}), using defaults");
            // Read error (e.g. non-UTF-8 / unreadable) is treated like corruption
            // (AC#4): best-effort backup of the raw on-disk bytes. `read_to_string`
            // failed so there is no `contents` in scope — re-read the raw bytes. If
            // even that fails the file is truly unreadable: log and continue to
            // defaults; never panic, never block boot.
            match std::fs::read(&path) {
                Ok(raw) => backup_corrupt_config(&path, &raw, warnings),
                Err(read_err) => log::warn!(
                    "[config] Could not read raw bytes of unreadable config.json for backup ({read_err}); continuing with defaults"
                ),
            }
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

    // ---------------------------------------------------------------------------
    // Migration: sttPriority / llmPriority → sttProvider / llmProvider
    //
    // In v0.4.2 we renamed the priority-list fields to a single provider string.
    // Old config files still have sttPriority / llmPriority populated but no
    // sttProvider / llmProvider entry, so serde assigns the field defaults
    // ("groq" / "deepseek"). We detect this by checking whether the provider
    // field is still at its default value while the legacy list is non-empty.
    // Only then do we promote the first entry of the old list.
    //
    // Guard: if the user had already written a real sttProvider value (i.e. the
    // field was present in the JSON), serde will have deserialized it to a
    // non-default string and we skip the migration entirely.
    // ---------------------------------------------------------------------------
    let mut migrated = false;

    if config.stt_provider == default_stt_provider() && !config.stt_priority.is_empty() {
        let promoted = config.stt_priority[0].clone();
        log::info!("[config] Migrated legacy sttPriority[0]=\"{promoted}\" to sttProvider");
        config.stt_provider = promoted;
        migrated = true;
    }

    if config.llm_provider == default_llm_provider() && !config.llm_priority.is_empty() {
        let promoted = config.llm_priority[0].clone();
        log::info!("[config] Migrated legacy llmPriority[0]=\"{promoted}\" to llmProvider");
        config.llm_provider = promoted;
        migrated = true;
    }

    if migrated {
        log::info!("[config] Migrated legacy sttPriority/llmPriority to provider fields");
        config.stt_priority.clear();
        config.llm_priority.clear();
        // Persist immediately so the next start is clean and needs no migration.
        let backup_file = backup_pre_migration_config(app_data_dir, "sttPriority/llmPriority");
        if let Err(e) = save_config(app_data_dir, &config) {
            let msg = migration_save_warning("sttPriority/llmPriority", &e, backup_file);
            log::warn!("[config] {msg}");
            warnings.push(msg);
        }
    }

    // ---------------------------------------------------------------------------
    // Migration: hotkey / hotkey_mode → hotkey_slots
    //
    // In the dual-hotkey redesign we replaced the flat `hotkey` / `hotkey_mode`
    // fields with `hotkey_slots: Vec<HotkeySlot>`. Old config files have the
    // flat fields but no `hotkey_slots` key, so serde assigns an empty Vec via
    // the `#[serde(default)]` annotation.
    //
    // When we detect an empty slots list we populate it from the legacy fields:
    //   - Slot 0: hotkey + hotkey_mode (or the compiled-in defaults if those
    //     fields are also missing / empty).
    //   - Slot 1: disabled (empty hotkey string).
    //
    // This is intentionally a one-way migration: once hotkey_slots is written
    // to disk the flat fields are no longer consulted. We do NOT clear the flat
    // fields -- they stay as tombstones so truly old binaries can still read a
    // value from them (forward-compat).
    // ---------------------------------------------------------------------------
    if config.hotkey_slots.is_empty() {
        let slot0_hotkey = if config.hotkey.is_empty() {
            default_hotkey()
        } else {
            config.hotkey.clone()
        };
        let slot0_mode = config.hotkey_mode;

        log::info!(
            "[config] Migrated legacy hotkey=\"{slot0_hotkey}\" mode={slot0_mode:?} to hotkey_slots[0]"
        );

        config.hotkey_slots = vec![
            HotkeySlot {
                hotkey: slot0_hotkey,
                mode: slot0_mode,
                insert_and_send: false,
            },
            HotkeySlot {
                hotkey: String::new(), // slot 1 disabled
                mode: HotkeyMode::Hold,
                insert_and_send: false,
            },
        ];

        // Persist immediately so future starts skip this migration path.
        let backup_file = backup_pre_migration_config(app_data_dir, "hotkey_slots");
        if let Err(e) = save_config(app_data_dir, &config) {
            let msg = migration_save_warning("hotkey_slots", &e, backup_file);
            log::warn!("[config] {msg}");
            warnings.push(msg);
        }
    }

    // ---------------------------------------------------------------------------
    // Migration: global `insert_and_send` → per-slot `insert_and_send`
    //
    // In the per-slot redesign we moved `insert_and_send` from `AppConfig` into
    // each `HotkeySlot`. Old config files may have the global flag set but the
    // slots still have their serde default (`false`).
    //
    // We detect this by checking whether the global flag is `true` while ALL
    // slots still carry the default value (`false`). In that case we propagate
    // the global value to all slots and clear the global flag.
    //
    // If any slot already has `insert_and_send = true` (set by a newer binary),
    // we leave everything as-is to avoid overwriting intentional per-slot config.
    // ---------------------------------------------------------------------------
    if config.insert_and_send && config.hotkey_slots.iter().all(|s| !s.insert_and_send) {
        log::info!(
            "[config] Migrated global insert_and_send=true to {} slot(s)",
            config.hotkey_slots.len()
        );
        for slot in &mut config.hotkey_slots {
            slot.insert_and_send = true;
        }
        // Clear the global flag so we no longer re-trigger this migration.
        config.insert_and_send = false;
        let backup_file = backup_pre_migration_config(app_data_dir, "insert_and_send_per_slot");
        if let Err(e) = save_config(app_data_dir, &config) {
            let msg = migration_save_warning("insert_and_send per-slot", &e, backup_file);
            log::warn!("[config] {msg}");
            warnings.push(msg);
        }
    }

    // ---------------------------------------------------------------------------
    // Validation: reject unknown provider values and fall back to defaults.
    // ---------------------------------------------------------------------------
    const VALID_STT_PROVIDERS: &[&str] = &["groq", "openai", "local"];
    const VALID_LLM_PROVIDERS: &[&str] = &["deepseek", "openai", "anthropic", "groq", "openrouter"];

    if !VALID_STT_PROVIDERS.contains(&config.stt_provider.as_str()) {
        log::warn!(
            "[config] Unknown stt_provider {:?}, falling back to \"groq\"",
            config.stt_provider
        );
        config.stt_provider = default_stt_provider();
    }

    if !VALID_LLM_PROVIDERS.contains(&config.llm_provider.as_str()) {
        log::warn!(
            "[config] Unknown llm_provider {:?}, falling back to \"deepseek\"",
            config.llm_provider
        );
        config.llm_provider = default_llm_provider();
    }

    // ---------------------------------------------------------------------------
    // Groq-Llama Default: when STT provider is Groq and the user has a Groq key
    // but no DeepSeek key, auto-select Groq as LLM provider too.
    //
    // This lets users complete onboarding with a single API key.  Only triggers
    // when llm_provider is still on the default "deepseek" — never overrides an
    // explicit user choice that already has a working key.
    //
    // This block MUST run before the general auto-fallback below, otherwise the
    // fallback would pick up the same Groq key via a different code path and the
    // targeted log message would never appear.
    // ---------------------------------------------------------------------------
    if config.stt_provider == "groq"
        && config.llm_provider == "deepseek"
        && config.deepseek_api_key.is_empty()
        && !config.groq_api_key.is_empty()
    {
        config.llm_provider = "groq".to_string();
        log::info!(
            "[config] STT provider is Groq with API key present, auto-selecting Groq LLM (no DeepSeek key configured)"
        );
    }

    // ---------------------------------------------------------------------------
    // Auto-fallback: if the chosen llm_provider has no API key, switch to the
    // first alternative that does have a key.
    //
    // This avoids a confusing 401 error for users who set up Groq/OpenAI but
    // left DeepSeek (the default) unconfigured.  We only auto-switch when the
    // current provider's key is EMPTY; an explicit choice with a present key is
    // never touched.
    //
    // Preference order for the fallback search: deepseek, openai, groq, anthropic
    // (same as VALID_LLM_PROVIDERS order so the "best" provider wins first).
    // If every key is empty we leave the config as-is -- the user will get a
    // clear error at runtime when they actually trigger cleanup.
    // ---------------------------------------------------------------------------
    let current_key_empty = match config.llm_provider.as_str() {
        "deepseek"    => config.deepseek_api_key.is_empty(),
        "openai"      => config.openai_api_key.is_empty(),
        "anthropic"   => config.anthropic_api_key.is_empty(),
        "groq"        => config.groq_api_key.is_empty(),
        "openrouter"  => config.openrouter_api_key.is_empty(),
        _             => false, // already validated above; unreachable in practice
    };

    if current_key_empty {
        // Walk the preference list and pick the first provider that has a key.
        let candidates: &[(&str, &str)] = &[
            ("deepseek",   &config.deepseek_api_key),
            ("openai",     &config.openai_api_key),
            ("groq",       &config.groq_api_key),
            ("anthropic",  &config.anthropic_api_key),
            ("openrouter", &config.openrouter_api_key),
        ];
        if let Some((name, _)) = candidates
            .iter()
            .find(|(n, k)| *n != config.llm_provider.as_str() && !k.is_empty())
        {
            let old = config.llm_provider.clone();
            config.llm_provider = name.to_string();
            log::info!(
                "[config] llm_provider \"{old}\" has no API key, auto-switching to \"{name}\""
            );
        }
    }

    config
}

/// Saves the configuration to `{app_data_dir}/config.json`.
///
/// Creates the directory if it does not exist.
///
/// # Errors
/// Returns an error if the directory cannot be created, the file cannot be
/// written, or serialization fails.
///
/// Low-level writer reserved for single-threaded boot/migration (before `AppState` exists).
/// All runtime saves MUST go through `AppState::save_config_locked`, which holds the
/// `config_disk_write` lock across the read-modify-write cycle (ROB-04). Calling this directly
/// from a runtime path bypasses that serialization invariant.
pub(crate) fn save_config(app_data_dir: &Path, config: &AppConfig) -> anyhow::Result<()> {
    std::fs::create_dir_all(app_data_dir)?;

    let path = app_data_dir.join(CONFIG_FILE);
    let contents = serde_json::to_string_pretty(config)?;

    crate::fs::save_atomic(&path, contents.as_bytes())?;

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
        assert_eq!(cfg.stt_provider, "groq");
        assert_eq!(cfg.llm_provider, "deepseek");
        // deprecated fields are empty by default
        assert!(cfg.stt_priority.is_empty());
        assert!(cfg.llm_priority.is_empty());
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
            stt_provider: "openai".to_string(),
            llm_provider: "anthropic".to_string(),
            stt_priority: Vec::new(),
            llm_priority: Vec::new(),
            language: "en".to_string(),
            cleanup_style: CleanupStyle::Chat,
            hotkey: "ctrl+alt+r".to_string(),
            hotkey_mode: HotkeyMode::Toggle,
            hotkey_slots: vec![
                HotkeySlot { hotkey: "ctrl+alt+r".to_string(), mode: HotkeyMode::Toggle, insert_and_send: true },
                HotkeySlot { hotkey: String::new(), mode: HotkeyMode::Hold, insert_and_send: true },
            ],
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
            local_whisper_model: "tiny-q5_1".to_string(),
            local_whisper_gpu: false,
            license_key: String::new(),
            license_validated_at: 0,
            license_source: String::new(),
            ls_instance_id: String::new(),
            ls_last_validated_at: 0,
            insert_and_send: true,
            autostop_silence_secs: 1.5,
            auto_mode_silence_secs: 3.0,
            bar_x: Some(123.5),
            bar_y: Some(456.0),
            bubble_recording_mode: "toggle".to_string(),
            bubble_tap_mode: "autostop".to_string(),
            bubble_tap_auto_send: true,
            bubble_tap_silence_secs: 3.0,
            bubble_long_press_mode: "hold".to_string(),
            bubble_long_press_auto_send: false,
            bubble_long_press_silence_secs: 1.5,
            openrouter_api_key: "sk-or-test-key".to_string(),
            onboarding: OnboardingState::default(),
            voice_command_enabled: false,
            first_install_at: 0,
            feedback_webhook_url: "https://example.com/feedback".to_string(),
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
        assert!(json.contains("sttProvider"), "expected camelCase 'sttProvider'");
        assert!(json.contains("llmProvider"), "expected camelCase 'llmProvider'");
        assert!(json.contains("cleanupStyle"), "expected camelCase 'cleanupStyle'");
        assert!(json.contains("hotkeyMode"), "expected camelCase 'hotkeyMode'");
        assert!(json.contains("sttModel"), "expected camelCase 'sttModel'");
        assert!(json.contains("customPrompt"), "expected camelCase 'customPrompt'");
    }

    /// Default stt_provider is "groq".
    #[test]
    fn test_default_stt_provider() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.stt_provider, "groq");
    }

    /// Default llm_provider is "deepseek".
    #[test]
    fn test_default_llm_provider() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.llm_provider, "deepseek");
    }

    /// stt_provider and llm_provider round-trip through save/load.
    #[test]
    fn test_provider_fields_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            stt_provider: "openai".to_string(),
            llm_provider: "anthropic".to_string(),
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).unwrap();
        let loaded = load_config(dir.path());
        assert_eq!(loaded.stt_provider, "openai");
        assert_eq!(loaded.llm_provider, "anthropic");
    }

    /// Partial JSON without provider fields uses defaults ("groq", "deepseek").
    #[test]
    fn test_partial_json_fills_provider_defaults() {
        let dir = temp_dir();
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(cfg.stt_provider, "groq");
        assert_eq!(cfg.llm_provider, "deepseek");
    }

    /// Old config.json with stt_priority/llm_priority but no provider fields
    /// migrates the first entry of each list to the new provider fields.
    #[test]
    fn test_old_config_with_priority_fields_migrates_to_provider() {
        let dir = temp_dir();
        // Simulate a legacy config.json that has the old priority lists but no new fields.
        let legacy = r#"{
            "language": "de",
            "sttPriority": ["openai", "groq"],
            "llmPriority": ["anthropic", "openai"]
        }"#;
        std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        // Migration should promote the first entry of each list.
        assert_eq!(cfg.stt_provider, "openai", "sttPriority[0] should be promoted to stt_provider");
        assert_eq!(cfg.llm_provider, "anthropic", "llmPriority[0] should be promoted to llm_provider");
        // After migration the legacy lists are cleared.
        assert!(cfg.stt_priority.is_empty(), "stt_priority should be cleared after migration");
        assert!(cfg.llm_priority.is_empty(), "llm_priority should be cleared after migration");
    }

    /// Migration is persisted: a second load_config call reads the already-migrated
    /// on-disk file and does NOT touch the provider fields again.
    #[test]
    fn test_migration_is_persisted_to_disk() {
        let dir = temp_dir();
        let legacy = r#"{
            "language": "de",
            "sttPriority": ["openai", "groq"],
            "llmPriority": ["anthropic", "openai"]
        }"#;
        std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

        // First load triggers migration + save.
        let _ = load_config(dir.path());

        // Second load reads the already-migrated file.
        let cfg2 = load_config(dir.path());
        assert_eq!(cfg2.stt_provider, "openai");
        assert_eq!(cfg2.llm_provider, "anthropic");
        assert!(cfg2.stt_priority.is_empty());
        assert!(cfg2.llm_priority.is_empty());
    }

    /// Fresh config (no sttPriority / llmPriority) keeps the defaults.
    #[test]
    fn test_fresh_config_keeps_defaults() {
        let dir = temp_dir();
        // Only a Groq key is set, no priority lists, no explicit llmProvider.
        // The default llm_provider is "deepseek", but deepseek has no key while
        // Groq does -- so the auto-fallback switches llm_provider to "groq".
        let fresh = r#"{"groqApiKey": "gsk_test"}"#;
        std::fs::write(dir.path().join("config.json"), fresh.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(cfg.stt_provider, "groq", "fresh config should keep default stt_provider");
        assert_eq!(
            cfg.llm_provider, "groq",
            "auto-fallback: deepseek has no key, groq does, so llm_provider should switch to groq"
        );
    }

    /// Config that already has an explicit sttProvider is NOT overwritten by migration,
    /// even when sttPriority is also present.
    #[test]
    fn test_explicit_provider_field_suppresses_migration() {
        let dir = temp_dir();
        // User had already explicitly set sttProvider = "local" before this session.
        let already_set = r#"{
            "sttProvider": "local",
            "llmProvider": "groq",
            "sttPriority": ["openai", "groq"],
            "llmPriority": ["anthropic", "openai"]
        }"#;
        std::fs::write(dir.path().join("config.json"), already_set.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        // Explicit values must be preserved -- migration must NOT overwrite them.
        assert_eq!(cfg.stt_provider, "local");
        assert_eq!(cfg.llm_provider, "groq");
    }

    /// An unknown provider value is rejected and falls back to the default.
    #[test]
    fn test_unknown_provider_falls_back_to_default() {
        let dir = temp_dir();
        let bad = r#"{"sttProvider": "fakeai", "llmProvider": "madeup"}"#;
        std::fs::write(dir.path().join("config.json"), bad.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(cfg.stt_provider, "groq", "unknown stt_provider should fall back to groq");
        assert_eq!(cfg.llm_provider, "deepseek", "unknown llm_provider should fall back to deepseek");
    }

    /// Provider fields serialize with camelCase keys.
    #[test]
    fn test_provider_fields_serialize_camel_case() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("sttProvider"), "expected camelCase 'sttProvider'");
        assert!(json.contains("llmProvider"), "expected camelCase 'llmProvider'");
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
            webhook_url: "https://hooks.example.com/klarvo".to_string(),
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).unwrap();
        let loaded = load_config(dir.path());
        assert_eq!(loaded.webhook_url, "https://hooks.example.com/klarvo");
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

    // --- local_whisper field tests ---

    /// Default `local_whisper_model` is the German-optimized model.
    #[test]
    fn test_default_local_whisper_model_is_german() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.local_whisper_model, "tiny-german-1224-q8_0");
    }

    /// Default `local_whisper_gpu` is `true`.
    #[test]
    fn test_default_local_whisper_gpu_is_true() {
        let cfg = AppConfig::default();
        assert!(cfg.local_whisper_gpu, "default local_whisper_gpu should be true");
    }

    /// `local_whisper_model` and `local_whisper_gpu` round-trip through save/load.
    #[test]
    fn test_local_whisper_fields_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            local_whisper_model: "tiny-q5_1".to_string(),
            local_whisper_gpu: false,
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).expect("save should succeed");
        let loaded = load_config(dir.path());
        assert_eq!(loaded.local_whisper_model, "tiny-q5_1");
        assert!(!loaded.local_whisper_gpu);
    }

    /// Partial JSON without `localWhisperModel` fills in the default.
    #[test]
    fn test_partial_json_local_whisper_model_defaults_to_small() {
        let dir = temp_dir();
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(
            cfg.local_whisper_model, "tiny-german-1224-q8_0",
            "missing localWhisperModel should default to German model"
        );
        assert!(
            cfg.local_whisper_gpu,
            "missing localWhisperGpu should default to true"
        );
    }

    /// Config serializes local_whisper fields with camelCase keys.
    #[test]
    fn test_local_whisper_fields_serialize_camel_case() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(
            json.contains("localWhisperModel"),
            "expected camelCase 'localWhisperModel'"
        );
        assert!(
            json.contains("localWhisperGpu"),
            "expected camelCase 'localWhisperGpu'"
        );
    }

    // --- bar_x / bar_y position persistence tests ---

    /// Default bar_x and bar_y are None (no saved position on first run).
    #[test]
    fn test_default_bar_position_is_none() {
        let cfg = AppConfig::default();
        assert!(cfg.bar_x.is_none(), "default bar_x should be None");
        assert!(cfg.bar_y.is_none(), "default bar_y should be None");
    }

    /// bar_x and bar_y round-trip through save/load with concrete values.
    #[test]
    fn test_bar_position_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            bar_x: Some(320.5),
            bar_y: Some(1024.0),
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).expect("save should succeed");
        let loaded = load_config(dir.path());
        assert_eq!(loaded.bar_x, Some(320.5));
        assert_eq!(loaded.bar_y, Some(1024.0));
    }

    /// Old config.json without barX / barY loads with None defaults (backwards compat).
    #[test]
    fn test_old_config_without_bar_position_loads_with_none() {
        let dir = temp_dir();
        // Simulate a config.json written before bar_x/bar_y were added.
        let legacy = r#"{"language": "de", "groqApiKey": "gsk_test"}"#;
        std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

        let cfg = load_config(dir.path());
        assert!(
            cfg.bar_x.is_none(),
            "bar_x should be None when field is absent in config.json"
        );
        assert!(
            cfg.bar_y.is_none(),
            "bar_y should be None when field is absent in config.json"
        );
    }

    /// bar_x and bar_y serialize with camelCase keys.
    #[test]
    fn test_bar_position_serializes_camel_case() {
        let cfg = AppConfig {
            bar_x: Some(100.0),
            bar_y: Some(200.0),
            ..AppConfig::default()
        };
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("\"barX\""), "expected camelCase 'barX'");
        assert!(json.contains("\"barY\""), "expected camelCase 'barY'");
    }

    /// `AutoStop` and `Auto` variants serialize with lowercase names.
    #[test]
    fn test_hotkey_mode_new_variants_serialize() {
        let autostop = serde_json::to_string(&HotkeyMode::AutoStop).unwrap();
        let auto = serde_json::to_string(&HotkeyMode::Auto).unwrap();
        assert_eq!(autostop, r#""autostop""#);
        assert_eq!(auto, r#""auto""#);
    }

    /// `AutoStop` and `Auto` variants deserialize from lowercase strings.
    #[test]
    fn test_hotkey_mode_new_variants_deserialize() {
        let autostop: HotkeyMode = serde_json::from_str(r#""autostop""#).unwrap();
        let auto: HotkeyMode = serde_json::from_str(r#""auto""#).unwrap();
        assert_eq!(autostop, HotkeyMode::AutoStop);
        assert_eq!(auto, HotkeyMode::Auto);
    }

    /// All four `HotkeyMode` variants survive a save/load round-trip.
    #[test]
    fn test_all_hotkey_modes_roundtrip() {
        for mode in [HotkeyMode::Toggle, HotkeyMode::Hold, HotkeyMode::AutoStop, HotkeyMode::Auto] {
            let dir = temp_dir();
            let cfg = AppConfig { hotkey_mode: mode, ..AppConfig::default() };
            save_config(dir.path(), &cfg).unwrap();
            let loaded = load_config(dir.path());
            assert_eq!(loaded.hotkey_mode, mode, "mode {mode:?} should survive roundtrip");
        }
    }

    /// Default values for new recording-mode config fields are correct.
    #[test]
    fn test_new_recording_mode_defaults() {
        let cfg = AppConfig::default();
        assert!(!cfg.insert_and_send, "insert_and_send should default to false");
        assert!((cfg.autostop_silence_secs - 2.0).abs() < f32::EPSILON);
        assert!((cfg.auto_mode_silence_secs - 2.0).abs() < f32::EPSILON);
    }

    /// New recording-mode fields survive a save/load round-trip.
    #[test]
    fn test_new_recording_mode_fields_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            insert_and_send: true,
            autostop_silence_secs: 1.5,
            auto_mode_silence_secs: 3.0,
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).unwrap();
        let loaded = load_config(dir.path());
        // Migration: global insert_and_send=true is moved to slots, global reset to false
        assert!(!loaded.insert_and_send, "global insert_and_send should be false after migration");
        assert!(loaded.hotkey_slots.iter().all(|s| s.insert_and_send),
            "all slots should have insert_and_send=true after migration");
        assert!((loaded.autostop_silence_secs - 1.5).abs() < f32::EPSILON);
        assert!((loaded.auto_mode_silence_secs - 3.0).abs() < f32::EPSILON);
    }

    /// Partial JSON without new fields fills in defaults (backward compat).
    #[test]
    fn test_new_fields_absent_from_json_use_defaults() {
        let dir = temp_dir();
        let partial = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), partial.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert!(!cfg.insert_and_send);
        assert!((cfg.autostop_silence_secs - 2.0).abs() < f32::EPSILON);
        assert!((cfg.auto_mode_silence_secs - 2.0).abs() < f32::EPSILON);
    }

    // -----------------------------------------------------------------------
    // HotkeySlot tests
    // -----------------------------------------------------------------------

    /// `HotkeySlot` serializes with camelCase keys.
    #[test]
    fn test_hotkey_slot_serializes_camel_case() {
        let slot = HotkeySlot {
            hotkey: "ctrl+shift+d".to_string(),
            mode: HotkeyMode::Hold,
            insert_and_send: false,
        };
        let json = serde_json::to_string(&slot).unwrap();
        assert!(json.contains("\"hotkey\""), "expected key 'hotkey'");
        assert!(json.contains("\"mode\""), "expected key 'mode'");
        assert!(json.contains("\"hold\""), "expected mode value 'hold'");
    }

    /// `HotkeySlot` deserializes correctly from JSON.
    #[test]
    fn test_hotkey_slot_deserializes() {
        let json = r#"{"hotkey":"ctrl+shift+d","mode":"toggle"}"#;
        let slot: HotkeySlot = serde_json::from_str(json).unwrap();
        assert_eq!(slot.hotkey, "ctrl+shift+d");
        assert_eq!(slot.mode, HotkeyMode::Toggle);
    }

    /// `HotkeySlot` round-trips through serialize → deserialize without loss.
    #[test]
    fn test_hotkey_slot_roundtrip() {
        for mode in [HotkeyMode::Toggle, HotkeyMode::Hold, HotkeyMode::AutoStop, HotkeyMode::Auto] {
            let slot = HotkeySlot {
                hotkey: "ctrl+shift+x".to_string(),
                mode,
                insert_and_send: false,
            };
            let json = serde_json::to_string(&slot).unwrap();
            let back: HotkeySlot = serde_json::from_str(&json).unwrap();
            assert_eq!(back, slot, "HotkeySlot with mode {mode:?} should survive roundtrip");
        }
    }

    /// `HotkeySlot::is_enabled` returns `true` for non-empty hotkeys and `false` for empty.
    #[test]
    fn test_hotkey_slot_is_enabled() {
        let enabled = HotkeySlot { hotkey: "ctrl+shift+d".to_string(), mode: HotkeyMode::Hold, insert_and_send: false };
        let disabled = HotkeySlot { hotkey: String::new(), mode: HotkeyMode::Hold, insert_and_send: false };
        assert!(enabled.is_enabled());
        assert!(!disabled.is_enabled());
    }

    // -----------------------------------------------------------------------
    // hotkey_slots default / migration / roundtrip
    // -----------------------------------------------------------------------

    /// Default config has exactly 2 slots: slot 0 enabled, slot 1 disabled.
    #[test]
    fn test_default_hotkey_slots() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.hotkey_slots.len(), 2, "default should have exactly 2 slots");
        assert_eq!(cfg.hotkey_slots[0].hotkey, "ctrl+shift+d");
        assert_eq!(cfg.hotkey_slots[0].mode, HotkeyMode::Hold);
        assert!(cfg.hotkey_slots[1].hotkey.is_empty(), "slot 1 should be disabled by default");
    }

    /// `hotkey_slots` round-trips through save/load without data loss.
    #[test]
    fn test_hotkey_slots_roundtrip() {
        let dir = temp_dir();
        let cfg = AppConfig {
            hotkey_slots: vec![
                HotkeySlot { hotkey: "ctrl+shift+d".to_string(), mode: HotkeyMode::Toggle, insert_and_send: false },
                HotkeySlot { hotkey: "ctrl+shift+f".to_string(), mode: HotkeyMode::AutoStop, insert_and_send: false },
            ],
            ..AppConfig::default()
        };
        save_config(dir.path(), &cfg).expect("save should succeed");
        let loaded = load_config(dir.path());
        assert_eq!(loaded.hotkey_slots, cfg.hotkey_slots);
    }

    /// Migration: old config with only `hotkey` + `hotkey_mode`, no `hotkey_slots`.
    /// Slot 0 must be populated from the legacy fields; slot 1 must be disabled.
    #[test]
    fn test_migration_legacy_hotkey_fields_to_slots() {
        let dir = temp_dir();
        let legacy = r#"{"hotkey":"ctrl+alt+r","hotkeyMode":"toggle"}"#;
        std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

        let cfg = load_config(dir.path());

        assert_eq!(cfg.hotkey_slots.len(), 2, "migration should produce exactly 2 slots");
        assert_eq!(cfg.hotkey_slots[0].hotkey, "ctrl+alt+r", "slot 0 hotkey must come from legacy field");
        assert_eq!(cfg.hotkey_slots[0].mode, HotkeyMode::Toggle, "slot 0 mode must come from legacy field");
        assert!(cfg.hotkey_slots[1].hotkey.is_empty(), "slot 1 must be disabled after migration");
        assert_eq!(cfg.hotkey_slots[1].mode, HotkeyMode::Hold);
    }

    /// Migration: old config with neither `hotkey_slots` nor legacy fields.
    /// Slot 0 must fall back to the compiled-in defaults.
    #[test]
    fn test_migration_empty_config_uses_defaults_for_slot0() {
        let dir = temp_dir();
        let empty = r#"{}"#;
        std::fs::write(dir.path().join("config.json"), empty.as_bytes()).unwrap();

        let cfg = load_config(dir.path());

        assert_eq!(cfg.hotkey_slots.len(), 2);
        assert_eq!(cfg.hotkey_slots[0].hotkey, "ctrl+shift+d", "slot 0 should fall back to default hotkey");
        assert_eq!(cfg.hotkey_slots[0].mode, HotkeyMode::Hold, "slot 0 should fall back to Hold mode");
        assert!(cfg.hotkey_slots[1].hotkey.is_empty());
    }

    /// Migration is persisted: a second load_config call reads the already-migrated
    /// on-disk file and does NOT re-run the migration.
    #[test]
    fn test_hotkey_slots_migration_is_persisted() {
        let dir = temp_dir();
        let legacy = r#"{"hotkey":"ctrl+alt+r","hotkeyMode":"toggle"}"#;
        std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

        // First load triggers migration + save.
        let _ = load_config(dir.path());

        // Second load reads the already-migrated file.
        let cfg2 = load_config(dir.path());
        assert_eq!(cfg2.hotkey_slots.len(), 2);
        assert_eq!(cfg2.hotkey_slots[0].hotkey, "ctrl+alt+r");
        assert_eq!(cfg2.hotkey_slots[0].mode, HotkeyMode::Toggle);
    }

    /// New config already containing `hotkey_slots` is NOT overwritten by migration.
    #[test]
    fn test_existing_hotkey_slots_suppresses_migration() {
        let dir = temp_dir();
        // Config already has hotkey_slots -- migration must leave them untouched.
        let modern = r#"{
            "hotkey": "ctrl+alt+r",
            "hotkeyMode": "toggle",
            "hotkeySlots": [
                {"hotkey": "ctrl+shift+d", "mode": "hold"},
                {"hotkey": "ctrl+shift+f", "mode": "autostop"}
            ]
        }"#;
        std::fs::write(dir.path().join("config.json"), modern.as_bytes()).unwrap();

        let cfg = load_config(dir.path());

        // hotkey_slots must be exactly what was in the JSON, not replaced by legacy fields.
        assert_eq!(cfg.hotkey_slots.len(), 2);
        assert_eq!(cfg.hotkey_slots[0].hotkey, "ctrl+shift+d");
        assert_eq!(cfg.hotkey_slots[0].mode, HotkeyMode::Hold);
        assert_eq!(cfg.hotkey_slots[1].hotkey, "ctrl+shift+f");
        assert_eq!(cfg.hotkey_slots[1].mode, HotkeyMode::AutoStop);
    }

    /// `hotkey_slots` serializes as camelCase `"hotkeySlots"` in JSON.
    #[test]
    fn test_hotkey_slots_serializes_camel_case() {
        let cfg = AppConfig::default();
        let json = serde_json::to_string(&cfg).unwrap();
        assert!(json.contains("hotkeySlots"), "expected camelCase 'hotkeySlots'");
    }

    // -----------------------------------------------------------------------
    // Auto-fallback for llm_provider when the configured provider has no key
    // -----------------------------------------------------------------------

    /// llm_provider is "deepseek" (default) but deepseek key is empty while
    /// openai key is present → auto-switch to "openai".
    #[test]
    fn test_llm_provider_auto_fallback_to_openai() {
        let dir = temp_dir();
        let json = r#"{
            "llmProvider": "deepseek",
            "deepseekApiKey": "",
            "openaiApiKey": "sk-openai-test-key"
        }"#;
        std::fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(
            cfg.llm_provider, "openai",
            "should auto-switch away from deepseek when its key is empty and openai has a key"
        );
    }

    /// llm_provider is "deepseek" and deepseekApiKey is non-empty → no switch.
    #[test]
    fn test_llm_provider_no_switch_when_key_present() {
        let dir = temp_dir();
        let json = r#"{
            "llmProvider": "deepseek",
            "deepseekApiKey": "ds-real-key-abc",
            "openaiApiKey": "sk-openai-test-key"
        }"#;
        std::fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(
            cfg.llm_provider, "deepseek",
            "should NOT switch when the chosen provider already has a key"
        );
    }

    /// All API keys empty → llm_provider stays at default, no panic.
    #[test]
    fn test_llm_provider_no_switch_when_all_keys_empty() {
        let dir = temp_dir();
        // No keys at all; serde fills everything with defaults.
        let json = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(
            cfg.llm_provider, "deepseek",
            "should stay at default when every provider key is empty"
        );
    }

    // -------------------------------------------------------------------------
    // Groq-Llama Default tests
    // -------------------------------------------------------------------------

    /// Main case: STT is Groq with a key, LLM is still on the default "deepseek"
    /// and no DeepSeek key is configured → llm_provider is auto-switched to "groq".
    #[test]
    fn test_groq_llama_default_auto_switch() {
        let dir = temp_dir();
        let json = r#"{"sttProvider": "groq", "groqApiKey": "gsk_test_key"}"#;
        std::fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(cfg.stt_provider, "groq");
        assert_eq!(
            cfg.llm_provider, "groq",
            "Groq-Llama Default: STT=groq with key, no DeepSeek key → llm_provider must become groq"
        );
    }

    /// Guard: when a DeepSeek key IS present the user has intentionally configured
    /// DeepSeek — the auto-switch must NOT fire.
    #[test]
    fn test_groq_llama_default_no_override_with_deepseek_key() {
        let dir = temp_dir();
        let json = r#"{
            "sttProvider": "groq",
            "groqApiKey": "gsk_test_key",
            "deepseekApiKey": "ds_test_key"
        }"#;
        std::fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(
            cfg.llm_provider, "deepseek",
            "DeepSeek key present → must stay deepseek, Groq-Llama Default must not fire"
        );
    }

    /// Guard: when the user has explicitly chosen a different LLM provider (e.g.
    /// "openai") the auto-switch must NOT fire, regardless of key state.
    #[test]
    fn test_groq_llama_default_no_override_explicit_provider() {
        let dir = temp_dir();
        let json = r#"{
            "sttProvider": "groq",
            "llmProvider": "openai",
            "groqApiKey": "gsk_test_key",
            "openaiApiKey": "sk_openai_key"
        }"#;
        std::fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(
            cfg.llm_provider, "openai",
            "Explicit llmProvider=openai with key present → must not be overridden"
        );
    }

    /// Guard: STT provider is not Groq (e.g. "openai") → no switch, even if a
    /// Groq key is present and DeepSeek key is absent.
    #[test]
    fn test_groq_llama_default_no_switch_non_groq_stt() {
        let dir = temp_dir();
        let json = r#"{
            "sttProvider": "openai",
            "openaiApiKey": "sk_openai_key",
            "groqApiKey": "gsk_test_key"
        }"#;
        std::fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        // The general auto-fallback may still switch llm_provider if deepseek key
        // is empty and another key exists.  What matters here is that the switch
        // (if it happens) is NOT caused by the Groq-Llama Default path.
        // The STT provider is "openai", so the Groq-Llama Default condition
        // (stt_provider == "groq") is false and the block is skipped entirely.
        assert_eq!(
            cfg.stt_provider, "openai",
            "stt_provider should remain openai"
        );
        // llm_provider will be switched by the general auto-fallback to groq
        // (deepseek has no key, groq does) — that is correct behaviour and does
        // NOT indicate the Groq-Llama Default fired.
        assert_ne!(
            cfg.llm_provider, "deepseek",
            "general auto-fallback should switch away from keyless deepseek"
        );
    }

    /// Guard: Groq key is also empty → switching would produce a 401 anyway,
    /// so the auto-switch must NOT fire.
    #[test]
    fn test_groq_llama_default_no_switch_no_groq_key() {
        let dir = temp_dir();
        // stt_provider = groq (default), llm_provider = deepseek (default),
        // but both keys are absent.
        let json = r#"{"language": "de"}"#;
        std::fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();
        let cfg = load_config(dir.path());
        assert_eq!(cfg.stt_provider, "groq");
        assert_eq!(
            cfg.llm_provider, "deepseek",
            "No keys at all → Groq-Llama Default must not fire, llm_provider stays deepseek"
        );
    }

    // ---------------------------------------------------------------------------
    // Golden-Master / Characterization Test: AppConfig Full-Field Roundtrip
    //
    // PURPOSE: Characterization test that nails down the CURRENT save/load
    // behaviour for every field in AppConfig. If any field silently drops,
    // renames, or transforms during a save→load cycle this test catches it.
    //
    // The test characterizes the REAL save/load path: save_config writes JSON
    // to disk, load_config reads + migrates + validates + returns the struct.
    //
    // KNOWN ROUNDTRIP ANOMALIES (IST-Verhalten, not bugs to fix here):
    //
    // 1. `stt_priority` / `llm_priority` (deprecated):
    //    Cannot be set to a non-empty Vec and roundtripped cleanly.
    //    Reason: load_config's migration logic promotes stt_priority[0] to
    //    stt_provider when stt_provider == default ("groq"), then clears the
    //    list. Setting stt_provider to non-default suppresses migration, but
    //    then the list still survives on disk while stt_priority remains
    //    ignored by all production code. In this test both are set to empty
    //    Vec (the safe value) and verified to stay empty.
    //
    // 2. `insert_and_send` (deprecated global flag):
    //    When insert_and_send = true AND all hotkey_slots have
    //    insert_and_send = false, load_config migrates: it sets every slot's
    //    insert_and_send = true and clears the global flag to false.
    //    Result: the loaded struct differs from the saved struct.
    //    In this test we set insert_and_send = false to bypass the migration
    //    and characterize the clean case. The migration path is separately
    //    tested in test_insert_and_send_migration_to_slots.
    //
    // 3. `hotkey_slots` (empty Vec triggers migration):
    //    An empty hotkey_slots Vec triggers migration from the legacy flat
    //    hotkey/hotkey_mode fields. We always set a non-empty slots vec.
    //
    // 4. API key env-var fallback:
    //    load_config overwrites groq_api_key, deepseek_api_key, openai_api_key,
    //    anthropic_api_key, turso_url, turso_token from env vars when the
    //    saved value is empty. All API key fields here are set to non-empty
    //    test strings so the fallback logic is bypassed.
    //
    // 5. LLM auto-fallback:
    //    load_config auto-switches llm_provider when the chosen provider has
    //    no key. We set llm_provider = "deepseek" with a non-empty
    //    deepseek_api_key, so no switch fires.
    //
    // 6. `device_id`:
    //    AppConfig::default() generates a fresh UUID each call. When we
    //    explicitly set device_id = "test-device-uuid" and save/load, the
    //    exact string is preserved (no re-generation on load). This is correct.
    // ---------------------------------------------------------------------------

    /// Constructs an AppConfig with every field set to a non-default value and
    /// verifies that save_config → load_config returns a byte-for-byte identical
    /// struct. Any field that silently drops or transforms will cause an assertion
    /// failure, identifying fragility in the persistence layer.
    #[test]
    fn test_appconfig_golden_master_full_field_roundtrip() {
        let dir = temp_dir();

        // Build a config where every field is explicitly set to a non-default
        // value so that no field can hide behind a default.
        let original = AppConfig {
            // --- API keys (non-empty so env-var fallback is bypassed) ---
            groq_api_key: "gsk-golden-master-groq".to_string(),
            deepseek_api_key: "sk-ds-golden-master".to_string(),
            openai_api_key: "sk-openai-golden-master".to_string(),
            anthropic_api_key: "sk-ant-golden-master".to_string(),
            openrouter_api_key: "sk-or-golden-master".to_string(),

            // --- Providers (non-default to suppress migration side-effects) ---
            // stt_provider = "local" (default is "groq")
            // llm_provider = "deepseek" with key set → no auto-fallback fires
            stt_provider: "local".to_string(),
            llm_provider: "deepseek".to_string(),

            // --- Deprecated lists (kept empty: non-empty triggers migration that
            // clears them, making a roundtrip impossible). ---
            stt_priority: Vec::new(),
            llm_priority: Vec::new(),

            // --- Core settings ---
            language: "de".to_string(),
            cleanup_style: CleanupStyle::Verbatim,
            hotkey: "ctrl+alt+x".to_string(),           // non-default
            hotkey_mode: HotkeyMode::Toggle,             // non-default (default = Hold)

            // Non-empty slots vec prevents hotkey migration from firing.
            hotkey_slots: vec![
                HotkeySlot {
                    hotkey: "ctrl+alt+x".to_string(),
                    mode: HotkeyMode::Toggle,
                    insert_and_send: true,
                },
                HotkeySlot {
                    hotkey: "ctrl+alt+y".to_string(),
                    mode: HotkeyMode::AutoStop,
                    insert_and_send: false,
                },
            ],

            audio_device: Some("Golden Master Mic".to_string()),
            stt_model: "whisper-large-v3".to_string(),
            custom_prompt: "Golden master custom prompt.".to_string(),

            profiles: vec![
                AppProfile {
                    name: "Browser".to_string(),
                    app_pattern: "chrome".to_string(),
                    cleanup_style: CleanupStyle::Chat,
                    language: "en".to_string(),
                    custom_prompt: "Be concise.".to_string(),
                },
                AppProfile {
                    name: "Terminal".to_string(),
                    app_pattern: "powershell".to_string(),
                    cleanup_style: CleanupStyle::Verbatim,
                    language: "de".to_string(),
                    custom_prompt: String::new(),
                },
            ],

            autostart: true,
            whisper_mode: true,
            command_hotkey: "ctrl+shift+g".to_string(),
            output_language: "en".to_string(),

            snippets: vec![
                TextSnippet {
                    name: "sig".to_string(),
                    content: "Viele Grüße,\nAndy".to_string(),
                },
                TextSnippet {
                    name: "addr".to_string(),
                    content: "Teststraße 1, 12345 Berlin".to_string(),
                },
            ],

            voice_notes_hotkey: "ctrl+shift+v".to_string(),
            webhook_url: "https://golden.example.com/hook".to_string(),

            // Non-empty so env-var fallback is bypassed.
            turso_url: "libsql://golden-master.turso.io".to_string(),
            turso_token: "golden-turso-jwt-token".to_string(),

            device_id: "golden-master-device-uuid-1234".to_string(),

            bubble_size: 1.5,
            bubble_opacity: 0.5,

            advanced: AdvancedSettings {
                stt_prompt_de: "Custom DE prompt golden master.".to_string(),
                stt_prompt_en: "Custom EN prompt golden master.".to_string(),
                stt_prompt_auto: "Custom auto prompt golden master.".to_string(),
                stt_temperature: 0.3,
                llm_system_prompt_polished: "Polished system prompt.".to_string(),
                llm_system_prompt_verbatim: "Verbatim system prompt.".to_string(),
                llm_system_prompt_chat: "Chat system prompt.".to_string(),
                llm_command_mode_prompt: "Command mode prompt.".to_string(),
                llm_temperature: 0.7,
                llm_max_tokens: 1024,
                llm_model_deepseek: "deepseek-chat".to_string(),
                llm_model_openai: "gpt-4o".to_string(),
                llm_model_anthropic: "claude-3-5-sonnet".to_string(),
                llm_model_groq: "llama3-70b-8192".to_string(),
                chunk_threshold: 1200,
                chunk_target_size: 900,
                silence_threshold: 0.01,
                whisper_mode_threshold: 0.002,
                min_recording_ms: 750,
                whisper_mode_gain: 5.0,
                auto_paste: false,
                paste_delay_ms: 100,
                auto_capitalize: false,
                webhook_headers: r#"{"X-Custom-Header": "golden"}"#.to_string(),
                webhook_timeout_secs: 30,
                log_level: "debug".to_string(),
                ui_scale: "large".to_string(),
            },

            local_whisper_model: "base-q5_1".to_string(),
            local_whisper_gpu: false,

            license_key: "GOLDEN-MASTER-LICENSE-KEY".to_string(),
            license_validated_at: 1_700_000_000,
            license_source: "hmac".to_string(),
            ls_instance_id: "golden-ls-instance-uuid".to_string(),
            ls_last_validated_at: 1_700_001_000,

            // insert_and_send = false: if set to true while ALL slots have
            // insert_and_send = false, load_config migrates (propagates to slots
            // and clears this flag), making roundtrip impossible.
            // The migration path is covered by test_insert_and_send_migration_to_slots.
            insert_and_send: false,

            autostop_silence_secs: 3.5,
            auto_mode_silence_secs: 4.0,
            bar_x: Some(200.0),
            bar_y: Some(800.0),

            bubble_recording_mode: "toggle".to_string(),
            bubble_tap_mode: "autostop".to_string(),
            bubble_tap_auto_send: true,
            bubble_tap_silence_secs: 1.5,
            bubble_long_press_mode: "auto".to_string(),
            bubble_long_press_auto_send: true,
            bubble_long_press_silence_secs: 2.5,

            onboarding: OnboardingState {
                completed: true,
                skipped: false,
                current_step: 5,
                mode: "cloud".to_string(),
                language: "de".to_string(),
                track: "expert".to_string(),
            },

            voice_command_enabled: true,
            first_install_at: 1_710_000_000,
            feedback_webhook_url: "https://golden.example.com/feedback".to_string(),
        };

        save_config(dir.path(), &original).expect("save_config must succeed");
        let loaded = load_config(dir.path());

        assert_eq!(
            loaded, original,
            "Full AppConfig roundtrip (save→load) must preserve every field exactly. \
            A failure here means a field was silently dropped, renamed, or transformed \
            during persistence. Check the golden master anomalies documented above \
            the test for known load_config side-effects."
        );
    }

    /// Characterization test: documents the `insert_and_send` migration side-effect.
    ///
    /// When the global `insert_and_send = true` flag is saved and ALL slots have
    /// `insert_and_send = false`, load_config propagates the flag to all slots
    /// and clears the global field to false. This is an intentional migration but
    /// means the loaded struct DIFFERS from the saved struct on the first load
    /// after migration. Subsequent loads are clean.
    #[test]
    fn test_insert_and_send_migration_to_slots() {
        let dir = temp_dir();

        // Save with global insert_and_send = true, slots both false.
        let before_migration = AppConfig {
            insert_and_send: true,
            // Use a non-empty stt_provider + deepseek key to suppress other migrations.
            stt_provider: "openai".to_string(),
            llm_provider: "openai".to_string(),
            openai_api_key: "sk-openai-test".to_string(),
            hotkey_slots: vec![
                HotkeySlot {
                    hotkey: "ctrl+shift+d".to_string(),
                    mode: HotkeyMode::Hold,
                    insert_and_send: false, // will be migrated to true
                },
                HotkeySlot {
                    hotkey: String::new(),
                    mode: HotkeyMode::Hold,
                    insert_and_send: false, // will be migrated to true
                },
            ],
            ..AppConfig::default()
        };
        save_config(dir.path(), &before_migration).expect("save must succeed");

        let loaded = load_config(dir.path());

        // After migration: global flag cleared, all slots set to true.
        assert!(
            !loaded.insert_and_send,
            "Migration must clear global insert_and_send flag"
        );
        assert!(
            loaded.hotkey_slots.iter().all(|s| s.insert_and_send),
            "Migration must propagate insert_and_send=true to all slots"
        );

        // The loaded struct differs from the saved struct (this is the documented
        // anomaly: one-way migration produces a different struct on first load).
        assert_ne!(
            loaded, before_migration,
            "First load after insert_and_send migration produces a different struct (expected)"
        );

        // Second load must be idempotent (no further migration).
        let loaded2 = load_config(dir.path());
        assert_eq!(loaded2, loaded, "Second load must be idempotent after migration");
    }

    // -----------------------------------------------------------------------
    // Story 1.4 — Pre-migration backup + error propagation (ROB-05 / ADR-0015)
    // -----------------------------------------------------------------------

    /// Collect all `config.json.pre-migration-*` backup files present in `dir`.
    fn pre_migration_backups(dir: &Path) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .expect("read dir")
            .map(|e| e.expect("dir entry").path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("config.json.pre-migration-"))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// sttPriority/llmPriority migration writes a pre-migration backup.
    #[test]
    fn test_migration_backup_written_on_stt_priority_migration() {
        let dir = temp_dir();
        // Include hotkeySlots to suppress the hotkey_slots migration; otherwise both
        // migrations fire and two backups are written, breaking the len()==1 assertion.
        let legacy = r#"{
            "sttPriority": ["openai"],
            "llmPriority": ["anthropic"],
            "hotkeySlots": [
                {"hotkey": "ctrl+alt+r", "mode": "hold", "insertAndSend": false},
                {"hotkey": "", "mode": "hold", "insertAndSend": false}
            ]
        }"#;
        std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

        let _ = load_config(dir.path());

        let backups = pre_migration_backups(dir.path());
        assert_eq!(backups.len(), 1, "exactly one pre-migration backup should be written");
        let content = std::fs::read_to_string(&backups[0]).expect("backup must be readable");
        assert!(
            content.contains("sttPriority"),
            "backup should contain the pre-migration legacy field"
        );
    }

    /// hotkey_slots migration writes a pre-migration backup.
    #[test]
    fn test_migration_backup_written_on_hotkey_slots_migration() {
        let dir = temp_dir();
        // Suppress sttPriority migration by providing explicit provider fields.
        let legacy = r#"{"sttProvider": "groq", "llmProvider": "deepseek", "hotkey": "ctrl+alt+r", "hotkeyMode": "toggle"}"#;
        std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

        let _ = load_config(dir.path());

        let backups = pre_migration_backups(dir.path());
        assert!(
            !backups.is_empty(),
            "pre-migration backup must be written for hotkey_slots migration"
        );
    }

    /// insert_and_send per-slot migration writes a pre-migration backup.
    #[test]
    fn test_migration_backup_written_on_insert_and_send_migration() {
        let dir = temp_dir();
        let before = AppConfig {
            insert_and_send: true,
            stt_provider: "openai".to_string(),
            llm_provider: "openai".to_string(),
            openai_api_key: "sk-test".to_string(),
            hotkey_slots: vec![
                HotkeySlot {
                    hotkey: "ctrl+shift+d".to_string(),
                    mode: HotkeyMode::Hold,
                    insert_and_send: false,
                },
                HotkeySlot {
                    hotkey: String::new(),
                    mode: HotkeyMode::Hold,
                    insert_and_send: false,
                },
            ],
            ..AppConfig::default()
        };
        save_config(dir.path(), &before).unwrap();

        let _ = load_config(dir.path());

        let backups = pre_migration_backups(dir.path());
        assert!(
            !backups.is_empty(),
            "pre-migration backup must be written for insert_and_send migration"
        );
    }

    /// No pre-migration backup when an already-migrated config is loaded.
    #[test]
    fn test_no_migration_backup_when_no_migration_runs() {
        let dir = temp_dir();
        // Fully-migrated config: explicit providers, non-empty hotkey_slots, no global insert_and_send.
        let modern = r#"{
            "sttProvider": "groq",
            "llmProvider": "deepseek",
            "hotkeySlots": [
                {"hotkey": "ctrl+alt+r", "mode": "hold", "insertAndSend": false},
                {"hotkey": "", "mode": "hold", "insertAndSend": false}
            ]
        }"#;
        std::fs::write(dir.path().join("config.json"), modern.as_bytes()).unwrap();

        let _ = load_config(dir.path());

        let backups = pre_migration_backups(dir.path());
        assert!(
            backups.is_empty(),
            "no pre-migration backup should be written when no migration runs"
        );
    }

    /// Pre-migration backup is valid JSON containing the original on-disk state.
    #[test]
    fn test_migration_backup_is_valid_json() {
        let dir = temp_dir();
        let legacy = r#"{"sttPriority": ["openai"], "groqApiKey": "gsk-test"}"#;
        std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

        let _ = load_config(dir.path());

        let backups = pre_migration_backups(dir.path());
        assert!(!backups.is_empty(), "backup must exist");
        let content = std::fs::read_to_string(&backups[0]).unwrap();
        serde_json::from_str::<serde_json::Value>(&content)
            .expect("backup must be valid JSON");
        // Verify it's the pre-migration snapshot (legacy field still present).
        assert!(content.contains("sttPriority"), "backup must preserve pre-migration fields");
        assert!(content.contains("gsk-test"), "backup must preserve user data (api key)");
    }

    /// Migration write failure is propagated to the warnings vec (Linux only).
    #[test]
    #[cfg(unix)]
    fn test_migration_write_error_propagated_to_warnings() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_dir();
        // Legacy config that triggers the hotkey_slots migration, carrying a
        // secret so we can assert AC-4 (keys survive a failed migration save).
        let legacy = r#"{"sttProvider": "groq", "llmProvider": "deepseek", "hotkey": "ctrl+alt+r", "groqApiKey": "gsk-survival-test"}"#;
        std::fs::write(dir.path().join("config.json"), legacy.as_bytes()).unwrap();

        // Make the directory read-only so save_config (and the backup write) fail.
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

        let mut warnings: Vec<String> = Vec::new();
        let _ = load_config_reporting(dir.path(), &mut warnings);

        // Restore permissions so TempDir can clean up on drop.
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            warnings
                .iter()
                .any(|w| w.contains("migration") && w.contains("could not be saved")),
            "migration write failure must be propagated to warnings; got: {warnings:?}"
        );

        // AC-4: the on-disk config.json must still hold the user's keys/license
        // after a failed migration save — save_config never overwrote the
        // in-place file, so the pre-migration original (with the secret) stays.
        let on_disk = std::fs::read_to_string(dir.path().join("config.json"))
            .expect("config.json must still be readable after a failed migration save");
        assert!(
            on_disk.contains("gsk-survival-test"),
            "keys/license must survive a failed migration save on disk; got: {on_disk}"
        );
    }

    // -----------------------------------------------------------------------
    // Story 1.2 — Backup-on-corrupt recovery in load_config (ROB-02 / ADR-0015)
    // -----------------------------------------------------------------------

    /// Collect all `config.json.corrupt-*` backup files present in `dir`.
    /// The `<unix_ts>` suffix is non-deterministic, so tests glob by prefix.
    fn corrupt_backups(dir: &Path) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .expect("read dir")
            .map(|e| e.expect("dir entry").path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("config.json.corrupt-"))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// (a) Corrupt JSON → defaults returned; exactly one timestamped backup
    /// exists whose bytes equal the original corrupt content; warning recorded.
    #[test]
    fn test_corrupt_json_is_backed_up_with_warning() {
        let dir = temp_dir();
        let corrupt: &[u8] = br#"{ "groqApiKey": "gsk_brokenJSON_no_closing_brace "#;
        std::fs::write(dir.path().join(CONFIG_FILE), corrupt).unwrap();

        let mut warnings = Vec::new();
        let _cfg = load_config_reporting(dir.path(), &mut warnings);

        let backups = corrupt_backups(dir.path());
        assert_eq!(backups.len(), 1, "exactly one corrupt backup expected, got {backups:?}");
        let backed_up = std::fs::read(&backups[0]).expect("read backup");
        assert_eq!(
            backed_up.as_slice(),
            corrupt,
            "backup must contain the original corrupt bytes verbatim"
        );
        assert!(!warnings.is_empty(), "a user-facing warning must be recorded");
    }

    /// (b) Recoverability → a corrupt file embedding a known api-key-like
    /// substring is preserved in the backup (real data, not just any file).
    #[test]
    fn test_corrupt_backup_preserves_recoverable_data() {
        let dir = temp_dir();
        let secret = "gsk_live_RECOVERABLE_KEY_123";
        // Valid JSON object followed by trailing junk → serde parse error.
        let corrupt = format!("{{\"groqApiKey\":\"{secret}\"}} <<<corrupt tail>>>");
        std::fs::write(dir.path().join(CONFIG_FILE), corrupt.as_bytes()).unwrap();

        let mut warnings = Vec::new();
        let _cfg = load_config_reporting(dir.path(), &mut warnings);

        let backups = corrupt_backups(dir.path());
        assert_eq!(backups.len(), 1, "exactly one corrupt backup expected, got {backups:?}");
        let backed_up = std::fs::read_to_string(&backups[0]).expect("read backup");
        assert!(
            backed_up.contains(secret),
            "backup must preserve the recoverable key substring, got: {backed_up}"
        );
    }

    /// (c) NotFound → no file present; defaults returned; NO backup created and
    /// NO warning recorded. Deliberately asserts ABSENCE (non-tautological).
    #[test]
    fn test_missing_config_is_not_backed_up() {
        let dir = temp_dir();
        // No config.json written at all.
        let mut warnings = Vec::new();
        let _cfg = load_config_reporting(dir.path(), &mut warnings);

        assert!(
            corrupt_backups(dir.path()).is_empty(),
            "an absent file must NOT create a corrupt backup"
        );
        assert!(warnings.is_empty(), "an absent file must NOT record a warning");
    }

    /// (d) Read error → non-UTF-8 bytes fail `read_to_string` (catch-all Err
    /// arm); best-effort backup of the raw bytes exists; warning recorded.
    #[test]
    fn test_unreadable_non_utf8_config_is_backed_up() {
        let dir = temp_dir();
        // Invalid UTF-8 → read_to_string returns Err (kind != NotFound) → AC#4 arm.
        let raw: &[u8] = &[0xff, 0xfe, 0x00, 0x9c, 0x80];
        std::fs::write(dir.path().join(CONFIG_FILE), raw).unwrap();

        let mut warnings = Vec::new();
        let _cfg = load_config_reporting(dir.path(), &mut warnings);

        let backups = corrupt_backups(dir.path());
        assert_eq!(backups.len(), 1, "a read error must trigger a best-effort backup");
        let backed_up = std::fs::read(&backups[0]).expect("read backup");
        assert_eq!(
            backed_up.as_slice(),
            raw,
            "backup must contain the raw unreadable bytes verbatim"
        );
        assert!(!warnings.is_empty(), "a warning must be recorded for the read error");
    }

    /// (e) ROB-02 end-to-end (AC#2) → the default returned for a corrupt file
    /// has `first_install_at == 0` (the exact trigger of the lib.rs first-install
    /// overwrite); after simulating that overwrite via `save_config`, the corrupt
    /// backup STILL exists → the repairable→total-loss transition is impossible.
    #[test]
    fn test_rob02_backup_survives_first_install_overwrite() {
        let dir = temp_dir();
        let corrupt: &[u8] = br#"<<< not json at all >>>"#;
        std::fs::write(dir.path().join(CONFIG_FILE), corrupt).unwrap();

        let mut warnings = Vec::new();
        let cfg = load_config_reporting(dir.path(), &mut warnings);

        // This is the exact condition that triggers lib.rs:717's overwrite.
        assert_eq!(
            cfg.first_install_at, 0,
            "default must have first_install_at == 0 (the ROB-02 overwrite trigger)"
        );

        let backups_before = corrupt_backups(dir.path());
        assert_eq!(backups_before.len(), 1, "corrupt backup must exist after load");

        // Simulate the lib.rs:722 first-install overwrite: stamp first_install_at
        // and persist a fresh default over the on-disk config.json.
        let mut fresh = cfg.clone();
        fresh.first_install_at = 1_700_000_000;
        save_config(dir.path(), &fresh).expect("save_config (simulated first-install overwrite)");

        let backups_after = corrupt_backups(dir.path());
        assert_eq!(
            backups_after, backups_before,
            "corrupt backup must survive the first-install overwrite"
        );
    }

    /// (f) No stray temps → after a corrupt-backup the dir contains only
    /// `config.json` (rewritten fresh by the default-save during migration) plus
    /// one `config.json.corrupt-<ts>`; no leftover `save_atomic` temp file.
    #[test]
    fn test_corrupt_backup_leaves_no_stray_temp_files() {
        let dir = temp_dir();
        let corrupt: &[u8] = br#"{ broken "#;
        std::fs::write(dir.path().join(CONFIG_FILE), corrupt).unwrap();

        let mut warnings = Vec::new();
        let _cfg = load_config_reporting(dir.path(), &mut warnings);

        let mut names: Vec<String> = std::fs::read_dir(dir.path())
            .expect("read dir")
            .map(|e| e.expect("dir entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(
            names.len(),
            2,
            "expected exactly config.json + one corrupt backup, got {names:?}"
        );
        assert!(names.iter().any(|n| n == CONFIG_FILE), "config.json must be present");
        assert!(
            names.iter().any(|n| n.starts_with("config.json.corrupt-")),
            "a corrupt backup must be present, got {names:?}"
        );
    }

    /// (g) Truly unreadable (AC#4 inner branch) → when `config.json` cannot be
    /// read EITHER as a string OR as raw bytes, `load_config` must fall back to
    /// defaults WITHOUT a backup and WITHOUT a warning, and must never panic.
    /// Pins the `Err(read_err)` arm (`load_config_reporting`, the inner branch
    /// after `read_to_string` AND `std::fs::read` both fail) that spec (d) — where
    /// the raw re-read SUCCEEDS — never reaches. The trigger makes `config.json` a
    /// DIRECTORY: both reads fail with a kind that is NOT `NotFound`. Chosen over
    /// `chmod 000` because it is root-independent (CI often runs as root, where
    /// permission bits are bypassed and a `chmod`-based trigger silently misfires).
    #[test]
    fn test_truly_unreadable_config_no_backup_no_warning() {
        let dir = temp_dir();
        // A directory at the config path: `read_to_string` and `read` both Err
        // with kind != NotFound → catch-all arm, then the inner `Err(read_err)`.
        std::fs::create_dir(dir.path().join(CONFIG_FILE)).unwrap();

        let mut warnings = Vec::new();
        let cfg = load_config_reporting(dir.path(), &mut warnings);

        // Boot continues on defaults; `first_install_at` is env-fallback-independent.
        assert_eq!(
            cfg.first_install_at, 0,
            "a truly-unreadable config must fall back to default (first_install_at == 0)"
        );
        // AC#4: NO backup for the truly-unreadable sub-branch (line is log-only).
        assert!(
            corrupt_backups(dir.path()).is_empty(),
            "a file unreadable even as raw bytes must NOT create a corrupt backup"
        );
        // AC#4: NO warning — distinguishes this from spec (d)'s raw-read-succeeds path.
        assert!(
            warnings.is_empty(),
            "the truly-unreadable sub-branch must NOT record a warning, got {warnings:?}"
        );
    }

    /// (h) Backup-write failure (AC#5) → when `save_atomic` itself fails, the
    /// private `backup_corrupt_config` helper must DOWNGRADE to a "could not be
    /// backed up" warning, never panic, and never propagate an error (it returns
    /// `()`), so boot is never blocked. Pins the `save_atomic` Err arm untouched by
    /// specs (a)-(f). Trigger: a backup target whose PARENT directory does not
    /// exist — `fs::tests::test_save_atomic_errors_when_parent_missing` proves
    /// `save_atomic` errors here, and `NamedTempFile::new_in` on a missing dir fails
    /// regardless of uid (root-independent, unlike a read-only-dir `chmod`).
    #[test]
    fn test_backup_write_failure_records_degraded_warning() {
        let dir = temp_dir();
        // Parent (`does_not_exist/`) is absent → save_atomic returns Err.
        let unwritable = dir.path().join("does_not_exist").join(CONFIG_FILE);

        let mut warnings = Vec::new();
        // Direct call to the in-module-private helper (reachable via `use super::*`).
        // Returning normally (no panic / unwind) exercises the infallible contract.
        backup_corrupt_config(&unwritable, b"recoverable bytes", &mut warnings);

        assert_eq!(warnings.len(), 1, "exactly one degraded warning must be recorded");
        assert!(
            warnings[0].contains("could not be backed up"),
            "the DEGRADED warning must signal the failed backup (not the success text), got: {}",
            warnings[0]
        );
        // The write failed → no `config.json.corrupt-*` left anywhere under the temp dir.
        assert!(
            corrupt_backups(dir.path()).is_empty(),
            "a failed backup write must not leave a config.json.corrupt-* file"
        );
    }

    /// (i) Warning content (D1 / AC#1) → on a successful corrupt-backup the
    /// recorded warning must NAME the actual backup file so the user knows where to
    /// recover from, and carry the `config.json.corrupt-` prefix (locking the
    /// string-built name against a `with_extension` regression that would yield
    /// `config.corrupt`). Specs (a)/(b)/(d) only assert the warning is non-empty —
    /// they would still pass with a content-free or misleading message.
    #[test]
    fn test_corrupt_backup_warning_names_recovery_file() {
        let dir = temp_dir();
        let corrupt: &[u8] = br#"{ "groqApiKey": "gsk_broken "#;
        std::fs::write(dir.path().join(CONFIG_FILE), corrupt).unwrap();

        let mut warnings = Vec::new();
        let _cfg = load_config_reporting(dir.path(), &mut warnings);

        let backups = corrupt_backups(dir.path());
        assert_eq!(backups.len(), 1, "exactly one corrupt backup expected, got {backups:?}");
        assert_eq!(warnings.len(), 1, "exactly one warning recorded");

        let backup_name = backups[0]
            .file_name()
            .and_then(|n| n.to_str())
            .expect("backup file name");
        assert!(
            warnings[0].contains(backup_name),
            "warning must name the actual backup file '{backup_name}' for recovery (D1), got: {}",
            warnings[0]
        );
        assert!(
            warnings[0].contains("config.json.corrupt-"),
            "warning must carry the string-built name, not a with_extension 'config.corrupt', got: {}",
            warnings[0]
        );
    }

    /// (j) Valid config (false-positive guard) → a fully valid, parseable
    /// `config.json` must produce NO corrupt backup and NO warning, and its parsed
    /// contents must survive (not be silently defaulted). Guards against a
    /// regression that backs up / warns unconditionally — the inverse of AC#1, for
    /// which no prior spec asserted the happy path stays clean.
    #[test]
    fn test_valid_config_records_no_backup_or_warning() {
        let dir = temp_dir();
        // A non-default, valid config written through the real serializer.
        let saved = AppConfig {
            groq_api_key: "valid-sentinel-key".to_string(),
            ..AppConfig::default()
        };
        save_config(dir.path(), &saved).expect("save valid config");

        let mut warnings = Vec::new();
        let cfg = load_config_reporting(dir.path(), &mut warnings);

        assert!(warnings.is_empty(), "a valid config must NOT record a warning, got {warnings:?}");
        assert!(
            corrupt_backups(dir.path()).is_empty(),
            "a valid config must NOT create a config.json.corrupt-* backup"
        );
        // Non-empty key is untouched by the env-var fallback → contents truly round-tripped.
        assert_eq!(
            cfg.groq_api_key, "valid-sentinel-key",
            "the valid config's contents must round-trip through load, not be defaulted"
        );
    }
}
