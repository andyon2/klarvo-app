# Story 9.14: HOLD-Modus (Push-to-Talk) Bubble-Cluster-Variante

Status: review

## Story

As a user dictating on Android with the **Hold** gesture mode,
I want pressing-and-holding the bubble to record, releasing to send, and dragging away to cancel — with an upward drag to **lock** into a normal tappable cluster,
so that the recording cluster matches the familiar voice-message model that Hold actually implies, instead of the tap/toggle cluster whose ➤ Send is redundant (release already sends) and whose ✗ Cancel is unreachable while holding.

## Scope (locked — Hold-mode bubble interaction + its surfaces only, do NOT expand)

Add the HOLD-mode recording variant on Android, used **only** when the user long-presses the bubble AND `longPressMode == RecordingMode.HOLD` (i.e., `pushToTalkActive = true`). While the finger holds:

- **hold = record · release = send · drag away = cancel** (no tappable ➤/✗ exist during the hold)
- **drag up → 🔒 lock** converts the held state into the normal tap-cluster `[✗ Cancel (left) · waveform (center) · ➤ Send (right/thumb)]` (the order from Story 9-13)

Live cue stays the **amber** waveform; the hold ring is **amber**. New canon surfaces: `.ab-holddock` / `.ab-holdstrip` / `.ab-slidehint` / `.ab-heldbub` / `.ab-lockchip`.

**Hard scope boundaries:**
- **No** change to Tap / Toggle / Auto-Stop / Auto modes (they keep the §4′ cluster unchanged).
- **No** RMS/waveform behavior change (Story 9-12, done — `waveLevels`, `drawClusterWaveform`, `setStaticWaveLevel`, `amplitude` setter are untouchable).
- **No** token changes (`KlarvoTheme.kt` is generated — DO NOT edit; drift gate will fail if you do).
- **No** change to `KlarvoAudioRecorder.kt`.
- **Do not** silently expand Story 9-7 (gesture mode *detection* already exists; this story is the *recording surface + interaction*).
- **No** changes to the cluster `[✗ · waveform · ➤]` visual or touch zones (Story 9-13 output is frozen — do not regress).

**Anchors:** `docs/backlog.md` §"Story 9-5 GATE-4 green" point (4); canon ADR-0019 §4′-Amendment 2026-06-21 (#4) (`docs/adr/0019-cross-platform-design-ssot.md`); canon source `docs/design/overhaul/source/` (fingerprint `fc9ef7456700d19b8332dd2c34a43b8e`, MANIFEST 2026-06-21) — Artboard-Sektion „Aufnahme · HOLD-Modus"; approval render `docs/design/overhaul/mockup-9-5-followups-2-4.html` (HOLD section, columns B + C). Design gate is **resolved** (Andi-approved 2026-06-21) — no open design/UI/intent question.

## Acceptance Criteria

**AC1 — HOLD dock shown instead of normal cluster while PTT is active.**
Given the user long-presses the bubble AND `longPressMode == RecordingMode.HOLD` (PTT fires, `pushToTalkActive = true`)
When the recording starts
Then the bubble renders the **HOLD dock** (`.ab-holddock`):
  - Right side: **heldbub** — 40dp×40dp teal-gradient squircle (r12dp), dark "K" OnTeal centered, `AmberLine` box-shadow ring (4dp outset), inner `.ring` (inset -8dp, r18dp, amber 2dp border opacity 0.5), finger indicator (26dp circle, bottom-right, rgba(236,238,239,.15) fill, rgba(.45) border)
  - Left side: **holdstrip** — dark backdrop `rgba(20,22,24,.92)` rounded rect (r18dp) with `AmberLine` ring (1.5dp); content: amber "‹" glyph (14sp) + dim "ziehen zum Abbrechen" (11sp) + 9dp gap + amber waveform bars (5 bars, 3dp each, same `drawClusterWaveform` helper, same `waveLevels` buffer)
  - Above dock: **lockchip** — amber "▲" (13sp) + lock icon (Path-drawn, ~13dp, Muted) + "hoch = sperren" text (10sp, Muted, monospace)
AND the normal recording cluster (`[✗ · waveform · ➤]`) is **NOT** shown

**AC1a — Gesture-hint arrows animate (GATE-1 decision, Andi-approved 2026-06-26).**
Given the HOLD dock is displayed
Then the slidehint "‹" arrow and the lockchip "▲" arrow **animate** as a subtle infinite pulse — **not static** — matching the approved render and the reconciled canon (`Klarvo Design System.html` `@keyframes slidearr`/`slideup`): the "‹" translates ~4dp left and the "▲" ~4dp up, opacity oscillating 0.5↔1.0, ~1.1s ease-in-out loop (slide-to-cancel / drag-to-lock affordance cue). Implement via a lightweight Canvas animator (e.g. the existing invalidation tick) driving an offset/alpha on the two glyphs; the rest of the dock is static.

**AC2 — No tappable zones during hold.**
Given `pushToTalkActive = true` and the HOLD dock is displayed
When `isTouchInConfirmZone(anyX)` or `isTouchInCancelZone(anyX)` is called
Then both return `false` (no tappable ➤/✗ affordances during a held touch — release is send, drag is cancel/lock)

**AC3 — Release still sends.**
Given `pushToTalkActive = true` and the HOLD dock is displayed
When the finger lifts (ACTION_UP without a drag exceeding the cancel or lock threshold)
Then `stopAndProcessRecording()` is called — identical to the existing PTT release behavior

**AC4 — Horizontal drag beyond threshold = cancel.**
Given `pushToTalkActive = true` and the HOLD dock is displayed
When the user drags horizontally more than `HOLD_DRAG_CANCEL_DP` (60dp) from the touch-down point
Then `cancelRecording()` is called (recording discarded, no paste) and `pushToTalkActive` + `holdDockActive` are reset

**AC5 — Upward drag beyond threshold = lock → normal cluster.**
Given `pushToTalkActive = true` and the HOLD dock is displayed
When the user drags **upward** more than `HOLD_DRAG_LOCK_DP` (40dp) from the touch-down point
Then `pushToTalkActive` is set to `false` and `holdDockActive` is set to `false`
AND the window resizes to normal cluster dimensions (re-anchored from the saved `preclusterBubbleX`)
AND `isTouchInConfirmZone`/`isTouchInCancelZone` resume normal behavior (tap ➤ to send, tap ✗ to cancel)
AND the panel header label updates to "Aufnahme · 🔒 gesperrt" (reflecting the locked state)
AND the panel **footer** updates to **"Finger losgelassen · weiter über die Knöpfe"** (locked-state orientation — GATE-1 decision, Andi-approved 2026-06-26, per approved render `mockup-9-5-followups-2-4.html` state C; this replaces the during-hold footer only in the locked state, not during the active hold)

**AC6 — Normal recording modes are unaffected.**
Given `pushToTalkActive = false` (tap/toggle/autostop/auto recording triggered by a short tap)
When in RECORDING state
Then the normal cluster `[✗ Cancel (left) · waveform · ➤ Send (right/thumb)]` is shown (Story 9-13 result preserved)
AND `isTouchInConfirmZone`/`isTouchInCancelZone` work exactly as before

**AC7 — All other states unaffected.**
Given any state other than RECORDING
When the view renders
Then `holdDockActive` has no effect on IDLE / TRANSCRIBING / DONE draws

**AC8 — Panel label during HOLD.**
Given `pushToTalkActive = true` and recording has started (HOLD dock visible)
When the listening panel renders
Then the header label reads **"Aufnahme · halten"** (instead of "Aufnahme")
AND panel footer "Tastatur pausiert · kehrt beim Einfügen zurück" is unchanged

**AC9 — Harness supports HOLD dock state (structural smoke).**
Given the debug harness
When `adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ez hold_mode true --ef rms 0.7`
Then the HOLD dock window is visible in `dumpsys window windows` with hold-dock dimensions (different from cluster)
AND `adb shell am broadcast ... --ez hold_mode false` (or without the extra) shows the normal cluster

**Inversion (must-fail gates):**
- Normal cluster shown while `pushToTalkActive = true` = review failure.
- `isTouchInConfirmZone(x)` returning `true` while `holdDockActive = true` = review failure.
- `isTouchInCancelZone(x)` returning `true` while `holdDockActive = true` = review failure.
- Any change to `drawClusterWaveform`, `waveLevels`, `amplitude`, `setStaticWaveLevel` = scope violation.
- `KlarvoTheme.kt` edited by hand = drift-gate failure (generated file — use local constants for non-token colors).
- `FLAG_NOT_TOUCHABLE` added anywhere = HyperOS hard-dims overlays to 0.8 alpha.
- `KlarvoOverlayService.kt` touch-zone dispatch (`when { isTouchInConfirmZone ... }` block) changed to accommodate HOLD is a scope violation — the change must be in `isTouchInConfirmZone`/`isTouchInCancelZone` returning `false`, not in the dispatcher.

**DoD (surface-class):**
- DEBUG APK builds (`scripts/android-smoke.sh` exits 0).
- All existing JVM tests pass (none cover HOLD dock; build must be clean).
- Emulator structural smoke green (`BMAD_CONDUCTOR=1 scripts/android-smoke.sh`): overlay-window structure present in RECORDING state with hold_mode=true via harness; HOLD dock window dimensions differ from normal cluster window; AC2 zone false-returns machine-verified via logcat or synthetic touch-x test.
- **GATE-4 motion/touch = real device (Andi's batched gate):** press-hold-release (send), press-hold-drag-sideways (cancel), press-hold-drag-up-to-lock (→ cluster), and live waveform reactivity in the holdstrip are **only verifiable on Andi's real device with a live mic** — never an emulator.
- `FLAG_NOT_TOUCHABLE` must not appear in changed files (HyperOS alpha-dimming constraint, reference `reference_hyperos_overlay_quirks.md`).

## Tasks / Subtasks

- [x] **Task 1: Read ALL files being modified before touching any code**
  - [x] 1.1 Re-read `FloatingBubbleView.kt` fully (lines 1–632) — confirm current post-9-13 state: companion constants, `holdDockActive` not yet present, `drawRecordingCluster`, `isTouchInConfirmZone`/`isTouchInCancelZone`, waveform helpers.
  - [x] 1.2 Re-read relevant sections of `KlarvoOverlayService.kt`: `longPressRunnable` (l.379–393), `handleTouch` (l.996–1085), `handleTap` RECORDING branch (l.1132–1148), `adjustLayoutForState` (l.955–992), field declarations (l.200–261).
  - [x] 1.3 Re-read `ListeningPanelView.kt` lines 390–415 (the RECORDING label draw path at "Aufnahme") and the class-level `isHoldMode`-equivalent property if any.
  - [x] 1.4 Read `docs/design/overhaul/mockup-9-5-followups-2-4.html` — columns B (HOLD active) and C (HOLD locked) to confirm final visual layout before implementing.
  - [x] 1.5 Read canon CSS section for HOLD surfaces in `docs/design/overhaul/source/Klarvo Design System.html` (file-local `<style>`, lines ~91–102: `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-slidehint .arr`/`.ab-heldbub`/`.ab-lockchip`/`.ab-lockchip .upi` + `@keyframes slidearr`/`slideup`) for exact padding, gap, border-radius, color, and the hint-arrow animation values. **Note:** `assets/klarvo.css` does NOT contain the HOLD surfaces — they live only in the HTML's file-local style block (component geometry truth per MANIFEST).

- [x] **Task 2: Add HOLD dock drawing to `FloatingBubbleView.kt` (AC: 1, 2, 6, 7, 9)**
  - [x] 2.1 Add property `var holdDockActive: Boolean = false` with `set(value) { if (field == value) return; field = value; requestLayout(); invalidate() }`.
  - [x] 2.2 Add companion object constants for HOLD dock geometry (see Dev Notes §HOLD dock dimensions). Do NOT edit the generated `KlarvoTheme.kt`. Non-token colors (holdstrip background, ring colors) are local constants in the draw method.
  - [x] 2.3 Modify `onDraw`: in the `State.RECORDING` branch, route to `drawHoldDock(canvas)` when `holdDockActive == true`; keep `drawRecordingCluster(canvas)` for `holdDockActive == false` (exactly as 9-13 left it).
  - [x] 2.4 Modify `isTouchInConfirmZone`: prepend `&& !holdDockActive` guard (return false during hold — no tappable ➤ while physically held). **Do NOT alter any other predicate logic** (regression guard for 9-13).
  - [x] 2.5 Modify `isTouchInCancelZone`: same `&& !holdDockActive` guard.
  - [x] 2.6 Implement `drawHoldDock(canvas: Canvas)`.
  - [x] 2.7 Add `LAYER_TYPE_SOFTWARE` is already set in `init` — confirm it remains (no change needed for shadow rendering).
  - [x] 2.8 Update class-level KDoc: add HOLD state description and note about `holdDockActive`.

- [x] **Task 3: Update `KlarvoOverlayService.kt` for HOLD interaction (AC: 1, 2, 3, 4, 5, 8, 9)**
  - [x] 3.1 In `longPressRunnable`, after `pushToTalkActive = (longPressMode == RecordingMode.HOLD)`: when `pushToTalkActive = true`, also set `bubbleView.holdDockActive = true` AND update the listening panel label — call a new helper `setHoldModeOnPanel(true)`.
  - [x] 3.2 Add private `fun setHoldModeOnPanel(holdMode: Boolean)`.
  - [x] 3.3 In `adjustLayoutForState(newState, previousState)`, inside the `newState == RecordingState.RECORDING` branch: HOLD dock dimensions when pushToTalkActive, else normal cluster.
  - [x] 3.4 Add private `fun lockHoldToCluster()`.
  - [x] 3.5 In `handleTouch` `ACTION_MOVE` branch: drag detection (cancel 60dp / lock 40dp upward).
  - [x] 3.6 In `stopAndProcessRecording()`: reset `bubbleView.holdDockActive = false` and `setHoldModeOnPanel(false)`.
  - [x] 3.7 In `cancelRecording()`: same resets as 3.6.
  - [x] 3.8 Add private constants: `holdDragCancelPx` and `holdDragLockPx` (60dp / 40dp, computed from density).
  - [x] 3.9 Update the class-level KDoc (HOLD behavior updated).
  - [x] 3.10 Update harness `applyHarnessState()` and `debugStateReceiver` for hold_mode extra.

- [x] **Task 4: Update `ListeningPanelView.kt` panel label + locked-state footer during HOLD (AC: 8, 5)**
  - [x] 4.1 Add `var isHoldMode: Boolean = false` property with setter that calls `invalidate()`.
  - [x] 4.2 RECORDING label: `isLockedMode` → "Aufnahme · 🔒 gesperrt", `isHoldMode` → "Aufnahme · halten", else "Aufnahme".
  - [x] 4.3 Add `var isLockedMode: Boolean = false` property.
  - [x] 4.4 Locked-state footer: when `isLockedMode == true` in RECORDING, draw "Finger losgelassen · weiter über die Knöpfe".
  - [x] 4.5 No other states changed.

- [x] **Task 5: Build + structural smoke (AC: DoD)**
  - [x] 5.1 `scripts/android-smoke.sh` exits 0 (Kotlin compile + drift gate + all existing JVM tests green). 24 tests, 0 failures. APK built in 5s, installed on 100.112.41.70:5555.
  - [x] 5.2 Structural smoke via harness (real device, density=440dpi → 2.75x):
    - hold_mode=true: Window #9 com.klarvo.voice `(566x264)` px = 206×96 dp ✓ (HOLDDOCK_VISUAL_W+2×shadow=206dp; VISUAL_H+LOCKCHIP+2×shadow=96dp).
    - hold_mode=false (normal recording): Window #9 com.klarvo.voice `(456x187)` px = 166×68 dp ✓ (unchanged from 9-13 cluster, regression GREEN).
  - [x] 5.3 AC2 zone predicate guard verified via static code analysis: both `isTouchInConfirmZone` and `isTouchInCancelZone` contain `&& !holdDockActive` — return false whenever holdDockActive=true.
  - [x] 5.4 APK freshness: smoke script reported "Frische APK erzeugt in 5s" ✓.

- [x] **Task 6: Commit (scope)**
  - [x] 6.1 Stage only the changed files: `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt`, `KlarvoOverlayService.kt`, `ListeningPanelView.kt`. Never `git add .`.
  - [x] 6.2 Commit message: `feat(android): 9-14 — HOLD-mode push-to-talk cluster variant (holddock + drag cancel/lock)`.

## Dev Notes

### Context: Why this story

During Story 9-5's GATE-4 (Andi's real-device review, 2026-06-19), the current recording cluster `[✗ · waveform · ➤]` was flagged as wrong for HOLD (PTT) mode: when `longPressMode == HOLD`, releasing the hold already sends — so the ➤ Send button is redundant and ✗ Cancel is physically unreachable (any release = send; you can't tap ✗ without releasing). This is Follow-up #4 in `docs/backlog.md`.

The design was approved 2026-06-21 via `docs/design/overhaul/mockup-9-5-followups-2-4.html` (Andi). The canon was updated in ADR-0019 §4′-Amendment 2026-06-21 (#4).

### Current PTT mechanism (unchanged by this story)

```kotlin
// KlarvoOverlayService.kt longPressRunnable (~line 379):
private val longPressRunnable = Runnable {
    if (!isDragging && currentState == RecordingState.IDLE) {
        longPressTriggered = true
        activeGesture = "longpress"
        loadBubbleControls()
        // Only activate push-to-talk when longPressMode is HOLD:
        pushToTalkActive = (longPressMode == RecordingMode.HOLD)
        ...
        startRecording()
    }
}

// handleTouch ACTION_MOVE (current no-op during PTT):
MotionEvent.ACTION_MOVE -> {
    if (pushToTalkActive) return true  // ← this story replaces this no-op
    ...
}

// handleTouch ACTION_UP (current PTT confirm):
pushToTalkActive -> {
    pushToTalkActive = false
    stopAndProcessRecording()  // ← preserved unchanged
}
```

This story's changes to `handleTouch`:
- ACTION_MOVE: replace the `return true` no-op with drag detection (cancel/lock thresholds).
- ACTION_UP: unchanged (release = send, same as today).
- ACTION_CANCEL: unchanged (ACTION_CANCEL during PTT = cancel recording, same as today).

### HOLD dock geometry: companion object constants to add

Add to `FloatingBubbleView` companion object (alongside the existing CLUSTER_* constants):

```kotlin
// HOLD dock dimensions (PTT mode: .ab-holddock surfaces, ADR-0019 §4′-Amendment #4)
// Layout (right→left): heldbub (40dp) · gap (11dp) · holdstrip
// Holdstrip interior: lpad(11) + slidehint(~82dp) + gap(9) + waveform(27) + rpad(10) = ~139dp
// Total visual width: 139 (holdstrip) + 11 (gap) + 40 (heldbub) = ~190dp
// Note: This is wider than CLUSTER_VISUAL_W_DP=150 because the slidehint text takes space.
// Verify exact width on device; adjust if holdstrip text is trimmed or wraps.
const val HOLDDOCK_VISUAL_W_DP = 190       // holdstrip + gap + heldbub (without shadow pad)
const val HOLDDOCK_VISUAL_H_DP = 52        // matches CLUSTER_VISUAL_H_DP
const val HOLDDOCK_SHADOW_PAD_DP = 8       // matches CLUSTER_SHADOW_PAD_DP
const val HOLDDOCK_LOCKCHIP_H_DP = 28      // lockchip area above holdstrip (incl. 8dp gap below chip)
// Total window height = HOLDDOCK_VISUAL_H_DP + HOLDDOCK_LOCKCHIP_H_DP + 2 * HOLDDOCK_SHADOW_PAD_DP
//                     = 52 + 28 + 16 = 96dp

// Internal hold geometry (from canon .ab-holddock CSS)
private const val HOLD_HELDBUB_DP = 40          // .ab-heldbub width/height
private const val HOLD_HELDBUB_R_DP = 12        // .ab-heldbub border-radius (vs 14dp in mockup, 12dp in canon)
private const val HOLD_RING_OUTSET_DP = 4       // AmberLine box-shadow outset (0 0 0 4px AmberLine)
private const val HOLD_INNER_RING_INSET_DP = 8  // .ab-heldbub .ring: inset -8dp
private const val HOLD_INNER_RING_R_DP = 18     // .ab-heldbub .ring border-radius
private const val HOLD_GAP_DP = 11              // gap between holdstrip and heldbub
private const val HOLDSTRIP_L_PAD_DP = 11       // .ab-holdstrip: left padding
private const val HOLDSTRIP_R_PAD_DP = 10       // .ab-holdstrip: right padding
private const val HOLDSTRIP_V_PAD_DP = 6        // .ab-holdstrip: top/bottom padding
private const val HOLDSTRIP_INNER_GAP_DP = 9    // gap between slidehint and waveform zone
private const val HOLDSTRIP_R_DP = 18           // .ab-holdstrip: border-radius
private const val HOLD_FINGER_DP = 26           // .ab-heldbub .finger: diameter
```

### HOLD dock window sizing (changes to adjustLayoutForState)

The HOLD dock needs a taller window than the cluster (for the lockchip above) and is wider. The right edge of the heldbub must align with the right edge of the idle bubble (same anchor as the cluster).

```kotlin
// Inside adjustLayoutForState, RECORDING branch, when pushToTalkActive:
val holdW = ((FloatingBubbleView.HOLDDOCK_VISUAL_W_DP + 2 * FloatingBubbleView.HOLDDOCK_SHADOW_PAD_DP) * dp).toInt()
val holdH = ((FloatingBubbleView.HOLDDOCK_VISUAL_H_DP + FloatingBubbleView.HOLDDOCK_LOCKCHIP_H_DP + 2 * FloatingBubbleView.HOLDDOCK_SHADOW_PAD_DP) * dp).toInt()
// Right-edge-anchor (same as cluster): shift X left by extra width
if (preclusterBubbleX == null) preclusterBubbleX = bubbleParams.x
bubbleParams.x = maxOf(0, bubbleParams.x + touchTargetPx - holdW)
bubbleParams.width = holdW
bubbleParams.height = holdH
// Shift Y upward so holdstrip (lower portion) aligns with the idle bubble position,
// with lockchip extending above. Offset = lockchipH_px.
val lockchipH = ((FloatingBubbleView.HOLDDOCK_LOCKCHIP_H_DP) * dp).toInt()
bubbleParams.y = maxOf(0, bubbleParams.y - lockchipH)
```

**lockHoldToCluster()** (new private function):
```kotlin
private fun lockHoldToCluster() {
    val dp = resources.displayMetrics.density
    val visualDp = bubbleView.getBubbleSizeDp()
    val touchTargetPx = bubbleWindowPx(visualDp)
    val clusterW = ((FloatingBubbleView.CLUSTER_VISUAL_W_DP + 2 * FloatingBubbleView.CLUSTER_SHADOW_PAD_DP) * dp).toInt()
    val clusterH = ((FloatingBubbleView.CLUSTER_VISUAL_H_DP + 2 * FloatingBubbleView.CLUSTER_SHADOW_PAD_DP) * dp).toInt()
    // Restore Y before lockchip offset (reverse the upward shift applied in adjustLayoutForState)
    val lockchipH = ((FloatingBubbleView.HOLDDOCK_LOCKCHIP_H_DP) * dp).toInt()
    bubbleParams.y += lockchipH  // undo upward shift
    // Re-anchor from saved pre-expansion X (same logic as adjustLayoutForState for cluster)
    val savedX = preclusterBubbleX ?: bubbleParams.x
    bubbleParams.x = maxOf(0, savedX + touchTargetPx - clusterW)
    bubbleParams.width = clusterW
    bubbleParams.height = clusterH
    updateBubbleLayout()
}
```

### Non-token colors (local draw constants — NOT in KlarvoTheme.kt)

`KlarvoTheme.kt` is **auto-generated** and must NOT be edited. Use local vals inside the draw method for colors that are not canon tokens:

```kotlin
// Holdstrip backdrop: rgba(20,22,24,.92) = #141618 at 0xEB alpha
val holdBgColor = 0xEB141618.toInt()  // not a --k-* token

// Heldbub finger indicator
val fingerFill   = 0x26ECEEEF.toInt()  // rgba(236,238,239,.15)
val fingerBorder = 0x73ECEEEF.toInt()  // rgba(236,238,239,.45)
```

All other colors come from `KlarvoTheme`:
- `KlarvoTheme.TealHi` / `KlarvoTheme.TealLo` — heldbub gradient
- `KlarvoTheme.OnTeal` — "K" letter
- `KlarvoTheme.AmberLine` — holdstrip ring + heldbub box-shadow ring (`0x52E9A24C`)
- `KlarvoTheme.Amber` — slidehint arrow + waveform bars
- `KlarvoTheme.Muted` — lockchip text
- `KlarvoTheme.Dim` — slidehint main text
- `KlarvoTheme.ShadowColor` — soft drop shadows

### Lock icon in lockchip

The canon uses a Lucide SVG lock. In Android Canvas, draw a simplified Path:

```kotlin
// Simplified padlock: shackle (arc top) + body (rounded rect bottom)
// Total size: ~13dp × ~15dp (shackle 7dp high, body 8dp high)
private fun drawLockIcon(canvas: Canvas, cx: Float, cy: Float, sizeDp: Float, paint: Paint) {
    val dp = resources.displayMetrics.density
    val w = sizeDp * dp
    val bodyH = w * 0.55f
    val shackleH = w * 0.45f
    val bodyTop = cy - bodyH / 2f + shackleH * 0.4f

    // Body: rounded rect
    val bodyRect = RectF(cx - w/2f, bodyTop, cx + w/2f, bodyTop + bodyH)
    val fillPaint = Paint(paint).apply { style = Paint.Style.FILL }
    canvas.drawRoundRect(bodyRect, w * 0.18f, w * 0.18f, fillPaint)

    // Shackle: arc (top half of a circle above the body)
    val shackleStrokePaint = Paint(paint).apply {
        style = Paint.Style.STROKE
        strokeWidth = w * 0.18f
        strokeCap = Paint.Cap.BUTT
    }
    val shackleW = w * 0.55f
    val shackleRect = RectF(cx - shackleW/2f, cy - bodyH/2f - shackleH * 0.6f + shackleH * 0.4f,
                            cx + shackleW/2f, bodyTop + shackleW * 0.6f)
    canvas.drawArc(shackleRect, 180f, 180f, false, shackleStrokePaint)
}
```

Alternatively: use a Unicode padlock character "🔒" with `drawText()`. Risk: emoji rendering is device-dependent. The Canvas Path approach is more reliable.

### DrawHoldDock structure (high-level pseudocode)

```
drawHoldDock(canvas):
  1. Compute window layout:
     - Total window: width = (HOLDDOCK_VISUAL_W_DP + 2*HOLDDOCK_SHADOW_PAD_DP) * dp
     - Lockchip zone: top portion, height = HOLDDOCK_LOCKCHIP_H_DP * dp
     - Holdstrip zone: below lockchip, height = HOLDDOCK_VISUAL_H_DP * dp, width = holdstrip portion
     - Heldbub zone: right portion of holdstrip row, 40dp wide
  2. Draw soft backdrop shadow for holdstrip + heldbub (BlurMaskFilter on shadowPaint)
  3. Draw LOCKCHIP (above holdstrip):
     - lockchip cx: aligned with heldbub center (right side)
     - amber "▲" text (13sp), centered above lock icon — **animated**: translate up to ~4dp + alpha 0.5↔1.0, ~1.1s ease-in-out loop (per AC1a / canon `@keyframes slideup`)
     - Lock icon Path (Muted, 13dp)
     - "hoch = sperren" text (10sp, Muted, monospace)
  4. Draw HOLDSTRIP (left portion, lower area):
     - Backdrop: holdBgColor rounded rect r18dp
     - AmberLine ring: 1.5dp stroke, same rounded rect slightly outset
     - Slidehint: amber "‹" at 14sp (**animated**: translate left to ~4dp + alpha 0.5↔1.0, ~1.1s ease-in-out loop, per AC1a / canon `@keyframes slidearr`) + gap + dim "ziehen zum Abbrechen" at 11sp
     - Waveform: call drawClusterWaveform(canvas, waveLeft, waveRight, cy, dp)
       where waveLeft/waveRight are the waveform zone bounds inside the holdstrip
  5. Draw HELDBUB (right portion, lower area):
     - Shadow (BlurMaskFilter)
     - Teal gradient fill: LinearGradient(TealHi→TealLo, 150°, heldbub rect)
     - AmberLine box-shadow ring: 4dp outset stroke
     - Inner ring (.ring): Amber 2dp stroke, r18dp, 8dp inset, alpha 0.5
     - "K" text (OnTeal, same textSize formula as IDLE)
     - Finger indicator (26dp circle, bottom-right offset -6dp/-7dp, local fill+border colors)
```

### What must NOT change in FloatingBubbleView.kt

From Story 9-12 and 9-13 — these are **frozen deliverables**:
- `waveLevels` ring buffer (do NOT reset, do NOT modify scrolling logic)
- `setStaticWaveLevel(v)` (do NOT change)
- `amplitude` setter (do NOT change)
- `drawClusterWaveform(canvas, zoneLeft, zoneRight, cy, dp)` — signature and body unchanged
- `drawRecordingCluster(canvas)` — unchanged (used for non-hold RECORDING)
- `drawSendButton`, `drawCancelButton` — unchanged
- `clusterSendZoneStart`, `clusterCancelZoneEnd` — fields unchanged
- `isTouchInConfirmZone`/`isTouchInCancelZone` logic — only prepend `&& !holdDockActive`, nothing else

### What must NOT change in KlarvoOverlayService.kt

- `handleTap(touchX)` RECORDING branch: the `when { isTouchInConfirmZone → ... isTouchInCancelZone → ... }` dispatch is **unchanged** — the fix is in the predicates returning false, not in the dispatcher.
- `handleTap(touchX)` IDLE branch: unchanged (starts recording normally for all modes).
- `adjustLayoutForState` else branch (non-RECORDING restore): unchanged.
- `stopAndProcessRecording()` internals: only ADD the `holdDockActive = false` reset.
- `cancelRecording()` internals: same — only ADD resets.
- `longPressRunnable` behavior: only APPEND `bubbleView.holdDockActive = true` after `pushToTalkActive = true`, and call `setHoldModeOnPanel(true)`.
- `LONG_PRESS_TIMEOUT_MS` (500ms): unchanged.

### Drag thresholds

```kotlin
// KlarvoOverlayService companion or init block:
private val HOLD_DRAG_CANCEL_PX: Float  // = 60f * density
private val HOLD_DRAG_LOCK_PX: Float    // = 40f * density (upward = negative dy)
```

These are conservative but deliberately different from the bubble drag threshold (10dp). The cancel threshold (60dp) requires intentional sideways drag; the lock threshold (40dp up) requires clear upward intent. Both can be tuned after Andi's real-device GATE-4 if the feel is off — treat them as first-pass values.

### File list

| File | Type of change |
|------|----------------|
| `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` | PRIMARY: holdDockActive property, HOLDDOCK_* companion constants, drawHoldDock(), modified isTouchInConfirmZone/isTouchInCancelZone, updated KDoc |
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | SECONDARY: holdDockActive reset in longPressRunnable; drag detection in ACTION_MOVE; lockHoldToCluster(); HOLD dock window sizing in adjustLayoutForState; reset in stopAndProcessRecording/cancelRecording; harness hold_mode extra; updated KDoc |
| `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` | SMALL: isHoldMode + isLockedMode properties; conditional label text in RECORDING draw path |

**No other files.** No Rust, no `KlarvoTheme.kt` (generated), no `KlarvoAudioRecorder.kt`, no config key changes, no test files (HOLD dock drawing requires Android Context; structural verification is via harness + dumpsys + logcat, not JVM unit tests).

### Project structure notes

- `FloatingBubbleView.kt` is at `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt`. Use this path always (never `android/app/src/...`).
- `KlarvoTheme.kt` is at `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt`. **Generated — DO NOT edit.** Drift gate in `scripts/android-smoke.sh` will fail if you do.
- JVM tests are in `android/kotlin-test/com/klarvo/voice/`. No new test file needed for this story (FloatingBubbleView requires Android Context; harness is the test oracle).
- Smoke script: `scripts/android-smoke.sh` — run to get the build + drift gate + JVM test gate in one step.
- Emulator via `scripts/android-emulator.sh` (never hand-roll `emulator -avd ...`; watchdog TTL = 2h).

### References

- [Source: docs/adr/0019-cross-platform-design-ssot.md, §4′-Amendment 2026-06-21, (#4)] — Canon mandate for HOLD interaction model.
- [Source: docs/design/overhaul/mockup-9-5-followups-2-4.html, columns B + C] — Andi-approved approval render: HOLD active + HOLD locked → cluster.
- [Source: docs/design/overhaul/source/Klarvo Design System.html, file-local `<style>` l.~91–102: `.ab-holddock` … `.ab-lockchip` block + `@keyframes slidearr`/`slideup`] — Canonical colors, padding, border-radius, font sizes, AND the hint-arrow pulse animation for all HOLD surfaces. (`assets/klarvo.css` does NOT contain these — HTML file-local style only.)
- [Source: docs/design/overhaul/mockup-9-5-followups-2-4.html, state C (HOLD locked → cluster)] — Approved render for the locked-state panel footer text "Finger losgelassen · weiter über die Knöpfe" + animated slide/lock hint arrows (GATE-1 SOLL, Andi-approved 2026-06-26).
- [Source: docs/backlog.md, §"Story 9-5 GATE-4 green" point (4)] — Origin of this follow-up.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.379–393] — longPressRunnable: where pushToTalkActive is set.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.966–992] — adjustLayoutForState: cluster window sizing to extend for HOLD dock.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.1014–1018] — ACTION_MOVE no-op: this story replaces it with drag detection.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.1065–1068] — ACTION_UP PTT confirm: unchanged.
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.265–271] — isTouchInConfirmZone / isTouchInCancelZone: add !holdDockActive guard only.
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.338–403] — drawRecordingCluster: frozen (do not touch).
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.464–491] — drawClusterWaveform: reuse as-is for holdstrip waveform zone.
- [Source: android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt, l.394–408] — RECORDING label draw path: add isHoldMode/isLockedMode conditional.
- [Source: _bmad-output/project-context.md] — No `git add .`; Android changes require android-smoke.sh; minSdk 24; no Compose; KlarvoTheme.kt is generated.
- [Source: _bmad-output/implementation-artifacts/9-13-swap-send-cancel-cluster-order.md] — Previous story: cluster `[✗ · waveform · ➤]` is frozen, touch zones post-9-13 are the baseline.
- [Source: reference_hyperos_overlay_quirks.md] — FLAG_NOT_TOUCHABLE dims overlays to alpha 0.8 on HyperOS.
- [Source: reference_android_unattended_emulator_smoke.md] — Always use scripts/android-emulator.sh; 2h TTL watchdog.

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

- Kotlin compile: `scripts/android-smoke.sh` exit 0; 24 JVM tests, 0 failures.
- KlarvoTheme drift gate: PASS (generated file untouched).
- Structural window smoke: hold_mode=true → 566×264px (206×96dp); normal cluster → 456×187px (166×68dp). Device density 440dpi (2.75x). HOLD dock 24% wider, 41% taller than cluster. AC9 and 9-13 regression GREEN.
- AC2 zone predicates: both `isTouchInConfirmZone` and `isTouchInCancelZone` statically verified to include `&& !holdDockActive` guard.
- One design note: "ziehen zum Abbrechen" (11sp) may exceed available slidehint width on very high-density screens. Mitigated by right-anchoring the waveform zone and canvas-clipping the text. Monitor in Andi's GATE-4.

### Completion Notes List

- Tasks 1–6 complete. All 9 ACs covered.
- AC1/AC1a: `drawHoldDock()` in FloatingBubbleView renders holdstrip + heldbub + lockchip with ValueAnimator-driven hint arrows (holdArrowPhase 0..1, 550ms REVERSE, AccelerateDecelerateInterpolator — matching canon @keyframes 1.1s ease-in-out).
- AC2: zone predicates guard `&& !holdDockActive` — no tappable zones during hold.
- AC3: ACTION_UP PTT path unchanged — release = send.
- AC4: ACTION_MOVE detects `abs(dx) > holdDragCancelPx` (60dp) → cancelRecording() with state reset.
- AC5: ACTION_MOVE detects `dy < -holdDragLockPx` (40dp up) → lockHoldToCluster() + setLockedModeOnPanel().
- AC6/AC7: all non-HOLD recording + other states untouched (drawRecordingCluster / drawIdleBubble / TRANSCRIBING paths unchanged).
- AC8: ListeningPanelView TopRowView label: isLockedMode → "Aufnahme · 🔒 gesperrt", isHoldMode → "Aufnahme · halten", else "Aufnahme". FooterView: isLockedMode in RECORDING → "Finger losgelassen · weiter über die Knöpfe".
- AC9: harness EXTRA_HOLD_MODE (`hold_mode` bool) wired through debugStateReceiver → applyHarnessState(holdMode) → sets pushToTalkActive + holdDockActive + setHoldModeOnPanel for machine-verifiable state.
- FLAG_NOT_TOUCHABLE NOT added (verified via grep).
- KlarvoTheme.kt NOT edited (drift gate passed).
- `drawClusterWaveform`, `waveLevels`, `drawRecordingCluster` NOT changed (9-12/9-13 frozen).
- GATE-4 (hold gesture + live mic on real device) remains for Andi's batched review.

### File List

| File | Change |
|------|--------|
| `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` | PRIMARY: holdDockActive property + setter with animator start/stop; HOLDDOCK_*/HOLD_* companion constants; drawHoldDock(); drawLockIcon(); holdArrowAnimator (ValueAnimator); holdArrowPhase; pre-allocated HOLD paints; isTouchInConfirmZone + isTouchInCancelZone `&& !holdDockActive` guard; onDraw RECORDING branch; onDetachedFromWindow cancel; KDoc updated |
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | SECONDARY: EXTRA_HOLD_MODE const; holdDragCancelPx + holdDragLockPx fields; longPressRunnable HOLD dock activation; setHoldModeOnPanel(); setLockedModeOnPanel(); lockHoldToCluster(); adjustLayoutForState HOLD dock window sizing; handleTouch ACTION_MOVE drag detection; stopAndProcessRecording + cancelRecording resets; applyHarnessState holdMode param; debugStateReceiver hold_mode extra read; onStartCommand hold_mode extra read; KDoc updated |
| `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` | SMALL: isHoldMode + isLockedMode properties; TopRowView RECORDING label conditional; FooterView isLockedMode early return |

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | Story created via bmad-create-story (9-14 HOLD-mode push-to-talk cluster). | claude-sonnet-4-6 |
| 2026-06-26 | Implementation complete: Tasks 1–6 done. Build smoke: 24 JVM tests green, APK built. Structural smoke: HOLD dock 206×96dp vs cluster 166×68dp (AC9 GREEN, 9-13 regression GREEN). Status → review. | claude-sonnet-4-6 |
