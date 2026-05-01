---
title: "Sprint-Change-Proposal — Phasen-zu-Epics Re-Strukturierung"
date: 2026-05-01
author: Andy + BMad-Correct-Course-Workflow
status: approved
trigger: Phase/Stream-Hybrid drift away from BMad-Standard
scope: Moderate (Backlog-Reorg, kein Replan)
inputs:
  - _bmad-output/planning-artifacts/phase-2-scope-lock.md
  - _bmad-output/planning-artifacts/epics/epic-phase-2-a.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/implementation-artifacts/sprint-status.yaml
  - _bmad-output/implementation-artifacts/epic-phase-2-a-retro-2026-05-01.md
  - docs/backlog.md
  - docs/adr/0014-whisper-local-stt.md
  - memory/feedback_status_drift_audit
  - memory/feedback_memory_path_hygiene
  - memory/project_phase2_scope_lock
---

# Sprint-Change-Proposal — Phasen-zu-Epics Re-Strukturierung

## Section 1: Issue Summary

**Was:** Phase-2-A wurde als Stream-Modell (Letter-IDs A/B/C/D/E/F + Phase-2-A/B-Buckets) organisiert, das vom BMad-Standard-Epic-Modell abweicht. Phase-2-B würde diese Drift fortschreiben — Stories sind aber thematisch geschlossen und gehören in Standard-BMad-Epic-Form. Ohne Korrektur driftet Sprint-Tracking weiter weg von BMad-Konvention.

**Wann/Wie entdeckt:** Vor Phase-2-B-Story-Writing (2026-05-01). Andy flaggte den Drift im correct-course-Trigger: "Stream-Modell war Phase-2-A-spezifisch (orthogonale Cross-Cutting-Bündel). Phase-2-B ist thematisch geschlossen — gehört in Standard-BMad-Epic-Form, bevor wir mehr Story-Files erzeugen."

**Sekundär-Befund:** Brief-Phasenplan-Vokabular (Phase 0/1/2/3/4) wurde im Sprint-Tracking (`sprint-status.yaml`-Buckets, `phase-2-scope-lock.md`) load-bearing — das kennt BMad nicht als Konstrukt. Roadmap-Funktion in BMad ist **die Epic-Liste in Reihenfolge + MVP-Boundary-Marker**, kein paralleles Phasen-Modell.

**Tertiär-Befund:** Epic 6 (Observability, FR37-FR40) und Epic 7 (V1→V2 Migration, FR41-FR43) wurden in Phase-1-Closure ausgelassen (Phase-1-FR-Scope endete bei FR35) und hängen seit 2026-04-26 ohne Phase-Zuordnung in `sprint-status.yaml`. Brauchen entweder Eröffnung oder explizite `backlog`-Markierung mit Trigger-Bedingung.

## Section 2: Impact Analysis

### Epic Impact

| Epic | Status vorher | Status nachher | Änderung |
|------|---------------|----------------|----------|
| Epic 1A/1B/1C/2/3/4 | done | done | unverändert |
| Epic 5 (Developer-Gates) | done | **in-progress (re-opened)** | 2 Hardening-Stories aus Phase-2-A-Retro AI-2/AI-3 |
| Epic 6 (Observability) | backlog (kein Phase) | backlog → **Trigger erfüllt durch Epic-10-Vorlauf** | wird vor Epic 8 dispatched |
| Epic 7 (V1→V2 Migration) | backlog (kein Phase) | backlog (Trigger: Onboarding-Flow) | bleibt deferred |
| **Epic 8 (neu): Recording Modes & Hotkeys** | — | backlog | enthält 2.B.A1 (done, Outlier) + Second-Hotkey-Slot |
| **Epic 9 (neu): UX Surface** | — | backlog | Pill-Bar / Return-Focus / History / Toasts |
| **Epic 10 (neu): Whisper-Local STT-Plugin** | — | backlog | ADR-0014 Accepted (commit 0513969); Substrate-Validation |
| epic-phase-2-a (Hybrid-Outlier) | done | done | bleibt als historisches Hybrid-Epic (path-hygiene); kein Rename |
| epic-phase-2-b (Hybrid-Bucket) | in-progress | **gelöscht aus sprint-status** | Inhalt nach Epic 8/9/10 verteilt |

### Story Impact

- **`2b-a1-toggle-autostop-wait-and-type-modes.md`** (done): Filename + Letter-ID bleiben (path-hygiene). Wird in Epic 8 als Outlier-Eintrag referenziert.
- **Backlog-Stories `2b-a2/a3/a5/a6/a9/b1/b2`**: Re-mapped zu Epic 8/9/10. Story-Files existieren noch nicht — werden mit neuem Naming-Schema (`epic-8-…md`, `epic-9-…md`, `epic-10-…md`) via `bmad-create-story` angelegt, NICHT in dieser Proposal.
- **Phase-2-A-Retro AI-5 (Pill-Bar UX-Mini-Pass)**: Wird als eigene Pre-Story-Decision-Doc `pill-bar-ux-decisions.md` materialisiert (analog ADR-0013 vor A4-Story).
- **Phase-2-A-Retro AI-6 (Whisper-Local-Choice)**: Bereits erledigt durch ADR-0014 (Accepted, commit 0513969). Kein offener Punkt.

### Artifact Conflicts

| Artifact | Änderung |
|----------|----------|
| `epics.md` | Epic 8/9/10 als Stub-Header anhängen, MVP-Boundary-Marker zwischen Epic 10 und Epic 11+ einfügen |
| `sprint-status.yaml` | Phasen-Buckets weg (`epic-phase-2-b` raus, `epic-phase-2-a` bleibt als Hybrid-Outlier-done-Eintrag); Epic 5 → in-progress; Epic 6/8/9/10 als flache Einträge |
| `backlog.md` | Phasen-Headers (`Phase 2/3/4`) → thematische Buckets (`Windows-Daily-Drive-Kandidaten` / `Android-Daily-Drive-Kandidaten` / `MVP-Closure-Kandidaten`) |
| `_archive/phase-2-scope-lock.md` | Move (kein rename) — historischer Closure-Snapshot, kein BMad-Konstrukt |
| `product-brief-klarvo.md` | 2 kosmetische Phasen-Referenz-Edits |
| `pill-bar-ux-decisions.md` | Neu — Pre-Story-Decision-Doc für Pill-Bar (4 Decision-Points) |

### Technical Impact

Keine Code-Änderung. Pure Dokumentations-Reorg.

## Section 3: Recommended Approach

**Gewählt: Direct Adjustment (Option 1 aus Checklist Sec 4.1).** Issue ist via Artifact-Edits adressierbar — keine Code-Rollbacks, keine PRD-Goal-Änderung, keine MVP-Reduktion.

### Sequencing-Decision

1. **Epic 5 Re-Open** (Hardening-Bündel, AI-2 + AI-3 aus Retro) — vor allem anderen, weil Quality-Gate den nachfolgenden Epic-Stories Sicherheit gibt.
2. **Epic 6** (Observability) — vor Epic 8/9/10, weil Whisper-Local-Debugging (Epic 10) und externe-Tester-Triage strukturelle Logs brauchen.
3. **Epic 8** (Recording Modes & Hotkeys) — User-Value-Wave, dependency-frei nach Hardening.
4. **Epic 9** (UX Surface) — nach Pill-Bar UX-Mini-Pass; Welle-1-Stories (Return-Focus, Toasts) dependency-frei.
5. **Epic 10** (Whisper-Local) — kann parallel zu Epic 8/9 laufen (Substrate-Validation; ADR-0014 Accepted).
6. **Epic 7** (V1→V2 Migration) — bleibt deferred, Trigger = Onboarding-Flow-Eröffnung.

### Begründung

- **Epic 5 Re-Open statt eigener Mini-Epic:** AI-2 (Lint-Gate `clippy::disallowed_methods` für `expect`/`unwrap`) und AI-3 (REQUIRED_KEYS-Drift-xtask) sind 1:1 Epic-5-Substanz (Developer-Gate-Infrastructure). Honest re-open > artificial new epic.
- **Epic 6 vor Epic 8:** Gleiche Logik wie ADR-0013-vor-A4 in Phase-2-A — Foundation/Observability-Investment vor User-Value-Wellen.
- **Phasen-Vokabular Cleanup minimal-invasiv:** `epics.md` Historische Phase-Referenzen ("Phase-0-etabliert") bleiben als faktischer Kontext. Forward-Anchors ("Phase-2-UI-Expansion") werden zu Epic-Anchors umgeschrieben.
- **`epic-phase-2-a` bleibt als Hybrid-Outlier:** Keine Rückwärts-Rename von done-Material (path-hygiene per `feedback_memory_path_hygiene`). Sprint-Status zeigt es als done, klare Konvention dass das die Hybrid-Form vor BMad-Reset war.

### Effort-Estimate / Risk

- **Effort:** Low. Pure Doku-Edits. ~30 min Total-Work.
- **Risk:** Low. Keine Code-Änderung; keine in-flight-Stories betroffen (Phase-2-B hat noch keine Story-Files für die zu re-mappenden Items).
- **Timeline-Impact:** Neutral. Sequenz ändert sich (Epic 5 → 6 → 8/9/10), Volumen bleibt.

## Section 4: Detailed Change Proposals

### Edit 1: `epics.md` — Epic 5 Re-Open-Note

**Section:** Epic 5 Implementation Notes (nach Zeile L2244)

**Diff:**
```diff
 **Implementation Notes:**
 - FR35 (verify-release G2) und FR34 (lint-events G3) sind Phase-0-etabliert — Epic 5 **extends + hardens**, re-initialisiert nicht.
 - FR32 enforced Epic 1B FR6-Invariante auf xtask-Ebene (Pre-Commit-Mirror der Boot-Time-Executor-Strictness).
 - Persona-Achsentrennung: dies ist Plugin-Author-Tooling, kein End-User-Feature.
+
+**Re-Open-Erweiterung (2026-05-01, post-Phase-2-A-Retro):** Epic 5 wird re-opened für 2 Hardening-Stories aus Phase-2-A-Retro AI-2 + AI-3:
+- **5.5 Disallowed-Methods-Lint-Gate**: `clippy::disallowed_methods` für `expect`/`unwrap` in `klarvo-core` / `klarvo-windows-shell` / `klarvo-orchestrator`; Test-Module via `#[allow]`. Trigger: Phase-2-A-Retro Reibungsstelle 2 (Fail-Soft-Pattern wiederholt nachgepatcht in 4 Stories).
+- **5.6 REQUIRED_KEYS-Drift-Detection-xtask**: Parse JSON-Locale-Files, diff gegen `i18n.rs::REQUIRED_KEYS`. Trigger: Phase-2-A-Retro Reibungsstelle 3 (REQUIRED_KEYS-Drift in A4 → A8-Sub → D2 Nachpflege-Kette; G3-Lint catched anderes).
```

### Edit 2: `epics.md` — Epic 6 Forward-Anchor-Cleanup

**Section:** Epic 6 Implementation Notes (Zeile L2258)

**Diff:**
```diff
- FR40 ist explizit Phase-1-Stub; UI-triggered-Zip-Generation-Forward-Reference → Phase 2 als Inline-Notiz im Epic-6-Stub-Story-AC.
+ FR40 ist als Foundation-Stub committed (`klarvo-core::telemetry::export`); UI-triggered-Zip-Generation ist deferred-to-Epic-9 (UX Surface) als Settings-UI-gebundener Trigger.
```

### Edit 3: `epics.md` — Epic 7 Forward-Anchor-Cleanup

**Section:** Epic 7 Implementation Notes (Zeile L2274)

**Diff:**
```diff
- FR43 Exclude-Policy-AC: Verbatim-only-V2-Forward-Reference → Polished-Mode-Rebuild Phase 2 als Inline-Notiz (kein Platzhalter-Story, nur Kommentar).
+ FR43 Exclude-Policy-AC: Verbatim-only-V2-Forward-Reference → Polished-Cleanup-Plugin-Rebuild (deferred, MVP-Closure-Kandidat) als Inline-Notiz (kein Platzhalter-Story, nur Kommentar).
+
+**Eröffnungs-Trigger:** Epic 7 ist `backlog` mit Trigger = Onboarding-Flow-Eröffnung (v1-Import-UI-Button). Ohne UI-Surface ist die CLI-Migration Dev-only und für Andy als einzigen v1-Nutzer nicht load-bearing.
```

### Edit 4: `epics.md` — Epic 8/9/10 Stub-Header anhängen + MVP-Boundary

**Section:** Nach Epic 7 (nach L2274)

**Inhalt (kompletter Append-Block):**

```markdown
---

### Epic 8: Recording Modes & Hotkeys

Andy nutzt Toggle / AutoStop / Wait-and-Type Recording-Modi und einen zweiten Hotkey-Slot ohne `config.toml`-Edit. Recording-UX-Vollständigkeit für Daily-Drive.

**FRs covered:** Brief-bezogen (Recording-Modes, Second-Hotkey-Slot) — keine FR-Numerierung in PRD; bezieht sich auf `backlog.md` Items "Toggle + AutoStop + Wait-and-Type Recording-Modi" + "Second Hotkey-Slot".

**Dependencies:** Epic 5 (Lint-Gate vor Story-Writing, AI-2-Trigger), Epic-Phase-2-A (Settings-Service Foundation, ADR-0011 Hotkey-Backend, ADR-0012 Orchestrator-Owner).

**Implementation Notes:**
- 2.B.A1-Story (`2b-a1-toggle-autostop-wait-and-type-modes.md`) ist done — Letter-ID-Outlier (path-hygiene; Hybrid-Form-Erbe vor BMad-Reset 2026-05-01). Epic 8 referenziert das File, neue Stories folgen Naming-Schema `epic-8-…md`.
- Second-Hotkey-Slot extended ADR-0011 (Phase-1-Hotkey-Foundation) additiv.

---

### Epic 9: UX Surface — Pill Bar, Return-Focus, History, Toasts

Andy hat eine Pill-Bar-Visualisierung der laufenden Recording-Session, Return-Focus zum vorherigen Window nach Paste, History-Panel im Settings-Window und Windows-Toast-Notifications für relevante Events. UX-Daily-Drive-Vollständigkeit.

**FRs covered:** Brief-bezogen (UX-Polish + History-Visibility) — siehe `backlog.md` Items "Floating Pill Bar", "Return-Focus Feature", "History-Panel", "Windows-Toast-Notifications".

**Dependencies:** Epic-Phase-2-A (Settings-Window-Surface), Pre-Story-Decision-Doc `_bmad-output/planning-artifacts/pill-bar-ux-decisions.md` (vor Pill-Bar-Story).

**Implementation Notes:**
- Pill-Bar UX-Mini-Pass blockt Pill-Bar-Story (analog ADR-0013-vor-A4-Pattern aus Phase-2-A); Return-Focus + Toasts dependency-frei.
- Epic 6 (FR40 Log-Export-Stub) bekommt UI-Trigger in Epic 9 als Settings-UI-Aktion (Cross-Epic-Konsumption).

---

### Epic 10: Whisper-Local STT-Plugin (Substrate-Validation)

Substrate-Validation-Test: Zweiter STT-Plugin als reiner Trait-Impl, ohne Core-Änderung. Validiert Plugin-Architektur durch Cloud-vs-Local-Differenzierung. Zusatznutzen: Offline-Diktat-Kapazität.

**FRs covered:** Brief §Erfolgskriterien ("Neue Feature-Entwicklung ... erfordert keine Änderung an Shell-Code"); siehe `backlog.md` Items "Zweiter STT-Plugin (Trait-Stability-Test)" + "Audio-Capture-Config-Overrides via ShellConfig".

**Dependencies:** Epic 6 (strukturierte Logs für RTF / Latency / Drop-Diagnostik), Epic-Phase-2-A (Settings-Audio-Section für Capture-Overrides), ADR-0014 (Accepted, commit 0513969).

**Implementation Notes:**
- ADR-0014 ist Architektur-Anker; Plugin-Crate-Name + Cloud-vs-Local-Differenzierung dokumentiert.
- Audio-Capture-Config-Overrides (Story 3.7 §Phase-2-Expansion-Carry-Over) ist als zweite Epic-10-Story sinnvoll, weil Whisper-Local andere Capture-Defaults (16kHz Mono Float) als Cloud-Provider braucht.
- Observability-AC kann bei Bedarf inline statt eigener Epic-6-Story; Cross-Epic-Sequencing entscheidet beim Story-Writing.

---

## MVP-Boundary

**Innerhalb MVP-Scope:** Epic 1A — Epic 10 + Epic-Phase-2-A (Hybrid-Outlier).

**Post-MVP-Trigger-Epics (in `epics.md` ungestaffelt; Eröffnung erst bei Trigger):**
- Epic 7: V1→V2 Migration (Trigger: Onboarding-Flow-Eröffnung)
- Weitere Epics ergeben sich aus `backlog.md` MVP-Closure-Kandidaten + Post-MVP-P1/P2.

MVP-Definition referenziert `product-brief-klarvo.md` §Erfolgskriterien (Pipeline-Vollständigkeit + 2-Min-First-Diktat + Lizenz-System + v1-Import-Button auf Win + Android).
```

### Edit 5: `sprint-status.yaml` — Phasen-Buckets-Cleanup + Epic-Re-Strukturierung

Vollständiger Re-Write der `development_status:`-Section. Phase-1-Epics + epic-phase-2-a bleiben unverändert (done). Phase-2-B-Bucket gelöscht. Epic 5 → in-progress mit 2 neuen Stories. Epic 6/8/9/10 als flache Einträge.

(Konkrete YAML-Form: siehe Edit-Patch-Implementation in dieser Session.)

### Edit 6: `backlog.md` — Phasen-Headers → thematische Buckets

| Alt | Neu |
|-----|-----|
| `## Phase 2 — Windows daily usable` | `## Windows-Daily-Drive-Kandidaten` |
| `## Phase 3 — Android daily usable` | `## Android-Daily-Drive-Kandidaten` |
| `## Phase 4+ — MVP-Completion + Moat` | `## MVP-Closure-Kandidaten` |
| `## Post-MVP P1 (Early-Post-MVP, nach Phase 4)` | `## Post-MVP P1 (Early-Post-MVP, nach MVP-Closure)` |
| `## Post-MVP P2 (Power-Features, später als P1)` | unverändert |

Zusätzlich: `**Phase-Goal**`-Sub-Header in den drei Bucket-Intros umlabeln zu `**Bucket-Outcome**` und Phasenplan-Referenzen ("ref Product-Brief §Phasenplan") aktualisieren.

### Edit 7: `_archive/phase-2-scope-lock.md` — Move

`mv _bmad-output/planning-artifacts/phase-2-scope-lock.md _bmad-output/planning-artifacts/_archive/phase-2-scope-lock.md`

Filename bleibt original. Folder-Position macht Historik-Status explizit.

Cross-Refs aktualisieren in:
- `_bmad-output/planning-artifacts/epics/epic-phase-2-a.md` (source_docs-Frontmatter)
- `_bmad-output/implementation-artifacts/epic-phase-2-a-retro-2026-05-01.md` (inputs-Frontmatter)
- `memory/project_phase2_scope_lock.md` (Inline-Path-Reference; Memory-Update via memory-System, nicht in dieser Proposal)

### Edit 8: `product-brief-klarvo.md` — 2 kosmetische Edits

| Zeile | Alt | Neu |
|-------|-----|-----|
| L75 | `**MVP-Abschluss (Phase 4, 3–5 Monate Vollzeit konservativ geschätzt)**:` | `**MVP-Abschluss (3–5 Monate Vollzeit konservativ geschätzt)**:` |
| L87 | `... headless testbaren Core ab Phase 0, ...` | `... headless testbaren Core ab Workspace-Foundation, ...` |

### Edit 9: `pill-bar-ux-decisions.md` — Neu

Pre-Story-Decision-Doc für Pill-Bar (4 Decision-Points: Shape, Drag, Waveform-Render-Owner, Auto-Hide-Logic). Format analog ADR-0013, aber als UX-Mini-Pass. Blockt Epic-9-Pill-Bar-Story.

## Section 5: Implementation Handoff

**Scope-Klassifikation: Moderate** (Backlog-Reorganization).

**Handoff:** Andy als PO/DEV (Solo-Setup). Routing-Plan:

1. **Sofort (in dieser Session):** Edits 1-9 ausführen (bzw. wurden ausgeführt).
2. **Nach Doc-Edits:** Memory-Updates (separate Commits außerhalb der Doc-Edits per `feedback_adr_amendment_convention`-Pattern):
   - Update `project_phase2_scope_lock` mit Hinweis auf Re-Strukturierung + neuer Path `_archive/phase-2-scope-lock.md`.
   - Optional neuer Memory-Eintrag `feedback_phase_vs_epic_distinction` als Drift-Lehre.
3. **Vor Epic-5-Re-Open-Story-Writing:** AI-4 aus Phase-2-A-Retro ausführen (ADR-0013 §181 Amendment, Pre-Validation-Modell).
4. **Story-File-Generation für Epic 5.5/5.6/6/8/9/10:** Out-of-Scope für diese Proposal; läuft via `bmad-create-story`.

**Success-Kriterium:** `bmad-sprint-status` zeigt nach den Edits saubere flache Epic-Liste ohne Phasen-Buckets, mit Epic 5 in-progress und Epic 6/8/9/10 backlog.

---

## Workflow-Completion-Notes

- **Issue addressed:** Phase/Stream-Hybrid drift back to BMad-Standard
- **Change scope:** Moderate (Backlog-Reorg)
- **Artifacts modified:** epics.md, sprint-status.yaml, backlog.md, product-brief-klarvo.md, _archive/phase-2-scope-lock.md (move), pill-bar-ux-decisions.md (neu)
- **Routed to:** Andy (PO/DEV)
- **Approval:** 2026-05-01 (in correct-course-Session)
