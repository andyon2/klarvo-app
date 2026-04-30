---
name: Story 2.A.D3 — Graceful-Shutdown pipeline_task
phase: 2
wave: A
story_id: "2.A.D3"
status: ready-for-dev
dependencies: []
adr_refs:
  - docs/adr/0008-shell-adapter-interface-shape.md
source_ref: "deferred-work.md F4 / Epic-3-Code-Review; memory/project_shell_session_lifecycle"
---

# Story 2.A.D3: Graceful-Shutdown `pipeline_task.abort()`

## Outcome

Session-Teardown bei App-Exit ist deterministisch. Aktuell: `drop(pipeline_task)` in `on_release`
detacht den Task ohne ihn zu stoppen — laufende Pipeline läuft nach Release weiter (gewollt für
normale Hotkey-Release-Semantik). Problem: bei App-Exit während laufender Pipeline bleibt der
Task orphaned ohne sauberes Teardown.

Nach dem Fix: `SessionOrchestrator` erhält eine `shutdown()`-Methode, die `pipeline_task.abort()`
aufruft wenn eine Session aktiv ist. Die Tauri-App ruft `shutdown()` im `on_window_event(CloseRequested)`-
oder `on_exit`-Hook auf.

3-State-Lifecycle (`Idle / Recording / Completed`) aus Story-3.3/3.5 bleibt erhalten.
Kein Panic-on-Drop bei laufender Pipeline. `on_release`-Semantik unverändert.

## Scope-Fence

**In-Scope:**
- `klarvo-shell-orchestrator/src/session.rs` — `shutdown()`-Methode + `pipeline_task.abort()`
- `shells/windows/src-tauri/src/main.rs` — `shutdown()`-Aufruf in App-Exit-Hook

**Nicht-in-Scope:**
- `on_release`-Semantik ändern (hotkey-release soll Pipeline NICHT abbrechen)
- Toggle-Mode für Multi-Utterance (Phase-2+)
- Graceful-await auf Pipeline-Completion bei Exit (abort ist das Ziel, kein `join().await`)

## Acceptance Criteria

### AC-1 — `SessionOrchestrator::shutdown()` existiert und ist idempotent

**Given** `klarvo-shell-orchestrator` kompiliert  
**When** `shutdown()` aufgerufen wird  
**Then**
- Methode existiert: `pub async fn shutdown(&self)`.
- Ist idempotent: mehrfaches Aufrufen erzeugt kein Panic / kein doppeltes Abort.
- Im `SessionState::Recording`-Zustand: `pipeline_task.abort()` wird aufgerufen.
- Im `SessionState::Idle`-Zustand: No-op.
- `event_bus.emit(Event::RecordingStopped { ... })` wird NICHT emittiert (kein Recording-Event
  bei forciertem Shutdown — unterscheidet sich von `on_release`).

---

### AC-2 — App-Exit ruft `shutdown()` auf

**Given** `shells/windows/src-tauri/src/main.rs`  
**When** Tauri App-Exit-Event ausgelöst wird (z.B. `on_exit` oder `CloseRequested` mit Accept)  
**Then**
- `orchestrator.shutdown().await` (oder synchrones Äquivalent) wird aufgerufen bevor der
  Tauri-Prozess terminiert.
- Kein Panic, kein Deadlock beim Exit.

---

### AC-3 — `on_release`-Semantik unverändert

**Given** normale Hotkey-Release während laufender Pipeline  
**When** `on_release()` aufgerufen wird  
**Then**
- Pipeline-Task läuft weiter (wie bisher: `drop(capture_handle)` → Channel closed → Pipeline liefert).
- `on_release` ruft NICHT `pipeline_task.abort()` auf.
- Bestehende Tests für `on_release` bleiben grün.

---

### AC-4 — Kein Panic-on-Drop

**Given** `SessionOrchestrator` wird gedroppt während `pipeline_task` läuft  
**When** Drop ausgeführt wird  
**Then**
- Kein Panic, kein UB.
- Task wird durch Drop implizit detacht (Tokio-Default) — kein Abort im Drop-Impl erforderlich
  (Abort geschieht explizit via `shutdown()`; Drop-Impl bleibt default).

---

## Technical Notes

- `pipeline_task.abort()` sendet `CancellationToken`-äquivalentes Signal zu Tokio; der Task
  wird beim nächsten `.await`-Point gecancelt. Keine Garantie sofortiger Terminierung.
- `RecordingCompleted`-Guard aus Story-3.5: `pipeline_task` ist `JoinHandle<()>`. Nach `abort()`,
  wenn der Task `RecordingCompleted`-Event vor Cancellation noch emittiert hat, ist das korrekt.
  Wenn nicht, ist das auch akzeptabel (Forced-Exit-Pfad).
- Windows-Tray-App: App-Exit kann via Tray-"Exit"-Button oder `Ctrl+C` ausgelöst werden.
  Beide Pfade müssen `shutdown()` triggern.
