// ADR-0011 SD-4: hotkey registration in .setup(..), not in a Command-Handler.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// AC-E: Non-Windows builds fail immediately with an actionable message rather than
// producing a silent empty binary or cryptic linker errors.
#[cfg(not(target_os = "windows"))]
compile_error!("shells/windows requires Windows target");

// Phase-2: xtask subcommand will synchronise version automatically between
// Cargo.toml and tauri.conf.json. Until then, keep both in sync manually.

#[cfg(target_os = "windows")]
fn main() {
    use std::sync::Arc;

    use klarvo_core::audio::vad::RmsVad;
    use klarvo_core::event::EventBus;
    use klarvo_core::keystore::KeyStore;
    use klarvo_core::time::MonotonicClock;
    use klarvo_shell_orchestrator::SessionOrchestrator;
    use tauri::image::Image;
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::TrayIconBuilder;

    use klarvo_windows_shell::audio::make_audio_source;
    use klarvo_windows_shell::bridge::{EventMirror, TauriErrorEmitter};
    use klarvo_windows_shell::config::{self, ShellConfig};
    use klarvo_windows_shell::hotkey::register_hotkey;
    use klarvo_windows_shell::keystore::{make_keystore, verify_keystore_ready};
    use klarvo_windows_shell::paste::WinSendInputPasteBackend;

    /// Construct the `PluginRegistry` with all Phase-1 plugins registered.
    ///
    /// `_keystore` parameter is reserved for Phase-2 Groq-plugin registration.
    /// Groq-plugin stubs are commented below for Epic-2 (Story 2.1/2.2) reference.
    fn build_plugin_registry(_keystore: Arc<dyn KeyStore>) -> klarvo_core::registry::PluginRegistry {
        let mut registry = klarvo_core::registry::bootstrap();
        klarvo_plugin_verbatim::register(&mut registry);
        // Epic-2 registers GroqStt + GroqCleanup here (Story 2.1/2.2):
        // klarvo_plugin_groq::register_stt(&mut registry, _keystore.clone());
        // klarvo_plugin_groq::register_cleanup(&mut registry, _keystore.clone());
        registry
    }

    let specta_builder = klarvo_windows_shell::specta_builder();
    let i18n_table = klarvo_windows_shell::i18n::load_default();

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            // Bootstrap sequence (Story 3.10):
            // Step 1:  resolve_config_path
            // Step 2:  load_config (fail-soft → ShellConfig::default on error)
            // Step 3:  make_keystore + verify_keystore_ready (fail-soft → continue on error)
            // Step 4:  TauriErrorEmitter::new
            // Step 5:  make_audio_source
            // Step 6:  WinSendInputPasteBackend
            // Step 7:  MonotonicClock (Phase-1 default Clock impl)
            // Step 8:  RmsVad (Phase-1-Default VadProvider)
            // Step 9:  parse_embedded manifest + build_plugin_registry (fatal on error)
            //          EventBus::new (between 9 and 10 — injected into SessionOrchestrator)
            // Step 10: SessionOrchestrator::new (fatal on error)
            // Step 11: app.manage (State-insertion)
            // Step 12: Hotkey-registration (fail-soft → emit error + continue)
            // Step 13: Tray-Icon + EventMirror spawn
            //
            // # Bootstrap-Error-Policy
            //
            // Fail-soft (continue with defaults/no-op) for Steps 1-8, 12: App remains
            // functional or degraded but launchable. Fatal (return Err) for Steps 9-10:
            // without a valid manifest + orchestrator, the App has no meaningful function.
            //
            // No new i18n-keys in Story 3.10. Error-emit-sites use keys from:
            //   - Story 3.2: error.config.*, error.keystore.read_failed
            //   - Story 3.3: error.audio.start_failed, error.config.output_target_not_found
            //   - Story 3.6: error.hotkey.*
            //   - Story 3.9: error.keystore.read_failed

            // Steps 1-2: Config (fail-soft — Config-Miss means app starts with default hotkey +
            // default output-target; better than no app-start; user can create config after start)
            let config = match config::resolve_config_path() {
                Ok(path) => match config::load_config(&path) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!(error = %e.message, "ShellConfig load failed; using defaults");
                        ShellConfig::default()
                    }
                },
                Err(e) => {
                    tracing::error!(error = %e.message, "config path resolution failed; using defaults");
                    ShellConfig::default()
                }
            };

            // Step 3: Keystore (fail-soft — Credential Manager boot-race is ephemeral;
            // per-plugin key errors surface lazily in Plugin-Init, not at boot)
            let keystore: Arc<dyn KeyStore> = make_keystore();
            {
                let ks = Arc::clone(&keystore);
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = verify_keystore_ready(ks.as_ref()).await {
                        tracing::error!(error = %e.message, "keystore boot-readiness check failed; continuing");
                    }
                });
            }

            // Step 4: Error emitter
            let emitter: Arc<dyn klarvo_core::event::ErrorEmitter> =
                Arc::new(TauriErrorEmitter::new(app.handle().clone()));

            // Step 5: Audio source (CpalAudioSource, WASAPI)
            let audio = make_audio_source();

            // Step 6: Paste backend (Win32 SendInput Ctrl+V)
            let paste: Arc<dyn klarvo_core::output::PasteBackend> = Arc::new(WinSendInputPasteBackend);

            // Step 7: Clock (Phase-1 default: MonotonicClock — session-relative monotone ms)
            // Phase-2: revisit if wall-clock timestamps are required for cross-session correlation.
            let clock: Arc<dyn klarvo_core::time::Clock> = Arc::new(MonotonicClock::new());

            // Step 8: VAD (Phase-1 default: RmsVad energy threshold)
            // Phase-2 default VAD: RmsVad (energy threshold). Phase-2+ may substitute SileroVad.
            let vad: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::vad::VadProvider>>> =
                Arc::new(tokio::sync::Mutex::new(Box::new(RmsVad::new())));

            // Step 9: Manifest + Registry (fatal — without valid manifest + registry the app
            // has no pipeline and no meaningful voice-transcription function)
            let manifest = Arc::new(klarvo_core::manifest::parse_embedded().map_err(|e| {
                tracing::error!(error = %e.message, "manifest parse failed");
                e
            })?);
            let registry = Arc::new(build_plugin_registry(Arc::clone(&keystore)));

            // EventBus constructed here (before Step 10) so SessionOrchestrator can emit
            // RecordingStarted/Stopped. Managed as State in Step 11 to keep sender alive.
            let event_bus = Arc::new(EventBus::new(64));

            // Step 10: SessionOrchestrator (fatal — constructor is infallible per Story 3.3
            // AC-B; type-shape mismatches are programmer errors caught at compile time)
            let orch = Arc::new(SessionOrchestrator::new(
                Arc::clone(&registry),
                Arc::clone(&manifest),
                audio,
                config.output_target_id.clone(),
                paste,
                Arc::clone(&emitter),
                clock,
                vad,
                Arc::clone(&event_bus),
            ));

            // Step 11: State management — all slots must be registered before Step 12
            // (hotkey callback accesses Arc<SessionOrchestrator> via tauri::State)
            // app.manage(orch)     → consumed by hotkey-callback (Step 12) + tray-subscription (Step 13)
            // app.manage(config)   → consumed by future Settings-Read-Commands (Phase-2)
            // app.manage(keystore) → consumed by future xtask set-key Command (Phase-2)
            // app.manage(emitter)  → consumed by error-emit call-sites in commands (Phase-2)
            debug_assert!(app.manage(Arc::clone(&orch)));
            debug_assert!(app.manage(Arc::new(config.clone())));
            debug_assert!(app.manage(Arc::clone(&keystore)));
            debug_assert!(app.manage(Arc::clone(&emitter)));
            app.manage(i18n_table);
            specta_builder.mount_events(app);

            // Step 12: Hotkey registration (fail-soft — parse or registration failure emits
            // Toast via TauriErrorEmitter internally; app starts without hotkey rather than exit)
            register_hotkey(app, &config);

            // Step 13: Tray-Icon + EventMirror spawn
            //
            // EventBus already constructed before Step 10 and injected into SessionOrchestrator.
            // Managed as State so the broadcast sender lives for the app lifetime.
            let event_bus_rx_tray = event_bus.subscribe();
            let event_bus_rx_mirror = event_bus.subscribe();
            debug_assert!(app.manage(Arc::clone(&event_bus)));

            // Step 13a: Tray icon with recording-state indicator
            // TODO Phase-2-Branding: replace placeholder icons with finalized assets
            let idle_icon = Image::from_bytes(include_bytes!("../icons/tray-idle.png"))
                .expect("tray-idle.png must be a valid PNG");
            let recording_icon = Image::from_bytes(include_bytes!("../icons/tray-recording.png"))
                .expect("tray-recording.png must be a valid PNG");

            let menu = MenuBuilder::new(app)
                .item(&MenuItemBuilder::with_id("info", "Klarvo").enabled(false).build(app)?)
                .item(&MenuItemBuilder::with_id("quit", "Exit").build(app)?)
                .build()?;

            let tray = TrayIconBuilder::with_id("klarvo-tray")
                .icon(idle_icon.clone())
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            // Recording-state indicator: switch tray icon on RecordingStarted/RecordingStopped.
            // Subscribes independently from EventMirror (Step 13b) per AC-G separation requirement.
            let tray_handle = tray.clone();
            let idle_icon_tray = idle_icon.clone();
            let recording_icon_tray = recording_icon.clone();
            let mut tray_rx = event_bus_rx_tray;
            tauri::async_runtime::spawn(async move {
                use klarvo_core::event::Event;
                while let Ok(event) = tray_rx.recv().await {
                    match event {
                        Event::RecordingStarted { .. } => {
                            let _ = tray_handle.set_icon(Some(recording_icon_tray.clone()));
                        }
                        Event::RecordingStopped { .. } => {
                            let _ = tray_handle.set_icon(Some(idle_icon_tray.clone()));
                        }
                        _ => {}
                    }
                }
            });

            // Step 13b: EventMirror — ref Story 3.8 AC-D.
            // Subscribes independently from the tray state task (separate broadcast::Receiver).
            EventMirror::new(app.handle().clone()).start(event_bus_rx_mirror);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod smoke {
    /// Manual smoke test: run `cargo tauri dev`, verify tray icon appears in system tray,
    /// press configured hotkey, verify tray icon switches to recording state, release hotkey,
    /// verify tray returns to idle state. Exit via Tray → Exit menu item.
    ///
    /// Not automated: Tauri dev-server requires a display context + WebView runtime.
    #[test]
    #[ignore = "requires running Tauri app with display context"]
    fn bootstrap_smoke_test() {
        // Manual verification steps:
        // (a) cargo tauri dev -- verify App-Window opens
        // (b) verify Tray-Icon visible in system tray (idle icon)
        // (c) press configured hotkey -- verify Tray-Icon switches to recording icon
        //     and Recording-State-Indicator changes
        // (d) release hotkey -- verify Tray-Icon returns to idle icon
        // (e) cargo test -p klarvo-shell-orchestrator -- headless unit tests cover E2E logic
        unimplemented!("manual smoke test — see comments above")
    }

    /// Compile-check: all DI-types used in the setup closure are importable and type-compatible.
    ///
    /// This test only needs to compile — no assertions. Type-wiring correctness is
    /// validated fully in Story-3.11 E2E integration tests.
    #[test]
    fn setup_closure_types_compile() {
        fn _assert_types_compile() {
            use std::sync::Arc;
            use klarvo_shell_orchestrator::SessionOrchestrator;
            use klarvo_core::time::MonotonicClock;
            use klarvo_core::audio::vad::RmsVad;
            // Type-annotations ensure these types are importable and compatible.
            let _: fn(
                Arc<klarvo_core::registry::PluginRegistry>,
                Arc<klarvo_core::manifest::PipelineManifest>,
            ) = |_, _| {};
            let _ = MonotonicClock::new();
            let _ = RmsVad::new();
            let _: fn() -> Arc<SessionOrchestrator>;
        }
    }
}
