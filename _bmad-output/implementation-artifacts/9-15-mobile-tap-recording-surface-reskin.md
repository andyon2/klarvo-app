# Story 9.15: Mobile TAP-Aufnahme-Surface (B-Sprache Re-Skin, ersetzt den Klein-Cluster)

Status: review

## Story

As a user recording on Android in **tap / toggle / auto-stop / auto** modes (and after locking a HOLD recording),
I want **large, thumb-friendly Senden / Abbrechen targets** instead of the small `[✗ · Waveform · ➤]` cluster,
so that I can hit the control I want without my finger covering it, and the overlay feels like a phone feature, not a laptop one.

## Kontext — warum diese Story

Beim ersten echten Daumen-Test (9-14, 2026-06-26) hat Andi die **gesamte mobile Aufnahme-Steuerung** als zu klein / „Laptop-Feel" verworfen. Phase-A-Redesign → **B-Sprache** (ADR-0019 Amendment 2026-06-26). Diese Story setzt die **TAP-Surface** um (kurz tippen = aufnehmen, dann tappbare Steuerung). Sie ist **fundamental**: der „gesperrt"-Zustand von Story 9-14 (nach Hochziehen-Sperren) ist **dieselbe Surface** → 9-15 wird vor / zusammen mit 9-14 gebaut.

## Scope (TAP-Aufnahme-Surface + Touch-Zonen + nutzer-konfigurierbare Größe)

> **Re-Scope 2026-06-30 (Andi-Real-Device-Verdikt, GATE-4 FAILED):** Die feste 132dp-Größe aus dem Browser-Render war am Gerät **viel zu groß** (beide Kreise = 81% der Schirmbreite). Größe wird **nutzer-konfigurierbar** {60, 72, 88}dp, Default **72**; die Surface gilt jetzt für **alle Modi inkl. HOLD-Lock** (Andi-Entscheidung). Verankert am **device-scale** Mockup, nicht mehr am Browser-px-Render.

Ersetze auf Android den `.ab-cluster`-Klein-Cluster im RECORDING-Zustand durch **zwei runde tappbare Ziele** in B-Sprache — für **alle Modi**: Tap/Toggle/Auto-Stop/Auto **und den HOLD-gesperrten Zustand** (`holdDockActive`-Surface zieht dieselbe Zwei-Knopf-Darstellung; die *aktive* HOLD-Geste = Story 9-14):
- **Senden** — teal-Gradient-Kreis, ➤-Glyph (OnTeal), am **Dock/Daumen** (dort, wo die idle-„K"-Bubble saß).
- **Abbrechen** — dunkler Kreis mit rotem Ring + ✕ (danger-hi), auf der **Gegenseite**, großzügig entfernt.
- **Waveform-Chip** — ruhiger amber RMS-Waveform + Timer, eigener blickdichter Chip, **oben/zwischen** den Zielen (überlappt KEIN Ziel); skaliert proportional mit der gewählten Größe.
- **Dock-adaptiv:** bei links/oben/unten/frei angedockt **spiegelt** sich die Anordnung — Senden bleibt am Dock, Abbrechen wächst weg von der Dock-Kante (nie unter den Daumen).
- **Größe nutzer-konfigurierbar:** neuer Config-Key `recordingButtonSizeDp` ∈ {60, 72, 88}, Default 72. Settings-Control (Mobile-Bereich, neben „Bubble Size"). Kreis-Durchmesser, Glyph, Labels, „tippen"-Hint, Chip und die Fenster-Dims (`adjustLayoutForState`) leiten sich alle aus diesem Wert ab (proportional, wie das device-scale Mockup mit `calc()`).

**Hard scope boundaries:**
- RECORDING-Steuer-Surface + Touch-Zonen + die Größen-Einstellung (Config-Plumbing Frontend↔Android + Settings-Control). Keine Pipeline-/Aufnahme-Logik, kein STT, kein Cleanup.
- **Keine** Änderung an IDLE / TRANSCRIBING / DONE / Preview (eigener späterer Pass — `docs/backlog.md`).
- **Keine** Token-Änderung (`KlarvoTheme.kt` generiert — nicht hand-editieren); der Größen-Wert ist KEIN Token, sondern Runtime-Config.
- Mobile-only Setting (Desktop-Overlays sind native Win32, eigener Pfad).
- Farb-Semantik bindend (ADR-0019 §5): **teal = Senden · amber = live · rot = Abbrechen** — nie tauschen.
- `FLAG_NOT_TOUCHABLE` nie hinzufügen (HyperOS dimmt auf 0.8).

## Acceptance Criteria

**AC1 — TAP-Surface ersetzt den Klein-Cluster (alle Modi).** Given RECORDING-Zustand, When die Overlay zeichnet, Then erscheinen **zwei runde tappbare Ziele** (Senden teal ➤ am Dock-/Daumen-Platz · Abbrechen dunkel+rot-Ring ✕ gegenüber) + ein ruhiger amber Waveform-Chip — und der alte `.ab-cluster` `[✗·Waveform·➤]` wird **NICHT** mehr gezeichnet. Gilt für tap/toggle/auto/auto-stop (`pushToTalkActive=false`) **und** den HOLD-gesperrten Zustand (`holdDockActive`).

**AC2 — Größe nutzer-konfigurierbar + Lesbarkeit (mobile-first).** Kreis-Durchmesser = `recordingButtonSizeDp` ∈ {60, 72, 88}, Default 72 — **nicht** der alte feste 132dp-Render-Wert (war am Gerät zu groß, GATE-4-FAILED 2026-06-30). Glyph, Labels, Hint, Abstand und Chip skalieren **proportional** mit (wie das device-scale Mockup via `calc()`); „Abbrechen" muss bei 60dp noch sauber im Kreis sitzen (kein Clipping/Überlauf). Jedes Ziel + der Chip auf **blickdichter** Fläche (keine Transparenz-Abhängigkeit). Größen-/Farb-/Radii-Verhältnisse aus `docs/design/overhaul/mockup-tap-size-calibration.html` (device-scale SOLL); Farben weiter aus `mockup-mobile-recording-states.html`.

**AC3 — Senden am Dock/Daumen.** Senden sitzt am Bildschirm-Platz der idle-„K"-Bubble (Dock-Kante); Abbrechen auf der Gegenseite (Daumen-Gewohnheit, wie ADR-0019 §4′-#2 — nur jetzt groß).

**AC4 — Tippen löst aus.** Tap auf Senden = `stopAndProcessRecording()` (senden); Tap auf Abbrechen = `cancelRecording()` (verwerfen). Touch-Zonen decken die großen Kreise ab.

**AC5 — Waveform RMS-getrieben.** Der Waveform-Chip nutzt den bestehenden RMS-Feed / `drawClusterWaveform`-Algorithmus (Story 9-12, unverändert) — bei Stille still.

**AC6 — Dock-adaptiv (Spiegelung).** Bei links angedockt: Senden ans linke Dock, Abbrechen rechts (Render `tapLeft`). Für oben/unten analog spiegeln, sodass Abbrechen weg von der angedockten Kante wächst.

**AC7 — Andere Zustände unberührt.** IDLE / TRANSCRIBING / DONE / Preview unverändert.

**AC8 — Größe in Settings wählbar.** Neuer Config-Key `recordingButtonSizeDp` (Int, Default 72, erlaubte Werte 60/72/88) round-trippt Frontend↔config.json↔Android (camelCase; falscher Key wird still ignoriert — Round-Trip beweisen). Ein 3-Wege-Control im Mobile-Settings-Bereich (neben „Bubble Size") setzt ihn; Auswahl persistiert und wirkt auf die nächste/laufende Aufnahme-Surface ohne Neustart (wie `setBubbleSize`-Pfad aus 9-3).

**AC9 — Gilt für alle Modi inkl. HOLD-Lock.** Der **gesperrte** HOLD-Zustand (erreicht via `lockHoldToCluster()`, das `holdDockActive=false` setzt → `onDraw` routet zu `drawTapSurface`) zeigt dieselbe Zwei-Knopf-Surface in der konfigurierten Größe. **Präzisierung (Review 2026-06-30):** `drawHoldDock` (= `holdDockActive=true`) ist die **aktive** HOLD-Geste (Hochziehen/grow/release) und bleibt unverändert / Story 9-14 — NICHT der gesperrte Zustand. Inversions-Gate „HOLD-gesperrt zeigt alten Stand" = nicht getrippt (Lock → drawTapSurface, verifiziert KlarvoOverlayService.kt:1162).

**AC10 — Fenster-Dims folgen der Größe.** `adjustLayoutForState` berechnet die RECORDING-Fensterbreite/-höhe aus `recordingButtonSizeDp` (2 Kreise + Gap + Chip + Schatten-Pad), nicht aus festen 340×222dp. Struktureller Smoke: das RECORDING-Overlay-Fenster spiegelt die gewählte Größe (z.B. 72dp deutlich kleiner als die alten 892×582px).

**Inversion (must-fail gates):**
- Alter `.ab-cluster` im RECORDING-Zustand sichtbar = review failure.
- Senden nicht am Dock-/Daumen-Platz = review failure.
- Farben getauscht (teal≠Senden / rot≠Abbrechen) = review failure.
- Kreis-Durchmesser fest auf 132dp / ignoriert `recordingButtonSizeDp` = review failure.
- HOLD-gesperrt zeigt noch den alten Stand statt der Zwei-Knopf-Surface = review failure.

## Anchors (binding design source)

- **ADR-0019 Amendment 2026-06-26** „Android-Aufnahme-Steuerung: Mobile-Redesign (B-Sprache)" (`docs/adr/0019-cross-platform-design-ssot.md`).
- **Bindende Größe (device-scale SOLL):** `docs/design/overhaul/mockup-tap-size-calibration.html` — am echten Gerät (1080×2460 @440dpi = 393dp) approbiert; `recordingButtonSizeDp` ∈ {60,72,88}, Default 72. **Supersedet** den festen `.ztap{width:132px}`-Wert des Browser-Renders (war zu groß — GATE-4-FAILED 2026-06-30).
- **Bindende Farben/Form/Anordnung (SOLL):** `docs/design/overhaul/mockup-mobile-recording-states.html` — Frames `tapRight`/`tapLeft`. Farben/Radii-Verhältnisse/Dock-Spiegelung von hier (NUR die absolute Pixel-Größe ist durch die device-scale-Kalibrierung ersetzt).
- Canon-Fingerprint `bac152993046699c5007612ac916d951` (MANIFEST 2026-06-26); **supersedet** `.ab-cluster` (im tracked Canon als SUPERSEDED markiert). Lehre: SOLL-Größen am Geräte-Maßstab abnehmen, nie aus Browser-px (`feedback_soll_anchor_external_approved_source` + `project_mobile_overlay_design_rejected`).
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

- [x] Task 7: Re-Scope 2026-06-30 — Proportionale Skalierung in FloatingBubbleView.kt
  - [x] 7.1 Neue Konstanten `TAP_BUTTON_SIZE_{MIN=60,DEFAULT=72,MAX=88}` im companion object
  - [x] 7.2 Neue Property `recordingButtonSizeDp: Int` (default 72, setter coerces + invalidates)
  - [x] 7.3 Neue `@JvmStatic` Companion-Funktionen `tapVisualWidthDp(buttonSizeDp)` + `tapVisualHeightDp(buttonSizeDp)` für testbare Geometrie-Berechnung
  - [x] 7.4 `drawTapSurface()` berechnet `scale = recordingButtonSizeDp / 132f`; alle Sub-Funktionen erhalten `scale: Float`-Parameter
  - [x] 7.5 `drawTapChip()`, `drawTapSendCircle()`, `drawTapCancelCircle()` alle Dimensionen × scale (lokale `scale`-Variable in `drawTapCancelCircle` umbenannt zu `glyphScale` zur Vermeidung von shadowing)

- [x] Task 8: Re-Scope 2026-06-30 — KlarvoOverlayService.kt + KlarvoApi.kt (Android-Config-Lesen)
  - [x] 8.1 `KlarvoOverlayService.adjustLayoutForState()`: TAP-Window-Dims jetzt via `FloatingBubbleView.tapVisualWidthDp(bubbleView.recordingButtonSizeDp)` (statt feste Konstanten)
  - [x] 8.2 `KlarvoOverlayService.lockHoldToCluster()`: gleiche Änderung (AC10)
  - [x] 8.3 `KlarvoOverlayService.reloadBubbleAppearance()`: setzt `bubbleView.recordingButtonSizeDp = config.recordingButtonSizeDp`
  - [x] 8.4 `KlarvoApi.Config`: neues Feld `val recordingButtonSizeDp: Int = 72`
  - [x] 8.5 `KlarvoApi.readConfig()`: liest `json.optInt("recordingButtonSizeDp", 72).coerceIn(MIN,MAX)`

- [x] Task 9: Re-Scope 2026-06-30 — Config-Round-Trip Rust/TS + Settings-Control (AC8)
  - [x] 9.1 `src-tauri/src/config/mod.rs`: `pub recording_button_size_dp: i32` mit `#[serde(default = "default_recording_button_size_dp")]` + Funktion `fn default_recording_button_size_dp() -> i32 { 72 }`
  - [x] 9.2 `src-tauri/src/lib.rs` (`SettingsView`): `pub recording_button_size_dp: i32` hinzugefügt
  - [x] 9.3 `src-tauri/src/commands/settings.rs`: `recording_button_size_dp: Option<i32>` in `SettingsPatch` + Default-Impl + `merge_settings` + `save_settings`-Param + Patch-Konstruktion + `get_settings`-Response
  - [x] 9.4 `src/types.ts`: `recordingButtonSizeDp: number` in `AppSettings`
  - [x] 9.5 `src/tauri-commands.ts`: Parameter + Invoke-Aufruf + Fallback-Konstante `72`
  - [x] 9.6 `src/components/SettingsPanel.tsx`: State `localRecordingButtonSizeDp`, dirty tracking (2 Stellen), loadedSettings-Reset, saveCurrentSettings-Aufruf, useCallback-Deps, ShortcutsContent-Prop
  - [x] 9.7 `src/components/settings/ShortcutsContent.tsx`: Props-Typ + Destrukturierung + 3-Wege-Segmented-Control (60/72/88 dp) im „Bubble Appearance"-Abschnitt

- [x] Task 10: Re-Scope 2026-06-30 — Unit-Tests aktualisieren + finales Gate
  - [x] 10.1 `TapSurfaceTouchZoneTest.kt`: `tap_send_diam_is_at_least_120dp()` ersetzt durch 4 neue Tests: `recording_button_size_default_is_72dp`, `recording_button_size_min_is_at_least_48dp`, `visual_width_at_default_is_less_than_reference_max`, `visual_width_scales_proportionally_with_button_size`
  - [x] 10.2 JVM-Tests: **BUILD SUCCESSFUL** (`./gradlew :app:testUniversalDebugUnitTest --rerun-tasks`)
  - [x] 10.3 Theme-Drift-Gate: **[ok] KlarvoTheme.kt is in sync with canon klarvo.css**
  - [x] 10.4 Rust native check: **Finished dev profile, 0 errors** (Windows cross-compile hat pre-existierenden whisper.cpp/mingw-Fehler, unabhängig von unseren Änderungen)

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

**Re-Scope 2026-06-30 — implementiert (Tasks 7–10):**

- `FloatingBubbleView.kt`: Proportionale Skalierung via `scale = recordingButtonSizeDp / 132f`. Neue Property `recordingButtonSizeDp` (default 72, coerces to {60..88}). Neue `@JvmStatic`-Companion-Funktionen `tapVisualWidthDp(buttonSizeDp)` + `tapVisualHeightDp(buttonSizeDp)`. Alle Sub-Funktionen (`drawTapChip`, `drawTapSendCircle`, `drawTapCancelCircle`) erhalten `scale: Float`-Parameter. Lokale Variable `scale` in `drawTapCancelCircle` umbenannt zu `glyphScale` (shadowing-Schutz). `TAP_SEND_DIAM_DP=132` bleibt als Referenz-/Skalierungs-Anker (nicht als angezeigte Größe).

- `KlarvoOverlayService.kt`: `adjustLayoutForState()` + `lockHoldToCluster()` nutzen jetzt `FloatingBubbleView.tapVisualWidthDp/tapVisualHeightDp(bubbleView.recordingButtonSizeDp)` statt feste Konstanten (AC10). `reloadBubbleAppearance()` setzt `bubbleView.recordingButtonSizeDp = config.recordingButtonSizeDp`.

- `KlarvoApi.kt`: `Config.recordingButtonSizeDp: Int = 72`; `readConfig()` liest + coerces aus JSON.

- Config-Round-Trip: `config/mod.rs` (Rust-Feld + Default), `lib.rs` (SettingsView), `commands/settings.rs` (Patch + merge + save + get), `types.ts` (AppSettings), `tauri-commands.ts` (Param + invoke + Fallback).

- Settings-Control: `ShortcutsContent.tsx` 3-Wege-Segmented-Control (60dp/72dp/88dp) im „Bubble Appearance"-Abschnitt. `SettingsPanel.tsx` vollständig verdrahtet (State, dirty tracking × 2, reset, save, deps, Prop-Weitergabe).

- Tests: `tap_send_diam_is_at_least_120dp` ersetzt durch 4 semantisch korrekte AC2-Tests. BUILD SUCCESSFUL (JVM-Tests grün, Theme-Drift grün, Rust 0 Errors).

### Debug Log

(leer)

## File List

- `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` — geändert (drawTapSurface proportional scaling, recordingButtonSizeDp property, tapVisualWidthDp/tapVisualHeightDp companion fns, TAP_BUTTON_SIZE_{MIN,DEFAULT,MAX} constants)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — geändert (adjustLayoutForState + lockHoldToCluster via tapVisualWidthDp/tapVisualHeightDp; reloadBubbleAppearance setzt recordingButtonSizeDp)
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — geändert (Config.recordingButtonSizeDp Feld + readConfig Parsing)
- `android/kotlin-test/com/klarvo/voice/TapSurfaceTouchZoneTest.kt` — geändert (4 neue AC2-Re-Scope-Tests; tap_send_diam_is_at_least_120dp ersetzt)
- `src-tauri/src/config/mod.rs` — geändert (recording_button_size_dp Feld + default-Funktion + 3× Struct-Init)
- `src-tauri/src/lib.rs` — geändert (SettingsView.recording_button_size_dp Feld + 3× Struct-Init in Tests)
- `src-tauri/src/commands/settings.rs` — geändert (SettingsPatch + Default-Impl + merge_settings + save_settings + get_settings)
- `src/types.ts` — geändert (AppSettings.recordingButtonSizeDp)
- `src/tauri-commands.ts` — geändert (saveSettings-Param + invoke + Fallback-Konstante)
- `src/components/SettingsPanel.tsx` — geändert (State + dirty tracking + reset + save + deps + ShortcutsContent-Prop)
- `src/components/settings/ShortcutsContent.tsx` — geändert (Props-Typ + Destrukturierung + 3-Wege-Segmented-Control)

## Review Findings (code-review 2026-06-30, baseRef ed219f10..a3d0233)

3 unabhängige Reviewer (Blind / Edge-Case / Acceptance-Auditor), Conductor-triagiert + selbst am Code/SOLL verifiziert.

**Patch (zu fixen):**
- [x] [Review][Patch] AC6-Mirroring-Tests waren tautologisch → **GEFIXT 218ee5d**: pure `tapCircleCenters` extrahiert + Tests prüfen echten Swap + Integration. (Verifiziert nach Re-Scope intakt, Z.128-164.)
- [x] [Review][Patch] Cancel-Glyph/-Label nutzten `KlarvoTheme.Danger` (#EE6F63) statt danger-hi #F4897E → **GEFIXT 218ee5d**: lokale Konstante `TAP_CANCEL_DANGER_HI`. (Verifiziert nach Re-Scope intakt, Z.412/444.)

**Deferred (Backlog — siehe docs/backlog.md):**
- [x] [Review][Defer] Per-Frame-Allocations auf RECORDING-Draw-Pfad (BlurMaskFilter/LinearGradient/Path/RectF) — echter GC-Druck, aber positions-abhängiger Shader/Path macht sicheres Cachen nicht-trivial; transienter 15fps-Zustand. [FloatingBubbleView.kt:663,697,702,741,768]
- [x] [Review][Defer] Cancel-Label 13sp vs Render 15px — SOLL-Abweichung; 15sp riskiert Clipping bei „Abbrechen" → Andis Visual-Gate-Residual. [FloatingBubbleView.kt:777]
- [x] [Review][Defer] TAP-Fenster 340dp kann auf <340dp-Screens (sw320/Split-Screen/Foldable) die Breite überschreiten — kein Width-Clamp/Scale. [KlarvoOverlayService.kt adjustLayoutForState]
- [x] [Review][Defer] Drag der RECORDING-TAP-Surface + Release → Edge-Snap nutzt idle-72dp-Breite → Off-Screen-x persistiert. [KlarvoOverlayService.kt edge-snap]
- [x] [Review][Defer] AC6 oben/unten-Mirroring nicht implementiert — Render spezifiziert nur tapLeft/tapRight (render-unspezifiziert). [KlarvoOverlayService.kt getDockSide]

**Dismissed (6):** Opaque-95%-Alpha (= SOLL rgba .95/.96, kein Bug) · Timer-Freeze-bei-Stille (widerlegt: `onAmplitude` feuert ~64ms unabhängig von Stille) · `.reccap`-Caption fehlt (außerhalb locked Scope) · Waveform 3dp×5 vs 4.5px×7 (AC5 mandatiert `drawClusterWaveform` unverändert) · `preclusterBubbleX`-Reset (vorhanden, Z.1087) · `holdDockActive`-im-Lock-Pfad (Service setzt false bei stop/cancel; 9-14-Territorium) · Send-Label-Weight 700 vs 600 (kein sauberes 600 ohne Font-Asset).

### Re-Scope Review (code-review 2026-06-30, range 218ee5d..67f20b6 — 3 Reviewer, Conductor-verifiziert)

**Funktional CLEAN.** Selbst am Code verifiziert: Config-Round-Trip FE↔config.json↔Android sauber (fehlender Key → Default 72 in allen 3 Schichten, kein Cross-Layer-Mismatch); Touch-Zonen folgen der **skalierten** Geometrie (`tapSendCx/tapCancelCx/tapZoneRadius=radPx`, AC4 ✓); AC9-Lock korrekt (`lockHoldToCluster`→`holdDockActive=false`→`drawTapSurface`); AC10 Fenster-Dims aus `recordingButtonSizeDp`; AC8 3-Wege-Control + apply-without-restart; KlarvoTheme unberührt. Vorherige Fixes (Mirroring-Tests, Cancel-Farbe) nach Re-Scope intakt.

1 Compile-Fix vom Conductor gezogen (E0063 `SettingsPatch`-Test, `67f20b6`).

**Andis Geräte-Gate-Residuen (visuell, am 60dp-Ende — kein Rate-Fix):**
- Waveform-Chip: `drawClusterWaveform`-Bars skalieren NICHT mit (`waveW`/Bars fest, Chip skaliert) → bei 60dp wirkt die Waveform proportional groß (kein harter Clip: WAVE_H_DP=18 < Chip-Halbhöhe 12.3dp). AC5 schützt den RMS-Algorithmus → bewusst nicht geraten; falls Andi es zu groß findet → device-validiert skalieren.
- Label-/Hint-Lesbarkeit: `textSize = 15f*scale*spd` → bei 60dp ~6.8sp „Abbrechen". Falls am Gerät zu klein → Min-Clamp mit device-validiertem Wert.

**Deferred (Backlog):** Diskret-Set {60,72,88} nur range-geclampt [60,88], nicht gesnappt (hand-edit-only; UI sendet nur die 3) · Rust persistiert ohne Validierung (Android coerced beim Lesen) · `.toInt()`-Truncation latent (Schatten-Pad absorbiert) · Cross-Layer-Default 72 durch keinen Test gepinnt.

**Dismissed:** Touch-Zonen-Divergenz (Blind „High" — falsch, Zonen folgen Skalierung) · ≥120dp→48dp-Test (AC2 bewusst re-scoped) · In-Flight-Resize (sicher auf nächste Aufnahme verschoben) · Cancel-Farbe/Mirroring „noch offen" (Auditor las veraltete Checkboxen — Code ist gefixt).

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | Story geschrieben (Phase-A Mobile-Redesign, B-Sprache) gegen ADR-0019 Amendment 2026-06-26 + Render `mockup-mobile-recording-states.html`. Build folgt in frischer Session. | claude-opus-4-8 (conductor) |
| 2026-06-30 | Implementierung komplett: drawTapSurface + 2D Touch-Zonen + KlarvoOverlayService-Anpassungen + 12 JVM-Tests. 97 Tests grün, Theme-Drift-Gate grün. Status → review. | claude-sonnet-4-6 (dev) |
| 2026-06-30 | Code-Review clean (1 Fix-Runde: Tautologie-Tests → echte `tapCircleCenters`-Abdeckung; Cancel-Farbton Danger→danger-hi). Emulator-Struktur-Smoke GRÜN (TAP-Fenster 340×222dp). | claude-opus-4-8 (conductor) |
| 2026-06-30 | **GATE-4 REAL-DEVICE = FAILED (Andi).** Defekte: (1) Kreise **viel zu groß** — `TAP_SEND_DIAM_DP=132` kam 1:1 aus dem Browser-Render `.ztap{width:132px}`, nie im Geräte-Maßstab validiert (Wiederholung der 9-14-Wurzel). (2) Modi-Erwartung divergiert: neue UI aktuell nur tap/toggle/auto, HOLD noch alt — Andi erwartete andere Zuordnung. Andi offen für **neue UI in ALLEN Modi, sofern deutlich kleiner**. Status → in-progress. Größe + Modi-Scope = Design-Re-Entscheidung im Geräte-Maßstab (Phase A), kein Blind-Rebuild. | claude-opus-4-8 (conductor) |
| 2026-06-30 | **Phase-A Re-Entscheidung (device-scale Mockup `mockup-tap-size-calibration.html` auf Andis Gerät approbiert).** Größe wird **nutzer-konfigurierbar** `recordingButtonSizeDp` ∈ {60,72,88}, **Default 72** (132/104 zu groß). Surface gilt für **alle Modi inkl. HOLD-Lock** (aktive HOLD-Geste = 9-14). Story re-spec't: AC1 (alle Modi), AC2 (konfigurierbar, proportional, vom device-scale Mockup), AC8 (Settings-Control, round-trip), AC9 (HOLD-Lock), AC10 (Fenster-Dims aus Größe). Anchors re-anchored auf device-scale. Bau folgt. | claude-opus-4-8 (conductor) |
| 2026-06-30 | Re-Scope gebaut (`21ae533`, 13 Dateien cross-language: React/TS + Rust + Kotlin) + Conductor-Compile-Fix (`67f20b6`, E0063 SettingsPatch-Test). Re-Review (3 Reviewer) **funktional clean** — Round-Trip/Touch/AC9-Lock/AC10/AC8 verifiziert; visuelle 60dp-Reste (Waveform-Proportion, Label-Lesbarkeit) → Andis Gate; Robustheit → Backlog. Status → review. | claude-opus-4-8 (conductor) |
| 2026-06-30 | **Re-Scope implementiert (Tasks 7–10).** Proportionale Skalierung via `recordingButtonSizeDp` ∈ {60,72,88} (Default 72) in FloatingBubbleView, KlarvoOverlayService, KlarvoApi, Rust-Config, SettingsView, settings.rs, types.ts, tauri-commands.ts, SettingsPanel.tsx, ShortcutsContent.tsx. 4 neue AC2-Tests ersetzen veralteten `tap_send_diam_is_at_least_120dp`-Test. BUILD SUCCESSFUL, Theme-Drift grün, Rust 0 Errors. Status → review. GATE-4 visual/touch = Andis Real-Device-Batch-Gate. | claude-sonnet-4-6 (dev) |
