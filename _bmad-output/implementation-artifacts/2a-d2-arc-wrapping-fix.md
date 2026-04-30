---
name: Story 2.A.D2 — Arc-Wrapping-Duplikat-Fix
phase: 2
wave: A
story_id: "2.A.D2"
status: done
dependencies: []
adr_refs: []
source_ref: "deferred-work.md F3 / Epic-3-Code-Review"
---

# Story 2.A.D2: Arc-Wrapping-Duplikat-Fix

## Outcome

`State<Arc<T>>`-Pattern in Windows-Shell-Commands wird auf `State<T>` vereinfacht.
Hintergrund: Tauri v2 wraps managed values intern in `Arc` (`State(Arc::new(state))`), sodass
`app.manage(Arc<Settings>)` + `State<Arc<Settings>>` = `Arc<Arc<Settings>>` erzeugt —
doppelte Ref-Count-Indirektion ohne Mehrwert.

Nach dem Fix: `app.manage(settings_value)` + `State<Settings>` — ein Arc-Layer (Tauri-intern).
Kein Behavior-Change. Kein API-Change in `klarvo-core`.

## Scope-Fence

**In-Scope:**
- `shells/windows/src-tauri/src/main.rs` — `app.manage(Arc::clone(&settings))` → `app.manage(settings_value)`
- `shells/windows/src-tauri/src/commands/settings.rs` — alle Command-Signaturen `State<Arc<Settings>>` → `State<Settings>`
- Analog für `Arc<SessionOrchestrator>` falls dort gleiches Muster besteht

**Nicht-in-Scope:**
- `klarvo-core`-API (kein Change)
- `klarvo-shell-orchestrator`-API (kein Change)
- Behavior-Change irgendeiner Art

## Acceptance Criteria

### AC-1 — Compiler-Verification: kein `State<Arc<_>>` mehr in Shell-Commands

**Given** `shells/windows/src-tauri/src/commands/*.rs`  
**When** die Story gemergt ist  
**Then**
- Keine Command-Signatur enthält `tauri::State<'_, Arc<_>>` mehr.
- `app.manage(Arc::clone(&_))` Pattern ist aus `main.rs` entfernt (für Settings + SessionOrchestrator).
- Stattdessen wird direkt `app.manage(value)` mit dem Inner-Typ aufgerufen.

---

### AC-2 — Keine Regressionen: Tests grün

**Given** `cargo test` nach dem Fix  
**When** Tests laufen (ohne `--exclude klarvo-bridge-jni`)  
**Then**
- `cargo test -p klarvo-windows-shell` grün (oder äquivalentes Shell-Test-Subset auf Linux CI).
- `cargo test -p klarvo-core` grün.
- `cargo test -p klarvo-shell-orchestrator` grün.
- Kein neues `error[E]` oder `warning[W]` (Clippy-clean).

---

### AC-3 — Keine Behavior-Change im Settings-Flow

**Given** die geänderten Command-Handler  
**When** `get_setting` / `set_setting` Commands ausgeführt werden  
**Then**
- Alle 8 Settings-Commands funktionieren identisch wie vor dem Fix (lesen/schreiben/emittieren).
- `settings-changed`-Events werden weiterhin korrekt emittiert.

---

## Tasks/Subtasks

- [x] T1: `SessionOrchestrator` erhält `#[derive(Clone)]` — shallow Clone via Arc-Felder
- [x] T2: `settings.rs` — `use Arc` entfernen, alle 8 `State<'_, Arc<Settings>>` → `State<'_, Settings>`
- [x] T3: `main.rs` — Settings-Konstruktion ohne `Arc::new()`, `app.manage(settings)` statt `Arc::clone`
- [x] T4: `main.rs` — `let orch = SessionOrchestrator::new(...)` (kein `Arc::new`), `app.manage(orch)` direkt
- [x] T5: `hotkey.rs` — `app.state::<SessionOrchestrator>()` statt `Arc<SessionOrchestrator>`, Test/Kommentar aktualisieren
- [x] T6: `i18n.rs` — 3 fehlende REQUIRED_KEYS aus A4-Arbeit nachgepflegt (pre-existing, blockiert AC-2)
- [x] T7: Regressionstest `cargo test -p klarvo-core -p klarvo-shell-orchestrator -p klarvo-windows-shell --lib` → alle grün

### Review Findings

Code-Review 2026-04-30 (3-Layer: Blind Hunter, Edge Case Hunter, Acceptance Auditor) auf D2-fokussiertem Diff (Option B: `commands/settings.rs` ausgeklammert, da unter A4 Pass-1+Pass-2 reviewt). Result: **0 Patches, 0 Decisions, 10 Defer, 9 Dismiss** — Story substanziell sauber. Acceptance Auditor: "No findings" (alle ACs verifiziert, klarvo-core-Scan-Claim per `grep` bestätigt, Scope-Fence gehalten).

#### Deferred (10) — A4-Pass-3-Kandidaten (8) + D2-Out-of-Scope-Folge-Items (2)

- [x] [Review][Defer] Kein Test für `Clone`-shared-state-Semantik (Future-Footgun: nicht-Arc-Field bricht Invariante still) [`klarvo-shell-orchestrator/src/session.rs:39`] — deferred, good-to-have, ACs verlangen nicht
- [x] [Review][Defer] Andere Tauri-State-Typen (`config`, `keystore`, `emitter`, `clock`) noch in `Arc::clone(...)` → gleiches Doppel-Arc-Pattern wie für Settings/Orch [`shells/windows/src-tauri/src/main.rs:242-245`] — deferred, **explizit out-of-scope** per Story-Scope-Fence; Folge-Story-Kandidat
- [x] [Review][Defer] `debug_assert!(app.manage(x))` no-op in Release-Builds (gilt für alle 6 manage-Sites) [`shells/windows/src-tauri/src/main.rs:241-246`] — deferred, pre-existing Pattern, D2 erweitert nur Footprint
- [x] [Review][Defer] `ts_ms: 0u64` hardcoded in 3 `app.error`-Emit-Sites (verletzt MonotonicClock-Konvention) [`shells/windows/src-tauri/src/main.rs:113,153,170`] — deferred, A4-Pass-3-Kandidat, Frontend-Dedup/Sort kollabiert auf t=0
- [x] [Review][Defer] `resolve_config_path()`-Err-Arm asymmetrisch — kein `app.error`, `toml_loaded_ok=false` ohne User-Signal [`shells/windows/src-tauri/src/main.rs:118-121`] — deferred, A4-Pass-3-Kandidat (A4-P5 hat nur inner Arm gefixt)
- [x] [Review][Defer] `migrate_from_toml_if_needed`-Err nur `tracing::warn!` — kein User-Toast [`shells/windows/src-tauri/src/main.rs:200-202`] — deferred, A4-Pass-3-Kandidat, Silent-Failure-Hole
- [x] [Review][Defer] `PathBuf::new()`-Sentinel + `as_os_str().is_empty()`-Check → `Option<PathBuf>` wäre selbst-erklärend [`shells/windows/src-tauri/src/main.rs:142,149`] — deferred, A4-Pass-3-Kandidat, Readability
- [x] [Review][Defer] `app.error` in `.setup()` emittiert → potenziell vor Frontend-Listener-Attach (Lost-Toast-Risiko) [`shells/windows/src-tauri/src/main.rs:108-115,151-154,168-171`] — deferred, A4-Pass-3-Kandidat, Timing (vermutlich harmlos)
- [x] [Review][Defer] Boot-N Corrupt-TOML → UI-Write → Boot-N+1 Fixed-TOML: Sentinel-Key-Presence ≠ Migration-Done; 4 von 5 TOML-Feldern verlieren [`shells/windows/src-tauri/src/main.rs:187-202` + `klarvo-core/src/settings/migrate_from_toml_if_needed`] — deferred, A4-Architektur-Frage, Migration-Sentinel-Design
- [x] [Review][Defer] Keine Tests für neue Boot-Branches (app_data_dir-fail, mkdir-fail, db-open-fail, migration-fail) [`shells/windows/src-tauri/src/main.rs:133-203`] — deferred, A4-Pass-3-Kandidat, Test-Gap

#### Dismissed (9, dokumentiert für Pass-Vermeidung)

- **D1 — `SessionOrchestrator: Clone`-Safety** — Auditor verifiziert: alle Felder `Arc<…>`, Clone ist shallow, AC-3-Behavior gehalten. Falls Pass-Reviewer das wieder flaggen: erste Antwort = "AA-verified all fields Arc, see session.rs:39-55".
- **D2 — Hotkey full-struct Clone-Overhead vs Arc-Refcount-Bump** — Negligible (Atomic-Ops, all-Arc shallow). By design per Story-Outcome.
- **D3 — Hotkey-Callback-Registration-Race** — Nicht möglich: `register_hotkey` läuft in Step 12, nach `app.manage()` Step 11; Closure kann erst bei laufendem Event-Loop feuern.
- **D4 — `parse_failed`-Toast vor i18n-Table-Loaded** — A4-reviewed (P5); Frontend-Toast-i18n ist separate Listener-Concern.
- **D5 — `expect()` in Fallback-Chain panics in Release** — A4 P3 reviewed Two-Step-Fallback (TauriSettingsEmitter → NoopSettingsEmitter); finales `expect` ist dokumentiertes Last-Resort gegen "rusqlite in-memory infallible".
- **D6 — `create_dir_all`-Fail nicht gating Fallback** — A4-P1 reviewed; nachfolgender `Settings::open()`-Fail triggered ohnehin Fallback-Pfad.
- **D7 — TOML-Control-Char in Migration aborts silently** — A4-P2-P5 reviewed: `validate_setting_value` läuft per Row in Migration-Tx (Read/Write-Symmetrie).
- **D8 — `NoopSettingsEmitter` swallows Settings-Events im Fallback** — A4-P13 reviewed: in-memory-Pfad nutzt `Arc::clone(&settings_emitter)` mit echtem TauriSettingsEmitter; NoopSettingsEmitter ist nur Last-Resort wenn auch in_memory-Init failt.
- **D9 — `tauri::Emitter as _` Import-Placement im Function-Body** — Style.

## Dev Agent Record

### Implementation Notes

**Settings-Fix:**
`Settings` hat `Mutex<Connection>` + `Arc<dyn SettingsEmitter>` → erfüllt `Send + Sync + 'static` für direktes `app.manage()`. `Arc::new()` in der Konstruktionslogik entfernt, `app.manage(settings)` konsumiert den Wert. Alle 8 Command-Signaturen auf `State<'_, Settings>` umgestellt.

**SessionOrchestrator-Fix:**
`SessionOrchestrator` hat ausschließlich `Arc<_>`-Felder → `#[derive(Clone)]` ist cheap (nur Arc-Pointer-Copy). Der Hotkey-Callback ruft `app.state::<SessionOrchestrator>().inner().clone()` auf — liefert nun direkt `SessionOrchestrator` (shallow clone) statt `Arc<SessionOrchestrator>`. Keine `app.manage(Arc::clone(...))` mehr für beide Typen.

**klarvo-core-Scan:**
Gezielter Scan von `audio/`, `pipeline/`, `registry.rs` auf `Arc<Box<dyn T>>`, `Arc::new(Arc::clone(...))`, `Arc<Arc<_>>`. **Kein Fund** — alle Arc-Verwendungen in klarvo-core sind korrekt und nicht redundant.

**REQUIRED_KEYS-Fix (AC-2):**
`en.json` enthielt seit A4-Arbeit 3 Keys ohne REQUIRED_KEYS-Eintrag: `error.config.parse_failed`, `error.settings.in_memory_fallback`, `error.settings.validation`. Emit-Sites existieren in `main.rs`. Fix: Keys in `REQUIRED_KEYS` aufgenommen.

### File List

- `klarvo-shell-orchestrator/src/session.rs` — `#[derive(Clone)]` auf `SessionOrchestrator`
- `shells/windows/src-tauri/src/commands/settings.rs` — `use Arc` entfernt, 8 `State<Arc<Settings>>` → `State<Settings>`
- `shells/windows/src-tauri/src/main.rs` — Settings + Orch ohne doppeltes Arc, `app.manage(orch/settings)` direkt
- `shells/windows/src-tauri/src/hotkey.rs` — State-Zugriff + Test auf `SessionOrchestrator` (ohne Arc)
- `shells/windows/src-tauri/src/i18n.rs` — 3 REQUIRED_KEYS nachgepflegt

### Change Log

- 2026-04-29: Arc-Duplikat-Fix für Settings + SessionOrchestrator; klarvo-core-Scan: kein Fund; i18n REQUIRED_KEYS-Patch (A4-Nachpflege)

## Technical Notes

- Wenn `Settings` kein `Send + Sync + 'static` erfüllt: Prüfen ob `Settings` bereits intern
  `Arc<Mutex<SettingsInner>>` hat (ja — dann ist `Settings: Clone + Send + Sync` trivial).
- Falls `SessionOrchestrator` nicht direkt managebar ist (wegen `app.manage` + Hotkey-Callback-Clone):
  Hotkey-Callback auf `app.handle().state::<SessionOrchestrator>()` umstellen statt `Arc::clone`.
- Scope-Discrepancy-Note: Dispatch-Plan nennt "klarvo-core Audio-Pipeline-Pfad" als Touch-Boundary;
  primäre Stelle ist Shell-Code. **klarvo-core-Scan ist Teil dieser Story:** Agent soll
  `klarvo-core/src/` (insb. `audio/`, `pipeline/`, `registry.rs`) gezielt nach redundantem
  `Arc`-Wrapping suchen (z. B. `Arc<Box<dyn T>>` wo `Arc<dyn T>` reicht, oder `Arc::new(Arc::clone(...))`-Pattern).
  Jeder Fund → eigener Commit; kein Fund → Note im PR-Body "klarvo-core scan: no redundant Arc found".
