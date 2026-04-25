//! `xtask generate-bindings` — delegates to the `export-bindings` binary in
//! `shells/windows/src-tauri/`, which writes TypeScript bindings to
//! `shells/windows/src/bindings/index.ts` via the shared specta builder.
//!
//! CI (`ci-bindings-drift.yml`) runs this subcommand and then asserts
//! `git diff --exit-code` on the bindings output.
//!
//! # Render/Write Separation (Story 5.2 AC-A)
//!
//! `render()` invokes `export-bindings` (which writes to `index.ts`) then reads and returns
//! the generated content. `write_to_disk()` performs an explicit filesystem write of any
//! string to `index.ts`. `run()` calls `render()` — `export-bindings` already wrote the
//! file as a side effect, so `write_to_disk()` is not called in the happy path.
//!
//! This separation enables `cargo xtask bindings-drift` (FR33) to call `render()` to obtain
//! the generated content for byte-identity comparison without needing a separate code path.

use std::path::PathBuf;
use std::process::{Command, ExitCode};

/// Returns the canonical path of the committed bindings file.
pub(crate) fn bindings_path() -> PathBuf {
    // xtask/Cargo.toml → workspace root is the parent of CARGO_MANIFEST_DIR.
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask has a parent directory (workspace root)")
        .to_path_buf();
    workspace_root
        .join("shells")
        .join("windows")
        .join("src")
        .join("bindings")
        .join("index.ts")
}

/// Invoke `export-bindings`, wait for completion, read the generated `index.ts`, and return
/// its content as a `String`.
///
/// As a side effect `export-bindings` overwrites `shells/windows/src/bindings/index.ts`.
/// Callers that only need the content for comparison (e.g., `bindings-drift`) should snapshot
/// the committed file before calling `render()` and restore it afterwards if drift is detected.
pub fn render() -> Result<String, Box<dyn std::error::Error>> {
    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--package",
            "klarvo-windows-shell",
            "--bin",
            "export-bindings",
        ])
        .status()?;

    if !status.success() {
        return Err(format!(
            "export-bindings exited with {}",
            status.code().unwrap_or(-1)
        )
        .into());
    }

    let content = std::fs::read_to_string(bindings_path())?;
    Ok(content)
}

/// Write `content` to `shells/windows/src/bindings/index.ts`.
///
/// Used as an explicit write step when callers have already obtained the content via `render()`
/// but need to restore or overwrite the committed file (e.g., after a drift check).
pub(crate) fn write_to_disk(content: &str) -> ExitCode {
    match std::fs::write(bindings_path(), content) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask generate-bindings: write_to_disk failed: {e}");
            ExitCode::from(1)
        }
    }
}

pub fn run() -> ExitCode {
    match render() {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("xtask generate-bindings: {e}");
            ExitCode::from(1)
        }
    }
}
