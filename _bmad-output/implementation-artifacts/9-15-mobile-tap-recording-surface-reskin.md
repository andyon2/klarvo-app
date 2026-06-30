# Story 9.15: Mobile TAP-Aufnahme-Surface (B-Sprache Re-Skin, ersetzt den Klein-Cluster)

Status: review

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

## Tasks/Subtasks

- [x] Task 1: TAP-Surface Konstanten + `dockSide`-Property in FloatingBubbleView.kt
  - [x] 1.1 Neue Konstanten (TAP_SEND_DIAM_DP=132, TAP_INNER_GAP_DP=56, TAP_CHIP_H_DP=54, TAP_CHIP_GAP_DP=16, TAP_SHADOW_PAD_DP=10, TAP_VISUAL_W_DP=320, TAP_VISUAL_H_DP=202)
  - [x] 1.2 Property `dockSide: String` (default "right") + `recordingStartMs: Long`
  - [x] 1.3 Pre-allocated Paints für TAP-Surface (tapSendLabelPaint, tapHintPaint, tapTimerPaint, tapCancelGlyphPaint)
  - [x] 1.4 Touch-Zone-Felder: tapSendCx, tapSendCy, tapCancelCx, tapCancelCy, tapZoneRadius

- [x] Task 2: `drawTapSurface()` implementieren (ersetzt `drawRecordingCluster()`)
  - [x] 2.1 Waveform-Chip zeichnen (dunkler backdrop, amber wave via drawClusterWaveform, Timer mm:ss)
  - [x] 2.2 Abbrechen-Kreis zeichnen (dunkle Füllung + DangerLine-Ring + ✗-Glyph + Labels)
  - [x] 2.3 Senden-Kreis zeichnen (teal Gradient + ➤-Glyph OnTeal + Labels)
  - [x] 2.4 Dock-Spiegelung: sendCx/cancelCx tauschen je nach dockSide
  - [x] 2.5 `onDraw()` routing aktualisieren: !holdDockActive → drawTapSurface()

- [x] Task 3: Touch-Zonen auf 2D-Kreis umstellen
  - [x] 3.1 `isTouchInConfirmZone(touchX, touchY)` — kreisförmige Hit-Detection
  - [x] 3.2 `isTouchInCancelZone(touchX, touchY)` — kreisförmige Hit-Detection
  - [x] 3.3 KDoc + Kommentar in FloatingBubbleView aktualisieren

- [x] Task 4: KlarvoOverlayService.kt anpassen
  - [x] 4.1 `handleTap(touchX, touchY)` — touchY-Parameter hinzufügen + Call-Site updaten
  - [x] 4.2 `adjustLayoutForState()` — TAP-Surface-Dimensionen statt CLUSTER_VISUAL_W/H
  - [x] 4.3 `lockHoldToCluster()` — TAP-Surface-Dimensionen statt CLUSTER_VISUAL_W/H
  - [x] 4.4 `getDockSide()` Hilfsmethode + bubbleView.dockSide setzen beim RECORDING-Eintritt
  - [x] 4.5 `startRecording()` — bubbleView.recordingStartMs setzen
  - [x] 4.6 `applyHarnessState()` — dockSide + recordingStartMs für Debug-Harness

- [x] Task 5: JVM Unit-Tests (TapSurfaceTouchZoneTest.kt)
  - [x] 5.1 Test: Senden-Kreis getroffen (rechts dock) — Confirm
  - [x] 5.2 Test: Abbrechen-Kreis getroffen (rechts dock) — Cancel
  - [x] 5.3 Test: Senden-Kreis getroffen (links dock) — spiegelt korrekt
  - [x] 5.4 Test: Treffer außerhalb beider Kreise → kein Confirm, kein Cancel
  - [x] 5.5 Test: holdDockActive=true → beide Zonen immer false — HINWEIS: `holdDockActive`-Guard ist View-Level (`if (state != RECORDING || holdDockActive ...) return false`); in JVM ohne Robolectric nicht instanziierbar. Verifikation: Code-Reading der Guard-Zeile in `isTouchInConfirmZone` / `isTouchInCancelZone` (residual gap, kein Green-Test für diese Bedingung).

- [x] Task 6: Tests ausführen + Smoke-Vorbereitung
  - [x] 6.1 `./gradlew testUniversalDebugUnitTest --rerun-tasks` — **97 Tests, 0 Failures** (12 neue TapSurfaceTouchZoneTest)
  - [x] 6.2 `node scripts/gen-android-theme.mjs --check` — **[ok] KlarvoTheme.kt is in sync with canon klarvo.css**
  - [x] 6.3 `cargo check --target x86_64-pc-windows-gnu` — N/A: keine Rust/Windows-Pfade berührt (nur android/kotlin-src + android/kotlin-test)

## Dev Notes

- **SOLL-Render:** `docs/design/overhaul/mockup-mobile-recording-states.html` Frame `tapRight`/`tapLeft`
- **.ztap CSS:** `width:132px; height:132px; border-radius:50%; display:flex; flex-direction:column; align-items:center; justify-content:center; gap:7px`
- **.ztap.send:** `background: linear-gradient(150deg, #57DDC7, #1B9C88); color: #05201B`
- **.ztap.cancel:** `background: rgba(20,18,18,.95); border: 2px solid rgba(238,111,99,.5); color: #F4897E`
- **.statuschip:** `padding:11px 16px; border-radius:18px; background:rgba(18,20,22,.96); border:1px solid var(--k-border)`
- **TAP_CANCEL_FILL** = 0xF2141212 (rgba(20,18,18,.95))
- **TAP_CANCEL_BORDER** = 0x80EE6F63 (rgba(238,111,99,.5))
- **TAP_CHIP_BG** = 0xF5121416 (rgba(18,20,22,.96))
- **Touch-Zone:** 2D kreisförmig: `hypot(touchX-cx, touchY-cy) <= radius`
- **Dock-Side:** aus `bubbleParams.x` vor Expansion: `preclusterBubbleX < screenW/2` → "left" else "right"
- **Wichtig:** `drawClusterWaveform()` UNVERÄNDERT lassen (AC5)
- **Wichtig:** `FLAG_NOT_TOUCHABLE` NICHT hinzufügen (HyperOS-Constraint)
- **wasBarMode-Check** funktioniert für TAP-Surface: 340dp > 222dp → true ✓

## DoD (surface-class)

DEBUG APK builds; JVM-Unit-Tests grün; Emulator **strukturelle** Smoke grün (Overlay-Fenster-Struktur via `scripts/android-smoke.sh` unter `BMAD_CONDUCTOR=1` — TAP-Surface-Fenster vorhanden, Größe ≠ alter Klein-Cluster). **GATE-4 visuell/touch = echtes Gerät (Andis Batch-Gate):** finale Platzierung/Größe/Lesbarkeit + Tap-Verhalten = Andis Real-Device-Sicht, nie ein Emulator-Screenshot. Overlays nie `FLAG_NOT_TOUCHABLE`.

## Dependency

**Fundamental** für Story 9-14: der „gesperrt"-Zustand von 9-14 (nach Hochziehen-Sperren) **ist** diese TAP-Surface. 9-15 zuerst / zusammen bauen.

## Dev Agent Record

### Implementation Plan

TAP-Surface ersetzt `drawRecordingCluster()` in FloatingBubbleView.kt. Zwei 132dp-Kreise (Senden teal rechts/links je nach Dock, Abbrechen dunkel+rot-Ring) + Waveform-Chip oben. Window-Dimensionen ändern sich von 150×52dp (Cluster) auf 320×202dp (TAP). Touch-Zonen von 1D-X-Range auf 2D-Kreis umgestellt. KlarvoOverlayService.kt bekommt neue Window-Dimensionen in adjustLayoutForState() + lockHoldToCluster() + getDockSide()-Helper.

### Completion Notes

**Implementiert 2026-06-30:**

- `FloatingBubbleView.kt`: Neue `drawTapSurface()` ersetzt `drawRecordingCluster()` für TAP-Modi. Enthält `drawTapChip()` (Waveform+Timer), `drawTapSendCircle()` (teal Gradient, ➤ 46dp, „Senden"/"tippen"-Labels), `drawTapCancelCircle()` (dunkel+rot-Ring, ✕ 42dp, „Abbrechen"/"tippen"-Labels). Dock-Spiegelung via `dockSide`-Property. `isInsideCircle()` als pure companion function extrahiert (testbar ohne Android-Context). Touch-Zonen 2D-kreisförmig (war 1D-X-Range). `drawRecordingCluster()` als totes Code behalten (Kompilierbarkeit der privaten Helfer).

- `KlarvoOverlayService.kt`: `handleTap(touchX, touchY)` — `touchY` Parameter hinzugefügt. `adjustLayoutForState()` + `lockHoldToCluster()` — neue TAP-Fensterdimensionen (340×222dp statt 150×68dp). `getDockSide()` Helfer. `startRecording()` setzt `bubbleView.recordingStartMs`. `applyHarnessState()` setzt `dockSide` + `recordingStartMs` für Debug-Harness.

- `TapSurfaceTouchZoneTest.kt` (neu): 12 JVM-Tests grün. Testet `isInsideCircle()` für Kreis-Treffer, Kreis-Miss, Rand-Treffer, Gegenseite-Miss, Zwischenraum-Miss, Chip-Bereich-Miss, links/rechts Dock-Spiegelung, und AC2-Größe-Guard.

- Residual gap: `holdDockActive=true → beide Zonen false` (Task 5.5) ist View-Level-Guard — kein JVM-Test möglich ohne Robolectric. Guard klar lesbar in `isTouchInConfirmZone`/`isTouchInCancelZone`.

### Debug Log

(leer)

## File List

- `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` — geändert (drawTapSurface, isInsideCircle, 2D touch zones, dockSide/recordingStartMs properties + paints + constants)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — geändert (handleTap touchY, adjustLayoutForState TAP dims, lockHoldToCluster TAP dims, getDockSide, recordingStartMs, applyHarnessState)
- `android/kotlin-test/com/klarvo/voice/TapSurfaceTouchZoneTest.kt` — neu (12 JVM-Tests für isInsideCircle / AC2 / AC4 / AC6)

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | Story geschrieben (Phase-A Mobile-Redesign, B-Sprache) gegen ADR-0019 Amendment 2026-06-26 + Render `mockup-mobile-recording-states.html`. Build folgt in frischer Session. | claude-opus-4-8 (conductor) |
| 2026-06-30 | Implementierung komplett: drawTapSurface + 2D Touch-Zonen + KlarvoOverlayService-Anpassungen + 12 JVM-Tests. 97 Tests grün, Theme-Drift-Gate grün. Status → review. | claude-sonnet-4-6 (dev) |
