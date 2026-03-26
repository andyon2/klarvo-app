//! Shared test utilities used across multiple test modules.
//!
//! Re-exported from here so each module doesn't need to duplicate setup logic.

use tempfile::TempDir;

use crate::config::AppConfig;
use crate::dictionary::Dictionary;
use crate::AppState;

/// Creates a temporary directory that lives as long as the returned `TempDir`.
pub fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("failed to create temp dir")
}

/// Builds a minimal [`AppState`] backed by an in-memory SQLite database.
///
/// Uses default [`AppConfig`] and an empty [`Dictionary`].  The `app_data_dir`
/// is set to the supplied `TempDir` path.
pub fn make_state(dir: &TempDir) -> AppState {
    let db = rusqlite::Connection::open_in_memory()
        .expect("in-memory SQLite must always open successfully");
    AppState::new(
        AppConfig::default(),
        Dictionary::new(),
        dir.path().to_path_buf(),
        db,
        false, // is_early_adopter
    )
}
