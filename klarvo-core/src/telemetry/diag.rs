//! Pre-tracing boot-stage diagnostic markers (Story 12.2).
//!
//! Writes synchronously to `%APPDATA%\Klarvo\diag\boot-marker.txt` before the
//! `tracing-appender` subscriber is initialised. Useful for diagnosing silent-fail
//! boot paths where the rolling-file log is empty or absent.
//!
//! Each call appends one timestamped line. The file persists across runs so that
//! crash-on-restart sequences leave a breadcrumb trail.

use std::io::Write;
use std::path::{Path, PathBuf};

fn marker_path() -> PathBuf {
    std::env::var("APPDATA")
        .map(|d| {
            PathBuf::from(d)
                .join("Klarvo")
                .join("diag")
                .join("boot-marker.txt")
        })
        .unwrap_or_else(|_| {
            std::env::temp_dir()
                .join("Klarvo")
                .join("diag")
                .join("boot-marker.txt")
        })
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// Write a single boot-stage marker line synchronously.
///
/// Appends `[<unix_ms>ms] <stage>: <detail>\n` to the diagnostic file.
/// On open failure, retries with a fallback path inside `temp_dir`; as a last
/// resort writes to `stderr` via `eprintln!`.
pub fn write_boot_marker(stage: &str, detail: &str) {
    write_marker_to(&marker_path(), stage, detail);
}

fn write_marker_to(path: &Path, stage: &str, detail: &str) {
    let ts = now_millis();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let line = format!("[{ts}ms] {stage}: {detail}\n");
    let result = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .and_then(|mut f| f.write_all(line.as_bytes()));
    if result.is_err() {
        let fallback = std::env::temp_dir().join("klarvo-diag-fallback.txt");
        let _ = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&fallback)
            .and_then(|mut f| f.write_all(line.as_bytes()));
        eprintln!("[klarvo-diag] {stage}: {detail}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn writes_marker_to_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("boot-marker.txt");
        write_marker_to(&path, "Stage 0", "main() entered");
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Stage 0"));
        assert!(content.contains("main() entered"));
    }

    #[test]
    fn appends_multiple_markers_as_separate_lines() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("boot-marker.txt");
        write_marker_to(&path, "Stage 0", "main() entered");
        write_marker_to(&path, "Stage 1", "log_dir resolved = C:\\test\\logs");
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("Stage 0"));
        assert!(content.contains("Stage 1"));
        assert_eq!(content.lines().count(), 2);
    }

    #[test]
    fn marker_line_contains_timestamp_and_detail() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("boot-marker.txt");
        write_marker_to(&path, "Stage 2", "init_tracing returned Some");
        let content = fs::read_to_string(&path).unwrap();
        // timestamp format: [<ms>ms]
        assert!(content.contains("ms]"));
        assert!(content.contains("init_tracing returned Some"));
    }

    #[test]
    fn creates_parent_dirs_automatically() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("dirs").join("boot-marker.txt");
        write_marker_to(&path, "Stage 0", "test");
        assert!(path.exists());
    }

    #[test]
    fn fallback_on_unwriteable_path_does_not_panic() {
        // Pass a path whose parent is a file (cannot mkdir over it) so the
        // primary write fails, exercising the fallback path.
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("not-a-dir");
        fs::write(&blocker, b"block").unwrap();
        let bad_path = blocker.join("boot-marker.txt");
        // Must not panic — fallback + eprintln cover the failure.
        write_marker_to(&bad_path, "Stage 0", "test fallback");
    }
}
