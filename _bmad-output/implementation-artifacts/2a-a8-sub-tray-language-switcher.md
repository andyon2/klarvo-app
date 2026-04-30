---
name: Story 2.A.A8-Sub — Tray-Language-Switcher
phase: 2
wave: A
story_id: "2.A.A8-Sub"
status: review
dependencies:
  - "2.A.A4"
adr_refs:
  - docs/adr/0013-settings-persistence-schema.md
source_ref: "_bmad-output/planning-artifacts/epics/epic-phase-2-a.md A8-Sub; ADR-0013 Sub-Decision 5"
---

# Story 2.A.A8-Sub: Tray-Language-Switcher

## Outcome

Das Tray-Menü zeigt ein Language-Submenu. Wenn User `ui.language` im Settings-Panel ändert
(via `set_ui_language`-Command aus A4), aktualisiert das Tray-Menü seine Labels live ohne
App-Neustart. Kein eigener Settings-Write durch A8-Sub — nur Reaktion auf `"settings.changed"`-Event.

`TauriSettingsEmitter` (A4) emittiert bei jedem Settings-Write `SettingsChangedEvent { key, new_value }`.
A8-Sub subscribed auf dieses Event im Backend (Tauri `app.listen()`), filtert auf key = `"ui.language"`
und rebuild den Tray-Menu.

## Scope-Fence

**In-Scope:**
- Tray-Menü: Language-Submenu mit den verfügbaren Locales (en/de)
- Backend: `app.listen("settings.changed", ...)` für key = `"ui.language"` → `tray.set_menu()`
- i18n-Keys: mindestens `tray.language_switcher.label` + Locale-Labels (z. B. `tray.language.en`, `tray.language.de`)
- Neue i18n-Keys in `en.json` + `de.json` eintragen

**Nicht-in-Scope:**
- Settings-Write (kein `set_ui_language`-Aufruf durch A8-Sub — nur Listen)
- Frontend-WebView-Locale-Reload → C3
- Second-Language-Axis-Switching (Dictionary-Language / Output-Language) — separates Feature

## Acceptance Criteria

### AC-1 — Language-Submenu im Tray-Menü bei Boot

**Given** Klarvo startet mit einer konfigurierten `ui.language` (z. B. `"de"`)  
**When** Tray-Icon initialisiert wird  
**Then**
- Tray-Menü enthält ein Language-Submenu (z. B. `"Sprache"` / `"Language"` — je nach boot-locale).
- Submenu zeigt die verfügbaren Locales: `"English"` + `"Deutsch"` (oder deren i18n-Labels).
- Aktuelle Locale ist im Submenu visuell erkennbar (z. B. via CheckMenuItem oder Label-Suffix `"✓"`).
- `tray.language_switcher.label` i18n-Key genutzt für den Submenu-Header.

---

### AC-2 — Event-Subscription: `settings.changed` + key-Filter

**Given** `TauriSettingsEmitter.emit_settings_changed("ui.language", "en")` aufgerufen  
**When** `"settings.changed"`-Event emittiert wird  
**Then**
- Backend-Listener filtert auf `key == "ui.language"`.
- Events mit anderem Key (z. B. `"hotkey.slot1.combo"`) werden ignoriert.
- Kein Deadlock, kein Panic im Listener.

---

### AC-3 — Live-Tray-Update nach Language-Wechsel

**Given** User wechselt `ui.language` von `"de"` auf `"en"` via Settings-Panel (A4)  
**When** `"settings.changed"` Event mit `key = "ui.language"`, `new_value = "en"` eingeht  
**Then**
- Tray-Menü wird mit neuer Locale rebuilt: Submenu-Header = englischer `tray.language_switcher.label`-Wert.
- Exit-Label und andere Tray-Labels werden ebenfalls zur neuen Locale aktualisiert
  (alle `tray.*`-Keys re-laden via i18n_table).
- `tray.set_menu(new_menu)` wird aufgerufen (kein App-Neustart).

---

### AC-4 — i18n-Keys registriert und Coverage-Gate-grün

**Given** `cargo xtask lint-events` nach der Story  
**When** G3-Sub-Lint B (Locale-Coverage) läuft  
**Then**
- `tray.language_switcher.label` in `en.json` + `de.json` vorhanden.
- Locale-spezifische Labels (z. B. `tray.language.en`, `tray.language.de`) in beiden Locales vorhanden.
- `cargo xtask lint-events` Exit 0 (kein new orphan key, kein missing key).

---

### AC-5 — Kein Settings-Write durch Tray-Language-Click (Scope-Fence)

**Given** User klickt einen Language-Eintrag im Tray-Submenu  
**When** Menu-Event-Handler läuft  
**Then**
- **Kein `set_ui_language`-Command-Aufruf durch A8-Sub's Menu-Event-Handler.**
  (A8-Sub ist nur reaktiv; aktives Switching via Settings-Panel ist A4/C3-Scope).
- Option A (einfach): Menu-Click triggert `app.emit("settings.changed", ...)`-simulierten Pfad
  via `set_ui_language`-Command.
- Option B (minimal): Menu-Click ist read-only / greyed-out in Phase-2-A; Language-Switch
  ausschließlich über Settings-Panel. Scope-Decision liegt bei Implementierung.
- Whichever Option gewählt: Scope-Fence ist dokumentiert im Commit-Message.

---

## Technical Notes

- `TrayIconBuilder::with_id("klarvo-tray")` (main.rs:334) erzeugt den tray. `tray.set_menu()`
  updated das Menu nach Boot.
- Tauri v2 `app.listen()` für global events: `app_handle.listen("settings.changed", |event| {...})`.
  Listener-Handle muss gespeichert werden (sonst wird er beim Drop entfernt).
- i18n_table ist aktuell als `State<HashMap<String, String>>` managed (main.rs:308).
  Um neue Tray-Labels bei Locale-Wechsel zu laden: i18n_table via State laden + neue Labels
  aus der neuen Locale-Datei lesen (oder i18n_table als `Arc<RwLock<HashMap>>` upgraden).
- Tauri-`CheckMenuItem` (`tauri::menu::CheckMenuItemBuilder`) für die aktuelle Locale markieren.
- Falls `i18n_table`-Update-Mechanismus für C3 designed wird: A8-Sub kann davon profitieren
  (shared reload-logic). Keine Voraussetzung für A8-Sub, aber koordinieren wenn C3 parallel.

## Spec-Deviations

**AC-5 Option B gewählt (Menu-Click ist no-op):**
Die Sprach-Items im Tray-Submenu werden als `CheckMenuItem` mit `enabled(false)` gerendert
— sie zeigen die aktive Locale visuell an, lösen aber keinen Settings-Write aus. Active
switching bleibt ausschließlich beim Settings-Panel (`set_ui_language`-Command, A4); A8-Sub
ist rein reaktiv auf `settings.changed` (Outcome-Spec). Der `on_menu_event`-Handler ignoriert
`language.*`-IDs defensiv mit einem `tracing::debug` (für den Fall, dass eine Plattform die
Events trotz `enabled(false)` zustellt).

**Co-located Hygiene-Patch in `i18n.rs::REQUIRED_KEYS`:**
A4-Pass-2-Patch P2-P11 hat `error.unknown` in `en.json`/`de.json` ergänzt, aber die
`REQUIRED_KEYS`-Konstante in `shells/windows/src-tauri/src/i18n.rs` nicht mitgepflegt — das
liess den `no_orphan_keys_in_en_json`-Test seit 2026-04-30 rot. Da A8-Sub diese Konstante
ohnehin um drei `tray.*`-Keys erweitert, wurde `error.unknown` in derselben Edit als
A4-Followup ergänzt; der Test ist wieder grün.

## Dev Agent Record

### Completion Notes

- AC-1: `tray::build_menu` erzeugt das Boot-Menü mit Info-Item, Language-Submenu (Header
  via `tray.language_switcher.label`) und Exit-Item. Submenu enthält `CheckMenuItem`s für
  jede Locale aus `tray::SUPPORTED_LOCALES`; das Item, dessen Code dem aktiven `ui.language`
  entspricht, wird gechecked. Fallback: fehlende i18n-Keys nutzen den Locale-Code bzw.
  `"Language"`/`"Exit"` als statische Defaults, damit ein unvollständig übersetzter Locale
  das Tray nicht blockiert.
- AC-2: Neuer `app.listen("settings.changed", …)`-Handler in `main.rs` Step 11c filtert
  strikt auf `key == "ui.language"`; alle anderen Keys werden früh per `return` abgebrochen.
  Payload-Parse-Fehler werden ignoriert (kein Panic). Der bestehende Recording-Mode-Listener
  in Step 11b läuft unverändert parallel — beide Listener arbeiten unabhängig und kollidieren
  nicht.
- AC-3: Bei `key=="ui.language"` ruft der Listener `tray::rebuild_for_locale(app, new_locale)`,
  das die Locale-Datei frisch via `i18n::load(new_locale)` lädt, das Menü neu baut und über
  `app.tray_by_id("klarvo-tray")` mit `tray.set_menu(Some(menu))` ersetzt. Damit werden alle
  `tray.*`-Labels (Submenu-Header, Locale-Items, Exit-Label) zur neuen Locale aktualisiert,
  ohne App-Neustart oder Mutation des Boot-`I18nTable`-Snapshots.
- AC-4: `tray.language_switcher.label`, `tray.language.en`, `tray.language.de` sind in
  `en.json` + `de.json` registriert und in `REQUIRED_KEYS` aufgenommen. `cargo xtask
  lint-events` Exit 0 (5 Events, kein Drift). Co-located: `error.unknown` ebenfalls in
  `REQUIRED_KEYS` ergänzt (siehe Spec-Deviations); Orphan-Test wieder grün.
- AC-5: Option B (read-only/disabled). `CheckMenuItem::enabled(false)` für alle Locale-Items;
  `set_ui_language`-Command wird nicht aus `on_menu_event` aufgerufen. Defensive Debug-Log
  wenn doch ein `language.*`-Click ankommt.

### File List

- `shells/windows/src-tauri/src/tray.rs` (neu — `TRAY_ID`, `SUPPORTED_LOCALES`, `build_menu`,
  `rebuild_for_locale` + 2 Unit-Tests)
- `shells/windows/src-tauri/src/lib.rs` (geändert — `pub mod tray;`)
- `shells/windows/src-tauri/src/main.rs` (geändert — boot-locale snapshot, Step 11c
  `settings.changed`-Listener für `ui.language`, Tray-Builder nutzt `tray::build_menu`)
- `shells/windows/src-tauri/src/i18n.rs` (geändert — REQUIRED_KEYS um 3 tray-Keys +
  co-located `error.unknown` erweitert)
- `shells/windows/locales/en.json` (geändert — 3 neue `tray.*`-Keys)
- `shells/windows/locales/de.json` (geändert — 3 neue `tray.*`-Keys)
- `_bmad-output/implementation-artifacts/2a-a8-sub-tray-language-switcher.md` (geändert —
  Spec-Deviations + Dev Agent Record + Status review)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (geändert — Story-Status
  ready-for-dev → review)

### Change Log

- 2026-04-30: Story 2.A.A8-Sub implementiert — Tray-Language-Submenu, reaktiver
  `settings.changed`-Listener, 3 neue i18n-Keys; alle xtask-Gates grün, 19 windows-shell
  Lib-Tests grün (inkl. 2 neue tray-Tests); A4-Followup-Patch `error.unknown` co-located.

## Review Findings

Code-Review 2026-04-30 (Blind Hunter + Edge Case Hunter + Acceptance Auditor, parallele isolierte Reviewer).

**Triage:** 1 decision-needed, 3 patches, 4 deferred, 12 dismissed (false-positives / by-design / out-of-scope).

**Acceptance-Verdict (Auditor):** Alle 5 ACs sind durch sauberen, well-scoped Code in `tray.rs`, `main.rs` Step 11c, `i18n.rs` REQUIRED_KEYS-Update und Locale-Files erfüllt. Spec-Deviations (AC-5 Option B + co-located `error.unknown`) sind akkurat dokumentiert. **Caveat:** Working-Tree enthält substantielle 2.B.A1-Code-Review-Closure-Patches die nicht in A8-Sub's File List stehen (siehe Decision-Needed unten).

### Decision Needed

- [ ] [Review][Decision] **Scope-Coupling — 2.B.A1-Code-Review-Closure im Working-Tree** — Auditor: ~330 LOC `klarvo-shell-orchestrator/src/session.rs`-Refactor + `klarvo-core/src/recording/mod.rs` (`Copy`-Derive) + `klarvo-core/src/settings/mod.rs` (Default-Const-Refactor) + `shells/windows/src-tauri/src/commands/settings.rs` (AppError-Propagation) + `shells/windows/src/bindings/index.ts` (regenerated) + 2.B.A1-Spec-Amendments + `sprint-status.yaml`-Flip `2b-a1: review→done` + i18n-Key `error.recording.timeout` sind im uncommitted-Diff, gehören aber zur 2.B.A1-Code-Review-Closure (Post-29ce800), nicht zu A8-Sub's File List. Optionen: (A) Working-Tree in zwei Commits splitten — A8-Sub-only + 2.B.A1-Code-Review-Closure separat; (B) gemischter Commit mit klarer Cross-Story-Coupling-Doku im Body; (C) Pragmatisch akzeptieren und in A8-Sub-Commit-Message als Coupling dokumentieren.

### Patch

- [ ] [Review][Patch] **Listener-Diagnostik fehlt — keine Tracing-Breadcrumb auf gefilterten/dropped Events** [`shells/windows/src-tauri/src/main.rs:351-365`] — Step-11c-Listener returnt silent bei (a) Parse-Fail, (b) non-`ui.language`-Key, (c) fehlendem `newValue`. Future-Debug von "warum zeigt mein Tray die alte Locale" hat keinen Breadcrumb. Fix: `tracing::trace!`/`tracing::warn!` in jedem Early-Return.

- [ ] [Review][Patch] **`newValue`-Validation gegen SUPPORTED_LOCALES fehlt — silent fallthrough** [`shells/windows/src-tauri/src/main.rs:360-363` + `shells/windows/src-tauri/src/tray.rs:69`] — Bei `newValue = "fr"` / `""` / `"EN"`: `i18n::load` fällt zurück auf en-Tabelle (correct), aber `active_locale` bleibt die Raw-String → `active_locale == *code` ist für beide en/de false → Tray zeigt keinen Checkmark, kein Error-Log. Fix: `new_locale` gegen `tray::SUPPORTED_LOCALES`-Codes validieren vor `rebuild_for_locale`, `tracing::warn!` bei unsupported.

- [ ] [Review][Patch] **`SUPPORTED_LOCALES`-Drift gegen `ShellConfig::ui_language`-Allow-List ungesichert** [`shells/windows/src-tauri/src/tray.rs:30-33`] — `tray::SUPPORTED_LOCALES = &[("en", …), ("de", …)]` und die Schema-Validation in `ShellConfig` (Story 4.1 AC-C) sind separate Sources-of-Truth. Adding `"fr"` zu einem ohne das andere → Tray ohne Submenu-Eintrag + ohne Checkmark. Fix: Unit-Test der asserted, dass `SUPPORTED_LOCALES`-Codes ⊆ `ShellConfig`-Allow-List.

### Deferred

- [x] [Review][Defer] **A8-F1 — Rapid Locale-Toggling: kein Debounce/Coalescing** [`shells/windows/src-tauri/src/main.rs:351-365`] — deferred, low-priority (User-driven Event, kein Hot-Path)
- [x] [Review][Defer] **A8-F2 — `build_menu`/`tray.set_menu`-Failure → kein User-Toast** [`shells/windows/src-tauri/src/tray.rs:92-105`] — deferred, ErrorEmitter-Wiring in Listener nötig (medium-low priority)
- [x] [Review][Defer] **A8-F3 — Old-Menu-Lifecycle / Win32-GDI-Handle-Leak unverifiziert** [`shells/windows/src-tauri/src/tray.rs:103`] — deferred, Tauri-Internal-Behavior; out-of-scope für A8-Sub
- [x] [Review][Defer] **A8-F4 — 2.B.A1-Code-Review-Findings (8 Items) — Review unter 2.B.A1-Spec** — Blind-Hunter erhob 8 Findings auf `session.rs`/`commands/settings.rs`/`recording/mod.rs` (AppError-`From`-Conversion-Visibility, AutoStop-Timeout-Cancel-Drops-VAD-Guard, `RecordingCompleted`-Emission-im-Refactor, `SessionState`-by-value-Pattern, sync-`set_recording_mode_slot1`, Test-Race in `autostop_transitions_to_idle_after_vad`). Per Scope-Audit gehören diese unter 2.B.A1's Spec, nicht A8-Sub. Closure abhängig von Decision-Needed Resolution.
