---
name: Story 3.6 — Hotkey Wire-Up
epic: 3
story_number: "3.6"
status: Draft
dependencies:
  - "3.1"
  - "3.2"
  - "3.3"
---

# Story 3.6: Hotkey Wire-Up

## Outcome

`tauri-plugin-global-shortcut` v2 ist aktiviert. Der Hotkey-String aus `ShellConfig.hotkey`
wird als `Shortcut` geparst und registriert. `ShortcutState::Pressed`/`Released` dispatchen
via `tauri::async_runtime::spawn` an `SessionOrchestrator.on_press()`/`on_release()`.
Zwei neue i18n-Keys für Hotkey-Parse- und Registrierungs-Fehler sind in `locales/en.json`
und `locales/de.json` eingetragen.

## Acceptance Criteria

### AC-A — Plugin-Activation

**Given** Story 3.1 hat den `.setup()`-Hook in `main.rs` mit einem Slot für Plugin-Adds
reserviert  
**When** `.plugin(tauri_plugin_global_shortcut::Builder::new().build())` zum `.setup()`-Chain
hinzugefügt wird  
**Then**

- Das Plugin ist in `shells/windows/src-tauri/Cargo.toml` unter den Tauri-Plugins gepinnt
  (konkrete v2-Version, analog ADR-0002-Pinning-Präzedenz)
- `.plugin(tauri_plugin_global_shortcut::Builder::new().build())` steht im `.setup()`-Hook
  **vor** dem Shortcut-Registrierungs-Code (AC-C)
- Ein Rustdoc-Kommentar am Plugin-Add-Block verweist auf ADR-0011 §SD-4:
  `// tauri-plugin-global-shortcut activated here (ADR-0011 SD-4).`

### AC-B — Hotkey-String-Parsing

**Given** `ShellConfig.hotkey` enthält einen String (z.B. `"CommandOrControl+Shift+Space"`)  
**When** der Parsing-Code im `.setup()`-Hook ausgeführt wird  
**Then**

- `Shortcut::from_str(&config.hotkey)` (Plugin-API) produziert ein `Shortcut`-Objekt
- Bei Parse-Fail (ungültiger Hotkey-String):
  ```rust
  error_emitter.emit_error("error.hotkey.parse_failed", clock.now_ms()).await;
  ```
  Bootstrap läuft weiter (Shell ist ohne Hotkey degraded, nicht gecrasht)
- Parsing passiert in `.setup()` vor der Registration — Fehler wird vor der Registration
  emittiert, nicht versteckt in einem späteren Call

### AC-C — Registration + Dispatch

**Given** `Shortcut`-Parsing per AC-B war erfolgreich  
**When** der Shortcut registriert wird  
**Then**

- Die Registration folgt diesem Pattern:
  ```rust
  app.global_shortcut().on_shortcut(shortcut, move |app, _shortcut, event| {
      let orch = app.state::<Arc<SessionOrchestrator>>().inner().clone();
      match event.state() {
          ShortcutState::Pressed  => {
              tauri::async_runtime::spawn(async move { orch.on_press().await });
          }
          ShortcutState::Released => {
              tauri::async_runtime::spawn(async move { orch.on_release().await });
          }
      }
  });
  ```
- `tauri::async_runtime::spawn` (nicht plain `tokio::spawn`) ist der Shell-seitige
  Dispatch-Wrapper — der Orchestrator selbst nutzt intern plain `tokio::spawn`
  (ref Story 3.3 Technical Notes)
- `tauri::State<Arc<SessionOrchestrator>>` wird von Story 3.10 (Bootstrap-Integration)
  in `tauri::App` gesetzt; diese Story setzt nur voraus, dass der State zum
  Callback-Execution-Zeitpunkt verfügbar ist

### AC-D — Registration-Failure-Path

**Given** der Shortcut ist bereits durch ein anderes System-Tool belegt oder die
Registration schlägt aus anderem Grund fehl  
**When** `on_shortcut(...)` ein `Err` liefert  
**Then**

- Fehler-Mapping:
  ```rust
  error_emitter.emit_error("error.hotkey.registration_failed", clock.now_ms()).await;
  ```
- Bootstrap läuft weiter — Shell ist in einem degraded State (Tray-Icon zeigt Fehler,
  wenn SD-4-Degraded-Tray-Mode gewählt ist) aber nicht crashed
- Frontend erhält `app.error`-Event (ADR-0009) und kann dem User eine Instruktion zeigen

### AC-E — Key-Repeat-Delegation

**Given** Windows emittiert bei gedrücktem Hotkey Key-Repeat-Events (ShortcutState::Pressed)  
**When** die Dispatch-Logic im Callback ausgeführt wird  
**Then**

- Der Dispatch-Code in `shells/windows/` hat **keine** eigene Key-Repeat-Guard-Logik
- Key-Repeat-Filtering liegt ausschließlich im `SessionOrchestrator` (ADR-0011 §SD-3,
  Story 3.3 AC-D)
- Rustdoc am Dispatch-Block expliziert:
  `// Key-repeat filtering lives in SessionOrchestrator (ADR-0011 SD-3).`

### AC-F — i18n-Keys

**Given** Story 3.1 AC-D hat `locales/en.json` + `locales/de.json` angelegt  
**When** diese Story committed wird  
**Then**

- `locales/en.json` enthält mindestens:
  ```json
  {
    "error.hotkey.parse_failed": "Hotkey configuration is invalid. Please check config.toml.",
    "error.hotkey.registration_failed": "Hotkey could not be registered. Another application may already be using it."
  }
  ```
- `locales/de.json` enthält die gleichen Keys; deutsche Übersetzung ist Delegate-Choice;
  TODO-Marker-Pattern analog Story 3.2 AC-G ist akzeptiert
- Beide Locale-Files bleiben valides JSON nach der Ergänzung

### AC-G — Test-Shape (optional + ignored)

**Given** Tauri-MockRuntime-basiertes Testen für das Global-Shortcut-Plugin ist nicht
trivial (Plugin interagiert mit OS-Shortcut-Service)  
**When** Tests implementiert werden  
**Then**

- Empfehlung (b) für Welle-3: `#[test] #[ignore] fn hotkey_manual_test` mit Instruktion:
  ```
  // MANUAL TEST: Start app, press CommandOrControl+Shift+Space, observe recording.
  // cargo xtask test-hotkey-manual
  // This test is an anchor for the xtask smoke-test subcommand (Phase-2 enhancement).
  ```
- (a) Automatisierter Test via MockRuntime + Plugin-Fixture ist Phase-2-Testing-Enhancement:
  wenn `MockRuntime` Shortcut-Plugin-Simulation unterstützt, wird `#[test]` geschrieben der
  einen Orchestrator-Mock erhält und manuelle Event-Emission verifiziert
- Mindestens ein Compile-Check-Test (kein `#[ignore]`) verifiziert, dass
  `Arc<SessionOrchestrator>` in `tauri::State<...>` constructable ist

## Technical Notes

### `tauri::async_runtime::spawn` vs. `tokio::spawn` (Shell-Scope)

Der Shortcut-Callback läuft auf einem Tauri-internen Thread. `tauri::async_runtime::spawn`
übergibt an die Tauri-verwaltete tokio-Runtime (`memory/project_shell_runtime_model`:
Single Tauri-managed tokio-Runtime). Plain `tokio::spawn` würde außerhalb des
Tauri-Runtime-Kontexts fehlschlagen wenn der Callback-Thread keine Runtime-Handle hat.
Merke: `klarvo-shell-orchestrator` selbst (Tauri-frei) nutzt intern `tokio::spawn` —
das ist korrekt, weil er zur Ausführungszeit immer innerhalb der Tauri-managed-Runtime liegt.
Der Shell-seitige Dispatch-Wrapper ist `tauri::async_runtime::spawn`.

### `tauri::State<Arc<SessionOrchestrator>>` Availability

Das Tauri-State-Management (`app.manage(...)`) passiert in Story 3.10 (Bootstrap-Integration)
in `.setup()`, **bevor** die Shortcut-Registration läuft (Setup-Ordering-Invariante).
`app.state::<Arc<SessionOrchestrator>>()` im Callback panikt nicht, solange Bootstrap-Order
eingehalten wird. Story 3.10 trägt die Verantwortung für korrekte Ordering.

### Plugin-Version-Pinning

`tauri-plugin-global-shortcut` wird als exakte Version gepinnt (analog ADR-0002-Präzedenz
für `tauri-specta`). Version wird in Story-3.1 oder Story-3.6-Cargo.toml-Edit festgelegt.
Upgrade-Gate: beim Phase-2-Start, nicht automatisch via `cargo update`.

### Hotkey-Parse-Failure → Degraded-Mode

Bei Hotkey-Parse-Fail bleibt der `error_emitter.emit_error(...)` der einzige Artefakt —
keine Panic, kein `unwrap()`. Bootstrap läuft bis zum App-Open ohne Hotkey. Der User sieht
den Error via `app.error`-Frontend-Event und kann `config.toml` korrigieren + App neu starten.
Diese Degraded-Mode-Semantik folgt ADR-0009 SD-4 Soft-Recommendation (c).

## Dependencies

- Story 3.1 — Tauri-Skeleton (`.setup()`-Hook, Crate-Setup, `locales/`-Files)
- Story 3.2 — `ShellConfig` (liefert `hotkey`-String)
- Story 3.3 — `SessionOrchestrator` (implementiert `on_press`/`on_release`)
- ADR-0011 §SD-2 — Event-State-Dispatch-Pattern
- ADR-0011 §SD-3 — Key-Repeat-Guard (Orchestrator-Scope, nicht hier)
- ADR-0011 §SD-4 — Integration-Point (`.setup()`-Hook)
- ADR-0012 §SD-3 — Windows-Shell-Integration (Orchestrator in Tauri-State)
- ADR-0009 §SD-1 — `app.error`-Event (Error-Propagation bei Fail-Paths)
- `memory/project_shell_runtime_model` — Single tokio-Runtime
- `memory/project_shell_session_lifecycle` — 7-Step-Topology (Press/Release-Semantik)
