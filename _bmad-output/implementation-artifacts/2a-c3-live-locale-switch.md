---
name: Story 2.A.C3 — Live-Locale-Switch
phase: 2
wave: A
story_id: "2.A.C3"
status: ready
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
