#![cfg(feature = "v1-import")]
//! v1 → v2 Einmal-Import (Parse-Only-Bundle).
//!
//! Reads Klarvo v1 AppData on Windows (`%APPDATA%\com.klarvo.voice\`) and
//! produces a `V1ImportBundle` in memory. Does **not** write to any v2
//! store — Phase-1 writer will wire the bundle to the final stores once
//! those schemas exist.
//!
//! See `docs/adr/0004-v1-to-v2-migration-strategy.md` and
//! `docs/migration/v1-to-v2.md`.

use std::path::{Path, PathBuf};

pub mod config;
pub mod dictionary;
pub mod history;
pub mod keys;

#[cfg(test)]
pub(crate) mod test_util;

pub use config::V1Settings;
pub use dictionary::V1Dictionary;
pub use history::{V1HistoryEntry, V1UsageEntry};
pub use keys::V1ApiKeys;

/// v1 Tauri identifier (source: v1 `src-tauri/tauri.conf.json`).
pub const V1_APP_IDENTIFIER: &str = "com.klarvo.voice";

/// Warnings accumulated during a partial migration.
///
/// Each variant names the offending file (or SQLite table) so the Phase-1
/// writer can surface the list to the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum V1ImportWarning {
    /// The AppData root directory does not exist.
    AppDataMissing { path: PathBuf },
    /// A known v1 file is missing inside AppData.
    FileMissing { file: &'static str },
    /// A known v1 file exists but could not be parsed.
    ParseError { file: &'static str, detail: String },
    /// A single SQLite row was malformed; the row was skipped, the rest of the table was still imported.
    RowSkipped { table: &'static str, detail: String },
}

/// Parse-only bundle of v1 data. See ADR-0004.
///
/// Not `Serialize` by design: API-keys are held as `SecretString` and the
/// bundle is for in-memory hand-off to a writer only.
#[derive(Debug)]
pub struct V1ImportBundle {
    pub history: Option<Vec<V1HistoryEntry>>,
    pub usage: Option<Vec<V1UsageEntry>>,
    pub settings: Option<V1Settings>,
    pub api_keys: V1ApiKeys,
    pub dictionary: Option<V1Dictionary>,
    pub warnings: Vec<V1ImportWarning>,
}

impl V1ImportBundle {
    fn empty() -> Self {
        Self {
            history: None,
            usage: None,
            settings: None,
            api_keys: V1ApiKeys::empty(),
            dictionary: None,
            warnings: Vec::new(),
        }
    }

    /// `true` if at least one section was successfully loaded.
    pub fn is_nonempty(&self) -> bool {
        self.history.is_some()
            || self.usage.is_some()
            || self.settings.is_some()
            || !self.api_keys.is_empty()
            || self.dictionary.is_some()
    }
}

/// Default production path: `%APPDATA%\com.klarvo.voice\` on Windows.
/// Returns `None` on non-Windows platforms (and when `%APPDATA%` is unset).
pub fn resolve_default_v1_path() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(|p| PathBuf::from(p).join(V1_APP_IDENTIFIER))
    } else {
        None
    }
}

/// Load a v1 AppData directory from an explicit path (for tests + injected paths).
///
/// Best-effort per section — see ADR-0004 §4. Returns a bundle whose
/// `warnings` field reports every missing/corrupted file and skipped row.
pub fn load_from_path(appdata: &Path) -> V1ImportBundle {
    if !appdata.exists() {
        let mut bundle = V1ImportBundle::empty();
        bundle.warnings.push(V1ImportWarning::AppDataMissing {
            path: appdata.to_path_buf(),
        });
        return bundle;
    }

    let mut warnings = Vec::new();
    let (history, usage) = history::load(appdata, &mut warnings);
    let (settings, api_keys) = config::load(appdata, &mut warnings);
    let dictionary = dictionary::load(appdata, &mut warnings);

    V1ImportBundle {
        history,
        usage,
        settings,
        api_keys,
        dictionary,
        warnings,
    }
}

/// Load from the production default path, or return a bundle containing
/// only an `AppDataMissing` warning if the default cannot be resolved.
pub fn load_default() -> V1ImportBundle {
    match resolve_default_v1_path() {
        Some(path) => load_from_path(&path),
        None => {
            let mut bundle = V1ImportBundle::empty();
            bundle.warnings.push(V1ImportWarning::AppDataMissing {
                path: PathBuf::from("<no APPDATA>"),
            });
            bundle
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bundle_reports_not_nonempty() {
        let bundle = V1ImportBundle::empty();
        assert!(!bundle.is_nonempty());
        assert!(bundle.warnings.is_empty());
    }

    #[test]
    fn missing_appdata_yields_single_warning() {
        let nowhere = PathBuf::from("/definitely/not/a/real/path/klarvo-v1");
        let bundle = load_from_path(&nowhere);
        assert!(!bundle.is_nonempty());
        assert_eq!(bundle.warnings.len(), 1);
        assert!(matches!(
            &bundle.warnings[0],
            V1ImportWarning::AppDataMissing { .. }
        ));
    }

    #[test]
    fn empty_appdata_dir_yields_three_file_missing_warnings() {
        let tmp = test_util::tempdir();
        let bundle = load_from_path(tmp.path());
        assert!(!bundle.is_nonempty());
        let missing: Vec<_> = bundle
            .warnings
            .iter()
            .filter_map(|w| match w {
                V1ImportWarning::FileMissing { file } => Some(*file),
                _ => None,
            })
            .collect();
        assert_eq!(missing.len(), 3);
        assert!(missing.contains(&"history.db"));
        assert!(missing.contains(&"config.json"));
        assert!(missing.contains(&"dictionary.json"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn resolve_default_returns_appdata_path_on_windows() {
        // If APPDATA is set, the resolver must include the v1 identifier segment.
        if let Some(path) = resolve_default_v1_path() {
            assert!(path.ends_with(V1_APP_IDENTIFIER));
        }
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn resolve_default_returns_none_on_non_windows() {
        assert!(resolve_default_v1_path().is_none());
    }

}
