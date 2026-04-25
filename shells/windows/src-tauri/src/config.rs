// Windows-only: depends on APPDATA env var. Crate-level cfg-gate in main.rs/lib.rs applies.
use std::path::{Path, PathBuf};

use klarvo_core::{AppError, AppErrorKind};

/// Typed representation of `%APPDATA%\Klarvo\config.toml`.
///
/// All fields have sensible defaults so an empty `config.toml` is valid.
/// `#[serde(deny_unknown_fields)]` rejects unrecognised keys immediately with a
/// clear error (ref `feedback_manifest_compile_contract`).
///
/// Story 4.1 replaced the single `locale` field with three independent i18n axes
/// (`ui_language`/`dictionary_language`/`output_language`) per FR26 and
/// `memory/project_i18n_three_axes`. v1→v2 config-migration is Epic 7 scope;
/// Phase-1 has no live testers (ref `memory/project_ea_withdrawn`), so a hard
/// replace without a `#[serde(alias = "locale")]` shim is acceptable.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShellConfig {
    /// Global hotkey for push-to-talk (default from PRD `scopeLock.hotkeyDefault`).
    #[serde(default = "ShellConfig::default_hotkey")]
    pub hotkey: String,

    /// Registry ID of the active output-target plugin (default: clipboard).
    #[serde(default = "ShellConfig::default_output_target")]
    pub output_target_id: String,

    /// UI-language axis (Shell-rendered strings). Supported values: `en`, `de`.
    /// Consumed by Story 4.2 for locale-aware i18n-table loading.
    #[serde(default = "ShellConfig::default_ui_language")]
    pub ui_language: String,

    /// Dictionary-language axis (Plugin-Dictionary-Lookups). Supported values: `en`, `de`.
    /// Phase-1: not consumed by any plugin. Phase-2+ will route this into
    /// dictionary-aware STT/cleanup plugins (PRD FR26 axis 2).
    #[serde(default = "ShellConfig::default_dictionary_language")]
    pub dictionary_language: String,

    /// Output-language axis (Cleanup-Stage target language). Supported values: `en`, `de`.
    /// Phase-1: not consumed by any plugin. Phase-2+ will route this into
    /// cleanup/translation plugins (PRD FR26 axis 3).
    #[serde(default = "ShellConfig::default_output_language")]
    pub output_language: String,
}

impl ShellConfig {
    fn default_hotkey() -> String {
        "CommandOrControl+Shift+Space".to_string()
    }

    fn default_output_target() -> String {
        "clipboard".to_string()
    }

    fn default_ui_language() -> String {
        "en".to_string()
    }

    fn default_dictionary_language() -> String {
        "en".to_string()
    }

    fn default_output_language() -> String {
        "en".to_string()
    }
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            hotkey: ShellConfig::default_hotkey(),
            output_target_id: ShellConfig::default_output_target(),
            ui_language: ShellConfig::default_ui_language(),
            dictionary_language: ShellConfig::default_dictionary_language(),
            output_language: ShellConfig::default_output_language(),
        }
    }
}

/// Resolve `%APPDATA%\Klarvo\config.toml` from the environment.
///
/// Pure resolver — does not create the directory or open the file.
///
/// # Errors
///
/// Returns `AppError { kind: Configuration, user_message: Some("error.config.missing") }`
/// when the `APPDATA` environment variable is not set.
pub fn resolve_config_path() -> Result<PathBuf, AppError> {
    let appdata = std::env::var("APPDATA").map_err(|_| AppError {
        kind: AppErrorKind::Configuration,
        message: "APPDATA environment variable not set".to_string(),
        user_message: Some("error.config.missing".to_string()),
        retryable: false,
    })?;
    Ok(PathBuf::from(appdata).join("Klarvo").join("config.toml"))
}

/// Parse and validate a TOML string into `ShellConfig`.
///
/// Separated from file I/O so unit tests can operate on in-memory strings directly
/// without touching the filesystem (see `#[cfg(test)]` module).
fn parse_from_str(raw: &str) -> Result<ShellConfig, AppError> {
    let config: ShellConfig = toml::from_str(raw).map_err(|e| {
        let msg = e.to_string();
        let user_key = if msg.contains("unknown field") {
            "error.config.unknown_field"
        } else {
            "error.config.missing"
        };
        AppError {
            kind: AppErrorKind::Configuration,
            message: msg,
            user_message: Some(user_key.to_string()),
            retryable: false,
        }
    })?;

    for (field_name, value) in [
        ("ui_language", &config.ui_language),
        ("dictionary_language", &config.dictionary_language),
        ("output_language", &config.output_language),
    ] {
        if !matches!(value.as_str(), "en" | "de") {
            return Err(AppError {
                kind: AppErrorKind::Configuration,
                message: format!("unsupported {field_name}: {value}"),
                user_message: Some("error.config.invalid_language".to_string()),
                retryable: false,
            });
        }
    }

    Ok(config)
}

/// Load and validate `config.toml` from `path`.
///
/// # Path convention
///
/// Use [`resolve_config_path`] to obtain the canonical Windows path
/// (`%APPDATA%\Klarvo\config.toml`).
///
/// # Error paths
///
/// | Situation | `AppErrorKind` | `user_message` key |
/// |-----------|---------------|--------------------|
/// | File not found | `Configuration` | `error.config.missing` |
/// | Unknown field in TOML | `Configuration` | `error.config.unknown_field` |
/// | Corrupt / unparseable TOML | `Configuration` | `error.config.missing` |
/// | Unsupported language axis (ui/dictionary/output) | `Configuration` | `error.config.invalid_language` |
///
/// // Phase-2: Settings-UI creates config.toml on first-run via xtask or settings-save.
/// // Phase-1: user creates config.toml manually. Missing file is not auto-generated.
/// // Story 3.10 wires ShellConfig into tauri::State<Arc<ShellConfig>>.
pub fn load_config(path: &Path) -> Result<ShellConfig, AppError> {
    let raw = std::fs::read_to_string(path).map_err(|e| AppError {
        kind: AppErrorKind::Configuration,
        message: format!("{}: {}", path.display(), e),
        user_message: Some("error.config.missing".to_string()),
        retryable: false,
    })?;
    parse_from_str(&raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_empty_toml_uses_defaults() {
        let cfg = parse_from_str("").unwrap();
        assert_eq!(cfg.hotkey, "CommandOrControl+Shift+Space");
        assert_eq!(cfg.output_target_id, "clipboard");
        assert_eq!(cfg.ui_language, "en");
        assert_eq!(cfg.dictionary_language, "en");
        assert_eq!(cfg.output_language, "en");
    }

    #[test]
    fn happy_path_explicit_values() {
        let cfg = parse_from_str(
            "hotkey = \"CommandOrControl+Shift+Space\"\n\
             output_target_id = \"clipboard\"\n\
             ui_language = \"de\"\n\
             dictionary_language = \"en\"\n\
             output_language = \"de\"",
        )
        .unwrap();
        assert_eq!(cfg.hotkey, "CommandOrControl+Shift+Space");
        assert_eq!(cfg.output_target_id, "clipboard");
        assert_eq!(cfg.ui_language, "de");
        assert_eq!(cfg.dictionary_language, "en");
        assert_eq!(cfg.output_language, "de");
    }

    #[test]
    fn unknown_field_rejected() {
        let err = parse_from_str("hotkey = \"X\"\nunknown_key = \"Y\"").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Configuration));
        assert_eq!(err.user_message.as_deref(), Some("error.config.unknown_field"));
    }

    /// After Story 4.1 hard-replace, the legacy `locale` field is itself an unknown field —
    /// so providing it (regardless of value) trips `deny_unknown_fields`, not the
    /// per-axis validation. v1→v2 config-migration is Epic 7 scope; Phase-1 has no
    /// live testers (`memory/project_ea_withdrawn`).
    #[test]
    fn legacy_locale_field_rejected_as_unknown_field() {
        let err = parse_from_str("locale = \"fr\"").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Configuration));
        assert_eq!(err.user_message.as_deref(), Some("error.config.unknown_field"));
    }

    #[test]
    fn invalid_ui_language_rejected() {
        let err = parse_from_str("ui_language = \"fr\"").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Configuration));
        assert_eq!(err.user_message.as_deref(), Some("error.config.invalid_language"));
        assert!(err.message.contains("ui_language"), "message should identify failing axis: {}", err.message);
    }

    #[test]
    fn invalid_dictionary_language_rejected() {
        let err = parse_from_str("dictionary_language = \"es\"").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Configuration));
        assert_eq!(err.user_message.as_deref(), Some("error.config.invalid_language"));
        assert!(err.message.contains("dictionary_language"), "message should identify failing axis: {}", err.message);
    }

    #[test]
    fn invalid_output_language_rejected() {
        let err = parse_from_str("output_language = \"it\"").unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Configuration));
        assert_eq!(err.user_message.as_deref(), Some("error.config.invalid_language"));
        assert!(err.message.contains("output_language"), "message should identify failing axis: {}", err.message);
    }

    #[test]
    fn mixed_languages_independent_axes() {
        let cfg = parse_from_str(
            "ui_language = \"de\"\n\
             dictionary_language = \"en\"\n\
             output_language = \"de\"",
        )
        .unwrap();
        assert_eq!(cfg.ui_language, "de");
        assert_eq!(cfg.dictionary_language, "en");
        assert_eq!(cfg.output_language, "de");
    }

    #[test]
    fn missing_file_returns_error() {
        let err =
            load_config(Path::new("/definitely/does/not/exist/config.toml")).unwrap_err();
        assert!(matches!(err.kind, AppErrorKind::Configuration));
        assert_eq!(err.user_message.as_deref(), Some("error.config.missing"));
    }
}
