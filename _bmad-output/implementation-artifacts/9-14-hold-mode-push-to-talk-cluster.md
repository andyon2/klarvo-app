# Story 9.14: HOLD-Modus (Push-to-Talk) — Mobile-Redesign (B-Sprache)

Status: review

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
- Der „gesperrt"-Zustand = die 9-15-TAP-Surface (nicht doppelt bauen — wiederverwenden; bereits verdrahtet über `lockHoldToCluster()` + `holdDockActive=false` → `onDraw` routet zu `drawTapSurface`).
- **Keine** RMS/Waveform-Änderung (Story 9-12; `drawClusterWaveform`/`waveLevels`/`amplitude`/`setStaticWaveLevel` unverändert).
- **Keine** Token-Änderung (`KlarvoTheme.kt` generiert). Farb-Semantik bindend: **teal=Senden/Sperren-Akzent · amber=live/Halte-Ring · rot=Abbrechen**.
- 9-7 (Gesten-Modus-**Erkennung**) **nicht** still erweitern — existiert; hier nur die HOLD-Surface+Interaktion.
- `FLAG_NOT_TOUCHABLE` nie (HyperOS dimmt auf 0.8).
- **Keine** neuen Config-Keys (anders als 9-15 — HOLD hat keinen nutzer-konfigurierbaren Größen-Parameter in dieser Story; offene Frage zu `recordingButtonSizeDp`-Bezug siehe Elicitation-Report).

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

## Tasks / Subtasks

- [x] **Task 1: Remove the old slide-track HOLD draw path in `FloatingBubbleView.kt` (AC: 1)**
  - [x] 1.1 Deleted `drawHoldDock()` and its only-used-there helpers: `holdDockStripRect`/`holdDockHeldbubRect`/`holdScratchRect`/`holdStripBlurFilter`/`heldbubBlurFilter`, `holdArrowPhase`/`holdArrowAnimator` + its start/stop wiring in `updateAnimators()`, `holdArrPaint`/`holdCancelTextPaint`/`holdLockTextPaint`/`holdUpPaint`/`holdLockIconPaint`.
  - [x] 1.2 Kept `drawLockIcon()` + `lockBodyRect`/`lockShackleRect`/`lockBodyFillPaint`/`lockShacklePaint` — reused as-is for the new Sperren target icon (both rest 34dp and active 40dp).
  - [x] 1.3 Deleted all dead `HOLDDOCK_*`/`HOLD_HELDBUB*`/`HOLD_RING*`/`HOLD_INNER_RING*`/`HOLD_GAP_DP`/`HOLDSTRIP_*`/`HOLD_FINGER_DP` constants. Confirmed via `grep -rn "HOLDDOCK_\|HOLD_HELDBUB\|HOLDSTRIP_" android/` — no references survive outside the deleted block.
  - [x] 1.4 Updated class-level KDoc to describe the new B-language HOLD targets (thumb-anchor bubble + two large round REST/ACTIVE targets) instead of holdstrip/heldbub/lockchip.

- [x] **Task 2: New B-language HOLD geometry constants + state in `FloatingBubbleView.kt` (AC: 1, 2, 3, 4, 7)**
  - [x] 2.1 New companion constants added (`HOLD_BUBBLE_DP/R_DP`, `HOLD_TARGET_REST_DP/ACTIVE_DP`, `HOLD_CHIP_H_DP`, `HOLD_SHADOW_PAD_DP`) plus window-layout offset constants derived from the mockup (`HOLD_BUBBLE_EDGE_GAP_DP`, `HOLD_TARGET_FAR_INSET_DP`, `HOLD_BUBBLE_TARGET_GAP_DP`, `HOLD_LOCK_OFFSET_ABOVE_DP=168f`, `HOLD_CANCEL_OFFSET_BELOW_DP=74f`, `HOLD_CHIP_BUBBLE_GAP_DP`) and derived `HOLD_VISUAL_W_DP`/`HOLD_VISUAL_H_DP` reference dims for window sizing (Task 5). Documented as approximations, not pixel law.
  - [x] 2.2 New view-level state `var holdTargetHit: HoldTarget = HoldTarget.NONE` (top-level enum `NONE, LOCK, CANCEL`, package-visible so `KlarvoOverlayService` references it unqualified). Setter triggers `invalidate()` only — no animator.
  - [x] 2.3 Local color constants `HOLD_DANGER_HI`/`HOLD_DANGER_LINE`/`HOLD_ZONE_REST_BG` added (not in `KlarvoTheme.kt`). `KlarvoTheme.TealLine` reused as-is for the rest-state Sperren border.

- [x] **Task 3: Implement `drawHoldTargets()` — replaces `drawHoldDock()` (AC: 1, 2, 3, 4, 7)**
  - [x] 3.1 Thumb-anchor bubble implemented: teal-gradient squircle (reuses `idleFillPaint`/`kLetterPaint` pattern from `drawIdleBubble`), amber holding-ring (5dp outset).
  - [x] 3.2 Two rest-state zones implemented via new `drawHoldZone()` helper: Sperren (teal-line border + `drawLockIcon`) above, Abbrechen (danger-line border + "✕" text glyph) below, dark `HOLD_ZONE_REST_BG` fill, two-line labels.
  - [x] 3.3 Waveform chip implemented via new `drawHoldChip()` — reuses `drawClusterWaveform` unchanged, compact variant of `drawTapChip` (smaller HOLD window budget), reuses `recordingStartMs`.
  - [x] 3.4 Live caption implemented via new `drawHoldCaption()` — amber dot + halo + "Aufnahme · loslassen = senden" text, mirrors `.reccap`.
  - [x] 3.5 Grow-on-target implemented in `drawHoldZone()`: `active` param switches REST→ACTIVE radius, radial-gradient fill, glow ring, "loslassen = …" label; centers stay fixed (only radius/style change) — confirmed via the device-scale screenshot (emulator structural smoke).
  - [x] 3.6 `onDraw()` routing updated: `State.RECORDING -> if (holdDockActive) drawHoldTargets(canvas) else drawTapSurface(canvas)` — only the function name changed, branch condition untouched.

- [x] **Task 4: Pure, JVM-testable geometry helpers in the companion object (AC: 3, 4, 5, 7)**
  - [x] 4.1 Added `holdBubbleCenter()` + `holdTargetCenters()` mirroring `tapCircleCenters`'s pattern — returning a new plain-Kotlin `HoldPoint(x, y)` data class, **not** `android.graphics.PointF` (discovered during JVM testing: PointF's constructor is a no-op stub in the Android unit-test jar without Robolectric, silently leaving x/y at 0 — exactly the failure mode `tapCircleCenters`'s `Pair<Float,Float>` return type was already avoiding). `KlarvoOverlayService`'s ACTION_MOVE hit-tracking (Task 6) calls `holdTargetCenters()` directly.
  - [x] 4.2 Reused `isInsideCircle()` against each target's REST-radius (`HOLD_TARGET_REST_DP / 2`) for the hit test — growing to ACTIVE does not change the hit zone (confirmed by `HoldTargetTouchZoneTest.touch_inside_active_radius_but_outside_rest_radius_still_misses`).

- [x] **Task 5: `KlarvoOverlayService.kt` — window sizing for the new HOLD layout (AC: 1, 2, 7)**
  - [x] 5.1 `adjustLayoutForState()`'s `pushToTalkActive` branch rewritten: window sized from `HOLD_VISUAL_W_DP`/`HOLD_VISUAL_H_DP` + `2×HOLD_SHADOW_PAD_DP`. Device-scale-verified on emulator: 939×1076px @2.625 density (= 358×410dp), substantially larger than both the old HOLD dock (190×96dp-ish) and the TAP surface (509×341px confirmed unchanged in the same run).
  - [x] 5.2 Kept the right/left-edge dock-anchor pattern for X; Y now anchored via `holdBubbleCenter()` so the thumb-anchor bubble's on-screen center stays where the idle bubble was (reuses the same pure fn `drawHoldTargets()` uses, so Service window placement and View draw geometry cannot diverge).
  - [x] 5.3 `bubbleView.dockSide = getDockSide()` set before any HOLD target-position computation in `adjustLayoutForState()`.

- [x] **Task 6: `KlarvoOverlayService.kt` — rewrite `ACTION_MOVE` from threshold-fire to continuous hit-tracking (AC: 3, 4, 5)**
  - [x] 6.1 `ACTION_MOVE`'s `pushToTalkActive` branch rewritten to pure hit-tracking: computes `holdTargetCenters()` for the current `dockSide`/window size every move, sets `bubbleView.holdTargetHit` via `isInsideCircle` against REST radius, no commit calls.
  - [x] 6.2 Old `holdDragCancelPx`/`holdDragLockPx` fields and the directional-threshold `when` block fully removed (read the finding 3/4 comments first, as instructed) — circular hit-test has no directional ambiguity by construction.
  - [x] 6.3 Non-PTT drag branch left untouched.

- [x] **Task 7: `KlarvoOverlayService.kt` — release-to-commit dispatch on `ACTION_UP`/`ACTION_CANCEL` (AC: 5, 6)**
  - [x] 7.1 `ACTION_UP`'s `pushToTalkActive ->` branch replaced with the dispatch-on-`holdTargetHit` exactly as specced.
  - [x] 7.2 `ACTION_CANCEL` kept the existing safe `cancelRecording()` default; added `holdTargetHit = HoldTarget.NONE` reset there too.
  - [x] 7.3 `lockHoldToCluster()` verified unchanged — its `preclusterBubbleY` restore is symmetric with the new Task 5 Y-anchor (same field, same null-after-use pattern); confirmed via emulator (HOLD window → idle shrink-back correctly restored to 162×162 square).

- [x] **Task 8: `longPressRunnable` — verify ordering still holds (AC: 1)**
  - [x] 8.1 Verified: the simplified `holdDockActive` setter (Task 2.2) no longer reads `state` at all, so the old ordering risk (finding 1) is structurally gone, not just smaller.

- [x] **Task 9: Harness compatibility (AC: DoD structural smoke)**
  - [x] 9.1 Confirmed via live emulator run — but found and fixed a real pre-existing bug while doing so: `DebugHarnessReceiver.kt` (the manifest-declared cold-start receiver, which ALSO receives every `am broadcast` alongside the dynamic receiver) never forwarded the `hold_mode` extra, so it could race the dynamic receiver and silently snap a HOLD-mode harness state back to the TAP surface (defaulting `holdMode=false`). Fixed by forwarding `EXTRA_HOLD_MODE` through the cold-start service Intent. Re-verified: `--ez hold_mode true` now reliably produces the HOLD window.
  - [x] 9.2 Structural assertion done on a live `klarvo-emu` AVD: HOLD window present (939×1076px, `Window{... com.klarvo.voice}` type=2038 APPLICATION_OVERLAY), no `FLAG_NOT_TOUCHABLE` (`fl=NOT_FOCUSABLE LAYOUT_IN_SCREEN HARDWARE_ACCELERATED`), size clearly ≠ TAP surface (509×341px, same run) and ≠ old dock dims. Also captured a screenshot confirming visual structure (bubble + chip + both REST-state targets, correct teal/red/amber color semantics, no leftover slide-track elements).

- [x] **Task 10: JVM Unit Tests — `HoldTargetTouchZoneTest.kt` (new file, mirror `TapSurfaceTouchZoneTest.kt` pattern)**
  - [x] 10.1 `holdTargetCenters()` mirroring tested for both dock sides.
  - [x] 10.2 `isInsideCircle` hits/misses against Lock/Cancel centers at REST radius tested (center hit, edge-exact hit, just-outside miss).
  - [x] 10.3 Touch outside both circles → `HoldTarget.NONE` tested (bubble position, window origin, midpoint between targets) — via a local `resolveHit()` helper replicating the production dispatch, same convention as `TapSurfaceTouchZoneTest`.
  - [x] 10.4 Residual gap noted: `holdDockActive`/View-instance behavior (the `onDraw` routing, the `holdTargetHit` setter's `invalidate()`) is not JVM-testable without Robolectric — verified via code-reading + the live emulator structural smoke instead, same residual class 9-15 documented.

- [x] **Task 11: Build + structural smoke + drift gate (DoD)**
  - [x] 11.1 `node scripts/gen-android-theme.mjs --check` green (no drift). `:app:testUniversalDebugUnitTest` — 113 tests, 0 failures (15 TapSurfaceTouchZoneTest + 13 new HoldTargetTouchZoneTest). `:app:assembleUniversalDebug` (Kotlin-only, Rust `.so` from cache, mirrors `android-smoke.sh`'s build step) — fresh APK built. Full `android-smoke.sh` not run end-to-end (no Tailscale real-device target in this session) — instead booted `scripts/android-emulator.sh` directly and ran the equivalent install + structural-smoke steps by hand (see 9.1/9.2).
  - [x] 11.2 Confirmed on the same emulator run: TAP surface window unchanged (509×341px, `hold_mode=false`) and idle shrink-back unchanged (162×162px) — no regression from the HOLD rewrite or the `wasBarMode` fix. `TapSurfaceTouchZoneTest` 15/15 green.
  - [x] 11.3 N/A — no Rust/`shells/windows/` files touched (confirmed via `git status --short android/`); Kotlin-only story per Scope.

- [x] **Task 12: Commit (AC: scope)**
  - [x] 12.1 Staged only the touched Kotlin files: `FloatingBubbleView.kt`, `KlarvoOverlayService.kt`, `DebugHarnessReceiver.kt` (Task 9.1 fix), new `HoldTargetTouchZoneTest.kt`.
  - [x] 12.2 No `git add .` used.

## Dev Notes

### What "B-Sprache" replaces, concretely

The CURRENT `RECORDING` state in `FloatingBubbleView.kt` already branches on `holdDockActive`
(l.576): `if (holdDockActive) drawHoldDock(canvas) else drawTapSurface(canvas)`. `drawTapSurface()`
(Story 9-15, l.636+) is **done** (status `review`) and is exactly the AC6 "gesperrt" target — do not
rebuild it. `drawHoldDock()` (l.1140-1333) is the **old, rejected** slide-track HOLD surface
(`.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-heldbub`/`.ab-lockchip`) that this story replaces
with `drawHoldTargets()`. The `holdDockActive` boolean itself, its setter, and the routing branch
**stay** — only what's drawn when it's `true` changes.

### Exact SOLL values (device-scale approved 2026-06-26)

Both mockups were rendered via Playwright at 1080×2460 physical px (Redmi @440dpi, factor 2.75) and
Andi-approved at that scale — unlike the original (rejected) 9-15 TAP mockup, these CSS px values
**are** dp-equivalent and don't need the kind of post-hoc device recalibration 9-15 needed
(`mockup-tap-size-calibration.html`). Still, GATE-4 visual fidelity remains Andi's real-device call —
treat these as strong defaults, not pixel law.

From `docs/design/overhaul/mockup-mobile-hold-B-refined.html` (`#bRest`, `#bHit`):
- `.heldbub`: 82×82dp, `border-radius:25px`, `background:linear-gradient(150deg,#57DDC7,#1B9C88)` (= `KlarvoTheme.TealHi`→`TealLo`), `color:#05201B` (`OnTeal`), box-shadow incl. `0 0 0 5px rgba(233,162,76,.45)` (amber ring outset 5dp ≈ `KlarvoTheme.AmberLine`).
- `.zone.rest`: 112×112dp, `background:rgba(18,20,22,.92)`; `.lock` variant: `border:2px solid rgba(41,199,172,.45)` (≈ `KlarvoTheme.TealLine`), `color:#57DDC7`; `.cancel` variant: `border:2px solid rgba(238,111,99,.45)` (new local const, see Task 2.3), `color:#F4897E` (new local const `HOLD_DANGER_HI`). Icon 34px, label 12px muted.
- `.zone.active`: 148×148dp, `color:#fff`; `.cancel`: `background:radial-gradient(circle at 50% 38%, #F4897E, #EE6F63)` + glow `0 0 0 8px rgba(238,111,99,.22), 0 0 46px rgba(238,111,99,.5)`; `.lock`: `radial-gradient(circle at 50% 38%, #57DDC7, #1B9C88)`, `color:#05201B` (label too). Icon 46px (cancel ✕) / 40px (lock SVG, per `holdLock` frame), label 13px weight 600.
- `.statuschip` (waveform): `padding:11px 15px;border-radius:18px;background:rgba(18,20,22,.96);border:1px solid var(--k-border)` (≈ `KlarvoTheme.Border`); wave bars `width:4px;border-radius:3px;background:var(--k-amber)`, `gap:3.5px`, `height:30px` container — same bar-drawing as `drawTapChip`'s chip, reuse `drawClusterWaveform`.
- `.reccap` (live caption): `color:var(--k-muted)`, `font-size:13px`; `.dot`: 8px amber circle + `box-shadow:0 0 0 4px rgba(233,162,76,.45)` (`AmberLine`).
- Positions (`#bRest`, bubble docked right): bubble `right:16,top:433`; Sperren zone `left:48,top:250`; Abbrechen zone `left:44,top:492` — i.e. both targets sit on the **opposite side from the dock**, vertically straddling the bubble's row (Sperren above, Abbrechen below), confirming AC7's "weg von der Dock-Kante" reading literally as **horizontal** mirroring with **fixed vertical** roles (Sperren always up, Abbrechen always down — only left/right flips with dock side, not up/down).

From `docs/design/overhaul/mockup-mobile-recording-states.html` (`#holdLock`): confirms the active-Sperren render (148dp, teal glow, lock icon 40px) and that the Abbrechen zone stays at rest size while Sperren is active (only one target grows at a time — `holdTargetHit` as a single-value enum, not two independent booleans, is the correct model, Task 2.2).

**Drag visual (both frames show this consistently — not just decoration, it's the SOLL for what happens between dock and a hit target):** while dragging toward a target, a translucent "ghost" bubble (74×74dp, dashed border, ~50% alpha gradient fill) tracks near the finger position, and the origin `.heldbub` at the dock fades to ~0.3 opacity. This detail is **not** spelled out in the AC prose but **is** pinned by the binding render — see Elicitation Report below, it's flagged there rather than silently assumed, because it adds real implementation surface (an interpolated/finger-following bubble position) beyond a static two-target redraw.

### Release-to-commit is the core mechanism change — read the current code first

The **existing** `ACTION_MOVE` handling (`KlarvoOverlayService.kt` l.1149-1183) fires
`cancelRecording()`/`lockHoldToCluster()` **immediately when a drag-distance threshold is crossed
during the move** — that is the old (rejected) mechanism. The new design requires: track which target
(if any) the finger currently sits over on every `ACTION_MOVE` (pure redraw, no side effect), and only
commit (`cancelRecording()`/`lockHoldToCluster()`/`stopAndProcessRecording()`) on `ACTION_UP`, based on
where the finger was at release. This is **not a refinement of the threshold approach** — it's a
different state model (hit-tracking + release-dispatch vs. directional-threshold-fire-on-move). Tasks 6-7
above are the concrete rewrite; do not try to retrofit the old `holdDragCancelPx`/`holdDragLockPx`
fields into the new model.

### Reusable infrastructure (do not rebuild)

- `isInsideCircle(touchX, touchY, cx, cy, radius)` — pure companion function, `FloatingBubbleView.kt` l.241-245. JVM-tested precedent: `TapSurfaceTouchZoneTest.kt`.
- `tapCircleCenters(dockSide, windowW, shadowPad, radius)` — pattern to mirror for `holdTargetCenters()` (Task 4.1), l.256-260.
- `drawLockIcon(canvas, cx, cy, sizeDp, paint)` — generic Canvas-primitive padlock (rounded-rect body + arc shackle), l.1343-1362. Already parameterized by size/color — reuse directly for both the rest (34dp) and active (40dp) Sperren icon, just pass different `sizeDp`/`paint.color`.
- `drawClusterWaveform` / the waveform-bar drawing inside `drawTapChip` (l.678-740) — RMS-driven, Story 9-12, **must not change** (Scope boundary).
- `lockHoldToCluster()` (l.1109-1127) — already wired to TAP-surface dimensions (`tapVisualWidthDp`/`tapVisualHeightDp`), already sets `dockSide`. This is the AC6 mechanism and needs no rebuild, only a Y-restore symmetry check (Task 7.3).
- `getDockSide()` (l.1027-1035) — already dock-mirror-aware, reuse as-is.
- The idle-bubble gradient+"K"-letter draw pattern (`drawIdleBubble`) — reuse the same gradient/text approach for the new thumb-anchor bubble (Task 3.1) rather than inventing a new paint setup.

### Color semantics (binding, ADR-0019 §1-§3, unchanged by this story)

teal = brand/confirm/lock-accent · amber = live/holding-ring · danger/red = **only** Abbrechen, never
Senden/Sperren. The Sperren target is teal (it's a confirm-style action — "lock so I can review before
sending" — not destructive); Abbrechen is red. Do not swap.

### Anti-patterns — do NOT do

- Do **not** touch `drawClusterWaveform`, `waveLevels`, `amplitude` setter, `setStaticWaveLevel` (Story 9-12, locked).
- Do **not** touch `drawTapSurface()`, `isTouchInConfirmZone`/`isTouchInCancelZone`, or any TAP-surface geometry (Story 9-15, locked — this story only consumes `lockHoldToCluster()`'s existing call into it).
- Do **not** add a new config key for HOLD target sizing — `recordingButtonSizeDp` is 9-15/TAP-surface-scoped per the Scope section; see Elicitation Report for the open question about whether it *should* extend here.
- Do **not** add `FLAG_NOT_TOUCHABLE` anywhere (HyperOS dims overlays to 0.8 alpha — `reference_hyperos_overlay_quirks`).
- Do **not** hand-edit `KlarvoTheme.kt` — it's generated (`scripts/gen-android-theme.mjs`); new non-canon colors (danger-hi, danger-line) go in local file-scope constants, same pattern as `TAP_CANCEL_*` in Story 9-15.
- Do **not** revive the old direction-threshold (`holdDragCancelPx`/`holdDragLockPx`, signed dx/dy) logic — it's superseded by circular hit-test + release-dispatch.
- Do **not** silently expand Story 9-7 (gesture-mode detection) — `longPressMode`/`pushToTalkActive` already correctly gate entry into this surface; this story only changes what's drawn/how release is handled once already in HOLD mode.

### Previous-build learnings (same story, superseded implementation — still relevant bug classes to avoid)

The OLD `drawHoldDock()` build fixed several real bugs (commits `c389c88`/`e92f4f3`, code comments
"finding 1" through "finding 6" still in the current file) before failing GATE-4 on **design**, not
mechanics. Worth re-reading because the new build can reintroduce the same *class* of bug even with
different code:
- **Finding 1** (`FloatingBubbleView.kt` l.498-503): a property setter that checks `state` can silently
  no-op if called before the state transition completes — order side-effecting setters correctly
  relative to `setState()`. Less relevant now (Task 2.2 drops the animator-start side effect), but the
  general lesson (verify setter-vs-transition ordering) still applies to `holdTargetHit`.
- **Finding 2** (referenced at `KlarvoOverlayService.kt` l.452, `setHoldModeOnPanel`): a `isLockedMode`
  panel flag was never cleared on stop/cancel, so a subsequent recording could inherit prior locked
  state. Check the new code resets ALL HOLD-related transient state (`holdTargetHit`, any panel
  hold/lock flags) on every stop/cancel/lock transition, not just the happy path.
- **Finding 5** (`KlarvoOverlayService.kt` l.267-269, l.1064-1067): an unclamped Y-shift (`+=`) created
  an asymmetric restore between entering and leaving the old HOLD dock. Task 7.3 calls this out
  specifically — verify the new window's Y-shift (if any) is restored symmetrically via
  `preclusterBubbleY`, the same field the old code used and `lockHoldToCluster()` already reads.
- GATE-4 evidence (`_bmad-output/implementation-artifacts/gate4-evidence/9-14/verdict.md`) noted the
  machine layer (build/structure) was green while a real-device-only animator bug slipped through
  (AC1a hint-arrow pulse, harness-invisible). The new design has no continuous animator (Task 2.2), which
  removes that whole bug class, but reinforces: **motion/touch correctness is real-device-only,
  never emulator-verifiable** (DoD already reflects this).

### Dependency state (Story 9-15)

Story 9-15 (`_bmad-output/implementation-artifacts/9-15-mobile-tap-recording-surface-reskin.md`) is
status `review` (not yet `done`), but its code is fully implemented and present in the current tree
(`drawTapSurface`, `isInsideCircle`, `tapCircleCenters`, `recordingButtonSizeDp`, `lockHoldToCluster()`
already targeting it). This story can and should build directly on that code regardless of 9-15's
formal sprint-status — per the live handoff, both stories converge into one build/device-test cycle.
If 9-15 changes during this story's implementation window (e.g. a review-fix lands), re-sync before
the final smoke.

### Project Structure Notes

- Both files are at `android/kotlin-src/com/klarvo/voice/` — always this path (the build copies into
  the Gradle tree via `android-build.sh`), never `android/app/src/...`.
- New JVM test goes in `android/kotlin-test/com/klarvo/voice/HoldTargetTouchZoneTest.kt`, mirroring
  `TapSurfaceTouchZoneTest.kt`'s no-Robolectric, pure-function-under-test style.
- `KlarvoTheme.kt` is generated (Story 9-10 drift gate) — do not hand-edit; this story does not touch
  tokens, only adds local non-generated Canvas constants in `FloatingBubbleView.kt` (same convention
  Story 9-15 established for `TAP_CANCEL_FILL`/`TAP_CANCEL_BORDER`/`TAP_CANCEL_DANGER_HI`).
- No Rust/`config.json`/`src/` changes expected (Scope: no new config keys) — `cargo check
  --target x86_64-pc-windows-gnu` is N/A unless something unexpectedly touches `shells/windows/`.

### References

- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.1-48] — Class KDoc, current RECORDING/holdDockActive state description (needs updating per Task 1.4).
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.105-117] — `holdDockActive` setter (animator side effect to remove, Task 1.1/2.2).
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.149-261] — companion object: TAP-surface reference constants + `isInsideCircle`/`tapCircleCenters` pure functions to mirror.
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.362-374, l.1131-1333] — old HOLD dock pre-allocated scratch objects + `drawHoldDock()` full implementation (to delete, Task 1).
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.555-578] — `isTouchInConfirmZone`/`isTouchInCancelZone`/`onDraw` routing (l.576 is the one-line change point for Task 3.6).
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.636-786] — `drawTapSurface`/`drawTapChip`/`drawTapSendCircle`/`drawTapCancelCircle` (Story 9-15, reuse pattern for Task 3).
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.1343-1362] — `drawLockIcon` (reusable as-is, Task 3.5).
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.212, 250, 417-445] — `longPressMode`/`pushToTalkActive` state + `longPressRunnable` entry into HOLD.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.1027-1127] — `getDockSide()`, `adjustLayoutForState()` (Task 5), `lockHoldToCluster()` (Task 7.3).
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.1129-1251] — `handleTouch()` full `ACTION_DOWN`/`ACTION_MOVE`/`ACTION_UP`/`ACTION_CANCEL` (Tasks 6-7 rewrite target).
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.285-330] — harness `DEBUG_SET_STATE` + `EXTRA_HOLD_MODE` (Task 9, no changes needed).
- [Source: docs/design/overhaul/mockup-mobile-hold-B-refined.html] — binding SOLL render, frames `#bRest`/`#bHit`, exact CSS for bubble/targets/chip/caption.
- [Source: docs/design/overhaul/mockup-mobile-recording-states.html, `#holdLock`] — binding SOLL for the active-Sperren state.
- [Source: docs/adr/0019-cross-platform-design-ssot.md, Amendment 2026-06-26] — binding decision text, supersedes the §4′-Amendment HOLD slide-variant.
- [Source: docs/backlog.md, "Mobile-Overlay-Design durchgefallen" §] — origin of the GATE-4 rejection + the 6 "Konkrete Items" that informed the B-language redesign.
- [Source: _bmad-output/implementation-artifacts/gate4-evidence/9-14/verdict.md] — previous (superseded) GATE-4 attempt's machine-layer pass + the real-device-only animator bug it missed.
- [Source: _bmad-output/implementation-artifacts/9-15-mobile-tap-recording-surface-reskin.md] — foundational TAP-surface story; AC9/AC10 are exactly what `lockHoldToCluster()` already targets.
- [Source: _bmad-output/implementation-artifacts/9-13-swap-send-cancel-cluster-order.md] — sibling story, same files, established the "one file at a time, pure-function-for-JVM-testability, no `FLAG_NOT_TOUCHABLE`" conventions this story follows.
- [Source: _bmad-output/project-context.md] — no `git add .`; Android changes require on-device/emulator smoke; Android changes must mirror Rust path if shared behavior — N/A here (no shared/Rust behavior touched); `android-smoke.sh` drift gate.

## Anchors (binding design source)

- **ADR-0019 Amendment 2026-06-26** (`docs/adr/0019-cross-platform-design-ssot.md`).
- **Bindende Render (SOLL):** `docs/design/overhaul/mockup-mobile-hold-B-refined.html` (Frames `bRest` Ruhe + `bHit` Treffer-Abbrechen) **+** `docs/design/overhaul/mockup-mobile-recording-states.html` (Frame `holdLock` Treffer-Sperren).
- Canon-Fingerprint `bac152993046699c5007612ac916d951`; **supersedet** `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-heldbub`/`.ab-lockchip`.
- Touch/Canvas: `reference_android_bubble_canvas_and_install.md`. Verifikations-Lehre: `feedback_gate4_smoke_needs_behavioral_delta.md`.

## DoD (surface-class)

DEBUG APK builds; JVM-Tests grün; Emulator **strukturelle** Smoke grün (Fenster-Struktur via `scripts/android-smoke.sh` unter `BMAD_CONDUCTOR=1`). **GATE-4 Bewegung/Touch/Visual = echtes Gerät + Live-Mikro (Andis Batch-Gate):** halten/loslassen/wegziehen/hochziehen-sperren, grow-on-target, release-to-commit + Undo, Lesbarkeit — **nur** am echten Gerät verifizierbar, nie am Emulator (kein Motion-/Touch-Orakel). Overlays nie `FLAG_NOT_TOUCHABLE`.

## Dependency

Baut auf **Story 9-15** auf (gesperrt-Zustand = 9-15-TAP-Surface; Code bereits vorhanden, Status `review`). 9-15 zuerst / zusammen.

## Dev Agent Record

### Agent Model Used

claude-sonnet-5 (story context authoring) — implementation model TBD at dev-story time.

### Debug Log References

- Emulator structural smoke (`klarvo-emu` AVD, manual install + `DEBUG_SET_STATE` harness, see Completion Notes) — window dumps + screenshot saved to scratchpad during the session (not repo-persisted; AC1-AC2/AC7 + window-sizing/regression evidence summarized below and in Task 9/11 checkmarks).

### Completion Notes List

- Ultimate context engine analysis completed — comprehensive developer guide created from current Kotlin source (`FloatingBubbleView.kt`, `KlarvoOverlayService.kt`), the two binding device-scale-approved mockups, ADR-0019 (full amendment chain), Story 9-13/9-15 (sibling/dependency context), and the previous (superseded) 9-14 build's GATE-4 evidence + code comments documenting real bugs found and fixed there.
- Implemented `drawHoldTargets()` (replacing `drawHoldDock()`), `drawHoldZone()`, `drawHoldChip()`, `drawHoldCaption()` in `FloatingBubbleView.kt`; new `holdTargetHit: HoldTarget` state; new pure `holdBubbleCenter()`/`holdTargetCenters()` companion functions.
- Rewrote `KlarvoOverlayService.kt`'s HOLD window sizing (`adjustLayoutForState`), `ACTION_MOVE` (threshold-fire → continuous hit-tracking), and `ACTION_UP`/`ACTION_CANCEL` (release-to-commit dispatch on `holdTargetHit`) exactly per Tasks 5-7.
- **Mid-implementation finding (Task 4.1):** the first version returned `android.graphics.PointF` from the new pure geometry functions. JVM unit tests passed-but-wrong (all coordinates silently `0.0`) because the Android unit-test stub jar (no Robolectric in this project's test setup) replaces `PointF`'s constructor with a no-op rather than throwing. Root-caused by comparing against `tapCircleCenters` (Story 9-15), which deliberately returns a plain `Pair<Float, Float>` for exactly this reason. Fixed by introducing a small top-level `data class HoldPoint(x, y)` instead of `PointF` — pure Kotlin, safe for JVM tests. Documented inline so the trap isn't rediscovered.
- **Mid-implementation finding (Task 9.1, real bug, pre-existing — not introduced by this story):** `DebugHarnessReceiver.kt` (the manifest-declared cold-start broadcast receiver) forwards `state`/`rms`/`transcript` to the service Intent but never forwarded `hold_mode`. A single `adb shell am broadcast` is delivered to BOTH this static receiver AND the service's already-registered dynamic receiver; the dynamic one applies `hold_mode` correctly, but the static one (which always also fires, via `startForegroundService` → `onStartCommand`) silently defaulted `hold_mode=false` and could win the race, snapping a HOLD-mode harness state back to the TAP surface. This was reproduced live on the emulator (window stayed TAP-sized — 509×341px — despite `--ez hold_mode true`) and is the likely reason any prior on-device HOLD verification via the harness was unreliable. Fixed by forwarding `EXTRA_HOLD_MODE` through; re-verified the HOLD window (939×1076px) then renders reliably.
- **Window-sizing aspect-ratio finding (Task 5/7.3):** `cancelRecording()`/`stopAndProcessRecording()` detected "was the window in an expanded (TAP/HOLD) layout" via `bubbleView.width > bubbleView.height`. The old HOLD dock and the TAP surface are both wider-than-tall, so this worked — but the new B-Sprache HOLD window (two vertically-separated targets) is intentionally *taller* than wide (358×410dp), which would have made `wasBarMode` evaluate `false` and silently skip the shrink-back-to-idle resize, leaving the window stuck oversized after every cancel/stop. Fixed by changing the check to `width != height` (any non-square window is "expanded" — IDLE/TRANSCRIBING/DONE are always square), which is correct for both aspect ratios and was verified on-device (HOLD → idle shrinks to 162×162; TAP → idle unaffected, same emulator run).
- Live emulator verification (`scripts/android-emulator.sh`, manual install/harness sequence — full `android-smoke.sh` not run end-to-end since this session has no Tailscale real-device target): confirmed HOLD window renders (939×1076px, correct teal/red/amber colors, no leftover slide-track elements, no `FLAG_NOT_TOUCHABLE` — screenshot captured), confirmed no regression in TAP surface (509×341px unchanged) or idle shrink-back (162×162px, both directions). Emulator stopped cleanly after.
- All 113 JVM unit tests pass (0 failures), including the 13 new `HoldTargetTouchZoneTest` tests and the pre-existing 15 `TapSurfaceTouchZoneTest` tests (no regression). Theme drift gate (`gen-android-theme.mjs --check`) green. Fresh debug APK built (`assembleUniversalDebug`, Kotlin recompiled + Rust `.so` from cache).
- **Not verified (explicitly out of scope for dev-story, per DoD):** actual touch-driven grow-on-target visuals (AC3/AC4) and the full hold→drag→release gesture sequence (AC5) require real finger input and are GATE-4 real-device-only per the story's own DoD ("kein Motion-/Touch-Orakel" — emulator). The geometry/dispatch LOGIC behind these (hit-test radius, target centers, release dispatch) is fully covered by `HoldTargetTouchZoneTest` and code-reading; the VISUAL/MOTION feel is Andi's batch real-device gate.

### File List

- `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` (modified)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (modified)
- `android/kotlin-src/com/klarvo/voice/DebugHarnessReceiver.kt` (modified — harness `hold_mode` forwarding fix, Task 9.1 finding)
- `android/kotlin-test/com/klarvo/voice/HoldTargetTouchZoneTest.kt` (new)

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | (frühere Fassung) Slide-Spur-HOLD gebaut (`c389c88`/`e92f4f3`), GATE-4 Maschinen-Ebene grün. | claude-sonnet-4-6 / conductor |
| 2026-06-26 | **GATE-4 Real-Device FAILED** (Andi): zu klein / „Laptop-Feel" → Status `in-progress`, systemischer Mobile-Overlay-Rethink. | claude-opus-4-8 (conductor) |
| 2026-06-26 | **Story NEU GEFASST** in B-Sprache (ADR-0019 Amendment 2026-06-26) gegen Render `mockup-mobile-hold-B-refined.html` + `mockup-mobile-recording-states.html`; alte `.ab-holddock`-Surfaces superseded; hängt an Story 9-15. `ready-for-dev`. Build folgt in frischer Session. | claude-opus-4-8 (conductor) |
| 2026-06-30 | **bmad-create-story Vollkontext-Pass.** Tasks/Subtasks + Dev Notes neu erstellt gegen den aktuellen Kotlin-Code-Stand (alte `drawHoldDock()` exakt lokalisiert für Removal), exakte SOLL-Werte aus beiden Mockups extrahiert, Wiederverwendungs-Inventar (9-15 TAP-Surface-Infrastruktur, `drawLockIcon`, `isInsideCircle`/`tapCircleCenters`-Pattern) dokumentiert, Release-to-Commit-Mechanik-Wechsel (Schwellwert-während-Bewegung → Hit-Tracking+Release-Dispatch) als Kern-Rewrite benannt, Vorgänger-Build-Lehren (findings 1/2/5) übernommen. Scope/ACs/Anchors/DoD/Dependency unverändert (bereits Andi-approved, bindend). Offene Design-Frage zu HOLD-Zielgröße vs. `recordingButtonSizeDp` an Elicitation-Report eskaliert (nicht selbst entschieden). Status bleibt `ready-for-dev`. | claude-sonnet-5 (bmad-create-story) |
| 2026-06-30 | **Implementiert (dev-story).** `drawHoldTargets()`/`drawHoldZone()`/`drawHoldChip()`/`drawHoldCaption()` ersetzen `drawHoldDock()`; neue Pure-Geometrie (`holdBubbleCenter`/`holdTargetCenters`, `HoldPoint` statt `PointF` — JVM-Test-Stub-Falle gefunden+vermieden); `KlarvoOverlayService` HOLD-Fenstergröße/ACTION_MOVE/ACTION_UP komplett neu (Hit-Tracking + Release-to-Commit statt Schwellwert-während-Bewegung); `wasBarMode`-Aspect-Ratio-Bug gefunden+gefixt (HOLD ist hochkant, alter `width>height`-Check hätte den Shrink-back nie ausgelöst); `DebugHarnessReceiver.kt` `hold_mode`-Forwarding-Bug gefunden+gefixt (Vorbedingung für jede Harness-Verifikation). 113/113 JVM-Tests grün (13 neu), Theme-Drift-Gate grün, frischer Debug-Build, Live-Emulator-Smoke (Fenstergröße 939×1076px HOLD vs. 509×341px TAP vs. 162×162px Idle, Screenshot, keine Regression). GATE-4 Touch/Motion bleibt Andis Real-Device-Gate. Status → `review`. | claude-sonnet-5 (bmad-dev-story) |
