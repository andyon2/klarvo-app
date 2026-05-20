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

    // Logging-Init (before Step 0): install rolling-file tracing subscriber so
    // all subsequent tracing! calls land in %APPDATA%\Klarvo\logs\.
    // `_tracing_guard` keeps the non-blocking writer alive until main() returns.
    let log_dir = std::env::var("APPDATA")
        .map(|d| std::path::PathBuf::from(d).join("Klarvo").join("logs"))
        .unwrap_or_else(|_| std::env::temp_dir().join("Klarvo").join("logs"));
    let _tracing_guard = klarvo_core::telemetry::logging::init_tracing(&log_dir);
    klarvo_core::telemetry::logging::install_panic_hook();

    use klarvo_core::audio::vad::RmsVad;
    use klarvo_core::event::{EventBus, DEFAULT_EVENT_BUS_CAPACITY};
    use klarvo_core::history::{HistoryBackend, NullHistoryBackend, SqliteHistoryStore};
    use klarvo_core::keystore::KeyStore;
    use klarvo_core::recording::RecordingMode;
    use klarvo_core::settings::{NoopSettingsEmitter, Settings, TomlMigrationSource};
    use klarvo_core::time::MonotonicClock;
    use klarvo_plugin_whisper_local::WhisperLocal;
    use klarvo_shell_orchestrator::SessionOrchestrator;
    use tauri::image::Image;
    use tauri::tray::TrayIconBuilder;
    use tauri::Listener;
    use tauri::Manager;

    use klarvo_windows_shell::audio::make_audio_source;
    use klarvo_windows_shell::bridge::{EventMirror, TauriErrorEmitter};
    use klarvo_windows_shell::commands::settings::TauriSettingsEmitter;
    use klarvo_windows_shell::config::{self, ShellConfig};
    use klarvo_windows_shell::focus::WinFocusCapture;
    use klarvo_windows_shell::hotkey::{register_hotkey, register_hotkey_slot2};
    use klarvo_windows_shell::keystore::{make_keystore, verify_keystore_ready};
    use klarvo_windows_shell::paste::WinSendInputPasteBackend;
    use klarvo_windows_shell::tray;

    fn build_plugin_registry(
        settings: &klarvo_core::settings::Settings,
        output_language: &str,
    ) -> klarvo_core::registry::PluginRegistry {
        let mut registry = klarvo_core::registry::bootstrap();
        klarvo_plugin_verbatim::register(&mut registry);

        // Whisper-local: conditional on model_path plugin-setting (ADR-0014 D-1).
        // If not configured → no-op (manifest drives STT plugin selection).
        // If configured but load fails → warn + skip; pipeline falls through to manifest error.
        match settings.get_plugin_setting("whisper-local", "model_path") {
            Ok(Some(model_path_str)) => {
                let model_path = std::path::Path::new(&model_path_str);
                match WhisperLocal::load(model_path, Some(output_language.to_string())) {
                    Ok(plugin) => {
                        registry.register_stt(
                            klarvo_plugin_whisper_local::ID,
                            std::sync::Arc::new(plugin),
                        );
                        tracing::info!(
                            target: "klarvo.bootstrap",
                            model_path = %model_path.display(),
                            language = output_language,
                            "whisper-local: plugin registered"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            target: "klarvo.bootstrap",
                            error = %e,
                            "whisper-local: model load failed; plugin NOT registered"
                        );
                    }
                }
            }
            Ok(None) => {
                tracing::debug!(
                    target: "klarvo.bootstrap",
                    "whisper-local: no model_path configured; plugin not registered"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "klarvo.bootstrap",
                    error = %e,
                    "whisper-local: settings read failed; plugin not registered"
                );
            }
        }

        registry
    }

    let specta_builder = klarvo_windows_shell::specta_builder();

    let app = tauri::Builder::default()
        // Story 9.4: native OS toast notifications on recording.delivered + session errors.
        .plugin(tauri_plugin_notification::init())
        // tauri-plugin-global-shortcut activated here (ADR-0011 SD-4); Story-3.6
        // `register_hotkey` consumes the plugin handle inside the .setup() closure.
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(specta_builder.invoke_handler())
        .setup(move |app| {
            // Bootstrap sequence (Story 3.10 + Story 4.2):
            // Step 0:  TauriErrorEmitter::new (before Step 1 — enables early-boot emit sites;
            //          Story 5.7: moved from Step 4 so Steps 1-2 can use it instead of json!()-emits)
            // Step 1:  resolve_config_path
            // Step 2:  load_config (fail-soft → ShellConfig::default on error)
            // Step 2b: i18n::load(ui_language) — locale-aware table; depends on config (Step 2)
            // Step 3:  make_keystore + verify_keystore_ready (fail-soft → continue on error)
            // Step 4:  make_audio_source
            // Step 5:  WinSendInputPasteBackend
            // Step 6:  MonotonicClock (Phase-1 default Clock impl)
            // Step 7:  RmsVad (Phase-1-Default VadProvider)
            // Step 8:  parse_embedded manifest + build_plugin_registry (fatal on error)
            //          EventBus::new (between 8 and 9 — injected into SessionOrchestrator)
            // Step 9:  SessionOrchestrator::new (fatal on error)
            // Step 10: app.manage (State-insertion)
            // Step 11: Hotkey-registration (fail-soft → emit error + continue)
            // Step 12: Tray-Icon + EventMirror spawn
            //
            // # Bootstrap-Error-Policy
            //
            // Fail-soft (continue with defaults/no-op) for Steps 0-7, 11: App remains
            // functional or degraded but launchable. Fatal (return Err) for Steps 8-9:
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
            // Step 0: Error emitter — created before Steps 1-2 (fail-soft: infallible constructor)
            use klarvo_core::event::ErrorEmitter as _;
            let emitter: Arc<dyn klarvo_core::event::ErrorEmitter> =
                Arc::new(TauriErrorEmitter::new(app.handle().clone()));

            let mut toml_loaded_ok = false;
            let config = match config::resolve_config_path() {
                Ok(path) => match config::load_config(&path) {
                    Ok(c) => {
                        toml_loaded_ok = true;
                        c
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "ShellConfig load failed; using defaults; settings migration skipped");
                        tauri::async_runtime::block_on(emitter.emit_error("error.config.parse_failed", 0));
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
                tauri::async_runtime::block_on(emitter.emit_error("error.settings.in_memory_fallback", 0));
                Settings::in_memory(Arc::clone(&settings_emitter))
                    .unwrap_or_else(|e| {
                        tracing::error!(error = %e, "in-memory settings init failed; using NoopSettings stub");
                        // Last-resort: a Settings constructed via in_memory always succeeds
                        // unless SQLite itself is broken; if that happens we still avoid panic.
                        Settings::in_memory(Arc::new(NoopSettingsEmitter))
                            .unwrap_or_else(|e| unreachable!("in-memory SQLite open is infallible: {e}"))
                    })
            } else {
                match Settings::open(&settings_db_path, Arc::clone(&settings_emitter)) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!(error = %e, path = %settings_db_path.display(), "settings db open failed; falling back to in-memory");
                        tauri::async_runtime::block_on(emitter.emit_error("error.settings.in_memory_fallback", 0));
                        Settings::in_memory(Arc::clone(&settings_emitter))
                            .unwrap_or_else(|e2| {
                                tracing::error!(error = %e2, "in-memory settings init failed; using NoopSettings stub");
                                Settings::in_memory(Arc::new(NoopSettingsEmitter))
                                    .unwrap_or_else(|e| unreachable!("in-memory SQLite open is infallible: {e}"))
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

            // Step 2e: Recording-Mode Arcs (fail-soft — defaults to Hold on DB-error).
            // Shared between SessionOrchestrator (reads mode per-press) and
            // set_recording_mode_slot* Commands (write on user change).
            let recording_mode_arc: Arc<tokio::sync::RwLock<RecordingMode>> = {
                let mode = settings.recording_mode_slot1().unwrap_or(RecordingMode::Hold);
                Arc::new(tokio::sync::RwLock::new(mode))
            };
            // Slot-2: optional second hotkey mode (Story 8.1). Defaults to Hold when not set.
            let recording_mode_arc_slot2: Arc<tokio::sync::RwLock<RecordingMode>> = {
                let mode = settings.recording_mode_slot2().unwrap_or(RecordingMode::Hold);
                Arc::new(tokio::sync::RwLock::new(mode))
            };

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

            // Step 4: Audio source (CpalAudioSource, WASAPI)
            let audio = make_audio_source();

            // Step 5: Paste backend (Win32 SendInput Ctrl+V)
            let paste: Arc<dyn klarvo_core::output::PasteBackend> = Arc::new(WinSendInputPasteBackend);

            // Step 5b: Focus capture (Win32 GetForegroundWindow / SetForegroundWindow)
            let focus_capture: Arc<dyn klarvo_core::output::FocusCapture> =
                Arc::new(WinFocusCapture);

            // Step 6: Clock (Phase-1 default: MonotonicClock — session-relative monotone ms)
            // Phase-2: revisit if wall-clock timestamps are required for cross-session correlation.
            let clock: Arc<dyn klarvo_core::time::Clock> = Arc::new(MonotonicClock::new());

            // Step 7: VAD (Phase-1 default: RmsVad energy threshold).
            // Phase-2+ may substitute SileroVad behind the same VadProvider trait.
            let vad: Arc<tokio::sync::Mutex<Box<dyn klarvo_core::audio::vad::VadProvider>>> =
                Arc::new(tokio::sync::Mutex::new(Box::new(RmsVad::new())));

            // Step 7b: History store (fail-soft — opens history.db, applies schema migrations).
            // Path mirrors settings.db in the same AppData dir.
            let history_store: Arc<dyn HistoryBackend> = {
                // Clamp settings i64 → u32: a corrupt/typo'd settings value (negative
                // or > u32::MAX) must not silently disable retention by wrapping.
                // `.max(1)` enforces "must keep at least one entry" so retention is
                // always engaged; `try_from` saturates at u32::MAX for absurd-large.
                let max_entries: u32 = u32::try_from(settings.history_max_entries().max(1))
                    .unwrap_or(u32::MAX);
                match app.path().app_data_dir() {
                    Ok(dir) => {
                        let history_db_path = dir.join("history.db");
                        match SqliteHistoryStore::open(&history_db_path, max_entries) {
                            Ok(store) => Arc::new(store),
                            Err(e) => {
                                tracing::error!(error = %e.message, path = %history_db_path.display(), "history db open failed; falling back to NullHistoryBackend");
                                Arc::new(NullHistoryBackend)
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "app_data_dir unavailable; using NullHistoryBackend");
                        Arc::new(NullHistoryBackend)
                    }
                }
            };

            // Step 8: Manifest + Registry (fatal — without valid manifest + registry the app
            // has no pipeline and no meaningful voice-transcription function)
            let manifest = Arc::new(klarvo_core::manifest::parse_embedded().map_err(|e| {
                tracing::error!(error = %e, "manifest parse failed");
                e
            })?);
            // output_language read before settings is moved into app.manage (Step 10).
            // Err-arm logs explicitly so a misconfigured / unreadable settings DB does
            // not silently downgrade a German-configured user to English transcription.
            let output_language = match settings.output_language() {
                Ok(lang) => lang,
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        "output_language read failed; falling back to \"en\" for STT"
                    );
                    "en".to_string()
                }
            };
            let registry = Arc::new(build_plugin_registry(&settings, &output_language));

            // EventBus constructed here (before Step 9) so SessionOrchestrator can emit
            // recording-lifecycle events (Started/Stopped/Completed). Managed as State
            // in Step 10 to keep sender alive.
            let event_bus = Arc::new(EventBus::new(DEFAULT_EVENT_BUS_CAPACITY));

            // Step 9: SessionOrchestrator (fatal — constructor is infallible per Story 3.3
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
                Arc::clone(&recording_mode_arc),
                Arc::clone(&recording_mode_arc_slot2),
                focus_capture,
                Arc::clone(&history_store),
            );

            // Step 10: State management — all slots must be registered before Step 11
            // (hotkey callback accesses SessionOrchestrator via tauri::State)
            // app.manage(orch)      → consumed by hotkey-callback (Step 11) + tray-subscription (Step 12)
            // app.manage(config)    → consumed by legacy read paths (Phase-2)
            // app.manage(keystore)  → consumed by future xtask set-key Command (Phase-2)
            // app.manage(emitter)   → consumed by error-emit call-sites in commands (Phase-2)
            // app.manage(clock)     → consumed by hotkey-callback for shared session-baseline ts_ms
            //                         (project_event_ts_ms_convention — single MonotonicClock origin)
            // app.manage(settings)            → consumed by Settings Tauri-Commands (Story 2.A.A4)
            // recording_mode_arc is intentionally NOT managed: per AC-7 the Arc is
            // orchestrator-internal; the `settings.changed` listener (Step 10b) is
            // the single writer, so the set_recording_mode_slot1 Command does not
            // need direct access via tauri::State.
            // Note: `debug_assert!(app.manage(...))` would only execute the manage
            // call in debug builds — in release the entire macro body is a no-op
            // and the state slot would be missing. Always run `manage` first, then
            // assert the boolean separately.
            let inserted_orch = app.manage(orch); debug_assert!(inserted_orch);
            let inserted_config = app.manage(Arc::new(config.clone())); debug_assert!(inserted_config);
            let inserted_keystore = app.manage(Arc::clone(&keystore)); debug_assert!(inserted_keystore);
            let inserted_emitter = app.manage(Arc::clone(&emitter)); debug_assert!(inserted_emitter);
            let inserted_clock = app.manage(Arc::clone(&clock)); debug_assert!(inserted_clock);
            // Read slot-1/slot-2 combos before `settings` is moved into Tauri-managed
            // state — Step 11b below cannot reach the moved-out value.
            let slot1_combo_pre_manage = settings.hotkey_slot1_combo().unwrap_or_default();
            let slot2_combo_pre_manage = settings.hotkey_slot2_combo().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "hotkey_slot2_combo read failed; slot 2 not registered");
                None
            });
            let inserted_settings = app.manage(settings); debug_assert!(inserted_settings);
            let inserted_history = app.manage(
                klarvo_windows_shell::commands::history::HistoryStoreState(history_store)
            );
            debug_assert!(inserted_history);
            // Story 9.5: ExportState — log_dir mirrors the path used by init_tracing above
            let log_dir_for_export = log_dir.clone();
            let inserted_export = app.manage(klarvo_windows_shell::commands::telemetry::ExportState {
                log_dir: log_dir_for_export,
                in_progress: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            });
            debug_assert!(inserted_export);
            // Snapshot the boot-time locale separately because `i18n_table` is
            // moved into managed state below; the listener (Step 10c) owns its
            // own freshly-loaded copy on every locale change.
            // Story 2.A.C3: `i18n_table` is now `SharedI18nTable = Arc<RwLock<I18nTable>>`.
            // `boot_i18n` holds a second Arc-reference for the tray setup below.
            let boot_locale = config.ui_language.clone();
            let notification_i18n = Arc::clone(&i18n_table);
            let boot_i18n = Arc::clone(&i18n_table);
            app.manage(i18n_table);
            specta_builder.mount_events(app);

            // Step 10b: settings.changed listener — keeps recording_mode_arc in sync when
            // set_recording_mode_slot1 Command writes a new mode (AC-7 live-update).
            //
            // Diagnostics: parse-failures on `RecordingMode::from_str` leave a
            // `tracing::warn!` breadcrumb (Re-Review Re-P1) so DB writes that bypass
            // the validating Tauri command (e.g. raw `set_raw` writes from a future
            // migration) don't silently desync `mode_arc` from persisted state.
            {
                use std::str::FromStr;
                let mode_arc_listener = Arc::clone(&recording_mode_arc);
                app.listen("settings.changed", move |event| {
                    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                        if payload.get("key").and_then(|v| v.as_str()) == Some("hotkey.slot1.mode") {
                            if let Some(new_value) = payload.get("newValue").and_then(|v| v.as_str()) {
                                match RecordingMode::from_str(new_value) {
                                    Ok(mode) => {
                                        let arc = Arc::clone(&mode_arc_listener);
                                        tauri::async_runtime::spawn(async move {
                                            *arc.write().await = mode;
                                        });
                                    }
                                    Err(_) => {
                                        tracing::warn!(
                                            value = new_value,
                                            "settings.changed hotkey.slot1.mode value not parseable as RecordingMode"
                                        );
                                    }
                                }
                            }
                        }
                    }
                });
            }

            // Step 10b-slot2: settings.changed listener — keeps recording_mode_arc_slot2
            // in sync when set_recording_mode_slot2 Command writes a new mode (Story 8.1).
            {
                use std::str::FromStr;
                let mode_arc_slot2_listener = Arc::clone(&recording_mode_arc_slot2);
                app.listen("settings.changed", move |event| {
                    if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
                        if payload.get("key").and_then(|v| v.as_str()) == Some("hotkey.slot2.mode") {
                            if let Some(new_value) = payload.get("newValue").and_then(|v| v.as_str()) {
                                match RecordingMode::from_str(new_value) {
                                    Ok(mode) => {
                                        let arc = Arc::clone(&mode_arc_slot2_listener);
                                        tauri::async_runtime::spawn(async move {
                                            *arc.write().await = mode;
                                        });
                                    }
                                    Err(_) => {
                                        tracing::warn!(
                                            value = new_value,
                                            "settings.changed hotkey.slot2.mode value not parseable as RecordingMode"
                                        );
                                    }
                                }
                            }
                        }
                    }
                });
            }

            // Step 10c: settings.changed listener — rebuilds the tray menu when the
            // user switches `ui.language` via the Settings-Panel (Story 2.A.A8-Sub
            // AC-2 + AC-3). Reactive only: A8-Sub never writes settings itself;
            // the menu items in the language sub-menu are visual indicators (AC-5
            // Option B). Other `settings.changed` keys are ignored.
            //
            // Diagnostics: every early-return path leaves a tracing breadcrumb so
            // a stale tray locale can be traced back to the cause (review P1+P2).
            // `newValue` is validated against `tray::SUPPORTED_LOCALES` to avoid
            // a checkmark-less menu when an unsupported locale slips through
            // upstream (review P2).
            {
                let app_handle = app.handle().clone();
                app.listen("settings.changed", move |event| {
                    let payload: serde_json::Value =
                        match serde_json::from_str(event.payload()) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::warn!(error = %e, "settings.changed payload not valid JSON");
                                return;
                            }
                        };
                    let key = payload.get("key").and_then(|v| v.as_str());
                    if key != Some("ui.language") {
                        tracing::trace!(?key, "settings.changed ignored (not ui.language)");
                        return;
                    }
                    let Some(new_locale) = payload.get("newValue").and_then(|v| v.as_str()) else {
                        tracing::warn!("settings.changed for ui.language missing newValue");
                        return;
                    };
                    if !tray::SUPPORTED_LOCALES.iter().any(|(code, _)| *code == new_locale) {
                        tracing::warn!(
                            locale = new_locale,
                            "ui.language change to unsupported locale ignored"
                        );
                        return;
                    }
                    tray::rebuild_for_locale(&app_handle, new_locale);
                });
            }

            // Step 11: Hotkey registration (fail-soft — parse or registration failure emits
            // Toast via TauriErrorEmitter internally; app starts without hotkey rather than exit)
            register_hotkey(app, &config);

            // Step 11b: Slot-2 Hotkey (conditional, D-3 — Story 8.1).
            // Only registered when configured and not identical to slot-1 (backend guard D-2).
            // Re-registration after settings-write is deferred to next boot (Dev Notes §kein-live-re-register).
            {
                let slot2_combo = slot2_combo_pre_manage;

                if let Some(ref combo2) = slot2_combo {
                    let slot1_combo = slot1_combo_pre_manage;
                    // P3 (Code-Review-Closure 2026-05-05): compare parsed Shortcuts so
                    // case / modifier-order / whitespace differences cannot let two
                    // colliding combos slip through and cause a silent OS-level
                    // override on the second on_shortcut registration.
                    use std::str::FromStr;
                    use tauri_plugin_global_shortcut::Shortcut;
                    let collides = match (Shortcut::from_str(combo2), Shortcut::from_str(&slot1_combo)) {
                        (Ok(a), Ok(b)) => a == b,
                        _ => combo2 == &slot1_combo,
                    };
                    if collides {
                        tracing::warn!(
                            combo = %combo2,
                            "hotkey slot-2 combo identical to slot-1; slot 2 not registered (D-2)"
                        );
                    } else {
                        register_hotkey_slot2(app, combo2);
                    }
                } else {
                    tracing::debug!("hotkey slot-2 not configured; skipping registration (D-3)");
                }
            }

            // Step 12: Tray-Icon + EventMirror spawn
            //
            // EventBus already constructed before Step 9 and injected into SessionOrchestrator.
            // Managed as State so the broadcast sender lives for the app lifetime.
            let event_bus_rx_tray = event_bus.subscribe();
            let event_bus_rx_mirror = event_bus.subscribe();
            let event_bus_rx_notification = event_bus.subscribe();
            let event_bus_rx_pill_bar = event_bus.subscribe();
            let inserted_event_bus = app.manage(Arc::clone(&event_bus)); debug_assert!(inserted_event_bus);

            // Step 12a: Tray icon with recording-state indicator (fail-soft per AC-F —
            // asset decode or builder errors log and skip; the app continues without
            // a system-tray icon rather than refusing to boot).
            // TODO Phase-2-Branding: replace placeholder icons with finalized assets
            let tray_setup = (|| -> tauri::Result<_> {
                let idle_icon = Image::from_bytes(include_bytes!("../icons/tray-idle.png"))?;
                let recording_icon = Image::from_bytes(include_bytes!("../icons/tray-recording.png"))?;
                // Story 2.A.A8-Sub: builder lives in `tray.rs` so the
                // `settings.changed` listener (Step 10c) can rebuild the same
                // layout when the user switches `ui.language`.
                // Story 2.A.C3: read-lock `boot_i18n` (SharedI18nTable) for the initial menu
                // build. Recover from poisoning fail-soft — at boot the table cannot have been
                // mutated yet, so a poisoned lock still holds intact data.
                let boot_i18n_guard = boot_i18n.read().unwrap_or_else(|e| e.into_inner());
                let menu = tray::build_menu(app, &*boot_i18n_guard, &boot_locale)?;
                let tray = TrayIconBuilder::with_id(tray::TRAY_ID)
                    .icon(idle_icon.clone())
                    .menu(&menu)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "quit" => app.exit(0),
                        // AC-5 Option B: language items are disabled CheckMenuItems —
                        // they should not fire menu events on Windows, but the dispatch
                        // ignores them defensively in case a platform delivers anyway.
                        other if other.starts_with("language.") => {
                            tracing::debug!(menu_id = other, "tray language item click ignored (AC-5 Option B)");
                        }
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
                    // Subscribes independently from EventMirror (Step 12b) per AC-G separation requirement.
                    let tray_handle = tray.clone();
                    let idle_icon_tray = idle_icon.clone();
                    let recording_icon_tray = recording_icon.clone();
                    let mut tray_rx = event_bus_rx_tray;
                    tauri::async_runtime::spawn(async move {
                        use klarvo_core::event::Event;
                        use tokio::sync::broadcast::error::RecvError;
                        loop {
                            match tray_rx.recv().await {
                                Ok(Event::RecordingStarted { .. }) => {
                                    let _ = tray_handle.set_icon(Some(recording_icon_tray.clone()));
                                }
                                Ok(Event::RecordingStopped { .. }) => {
                                    // Processing placeholder — same red icon until Phase-2 ships
                                    // a distinct processing icon. Tray returns to idle on
                                    // RecordingCompleted, not on RecordingStopped.
                                    let _ = tray_handle.set_icon(Some(recording_icon_tray.clone()));
                                }
                                Ok(Event::RecordingCompleted { .. }) => {
                                    let _ = tray_handle.set_icon(Some(idle_icon_tray.clone()));
                                }
                                Ok(Event::RecordingAborted { .. }) => {
                                    // Recording aborted — return tray icon to idle state immediately.
                                    let _ = tray_handle.set_icon(Some(idle_icon_tray.clone()));
                                }
                                Ok(_) => {}
                                Err(RecvError::Lagged(n)) => {
                                    tracing::warn!(skipped = n, "tray subscriber lagged; resyncing");
                                }
                                Err(RecvError::Closed) => break,
                            }
                        }
                    });
                }
                Err(e) => {
                    tracing::error!(error = %e, "tray setup failed; continuing without tray");
                }
            }

            // Step 12b: EventMirror — ref Story 3.8 AC-D. Always wired, independent of
            // tray outcome (separate broadcast::Receiver per AC-G).
            EventMirror::new(app.handle().clone()).start(event_bus_rx_mirror);

            // Step 12c: NotificationService — native OS toast on recording.delivered (AC-1,
            // Story 9.4) and on ErrorEmitted during active recording session (AC-2).
            klarvo_windows_shell::notification::NotificationService::new(
                app.handle().clone(),
                notification_i18n,
            )
            .start(event_bus_rx_notification);

            // Step 12d: PillBar overlay (Story 9.6) — transparent always-on-top
            // window declared in tauri.conf.json. Fail-soft: window-missing or
            // setup errors log and continue; recording pipeline is unaffected.
            match klarvo_windows_shell::overlay::pill_bar::PillBar::new(app.handle()) {
                Ok(pill_bar) => pill_bar.start(event_bus_rx_pill_bar),
                Err(e) => {
                    tracing::error!(error = %e, "pill-bar setup failed; continuing without overlay");
                }
            }

            Ok(())
        })
        .build(tauri::generate_context!());
    let app = match app {
        Ok(app) => app,
        Err(e) => {
            tracing::error!(error = %e, "Tauri setup failed");
            // Flush the non-blocking tracing writer before process::exit
            // skips Drop chains — without this, the error event above
            // sits in the writer's mpsc channel and never reaches the
            // rolling-file (loses the most diagnostically valuable line
            // when boot fails). Story-6.1 review patch P2.
            drop(_tracing_guard);
            std::process::exit(1);
        }
    };
    app.run(|app_handle, event| {
        if let tauri::RunEvent::Exit = event {
            // Use `try_state` to avoid a second panic in the exit handler if Setup
            // failed before `app.manage(orch)` ran — preserves the original boot error.
            if let Some(orch) = app_handle.try_state::<SessionOrchestrator>() {
                tauri::async_runtime::block_on(async move {
                    orch.shutdown().await;
                });
            }
        }
    });
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
