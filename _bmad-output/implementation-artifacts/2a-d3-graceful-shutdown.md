---
name: Story 2.A.D3 — Graceful-Shutdown pipeline_task
phase: 2
wave: A
story_id: "2.A.D3"
status: done
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
- Windows-Tray-App: App-Exit über Tray-"Exit"-Button löst `RunEvent::Exit` aus → `shutdown()`.
  Forcierte Beendigung (Task-Manager-Kill, `taskkill /F`, OS-Shutdown-Force) hat keinen
  Graceful-Pfad — out-of-scope. `Ctrl+C` ist Dev-only (nur bei attached Console, z. B.
  `cargo run` im Terminal) und kein produktiver User-Pfad — out-of-scope für diese Story;
  eine optionale Dev-Determinismus-Folge-Story (`tokio::signal::ctrl_c()` → `app_handle.exit(0)`)
  ist denkbar.
  *Amendment 2026-05-01 (D1-Code-Review-Resolution): ursprüngliche Formulierung*
  *„Tray-'Exit'-Button oder Ctrl+C — beide Pfade müssen `shutdown()` triggern" war über-formuliert*
  *für ein Tray-App ohne Console im Production-Launch.*

---

## Tasks

- [x] `SessionOrchestrator::shutdown()` in `session.rs` implementieren (AC-1, AC-4)
- [x] App-Exit-Hook in `main.rs` auf `RunEvent::Exit` + `block_on(shutdown())` umstellen (AC-2)
- [x] Tests: shutdown_while_idle, shutdown_while_recording, shutdown_is_idempotent, on_release regression (AC-1, AC-3)
- [x] Alle Tests grün (14 Session-Tests, 5 E2E-Tests)

## Dev Agent Record

### Completion Notes

- `pub async fn shutdown(&self)` in `session.rs` hinzugefügt: `mem::replace → Idle`, dann `pipeline_task.abort()` + `drop(capture_handle)` wenn Recording.
- `on_release` bleibt unverändert — kein abort(), kein Event-Unterschied.
- `main.rs`: `.run(context)` → `.build(context).run(handler)` mit `tauri::RunEvent::Exit`-Branch; `block_on(orch.shutdown())` deckt Tray-Quit (kanonischer User-Exit-Pfad). Ctrl+C / forcierte Beendigung sind out-of-scope (siehe Technical-Notes Amendment 2026-05-01).
- 4 neue Tests; alle 14 session_tests + 5 e2e_tests grün. Zero Regressions.
- Windows-Target nicht cross-compilierbar auf Linux; `main.rs`-Syntax via Lesekontrolle verifiziert (Tauri v2 standard API).

### File List

- `klarvo-shell-orchestrator/src/session.rs` — `shutdown()` hinzugefügt
- `klarvo-shell-orchestrator/tests/session_tests.rs` — 4 neue Tests
- `shells/windows/src-tauri/src/main.rs` — `.build().run(handler)` mit Exit-Hook

### Change Log

- 2026-05-01: Story 2.A.D3 implementiert — `SessionOrchestrator::shutdown()` + App-Exit-Hook + Tests
- 2026-05-01: Code-Review (3 Layer: Blind / Edge-Case / Acceptance-Auditor) — 17 unique Findings nach Dedup (1 decision-needed, 8 patch, 4 defer, 5 dismissed)
- 2026-05-01: Code-Review-Closure — D1 → Spec-Amendment (Tray-Exit-only); 8 Patches applied (P1 await-after-abort, P2 try_state, P3 Completed-not-emitted-assertion, P4+`shutdown_after_on_release_is_safe_noop`, P5 drain_events helper, P6 AutoStop/Toggle/sequential Tests, P7 shutdown_with_timeout helper, P8 Docstring-thread-safety); 4 Defers in `deferred-work.md` (D3-W1..W4); 22/22 Tests grün; status → done.

---

## Review Findings

> Quelle: `bmad-code-review` (3 parallele Layer), 2026-05-01.
> Coverage: AC-1..AC-4 alle satisfied (Auditor); Findings betreffen Härtung, Determinismus und Test-Robustheit.

### Decision-needed (resolved 2026-05-01)

- [x] **D1 → Spec-Amendment** (Option 2): Constraint Technical-Notes Zeile 96-97 war über-formuliert für ein Windows-Tray-App ohne Console im Production-Launch (Start-Menü/Desktop/Autostart). Ctrl+C ist Dev-only-Pfad (nur bei attached Console) und keine User-Surface. Forcierte Beendigung (Task-Manager-Kill, `taskkill /F`, OS-Shutdown-Force) hat strukturell keinen Graceful-Pfad — out-of-scope. **Tray-Exit ist die einzige reale graceful User-Exit-Surface** und ist über `RunEvent::Exit` → `shutdown()` abgedeckt. Spec-Amendment + Completion-Note-Korrektur applied; optionaler Dev-Determinismus (`tokio::signal::ctrl_c()` → `app_handle.exit(0)`) bleibt als denkbare Folge-Story (Out-of-Scope hier).

### Patch (all applied 2026-05-01)

- [x] [Review][Patch] **P1 — `pipeline_task.abort()` ohne Await ist nicht-deterministisch** [`klarvo-shell-orchestrator/src/session.rs:314`] (HIGH, blind+edge) — Fix: `let _ = pipeline_task.await;` nach `abort()` ergänzt; Determinismus aus Outcome-Zeile 17 jetzt code-seitig erfüllt.
- [x] [Review][Patch] **P2 — `app_handle.state::<SessionOrchestrator>()` panict bei Setup-Fail** [`shells/windows/src-tauri/src/main.rs:489`] (MEDIUM, blind+edge) — Fix: auf `try_state` umgestellt mit `if let Some(orch)`-Guard; Boot-Fail-Pfad wird nicht mehr durch Exit-Handler-Panic überschrieben.
- [x] [Review][Patch] **P3 — Test fehlt `RecordingCompleted`-not-emitted-Assertion** [`klarvo-shell-orchestrator/tests/session_tests.rs`] (MEDIUM, edge) — Fix: `assert!(!events.iter().any(|e| matches!(e, Event::RecordingCompleted { .. })))` in `shutdown_while_recording_aborts_and_no_stopped_event` ergänzt.
- [x] [Review][Patch] **P4 — Test-Name `on_release_semantics_unchanged_after_shutdown_impl` ist irreführend** [`klarvo-shell-orchestrator/tests/session_tests.rs`] (MEDIUM, blind+edge+auditor) — Fix: Body um Phase-2 erweitert (`shutdown()` → `on_release()` Stray-Path); plus separater Test `shutdown_after_on_release_is_safe_noop` für die Detached-Pipeline-Sequenz.
- [x] [Review][Patch] **P5 — Sleep-basiertes Event-Drain ist flake-anfällig** [`klarvo-shell-orchestrator/tests/session_tests.rs`] (MEDIUM, blind+auditor) — Fix: Helper `drain_events()` mit `tokio::time::timeout(50ms, rx.recv())`-Polling-Loop ersetzt fixed sleep + try_recv.
- [x] [Review][Patch] **P6 — Coverage-Lücke: AutoStop / Toggle press_mode + `on_release → shutdown` Sequenz** [`klarvo-shell-orchestrator/tests/session_tests.rs`] (MEDIUM, blind+edge) — Fix: 3 neue Tests (`shutdown_while_recording_autostop_does_not_deadlock`, `shutdown_while_recording_toggle_does_not_deadlock`, `shutdown_after_on_release_is_safe_noop`) — alle grün, Lock-Pfad-Deadlock-Risiko (D3-W3) wird vom 1s-Timeout-Wrapper enforced.
- [x] [Review][Patch] **P7 — `shutdown().await` ohne Timeout-Wrapper in Tests** [`klarvo-shell-orchestrator/tests/session_tests.rs`] (LOW, blind) — Fix: Helper `shutdown_with_timeout()` mit `tokio::time::timeout(1s, ...)` eingeführt; alle Shutdown-Tests verwenden ihn — Hangs surfacen jetzt als deterministische Test-Failure.
- [x] [Review][Patch] **P8 — `shutdown()`-Docstring claimt „Idempotent" ohne Thread-Safety-Klärung** [`klarvo-shell-orchestrator/src/session.rs:304-313`] (LOW, blind) — Fix: Docstring erweitert um Outcome-Determinismus-Klausel + „Idempotent and concurrent-safe: callers serialize via the `session_state` mutex; the second caller observes `Idle` and no-ops".

### Patch-Verification

- `cargo test -p klarvo-shell-orchestrator`: **22/22 grün** (17 session_tests + 5 e2e_tests + 0 lib + 0 doc); zero regressions.
- Test-Count nach Patch: 17 session_tests (= 13 Bestand + 4 von Story-Drop + neue: `shutdown_after_on_release_is_safe_noop`, `shutdown_while_recording_autostop_does_not_deadlock`, `shutdown_while_recording_toggle_does_not_deadlock`).
- Windows-Target nicht cross-compilierbar auf Linux; main.rs `try_state`-Patch via Lesekontrolle verifiziert (Tauri v2 standard API; `try_state` ist Standard-Method-Pendant zu `state`).

### Defer

- [x] [Review][Defer] **D3-W1 — `block_on` inside `RunEvent::Exit` runtime-deadlock concern** [`shells/windows/src-tauri/src/main.rs:490-492`] — `tauri::async_runtime::block_on` aus dem Tauri-Exit-Callback ist fragil, falls `shutdown()` etwas auf demselben Multi-Threaded-Runtime awaited, das Locks hält die für Drain nötig sind. Tauri's dokumentiertes Pattern nutzt aber genau `block_on` an dieser Stelle (Main-Thread, kein Worker). Reale Sorge, aber framework-pattern-konform. **Defer-Reason:** Framework-Pattern; revisit nur wenn Exit-Hangs in der Praxis beobachtet.
- [x] [Review][Defer] **D3-W2 — State-Race-Window zwischen `drop(state)` und `pipeline_task.abort()`** [`klarvo-shell-orchestrator/src/session.rs:312-315`] — Nach `drop(state)` könnte ein nebenläufiges `on_press()` den Lock greifen und ein neues Recording starten, **bevor** `shutdown()` den vorigen (bereits per `mem::replace` herausgenommenen) Task aborted. Neue Pipeline leakt past Shutdown. Dasselbe Pattern existiert in `on_release` (session.rs:352) und wurde dort bewusst gewählt für Lock-Ordering. Bei App-Exit sind Hotkey-Handler typisch schon abgemeldet → race ist theoretisch und bounded by Process-Death. **Defer-Reason:** Pre-existing pattern (deckungsgleich mit `on_release`); race bounded by app-exit; keine Regression.
- [x] [Review][Defer] **D3-W3 — AutoStop-Self-Cleanup × `shutdown()` Lock-Contention** [`klarvo-shell-orchestrator/src/session.rs:248 vs :309`] — AutoStop-Pipeline acquired `session_state`-Lock intern für Cleanup (Zeile 248). Wenn `shutdown()` den Lock zuerst hält und dann released → AutoStop-Lock-Wait queued → `abort()` löst AutoStop via JoinError. Andere Richtung: AutoStop hält Lock → `shutdown()` queued → AutoStop completed Idle → `shutdown()` sieht Idle → no-op. Flow-Analyse legt nahe: beide Pfade sicher. **Defer-Reason:** Verlangt deeper Concurrency-Spike (auch im Hinblick auf zukünftige Pipeline-await-under-lock-Erweiterungen); out-of-scope für 2.A.D3.
- [x] [Review][Defer] **D3-W4 — `.expect("error building tauri application")` swallowt Builder-Error-Kontext** [`shells/windows/src-tauri/src/main.rs:486`] — `.run().expect(...)` → `.build().expect(...).run(...)` führt zwei Panic-Sites statt einer ein. Kein structured logging, kein `app.error`-Toast (pre-build eh nicht erreichbar). Pre-existing Style; `.expect()` war an dieser Stelle bereits vorher. **Defer-Reason:** Pre-existing style; keine Regression durch diese Story.

### Dismissed (5 — Begründung im Triage)

- `drop(capture_handle)` detached-but-not-stopped (Blind#1) — false positive: `CaptureHandle` hat Drop-Impl, das den broadcast sender schließt (verifiziert via session.rs:357-358 Inline-Doc).
- `abort()` vor `drop(capture_handle)` Ordering (Blind#12) — abort ist non-blocking Signal; Order ist semantisch egal.
- Tray-Icon bleibt rot post-Shutdown (Edge#8) — Prozess terminiert, UI-State irrelevant.
- EventMirror-Flush bei Exit (Edge#17) — Process-Death suffices; verlorene Events sind by-design auf Forced-Exit-Pfad.
- Drop-Impl für `SessionOrchestrator` (Edge#14) — AC-4 mandatet explizit „Drop-Impl bleibt default".
