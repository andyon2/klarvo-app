//! Rolling-file tracing subscriber for Klarvo.
//!
//! # NFR5 — PII Protection
//!
//! Audio-Daten (PCM-Samples, Rohaudio) und Transkriptions-Text (STT-Output,
//! LLM-Output) DÜRFEN NICHT geloggt werden — weder in DEBUG- noch in
//! TRACE-Events. Logging beschränkt sich auf Metadata: Event-Typen,
//! Error-Keys, Latency-Werte (ts_ms), Plugin-IDs, Byte-Counts.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;

/// Maximum log level for release builds (INFO and above).
///
/// This constant is also used as a grep-sentinel by `xtask verify-release`
/// (Story 6.2) to confirm the release filter is in place.
#[cfg(not(debug_assertions))]
pub const RELEASE_MAX_LEVEL: LevelFilter = LevelFilter::INFO;

/// Maximum log level for debug builds (DEBUG and above).
#[cfg(debug_assertions)]
pub const RELEASE_MAX_LEVEL: LevelFilter = LevelFilter::DEBUG;

/// Initialise a global rolling-file tracing subscriber.
///
/// Creates `log_dir` if it does not exist, then sets up a daily-rotating file
/// appender with a maximum of 5 retained files. The subscriber is installed
/// globally via [`tracing::subscriber::set_global_default`].
///
/// Returns `Some(guard)` on success. The caller **must** keep the guard alive
/// for the full process lifetime — dropping it flushes and closes the writer,
/// causing all subsequent log events to be silently discarded.
///
/// Returns `None` (fail-soft) if the log directory cannot be created or the
/// appender builder fails. The application continues without file logging; no
/// panic is raised.
pub fn init_tracing(log_dir: &Path) -> Option<WorkerGuard> {
    if let Err(e) = std::fs::create_dir_all(log_dir) {
        eprintln!("[klarvo] failed to create log dir {}: {e}", log_dir.display());
        return None;
    }

    let file_appender = match tracing_appender::rolling::RollingFileAppender::builder()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("klarvo")
        .filename_suffix("log")
        .max_log_files(5)
        .build(log_dir)
    {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[klarvo] failed to build rolling file appender: {e}");
            return None;
        }
    };

    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let subscriber = tracing_subscriber::registry().with(
        fmt::layer()
            .with_ansi(false)
            .with_writer(non_blocking)
            .with_filter(RELEASE_MAX_LEVEL),
    );

    match tracing::subscriber::set_global_default(subscriber) {
        Ok(()) => Some(guard),
        Err(e) => {
            eprintln!("[klarvo] failed to set global tracing subscriber: {e}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that calling `init_tracing` with an uncreateable path returns
    /// `None` without panicking.
    ///
    /// Note: `set_global_default` can only succeed once per test binary.
    /// This test exercises only the error path (dir-creation failure) which
    /// returns before any subscriber is installed.
    #[test]
    fn init_tracing_with_uncreatable_dir_returns_none() {
        // A path containing a NUL byte is rejected before reaching the OS:
        // on Unix `CString::new` returns `NulError`; on Windows the
        // UTF-16 conversion path explicitly rejects strings with embedded
        // NUL bytes ("strings passed to WinAPI cannot contain NULs").
        // This is the most reliable cross-platform "uncreateable path".
        let bad_path = std::path::Path::new("\0klarvo_test_uncreatable");

        let result = init_tracing(bad_path);
        assert!(
            result.is_none(),
            "expected None for uncreateable log dir, got Some"
        );
    }
}
