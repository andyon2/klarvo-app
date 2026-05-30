use std::io::Write;
use std::path::Path;

use tempfile::NamedTempFile;

/// Atomically write `bytes` to `path`.
///
/// Writes to a temp file in the SAME directory as `path` (cross-device rename breaks atomicity),
/// fsyncs the contents, then atomically replaces the target via `NamedTempFile::persist`
/// (replaces an existing file atomically on all platforms; `persist` itself does not sync, so we
/// `sync_all` first). On crash between write and rename the previous file stays intact and the
/// orphan temp (random name) is never read as live config. Errors are returned, never swallowed.
pub fn save_atomic(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("save_atomic: path has no parent directory: {}", path.display()))?;
    let mut tmp = NamedTempFile::new_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.flush()?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?; // PersistError -> io::Error -> anyhow
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // (a) Happy path: writing to dir/config.json produces a file with exactly the bytes,
    // and NO leftover .tmp* files remain in the dir afterward.
    #[test]
    fn test_save_atomic_happy_path_no_leftover_temps() {
        let dir = tempdir().expect("failed to create temp dir");
        let target = dir.path().join("config.json");
        let bytes = b"hello world";

        save_atomic(&target, bytes).expect("save_atomic should succeed");

        let written = fs::read(&target).expect("target file should exist");
        assert_eq!(written, bytes);

        // Assert no leftover temp files in the dir (only config.json should be present)
        let entries: Vec<_> = fs::read_dir(dir.path())
            .expect("should read dir")
            .map(|e| e.expect("entry").file_name())
            .collect();
        assert_eq!(entries.len(), 1, "only the target file should remain, got: {:?}", entries);
        assert_eq!(entries[0], "config.json");
    }

    // (b) Atomic replace-over-existing: pre-create the target, then overwrite atomically.
    #[test]
    fn test_save_atomic_replaces_existing_file() {
        let dir = tempdir().expect("failed to create temp dir");
        let target = dir.path().join("config.json");

        fs::write(&target, b"old content").expect("pre-create file");
        save_atomic(&target, b"new content").expect("save_atomic should succeed");

        let result = fs::read(&target).expect("target file should exist");
        assert_eq!(result, b"new content");
    }

    // (c) Orphan temp ignored: a stray file in the dir does not affect save_atomic or load-back.
    #[test]
    fn test_save_atomic_unaffected_by_stray_sibling_file() {
        let dir = tempdir().expect("failed to create temp dir");
        let target = dir.path().join("config.json");
        let stray = dir.path().join(".tmpXYZ123_stray");

        // Create a stray file that looks like an orphaned temp
        fs::write(&stray, b"orphan bytes").expect("write stray file");

        let payload = b"real config content";
        save_atomic(&target, payload).expect("save_atomic should succeed");

        // The target reads back correctly
        let result = fs::read(&target).expect("target file should exist");
        assert_eq!(result, payload);

        // The stray file is still there (save_atomic does not touch unrelated files)
        assert!(stray.exists(), "stray file should remain untouched");
    }

    // (d) Error propagation: parent directory does not exist => returns Err, does not panic.
    #[test]
    fn test_save_atomic_errors_when_parent_missing() {
        let dir = tempdir().expect("failed to create temp dir");
        let nonexistent_parent = dir.path().join("does_not_exist").join("config.json");

        let result = save_atomic(&nonexistent_parent, b"data");
        assert!(result.is_err(), "should return Err when parent dir does not exist");
    }
}
