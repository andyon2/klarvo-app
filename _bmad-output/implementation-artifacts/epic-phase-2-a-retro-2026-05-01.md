---
title: "Epic Phase-2-A — Retrospective"
date: 2026-05-01
epic: phase-2-a
epic_status: done
participant: Andy (Project Lead, solo)
facilitation: BMad Retrospective Workflow (party-mode condensed)
inputs:
  - _bmad-output/planning-artifacts/epics/epic-phase-2-a.md
  - _bmad-output/planning-artifacts/_archive/phase-2-scope-lock.md
  - _bmad-output/implementation-artifacts/2a-*.md (9 Story-Files)
  - _bmad-output/implementation-artifacts/deferred-work.md
  - _bmad-output/implementation-artifacts/epic-3-code-review-2026-04-25.md
  - _bmad-output/implementation-artifacts/epic-4-code-review-2026-04-25.md
  - _bmad-output/implementation-artifacts/epic-5-code-review-2026-04-26.md
---

# Epic Phase-2-A — Retrospective

## Delivery-Bilanz

- **9 von 10 Stories `done`** (C1 Signed MSI bewusst → Phase-2-B/3, extern blockiert auf Cert-Beschaffung)
- F2 (JNI-Triage) Outcome = **Fix** (nicht Defer): Root-Cause Test-Isolation-Race im static `LISTENER`, kein Bridge-Bug. ADR-0003 `Proposed`→`Accepted` mit Amendment 2; `--exclude klarvo-bridge-jni` aus E1 entfernt → Phase-3-De-Risking strukturell entschärft.
- Welle-2-Topologie (8 echte Eng-Streams) ohne Merge-Konflikte abgewickelt.
- Realtime von Scope-Lock (2026-04-26) bis Phase-2-A-Closure (2026-05-01) **deutlich unter** Scope-Lock-Schätzung.

## Was gut lief

1. **A4 als Foundation hat getragen.** ADR-0013 mit 5 Sub-Decisions als Pre-Story-Decision war die richtige Investition — A8-Sub/C2/C3 konsumierten Settings-Service + 8 Tauri-Commands ohne Re-Design.
2. **2-Pass-Code-Review-Pattern auf A4** lieferte echten Wert: Pass-1 = 15 Patches, Pass-2 = 16 weitere Patches auf uncommitted-Diff, 4 Folge-Stories sauber ge-scopt statt ein-gepatcht. Story blieb `done`. Pattern-Kandidat für Foundation-Stories generell.
3. **D2 als Single-Refactor-Story sauber:** 0 Patches, 0 Decisions, 10 Defers (alle pre-existing oder out-of-scope). Beweis dass scharf gefencete Stories ohne Patch-Welle landen können.
4. **F2-Outcome = Fix.** Phase-3-De-Risking strukturell entschärft (statt Triage-Report mit Defer).

## Reibungsstellen (Pattern in ≥2 Stories)

1. **Cross-Story-Bleed bei Closure (2x):** A8-Sub-Working-Tree enthielt 2.B.A1-Closure-Code (~330 LOC); C3-Working-Tree enthielt D3-Code (`RunEvent::Exit`-Hook). Beide via Commit-Split resolved (`4f0e0f7`, `7803eda`, D3-bleed-Decision). Verstößt gegen `feedback_commit_hygiene`. Pattern: Welle-2-Parallel-Dispatch teilte einen Working-Tree.

2. **Fail-Soft-Pattern wiederholt nachgepatcht (4 Stories):** A4-P2 (`expect("settings mutex poisoned")` → `lock_conn()`-Helper), A4-P3 (`expect("in-memory infallible")` → Two-Step-Fallback), C3-P1/P2 (RwLock-`unwrap()` → `unwrap_or_else(|e| e.into_inner())`), F2-P1 (Test-Mutex-Poisoning). `feedback_scaffold_fail_soft_pattern` (2026-04-19) wird beim Schreiben nicht konsultiert.

3. **REQUIRED_KEYS-Drift:** A4-P2-P11 fügte `error.unknown` in JSON, vergaß Konstante in `i18n.rs::REQUIRED_KEYS` → Test rot bis A8-Sub das nachzog. D2 musste 3 weitere fehlende Keys aus A4 nachpflegen. **G3-Lint catched anderes — Drift-Mechanismus ist nicht xtask-abgedeckt.**

4. **Patch-Volumen A4-dominiert (44%):** 31 von ~70 Phase-2-A-Patches in A4. Foundation-Bias erkennbar — Implementations-Drift in Foundation-Layer war am höchsten. Pattern-konform mit Phase-1 Epic-3, also kein neues Phänomen.

## Architektur-Findings

- **Keine neuen ADRs in Phase-2-A**, nur Amendments. Foundation-Surface stabil.
- ADR-0013 Amendment 2: Event-Name Dot-Notation (`settings.changed` statt Kebab) per `reference_tauri_specta_rc24_event_name`.
- ADR-0003 Amendment 2: Test-Isolation-Race-Root-Cause + absolute Messwerte (200 events / 10 s, 0 drops; 10 events / 500 ms smoke).
- **ADR-0013 §181 widerspricht C2-Impl** (Pre-Validation-Modell statt Rollback) — Amendment offen, deferred als C2-W1 → AI-4.
- **3-Achsen-i18n** (`memory/project_i18n_three_axes`) hielt strukturell — C3 brauchte nur den `ui.language`-Pfad.

## Velocity vs Scope-Lock-Schätzung

Scope-Lock 2026-04-26 sagte "Phase-2-A: 3–5 Wochen". Tatsächlich Closure 2026-05-01 (~5 Tage Realtime). Überproportional aufwendig: **A4** (Foundation, 31 Patches über 2-Pass-Review) und **C2** (Win32-Async-Komplexität: 12 Patches inkl. Skip-if-equal-Fast-Path + RAII-Probe-Guard + AtomicI32-ID + `Shortcut::from_str`-Grammar-Gate). Schlank: D2, E1, F2.

## Process-Findings (BMad-spezifisch)

- **Pre-Story-Decision-Pattern (ADR-0013 vor A4) funktionierte.** Reproduzieren für Phase-2-B (Pill-Bar UX-Mini-Pass + 2nd-STT-Choice).
- **2-Pass-Code-Review nur auf A4** — One-Off. Kandidat für Foundation-Stories generell, nicht Default für alle.
- **Welle-2-Parallel-Dispatch (8 Eng-Streams):** Funktional ohne Merge-Konflikte, aber zwei Cross-Story-Bleeds (siehe Reibungsstelle 1). `feedback_commit_hygiene` reicht als Schutz nicht aus, wenn parallele Stories denselben Working-Tree-Status teilen → AI-1.
- **Cross-Cutting-Defer-Konsolidierung gut:** E1's 4 Decisions zur "CI-Hardening-Folge-Story" gebündelt statt einzeln in E1 gepatcht. Beispiel-Pattern für Phase-2-B: Bündel-Stories für gleichartige Cross-Cutting-Sweeps statt jede Defer einzeln scopen.

## Action-Items

### Process-Härtung

| AI | Maßnahme | Owner | Trigger |
|----|----------|-------|---------|
| **AI-1** | `feedback_commit_hygiene` erweitern um expliziten Pre-Closure-`git diff`-Audit-Schritt im Code-Review-Workflow | Andy (Memory-Update) | Sofort |
| **AI-2** | Phase-2-B-Bündel-Story: Lint-Gate `clippy::disallowed_methods` für `expect`/`unwrap` in `klarvo-core`/`klarvo-windows-shell`/`klarvo-orchestrator`; Test-Module via `#[allow]` | Phase-2-B-Hardening-Story | Vor 2.B.A2-Start |
| **AI-3** | xtask: REQUIRED_KEYS-Drift-Detection (parse JSON-Locale-Files, diff gegen `i18n.rs::REQUIRED_KEYS`) | Phase-2-B-Hardening-Story (zusammen mit AI-2 oder eigenständig) | Vor 2.B.A2-Start |

### Architektur-Closure

| AI | Maßnahme | Owner | Trigger |
|----|----------|-------|---------|
| **AI-4** | ADR-0013 §181 Amendment: Pre-Validation-Modell dokumentieren statt "rollt Settings-Mutation zurück" — C2-W1-Resolution | Andy (ADR-Amendment) | Vor 2.B.A2 |

### Pre-Story-Decisions Phase-2-B (in unmittelbarer Folge)

| AI | Maßnahme | Blockt |
|----|----------|--------|
| **AI-5** | Pill-Bar UX-Mini-Pass: 4 Decision-Points (Shape, Drag, Waveform-Render-Owner, Auto-Hide) | 2.B.A3 |
| **AI-6** | Whisper-Local-ADR-Stub: Plugin-Crate-Name + Cloud-vs-Local-Differenzierung-Begründung | 2.B.B1 |

## Carry-Overs in Phase-2-B (Index, authoritative in `deferred-work.md`)

- **4 A4-Folge-Stories:** P2-P1 Form-UX-Redesign, P2-P8 `user_snapshot`-Architectural-Extension, P2-P12 Corrupt-DB-Recovery, P2-P21 Locale-Allow-List-Two-System-Change.
- **CI-Hardening-Bündel-Story** (E1-D1 Toolchain-Pin, D2 `--locked`, D3 Lint-Gate, D5 timeout-minutes) für alle 4 Workflows.
- **5 Re-Findings auf 2.B.A1-Closure:** Triple-Tap-VAD-Lock-Race (größtes; Audit-Window in 2.B.A3 nötig), Test-Coverage-Lücken, Hard-Cap-Toast-vor-Cleanup, BYOK-Cost-Transparency, Default-Const-Typo.
- **C1 (Signed MSI):** extern, Cert-Beschaffung.

## Readiness-Assessment Phase-2-A → Phase-2-B

- **Testing/Quality:** Story-Level-Reviews durch; F2-Closure entfernt `--exclude klarvo-bridge-jni` aus E1 → Windows-Compile-CI deckt jetzt JNI-Crate mit ab. Keine Test-Gates rot.
- **Codebase-Health:** Foundation-Surface (A4) stabil — A8-Sub/C2/C3 konsumierten ohne Re-Design.
- **Stakeholder-Acceptance:** Solo, kein externer Tester aktiv (`memory/project_ea_withdrawn`). C1 + F1 sind exakt die zwei externen Beteiligten — beide nicht engineering-blockierend.
- **Unresolved Blockers:** Keine. AI-4 ist Doku-Drift, kein Code-Blocker.
- **Significant Discoveries (Epic-Update Phase-2-B Required?):** **Nein.** Phase-2-B-Epic-Plan steht. Caveat: A1-Re-F1 (Triple-Tap-VAD-Lock-Contention) erfordert Audit-Window in 2.B.A3-Story-Writing.

## Erste Retrospektive — Hinweis

Dies ist die erste BMad-Retrospektive für Klarvo v2. Phase-1-Epics (1A/1B/1C/2/3/4/5) waren als `optional` markiert und wurden nicht retrospekted. Damit etabliert dieses Dokument die Baseline für Phase-2-B-Follow-Up-Retrospektive.

## Next Steps

1. **Sofort (in dieser Session):** AI-5 + AI-6 als Pre-Flight-Decisions adressieren.
2. **Sofort (Memory):** AI-1 ausführen (`feedback_commit_hygiene`-Update).
3. **Vor Phase-2-B-Story-Writing:** AI-4 (ADR-0013 §181-Amendment) ausführen.
4. **Als erste Phase-2-B-Story:** AI-2 + AI-3 als Bündel-Story scopen und implementieren — vor 2.B.A2.
