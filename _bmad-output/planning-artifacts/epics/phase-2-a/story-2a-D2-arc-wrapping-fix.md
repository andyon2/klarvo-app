---
name: Story 2.A.D2 — Arc-Wrapping-Duplikat-Fix
phase: 2
wave: A
story_id: "2.A.D2"
status: ready
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
