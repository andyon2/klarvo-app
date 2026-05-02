---
title: "Pill-Bar UX-Mini-Pass — Pre-Story-Decisions"
status: accepted
date: 2026-05-01
accepted_date: 2026-05-03
form: UX-Mini-Pass (kein Full-UX-Design, kein ADR)
blocks: Epic 9 Pill-Bar-Story
pattern_ref: ADR-0013-vor-A4 (Phase-2-A) — Pre-Story-Decision-Pattern reproduziert
inputs:
  - _bmad-output/planning-artifacts/epics.md (Epic 9)
  - _bmad-output/implementation-artifacts/epic-phase-2-a-retro-2026-05-01.md (AI-5)
  - docs/backlog.md "Floating Pill Bar"
  - memory/feedback_premature_abstraction_guard
  - memory/project_shell_session_lifecycle (per-Hotkey-Cycle-Topology)
---

# Pill-Bar UX-Mini-Pass — Pre-Story-Decisions

## Zweck

Pill-Bar ist UI-Surface mit nicht-textbaren UX-Entscheidungen. Inline-Decisions in der Pill-Bar-Story produzieren schlechte Specs (gleicher Lehre wie ADR-0013-vor-A4 in Phase-2-A: Foundation-Decisions vor Story-Writing). Dieser Mini-Pass adressiert die 4 Decision-Points, die die Implementation-Story blocken — und nichts darüber hinaus.

**Out-of-Scope:** Polish-Iteration (Multi-Monitor, Snap-zu-Edges, Custom-Themes), Animations-Tuning, Accessibility-Audit. Das sind Post-MVP-Items.

## Status

**Accepted (2026-05-03).** Alle 4 Decisions durch Andy approved. Epic 9 Pill-Bar-Story ist dispatchable.

## Pattern-Referenz

ADR-0013 (Settings-Persistence-Schema) hat in Phase-2-A das Pre-Story-Decision-Pattern etabliert: 5 Sub-Decisions vor A4-Story-Writing → A4 + 3 Folge-Stories (A8-Sub/C2/C3) konsumierten ohne Re-Design (Phase-2-A-Retro Reibungs-Befund Nr. 1: "A4 als Foundation hat getragen"). Dieser Doc reproduziert das Pattern für Pill-Bar — UX-Mini-Pass statt ADR (UX-Decision, nicht Architektur-Decision).

---

## Decision-Point 1: Shape

**Frage:** Welche Form hat die Pill-Bar im Idle-Zustand und beim aktiven Recording?

**Optionen:**
- **A** Fixe Größe (z.B. 320×48px), runde Ecken, immer gleicher Footprint, nur interner Inhalt wechselt zwischen Idle-Mic-Icon und Live-Waveform.
- **B** Adaptive Größe — schmal im Idle (96×32px reines Mic-Icon), expandiert bei Recording auf Waveform-Breite.
- **C** Floating-Capsule mit dynamischer Höhe je nach Mode (Hold = schmal; Toggle = mittel mit Stop-Button; AutoStop = mittel mit Countdown-Indikator).

**Decision: A — Fixe Größe (2026-05-03)**

YAGNI. Tauri-Window-Resize ist auf Win32 fragil; adaptive Size ist UX-Polish, kein MVP-Blocker. Footprint: 320×48px, runde Ecken.

## Decision-Point 2: Drag-Verhalten

**Frage:** Soll der User die Pill-Bar mit der Maus auf dem Bildschirm verschieben können? Wenn ja: wann und wie wird die Position persistiert?

**Optionen:**
- **A** Nicht draggable — Pill-Bar ist immer am festen Default-Position (z.B. unten-mitte). Andy bevorzugt deterministisch.
- **B** Draggable, Position wird via Settings-Service (ADR-0013-Konvention) als `ui.pill_bar.position_x` / `…position_y` persistiert — Live-Update beim Drop.
- **C** Draggable nur via Modifier+Drag (z.B. Ctrl+Drag), um versehentliche Verschiebungen zu vermeiden. Position-Persistierung wie B.

**Decision: A — Nicht draggable (2026-05-03)**

YAGNI. Feste Default-Position (unten-mitte). Kein Settings-Key, keine Hit-Box-Konflikte, deterministisch.

## Decision-Point 3: Waveform-Render-Owner

**Frage:** Wer rendert die Live-Waveform — Frontend (WebView via Web-Audio-API-Tap auf Audio-Stream) oder Backend (Rust → Tauri-Event mit pre-computed Wave-Frame-Buffer pro N ms)?

**Optionen:**
- **A** Frontend-Render: WebView empfängt Audio-Frames via Tauri-Event (z.B. `audio.frame`), rendert in Canvas. Niedrige Backend-Last, aber jeder Frame muss serialisiert werden.
- **B** Backend-Render: Rust pre-computed Wave-Frame-Buffer (z.B. 64 amplitude-bins pro 50ms), sendet als `pill_bar.waveform_tick`-Event mit `Vec<f32>` payload. Frontend zeichnet nur die bins.
- **C** Backend-Render-as-Image: Rust pre-renderet die Waveform als PNG/Bitmap, sendet base64-encoded. Niedrigste Frontend-Last, höchste Latenz + Payload-Größe.

**Decision: B — Backend pre-computed bins (2026-05-03)**

Backend ist Audio-Owner. 64 amplitude-bins × 4 byte × 20Hz ≈ 1KB/s IPC-Volume. `pill_bar.waveform_tick`-Event mit `Vec<f32>`-Payload + `ts_ms` (ts_ms-Convention, ref memory/project_event_ts_ms_convention).

## Decision-Point 4: Auto-Hide-Logic

**Frage:** Wann verschwindet die Pill-Bar — und wie kommt sie zurück?

**Optionen:**
- **A** Always-visible: Pill-Bar ist immer am Screen, im Idle als kleines Mic-Icon. Klar sichtbar, aber visuelles Rauschen.
- **B** Show-on-Recording-only: Pill-Bar erscheint mit Hotkey-Press (Step 1 der Session-Lifecycle, ref `memory/project_shell_session_lifecycle`), verschwindet nach Drop (Step 7) mit Fade-Out (z.B. 300ms).
- **C** Auto-Hide-after-Idle: Always-visible, aber dimmt nach X Sekunden ohne Recording (z.B. Opacity 0.3 nach 5s) — kommt zurück bei Hover oder Recording-Start.

**Decision: B — Show-on-Recording-only (2026-05-03)**

Minimalistisch, passt zu Power-User-Persona. Pill-Bar erscheint bei `RecordingStarted`-Event, verschwindet nach `RecordingCompleted` mit 300ms Fade-Out. Tray-Icon zeigt "läuft Klarvo" im Idle.

---

## Cross-Cutting-Notizen

- **Session-Lifecycle-Anker:** Pill-Bar visualisiert die per-Hotkey-Cycle 7-Step-Topology (`memory/project_shell_session_lifecycle`). Pill-Bar-State-Machine muss zu den 7 Steps mappen.
- **Settings-Service-Kopplung:** Falls Decision 2 = B oder C: Pill-Bar-Position-Felder erweitern Settings-Schema additiv (ADR-0013-Pattern, Format-Mutability bis Phase-4 erlaubt — siehe ADR-0013 Q4-Resolution).
- **i18n-Anker:** Pill-Bar hat keine User-facing Strings im MVP (nur Icons). Falls Tooltip / Status-Text nötig: registrierte i18n-Keys über Epic 4-Foundation; G3-Lint deckt das ab.

---

## Approval-Path

1. Andy beantwortet Decisions 1-4 (incremental oder batch).
2. Begründungs-Notizen pro Decision werden eingetragen (kurz, nicht ausschweifend).
3. Status-Frontmatter `open` → `accepted`.
4. Epic 9 Pill-Bar-Story-Writing dispatchable (`bmad-create-story`).
