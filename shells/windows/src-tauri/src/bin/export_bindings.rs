//! Standalone binding exporter — invoked by `cargo xtask generate-bindings`.
//!
//! Writes TypeScript bindings (commands + events) from the shared specta
//! builder in `lib.rs` to `shells/windows/src/bindings/index.ts`.
//!
//! Runs without a live Tauri app; the CI drift gate (`ci-bindings-drift.yml`)
//! calls this binary and then `git diff --exit-code` on the output path.

use std::path::PathBuf;

use specta_typescript::Typescript;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // crate root = shells/windows/src-tauri/
    // bindings out = shells/windows/src/bindings/index.ts
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        // INTENTIONAL: env!("CARGO_MANIFEST_DIR") always has a parent; this is a dev-tool binary
        // that is never shipped as a production artifact.
        #[allow(clippy::disallowed_methods)]
        .expect("src-tauri has a parent (shells/windows)")
        .join("src")
        .join("bindings")
        .join("index.ts");

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    klarvo_windows_shell::specta_builder().export(Typescript::default(), &out)?;

    println!("exported bindings → {}", out.display());
    Ok(())
}
