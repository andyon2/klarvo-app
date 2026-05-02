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
            .with_target(true)
            .with_thread_ids(false)
            .with_thread_names(true)
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

/// Extract a human-readable message from a panic payload.
///
/// Tries `&'static str` then `String` downcasts; returns `"<non-string panic payload>"` for
/// other types. Extracted as a separately-testable helper for AC-6.
fn extract_panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&'static str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "<non-string panic payload>".to_owned())
}

/// Installs a global panic hook that captures uncaught panics as `tracing::error!` events.
///
/// Must be called after [`init_tracing`] to ensure the rolling-file subscriber is active.
/// Replaces the default panic handler entirely — the default hook (which prints to stderr)
/// is not chained. In release builds with `windows_subsystem = "windows"`, stderr is not
/// visible, so the default hook provides no value.
///
/// **`Backtrace::force_capture` (vs. `capture`):** chosen deliberately so backtraces are
/// always present in the rolling-file log, regardless of whether `RUST_BACKTRACE` is set
/// in the user's environment. The cost (multi-KB allocation + symbol resolution per panic)
/// is acceptable because uncaught panics are rare and the diagnostic value of a guaranteed
/// backtrace outweighs the cost. Revisit if Klarvo ever runs panic-throttling test loops.
///
/// **Reentry safety:** the hook body is wrapped in `catch_unwind` so a panic *inside* the
/// hook itself (e.g., the tracing subscriber's writer panicking on a full disk, the
/// backtrace symbolizer panicking, OOM during allocation) does not trigger Rust's
/// recursive-panic abort. On hook-internal panic, a last-resort line is written to stderr.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let message = extract_panic_message(info.payload());
            let (file, line, column) = info
                .location()
                .map_or(("<unknown>", 0u32, 0u32), |l| {
                    (l.file(), l.line(), l.column())
                });
            let backtrace = std::backtrace::Backtrace::force_capture();
            tracing::error!(
                panic.message = %message,
                panic.location.file = %file,
                panic.location.line = line,
                panic.location.column = column,
                panic.backtrace = %backtrace,
                "uncaught panic"
            );
        }));
        if result.is_err() {
            use std::io::Write;
            let _ = writeln!(
                std::io::stderr(),
                "klarvo: panic-hook itself panicked while handling an uncaught panic"
            );
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_panic_message_from_static_str() {
        let payload: Box<dyn std::any::Any + Send> = Box::new("static str panic");
        assert_eq!(extract_panic_message(payload.as_ref()), "static str panic");
    }

    #[test]
    fn extract_panic_message_from_string() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(String::from("owned panic"));
        assert_eq!(extract_panic_message(payload.as_ref()), "owned panic");
    }

    #[test]
    fn extract_panic_message_non_string_fallback() {
        let payload: Box<dyn std::any::Any + Send> = Box::new(42u32);
        assert_eq!(
            extract_panic_message(payload.as_ref()),
            "<non-string panic payload>"
        );
    }

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
