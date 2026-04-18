//! Klarvo build orchestration — Phase-0 subcommands.

use std::process::ExitCode;

mod generate_bindings;
mod lint_events;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("--help") | Some("-h") => {
            print_help();
            ExitCode::SUCCESS
        }
        Some("generate-bindings") => generate_bindings::run(),
        Some("lint-events") => lint_events::run(),
        Some(cmd) => {
            eprintln!("xtask: subcommand '{cmd}' not implemented yet");
            ExitCode::from(2)
        }
    }
}

fn print_help() {
    println!("xtask — Klarvo build orchestration (Phase-0)");
    println!();
    println!("Subcommands:");
    println!("  generate-bindings   tauri-specta TS regen (shells/windows/src/bindings/index.ts)");
    println!("  lint-events         Validation-Patch G1 — enforce #[tauri_specta(event_name)] dot-notation");
    println!();
    println!("Planned (stubs):");
    println!("  lint-features       Cargo-feature naming convention enforcer");
    println!("  verify-release      Validation-Patch G2 — release hardening gate");
    println!("  new-plugin <name>   Plugin skeleton generator");
    println!("  build-all           Build core + plugins + shells");
    println!("  test-core           Headless core tests (Linux)");
    println!("  ci                  CI matrix aggregate");
}
