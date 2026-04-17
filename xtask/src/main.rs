fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("--help") | Some("-h") => print_help(),
        Some(cmd) => {
            eprintln!("xtask: subcommand '{cmd}' not implemented yet");
            std::process::exit(2);
        }
    }
}

fn print_help() {
    println!("xtask — Klarvo build orchestration (Phase-0 stub)");
    println!();
    println!("Planned subcommands:");
    println!("  generate-bindings   tauri-specta regen + drift check");
    println!("  lint-events         Validation-Patch G1 — specta::Event rename attr enforcer");
    println!("  lint-features       Cargo-feature naming convention enforcer");
    println!("  verify-release      Validation-Patch G2 — release hardening gate");
    println!("  new-plugin <name>   Plugin skeleton generator");
    println!("  build-all           Build core + plugins + shells");
    println!("  test-core           Headless core tests (Linux)");
    println!("  ci                  CI matrix aggregate");
}
