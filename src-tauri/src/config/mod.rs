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
// Configuration struct
// ---------------------------------------------------------------------------

/// Persisted application settings.
///
/// All fields have defaults via `Default` so a partially-written or
/// absent config file always yields a usable value.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    /// Groq API key for the Whisper STT provider.
    #[serde(default)]
    pub groq_api_key: String,

    /// DeepSeek API key for the LLM cleanup provider.
    #[serde(default)]
    pub deepseek_api_key: String,

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
}

fn default_language() -> String {
    "de".to_string()
}

fn default_cleanup_style() -> CleanupStyle {
    CleanupStyle::Polished
}

fn default_hotkey() -> String {
    "ctrl+shift+d".to_string()
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            groq_api_key: String::new(),
            deepseek_api_key: String::new(),
            language: default_language(),
            cleanup_style: default_cleanup_style(),
            hotkey: default_hotkey(),
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

    config
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
        assert_eq!(cfg.language, "de");
        assert_eq!(cfg.cleanup_style, CleanupStyle::Polished);
        assert_eq!(cfg.hotkey, "ctrl+shift+d");
    }

    /// Loading from a non-existent directory returns defaults without panicking.
    #[test]
    fn test_load_config_missing_file_returns_defaults() {
        let dir = temp_dir();
        let cfg = load_config(dir.path());
        assert_eq!(cfg, AppConfig::default());
    }

    /// Save then load round-trips the config correctly.
    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = temp_dir();

        let original = AppConfig {
            groq_api_key: "groq-test-key-abc".to_string(),
            deepseek_api_key: "ds-test-key-xyz".to_string(),
            language: "en".to_string(),
            cleanup_style: CleanupStyle::Chat,
            hotkey: "ctrl+alt+r".to_string(),
        };

        save_config(dir.path(), &original).expect("save should succeed");

        let loaded = load_config(dir.path());
        assert_eq!(loaded, original);
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
        assert_eq!(cfg.language, "de");
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
        assert!(json.contains("cleanupStyle"), "expected camelCase 'cleanupStyle'");
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
}
