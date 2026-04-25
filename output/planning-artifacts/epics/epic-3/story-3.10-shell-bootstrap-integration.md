---
name: Story 3.10 — Shell-Bootstrap-Integration + Tray-Icon Basic
epic: 3
story_number: "3.10"
status: Done
dependencies:
  - "3.1"
  - "3.2"
  - "3.3"
  - "3.4"
  - "3.5"
  - "3.6"
  - "3.7"
  - "3.8"
  - "3.9"
---

# Story 3.10: Shell-Bootstrap-Integration + Tray-Icon Basic

## Outcome

`shells/windows/src-tauri/src/main.rs` `.setup()`-Hook wires alle vorherigen Epic-3-Stories
zu einer lauffähigen App zusammen: ShellConfig + Keystore (fail-soft), `SessionOrchestrator`
mit vollständigen DI-Dependencies, Hotkey-Registration, Tray-Icon mit Recording-State-Indicator
und EventMirror-Spawn. App startet im Idle-State; erste Hotkey-Press triggert den vollständigen
7-Step-End-to-End-Flow.

## Acceptance Criteria

### AC-A — Bootstrap-Sequence + Error-Policy

**Given** alle vorherigen Epic-3-Stories (3.1–3.9) sind implementiert  
**When** der `.setup()`-Hook in `main.rs` implementiert wird  
**Then**

- Der Hook enthält eine Rustdoc-numerierte 13-Step-Sequenz:
  ```rust
  /// Bootstrap sequence (Story 3.10):
  /// Step 1: resolve_config_path
  /// Step 2: load_config (fail-soft → ShellConfig::default on error)
  /// Step 3: make_keystore + verify_keystore_ready (fail-soft → continue on error)
  /// Step 4: TauriErrorEmitter::new
  /// Step 5: make_audio_source
  /// Step 6: WinSendInputPasteBackend
  /// Step 7: MonotonicClock (Phase-1 default Clock impl)
  /// Step 8: RmsVad (Phase-1-Default VadProvider)
  /// Step 9: parse_embedded manifest + build_plugin_registry (fatal on error)
  /// Step 10: SessionOrchestrator::new (fatal on error)
  /// Step 11: app.manage (State-insertion)
  /// Step 12: Hotkey-registration (fail-soft → emit error + continue)
  /// Step 13: Tray-Icon + EventMirror spawn
  ```
- **Fatal-Exit-Policy:** Nur Step 9 (Manifest-Parse + Registry) und Step 10
  (SessionOrchestrator-Construction) dürfen `return Err(...)` aus dem Setup-Hook liefern.
  Rustdoc oberhalb des Hooks erklärt die Policy:
  ```
  /// # Bootstrap-Error-Policy
  ///
  /// Fail-soft (continue with defaults/no-op) for Steps 1-8, 12: App remains
  /// functional or degraded but launchable. Fatal (return Err) for Steps 9-10:
  /// without a valid manifest + orchestrator, the App has no meaningful function.
  ```
- Kein `.expect(...)` / `.unwrap()` im Setup-Hook außer an explizit fatalen Sites (Steps 9-10);
  alle anderen Err-Pfade nutzen `unwrap_or_else(|e| { tracing::error!(...); default })` oder
  äquivalentes Fail-Soft-Pattern

### AC-B — ShellConfig + Keystore-Init (Steps 1–3)

**Given** Story 3.2 hat `load_config` und Story 3.9 hat `make_keystore` + `verify_keystore_ready`  
**When** Steps 1–3 im Setup-Hook implementiert werden  
**Then**

- **Step 1–2 (Config):**
  ```rust
  let config_path = resolve_config_path();
  let config = match load_config(&config_path) {
      Ok(c) => c,
      Err(e) => {
          tracing::error!(error = %e, "ShellConfig load failed; using defaults");
          ShellConfig::default()
      }
  };
  ```
  Fail-Soft-Rationale in Rustdoc: Config-Miss bedeutet App mit Default-Hotkey + Default-Output-Target;
  besser als kein App-Start — User kann nach Start die Config anlegen.

- **Step 3 (Keystore):**
  ```rust
  let keystore: Arc<dyn KeyStore> = make_keystore();
  verify_keystore_ready(keystore.as_ref()).await.unwrap_or_else(|e| {
      tracing::error!(error = %e.message, "keystore boot-readiness check failed; continuing");
  });
  ```
  Fail-Soft-Rationale: Keystore-Boot-Failure ist ephemer (OS-Credential-Manager Race); per-Plugin-Key-Absence
  (Epic-2/1C) surfaced lazily beim ersten Plugin-Init. Boot-Fail rechtfertigt keinen App-Stop.

### AC-C — Orchestrator + Dependencies (Steps 4–10)

**Given** Config + Keystore initialisiert per AC-B  
**When** Steps 4–10 implementiert werden  
**Then**

- **Step 4:** `let emitter: Arc<dyn ErrorEmitter> = Arc::new(TauriErrorEmitter::new(app.handle().clone()));`
  (Story 3.8 — `TauriErrorEmitter` in `shells/windows/src-tauri/src/bridge.rs`)
- **Step 5:** `let audio = make_audio_source();`
  (Story 3.7 — zero-arg; `CpalAudioSource` ist Unit-Struct per Amendment-2 ADR-0009)
- **Step 6:** `let paste: Arc<dyn PasteBackend> = Arc::new(WinSendInputPasteBackend);`
  (Story 3.5 — Unit-Struct; Story 3.4 definiert das Trait in `klarvo-core`)
- **Step 7:** `let clock: Arc<dyn Clock> = Arc::new(MonotonicClock::new());`
  (Phase-1-Default aus `klarvo-core::time`; ADR-0001 / ADR-0003 +
  `memory/project_event_ts_ms_convention` — session-relative monotone ms.
  *Spec-Amendment 2026-04-25:* ursprünglich `SystemClock::default()` — der Type
  existiert nicht in `klarvo-core`; korrekt ist `MonotonicClock`.)
- **Step 8:** `let vad: Arc<tokio::sync::Mutex<Box<dyn VadProvider>>> = Arc::new(tokio::sync::Mutex::new(Box::new(klarvo_core::audio::vad::RmsVad::new())));`
  Phase-1-Default-VadProvider ist `RmsVad` (einzige konkrete Impl in `klarvo-core::audio::vad`).
  Rustdoc: `// Phase-1 default VAD: RmsVad (energy threshold). Phase-2+ may substitute SileroVad.`
- **Step 9 (fatal):**
  ```rust
  let manifest = Arc::new(klarvo_core::manifest::parse_embedded()
      .map_err(|e| {
          tracing::error!(error = %e.message, "manifest parse failed");
          e
      })?);
  let registry = Arc::new(build_plugin_registry(keystore.clone()));
  ```
  `build_plugin_registry` ist eine lokale helper `fn` in `main.rs` (oder `setup.rs`) die:
  ```rust
  fn build_plugin_registry(keystore: Arc<dyn KeyStore>) -> PluginRegistry {
      let mut registry = klarvo_core::registry::bootstrap();
      klarvo_plugin_verbatim::register(&mut registry);
      // Epic-2 registers GroqStt + GroqCleanup here (Story 2.1/2.2):
      // klarvo_plugin_groq::register_stt(&mut registry, keystore.clone());
      // klarvo_plugin_groq::register_cleanup(&mut registry, keystore.clone());
      registry
  }
  ```
  Delegate verifiziert die exakte `register`-API-Shape der Epic-2-Groq-Plugins und passt ggf. an.
  `build_plugin_registry` returniert kein `Result` — Plugin-Registration selbst ist infallible (panict bei
  duplicate ID, was nie passieren sollte); Keystore-Lookup-Fails sind lazy in Plugin-Init.

- **Step 10 (fatal):**
  ```rust
  let orch = Arc::new(SessionOrchestrator::new(
      registry,
      manifest,
      audio,
      config.output_target_id.clone(),
      paste,
      emitter.clone(),
      clock,
      vad,
  ));
  ```
  `SessionOrchestrator::new` ist infallible (kein Result) per Story 3.3 AC-B — Konstruktor
  setzt nur Felder. Fatal-Policy-Begründung: ohne Orchestrator hat die App keine Kernfunktion.
  Fehler hier wären Programmierfehler (falsche Typ-Shapes) → Panic ist akzeptabel.
  `.map_err(|e| ...)` entfällt, da kein Return-Value. Ggf. `Result`-wrap wenn Story-3.3-Impl
  abweicht — Delegate-Verification-Action.

### AC-D — tauri::State-Management (Step 11)

**Given** `orch`, `config`, `keystore`, `emitter` sind konstruiert per AC-C  
**When** Step 11 ausgeführt wird  
**Then**

- State-Insertion erfolgt **vor** Hotkey-Registration (AC-E) und Tray-Spawn (AC-F):
  ```rust
  app.manage(Arc::clone(&orch));
  app.manage(Arc::new(config));
  app.manage(Arc::clone(&keystore));
  app.manage(Arc::clone(&emitter));
  ```
- Rustdoc-Kommentar listet Consumer je State-Slot:
  ```
  // app.manage(orch)    → consumed by hotkey-callback (Step 12) + tray-subscription (Step 13)
  // app.manage(config)  → consumed by future Settings-Read-Commands (Phase-2)
  // app.manage(keystore) → consumed by future xtask set-key Command (Phase-2)
  // app.manage(emitter) → consumed by error-emit call-sites in commands (Phase-2)
  ```
- State-Manage-Invariante: Tauri `app.manage()` gibt `false` zurück wenn derselbe Type bereits
  registriert; das darf in Phase-1 nicht passieren (genau eine Setup-Ausführung). Ein
  `debug_assert!`-Wrap ist akzeptabel, Panic ist akzeptabel.

### AC-E — Hotkey-Registration (Step 12)

**Given** `app.manage(orch)` ist abgeschlossen per AC-D  
**When** Step 12 die Hotkey-Registration aus Story 3.6 aufruft  
**Then**

- Hotkey-Registration erfolgt via Aufruf der Story-3.6-Logik:
  ```rust
  if let Err(e) = register_hotkey(app, &config_loaded_in_step2, Arc::clone(&orch)) {
      emitter.emit_error(&e.user_message.unwrap_or_default(), 0).await;
      tracing::error!(error = %e.message, "hotkey registration failed; app starts without hotkey");
  }
  ```
  Die exakte Signatur von `register_hotkey` orientiert sich an Story-3.6-Impl. Falls Story 3.6
  keinen separaten Helper exportiert, ist der Inline-Call-Äquivalent akzeptabel.
- Fail-Soft: Hotkey-Register-Fail emittiert Error via `error_emitter` (User-Toast) und setzt
  fort — App startet, Hotkey fehlt, User sieht Toast. App-Exit wäre over-engineering für
  einen initialisierbaren Fehler.
- State-Availability-Invariante ist gewahrt: `app.manage(orch)` ist in Step 11 abgeschlossen,
  bevor Step 12 den Orchestrator über `tauri::State` referenziert.

### AC-F — Tray-Icon-Basic + Recording-State-Indicator (Step 13a)

**Given** alle Steps 1–12 sind abgeschlossen  
**When** das Tray-Icon gebaut und der Recording-State-Indicator-Task gespawned wird  
**Then**

- **Icon-Assets:** Zwei Placeholder-PNG-Dateien in `shells/windows/src-tauri/icons/`:
  - `tray-idle.png` — 16×16 Placeholder (z. B. graues Mikrofon-Icon oder Klarvo-Monogram)
  - `tray-recording.png` — 16×16 Placeholder (z. B. rotes Mikrofon-Icon)
  - Rustdoc-TODO: `// TODO Phase-2-Branding: replace placeholder icons with finalized assets`
  - Delegate-Choice: echte 16×16 PNGs generieren oder Datei-Stubs mit minimaler valid-PNG-Struktur

- **TrayIconBuilder:**
  ```rust
  use tauri::menu::{MenuBuilder, MenuItemBuilder};
  use tauri::tray::TrayIconBuilder;

  let menu = MenuBuilder::new(app)
      .item(&MenuItemBuilder::with_id("info", "Klarvo").enabled(false).build(app)?)
      .item(&MenuItemBuilder::with_id("quit", "Exit").build(app)?)
      .build()?;

  let tray = TrayIconBuilder::new()
      .icon(app.default_window_icon().unwrap().clone())
      .menu(&menu)
      .on_menu_event(|app, event| {
          if event.id.as_ref() == "quit" {
              app.exit(0);
          }
      })
      .build(app)?;
  ```
  Delegate verifiziert die exakte `TrayIconBuilder`-API-Shape gegen die tauri v2 Docs / Changelog
  (insbesondere `.icon()` akzeptiert `tauri::image::Image` oder `Icon` je nach RC-Version).
  Falls `build(app)?` `?` nicht im `.setup()`-Closure unterstützt, ist `.expect(...)` hier
  akzeptabel (Tray-Build-Fail ist fatal-level; keine Tray-App ohne Tray).
  Context-Menu hat mindestens `"Klarvo"` (Info-Label, disabled) und `"Exit"`.

- **Recording-State-Indicator:**
  ```rust
  let event_bus_rx = /* subscribe to EventBus-broadcast-channel */;
  let tray_handle = tray.clone();  // oder app.tray_handle()
  tokio::spawn(async move {
      while let Ok(event) = event_bus_rx.recv().await {
          match event {
              CoreEvent::RecordingStarted { .. } => {
                  let _ = tray_handle.set_icon(Some(recording_icon.clone()));
              }
              // Spec-Amendment 2026-04-25: 3-state indicator — Stopped retains
              // recording icon as "processing" placeholder until pipeline drains;
              // tray returns to idle on RecordingCompleted, not RecordingStopped
              // (see Story 3.3 Amendment 2026-04-25 for lifecycle semantics).
              // Phase-2-Branding ships a distinct processing icon (e.g. spinner overlay).
              CoreEvent::RecordingStopped { .. } => {
                  let _ = tray_handle.set_icon(Some(recording_icon.clone()));
              }
              CoreEvent::RecordingCompleted { .. } => {
                  let _ = tray_handle.set_icon(Some(idle_icon.clone()));
              }
              _ => {}
          }
      }
  });
  ```
  Pattern analog Story 3.8 AC-D (EventMirror), aber Consumer ist Tray-Icon statt `AppHandle::emit`.
  Delegate verifiziert `.set_icon()`-API-Shape für Tray-Handle in tauri v2; ggf. `tray.set_icon(icon)`
  statt `tray_handle.set_icon(icon)` je nach Handle-Typ.
  Recording-State-Subscription kommt von demselben EventBus-`broadcast::Receiver<CoreEvent>`
  den EventMirror nutzt (Step 13b). Separate Subscription (eigener `.subscribe()` Call).

### AC-G — EventMirror-Spawn (Step 13b)

**Given** Tray-Icon gebaut per AC-F  
**When** EventMirror gespawned wird  
**Then**

- EventMirror-Spawn analog Story 3.8 AC-D:
  ```rust
  // event_bus_rx_for_mirror is a separate subscriber from the EventBus broadcast channel
  EventMirror::new(app.handle().clone()).start(event_bus_rx_for_mirror);
  ```
  Rustdoc verweist auf Story 3.8: `// EventMirror started here; ref Story 3.8 AC-D.`
- Spawned-Task läuft für App-Lifetime (kein expliziter Shutdown — tokio-Runtime-Drop cancelt ihn sicher)
- Separation: EventMirror hat eigene `broadcast::Receiver<CoreEvent>`, Recording-State-Tray-Task
  hat ebenfalls eigene. Beide subscriben unabhängig via `event_bus.subscribe()` oder äquivalent.
  Delegate verifiziert die EventBus-API-Shape aus Story 3.8/3.3 (wahrscheinlich `broadcast::channel`
  mit `subscribe()` für Receiver-Clone).

### AC-H — Integration-Smoke-Test (#[ignore] + Compile-Verification)

**Given** `shells/windows/src-tauri/src/main.rs` ist implementiert  
**When** `cargo test -p <windows-shell-crate>` läuft  
**Then**

- Ein `#[test] #[ignore]` Bootstrap-Smoke-Test dokumentiert den manuellen Verifizierungs-Workflow:
  ```rust
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
  ```
- Ein **Compile-Verification-Test** stellt sicher dass alle Types wireable sind:
  ```rust
  #[test]
  fn setup_closure_types_compile() {
      // Validates that all DI-types are available at compile time.
      // This test only needs to compile — no assertions.
      fn _assert_types_compile() {
          use klarvo_shell_orchestrator::SessionOrchestrator;
          use klarvo_core::time::MonotonicClock;
          use klarvo_core::audio::vad::RmsVad;
          // Type-annotation ensures these types are importable and compatible.
          let _: fn(
              std::sync::Arc<klarvo_core::registry::PluginRegistry>,
              std::sync::Arc<klarvo_core::manifest::PipelineManifest>,
          ) = |_, _| {};
      }
  }
  ```
  Compile-Test ist akzeptabler Approximation; exakte Type-Wiring-Check ist in Story-3.11-E2E-Test.

### AC-I — i18n-Keys-Coverage-Audit

**Given** Stories 3.2, 3.3, 3.6, 3.9 registrieren i18n-Keys in `locales/en.json` und `locales/de.json`  
**When** Story 3.10 committed wird  
**Then**

- Story 3.10 registriert **keine neuen** i18n-Keys — alle Error-Emit-Sites in den
  Bootstrap-Steps nutzen Keys aus vorherigen Stories. Rustdoc am Setup-Hook expliziert das:
  ```
  // No new i18n-keys in Story 3.10. Error-emit-sites use keys from:
  //   - Story 3.2: error.config.*, error.keystore.read_failed
  //   - Story 3.3: error.audio.start_failed, error.config.output_target_not_found
  //   - Story 3.6: error.hotkey.*
  //   - Story 3.9: error.keystore.read_failed
  ```
- Delegate prüft dennoch dass alle in Steps 4–12 verwendeten i18n-Keys in `locales/en.json` +
  `locales/de.json` eingetragen sind. Fehlende Keys werden nachgetragen (Coverage-Audit-Nebenprodukt).
- `locales/en.json` + `locales/de.json` bleiben valides JSON nach der Ergänzung

## Technical Notes

### Bootstrap-Ordering-Invariante

Drei Abhängigkeiten bestimmen die Step-Reihenfolge:

1. **Emitter vor Orchestrator (Step 4 vor Step 10):** `SessionOrchestrator::new` nimmt
   `Arc<dyn ErrorEmitter>` — Emitter muss vor Orchestrator existieren.
2. **State-Manage vor Hotkey-Register (Step 11 vor Step 12):** Die Hotkey-Callback-Closure
   (ADR-0011 SD-2) holt `tauri::State<Arc<SessionOrchestrator>>`; diese muss registriert sein,
   bevor der Callback ausgelöst werden kann. `app.manage()` muss also vor
   `register_shortcut(...)` stehen.
3. **Hotkey vor Tray-Subscription (Step 12 vor Step 13):** Konventionell; kein technisches
   Muss, aber konsistente Reihenfolge erleichtert Debugging.

### Fail-Soft-Policy: Threshold

Rule-of-Thumb für Impl: wenn die App nach einem Step-Fail noch eine für den User meaningvolle
State hat (z. B. Idle-State + Tray sichtbar), fail-soft. Wenn die App ohne den Step keine
Kernfunktion (Voice-Transcription) anbieten kann und kein Degraded-Mode sinnvoll ist, fail-fatal.

- Config-Miss → Default-Config → Hotkey ist noch registrierbar → fail-soft
- Keystore-Boot-Miss → lazy per-Plugin-Key-Error → fail-soft
- Manifest-Parse-Fail → keine Pipeline möglich → fail-fatal
- Orchestrator-Konstruktion mit falschen Deps → Programmierfehler → panic akzeptabel

### VadProvider-Default: RmsVad (Phase-1 verifiziert)

`RmsVad` aus `klarvo_core::audio::vad::RmsVad` ist die einzige konkrete `VadProvider`-Impl
in der Phase-1-Codebase (Stand `klarvo-core/src/audio/vad/rms.rs`). Keine Phase-2-Substitution
(SileroVad) ist eingebunden. Rustdoc-Marker im Bootstrap verweist auf Phase-2-Extensibility.

### Tray-Icon-Assets: Placeholder-Policy

Phase-1 nutzt 16×16 PNG-Placeholders; sie sind gecheckt in
`shells/windows/src-tauri/icons/tray-idle.png` + `tray-recording.png`. Finale Assets kommen
mit Phase-2-Branding-Design. Der Commit dieser Story checkt minimale valide PNGs ein (kann mit
z. B. ImageMagick `convert -size 16x16 xc:gray tray-idle.png` generiert werden, oder als
leere Dummy-Datei wenn Tauri das akzeptiert). Delegate-Choice.

### Kein `.expect()` Panic in Setup außer bei fatalen Steps

Analog `memory/feedback_scaffold_fail_soft_pattern` (Phase-1-Establiert bei Epic-1C). Setup-Hook
ist Boot-critical-Path — unerwartet panickende non-fatal Steps wären UX-jarring und verhindern
das Tray-Icon-Feedback (User sieht nichts, nur OS-Crash-Dialog).

### Registry + Plugin-API-Shape (Delegate-Action-Item)

`klarvo_core::registry::bootstrap()` returniert eine leere `PluginRegistry`. Plugin-Registration
via `klarvo_plugin_verbatim::register(&mut registry)` ist verifiziert (provider.rs Zeile 12).
Für Epic-2-Groq-Plugins ist die genaue `register`-Signatur aus den Story-2.x-Impl-Dateien zu
verifizieren. Falls `build_plugin_registry` als separater Helper in Epic-2-Code existiert,
Delegate ersetzt die lokale `fn build_plugin_registry`-Impl durch den Epic-2-Helper.

## Dependencies

- Story 3.1 — Tauri-Skeleton (Crate, `main.rs`, `locales/`)
- Story 3.2 — `ShellConfig` + `load_config` + `resolve_config_path`
- Story 3.3 — `SessionOrchestrator::new` + `on_press`/`on_release`
- Story 3.4 — `PasteBackend`-Trait in `klarvo-core`
- Story 3.5 — `WinSendInputPasteBackend` in `shells/windows/`
- Story 3.6 — `register_hotkey` helper (oder inline Hotkey-Registration-Code)
- Story 3.7 — `make_audio_source()` Factory (zero-arg, Unit-Struct)
- Story 3.8 — `TauriErrorEmitter`, `EventMirror`
- Story 3.9 — `make_keystore()`, `verify_keystore_ready()`
- ADR-0009 Amendment 2 — Primary-Consumer = Orchestrator (kein ErrorEmitter-DI in CpalAudioSource)
- ADR-0012 §SD-3 — Windows-Shell-Integration (Bootstrap-Reference)
- `memory/project_shell_session_lifecycle` — 7-Step-Topology
- `memory/project_shell_runtime_model` — Single tokio-Runtime
- `memory/feedback_scaffold_fail_soft_pattern` — Fail-Soft statt `todo!()`/panic
