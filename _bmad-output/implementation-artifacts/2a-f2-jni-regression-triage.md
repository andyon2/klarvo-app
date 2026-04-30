---
name: Story 2.A.F2 — JNI Rate-Test Regression Triage
phase: 2
wave: A
story_id: "2.A.F2"
status: done
dependencies: []
adr_refs:
  - docs/adr/0003-jni-spike-outcome.md
source_ref: "memory/project_jni_spike_scope (2026-04-20-Regression); memory/feedback_spike_rigor"
---

# Story 2.A.F2: JNI Rate-Test Regression Triage

## Outcome

Root-Cause der 20-Hz-Regression in `klarvo-bridge-jni` ist bekannt und dokumentiert.
Der Spike (ADR-0003) lieferte 200/200 Events bei 20 Hz. Nach 2026-04-20 ist der
`twenty_hz_over_ten_seconds_no_drops`-Test auf HEAD nicht reproduzierbar.

Zwei akzeptable Outcomes:

**Fix-Outcome:** Rate-Test grün + ADR-0003-Amendment mit Root-Cause und Fix-Beschreibung.
E1's `--exclude klarvo-bridge-jni`-Flag kann danach entfernt werden (in F2-Closure-Commit).

**Defer-Outcome:** `docs/jni-regression-triage.md` mit Root-Cause + Defer-Begründung +
Backlog-Phase-3-Update. `--exclude`-Flag in E1 bleibt dokumentiert. Keine weiteren Phase-2-A-Aufgaben.

## Scope-Fence

**In-Scope:**
- Reproduktion auf aktuellem HEAD
- Git-Bisect zwischen Spike-Baseline-Commit und HEAD
- Fix ODER Triage-Report (beide sind vollständige Outcomes)

**Nicht-in-Scope:**
- JNI-Bridge-Rewrite (Phase-3)
- Kotlin-callbackFlow-Adapter (Phase-3)
- Android-NDK-Setup oder -Compile (Phase-3)

## Acceptance Criteria

### AC-1 — Reproduktion auf aktuellem HEAD

**Given** `klarvo-bridge-jni`-Crate, Tester hat JDK auf PATH (OpenJDK 17+)  
**When** `cargo test -p klarvo-bridge-jni` ausgeführt wird  
**Then**
- Ergebnis dokumentiert: Test fail / Test pass / Test panic / anderer Fehler.
- Falls nicht reproduzierbar (Test grün auf HEAD): AC-2 skippen; ADR-0003-Amendment mit
  "Regression spontan verschwunden + Commit-SHA" + Story als done-move.
- Falls reproduzierbar: weiter mit AC-2.

---

### AC-2 — Bisect-Root-Cause

**Given** reproduzierbarer Fehler auf HEAD  
**When** `git bisect` zwischen Spike-Baseline-Commit und HEAD läuft  
**Then**
- Breaking-Commit identifiziert (SHA + Commit-Message).
- Root-Cause beschrieben: Was hat sich geändert? (z.B. Dependency-Version-Bump,
  Code-Change in Bridge, Cargo-Feature-Change, Timing-Sensitivität).
- Root-Cause-Kategorie bestimmt: Fix möglich (→ AC-3) ODER Defer (→ AC-4).

---

### AC-3 — Fix-Outcome (wenn möglich)

**Given** Root-Cause bekannt und fix-bar  
**When** Fix implementiert und committed  
**Then**
- `cargo test -p klarvo-bridge-jni` grün auf HEAD.
- Test `twenty_hz_over_ten_seconds_no_drops` liefert ≥ 190 Events in 10s (±5% Toleranz per ADR-0003).
- ADR-0003 Amendment-2 anhängen: Root-Cause + Fix-Summary + neue Messwerte (absolut: N events in T s).
- Separater Commit für E1-`--exclude`-Flag-Entfernung (kein Dual-Action-Race laut Dispatch-Plan).

---

### AC-4 — Defer-Outcome (wenn Fix nicht Phase-2-A-angemessen)

**Given** Root-Cause bekannt, aber Fix zu aufwändig für Phase-2-A (z.B. JNI-Rewrite erforderlich)  
**When** Triage-Report committed  
**Then**
- `docs/jni-regression-triage.md` erstellt mit:
  - Breaking-Commit SHA + Beschreibung
  - Root-Cause-Erklärung
  - Warum Defer (klarer Satz)
  - Backlog-Phase-3-Eintrag-Referenz
- ADR-0003 Amendment-2 anhängen: Root-Cause + Defer-Entscheidung.
- `docs/backlog.md` Eintrag: `"JNI-Bridge-Fix"` mit Phase-3-Status + Source-Ref zu Triage-Report.
- `--exclude klarvo-bridge-jni` in E1 bleibt; F2-Closure-Commit enthält keine E1-Änderung.

---

## Technical Notes

- Spike-Baseline: Commit ed14014-Sukzessor (laut ADR-0003); `git log --oneline` auf
  `klarvo-bridge-jni`-Touches hilft Bisect-Range einzugrenzen.
- Setup für Test: JDK auf PATH, `javac` verfügbar. Test kompiliert `TestListener.java`
  in Tempdir + spawnt JVM intern. Kein Android-Emulator erforderlich.
- Per `memory/feedback_spike_rigor`: Messwerte sind absolut zu dokumentieren (N events in T s),
  nicht nur binary "grün/rot".
- JNI exclusion in `--exclude klarvo-bridge-jni` ist in `cargo test --workspace` nötig,
  da JNI-Tests JDK voraussetzen (nicht überall verfügbar). Fix-Outcome entfernt das Flag nur
  im E1-Workflow (CI), nicht zwingend im lokalen `cargo test --workspace`.

## Tasks/Subtasks

- [x] T1: AC-1 — Reproduktion auf HEAD dokumentieren
- [x] T2: AC-2 — Root-Cause-Analyse (Bisect + Shared-Static-Race identifizieren)
- [x] T3: AC-3 — Fix: TEST_MUTEX in audio_level_callback.rs + `cargo test -p klarvo-bridge-jni` grün
- [x] T4: ADR-0003 Amendment-2 mit Root-Cause + Fix-Summary + Messwerten
- [x] T5: E1 windows-ci.yml `--exclude klarvo-bridge-jni` entfernen (separater Commit)

### Review Findings

- [x] [Review][Defer] **TEST_MUTEX-Pattern silently violable** — deferred — Phase-3 JNI-Rewrite redesignt die Listener-Architektur (Multi-Listener-Hardening + RAII-API); Wrapper jetzt einzubauen wäre scope-creep über die F2-Scope-Fence "Nicht-in-Scope: JNI-Bridge-Rewrite" hinaus. Mini-Comment im Test-File-Top als Defense-against-future-self ist Teil der P-Patches. Quelle: blind+edge.
- [x] [Review][Defer] **Linux-CI exkludiert `klarvo-bridge-jni` weiterhin** — deferred — F2-Scope-Fence T5 ist explizit Windows-CI-Only (`E1 windows-ci.yml ... entfernen`). Linux-Epic-5-CI-Exclude (per `project_jni_spike_scope`-Memo) zu entfernen ist eine eigene CI-Hardening-Story; F2-Closure beweist nur Windows-Compile, JNI-Tests bleiben dev-machine-verifiziert bis Phase-3. Eintrag in `deferred-work.md` als F2-W5. Quelle: blind+edge.
- [x] [Review][Defer] **Production `register_listener` silently overwrites Listener** — deferred — Story-Outcome explizit "Test-Isolation-Bug, kein Production-Code-Bug"; `tracing::warn!`-Patch wäre Production-Code-Touch außerhalb der Scope-Fence. Multi-Listener-Hardening kommt mit Phase-3-JNI-Rewrite. ADR-Amendment-Sprache wird via P3/P4 nachgeschärft. Eintrag in `deferred-work.md` als F2-W6. Quelle: edge.
- [x] [Review][Patch] **Mutex-Poisoning maskiert echte Test-Failures** [klarvo-bridge-jni/tests/audio_level_callback.rs:94, 119] — Fixed: `.lock().unwrap_or_else(|p| p.into_inner())`. Quelle: blind+edge.
- [x] [Review][Patch] **`unregister_listener` wird auf Assert-Failure übersprungen** [klarvo-bridge-jni/tests/audio_level_callback.rs] — Fixed: `ListenerGuard`-Struct mit `Drop`-Impl in beide Tests integriert; explizite `unregister_listener()`-Aufrufe entfernt (RAII-Pattern per Memory `feedback_test_raii_cleanup_pattern`). Quelle: edge.
- [x] [Review][Patch] **ADR-Messwert-Tabelle „≥5 / ≥5" ist nicht-quantitativ** [docs/adr/0003-jni-spike-outcome.md] — Fixed: Smoke-Test-Eintrag mit `eprintln!`-Capture ergänzt, Re-Run gemessen → Smoke = 10 events / 500 ms; Tabellen-Zelle auf **10** aktualisiert. Quelle: blind.
- [x] [Review][Patch] **ADR-Widerspruch: `--test-threads=1` „bleibt gültig" vs. „nicht erforderlich"** [docs/adr/0003-jni-spike-outcome.md] — Fixed: Smoke-Test-Sektion mit `> Hinweis (post-Amendment 2)`-Box; Amendment-2-Konsequenz-Sektion eindeutig formuliert (Flag obsolet, ersatzlos entfernbar). Quelle: blind+edge.
- [x] [Review][Patch] **ADR-Header-Status-Line wurde nicht von „Proposed" auf „Accepted" geflippt** [docs/adr/0003-jni-spike-outcome.md:3] — Fixed: Header-Line `**Status:** Accepted (siehe Amendment 2 — 2026-05-01)`. Quelle: auditor.
- [x] [Review][Defer] **JDK auf windows-latest nur implicit verfügbar** [.github/workflows/windows-ci.yml:34-38] — deferred, pre-existing — kein expliziter `actions/setup-java`-Step; bei Runner-Image-Wechsel ohne JDK bricht CI silent. Aktuell kein Bug.
- [x] [Review][Defer] **`JavaVM::singleton()` + multiple integration-test binaries** [klarvo-bridge-jni/tests/audio_level_callback.rs:40-57] — deferred, pre-existing — moot bei aktuell einer Test-Datei; Edge bei Future-Refactoring.
- [x] [Review][Defer] **Background-Tasks nicht durch TEST_MUTEX serialisiert** [klarvo-bridge-jni/src/commands.rs:97-106] — deferred, pre-existing — `tokio::JoinHandle::abort()` ist non-blocking; Test-N's Bridge-Task kann noch in JNI-Call sein, wenn Test-(N+1) startet. Mutex serialisiert nur Test-Bodies, nicht Hintergrundtasks. Workaround `sleep(50ms/100ms)` ist best-effort.
- [x] [Review][Defer] **`stop_meter` abort vs. drain race** [klarvo-bridge-jni/src/commands.rs:97-106] — deferred, pre-existing — broadcast-Channel-Receiver kann nach `abort()` noch `recv().await`-Wakeup bekommen; nicht von F2 verursacht.

## Dev Agent Record

### Debug Log

**AC-1 Ergebnis (2026-05-01):** `cargo test -p klarvo-bridge-jni` → FAILED.
- `listener_receives_events_smoke`: FAILED (0 events in 500ms)
- `twenty_hz_over_ten_seconds_no_drops`: PASSED (210 events in 10s, innerhalb 190–210-Toleranz)

**Isolierter Lauf:** Beide Tests einzeln grün (smoke: OK; 20Hz: 200 events exakt).

**AC-2 Root-Cause:** Kein Breaking-Commit zwischen Spike (482c6c9) und HEAD — nur 2 Commits berühren JNI-Crate. Problem war seit Spike latent, aber nicht aufgefallen weil ADR-0003 den Spike mit `--test-threads=1` durchführte.

**Mechanismus:** Beide Tests teilen `LISTENER: Mutex<Option<Global<JObject<'static>>>>` und `RUNTIME: OnceLock<Runtime>`. Rust-Default-Testausführung ist multi-threaded. Wenn beide Tests parallel starten:
1. Test-A ruft `register_listener(L_A)` auf
2. Test-B ruft `register_listener(L_B)` auf → überschreibt `LISTENER` mit L_B
3. Beide Sessions senden Events an L_B
4. Test-A liest L_A.count → 0 (L_A hat nie Events bekommen)
5. Test-B liest L_B.count → ~210 (Events beider Sessions, liegt knapp in 190–210-Toleranz)

**Fix-Kategorie:** Test-Isolation-Bug, kein Production-Code-Bug. Fix: static `TEST_MUTEX: Mutex<()>` in Test-File serialisiert Tests die LISTENER teilen. Keine neue Crate-Dependency nötig.

### Completion Notes

Fix-Outcome gewählt. Root-Cause: Test-Isolation-Race in statischem `LISTENER`. Kein Production-Code-Bug.

- `static TEST_MUTEX: Mutex<()>` in `tests/audio_level_callback.rs` serialisiert alle Tests die `LISTENER` teilen.
- `cargo test -p klarvo-bridge-jni` ohne `--test-threads=1` läuft grün: 200 events / 10s (0 drops), smoke OK.
- ADR-0003 Amendment-2 angehängt (Root-Cause + Fix + Messwerte; Status Proposed → Accepted).
- E1 windows-ci.yml `--exclude klarvo-bridge-jni` entfernt (separater Commit 3fda8a2).

## File List

- `klarvo-bridge-jni/tests/audio_level_callback.rs` — TEST_MUTEX hinzugefügt
- `docs/adr/0003-jni-spike-outcome.md` — Amendment-2 angehängt
- `.github/workflows/windows-ci.yml` — `--exclude klarvo-bridge-jni` entfernt
- `_bmad-output/implementation-artifacts/2a-f2-jni-regression-triage.md` — Story-File (Tasks + Record + Status)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — Status-Updates

## Change Log

- 2026-05-01: Fix implementiert (cf07309) — TEST_MUTEX + ADR-0003-Amendment-2
- 2026-05-01: E1-Flag-Entfernung (3fda8a2) — windows-ci ohne --exclude
- 2026-05-01: Code-Review-Closure — 5 Patches (P1 Mutex-Poisoning-Fix, P2 RAII ListenerGuard,
  P3 ADR-Smoke-Count 10 absolute erfasst, P4 `--test-threads=1`-Widerspruch aufgelöst,
  P5 ADR-Header Proposed→Accepted), 6 Defers in `deferred-work.md` (F2-W1..W6); Status review→done.
