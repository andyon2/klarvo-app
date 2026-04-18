//! Typed accessor for the committed v1-AppData snapshot.
//!
//! The binary fixture lives at `<repo-root>/test-assets/v1-appdata/` per
//! the architecture convention (binaries under `test-assets/`, typed
//! accessors in `klarvo-test-fixtures/`). See
//! `output/planning-artifacts/architecture.md` §Structure Patterns.

use std::path::PathBuf;

/// Absolute path to the v1-AppData fixture directory.
///
/// Resolved from `CARGO_MANIFEST_DIR` at compile time, so the path is
/// stable regardless of the test-runner's current working directory.
pub fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("klarvo-test-fixtures must have a workspace-root parent")
        .join("test-assets")
        .join("v1-appdata")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_dir_exists() {
        let p = fixture_path();
        assert!(p.is_dir(), "fixture missing: {}", p.display());
        assert!(p.join("history.db").is_file());
        assert!(p.join("config.json").is_file());
        assert!(p.join("dictionary.json").is_file());
    }
}
