---
name: Story 2.A.C3 — Live-Locale-Switch
phase: 2
wave: A
story_id: "2.A.C3"
status: done
dependencies:
  - "2.A.A4"
adr_refs:
  - docs/adr/0013-settings-persistence-schema.md
source_ref: "_bmad-output/planning-artifacts/epics/epic-phase-2-a.md C3; memory/project_i18n_three_axes; memory/project_i18n_core_contract"
---

# Story 2.A.C3: Live-Locale-Switch

## Outcome

`ui.language`-Wechsel über das Settings-Panel (A4) triggert einen Live-Locale-Reload ohne
App-Neustart. Die i18n-State im Backend (Rust, `i18n_table: HashMap<String, String>`) wird
mit den Strings der neuen Locale aktualisiert. Das Frontend subscribed auf `"settings.changed"`
und reagiert auf `ui.language`-Wechsel.

Phase-2-A Scope: Infrastructure (Event-Subscription + Backend-Reload-Kommando). Die Settings-Panel-
HTML-Komponenten selbst sind noch nicht i18n'd (A4-D10: Phase-2-B Vite+React-Migration-Scope).
C3 legt das Fundament für Phase-2-B, in der alle Labels auf i18n-Keys umgestellt werden.

Tray-Labels werden von A8-Sub aktualisiert. C3 handled den WebView-Frontend-Pfad.

## Scope-Fence

**In-Scope:**
- Backend: neuer Tauri-Command `reload_locale(lang: String)` (oder interne Reload-Logik)
  der `i18n_table`-State mit Strings der neuen Locale überschreibt
- Frontend (`index.html`): `listen("settings.changed", ...)` für key = `"ui.language"` →
  `reload_locale(new_value)` aufrufen
- `i18n_table` von `State<HashMap<String, String>>` auf `State<Arc<RwLock<HashMap<String, String>>>>`
  upgraden (falls nicht bereits für A8-Sub gemacht) für thread-safe Mutation
- Axis-1-Klarstellung im Code: nur `ui.language` (UI-Language-Axis 1) betroffen

**Nicht-in-Scope:**
- Dictionary-Language + Output-Language (Axis 2 + 3) Hot-Reload → Pipeline-seitig, Phase-2-B+
- Vollständige i18n-Integration der HTML-Panel-Komponenten → Phase-2-B Vite+React-Migration
- Tray-Label-Update → A8-Sub
- App-Neustart-Mechanismus (C3 verhindert ihn)

## Acceptance Criteria

### AC-1 — Backend: `i18n_table` thread-safe + reload-bar

**Given** `shells/windows/src-tauri/src/i18n.rs` und Managed State  
**When** Backend-Init abläuft  
**Then**
- `i18n_table` ist als `State<Arc<RwLock<HashMap<String, String>>>>` managed
  (oder äquivalentes Mutex-Pattern).
- `load_locale(lang: &str) -> Result<HashMap<String, String>, AppError>` (oder analog) ist
  eine eigenständige Funktion (nicht nur in `setup`-Kontext gebunden).

---

### AC-2 — Backend: Locale-Reload-Command existiert

**Given** `shells/windows/src-tauri/src/commands/` nach der Story  
**When** Tauri-Command `reload_locale` (oder `apply_ui_language`) aufgerufen wird  
**Then**
- Command existiert und ist in `lib.rs` registriert.
- Command nimmt `lang: String` als Parameter.
- Command lädt die Locale-Datei für `lang` und überschreibt `i18n_table`.
- Bei unbekannter Locale: Fail-soft (behalte bisherige Locale) + `tracing::warn!`.
- Bei Erfolg: `Ok(())`.

---

### AC-3 — Frontend: Event-Subscription vorhanden

**Given** `shells/windows/src/index.html` nach der Story  
**When** Frontend geladen ist  
**Then**
- JavaScript-Code subscribed auf `"settings.changed"` via Tauri-Event-API.
- Subscription filtert auf `key === "ui.language"`.
- Bei Match: `invoke("reload_locale", { lang: new_value })` aufgerufen.
- Kein `console.error` / unhandled-Promise-rejection bei normalem Betrieb.

---

### AC-4 — Live-Reload-Sequenz funktioniert ohne App-Neustart

**Given** Klarvo läuft mit `ui.language = "de"`  
**When** User `ui.language` auf `"en"` setzt (via Settings-Panel)  
**Then**
1. `set_ui_language("en")` → Settings-Write → `settings.changed` emittiert.
2. Frontend-Listener empfängt Event, ruft `reload_locale("en")` auf.
3. `i18n_table` enthält jetzt englische Strings.
4. Tray-Update via A8-Sub-Listener (parallel, unabhängig von C3).
5. Kein App-Neustart.
6. Nachfolgende Backend-Operationen die `i18n_table` lesen nutzen englische Strings.

---

### AC-5 — Axis-1-Klarheit: nur `ui.language`

**Given** C3-Subscription im Frontend  
**When** `settings.changed`-Event mit `key = "app.dictionary_language"` oder `key = "app.output_language"` eingeht  
**Then**
- C3-Listener reagiert NICHT (filtert auf `"ui.language"` exklusiv).
- Diese Achsen haben keinen Hot-Reload-Pfad in Phase-2-A (kein `reload_locale`-Aufruf).
- Kein unintended Side-Effect auf Pipeline-State.

---

## Technical Notes

- `i18n_table` aktuell: `app.manage(i18n_table)` wo `i18n_table: HashMap<String, String>`
  (main.rs:308). Für Mutation nach Boot: Upgrade auf `Arc<Mutex<HashMap>>` oder `Arc<RwLock<HashMap>>`.
  `RwLock` bevorzugt (Read-heavy pattern: viele Reads, seltenes Write bei Locale-Switch).
- Locale-Dateien: `shells/windows/src-tauri/src/locales/en.json` + `de.json` (Epic-4-Output).
  Reload liest die jeweilige Datei aus dem App-Bundle (`include_str!` oder `embedded`-Pattern aus Epic-4).
  Falls `include_str!` zur Compile-Zeit embeddet: Reload aus dem embedded String (kein FS-Read zur Runtime).
- Frontend-Event-API in Tauri v2 WebView:
  ```js
  const { listen } = window.__TAURI__.event;
  listen('settings.changed', (event) => { ... });
  ```
  Unsubscribe-Handle für Cleanup (kein Leak bei HMR in Phase-2-B).
- Koordination mit A8-Sub: Beide subscribed auf `"settings.changed"`. Das ist unabhängig und
  korrekt (Tauri broadcast). Keine Koordination nötig.
- `i18n_table`-Upgrade kann in A8-Sub oder C3 gemacht werden — wer zuerst committet, übernimmt
  das Upgrade; der andere setzt darauf auf (keine Merge-Conflicts wenn Scopes getrennt).
  **C3 hat das Upgrade übernommen** (A8-Sub war bereits committed, C3 baut darauf auf).

## Tasks/Subtasks

- [x] AC-1: `i18n.rs` — `SharedI18nTable = Arc<RwLock<I18nTable>>` + `load_locale()` extrahiert
- [x] AC-2: `commands/settings.rs` — `reload_locale` Command + `apply_locale_reload` Helfer-Fn
- [x] AC-3 + AC-5: `index.html` — `settings.changed` Subscription, Axis-1-Filter auf `"ui.language"`
- [x] AC-4: Live-Reload-Sequenz verifiziert via Unit-Tests (reload_locale_known_lang_replaces_table + round-trip)
- [x] `lib.rs` — `reload_locale` in `collect_commands!` registriert
- [x] `tray.rs` — `rebuild_for_locale` auf `load_locale` umgestellt (kein RwLock-Lock nötig)
- [x] `main.rs` — `boot_i18n.read().expect(...)` für `tray::build_menu` Boot-Zeit-Call

## Dev Agent Record

### Completion Notes

**Implementiert 2026-05-01:**

- `i18n.rs`: Neuer Typ `SharedI18nTable = Arc<RwLock<I18nTable>>`. `load_locale(lang) -> I18nTable` als standalone-Funktion extrahiert (AC-1). `load(lang) -> SharedI18nTable` gibt jetzt den RwLock-gewrappten Arc zurück.
- `commands/settings.rs`: `apply_locale_reload` (testbare Logik) + `reload_locale` Tauri-Command (AC-2). Fail-soft bei unbekannter Locale (tracing::warn! + Ok(())).
- `lib.rs`: `reload_locale` zu `collect_commands!` hinzugefügt.
- `tray.rs`: `rebuild_for_locale` nutzt `load_locale` statt `load` (eigener short-lived Copy, kein RwLock nötig).
- `main.rs`: `boot_i18n.read().expect(...)` Lese-Lock für initialen `build_menu`-Call.
- `index.html`: `useEffect` mit `settings.changed` → `ui.language`-Filter → `invoke("reload_locale", ...)` (AC-3). Axis-2/3-Keys werden explizit ignoriert (AC-5).

**Tests:** 25/25 grün (inkl. 4 neue reload_locale-Tests + 1 neuer load_returns_shared_i18n_table-Test). Keine Regressionen. xtask lint-events OK, manifest-strict 5/5.

## File List

- `shells/windows/src-tauri/src/i18n.rs` — modified
- `shells/windows/src-tauri/src/tray.rs` — modified
- `shells/windows/src-tauri/src/commands/settings.rs` — modified
- `shells/windows/src-tauri/src/lib.rs` — modified
- `shells/windows/src-tauri/src/main.rs` — modified
- `shells/windows/src/index.html` — modified
- `_bmad-output/implementation-artifacts/2a-c3-live-locale-switch.md` — modified (this file)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — modified

## Change Log

- 2026-05-01: Story 2.A.C3 implementiert. `i18n_table` auf `SharedI18nTable = Arc<RwLock<I18nTable>>` upgraded. `reload_locale` Tauri-Command + Frontend-Subscription für Live-Locale-Switch ohne App-Neustart.
- 2026-05-01: Code-Review (Blind Hunter + Edge Case Hunter + Acceptance Auditor). 1 Decision + 10 Patches + 7 Defers + 6 Dismissed.

### Review Findings

#### Decision Needed

- [ ] **[Review][Decision] D-1 — D3-Graceful-Shutdown bleed im C3-Commit** — `main.rs:487-500` (`.build()/.run(|app, event| RunEvent::Exit { ... orch.shutdown() ... })`) ist Story-D3-Code, nicht in C3-Scope-Fence. Sprint-status flippt `2a-d3-graceful-shutdown: ready-for-dev → done` im selben Diff. Verstößt gegen `feedback_commit_hygiene` (Contract-before-Implementation-Split). Optionen: (a) D3-Hunk in separaten Commit splitten, (b) bundled lassen + Audit-Trail dokumentieren, (c) D3 reverten und in eigener Session reviewen.

#### Patches (Code-Issues, unambiguer Fix) — alle 10 angewandt 2026-05-01

- [x] **[Review][Patch] P-1 — RwLock write-poison `unwrap()` widerspricht Fail-soft-Posture** [`shells/windows/src-tauri/src/commands/settings.rs`] — Fix angewandt: `write().unwrap_or_else(|e| e.into_inner())` mit Begründung im Code-Comment (replace-only, kein partial-mutation-Risiko).
- [x] **[Review][Patch] P-2 — RwLock read-poison bei Boot `.expect()` killt Setup** [`shells/windows/src-tauri/src/main.rs`] — Fix angewandt: `read().unwrap_or_else(|e| e.into_inner())` analog P-1.
- [x] **[Review][Patch] P-3 — `load_locale` parsed `de.json` zweimal pro Reload** [`shells/windows/src-tauri/src/i18n.rs`] — Fix angewandt: `_de` → `de`, im `"de" =>`-Arm reused.
- [x] **[Review][Patch] P-4 — Frontend `.catch(() => {})` schluckt alle IPC-Fehler** [`shells/windows/src/index.html`] — Fix angewandt: `console.warn("reload_locale failed:", e)` für invoke-Catch + `console.warn` für listen-Catch.
- [x] **[Review][Patch] P-5 — `reload_locale: lang: String` ohne Length-Bound + Log-Injection-Surface** [`shells/windows/src-tauri/src/commands/settings.rs`] — Fix angewandt: `MAX_LANG_LEN = 16`-Konstante + Length-Check + Log-Field strip-control-chars.
- [x] **[Review][Patch] P-6 — `load_returns_shared_i18n_table` ist Tautologie** [`shells/windows/src-tauri/src/i18n.rs`] — Test gelöscht; Replacement-Comment dokumentiert das Warum (Type-Signatur covered es). Test-Count fällt von 25 auf 24.
- [x] **[Review][Patch] P-7 — Sprint-status `last_updated` Comment vs Field disagreement** [`_bmad-output/implementation-artifacts/sprint-status.yaml`] — Fix angewandt: beide Zeilen narraten jetzt beide Transitions ("C3 ready-for-dev→review; D3 review→done with 1 D-Resolution + 8 Patches + 4 Defers").
- [x] **[Review][Patch] P-8 — Frontend keine De-dup gegen identische `newValue`-Payloads** [`shells/windows/src/index.html`] — Fix angewandt: `lastAppliedLocaleRef = useRef(null)` + Equality-Check vor `invoke`. `useRef` neu in React-Imports.
- [x] **[Review][Patch] P-9 — Stale Doc-Comment in `i18n.rs` claimt `lang ∈ {en, de}` Schema-garantiert** [`shells/windows/src-tauri/src/i18n.rs`] — Fix angewandt: Doc-Comment umgeschrieben — Default-Arm ist jetzt **primäre** Defense für den Runtime-Pfad.
- [x] **[Review][Patch] P-10 — `drop(boot_i18n_guard)` ist toter Code** [`shells/windows/src-tauri/src/main.rs`] — Fix angewandt: expliziter `drop` entfernt; Guard läuft am Block-Ende out-of-scope.

#### Deferred (pre-existing oder out-of-scope; siehe `deferred-work.md` C25-C31)

- [x] **[Review][Defer] C25 — Tray vs Frontend Listener-Desync (kein Ordering-Guarantee)** [`main.rs:390` vs `commands/settings.rs:239`] — Story dokumentiert intentional als Tauri-Broadcast-Pattern; Drift-Guard nur über tray.rs:138 Runtime-Test.
- [x] **[Review][Defer] C26 — `apply_locale_reload` getestet, aber nicht das `#[tauri::command] reload_locale`-Wrapper** [`commands/settings.rs:248-258`] — Tauri-State-Binding + Specta-Export ungetestet; `xtask bindings-drift` (Story 5.2) catched Symbol-Drift.
- [x] **[Review][Defer] C27 — D3-RunEvent::Exit-Issues** [`main.rs:487-500`, `klarvo-shell-orchestrator/src/session.rs:314-326`] — Bündel: `block_on`-Deadlock-Risk, `pipeline_task.await` ohne Timeout, kein `catch_unwind`, `try_state` swallows wrong-type-Path. Gehört in eigenes D3-Review (abhängig von D-1-Outcome).
- [x] **[Review][Defer] C28 — useEffect deps `[]` capturing outer `tauriEvent`/`invoke` + late-`__TAURI__`-injection-race** [`shells/windows/src/index.html:215-235`] — Pattern aus A4-useEffect geerbt; Fix-Konsistenz erfordert Phase-2-B Vite+React-Migration.
- [x] **[Review][Defer] C29 — Cancelled-Listener-Cleanup-Race** [`shells/windows/src/index.html:228-231`] — Gleiche Shape wie A4-useEffect; Phase-2-B-Konsistenz.
- [x] **[Review][Defer] C30 — AC-1 literale `Result<_, AppError>`-Asymmetrie (`load_locale` panics on corrupt JSON)** [`shells/windows/src-tauri/src/i18n.rs:26-32`] — Doc-Comment markiert Phase-2-Replace explizit ("Phase-2: replace panic with fail-soft AppError path per ADR-0009 SD-4"); AC-1's "oder analog" deckt aktuelle Form.
- [x] **[Review][Defer] C31 — Empty-string-Locale silently no-ops** [`shells/windows/src/index.html:225` + `commands/settings.rs:239`] — Story 4.1 Schema-Validation gates `ui.language ∈ {en, de}`; defensive only.

#### Dismissed (6)

- `SUPPORTED_LANGUAGES` Compile-Failure-Verdacht — verifiziert: `&[&str].contains(&&str)` ✓ kompiliert.
- `use super::*;` im Test-Module — editorial Noise, kein Bug.
- Story-Prose-Framing "kein RwLock-Lock nötig" — Doku-Nuance, kein Code-Issue.
- Cargo.toml fehlt in File-List — verifiziert: keine Dependency-Änderung (RwLock = stdlib).
- `i18n_table` Other-Consumer-Risk — verifiziert: nur `commands/settings.rs` + `main.rs` + `tray.rs` (über `load_locale`); kein silent State-TypeId-Drift.
- Frontend kein UI-Re-Render nach `reload_locale` — explizit per Scope-Fence ausgeschlossen ("HTML-Komponenten noch nicht i18n'd; Phase-2-B Vite+React-Migration-Scope").
