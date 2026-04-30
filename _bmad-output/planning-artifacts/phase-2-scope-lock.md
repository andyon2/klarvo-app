---
title: "Phase 2 Scope Lock"
status: "locked"
phase: 2
created: "2026-04-26"
updated: "2026-04-26"
inputs:
  - _bmad-output/planning-artifacts/product-brief-klarvo.md
  - _bmad-output/planning-artifacts/product-brief-klarvo-distillate.md
  - _bmad-output/planning-artifacts/architecture.md
  - docs/rebuild-discussion.md
  - docs/backlog.md
  - _bmad-output/implementation-artifacts/deferred-work.md
  - _bmad-output/implementation-artifacts/epic-3-code-review-2026-04-25.md
  - _bmad-output/implementation-artifacts/epic-4-code-review-2026-04-25.md
  - _bmad-output/implementation-artifacts/epic-5-code-review-2026-04-26.md
  - docs/sanity-tester-onboarding.md
  - memory/project_phase1_complete
  - memory/project_klarvo_v2_rebuild
  - memory/project_market_positioning
  - memory/project_play_store_phase3_blocker
  - memory/project_android_playstore_risk
  - memory/project_jni_spike_scope
  - memory/feedback_phase_transition_discipline
  - memory/feedback_phase_boundary_check
  - memory/feedback_skip_with_rationale
  - memory/feedback_premature_abstraction_guard
  - memory/feedback_scope_lock_divergence_focus
---

# Phase 2 Scope Lock

## Outcome-Anker

> **Windows daily usable, validiert durch externen Tester mit signierter MSI-Installation und 1-Woche-Daily-Drive ohne `config.toml`-Edits.**

Brief-Original (§Phasenplan Phase 2: "Windows-Shell vollständig … Windows daily usable") ist load-bearing. Externe-Tester-Validation ist Sub-Form: Beweis, dass die Foundation trägt — nicht zusätzlicher Polish-Anspruch.

**Nicht Outcome-Form (explizit verworfen):**

- ⓒ "Settings-UI macht `config.toml` obsolet" — Sub-Form von ⓐ, nicht eigenständiges Ziel
- ⓓ "Phase-3-Vor-Check abgeschlossen (JNI grün, Play-Store-Audit submitted)" — Phase-3-De-Risking läuft als parallele Workstreams in Phase-2-A, ist aber nicht Phase-2-Outcome

## Phase-Boundary

### Was zu Phase 2 gehört
- Windows-Shell-Funktionsvollständigkeit für Power-User-Daily-Drive
- Settings-UI als Foundation-Layer (eliminiert `config.toml`-Friction)
- Substrate-Validation-Test: zweiter STT-Plugin als reiner Trait-Impl
- Phase-1-Carry-Over-Hardening und Tooling-Maturity (selektiv)

### Was explizit NICHT zu Phase 2 gehört (per Brief + PRD)
- **Lizenz-System** (HMAC + Trial + Cache + Grace) → Phase 4
- **OS-Keystore-Release-Default-Swap** → Phase 4 (Andy-Call 2026-04-21; v1-Doppelplacement bereinigt)
- **Polished-Cleanup-Plugin-Neubau** → Phase 4
- **Onboarding-Flow + v1-Import-UI-Button** → Phase 4
- **Android-Shell + AccessibilityService + Bubble + JNI-Bridge-Productivity** → Phase 3
- **Turso-Sync, Webhook, Reformate, Whisper-Modes, Stats-Panel, Cost-Tracking** → Post-MVP P1
- **Anthropic / OpenRouter / Custom-Prompts / App-Profiles / Voice-Notes / Snippets** → Post-MVP P2

---

## Wellen-Split

### Phase-2-A — Foundation + External-Validation + Phase-3-De-Risking (~3–5 Wochen)

**Ziel:** Settings-Panel als Foundation-Layer, signierte MSI für externen Tester, parallele Phase-3-Vor-Klärungen anstoßen.

**In-Scope:**

| ID | Item | Source | Dependencies |
|----|------|--------|--------------|
| **A4** | Minimales Settings-Panel | PRD L156 / Backlog "Minimales Settings-Panel" | ADR-0013 (Settings-Schema, Proposed) |
| **A8-Sub** | Tray-Language-Switcher | Backlog "Tray-Icon Extensions" (Subfeature C) | A4 |
| **C1** | Signierter MSI-Installer | Backlog "Signierter Installer / MSI-Distribution" | Code-Signing-Cert (extern beschafft) |
| **C2** | Hotkey-Konflikt-Erkennung | Backlog "Hotkey-Konflikt-Erkennung" | A4 |
| **C3** | Live-Locale-Switch (Hot-Reload) | Backlog "Live-Locale-Switch" | A4 |
| **E1** | Windows-Compile-CI-Gate (G6) | Backlog "Windows-Compile-CI-Gate" / Epic-3-Followup | — |
| **F1** | Play-Store-Policy-Audit (parallel-Start) | Backlog Phase-3 "Play-Store-Policy-Audit" / `memory/project_play_store_phase3_blocker` | — (extern: Google Developer Support) |
| **F2** | JNI-Test-Regression-Triage | `memory/project_jni_spike_scope` (2026-04-20-Regression) | — |
| **D2** | F3 Doppeltes Arc-Wrapping (Carry-Over) | `deferred-work.md` F3 / Epic-3-Review | — |
| **D3** | F4 Graceful-Shutdown `pipeline_task.abort()` | `deferred-work.md` F4 / Epic-3-Review | — (D5-Cross-Ref `RecordingCompleted` aus Story-3.5 vorhanden) |

**Pre-Story-Decision-Items (vor A4-Story):**
- ADR-0013 Settings-Persistence-Schema von `Proposed` → `Accepted` (Andy-Decision auf 5 Open Questions)

**Risiko-Flags (Divergenz-Anker per `feedback_scope_lock_divergence_focus`):**
- 5 parallele Workstreams in Phase-2-A → Koordinations-Overhead. C1 (Code-Signing-Cert-Beschaffung) und F1 (Google-Response-Lag 4–6 Wochen) hängen an externen Beteiligten und sind keine echten Konkurrenten zur Engineering-Zeit. Real-paralleler Engineering-Aufwand: A4+A8-Sub+C2+C3+E1+F2+D2+D3 = 8 Streams; davon sind C2/C3 explizit auf A4 sequenziert.
- Wenn Koord-Overhead spürbar wird: F2 JNI-Triage zuerst auf Triage-only herunterskalieren (Stunden-bis-Tage), Fix erst Phase-3-Vor-Check.
- D2/D3 sind selektiv, nicht der vollständige F-Sweep — wenn Phase-2-A überschätzt wird, weiter selektieren.

**Akzeptierte Schulden (NICHT in Phase-2-A behoben):**
- F11 TOML-Type-Mismatch UX → Phase-2-B nice-to-have (Settings-UI eliminiert das Problem strukturell)
- F14–F24 Epic-5-Defer-Items → Phase-2-B oder Post-MVP, nicht load-bearing für Daily-Drive

### Phase-2-B — User-Value-Wave (~4–6 Wochen)

**Ziel:** Recording-Mode-Vollständigkeit + Pill-Bar + 2nd-STT-Plugin-Substrate-Validation. "Windows daily usable" wird hier erreicht.

**In-Scope:**

| ID | Item | Source | Dependencies |
|----|------|--------|--------------|
| **A1** | Toggle + AutoStop + Wait-and-Type Recording-Modi | PRD L153 / Backlog "Toggle + AutoStop + Wait-and-Type" | ADR-0012 erweitert (Phase-1-Basis vorhanden) |
| **A2** | Second Hotkey-Slot | PRD L154 / Backlog "Second Hotkey-Slot" | A4 (Settings-Panel), ADR-0011 (Phase-1) |
| **A3** | Floating Pill Bar | PRD L179 / Backlog "Floating Pill Bar" | **Pill-Bar UX-Mini-Pre-Story** (Mock-Sketches) |
| **A5** | Return-Focus | Backlog "Return-Focus Feature" | — |
| **A6** | History-Panel | Backlog "History-Panel" | A4 (Main-Window) |
| **A9** | Windows-Toast-Notifications | Backlog "Windows-Toast-Notifications" | — |
| **B1** | Zweiter STT-Plugin (Substrate-Validation) | Brief §Erfolgskriterien / Backlog "Zweiter STT-Plugin" | **Plugin-Choice Pre-Story-Decision** (Empfehlung: Whisper-Local) |
| **B2** | Audio-Capture-Config-Overrides | Backlog "Audio-Capture-Config-Overrides" / Story 3.7 §Phase-2-Expansion | A4 (Audio-Section in Settings-UI) |

**Pre-Story-Decision-Items (vor entsprechenden Stories):**
- **Pill-Bar UX-Mini-Pass** (vor A3): Mock-Sketches für Shape, Drag, Waveform-Render-Owner, Auto-Hide-Logic. Output: kein Full-UX-Design, nur entscheidungs-genug-Spec für Implementation-Story. Begründung: UX ist nicht-textbar; Inline-Story-Decisions würden schlechte Spec produzieren.
- **2nd-STT-Plugin-Choice** (vor B1): Empfehlung Whisper-Local statt Deepgram (Cloud-vs-Local-Differenzierung; kein P1-OpenAI-Whisper-Cloud-Konflikt). Decision-Output: ADR-Stub + Plugin-Crate-Name.

**Akzeptierte Schulden:**
- F11/F12/F13 (Epic-4-Defer) bleiben offen, falls in Phase-2-A nicht mit-eingesammelt.
- Keine Pill-Bar-Polish-Iteration (Multi-Monitor, Snap-zu-Edges, etc.) — Phase-2-B schafft Bar, Polish-Iteration ist Post-MVP.

---

## Skip-Section (Out-of-Scope für Phase 2, mit Forward-Looking-Risks)

Per `feedback_skip_with_rationale`: jeder bewusste Phase-2-Skip dokumentiert Forward-Looking-Risk + Risk-Acceptance-Reason.

### A7 StylePicker (UI-Component) → Phase 4

- **Forward-Risk:** Externe Tester können Cleanup-Style nicht UI-wechseln (Verbatim/Chat/Polished). Bei Wunsch nach Polished-Output gibt es keine Switching-Möglichkeit.
- **Risk-Acceptance:** Verbatim-Default ist Brief-explizit Andy-Default ("ich nutze Polished nie"). Chat-Plugin existiert in Phase-2 noch nicht (Phase-4-Dep), Polished-Plugin wird erst Phase-4 neu gebaut. Ohne diese Plugins ist StylePicker eine UI-Hülle ohne Inhalt.
- **Trigger für Re-Visit:** Wenn ein Tester explizit nach Style-Switching fragt, ist das ein Phase-4-Vorzieh-Trigger, nicht Phase-2-Add-On.

### A10 Autostart + Notification-Area-Badge → Phase 4 (Onboarding-Polish)

- **Forward-Risk:** Tester muss Klarvo nach jedem Login manuell starten. Friction-Punkt für Daily-Drive.
- **Risk-Acceptance:** Power-User-Zielgruppe (per `memory/project_market_positioning`) toleriert manuellen Start; Phase-4-Onboarding-Polish ist Mass-Outreach-Sequenz, nicht Power-User-Validation. Sanity-Tester-Onboarding-Doc kann Manual-Start als Workaround dokumentieren.
- **Trigger für Re-Visit:** Wenn Tester-Feedback auf Autostart-Hürde dominiert, Phase-4-Vorzieh-Trigger.

### B3 PluginError → i18n-Key-Mapping → Phase 4

- **Forward-Risk:** 5 von 6 PluginError-Varianten (`Network`/`Auth`/`RateLimit`/`Fatal`/`UpstreamUnavailable`) zeigen generischen `error.internal`-Toast statt provider- oder grund-spezifischer User-Message. Tester sehen "Internal Error" statt "Authentifizierung fehlgeschlagen".
- **Risk-Acceptance:** Phase-1-Workaround (Epic-4-Review-Patch P1: `error.internal` registriert) ist als Stop-Gap dokumentiert. Provider-spezifische Keys sind kosmetisch — Funktionalität (Retry, Logs) ist intakt. Phase-4 bündelt das mit Polished + Onboarding-Polish.
- **Trigger für Re-Visit:** Wenn Tester `error.internal`-Toasts häufig sehen und Provider-Triage erschwert wird, Phase-2-B-Vorzieh-Trigger.

### C4 Debug-Export-Zip UI-Trigger → Phase-2-B nice-to-have (oder Phase 4)

- **Forward-Risk:** Tester kann bei Bugs keine standardisierte Logs+Config-Bundle exportieren; Manual-Folder-Zip-Workaround.
- **Risk-Acceptance:** `klarvo-core::telemetry::export`-Module-Stub existiert seit FR40 (Phase-1). UI-Trigger ist 1-Story; kann optional Phase-2-B mit-eingesammelt werden, sobald Settings-Panel-Surface steht. Falls Phase-2-B-Volumen knirscht, raus.
- **Trigger für Re-Visit:** Wenn Tester-Bug-Reports schwer reproduzierbar werden (Logs-Sammeln-Friction), in Phase-2-B aufnehmen.

### C5 Plugin-Author-Guide + Editor-Schema-Support → Phase-2-B nice-to-have (oder Phase 3+)

- **Forward-Risk:** Plugin-Author-Persona (Brief §Zielnutzer Sekundär) hat keine Onboarding-Doc. Validation-Persona-Onboarding ist nicht möglich.
- **Risk-Acceptance:** Plugin-Author-Persona ist Phase-2+-Trigger, nicht Phase-2-Daily-Drive-Concern. Externe Validation läuft über Sanity-Tester (Power-User), nicht Plugin-Authors. Editor-Schema (Taplo-LSP) ist DX-Polish, kein Funktions-Blocker.
- **Trigger für Re-Visit:** Wenn ein konkreter Plugin-Author-Onboarding-Anlass entsteht (externer Beitrag), Phase-2-B-Vorzieh-Trigger.

### Phase-3-Items (vollständig out-of-Phase-2, hier nicht skippt — Phase-3-Goal)
Android-Shell, Bubble-UX, AccessibilityPasteBackend, Android-v1-Import, Microsoft-/Play-Store-Distribution. Backlog-Phase-3-Section bleibt authoritative für diese.

---

## Open Items für Pre-Story-Decisions

Items, die vor entsprechender Story-Eröffnung User-Decision brauchen:

1. **ADR-0013 Settings-Persistence-Schema** — Status `Proposed`. 5 Open Questions zu beantworten (siehe ADR). Blockt: Story 2.A.4 (Settings-Panel).
2. **Pill-Bar UX-Mini-Pass** — Mock-Sketches + 4 Decision-Points (Shape, Drag, Waveform-Render-Owner, Auto-Hide). Format: kein Full-UX-Design, nur entscheidungs-genug-Spec. Blockt: Story 2.B.A3 (Pill Bar).
3. **2nd-STT-Plugin-Choice** — Empfehlung Whisper-Local. Decision-Output: ADR-Stub + Plugin-Crate-Name. Blockt: Story 2.B.B1 (Zweiter STT-Plugin).

---

## Volumen-Schätzung + Timeline-Konsistenz

| Welle | Volumen | Real-paralleler Eng-Aufwand |
|-------|---------|----------------------------|
| Phase-2-A | 3–5 Wochen | A4+A8-Sub+C2+C3+E1+F2+D2+D3 (C1+F1 extern-warten-getrieben) |
| Phase-2-B | 4–6 Wochen | A1+A2+A3+A5+A6+A9+B1+B2 |
| **Total Phase-2** | **7–11 Wochen** | passt knapp in 3–5-Monate-MVP-Timeline (`memory/project_klarvo_v2_rebuild`); Voraussetzung: keine Welle-Überlappung |

**Timeline-Risk:** Wenn Phase-2-A überzieht (>5 Wochen), Phase-2-B-Start verschiebt sich proportional. Phase-3-Start (Android) hängt an Phase-2-B-Closure + Play-Store-Audit-Resolution (parallel in F1 angestoßen).

---

## Cross-References

- `docs/backlog.md` — Single-Source-of-Truth-Item-Inventur (Phase 2/3/4/P1/P2). Dieses Scope-Lock ist Phase-Boundary-Snapshot, kein Backlog-Ersatz.
- `docs/adr/0013-settings-persistence-schema.md` — Mini-Arch-Pass, Status Proposed, blockt A4.
- `docs/adr/0011-hotkey-backend.md` — Phase-1-Hotkey-Foundation, additiv erweitert in A2.
- `docs/adr/0012-orchestrator-owner.md` — Phase-1-Orchestrator-Foundation, erweitert für A1 (Recording-Modi).
- `_bmad-output/planning-artifacts/architecture.md` §2 :247 (KeyStore-Phase-Placement, Phase-4-Confirmation), §2 :245 (Config-Hybrid-Decision, ADR-0013-Vorlage), §7 :311 (Play-Store-Phase-3-Blocker, F1-Source).
- `_bmad-output/planning-artifacts/product-brief-klarvo.md` §Phasenplan, §Erfolgskriterien (2nd-STT-Plugin als Validation).
- `_bmad-output/implementation-artifacts/deferred-work.md` — F1–F24 Defer-Items, selektiv eingesammelt in Phase-2-A (D2/D3).
- `memory/feedback_skip_with_rationale` — Skip-Section-Form-Convention.
- `memory/feedback_premature_abstraction_guard` — Begründung für ADR-0013-Mini-Pass statt Inline-Decision.
- `memory/feedback_scope_lock_divergence_focus` — Risiko-Flag-Konvention.

---

## Next Actions

1. Andy review + accept Scope-Lock → Status `locked` (bereits gesetzt; bei Divergenz revidieren).
2. ADR-0013 Open Questions beantworten → Status `Proposed` → `Accepted`.
3. `bmad-create-story-acs` für Phase-2-A starten (Reihenfolge nach Dependencies: A4 zuerst, dann A8-Sub/C2/C3 parallel, E1/F2/D2/D3/F1/C1 unabhängig).
4. Pill-Bar UX-Mini-Pass + 2nd-STT-Plugin-Choice → vor Phase-2-B-Start adressieren (entweder am Ende von Phase-2-A oder als Phase-2-B-Pre-Flight).
