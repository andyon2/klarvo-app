# Story 9.14: HOLD-Modus (Push-to-Talk) — vereinfacht (ein Abbrechen-Button)

Status: ready-for-dev

> **⚠️ NEU GEFASST 2026-07-01 — SUPERSEDET die Zwei-Ziel-B-Sprache-Fassung.** Die Zwei-Ziel-Variante
> (Sperren + Abbrechen, commits `ce20bb0`/`c431ba5`) fiel an Andis echtem Gerät durch: alles zu groß,
> Anker-K springt vs. Idle-Bubble, und die Design-Erkenntnis „Loslassen = senden braucht keinen eigenen
> Button" macht die zwei gleichberechtigten Ziele überflüssig. Diese Fassung baut das **vereinfachte**
> Modell (ADR-0019 Amendment 2026-07-01): **ein** Abbrechen-Button, Senden = Loslassen, kein Sperren.
> Tasks/Subtasks + Dev Notes werden von `bmad-create-story` gegen den neuen Canon neu generiert.

## Story

As a user dictating on Android with the **Hold** gesture,
I want to hold to record, **let go to send**, and drag to a single clear **Cancel** button to discard,
so that sending is effortless (just release) and only the destructive action needs a deliberate target — with thumb-sized, on-screen-fitting controls.

## Scope (locked — vereinfachtes HOLD, nur wenn `longPressMode == RecordingMode.HOLD`)

- **Anker-Bubble** (teal-Gradient-Squircle, „K", amber Halte-Ring) am Dock, wo der Finger hält — **Größe = Idle-Bubble-Größe** (`bubbleSizeDp`, responsive ~44dp). Kein eigener Größen-Parameter; entkoppelt vom Button-Regler. Kein Größen-/Orts-Sprung gegenüber der Idle-Bubble.
- **Ein** runder **Abbrechen-Button** (✗, dunkle blickdichte Ruhe-Fläche + roter Ring) — wächst weg von Daumen/Dock-Kante. **Größe am `recordingButtonSizeDp`-Regler.**
- **Grow-on-target:** sobald der Finger den Button erreicht, **wächst er + leuchtet rot**, Label → „loslassen = abbrechen".
- **Release-to-commit:** **Loslassen auf dem Button = `cancelRecording()`**. **Loslassen überall sonst = senden (`stopAndProcessRecording()`)**. **Zurückziehen** vom Button vor dem Loslassen = nichts (Undo).
- **Dynamik (bauen):** Ghost-Bubble folgt dem Finger · Origin-Bubble faded auf ~.32 beim Ziehen · Caption wechselt auf „Finger auf Abbrechen · loslassen löst aus".
- **Dock-adaptiv:** Button-Position spiegelt je Andock-Seite; wächst nie unter den Daumen.
- **Regler erweitern:** `recordingButtonSizeDp`-Auswahl um mehr + kleinere Stufen erweitern (näher an Idle-Größe). Sub-Label gilt dann für TAP **und** HOLD-Abbrechen.

**Hard scope boundaries:**
- **Kein Sperren / kein Lock→TAP** in HOLD — die gesamte Sperren-Mechanik (`lockHoldToCluster`, Lock-Ziel, hochziehen-sperren) entfällt für HOLD.
- **Kein** Senden-Button (Senden = Loslassen).
- **Nur** HOLD-Modus. Tap/Toggle/Auto-Stop/Auto = 9-15-TAP-Surface (nicht hier anfassen).
- **Keine** RMS/Waveform-Änderung (Story 9-12). **Keine** Token-Änderung (`KlarvoTheme.kt`). Farb-Semantik: **teal=Anker · amber=live · rot=Abbrechen**.
- 9-7 (Gesten-Modus-Erkennung) **nicht** still erweitern.
- `FLAG_NOT_TOUCHABLE` nie.

## Acceptance Criteria

**AC1 — Vereinfachte HOLD-Surface.** Given `pushToTalkActive` und Aufnahme startet, When gezeichnet wird, Then erscheinen: kleine Anker-Bubble (= Idle-Größe) am Dock + **ein** Abbrechen-Ruhe-Button (✗, rot-Ring) + ruhiger Waveform-Chip + Caption „Aufnahme · loslassen = senden" — und **kein** Sperren-Ziel, **kein** Senden-Button, keine alten `.ab-holddock`-Surfaces. Werte aus `mockup-mobile-hold-simple.html` Frame `sRest`.

**AC2 — Anker = Idle-Größe, kein Sprung.** Die Anker-Bubble hat dieselbe Größe wie die Idle-Bubble (`bubbleSizeDp`) und erscheint an derselben On-Screen-Position, an der die Idle-Bubble war (Daumen ist physisch dort) — kein sicht­barer Größen- oder Orts-Sprung beim Übergang idle→hold.

**AC3 — Abbrechen am Regler.** Der Abbrechen-Button skaliert mit `recordingButtonSizeDp`. Der Regler bietet mehr + kleinere Stufen als {60,72,88} (näher an Idle-Größe).

**AC4 — Grow-on-target.** When der Finger den Abbrechen-Button erreicht, Then wächst er + leuchtet rot + Label „loslassen = abbrechen". Render: `sHit`.

**AC5 — Release-to-commit + Senden-by-default.** Loslassen **auf** dem aktiven Abbrechen-Button → `cancelRecording()`. Loslassen **irgendwo sonst** → `stopAndProcessRecording()` (senden). Zurückziehen vom Button vor dem Loslassen → kein Auslösen.

**AC6 — Dynamik.** Beim Ziehen: Ghost-Bubble folgt dem Finger, Origin-Bubble faded, Caption wechselt auf „Finger auf Abbrechen · loslassen löst aus" (Render `sHit`).

**AC7 — Dock-adaptiv.** Button-Position spiegelt je Andock-Seite; wächst weg von Dock-Kante/Daumen.

**AC8 — Andere Modi/Zustände unberührt.** Tap/Toggle/Auto = 9-15-Surface; IDLE/TRANSCRIBING/DONE unverändert.

**Inversion (must-fail gates):**
- Sperren-Ziel oder ein Senden-Button im HOLD-Zustand sichtbar = review failure.
- Anker-Bubble größer als / versetzt zur Idle-Bubble = review failure (AC2).
- Auslösung beim Schwellwert *während der Bewegung* statt beim **Loslassen** = review failure (AC5).
- Loslassen außerhalb des Abbrechen-Buttons bricht ab statt zu senden = review failure (AC5).
- Farben getauscht = review failure.

## Tasks / Subtasks

> _Wird von `bmad-create-story` gegen den neuen Canon (`mockup-mobile-hold-simple.html`) + den aktuellen Code-Stand (commit `c431ba5`) neu generiert. Die alte Zwei-Ziel-Zerlegung ist hinfällig._

## Anchors (binding design source)

- **ADR-0019 Amendment 2026-07-01** (`docs/adr/0019-cross-platform-design-ssot.md`) — vereinfachtes HOLD.
- **Bindendes Render (SOLL):** `docs/design/overhaul/mockup-mobile-hold-simple.html` (Frames `sRest` + `sHit`), Fingerprint `7e2829a5625c224fb2227cff53cefa70`. **Supersedet** `mockup-mobile-hold-B-refined.html`.
- Baut auf der Vorgänger-Implementierung `c431ba5` auf (entfernt deren Lock-Ziel + zweites Ziel; entkoppelt Anker-Größe; koppelt Abbrechen an Regler).
- Touch/Canvas: `reference_android_bubble_canvas_and_install.md`. Verifikations-Lehre: `feedback_gate4_smoke_needs_behavioral_delta.md`.

## DoD (surface-class)

DEBUG APK builds (via `tauri android build`, Node 20); JVM-Tests grün; Emulator strukturelle Smoke grün (Fenster-Struktur via `dumpsys window` unter `BMAD_CONDUCTOR=1`). **GATE-4 Bewegung/Touch/Visual = echtes Gerät (Andis Batch-Gate):** halten/loslassen=senden/auf-Abbrechen-ziehen, grow-on-target, Ghost-Folgen, Anker = Idle-Größe ohne Sprung, Regler-Stufen — nur am echten Gerät verifizierbar. `FLAG_NOT_TOUCHABLE` nie.

## Dependency

Nutzt 9-15-TAP-Surface-Infrastruktur (Geometrie-Pattern, `recordingButtonSizeDp`) — aber **nicht** den Gesperrt-Zustand (kein Lock mehr). Regler-Erweiterung berührt 9-15-Settings-UI (`ShortcutsContent.tsx`).

## Dev Agent Record

> _Wird beim Build neu befüllt._

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | (frühere Fassung) Slide-Spur-HOLD gebaut (`c389c88`/`e92f4f3`), GATE-4 Real-Device FAILED (Andi). | conductor |
| 2026-06-26 | NEU GEFASST in B-Sprache (Zwei-Ziel: Sperren + Abbrechen) gegen `mockup-mobile-hold-B-refined.html`. | claude-opus-4-8 |
| 2026-06-30 | Implementiert (`ce20bb0`) + Code-Review-Fixes (`c431ba5`, A/B/C) + GATE-4 Struktur-Smoke grün. Status `review`. | claude-sonnet-5 / conductor |
| 2026-06-30 | **GATE-4 Real-Device FAILED (Andi):** zu groß · Anker-K springt vs. Idle · Design-Erkenntnis „Senden=Loslassen braucht keinen Button". | claude-opus-4-8 (conductor) |
| 2026-07-01 | **NEU GEFASST — vereinfacht** (ADR-0019 Amendment 2026-07-01): ein Abbrechen-Button · Senden=Loslassen · kein Sperren · Anker=Idle-Größe · Abbrechen am erweiterten Regler · Dynamik (Ghost/Fade/Caption) gebaut. Canon `mockup-mobile-hold-simple.html` (`7e2829a5…`). Zwei-Ziel-Scope superseded. `ready-for-dev`. | claude-opus-4-8 (conductor) |
