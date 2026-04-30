---
name: Story 2.A.F2 — JNI Rate-Test Regression Triage
phase: 2
wave: A
story_id: "2.A.F2"
status: review
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
