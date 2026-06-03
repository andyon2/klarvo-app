use std::process::Command;

fn main() {
    // git short hash for the About screen (best-effort; "nogit" if .git is
    // unavailable on the build machine, e.g. a robocopied dir without repo
    // metadata).
    //
    // NOTE: the *build time* shown in About is deliberately NOT captured here.
    // Cargo caches build-script output, and a mtime-preserving file sync
    // (robocopy) can make Cargo skip the rebuild entirely — so a build-script
    // timestamp can lie about freshness. The timestamp is instead the running
    // executable's mtime, read at runtime in commands::misc::get_build_info.
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "nogit".to_string());
    println!("cargo:rustc-env=KLARVO_BUILD_HASH={git_hash}");

    tauri_build::build()
}
