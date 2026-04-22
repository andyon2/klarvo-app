// tauri-plugin-global-shortcut is declared as dep here but registered in Story 3.6.
// ADR-0011 SD-4: registration happens in .setup(..), not in a Command-Handler.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// AC-E: Non-Windows builds fail immediately with an actionable message rather than
// producing a silent empty binary or cryptic linker errors.
#[cfg(not(target_os = "windows"))]
compile_error!("shells/windows requires Windows target");

// Phase-2: xtask subcommand will synchronise version automatically between
// Cargo.toml and tauri.conf.json. Until then, keep both in sync manually.

#[cfg(target_os = "windows")]
fn main() {
    let specta_builder = klarvo_windows_shell::specta_builder();
    let i18n_table = klarvo_windows_shell::i18n::load_default();

    tauri::Builder::default()
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            // Story 3.6 registers tauri-plugin-global-shortcut here
            // (on_shortcut → orchestrator.on_press/on_release).
            // Story 3.3 constructs SessionOrchestrator and inserts it into tauri::State here.
            app.manage(i18n_table);
            specta_builder.mount_events(app);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod smoke {
    /// Manual smoke test: run `cargo tauri dev`, verify the window opens, close via
    /// the X button, and confirm exit code 0 (not 101) in the terminal.
    ///
    /// Not automated: Tauri dev-server requires a display context + WebView runtime.
    /// Automated headless variant deferred to Phase-2 (xtask smoke-test-windows-shell).
    #[test]
    #[ignore = "requires display context — run manually: cargo tauri dev"]
    fn smoke_test_app_starts_and_exits_cleanly() {}
}
