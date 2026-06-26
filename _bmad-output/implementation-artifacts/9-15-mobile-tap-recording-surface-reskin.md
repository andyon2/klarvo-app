# Story 9.15: Mobile TAP-Aufnahme-Surface (B-Sprache Re-Skin, ersetzt den Klein-Cluster)

Status: ready-for-dev

## Story

As a user recording on Android in **tap / toggle / auto-stop / auto** modes (and after locking a HOLD recording),
I want **large, thumb-friendly Senden / Abbrechen targets** instead of the small `[✗ · Waveform · ➤]` cluster,
so that I can hit the control I want without my finger covering it, and the overlay feels like a phone feature, not a laptop one.

## Kontext — warum diese Story

Beim ersten echten Daumen-Test (9-14, 2026-06-26) hat Andi die **gesamte mobile Aufnahme-Steuerung** als zu klein / „Laptop-Feel" verworfen. Phase-A-Redesign → **B-Sprache** (ADR-0019 Amendment 2026-06-26). Diese Story setzt die **TAP-Surface** um (kurz tippen = aufnehmen, dann tappbare Steuerung). Sie ist **fundamental**: der „gesperrt"-Zustand von Story 9-14 (nach Hochziehen-Sperren) ist **dieselbe Surface** → 9-15 wird vor / zusammen mit 9-14 gebaut.

## Scope (locked — nur die TAP-Aufnahme-Surface + ihre Touch-Zonen)

Ersetze auf Android den `.ab-cluster`-Klein-Cluster im RECORDING-Zustand (für die Gesten-Modi **Tap/Toggle/Auto-Stop/Auto**) durch **zwei große runde tappbare Ziele** in B-Sprache:
- **Senden** — teal-Gradient-Kreis, ➤-Glyph (OnTeal), am **Dock/Daumen** (dort, wo die idle-„K"-Bubble saß).
- **Abbrechen** — dunkler Kreis mit rotem Ring + ✕ (danger), auf der **Gegenseite**, großzügig entfernt.
- **Waveform-Chip** — ruhiger amber RMS-Waveform + Timer, eigener blickdichter Chip, **oben/zwischen** den Zielen (überlappt KEIN Ziel).
- **Dock-adaptiv:** bei links/oben/unten/frei angedockt **spiegelt** sich die Anordnung — Senden bleibt am Dock, Abbrechen wächst weg von der Dock-Kante (nie unter den Daumen).

**Hard scope boundaries:**
- **Nur** der RECORDING-Steuer-Surface + dessen Touch-Zonen. Keine Pipeline-/Aufnahme-Logik, kein STT, kein Cleanup.
- **Keine** Änderung an IDLE / TRANSCRIBING / DONE / Preview (eigener späterer Pass — `docs/backlog.md`).
- **Keine** Token-Änderung (`KlarvoTheme.kt` generiert — nicht hand-editieren).
- Farb-Semantik bindend (ADR-0019 §5): **teal = Senden · amber = live · rot = Abbrechen** — nie tauschen.
- `FLAG_NOT_TOUCHABLE` nie hinzufügen (HyperOS dimmt auf 0.8).

## Acceptance Criteria

**AC1 — TAP-Surface ersetzt den Klein-Cluster.** Given RECORDING-Zustand mit `pushToTalkActive = false` (tap/toggle/auto), When die Overlay zeichnet, Then erscheinen **zwei große runde tappbare Ziele** (Senden teal ➤ am Dock-/Daumen-Platz · Abbrechen dunkel+rot-Ring ✕ gegenüber) + ein ruhiger amber Waveform-Chip — und der alte `.ab-cluster` `[✗·Waveform·➤]` wird **NICHT** mehr gezeichnet.

**AC2 — Größe & Lesbarkeit (mobile-first).** Ziel-Durchmesser ≥ ~120dp; großzügiger Abstand zwischen den Zielen; jedes Ziel + der Chip auf **blickdichter** Fläche (keine Transparenz-Abhängigkeit), lesbar über beliebigem Hintergrund. Werte exakt aus dem Render `mockup-mobile-recording-states.html` (Frame `tapRight`).

**AC3 — Senden am Dock/Daumen.** Senden sitzt am Bildschirm-Platz der idle-„K"-Bubble (Dock-Kante); Abbrechen auf der Gegenseite (Daumen-Gewohnheit, wie ADR-0019 §4′-#2 — nur jetzt groß).

**AC4 — Tippen löst aus.** Tap auf Senden = `stopAndProcessRecording()` (senden); Tap auf Abbrechen = `cancelRecording()` (verwerfen). Touch-Zonen decken die großen Kreise ab.

**AC5 — Waveform RMS-getrieben.** Der Waveform-Chip nutzt den bestehenden RMS-Feed / `drawClusterWaveform`-Algorithmus (Story 9-12, unverändert) — bei Stille still.

**AC6 — Dock-adaptiv (Spiegelung).** Bei links angedockt: Senden ans linke Dock, Abbrechen rechts (Render `tapLeft`). Für oben/unten analog spiegeln, sodass Abbrechen weg von der angedockten Kante wächst.

**AC7 — Andere Zustände unberührt.** IDLE / TRANSCRIBING / DONE / Preview unverändert.

**Inversion (must-fail gates):**
- Alter `.ab-cluster` im RECORDING-Zustand sichtbar = review failure.
- Senden nicht am Dock-/Daumen-Platz = review failure.
- Farben getauscht (teal≠Senden / rot≠Abbrechen) = review failure.

## Anchors (binding design source)

- **ADR-0019 Amendment 2026-06-26** „Android-Aufnahme-Steuerung: Mobile-Redesign (B-Sprache)" (`docs/adr/0019-cross-platform-design-ssot.md`).
- **Bindendes Render (SOLL):** `docs/design/overhaul/mockup-mobile-recording-states.html` — Frames `tapRight` (rechts angedockt) + `tapLeft` (Dock-Spiegelung). Exakte Farben/Radii/Größen = die CSS dieser Datei.
- Canon-Fingerprint `bac152993046699c5007612ac916d951` (MANIFEST 2026-06-26); **supersedet** `.ab-cluster` (im tracked Canon als SUPERSEDED markiert).
- Touch-/Canvas-Lehren: `reference_android_bubble_canvas_and_install.md`; Cluster-Anker/Daumen-Logik: Story 9-13.

## DoD (surface-class)

DEBUG APK builds; JVM-Unit-Tests grün; Emulator **strukturelle** Smoke grün (Overlay-Fenster-Struktur via `scripts/android-smoke.sh` unter `BMAD_CONDUCTOR=1` — TAP-Surface-Fenster vorhanden, Größe ≠ alter Klein-Cluster). **GATE-4 visuell/touch = echtes Gerät (Andis Batch-Gate):** finale Platzierung/Größe/Lesbarkeit + Tap-Verhalten = Andis Real-Device-Sicht, nie ein Emulator-Screenshot. Overlays nie `FLAG_NOT_TOUCHABLE`.

## Dependency

**Fundamental** für Story 9-14: der „gesperrt"-Zustand von 9-14 (nach Hochziehen-Sperren) **ist** diese TAP-Surface. 9-15 zuerst / zusammen bauen.

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | Story geschrieben (Phase-A Mobile-Redesign, B-Sprache) gegen ADR-0019 Amendment 2026-06-26 + Render `mockup-mobile-recording-states.html`. Build folgt in frischer Session. | claude-opus-4-8 (conductor) |
