//! Debug-Export-Zip. Phase-2-Surface für Settings-UI (Epic 9, Story 9.5).
//!
//! Spec: architecture.md §9 Observability + prd.md FR40.
//!
//! NFR5: KEINE Audio/Text-Daten exportieren. Nur: Rolling-File-Logs (`*.log`)
//! aus `log_dir` + Sys-Info. Subdirs werden non-rekursiv übersprungen.

use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::error::{AppError, AppErrorKind};

fn make_export_err(msg: String) -> AppError {
    AppError {
        kind: AppErrorKind::ExportFailed,
        message: msg,
        user_message: Some("error.telemetry.export.failed".into()),
        retryable: false,
    }
}

/// Erzeugt ein Debug-Export-Zip am gegebenen Pfad.
///
/// Inhalt: `sysinfo.txt` (Version/OS/Arch/Timestamp) + alle `*.log`-Files aus
/// `log_dir` unter `logs/`. Nur Top-Level (non-recursive). NFR5: keine Audio-
/// oder Transkriptions-Daten — Allowlist auf `*.log` setzt das hart durch.
///
/// `app_version` wird vom Caller im Shell-Crate via `env!("CARGO_PKG_VERSION")`
/// evaluiert, damit der Bug-Report die User-sichtbare App-Version zeigt
/// (statt der `klarvo-core`-Crate-Version).
///
/// Atomarität: Der Zip wird in eine Tempfile neben `out_path` geschrieben und
/// erst bei Erfolg per `persist` umbenannt. Bei jedem Bail räumt `Drop` das
/// Tempfile auf — kein halb-geschriebenes Zip im Downloads-Ordner.
pub fn export_debug_zip(
    log_dir: &Path,
    out_path: &Path,
    app_version: &str,
) -> Result<(), AppError> {
    if let Some(parent) = out_path.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| make_export_err(format!("create parent dir: {e}")))?;
    }

    let parent = out_path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| make_export_err(format!("create tempfile: {e}")))?;

    {
        let mut zip = ZipWriter::new(tmp.as_file_mut());
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        let exported_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let sysinfo = format!(
            "klarvo_version: {}\nos: {}\narch: {}\nexported_at: {}\n",
            app_version,
            std::env::consts::OS,
            std::env::consts::ARCH,
            exported_at,
        );
        zip.start_file("sysinfo.txt", opts)
            .map_err(|e| make_export_err(format!("zip entry sysinfo.txt: {e}")))?;
        zip.write_all(sysinfo.as_bytes())
            .map_err(|e| make_export_err(format!("zip write sysinfo.txt: {e}")))?;

        if log_dir.exists() {
            let entries = std::fs::read_dir(log_dir)
                .map_err(|e| make_export_err(format!("read log_dir: {e}")))?;
            for entry in entries {
                let entry = entry.map_err(|e| make_export_err(format!("dir entry: {e}")))?;
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                // NFR5-Allowlist: ausschließlich `*.log`-Files einpacken.
                if path.extension().and_then(|e| e.to_str()) != Some("log") {
                    continue;
                }
                let filename = match path.file_name().and_then(|n| n.to_str()) {
                    Some(n) => n.to_owned(),
                    None => continue,
                };
                // Concurrent-Rotation-Race: rolling-appender kann mid-iteration
                // eine Datei rotieren oder löschen. Wir loggen nicht, sondern
                // skippen still — der Rest des Exports ist wertvoller als ein
                // Abort wegen einer rotierten Datei.
                let mut file = match std::fs::File::open(&path) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                zip.start_file(format!("logs/{filename}"), opts).map_err(|e| {
                    make_export_err(format!("zip entry logs/{filename}: {e}"))
                })?;
                if std::io::copy(&mut file, &mut zip).is_err() {
                    // Partial entry — ZipWriter hat den entry header schon
                    // geschrieben, aber das Zip selbst wird verworfen sobald
                    // wir bailen. Deshalb continue ist unsafe; wir bailen.
                    return Err(make_export_err(format!("zip write logs/{filename}")));
                }
            }
        }

        zip.finish()
            .map_err(|e| make_export_err(format!("zip finish: {e}")))?;
    }

    tmp.persist(out_path)
        .map_err(|e| make_export_err(format!("persist tempfile: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zip::ZipArchive;

    const TEST_VERSION: &str = "9.5.0-test";

    #[test]
    fn export_debug_zip_writes_sysinfo_txt() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("logs");
        let out_path = tmp.path().join("out.zip");

        export_debug_zip(&log_dir, &out_path, TEST_VERSION).expect("should succeed");

        let file = std::fs::File::open(&out_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        let mut sysinfo = archive.by_name("sysinfo.txt").unwrap();
        let mut contents = String::new();
        sysinfo.read_to_string(&mut contents).unwrap();
        assert!(contents.contains(&format!("klarvo_version: {TEST_VERSION}")));
        assert!(contents.contains("os:"));
        assert!(contents.contains("arch:"));
        assert!(contents.contains("exported_at:"));
    }

    #[test]
    fn export_debug_zip_nonexistent_log_dir_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("nonexistent_logs");
        let out_path = tmp.path().join("out.zip");

        let result = export_debug_zip(&log_dir, &out_path, TEST_VERSION);
        assert!(result.is_ok(), "non-existent log_dir must not error: {result:?}");

        let file = std::fs::File::open(&out_path).unwrap();
        let archive = ZipArchive::new(file).unwrap();
        assert_eq!(archive.len(), 1, "only sysinfo.txt expected");
    }

    #[test]
    fn export_debug_zip_creates_parent_dir_if_needed() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("logs");
        let out_path = tmp.path().join("subdir").join("nested").join("out.zip");

        export_debug_zip(&log_dir, &out_path, TEST_VERSION)
            .expect("should create parent dirs");
        assert!(out_path.exists());
    }

    #[test]
    fn export_debug_zip_only_packs_log_extension() {
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().to_path_buf();
        std::fs::write(log_dir.join("klarvo.log"), b"line\n").unwrap();
        std::fs::write(log_dir.join("audio.wav"), b"FAKE_WAV").unwrap();
        std::fs::write(log_dir.join("session.db"), b"FAKE_DB").unwrap();
        let out_path = tmp.path().join("out.zip");

        export_debug_zip(&log_dir, &out_path, TEST_VERSION).expect("should succeed");

        let file = std::fs::File::open(&out_path).unwrap();
        let archive = ZipArchive::new(file).unwrap();
        let names: Vec<_> = archive.file_names().collect();
        assert!(names.contains(&"sysinfo.txt"));
        assert!(names.contains(&"logs/klarvo.log"));
        assert!(
            !names.iter().any(|n| n.ends_with(".wav") || n.ends_with(".db")),
            "non-.log files leaked into export: {names:?}"
        );
    }

    #[test]
    fn export_debug_zip_no_partial_on_failure() {
        // out_path parent ist non-writable — File::create im persist failt.
        // Tempfile-Pattern garantiert: out_path entsteht NICHT.
        let tmp = tempfile::tempdir().unwrap();
        let log_dir = tmp.path().join("logs");
        // out_path zeigt auf ein Pfad, dessen Parent zwar erzeugt wird, aber
        // wir simulieren einen Persist-Fail durch out_path == existing-dir.
        let out_path = tmp.path().to_path_buf(); // out_path ist das tmpdir selbst → persist failt

        let result = export_debug_zip(&log_dir, &out_path, TEST_VERSION);
        assert!(result.is_err(), "expected persist to fail when out_path is a dir");
        if let Err(e) = result {
            assert!(matches!(e.kind, AppErrorKind::ExportFailed));
        }
    }
}
