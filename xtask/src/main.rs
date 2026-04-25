//! Klarvo build orchestration — Phase-0 + Epic-5 subcommands.

use std::process::ExitCode;

mod bindings_drift;
mod generate_bindings;
mod lint_events;
mod manifest_strict;
mod verify_release;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let skip_cross_compile = args.contains(&"--skip-cross-compile".to_string());

    match args.first().map(String::as_str) {
        None | Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("generate-bindings") => generate_bindings::run(),
        Some("lint-events") => lint_events::run(),
        Some("verify-release") => verify_release::run(skip_cross_compile),
        Some("manifest-strict") => manifest_strict::run(),
        Some("bindings-drift") => bindings_drift::run(),
        Some(cmd) if cmd.starts_with("--") => {
            // Flags without a subcommand — fall through to help.
            print_help();
            ExitCode::SUCCESS
        }
        Some(cmd) => {
            eprintln!("xtask: unknown subcommand '{cmd}'");
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    println!("xtask — Klarvo build orchestration");
    println!();
    println!("Subcommands:");
    println!("  generate-bindings   tauri-specta TS regen (shells/windows/src/bindings/index.ts)");
    println!("  lint-events         G1+G3 lint gate — event-name policy, user-string detection, locale drift, wildcard-match");
    println!("  verify-release      G2 release-hardening gate (forbidden features, tracing-subscriber sentinel, android cross-compile)");
    println!("  manifest-strict     Pre-commit gate: validates bad-input fixtures against parse_from_str (FR32)");
    println!("  bindings-drift      Drift-Check: failt wenn shells/windows/src/bindings/index.ts nicht synchron mit generate-bindings-Output ist");
    println!();
    println!("Flags:");
    println!("  --skip-cross-compile  Skip aarch64-linux-android cross-compile check in verify-release (local-dev only; MUST NOT be set in CI)");
    println!();
    println!("Planned (stubs):");
    println!("  lint-features       Cargo-feature naming convention enforcer");
    println!("  new-plugin <name>   Plugin skeleton generator");
    println!("  build-all           Build core + plugins + shells");
    println!("  test-core           Headless core tests (Linux)");
    println!("  ci                  CI matrix aggregate");
}
