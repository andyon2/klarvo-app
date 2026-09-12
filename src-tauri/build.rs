use std::process::Command;

fn main() {
    // git short hash for the About screen. Precedence:
    //   1. KLARVO_BUILD_HASH from the environment -- set by sync-and-build.ps1,
    //      whose robocopy mirror (D:\apps\klarvo) deliberately carries no .git,
    //      so `git rev-parse` there can only fail;
    //   2. `git rev-parse --short HEAD` in the build tree (dev builds);
    //   3. "nogit" (best-effort, never fails the build).
    //
    // NOTE: the *build time* shown in About is deliberately NOT captured here.
    // Cargo caches build-script output, and a mtime-preserving file sync
    // (robocopy) can make Cargo skip the rebuild entirely -- so a build-script
    // timestamp can lie about freshness. The timestamp is instead the running
    // executable's mtime, read at runtime in commands::misc::get_build_info.
    println!("cargo:rerun-if-env-changed=KLARVO_BUILD_HASH");
    let git_hash = std::env::var("KLARVO_BUILD_HASH")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "nogit".to_string());
    println!("cargo:rustc-env=KLARVO_BUILD_HASH={git_hash}");

    tauri_build::build()
}
