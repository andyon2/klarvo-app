//! `xtask generate-bindings` — delegates to the `export-bindings` binary in
//! `shells/windows/src-tauri/`, which writes TypeScript bindings to
//! `shells/windows/src/bindings/index.ts` via the shared specta builder.
//!
//! CI (`ci-bindings-drift.yml`) runs this subcommand and then asserts
//! `git diff --exit-code` on the bindings output.

use std::process::{Command, ExitCode};

pub fn run() -> ExitCode {
    let status = Command::new("cargo")
        .args([
            "run",
            "--quiet",
            "--package",
            "klarvo-windows-shell",
            "--bin",
            "export-bindings",
        ])
        .status();

    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => {
            eprintln!(
                "xtask generate-bindings: export-bindings exited with {}",
                s.code().unwrap_or(-1)
            );
            ExitCode::from(s.code().and_then(|c| u8::try_from(c).ok()).unwrap_or(1))
        }
        Err(e) => {
            eprintln!("xtask generate-bindings: failed to spawn cargo: {e}");
            ExitCode::from(1)
        }
    }
}
