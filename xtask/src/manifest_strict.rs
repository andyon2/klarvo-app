//! `xtask manifest-strict` — Pre-commit gate: validates bad-input fixtures against
//! `klarvo_core::manifest::parse_from_str` (FR32).
//!
//! Enforces boot-time Executor-strictness locally before a bad manifest causes a runtime crash.
//! Implements Preventive Enforcement + Forcing-Sentinel per `memory/feedback_ci_gate_philosophy`.
//!
//! # Fixture Layout
//!
//! `xtask/test-fixtures/manifest-strict/expected.toml` defines the expected outcome per fixture.
//! Each `<name>.toml` in that directory is a fixture; adding a file + expected.toml entry extends
//! the suite without modifying Rust source.
//!
//! # Two-Layer FR6 Coverage
//!
//! - AC-C (bad-unknown-stage): tests the Compile-Time-Layer manifestation — serde-unknown-tag
//!   rejection via `PipelineStageType`'s `#[serde(tag = "type")]` enum.
//! - AC-F (bad-type-mismatch): tests the Boot-Time-Layer manifestation — Executor Type-Chaining-
//!   Check-1 in `run_pipeline`. `parse_from_str` alone does NOT catch type mismatches; this
//!   fixture requires the full boot-path (ref: `memory/project_type_chaining_runtime_layer.md`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use serde::Deserialize;

use klarvo_core::{
    AppErrorKind,
    audio::AudioBuffer,
    manifest::parse_from_str,
    pipeline::{StageData, executor::run_pipeline},
    registry::PluginRegistry,
};

#[derive(Deserialize)]
struct FixtureExpected {
    outcome: String,
    error_kind: Option<String>,
    user_message_key: Option<String>,
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-fixtures")
        .join("manifest-strict")
}

pub fn run() -> ExitCode {
    let dir = fixtures_dir();

    let expected_src = match std::fs::read_to_string(dir.join("expected.toml")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("manifest-strict: cannot read expected.toml: {e}");
            return ExitCode::from(2);
        }
    };
    let expected: HashMap<String, FixtureExpected> = match toml::from_str(&expected_src) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("manifest-strict: cannot parse expected.toml: {e}");
            return ExitCode::from(2);
        }
    };

    let mut keys: Vec<&String> = expected.keys().collect();
    keys.sort(); // deterministic output
    let total = keys.len();
    let mut passed = 0usize;

    for name in &keys {
        let fixture_path = dir.join(format!("{name}.toml"));
        let content = match std::fs::read_to_string(&fixture_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[FAIL] {name}: cannot read fixture file: {e}");
                continue;
            }
        };

        let result = run_fixture(name, &content);
        if check_result(name, &result, &expected[*name]) {
            passed += 1;
        }
    }

    eprintln!("manifest-strict: {passed}/{total} passed");
    if passed == total {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn run_fixture(name: &str, content: &str) -> Result<(), klarvo_core::AppError> {
    if name == "bad-type-mismatch" {
        // Type-Chaining is Runtime-Layer, not Parse-Layer (project_type_chaining_runtime_layer.md).
        // parse_from_str succeeds for a syntactically valid manifest with a known stage type.
        // We must run the full Executor boot-path to trigger the mismatch check.
        // Boot-Check-Ordering: Type-Chaining (Check-1) runs BEFORE Plugin-Lookup (Check-2),
        // so an empty PluginRegistry is sufficient — the mismatch fires before any lookup.
        let manifest = parse_from_str(content)?;
        let registry = PluginRegistry::new();
        // Cleanup-as-first-stage + Audio input triggers: "expected text, got audio".
        let audio_input = StageData::Audio(AudioBuffer {
            samples: vec![],
            sample_rate: 16_000,
            ts_ms_start: 0,
            ts_ms_end: 0,
        });
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("tokio current-thread runtime builds")
            .block_on(run_pipeline(&manifest, &registry, audio_input))
            .map(|_| ())
    } else {
        // Forcing sentinel: valid.toml MUST call parse_from_str for real.
        // A hardcoded ExitCode::SUCCESS bypass would fail here because parse_from_str
        // must actually return Ok for valid.toml — proving real execution occurred.
        parse_from_str(content).map(|_| ())
    }
}

fn check_result(
    name: &str,
    result: &Result<(), klarvo_core::AppError>,
    expected: &FixtureExpected,
) -> bool {
    match expected.outcome.as_str() {
        "ok" => {
            if result.is_ok() {
                eprintln!("[PASS] {name}: ok");
                true
            } else {
                let e = result.as_ref().unwrap_err();
                eprintln!(
                    "[FAIL] {name}: expected ok, got Err kind={} user_message={:?}",
                    kind_str(&e.kind),
                    e.user_message,
                );
                false
            }
        }
        "err" => match result {
            Err(e) => {
                let kind_ok = expected
                    .error_kind
                    .as_deref()
                    .map(|k| kind_str(&e.kind) == k)
                    .unwrap_or(true);
                let msg_ok = expected
                    .user_message_key
                    .as_deref()
                    .map(|k| e.user_message.as_deref() == Some(k))
                    .unwrap_or(true);

                if kind_ok && msg_ok {
                    let user_msg = e.user_message.as_deref().unwrap_or("(none)");
                    eprintln!("[PASS] {name}: PipelineValidation ({user_msg})");
                    true
                } else {
                    eprintln!(
                        "[FAIL] {name}: got Err kind={} user_message={:?} — expected kind={:?} key={:?}",
                        kind_str(&e.kind),
                        e.user_message,
                        expected.error_kind,
                        expected.user_message_key,
                    );
                    false
                }
            }
            Ok(_) => {
                eprintln!(
                    "[FAIL] {name}: expected Err (kind={:?} key={:?}), got Ok",
                    expected.error_kind, expected.user_message_key,
                );
                false
            }
        },
        other => {
            eprintln!("[FAIL] {name}: unknown outcome in expected.toml: {other:?}");
            false
        }
    }
}

fn kind_str(kind: &AppErrorKind) -> &'static str {
    match kind {
        AppErrorKind::PipelineValidation => "PipelineValidation",
        AppErrorKind::Network => "Network",
        AppErrorKind::Auth => "Auth",
        AppErrorKind::Validation => "Validation",
        AppErrorKind::RateLimit => "RateLimit",
        AppErrorKind::Internal => "Internal",
        AppErrorKind::UpstreamUnavailable => "UpstreamUnavailable",
        AppErrorKind::Configuration => "Configuration",
        AppErrorKind::Io => "Io",
        AppErrorKind::PermissionDenied => "PermissionDenied",
        AppErrorKind::KeyMissing => "KeyMissing",
        _ => "Unknown",
    }
}
