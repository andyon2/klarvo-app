//! Klarvo build orchestration — Phase-0 + Epic-5 subcommands.

use std::process::ExitCode;

mod bindings_drift;
mod gen_tokens;
mod generate_bindings;
mod lint_events;
mod manifest_strict;
mod verify_release;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        None | Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("generate-bindings") => reject_unexpected_flags("generate-bindings", &args[1..])
            .unwrap_or_else(generate_bindings::run),
        Some("lint-events") => {
            reject_unexpected_flags("lint-events", &args[1..]).unwrap_or_else(lint_events::run)
        }
        Some("verify-release") => {
            // `--skip-cross-compile` is the only verify-release flag; reject anything else.
            let mut skip_cross_compile = false;
            for arg in &args[1..] {
                match arg.as_str() {
                    "--skip-cross-compile" => skip_cross_compile = true,
                    other => {
                        eprintln!("xtask verify-release: unknown flag '{other}'");
                        return ExitCode::from(2);
                    }
                }
            }
            verify_release::run(skip_cross_compile)
        }
        Some("manifest-strict") => reject_unexpected_flags("manifest-strict", &args[1..])
            .unwrap_or_else(manifest_strict::run),
        Some("bindings-drift") => reject_unexpected_flags("bindings-drift", &args[1..])
            .unwrap_or_else(bindings_drift::run),
        Some("gen-tokens") => reject_unexpected_flags("gen-tokens", &args[1..])
            .unwrap_or_else(gen_tokens::run),
        Some(cmd) if cmd.starts_with("--") => {
            // Unknown flag without a subcommand — exit non-zero so CI tooling notices typos.
            eprintln!("xtask: unknown flag '{cmd}'");
            ExitCode::from(2)
        }
        Some(cmd) => {
            eprintln!("xtask: unknown subcommand '{cmd}'");
            ExitCode::from(2)
        }
    }
}

/// Returns `Some(non-zero ExitCode)` if `args` contains any flag — i.e. the subcommand
/// takes no flags but received one. Returns `None` when args are clean.
fn reject_unexpected_flags(subcommand: &str, args: &[String]) -> Option<ExitCode> {
    if let Some(flag) = args.iter().find(|a| a.starts_with("--")) {
        eprintln!("xtask {subcommand}: unknown flag '{flag}'");
        Some(ExitCode::from(2))
    } else {
        None
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
    println!("  gen-tokens          CSS Custom Properties aus design-tokens.toml generieren → shells/windows/src/styles/tokens.css");
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
