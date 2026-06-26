# Story 9.14: HOLD-Modus (Push-to-Talk) — Mobile-Redesign (B-Sprache)

Status: ready-for-dev

> **⚠️ NEU GEFASST 2026-06-26 — SUPERSEDET die frühere 9-14-Implementierung.** Die erste Umsetzung
> (Slide-Spur-HOLD, `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-heldbub`/`.ab-lockchip`; Commits
> `c389c88` + `e92f4f3`) fiel an Andis echtem Gerät durch (zu klein, Finger verdeckt die UI, „Laptop-Feel").
> Diese Story baut die HOLD-Geste in der **B-Sprache** neu (ADR-0019 Amendment 2026-06-26). Die Dev-Session
> **überarbeitet** die alten Surfaces, erweitert sie nicht. Die HOLD-**Intention** (halten/loslassen/wegziehen/
> hochziehen) bleibt — nur Surface, Größe, Geometrie und Feedback sind neu.

## Story

As a user dictating on Android with the **Hold** gesture mode,
I want large, thumb-friendly hold targets that **grow when my finger lands on them** and only fire **on release**,
so that I can see what I'm doing while my thumb is on screen, and cancelling or locking feels deliberate and reversible.

## Scope (locked — Hold-mode interaction + its B-language surfaces only)

HOLD-Aufnahme in B-Sprache, **nur** wenn `longPressMode == RecordingMode.HOLD` (`pushToTalkActive = true`):
- **Daumen-Anker-Bubble** (teal-Gradient, amber Halte-Ring) am Dock, wo der Finger hält.
- Zwei **große runde Ziele** wachsen heraus, weg von Daumen/Dock-Kante: **Sperren** (teal, Schloss-Icon, nach **oben** zur Display-Mitte) + **Abbrechen** (rot, ✗, weiter **unten**). Aufgeräumt, nicht überlappend; ruhiger Waveform-Chip an der Bubble.
- **Grow-on-target:** sobald der Finger ein Ziel erreicht, **wächst es + leuchtet** (rot bzw. teal) und der Text wird zu „loslassen = abbrechen" / „loslassen = sperren".
- **Release-to-commit + Undo:** **Loslassen auf einem Ziel löst aus**; Zurückziehen vor dem Loslassen = nichts passiert. **Loslassen ohne Ziel = senden** (`stopAndProcessRecording()`).
- **Hochziehen-Sperren:** Loslassen auf dem Sperren-Ziel → Aufnahme **gesperrt** → wandelt in die **TAP-Surface (Story 9-15)** (große tappbare Senden/Abbrechen) — jetzt kann man loslassen, ohne zu senden.
- **Dock-adaptiv:** Ziele/Anordnung spiegeln je Andock-Position; Ziele wachsen nie unter den Daumen.

**Hard scope boundaries:**
- **Nur** der HOLD-Modus. Tap/Toggle/Auto-Stop/Auto nutzen die TAP-Surface aus **9-15** (nicht hier bauen).
- Der „gesperrt"-Zustand = die 9-15-TAP-Surface (nicht doppelt bauen — wiederverwenden).
- **Keine** RMS/Waveform-Änderung (Story 9-12; `drawClusterWaveform`/`waveLevels`/`amplitude`/`setStaticWaveLevel` unverändert).
- **Keine** Token-Änderung (`KlarvoTheme.kt` generiert). Farb-Semantik bindend: **teal=Senden/Sperren-Akzent · amber=live/Halte-Ring · rot=Abbrechen**.
- 9-7 (Gesten-Modus-**Erkennung**) **nicht** still erweitern — existiert; hier nur die HOLD-Surface+Interaktion.
- `FLAG_NOT_TOUCHABLE` nie (HyperOS dimmt auf 0.8).

## Acceptance Criteria

**AC1 — HOLD-Surface statt Klein-Variante.** Given `pushToTalkActive = true` (Hold) und Aufnahme startet, When gezeichnet wird, Then erscheinen Daumen-Anker-Bubble + zwei große runde **Ruhe**-Ziele (Sperren teal/Schloss oben · Abbrechen rot/✗ unten) + ruhiger Waveform-Chip an der Bubble — und **keine** alte Slide-Spur / kein `.ab-holddock`-Klein-Dock. Werte exakt aus `mockup-mobile-hold-B-refined.html` (Frame `bRest`).

**AC2 — Aufgeräumt, kein Überlapp.** Waveform-Chip und die zwei Ziele überlappen **nicht**; Ziele groß (≥ ~112dp Ruhe), großzügig getrennt, auf blickdichten Flächen.

**AC3 — Grow-on-target (Abbrechen).** Given Hold aktiv, When der Finger das Abbrechen-Ziel erreicht, Then **wächst** das Ziel (≥ ~148dp) + **leuchtet rot** + Text „loslassen = abbrechen". Render: `bHit`.

**AC4 — Grow-on-target (Sperren).** Wie AC3 für das Sperren-Ziel beim Hochziehen: wächst + **leuchtet teal** + „loslassen = sperren". Render: `holdLock` (in `mockup-mobile-recording-states.html`).

**AC5 — Release-to-commit + Undo.** Loslassen **auf** dem aktiven Abbrechen-Ziel → `cancelRecording()`; Loslassen **auf** Sperren → gesperrt (→ TAP-Surface 9-15). **Zurückziehen** vom Ziel vor dem Loslassen → kein Auslösen (Undo). **Loslassen ohne Ziel** → `stopAndProcessRecording()` (senden).

**AC6 — Gesperrt = TAP-Surface (9-15).** Nach Loslassen auf Sperren wandelt die Surface in die **TAP-Surface aus Story 9-15** (große tappbare Senden + Abbrechen) — Loslassen sendet dann nicht mehr.

**AC7 — Dock-adaptiv.** Ziele/Anordnung spiegeln je Andock-Position (Ziele wachsen weg von der Dock-Kante / vom Daumen).

**AC8 — Andere Modi/Zustände unberührt.** Tap/Toggle/Auto-Stop/Auto = 9-15-Surface; IDLE/TRANSCRIBING/DONE unverändert.

**Inversion (must-fail gates):**
- Alte Slide-Spur / `.ab-holddock`-Surfaces im HOLD-Zustand sichtbar = review failure.
- Ziel wächst/leuchtet nicht beim Finger-Treffer = review failure (AC3/AC4).
- Auslösung beim Schwellwert-Überschreiten *während der Bewegung* statt beim **Loslassen** = review failure (AC5).
- Farben getauscht = review failure.

## Anchors (binding design source)

- **ADR-0019 Amendment 2026-06-26** (`docs/adr/0019-cross-platform-design-ssot.md`).
- **Bindende Render (SOLL):** `docs/design/overhaul/mockup-mobile-hold-B-refined.html` (Frames `bRest` Ruhe + `bHit` Treffer-Abbrechen) **+** `docs/design/overhaul/mockup-mobile-recording-states.html` (Frame `holdLock` Treffer-Sperren).
- Canon-Fingerprint `bac152993046699c5007612ac916d951`; **supersedet** `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-heldbub`/`.ab-lockchip`.
- Touch/Canvas: `reference_android_bubble_canvas_and_install.md`. Verifikations-Lehre: `feedback_gate4_smoke_needs_behavioral_delta.md`.

## DoD (surface-class)

DEBUG APK builds; JVM-Tests grün; Emulator **strukturelle** Smoke grün (Fenster-Struktur via `scripts/android-smoke.sh` unter `BMAD_CONDUCTOR=1`). **GATE-4 Bewegung/Touch/Visual = echtes Gerät + Live-Mikro (Andis Batch-Gate):** halten/loslassen/wegziehen/hochziehen-sperren, grow-on-target, release-to-commit + Undo, Lesbarkeit — **nur** am echten Gerät verifizierbar, nie am Emulator (kein Motion-/Touch-Orakel). Overlays nie `FLAG_NOT_TOUCHABLE`.

## Dependency

Baut auf **Story 9-15** auf (gesperrt-Zustand = 9-15-TAP-Surface). 9-15 zuerst / zusammen.

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | (frühere Fassung) Slide-Spur-HOLD gebaut (`c389c88`/`e92f4f3`), GATE-4 Maschinen-Ebene grün. | claude-sonnet-4-6 / conductor |
| 2026-06-26 | **GATE-4 Real-Device FAILED** (Andi): zu klein / „Laptop-Feel" → Status `in-progress`, systemischer Mobile-Overlay-Rethink. | claude-opus-4-8 (conductor) |
| 2026-06-26 | **Story NEU GEFASST** in B-Sprache (ADR-0019 Amendment 2026-06-26) gegen Render `mockup-mobile-hold-B-refined.html` + `mockup-mobile-recording-states.html`; alte `.ab-holddock`-Surfaces superseded; hängt an Story 9-15. `ready-for-dev`. Build folgt in frischer Session. | claude-opus-4-8 (conductor) |
