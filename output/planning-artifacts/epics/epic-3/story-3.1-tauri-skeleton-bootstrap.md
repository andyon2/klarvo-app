---
name: Story 3.1 — Tauri-Skeleton-Bootstrap
epic: 3
story_number: "3.1"
status: Draft
dependencies: []
---

# Story 3.1: Tauri-Skeleton-Bootstrap

## Outcome

Leere Tauri-v2-App (`shells/windows/src-tauri/`) startet via `cargo tauri dev` und schließt
sauber. Die App enthält einen vorbereiteten Plugin-Registrierungs-Slot, ein leeres Main-Window
und einen funktionsfähigen i18n-Translation-Tables-Loader.

## Acceptance Criteria

### AC-A — Crate + Cargo.toml-Setup

**Given** das Repository enthält noch kein `shells/windows/src-tauri/`-Crate  
**When** das Crate angelegt und `Cargo.toml` befüllt wird  
**Then**

- `shells/windows/src-tauri/Cargo.toml` listet als Workspace-Member alle folgenden Dependencies:
  - `tauri` v2 (aligned mit ADR-0002-RC-Stack),
  - `tauri-plugin-global-shortcut` v2 (aktueller stable v2-Release, Version pinned analog ADR-0002),
  - `tauri-specta` (Version-Pin aus ADR-0002),
  - `klarvo-core` als Workspace-Path-Dep
- `cargo tauri dev` aus dem `shells/windows/`-Verzeichnis startet ohne Compile-Error und ohne
  Runtime-Panic
- Die Crate ist im Workspace-`Cargo.toml` unter `members` eingetragen

### AC-B — Minimales `main.rs` + `.setup()`-Shape

**Given** das Crate existiert per AC-A  
**When** `shells/windows/src-tauri/src/main.rs` implementiert wird  
**Then**

- `fn main()` ruft `tauri::Builder::default()` auf und endet mit `.run()`
- `.setup(|app| { ... })`-Hook ist vorhanden und enthält einen Rustdoc-Kommentar:
  `// Story 3.6 registers tauri-plugin-global-shortcut here (on_shortcut → orchestrator.on_press/on_release).`
- Der Hook returniert `Ok(())` ohne sonstige Logik (Skeleton — kein Plugin aktiv registriert,
  kein Orchestrator konstruiert)
- Ein zweiter Rustdoc-Kommentar markiert die zukünftige Orchestrator-Construction-Site:
  `// Story 3.3 constructs SessionOrchestrator and inserts it into tauri::State here.`

### AC-C — Leeres Main-Window + Close-Behavior

**Given** die App startet per AC-A  
**When** der User das Fenster mit dem X-Button schließt  
**Then**

- Die App öffnet beim Start ein Main-Window (default-Size ist akzeptabel; kein WebView-Content
  erforderlich — ein HTML-Placeholder `<h1>Klarvo</h1>` genügt)
- Der X-Button-Click führt zu einem sauberen App-Exit mit Exit-Code 0 (kein Hang, kein Panic)
- Frontend-Phase-2-Dependencies (React, Svelte, Vue) sind noch nicht vorhanden; das Window
  rendert statisches HTML

### AC-D — i18n-Translation-Tables-Loader

**Given** `shells/windows/locales/` enthält `en.json` und `de.json`  
**When** die App bootet und der i18n-Loader initialisiert  
**Then**

- `shells/windows/src-tauri/src/i18n.rs` (oder äquivalentes Modul) lädt beide Locale-Files.
  Implementierung darf `include_str!()` (compile-time) oder Runtime-Load (aus App-Ressource-Dir)
  verwenden — Delegate-Choice
- Default-Locale ist `en`; der Loader returniert `en`-Tabelle wenn keine explizite Locale
  konfiguriert ist
- Der Loader exposed seinen Zustand als `Arc<I18nTable>` (wobei `I18nTable` ein Newtype oder
  Alias über `std::collections::HashMap<String, String>` ist) und trägt es in Tauri-managed-State
  ein via `app.manage(Arc::new(table))`
- **Boot-Time-Error-Handling:** Wenn `en.json` oder `de.json` kein valides JSON enthält, ruft
  der Loader `panic!()` mit einer Meldung, die auf das fehlerhafte File zeigt. Rustdoc-Kommentar
  direkt am Panic-Aufruf trägt folgenden Text:
  `// Phase-2: replace panic with fail-soft AppError path per ADR-0009 SD-4 Boot-Error-UX.`
- Die `locales/`-Dateien sind minimale valid-JSON-Objekte: `{}` ist ausreichend für Phase-1
  (Keys werden von Impl-Stories addiert)
- Kein Translation-Wire-Up zu UI-Elementen in dieser Story (nur Loader + managed-State)

### AC-E — `cfg(target_os = "windows")`-Gate

**Given** der Crate-Entry (`main.rs` oder `lib.rs`) soll Windows-only sein  
**When** das Crate auf einem non-Windows-Host cross-compiled wird  
**Then**

- `#[cfg(target_os = "windows")]`-Gate ist am Crate-Entry gesetzt (per ADR-0006 Amendment 2,
  das dieses Gate-Pattern als Platform-Impl-Convention etabliert)
- Ein `#[cfg(not(target_os = "windows"))]`-Companion emittiert `compile_error!("shells/windows
  requires Windows target")` — sodass non-Windows-Builds mit klarer Meldung abbrechen, nicht
  silent leere Binaries erzeugen
- `cargo check --target x86_64-unknown-linux-gnu` (oder gleichwertiges cross-check-Kommando)
  produziert den `compile_error!`-Text und keinen anderen Fehler

### AC-F — Plugin-Registration-Slot (Cargo.toml-Dep ohne aktive Registrierung)

**Given** `tauri-plugin-global-shortcut` v2 ist in Cargo.toml eingetragen (AC-A)  
**When** der `.setup()`-Hook ausgeführt wird  
**Then**

- `tauri_plugin_global_shortcut::Builder::new().build()` wird **nicht** in `.setup()` aufgerufen —
  die Registration ist via Rustdoc-Kommentar für Story 3.6 reserviert (AC-B trägt diesen
  Kommentar bereits)
- Das Crate kompiliert ohne `unused import`-Warnings für die Plugin-Crate: entweder ist der
  Import mit `#[allow(unused_imports)]` und Forward-Reference-Kommentar versehen, oder der
  Import ist noch nicht vorhanden und Cargo.toml-Dep trägt `optional = true` mit Notiz
  „activated in Story 3.6"
- **Rationale-Rustdoc:** Ein Modul-Level-Kommentar in `main.rs` erklärt:
  `// tauri-plugin-global-shortcut is declared as dep here but registered in Story 3.6.`
  `// ADR-0011 SD-4: registration happens in .setup(..), not in a Command-Handler.`

### AC-G — Bootstrap-Smoke-Test

**Given** die App startet per AC-A/B/C  
**When** der Smoke-Test ausgeführt wird  
**Then**

- **Wenn automated realisierbar:** Ein Skript oder Cargo-Integration-Test startet
  `cargo tauri dev` (oder das fertige Binary direkt), wartet 3 Sekunden, sendet SIGTERM (Unix)
  bzw. terminiert den Prozess programmatisch (Windows), und prüft Exit-Code 0 oder
  SIGTERM-termination (nicht Panic-Exit-Code 101)
- **Wenn nicht trivial automatisierbar** (Tauri-Dev-Server verlangt Display/WebView): Das AC
  wird als manuelle Smoke-Test-Instruktion in der `Technical Notes`-Sektion dokumentiert:
  `cargo tauri dev` starten, Fenster per X-Button schließen, Exit-Code 0 in Terminal prüfen.
  Der Test wird als `#[ignore]` in der Test-Suite geführt oder im `Makefile`/`xtask`-Subcommand
  als `smoke-test-windows-shell` registriert.
- Beide Varianten (automated + manuelle Fallback-Instruktion) sind in der Technical-Notes-Section
  dokumentiert

## Technical Notes

### Plugin-Version-Pinning

`tauri-plugin-global-shortcut` Version-Pin folgt der ADR-0002-Präzedenz für RC-Plugins: konkrete
Version in `Cargo.toml` pinned (kein `>=`-Wildcard). Upgrade-Gate beim Phase-2-Start analog
tauri-specta-RC-Upgrade-Policy.

### Project-Layout

Tauri v2 Convention: `src-tauri/` innerhalb `shells/windows/`. Workspace-Root kennt das Crate als
`member = ["shells/windows/src-tauri"]`. Die `tauri.conf.json` liegt in `src-tauri/`.

### Frontend-Scope-Fence

Kein React/Vue/Svelte in dieser Story. Das Main-Window rendert statisches HTML (Placeholder).
Frontend-Setup kommt in Phase 2 mit Settings-Panel + Pill-Bar. Keine `node_modules`, keine
`package.json` in `shells/windows/` erforderlich für diese Story.

### SD-4 Boot-Error-UX (Deferral-Anchor)

ADR-0009 §Sub-Decision 4 beschreibt drei Optionen für Pipeline-Boot-Validation-Errors (PipelineValidation
ErrorKind) die auftreten bevor Frontend-Listener aktiv sind. Story-3.1-Pre-Flight muss eine der
drei Optionen wählen:

- **(a)** Splash-Screen mit Pre-Event-Registration
- **(b)** Native OS-Error-Dialog via `tauri::dialog`
- **(c)** Degraded-Tray-Mode + Post-hoc-Emit (Soft-Recommendation ADR-0009)

Diese Story schafft den Skeleton; die SD-4-Resolution ist implizit durch die Story 3.3
(Orchestrator) + Story 3.8 (ErrorEmitter-Wiring) abgedeckt. AC-D enthält den Panic-Deferral-Anchor
für i18n-Load-Errors.

### i18n-Type-Shape

`I18nTable` ist Phase-1 minimal: `pub struct I18nTable(pub HashMap<String, String>)` oder
`pub type I18nTable = HashMap<String, String>`. Kein ICU-MessageFormat, keine Pluralization in
Phase 1. Locale-Selection-Config (aus `config.toml`) kommt in Story 3.2.

### Smoke-Test-Entscheidung

Tauri-Dev erfordert einen Display-Context (WebView). Auf dem Windows-Entwicklungsrechner ist
`cargo tauri dev` manuell ausführbar. CI-headless-Automatisierung ist nur via `xvfb-run` auf
Linux möglich, was Windows-Shell-Scope nicht passt. Empfehlung: manuelle Smoke-Test-Instruktion
+ `xtask smoke-test-windows-shell`-Subcommand als Dokumentations-Anchor.

## Dependencies

- Keine Story-Dependencies (Welle-1, dependency-free)
- ADR-0002 §Version-Pinning — tauri-specta RC-Pin-Präzedenz
- ADR-0006 Amendment 2 — `cfg(target_os = "windows")`-Gate-Pattern
- ADR-0009 §SD-4 — Boot-Error-UX-Deferral-Anchor
- ADR-0011 §SD-4 — Integration-Point: Registration in `.setup(..)`
- ADR-0012 §SD-3 — Windows-Shell-Bootstrap: Orchestrator-State-Slot (Story 3.3 füllt ihn)
