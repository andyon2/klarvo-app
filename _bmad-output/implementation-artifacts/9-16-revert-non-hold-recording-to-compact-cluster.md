# Story 9-16 — non-HOLD-Aufnahme zurück zum Kompakt-Cluster

**Status:** done (JVM/Build grün; GATE-4 Andi device-verified „passt" 2026-07-01)
**Epic:** 9 (Android Visual Overhaul) · **Branch:** `conductor/epic-9`
**Trigger:** Andi-Feedback nach 9-14/9-15-Geräteabnahme (2026-07-01): die großen TAP-Ziele mit Text gefallen für non-HOLD nicht → zurück zur Vorgänger-UI (kleine Symbole ohne Text, links/rechts der Waveform).

## Entscheidung (Design-Gate = Andi, gehomed)

Siehe **ADR-0019 Amendment 2026-07-01 #2**. non-HOLD-Aufnahme = Kompakt-Cluster `[✗ · amber Waveform · ➤]` (kleine Symbole ohne Text, fest 150×52dp, 1-D-Touch, Abbrechen LINKS / Senden RECHTS). HOLD (9-14) unverändert. Größen-Regler wirkt nur noch auf HOLD (Andi-Entscheidung).

## Acceptance Criteria

- **AC1** — Im RECORDING-State (non-HOLD) zeigt das Overlay den Kompakt-Cluster: kleine ✗ (links) + amber Waveform (mitte) + kleines ➤ (rechts), **keine** Textlabels. (`drawRecordingCluster` via onDraw-Dispatch.)
- **AC2** — HOLD-Modus (`holdDockActive=true`) bleibt unverändert die 9-14-Surface (`drawHoldTargets`).
- **AC3** — Tippen rechts (Waveform-rechts-Band) = Senden; tippen links = Abbrechen; Waveform-/Zwischenbereich = no-op. (1-D-X-Band-Zonen.)
- **AC4** — Aufnahme-Fenster hat wieder Cluster-Größe (150+2×8 × 52+2×8 dp), nicht die große TAP-Fenstergröße.
- **AC5** — Größen-Regler (`recordingButtonSizeDp`, Settings): non-HOLD-Cluster ist fix (unbeeinflusst); der Regler skaliert weiterhin den HOLD-Abbrechen-Button.
- **AC6** — Waveform (RMS-reaktiv, 9-12) unverändert.

## Implementierung

SSOT: `android/kotlin-src/com/klarvo/voice/` (gen/android beim Build gesynced).
- `FloatingBubbleView.kt`: onDraw-Dispatch non-HOLD → `drawRecordingCluster` (war toter Code seit 9-15); `isTouchInConfirmZone/isTouchInCancelZone` zurück auf 1-D-Cluster-Zonen (`clusterSendZoneStart`/`clusterCancelZoneEnd`); `drawTapSurface` + 2D-Kreis-Felder jetzt toter Code (behalten, symmetrisch); Header-/Feld-Kommentare invertiert.
- `KlarvoOverlayService.kt`: `adjustLayoutForState` non-HOLD-Branch → Cluster-Fenstergröße; `handleTap` 1-arg-Aufrufe.
- Tests: `TapSurfaceTouchZoneTest` bleibt grün (prüft `isInsideCircle`/Button-Size-Konstanten; Header ehrlich als „TAP tot, Button-Size steuert HOLD" annotiert).

**Reversibel:** Der Flip ist eine Dispatch-Zeile + Touch/Fenster; `drawTapSurface` liegt als toter Code bereit.

## DoD (surface-class)

DEBUG APK build + JVM-Unit-Tests grün — **verifiziert 2026-07-01** (`gradlew :app:testUniversalDebugUnitTest` BUILD SUCCESSFUL, `compileUniversalDebugKotlin` grün, keine neuen Warnings außer vorbestehend `scaledDensity`). **GATE-4 (Sicht/Touch am echten Gerät) = Andis Runde** via `scripts/android-smoke.sh` (Shortcut „Klarvo Android Smoke"): Cluster erscheint klein, Symbole ohne Text, tippen links/rechts = abbrechen/senden. Overlays nie `FLAG_NOT_TOUCHABLE`.

## Anchors

- **ADR-0019 Amendment 2026-07-01 #2** (`docs/adr/0019-cross-platform-design-ssot.md`).
- Bindende Vorgänger-Quelle: `.ab-cluster` @ git `e92f4f3` (pre-9-15).
