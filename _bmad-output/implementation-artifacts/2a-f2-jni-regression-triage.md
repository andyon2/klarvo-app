---
name: Story 2.A.F2 — JNI Rate-Test Regression Triage
phase: 2
wave: A
story_id: "2.A.F2"
status: ready-for-dev
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
