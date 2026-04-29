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
    use klarvo_core::event::{EventBus, DEFAULT_EVENT_BUS_CAPACITY};
    use klarvo_core::keystore::KeyStore;
    use klarvo_core::settings::{NoopSettingsEmitter, Settings, TomlMigrationSource};
    use klarvo_core::time::MonotonicClock;
    use klarvo_shell_orchestrator::SessionOrchestrator;
    use tauri::image::Image;
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    use tauri::tray::TrayIconBuilder;
    use tauri::Manager;

    use klarvo_windows_shell::audio::make_audio_source;
    use klarvo_windows_shell::bridge::{EventMirror, TauriErrorEmitter};
    use klarvo_windows_shell::commands::settings::TauriSettingsEmitter;
    use klarvo_windows_shell::config::{self, ShellConfig};
    use klarvo_windows_shell::hotkey::register_hotkey;
    use klarvo_windows_shell::keystore::{make_keystore, verify_keystore_ready};
    use klarvo_windows_shell::paste::WinSendInputPasteBackend;

    /// Construct the `PluginRegistry` with all Phase-1 plugins registered.
    ///
    /// Epic-2 (Story 2.1/2.2) will re-introduce a `keystore: Arc<dyn KeyStore>`
    /// parameter for Groq-plugin registration:
    ///   `klarvo_plugin_groq::register_stt(&mut registry, keystore.clone());`
    fn build_plugin_registry() -> klarvo_core::registry::PluginRegistry {
        let mut registry = klarvo_core::registry::bootstrap();
        klarvo_plugin_verbatim::register(&mut registry);
        registry
    }

    let specta_builder = klarvo_windows_shell::specta_builder();

    tauri::Builder::default()
        // tauri-plugin-global-shortcut activated here (ADR-0011 SD-4); Story-3.6
        // `register_hotkey` consumes the plugin handle inside the .setup() closure.
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            // Bootstrap sequence (Story 3.10 + Story 4.2):
            // Step 1:  resolve_config_path
            // Step 2:  load_config (fail-soft → ShellConfig::default on error)
            // Step 2b: i18n::load(ui_language) — locale-aware table; depends on config (Step 2)
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
            // Step 2b is currently a Panic-Path (Phase-1 stub): JSON-corruption in the
            // embedded locale files surfaces as a panic before .setup() returns. Phase-2
            // fail-soft (empty-table fallback + tracing::error!) is tracked under
            // ADR-0009 SD-4 (Boot-Error-UX); see also `i18n.rs::load` doc-comment.
            //
            // No new i18n-keys in Story 3.10. Error-emit-sites use keys from:
            //   - Story 3.2: error.config.*, error.keystore.read_failed
            //   - Story 3.3: error.audio.start_failed, error.config.output_target_not_found
            //   - Story 3.6: error.hotkey.*
            //   - Story 3.9: error.keystore.read_failed

            // Steps 1-2: Config (fail-soft — Config-Miss means app starts with default hotkey +
            // default output-target; better than no app-start; user can create config after start)
            //
            // `toml_loaded_ok` tracks whether the user's config.toml parsed cleanly.
            // It is the sole input to AC-2's TOML→SQLite migration decision (Step 2d):
            // a malformed file falls back to defaults at boot AND skips migration so that
            // partial/garbled values never silently overwrite the SQLite settings layer.
            // The user is notified via app.error (toast in the Settings-Panel — D1 from
            // code-review 2026-04-29).
            use tauri::Emitter as _;

            let mut toml_loaded_ok = false;
            let config = match config::resolve_config_path() {
                Ok(path) => match config::load_config(&path) {
                    Ok(c) => {
                        toml_loaded_ok = true;
                        c
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "ShellConfig load failed; using defaults; settings migration skipped");
                        let _ = app.handle().emit(
                            "app.error",
                            serde_json::json!({
                                "key": "error.config.parse_failed",
                                "ts_ms": 0u64,
                            }),
                        );
                        ShellConfig::default()
                    }
                },
                Err(e) => {
                    tracing::error!(error = %e, "config path resolution failed; using defaults");
                    ShellConfig::default()
                }
            };

            // Step 2b: i18n table for the resolved ui_language axis (FR26/FR27, Story 4.2).
            // Eagerly loaded after config so the active locale matches user choice; load() validates
            // both locale files at boot to surface JSON corruption regardless of selection.
            let i18n_table = klarvo_windows_shell::i18n::load(&config.ui_language);

            // Step 2c: Settings service (fail-soft — opens settings.db, applies schema migrations).
            // Path is resolved through Tauri's `app_data_dir` rather than the raw APPDATA env
            // so it follows the user's actual `tauri.conf.json` identifier and works under
            // alternative shells / sanitised environments. The parent directory is created
            // up-front to avoid a first-boot ENOENT on `Connection::open`.
            //
            // Fallback chain: file-backed → in-memory. If both fail, surface app.error to the
            // user instead of panicking (per `feedback_scaffold_fail_soft_pattern`); the
            // settings layer enters a no-op-Arc state where reads return defaults and writes
            // fail loudly with a Validation-shaped error.
            let settings_db_path = match app.path().app_data_dir() {
                Ok(dir) => {
                    if let Err(e) = std::fs::create_dir_all(&dir) {
                        tracing::error!(error = %e, dir = %dir.display(), "failed to create app_data_dir; using fallback path");
                    }
                    dir.join("settings.db")
                }
                Err(e) => {
                    tracing::error!(error = %e, "app_data_dir unavailable; falling back to in-memory settings");
                    std::path::PathBuf::new() // sentinel — open() will fail and trigger in-memory fallback
                }
            };

            let settings_emitter: Arc<dyn klarvo_core::settings::SettingsEmitter> =
                Arc::new(TauriSettingsEmitter::new(app.handle().clone()));

            let settings: Settings = if settings_db_path.as_os_str().is_empty() {
                tracing::warn!("opening in-memory settings (app_data_dir unavailable)");
                let _ = app.handle().emit(
                    "app.error",
                    serde_json::json!({ "key": "error.settings.in_memory_fallback", "ts_ms": 0u64 }),
                );
                Settings::in_memory(Arc::clone(&settings_emitter))
                    .unwrap_or_else(|e| {
                        tracing::error!(error = %e, "in-memory settings init failed; using NoopSettings stub");
                        // Last-resort: a Settings constructed via in_memory always succeeds
                        // unless SQLite itself is broken; if that happens we still avoid panic.
                        Settings::in_memory(Arc::new(NoopSettingsEmitter))
                            .expect("rusqlite in-memory open is infallible on healthy SQLite build")
                    })
            } else {
                match Settings::open(&settings_db_path, Arc::clone(&settings_emitter)) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!(error = %e, path = %settings_db_path.display(), "settings db open failed; falling back to in-memory");
                        let _ = app.handle().emit(
                            "app.error",
                            serde_json::json!({ "key": "error.settings.in_memory_fallback", "ts_ms": 0u64 }),
                        );
                        Settings::in_memory(Arc::clone(&settings_emitter))
                            .unwrap_or_else(|e2| {
                                tracing::error!(error = %e2, "in-memory settings init failed; using NoopSettings stub");
                                Settings::in_memory(Arc::new(NoopSettingsEmitter))
                                    .expect("rusqlite in-memory open is infallible on healthy SQLite build")
                            })
                    }
                }
            };

            // Step 2d: One-shot TOML→SQLite migration (D2 from code-review 2026-04-29:
            // strict-parse via `load_config`, no struct-level fallback). The validated `config`
            // from Steps 1-2 is the sole migration source: if `toml_loaded_ok == false` we
            // already surfaced `app.error` and skip migration to avoid persisting defaults
            // over potentially-recoverable user state.
            {
                let toml_src = if toml_loaded_ok {
                    Some(TomlMigrationSource {
                        hotkey_slot1_combo: config.hotkey.clone(),
                        output_target_id: config.output_target_id.clone(),
                        ui_language: config.ui_language.clone(),
                        dictionary_language: config.dictionary_language.clone(),
                        output_language: config.output_language.clone(),
                    })
                } else {
                    None
                };

                if let Err(e) = settings.migrate_from_toml_if_needed(toml_src.as_ref()) {
                    tracing::warn!(error = %e, "TOML→SQLite migration failed; continuing with empty settings");
                }
            }

            // Step 3: Keystore (fail-soft — Credential Manager boot-race is ephemeral;
            // per-plugin key errors surface lazily in Plugin-Init, not at boot).
            //
            // Synchronous readiness probe via block_on so the result is observable
            // before Step 10+ wire-up rather than racing with downstream plugin-init.
            // 2 s defensive timeout guards against pathologically hanging
            // Credential-Manager hardware; both timeout and probe-error are fail-soft.
            let keystore: Arc<dyn KeyStore> = make_keystore();
            {
                let ks = Arc::clone(&keystore);
                let probe = tauri::async_runtime::block_on(async move {
                    tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        verify_keystore_ready(ks.as_ref()),
                    )
                    .await
                });
                match probe {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        tracing::error!(error = %e, "keystore boot-readiness check failed; continuing");
                    }
                    Err(_elapsed) => {
                        tracing::error!("keystore boot-readiness check timed out after 2s; continuing");
                    }
                }
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

            // Step 8: VAD (Phase-1 default: RmsVad energy threshold).
            // Phase-2+ may substitute SileroVad behind the same VadProvider trait.
            let vad: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::vad::VadProvider>>> =
                Arc::new(tokio::sync::Mutex::new(Box::new(RmsVad::new())));

            // Step 9: Manifest + Registry (fatal — without valid manifest + registry the app
            // has no pipeline and no meaningful voice-transcription function)
            let manifest = Arc::new(klarvo_core::manifest::parse_embedded().map_err(|e| {
                tracing::error!(error = %e, "manifest parse failed");
                e
            })?);
            let registry = Arc::new(build_plugin_registry());

            // EventBus constructed here (before Step 10) so SessionOrchestrator can emit
            // recording-lifecycle events (Started/Stopped/Completed). Managed as State
            // in Step 11 to keep sender alive.
            let event_bus = Arc::new(EventBus::new(DEFAULT_EVENT_BUS_CAPACITY));

            // Step 10: SessionOrchestrator (fatal — constructor is infallible per Story 3.3
            // AC-B; type-shape mismatches are programmer errors caught at compile time)
            let orch = SessionOrchestrator::new(
                Arc::clone(&registry),
                Arc::clone(&manifest),
                audio,
                config.output_target_id.clone(),
                paste,
                Arc::clone(&emitter),
                Arc::clone(&clock),
                vad,
                Arc::clone(&event_bus),
            );

            // Step 11: State management — all slots must be registered before Step 12
            // (hotkey callback accesses SessionOrchestrator via tauri::State)
            // app.manage(orch)      → consumed by hotkey-callback (Step 12) + tray-subscription (Step 13)
            // app.manage(config)    → consumed by legacy read paths (Phase-2)
            // app.manage(keystore)  → consumed by future xtask set-key Command (Phase-2)
            // app.manage(emitter)   → consumed by error-emit call-sites in commands (Phase-2)
            // app.manage(clock)     → consumed by hotkey-callback for shared session-baseline ts_ms
            //                         (project_event_ts_ms_convention — single MonotonicClock origin)
            // app.manage(settings)  → consumed by Settings Tauri-Commands (Story 2.A.A4)
            debug_assert!(app.manage(orch));
            debug_assert!(app.manage(Arc::new(config.clone())));
            debug_assert!(app.manage(Arc::clone(&keystore)));
            debug_assert!(app.manage(Arc::clone(&emitter)));
            debug_assert!(app.manage(Arc::clone(&clock)));
            debug_assert!(app.manage(settings));
            let exit_label = i18n_table
                .get("tray.menu.exit")
                .cloned()
                .unwrap_or_else(|| "Exit".to_string());
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

            // Step 13a: Tray icon with recording-state indicator (fail-soft per AC-F —
            // asset decode or builder errors log and skip; the app continues without
            // a system-tray icon rather than refusing to boot).
            // TODO Phase-2-Branding: replace placeholder icons with finalized assets
            let tray_setup = (|| -> tauri::Result<_> {
                let idle_icon = Image::from_bytes(include_bytes!("../icons/tray-idle.png"))?;
                let recording_icon = Image::from_bytes(include_bytes!("../icons/tray-recording.png"))?;
                let menu = MenuBuilder::new(app)
                    .item(&MenuItemBuilder::with_id("info", "Klarvo").enabled(false).build(app)?)
                    .item(&MenuItemBuilder::with_id("quit", &exit_label).build(app)?)
                    .build()?;
                let tray = TrayIconBuilder::with_id("klarvo-tray")
                    .icon(idle_icon.clone())
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "quit" => app.exit(0),
                        other => tracing::warn!(menu_id = other, "unhandled tray menu event"),
                    })
                    .build(app)?;
                Ok((tray, idle_icon, recording_icon))
            })();

            match tray_setup {
                Ok((tray, idle_icon, recording_icon)) => {
                    // Recording-state indicator: 3-state switch following the recording lifecycle
                    // (see `Event` doc-comment in klarvo-core/src/event/bus.rs).
                    //   Started   → recording icon (red).
                    //   Stopped   → recording icon (placeholder for "processing" — pipeline is
                    //               still draining; audio capture has ended). TODO Phase-2-Branding:
                    //               distinct processing icon (e.g. red mic + spinner overlay).
                    //   Completed → idle icon (gray) — pipeline task has fully exited.
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
                                    // Processing placeholder — same red icon until Phase-2 ships
                                    // a distinct processing icon. Tray returns to idle on
                                    // RecordingCompleted, not on RecordingStopped.
                                    let _ = tray_handle.set_icon(Some(recording_icon_tray.clone()));
                                }
                                Event::RecordingCompleted { .. } => {
                                    let _ = tray_handle.set_icon(Some(idle_icon_tray.clone()));
                                }
                                _ => {}
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "tray setup failed; continuing without tray");
                }
            }

            // Step 13b: EventMirror — ref Story 3.8 AC-D. Always wired, independent of
            // tray outcome (separate broadcast::Receiver per AC-G).
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
            let _: fn() -> SessionOrchestrator;
        }
    }
}
