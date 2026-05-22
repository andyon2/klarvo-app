//! Verify that `init_tracing` actually produces non-empty log files when events
//! are emitted. Each integration test runs in its own process, so
//! `set_global_default` succeeds independently.
//!
//! Story 12.2 diagnostic: the user reports klarvo.*.log = 0 bytes after a full
//! boot even though `init_tracing` returned Some. There is no existing unit/integration
//! test that proves the happy path — this fills the gap.

use std::fs;
use std::time::Duration;

#[test]
fn init_tracing_writes_events_to_rolling_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log_dir = dir.path().to_path_buf();

    let guard = klarvo_core::telemetry::logging::init_tracing(&log_dir)
        .expect("init_tracing should return Some on a writeable temp dir");

    tracing::error!("test event 1 — error level");
    tracing::warn!("test event 2 — warn level");
    tracing::info!("test event 3 — info level");

    // Force the worker thread to flush by dropping the guard.
    drop(guard);

    // Tiny grace period in case the worker is still draining the channel
    // after the shutdown signal is sent (shutdown is sent with a 1s timeout
    // inside WorkerGuard::drop).
    std::thread::sleep(Duration::from_millis(100));

    let entries: Vec<_> = fs::read_dir(&log_dir)
        .expect("read_dir")
        .filter_map(|e| e.ok())
        .collect();

    assert!(
        !entries.is_empty(),
        "expected at least one log file in {}; got none",
        log_dir.display()
    );

    let total_bytes: u64 = entries
        .iter()
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum();

    assert!(
        total_bytes > 0,
        "expected log files to have content; total bytes = {}. Entries: {:?}",
        total_bytes,
        entries
            .iter()
            .map(|e| (e.file_name(), e.metadata().map(|m| m.len()).ok()))
            .collect::<Vec<_>>()
    );
}
