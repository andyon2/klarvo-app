//! Tiny `TempDir` helper shared across v1_import unit tests.
//!
//! Avoids pulling in `tempfile` as a dev-dep just for a few parse tests.

use std::path::{Path, PathBuf};

pub(crate) struct TempDir(PathBuf);

impl TempDir {
    pub(crate) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn tempdir() -> TempDir {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("klarvo-v1-import-test-{pid}-{n}"));
    std::fs::create_dir_all(&path).expect("create tempdir");
    TempDir(path)
}
