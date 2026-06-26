# Story 9.14: HOLD-Modus (Push-to-Talk) Bubble-Cluster-Variante

Status: ready-for-dev

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

- [ ] **Task 1: Read ALL files being modified before touching any code**
  - [ ] 1.1 Re-read `FloatingBubbleView.kt` fully (lines 1–632) — confirm current post-9-13 state: companion constants, `holdDockActive` not yet present, `drawRecordingCluster`, `isTouchInConfirmZone`/`isTouchInCancelZone`, waveform helpers.
  - [ ] 1.2 Re-read relevant sections of `KlarvoOverlayService.kt`: `longPressRunnable` (l.379–393), `handleTouch` (l.996–1085), `handleTap` RECORDING branch (l.1132–1148), `adjustLayoutForState` (l.955–992), field declarations (l.200–261).
  - [ ] 1.3 Re-read `ListeningPanelView.kt` lines 390–415 (the RECORDING label draw path at "Aufnahme") and the class-level `isHoldMode`-equivalent property if any.
  - [ ] 1.4 Read `docs/design/overhaul/mockup-9-5-followups-2-4.html` — columns B (HOLD active) and C (HOLD locked) to confirm final visual layout before implementing.
  - [ ] 1.5 Read canon CSS section for HOLD surfaces in `docs/design/overhaul/source/Klarvo Design System.html` (file-local `<style>`, lines ~91–102: `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-slidehint .arr`/`.ab-heldbub`/`.ab-lockchip`/`.ab-lockchip .upi` + `@keyframes slidearr`/`slideup`) for exact padding, gap, border-radius, color, and the hint-arrow animation values. **Note:** `assets/klarvo.css` does NOT contain the HOLD surfaces — they live only in the HTML's file-local style block (component geometry truth per MANIFEST).

- [ ] **Task 2: Add HOLD dock drawing to `FloatingBubbleView.kt` (AC: 1, 2, 6, 7, 9)**
  - [ ] 2.1 Add property `var holdDockActive: Boolean = false` with `set(value) { if (field == value) return; field = value; requestLayout(); invalidate() }`.
  - [ ] 2.2 Add companion object constants for HOLD dock geometry (see Dev Notes §HOLD dock dimensions). Do NOT edit the generated `KlarvoTheme.kt`. Non-token colors (holdstrip background, ring colors) are local constants in the draw method.
  - [ ] 2.3 Modify `onDraw`: in the `State.RECORDING` branch, route to `drawHoldDock(canvas)` when `holdDockActive == true`; keep `drawRecordingCluster(canvas)` for `holdDockActive == false` (exactly as 9-13 left it).
  - [ ] 2.4 Modify `isTouchInConfirmZone`: prepend `&& !holdDockActive` guard (return false during hold — no tappable ➤ while physically held). **Do NOT alter any other predicate logic** (regression guard for 9-13).
  - [ ] 2.5 Modify `isTouchInCancelZone`: same `&& !holdDockActive` guard.
  - [ ] 2.6 Implement `drawHoldDock(canvas: Canvas)`:
    - Compute window geometry from companion constants + density (see Dev Notes §Window sizing).
    - Draw **lockchip** in the upper portion of the window: amber "▲" text + lock-icon Path (draw a simplified padlock: small arc for shackle + rounded rect body, Muted color, ~13dp) + "hoch = sperren" text below (10sp, Muted, monospace via `Typeface.MONOSPACE`).
    - Draw **holdstrip** backdrop (dark fill `0xEB141618.toInt()`, r18dp, `AmberLine` ring stroke 1.5dp) in the lower-left zone.
    - Draw **slidehint** inside holdstrip: amber "‹" glyph (14sp) on left, then gap, then dim "ziehen zum Abbrechen" (11sp, Dim color) — both left-aligned in the holdstrip.
    - Draw **waveform** in the holdstrip to the right of the slidehint: call `drawClusterWaveform(canvas, waveLeft, waveRight, cy, dp)` using the waveform zone bounds inside the holdstrip — waveLeft/waveRight derived from holdstrip geometry. **Do NOT change `drawClusterWaveform` internals.**
    - Draw **heldbub** to the right of the holdstrip + gap: teal gradient (TealHi→TealLo, 150°), "K" OnTeal, 12dp corner radius, 40dp visual size — reuse the same gradient paint as IDLE.
    - Draw heldbub **amber ring** (box-shadow equivalent): `AmberLine` stroke 4dp, offset 4dp outset as a rounded rect around the heldbub.
    - Draw heldbub **inner ring** (`.ab-heldbub .ring`): r18dp, Amber border 2dp, alpha 0.5, inset -8dp from heldbub bounds.
    - Draw heldbub **finger indicator**: 26dp circle, bottom-right of heldbub at (-6dp, -7dp) offset, fill `0x26ECEEEF.toInt()`, stroke 1.5dp `0x73ECEEEF.toInt()`.
  - [ ] 2.7 Add `LAYER_TYPE_SOFTWARE` is already set in `init` — confirm it remains (no change needed for shadow rendering).
  - [ ] 2.8 Update class-level KDoc: add HOLD state description and note about `holdDockActive`.

- [ ] **Task 3: Update `KlarvoOverlayService.kt` for HOLD interaction (AC: 1, 2, 3, 4, 5, 8, 9)**
  - [ ] 3.1 In `longPressRunnable`, after `pushToTalkActive = (longPressMode == RecordingMode.HOLD)`: when `pushToTalkActive = true`, also set `bubbleView.holdDockActive = true` AND update the listening panel label — call a new helper `setHoldModeOnPanel(true)`.
  - [ ] 3.2 Add private `fun setHoldModeOnPanel(holdMode: Boolean)`: sets `panelView?.isHoldMode = holdMode` (if the panel is visible; no-op otherwise). Used in 3.1 and 3.5.
  - [ ] 3.3 In `adjustLayoutForState(newState, previousState)`, inside the `newState == RecordingState.RECORDING` branch:
    - If `pushToTalkActive` (HOLD dock): compute HOLD dock window dimensions (see Dev Notes §Window sizing); shift `bubbleParams.y` upward by `lockchipH_px` so the holdstrip aligns with the idle bubble's position (clamped to ≥0); apply right-edge-anchor for X using `HOLDDOCK_VISUAL_W_DP` total.
    - Else (normal cluster): existing code unchanged.
  - [ ] 3.4 Add private `fun lockHoldToCluster()`: re-anchors the window from `preclusterBubbleX` to normal cluster dimensions and calls `updateBubbleLayout()`. Restores Y to its pre-hold position (reverse the lockchip upward shift). Called in the upward-drag lock path.
  - [ ] 3.5 In `handleTouch` `ACTION_MOVE` branch: when `pushToTalkActive = true`:
    - Replace the `return true` no-op with actual drag detection.
    - Compute `dx = event.rawX - dragTouchStartX`, `dy = event.rawY - dragTouchStartY`.
    - If `abs(dx) > HOLD_DRAG_CANCEL_PX` (60dp × density): call `cancelRecording()`, reset `pushToTalkActive = false`, `bubbleView.holdDockActive = false`, `setHoldModeOnPanel(false)`, `return true`.
    - If `dy < -HOLD_DRAG_LOCK_PX` (upward; -40dp × density): call `lockHoldToCluster()`, set `pushToTalkActive = false`, `bubbleView.holdDockActive = false`, `setHoldModeOnPanel(false)`, update panel label to "Aufnahme · 🔒 gesperrt" — call `setLockedModeOnPanel()`, `return true`.
    - Otherwise: `return true` (still holding — no position update during hold, consistent with prior behavior).
  - [ ] 3.6 In `stopAndProcessRecording()` (or the path that clears recording state): reset `bubbleView.holdDockActive = false` and `setHoldModeOnPanel(false)`.
  - [ ] 3.7 In `cancelRecording()`: same resets as 3.6.
  - [ ] 3.8 Add private constants: `HOLD_DRAG_CANCEL_PX` and `HOLD_DRAG_LOCK_PX` — computed from dp at init time (or compute inline using density). 60dp and 40dp respectively.
  - [ ] 3.9 Update the class-level KDoc (line 33): correct the outdated `HOLD: Tap -> bar with [X][waveform][✓], Long-press -> PTT` comment to match the new Modell-B HOLD behavior.
  - [ ] 3.10 Update harness `applyHarnessState()` and `debugStateReceiver`: when `state == "recording"` and `intent.getBooleanExtra("hold_mode", false) == true`, set `bubbleView.holdDockActive = true` and `panelView?.isHoldMode = true` (and set `pushToTalkActive = true` for correct zone behavior). When `hold_mode` is absent or false, leave both false.

- [ ] **Task 4: Update `ListeningPanelView.kt` panel label + locked-state footer during HOLD (AC: 8, 5)**
  - [ ] 4.1 Add `var isHoldMode: Boolean = false` property with setter that calls `invalidate()`.
  - [ ] 4.2 In the RECORDING state label draw path (`canvas.drawText("Aufnahme", ...)` around line 407): replace the hardcoded string with `if (isHoldMode) "Aufnahme · halten" else "Aufnahme"`.
  - [ ] 4.3 Add `var isLockedMode: Boolean = false` property (same pattern) — used for the post-lock "Aufnahme · 🔒 gesperrt" label. In the label draw: check `isLockedMode` first, then `isHoldMode`, then default "Aufnahme".
  - [ ] 4.4 **Locked-state footer (GATE-1 / AC5):** in the RECORDING-state footer draw path (the "Tastatur pausiert · kehrt beim Einfügen zurück" text), when `isLockedMode == true` draw **"Finger losgelassen · weiter über die Knöpfe"** instead. Only in the locked state — during active hold (`isHoldMode`, not locked) and all other states the footer stays unchanged. (Approved render: `mockup-9-5-followups-2-4.html` state C.) `setLockedModeOnPanel()` (Task 3) must set `isLockedMode = true` so this and the label both fire.
  - [ ] 4.5 Do NOT change the TRANSCRIBING label ("Bereinigt…"), the timer, the grab-handle, the K badge, or the footer text in any state **other than** the locked state (4.4).

- [ ] **Task 5: Build + structural smoke (AC: DoD)**
  - [ ] 5.1 `scripts/android-smoke.sh` exits 0 (Kotlin compile + drift gate + all existing JVM tests green).
  - [ ] 5.2 Emulator structural smoke via harness:
    - `adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ez hold_mode true --ef rms 0.5` → `dumpsys window windows` confirms hold-dock window is present with different (wider/taller) dimensions than the cluster window.
    - `adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.5` (no hold_mode) → `dumpsys window windows` confirms normal cluster window dimensions (regression check for 9-13).
  - [ ] 5.3 Machine-verify AC2 zone behavior via logcat: in the harness hold_mode recording state, inject touch events or call the predicates directly via a test broadcast — confirm `isTouchInConfirmZone` and `isTouchInCancelZone` return false.
  - [ ] 5.4 Confirm APK freshness via build script timestamp gate (no in-UI version screen).

- [ ] **Task 6: Commit (scope)**
  - [ ] 6.1 Stage only the changed files: `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt`, `KlarvoOverlayService.kt`, `ListeningPanelView.kt`. Never `git add .`.
  - [ ] 6.2 Commit message: `feat(android): 9-14 — HOLD-mode push-to-talk cluster variant (holddock + drag cancel/lock)`.

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

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

### File List

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | Story created via bmad-create-story (9-14 HOLD-mode push-to-talk cluster). | claude-sonnet-4-6 |
