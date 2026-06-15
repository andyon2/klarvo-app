# Story 9.3: Bubble Idle Re-Skin + Responsive Sizing + Anchoring

Status: review

## Story

As a user with a focused text field,
I want the idle bubble to look right and sit correctly at any screen size,
so that it's reachable and unobtrusive.

## Acceptance Criteria

**AC1 — Teal "K" + glass ring replaces old white/icon idle rendering:**
Given the bubble is in idle state
When it is shown on screen
Then `FloatingBubbleView` draws a teal "K" letter centered in the bubble using `KlarvoTheme.Teal` (not the current white circle + app icon)
And a 4dp glass ring is drawn around the circle edge using a solid `KlarvoTheme.Surface` fill + a 4dp `KlarvoTheme.Teal` ring stroke (no `RenderEffect` blur — constraint AR5/DT4)
And the old `colorIdleBackground` (#F5F5F5 white), `drawIdleIcon()`, and `drawMicIconFallback()` methods are no longer rendered in IDLE state

**AC2 — Same bubble form across all states (no circle↔bar morph in idle):**
Given any state (idle / recording / processing)
When the bubble is visible
Then it stays in **circular** form during idle and processing — RECORDING bar mode (`BAR_WIDTH_DP = 220`) continues to work exactly as today for HOLD mode
And no new shape changes are introduced: the spec says "same form — no circle↔square morph"; the existing bar for RECORDING is out of scope for this story

**AC3 — Responsive size: visual = clamp(36dp, 0.11 × min(screenW, screenH)dp, 44dp):**
Given varying screen sizes
When the bubble is set up in `setupBubble()` / `reloadBubbleAppearance()`
Then the bubble's visual (drawn) size is computed as:
  `visualDp = clamp(36, (0.11 * min(screenW_dp, screenH_dp)).toInt(), 44)`
  where `screenW_dp` and `screenH_dp` are the screen dimensions in dp (pixels ÷ density)
And `setBubbleSize(visualDp)` is called with this computed value instead of `(BASE_BUBBLE_SIZE_DP * sizeScale)`
And the existing `bubbleSize` config scale factor is still respected: the clamp formula is applied to `BASE_BUBBLE_SIZE_DP * sizeScale` before clamping, OR the clamp replaces the scale factor — **decision: apply the spec formula directly (`0.11 × min(...)`) and ignore the `bubbleSize` scale multiplier** (the spec formula takes over responsive sizing; the scale setting becomes a no-op for this story — document this in Dev Notes)

**AC4 — Touch target ≥ 48dp via transparent padding:**
Given the computed `visualDp` (which may be 36–44dp, below the 48dp touch target)
When the bubble is displayed
Then the `WindowManager.LayoutParams` dimensions are set to `max(visualDp, 48) dp` in pixels (the full touch-target size)
And `FloatingBubbleView` draws the visible bubble centered within the larger touch-target bounds (the drawing origin must be offset to center the visual circle within the 48dp box)
And the touch-target padding area is transparent (the background remains transparent via `PixelFormat.TRANSLUCENT`)

**AC5 — Drag, edge-snap, remembered-side preserved and correct:**
Given the bubble is dragged
When the finger is released
Then the bubble edge-snaps: it slides to the nearest horizontal edge (left or right), maintaining its vertical position
And `PREF_X` and `PREF_Y` are saved so position is restored on next launch
And "remembered side" means the bubble stays on the side where it was last released (left or right), not at the raw pixel X coordinate

**AC6 — Keyboard jump-up (bubble position adjusts with keyboard):**
Given the keyboard opens
When the bubble Y position would be covered by the keyboard
Then the bubble Y is adjusted upward so it remains at least `NAV_BAR_CLEARANCE_PX = 56` px above the keyboard top
And nav-bar clearance uses fixed 56px, NOT `env(safe-area-inset-bottom)` (per AR5d — that value is unreliable/0 in the Android WebView; this is native Kotlin handling its own insets)

**AC7 — On-device smoke passes; APK freshness verified:**
Given all changes are complete
When `scripts/android-smoke.sh` is run
Then the APK builds cleanly and installs on device
And the bubble appears on field focus showing the teal "K" + glass ring idle state (not the old white circle + icon)
And drag, snap, and position memory work as expected on-device
And APK freshness verified via `scripts/android-build.sh` (or smoke) timestamp gate

**Inversion (must-fail gate):** A submission that still uses `drawIdleIcon()` / `drawMicIconFallback()` for the IDLE render path, or that uses `Color.parseColor("#F5F5F5")` as the idle background, must not pass review. A submission that ignores touch-target expansion (always drawing the same size as the touch area) must not pass review. A submission that saves the raw drag X without edge-snapping must not pass review.

**DoD:** On-device smoke that the bubble appears correctly (teal K + ring), correct size on a real phone, drag/snap/side-memory work; APK freshness verified.

## Tasks / Subtasks

- [x] **Task 1: Re-skin IDLE render in FloatingBubbleView.kt** (AC: 1, 2)
  - [x] 1.1 Add `import android.graphics.Typeface` and `import android.graphics.Paint.Align` (already imported via `*` but make explicit if needed)
  - [x] 1.2 Replace `colorIdleBackground` field with `KlarvoTheme`-based paints for the new idle rendering:
    - `idleCirclePaint`: `style = FILL`, `color = KlarvoTheme.Surface` (the glass fill — solid dark surface, not real blur)
    - `idleRingPaint`: `style = STROKE`, `color = KlarvoTheme.Teal`, `strokeWidth` = 4dp × density (computed in `onDraw` or `init`)
    - `kLetterPaint`: `style = FILL`, `color = KlarvoTheme.Teal`, `textAlign = CENTER`, `typeface = Typeface.DEFAULT_BOLD` (Geist is not yet wired to Canvas text — use bold system font for now; Geist font consumer is Story 9.5+)
  - [x] 1.3 Replace the `State.IDLE` arm in `onDraw()`:
    - Draw shadow: `canvas.drawCircle(cx, cy + radius * 0.06f, radius * 0.92f, shadowPaint)` (keep existing shadow paint)
    - Draw surface fill: `canvas.drawCircle(cx, cy, radius, idleCirclePaint)` (dark surface)
    - Draw teal ring: `canvas.drawCircle(cx, cy, radius - ringStrokeHalf, idleRingPaint)` (stroke inset by half stroke width so ring is fully inside the circle bounds)
    - Draw "K": set `kLetterPaint.textSize = radius * 0.65f`; `canvas.drawText("K", cx, cy + radius * 0.22f, kLetterPaint)` (the +0.22f baseline correction centers the capital letter visually; tune as needed)
  - [x] 1.4 Remove (or leave unused but do NOT call in IDLE): `drawIdleIcon()`, `drawMicIconFallback()`, `appIconDrawable`. Keep the `appIconDrawable` field and helper methods if removing them would require restructuring — just stop calling them from IDLE `onDraw`. Do not delete them in this story (9.4+ may or may not need them).
  - [x] 1.5 Remove `colorIdleBackground`, `colorRecordingBar`, `colorCancelBtn`, `colorConfirmBtn`, `colorProcessing` fields — replace with `KlarvoTheme` constants directly in the paint setup:
    - `circlePaint` in RECORDING_PTT: use `KlarvoTheme.Danger` (red/stop per DT5 semantics)
    - `circlePaint` in PROCESSING: use `KlarvoTheme.Teal` (brand/processing per DT5; NOT amber — amber = recording-tally only)
    - `colorCancelBtn` → `KlarvoTheme.Danger`; `colorConfirmBtn` → keep green? No — 9.3 scope is IDLE only; leave RECORDING bar colors as-is or migrate to `KlarvoTheme` as part of the same commit but do NOT change bar behavior
    - **Scope guidance:** Migrate ALL hardcoded `Color.parseColor(...)` calls to `KlarvoTheme` in this story — AC1 says "replace old rendering"; the PROCESSING and RECORDING_PTT states can be migrated to use `KlarvoTheme` constants (Teal for processing, Danger for recording bar) as part of cleanup, but their visual layout is NOT changed (just the color source)

- [x] **Task 2: Responsive size + touch-target expansion in KlarvoOverlayService.kt** (AC: 3, 4)
  - [x] 2.1 Add private helper `computeVisualSizeDp(): Int`:
    ```kotlin
    private fun computeVisualSizeDp(): Int {
        val dm = resources.displayMetrics
        val screenWdp = dm.widthPixels / dm.density
        val screenHdp = dm.heightPixels / dm.density
        val rawDp = (0.11f * minOf(screenWdp, screenHdp)).toInt()
        return rawDp.coerceIn(36, 44)
    }
    ```
    (Note: `getScreenDimensions()` returns pixels; divide by `density` to get dp. Alternatively call `getScreenDimensions()` and convert — either way is fine, but use density from `resources.displayMetrics.density`.)
  - [x] 2.2 In `setupBubble()`, replace `val sizeDp = (BASE_BUBBLE_SIZE_DP * sizeScale).toInt().coerceAtLeast(24)` with `val sizeDp = computeVisualSizeDp()` (the spec formula takes over; existing `bubbleSize` scale factor from config is superseded for now)
  - [x] 2.3 In `reloadBubbleAppearance()` (line ~1322), replace the same `newSizeDp` computation with `computeVisualSizeDp()`
  - [x] 2.4 Touch-target expansion: after calling `setBubbleSize(sizeDp)`, compute touch target:
    ```kotlin
    val touchTargetDp = maxOf(sizeDp, 48)
    val touchTargetPx = (touchTargetDp * dm.density).toInt()
    ```
    Set `bubbleParams.width = touchTargetPx` and `bubbleParams.height = touchTargetPx` (instead of `WRAP_CONTENT` for IDLE). The `FloatingBubbleView` will be sized to the touch-target box, and it must draw the visual circle centered within it.
  - [x] 2.5 **FloatingBubbleView must draw centered within the touch-target box:** In `FloatingBubbleView.onMeasure()`, the view is now given the touch-target dimensions. In `onDraw()`, compute the visual circle radius as `visualRadius = min(visualCirclePx, w, h) / 2 * (visualDp / touchTargetDp)` — **simpler approach:** pass `visualDp` to `FloatingBubbleView` separately from the view size. The cleanest implementation:
    - `setBubbleSize(visualDp)` controls the drawn radius
    - `onMeasure()` uses `bubbleSizeDp` for IDLE but the LayoutParams override the view's measured size to touch-target px
    - In `onDraw()` IDLE: `val radius = (bubbleSizeDp * density) / 2f` (the visual circle); `cx = w / 2f`, `cy = h / 2f` (centered in the larger touch-target view)
    - This means `onDraw` uses `bubbleSizeDp` for the drawn radius (visual), while the view's actual pixel size is the touch-target (set in LayoutParams). This works because `LayoutParams` wins over `onMeasure` — the view is stretched to touch-target px but draws the smaller visual circle centered within it.

- [x] **Task 3: Edge-snap + side-memory on drag release** (AC: 5)
  - [x] 3.1 Add `PREF_SIDE = "bubble_side"` (values: `"left"` / `"right"`) to the `companion object` constants
  - [x] 3.2 Modify `savePosition()` to also determine and save the side:
    ```kotlin
    private fun savePosition(x: Int, y: Int) {
        val (screenW, _) = getScreenDimensions()
        val dm = resources.displayMetrics
        val bubblePx = (bubbleView.getBubbleSizeDp() * dm.density).toInt()
        val side = if (x + bubblePx / 2 < screenW / 2) "left" else "right"
        getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
            .putInt(PREF_X, x)
            .putInt(PREF_Y, y)
            .putString(PREF_SIDE, side)
            .apply()
    }
    ```
  - [x] 3.3 Add edge-snap to the drag release (in `handleTouch`, `ACTION_UP`, `isDragging` branch):
    ```kotlin
    isDragging -> {
        // Edge-snap: slide to nearest horizontal edge
        val (screenW, _) = getScreenDimensions()
        val dm = resources.displayMetrics
        val bubblePx = (bubbleView.getBubbleSizeDp() * dm.density).toInt()
        val marginPx = (8 * dm.density).toInt()
        val midScreen = screenW / 2
        bubbleParams.x = if (bubbleParams.x + bubblePx / 2 < midScreen) {
            marginPx  // snap left
        } else {
            screenW - bubblePx - marginPx  // snap right
        }
        updateBubbleLayout()
        savePosition(bubbleParams.x, bubbleParams.y)
    }
    ```
  - [x] 3.4 On startup (`setupBubble()`), restore position using saved side:
    - Load `PREF_SIDE` from SharedPreferences
    - If saved side is `"left"`: default X = `marginPx`; if `"right"`: default X = `screenW - bubblePx - marginPx`
    - Restore Y from `PREF_Y` as before

- [x] **Task 4: Keyboard jump-up + nav-bar clearance** (AC: 6)
  - [x] 4.1 Add `NAV_BAR_CLEARANCE_PX = 56` constant to companion object (fixed px, per AR5d)
  - [x] 4.2 Check current `showBubble()` / `applyKeyboardState()` — when the keyboard becomes visible (`isOpen = true`), get the keyboard height. **Current approach:** the reflection-based fallback uses `getInputMethodWindowVisibleHeight()` which returns height in px. The accessibility-based path via `KlarvoAccessibilityService.onKeyboardVisibilityChanged()` calls `applyKeyboardState(true)` but does NOT pass the keyboard height.
  - [x] 4.3 Update `KlarvoAccessibilityService.onKeyboardVisibilityChanged()` (or its equivalent call site) to optionally pass keyboard height. The simplest approach: add an `adjustBubbleForKeyboard(keyboardHeightPx: Int)` method to `KlarvoOverlayService`:
    ```kotlin
    fun adjustBubbleForKeyboard(keyboardHeightPx: Int) {
        val (_, screenH) = getScreenDimensions()
        val dm = resources.displayMetrics
        val bubblePx = (bubbleView.getBubbleSizeDp() * dm.density).toInt()
        val maxY = screenH - keyboardHeightPx - NAV_BAR_CLEARANCE_PX - bubblePx
        if (bubbleParams.y > maxY) {
            bubbleParams.y = maxY.coerceAtLeast(0)
            updateBubbleLayout()
        }
    }
    ```
  - [x] 4.4 Update `KlarvoAccessibilityService.notifyKeyboardState()` to extract the IME window height using `AccessibilityWindowInfo.getBoundsInScreen(Rect)` (available API 21+, well within minSdk 24). The IME window's `top` coordinate gives keyboard top:
    ```kotlin
    private fun notifyKeyboardState() {
        val imeWindow = try {
            windows.firstOrNull { it.type == AccessibilityWindowInfo.TYPE_INPUT_METHOD }
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "windows list unavailable", e)
            return
        }
        val imeVisible = imeWindow != null
        if (imeVisible && imeWindow != null) {
            val rect = android.graphics.Rect()
            imeWindow.getBoundsInScreen(rect)
            val keyboardHeightPx = rect.height()
            KlarvoOverlayService.instance?.onKeyboardVisibilityChanged(imeVisible)
            KlarvoOverlayService.instance?.adjustBubbleForKeyboard(keyboardHeightPx)
        } else {
            KlarvoOverlayService.instance?.onKeyboardVisibilityChanged(imeVisible)
        }
    }
    ```
  - [x] 4.5 Fallback (reflection path): when `keyboardVisible` becomes true via the IMM reflection path in `checkKeyboardVisibility()`, call `adjustBubbleForKeyboard(height)` after detecting the keyboard height. The reflection call `getInputMethodWindowVisibleHeight()` already returns the height in px — use it directly.

- [x] **Task 5: Commit** (AC: all)
  - [x] 5.1 Stage only: `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` (modified), `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (modified), and `android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt` if touched
  - [x] 5.2 Never `git add .` — verify staged files only (no gen/ artifacts)
  - [x] 5.3 Commit message: `feat(android): 9-3 bubble idle re-skin + responsive sizing + anchoring`

## Dev Notes

### What Exists Today (Baseline) — Read Before Touching

**`FloatingBubbleView.kt`** (`android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt`, 478 LOC):
- Current idle render: `drawIdleIcon()` draws the Klarvo app launcher icon via `ContextCompat.getDrawable(context, R.mipmap.ic_launcher)`. Fallback is `drawMicIconFallback()` (draws a mic shape in grey).
- Current idle background: `colorIdleBackground = Color.parseColor("#F5F5F5")` — white/light grey circle.
- State enum: `IDLE, RECORDING, RECORDING_PTT, PROCESSING` — the enum names are preserved in this story; Story 9.5 will rename/add states (`TRANSCRIBING`, `DONE`). Do NOT change the enum in 9.3.
- Hardcoded colors (`#EF4444`, `#F59E0B`, `#22C55E`) — all will be replaced with `KlarvoTheme` constants in this story.
- `bubbleSizeDp`: currently set from `BASE_BUBBLE_SIZE_DP (56) × config.bubbleSize`; after 9.3, set from `computeVisualSizeDp()`.
- The `RECORDING` bar uses `BAR_WIDTH_DP = 220` — do NOT change bar behavior in this story.
- `drawWaveformBarsInZone()`: uses `whitePaint` for bar color — can stay white for now (waveform re-skin is 9.5 scope).

**`KlarvoOverlayService.kt`** (`android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt`, 1482 LOC):
- `setupBubble()` (~line 504): initializes `bubbleView`, reads `sizeScale` from config, calls `setBubbleSize()`, creates `bubbleParams` with `WRAP_CONTENT`, restores X/Y from SharedPreferences (`PREF_X`, `PREF_Y`).
- `reloadBubbleAppearance()` (~line 1322): called from `showBubble()` on every show — updates size and opacity. Must also be updated for responsive sizing.
- `savePosition()` (~line 685): saves X/Y to SharedPreferences. Add side detection here.
- `handleTouch()` ACTION_UP drag branch (~line 660): calls `savePosition()` — add edge-snap before saving.
- `getScreenDimensions()` (~line 544): returns `Pair<Int, Int>` in pixels.
- NO edge-snap exists today: `savePosition()` saves raw `bubbleParams.x` without snapping. The "default right side" is only a default at first launch.
- `KlarvoAccessibilityService.kt`: find it at `android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt` — it calls `KlarvoOverlayService.instance?.onKeyboardVisibilityChanged(isOpen)`. Check what `AccessibilityEvent` data is available for keyboard height.

**`KlarvoTheme.kt`** (`android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt`):
- Created in Story 9.2. All tokens are `const val Int` (raw ARGB ints, directly usable as `paint.color = KlarvoTheme.Teal`).
- DO NOT modify this file in Story 9.3 — it is complete.

### Color Semantics (DT5 — binding rule, not optional)

| Context | Color | Token |
|---------|-------|-------|
| Idle fill (glass surface) | `#16181A` | `KlarvoTheme.Surface` |
| Idle ring + "K" letter | `#29C7AC` | `KlarvoTheme.Teal` |
| Shadow | `0x33000000` | `KlarvoTheme.ShadowColor` |
| Processing spinner | `#29C7AC` | `KlarvoTheme.Teal` (brand/processing — NOT amber) |
| Recording bar/circle | `#EE6F63` | `KlarvoTheme.Danger` (stop/recording) |
| Cancel button | `#EE6F63` | `KlarvoTheme.Danger` |
| Confirm button | Keep green for now OR use `KlarvoTheme.Teal` | `KlarvoTheme.TealHi` |
| Waveform bars | White for now | `whitePaint` (no change in 9.3) |

**AMBER IS NOT USED IN IDLE.** Amber = recording tally light only (Story 9.5 scope). Using amber for the idle ring would violate DT5.

### Responsive Size Formula — Implementation Detail

```
screenW_dp = dm.widthPixels / dm.density
screenH_dp = dm.heightPixels / dm.density
raw = 0.11f * min(screenW_dp, screenH_dp)
visualDp = raw.toInt().coerceIn(36, 44)
```

On a 360dp wide phone: `0.11 × 360 = 39.6 → 39dp` (within clamp range).
On a 320dp wide phone: `0.11 × 320 = 35.2 → coerced to 36dp` (at minimum).
On a 420dp wide phone: `0.11 × 420 = 46.2 → coerced to 44dp` (at maximum).

The existing `bubbleSize` config scale factor is **superseded by this formula** in Story 9.3. The config field is not removed but is no longer multiplied into the bubble visual size. Document this explicitly in the commit or as a code comment so Story 9.5+ knows.

### Touch Target vs Visual Size — Drawing Pattern

The `FloatingBubbleView` will be placed in a `WindowManager` window of size `touchTargetPx × touchTargetPx` (at least 48dp × 48dp). The view fills that space. In `onDraw()`, the visual circle radius is computed from `bubbleSizeDp` (the smaller visual size), and drawn centered:

```kotlin
// onDraw() IDLE arm — drawing the visual circle within the larger touch-target view
val density = resources.displayMetrics.density
val visualRadius = (bubbleSizeDp * density) / 2f
val cx = w / 2f   // w = touch-target width (may be larger than visual)
val cy = h / 2f   // h = touch-target height
// Draw shadow, surface fill, ring, K — all at (cx, cy) with radius = visualRadius
```

This means `bubbleSizeDp` controls the drawn radius, and the view's pixel dimensions (set by LayoutParams) define the touch area. The transparent padding between visual circle edge and view edge is touch-responsive because the `setOnTouchListener` handles the full view area.

**Important:** `adjustLayoutForState()` sets `bubbleParams.width = WRAP_CONTENT` when switching states. After 9.3, IDLE must use explicit `touchTargetPx` dimensions. Update `adjustLayoutForState()` to handle this:
- IDLE → set `width = touchTargetPx`, `height = touchTargetPx`
- RECORDING bar → WRAP_CONTENT (as today, bar width is controlled by onMeasure)
- Other states → touchTargetPx

### Edge-Snap — Margin Convention

Use 8dp margin from screen edge (not 16dp — 16dp is the default startup margin, but after snap the bubble should be closer to the edge for a cleaner overlay feel). This can be adjusted; document the chosen value in code.

### Keyboard Jump-Up — Implementation Path

`KlarvoAccessibilityService.notifyKeyboardState()` walks the `windows` list for `TYPE_INPUT_METHOD`. `AccessibilityWindowInfo.getBoundsInScreen(Rect)` is available from API 21+ (well within minSdk 24) and gives the keyboard window bounds. The keyboard's `top` coordinate = `screenH - keyboardHeight`. So `rect.height()` = keyboard height in px.

Current behavior: `applyKeyboardState(true)` just calls `showBubble()` — no Y adjustment. The bubble may currently appear at the saved Y position even if it's behind the keyboard. Task 4 fixes this.

The reflection path (`checkKeyboardVisibility()`) already returns keyboard height in px via `getInputMethodWindowVisibleHeight()` — pass it directly to `adjustBubbleForKeyboard()` in that path too.

**Nav-bar clearance note:** The spec says 56px fixed clearance. This is the Android navigation bar height. Do NOT use `WindowInsetsCompat` or `env(safe-area-inset-bottom)` — those are unreliable for overlay Services on API 24+. The fixed 56px constant covers the nav bar on virtually all devices.

### Build Architecture (Critical — Same as 9.2)

`android/kotlin-src/` is the tracked source. `src-tauri/gen/android/` is gitignored generated output. Build scripts sync Kotlin sources:
- `scripts/android-build.sh`: full build + install
- `scripts/android-smoke.sh`: faster smoke build

No `Cargo.toml` or Rust changes in this story.

### What This Story Does NOT Do

- Does NOT implement RECORDING listening panel (Story 9.5)
- Does NOT implement TRANSCRIBING state (Story 9.5)
- Does NOT implement DONE state + check animation (Story 9.5)
- Does NOT rename/add enum values — `IDLE, RECORDING, RECORDING_PTT, PROCESSING` stay as-is
- Does NOT change waveform bar colors to teal (that's Story 9.5)
- Does NOT implement long-press popover (Story 9.8)
- Does NOT change the RECORDING bar shape/layout (bar still exists for HOLD mode)
- Does NOT apply Geist fonts to Canvas text — `textPaint` stays as system bold font for the "K" (Geist is bundled in `android/res-font/` from 9.2 but wiring it to `textPaint` is 9.5+ scope when panel text first appears)
- Does NOT touch Rust/Tauri/Desktop code

### Files Modified in This Story

| File | Change |
|------|--------|
| `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` | Modified — idle re-skin, color migration to KlarvoTheme |
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | Modified — responsive size, touch-target, edge-snap, keyboard jump |
| `android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt` | Possibly modified — keyboard height extraction |

**No files in `src-tauri/`, `src/`, `android/res-*` are modified.**

### References

- [Source: epics-visual-overhaul.md, Story 9.3] — ACs, DoD (FR1, FR7, AR5c/d)
- [Source: docs/design/overhaul/SPEC-studio-dark-overhaul.md, "Bubble States"] — idle: "Teal-K, dezenter Glas-Ring", responsive formula, same form no morph
- [Source: docs/design/overhaul/SPEC-studio-dark-overhaul.md, "⚠️ Machbarkeits-Constraints"] — no backdrop-blur (#3); touch targets ≥48dp; 56px nav-bar clearance (#4)
- [Source: docs/design/overhaul/SPEC-studio-dark-overhaul.md, "DT4"] — glass ring = 4dp ring (not blur); DT5 color semantics
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt] — Full baseline (478 LOC); state enum, onDraw structure, drawIdleIcon, hardcoded colors
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt] — setupBubble(), handleTouch(), savePosition(), adjustLayoutForState(), getScreenDimensions(); no edge-snap today
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt] — Created 9.2; all tokens const val Int, ready for paint.color assignment
- [Source: docs/adr/0018-android-bubble-rendering-tech.md] — View+Canvas substrate; no Compose; OvershootInterpolator for spring; minSdk 24
- [Source: _bmad-output/project-context.md] — minSdk 24, jni 0.21 pinned, no Compose, never git add .
- [Source: _bmad-output/project-context.md, "Critical Don't-Miss Rules"] — BYOK; no telemetry; Android changes require on-device smoke
- [Source: _bmad-output/implementation-artifacts/9-2-android-token-theme-source-and-fonts.md, "Dev Notes"] — KlarvoTheme.kt structure; canvas rendering pattern; const val Int form; font sync architecture

### Review Findings (code-review 2026-06-15, Opus; 3 layers: blind / edge / auditor)

- [ ] [Review][Patch] AC2 circle-size morph: RECORDING_PTT + PROCESSING draw radius=minOf(cx,cy) (=half the ≥48dp touch box) while IDLE uses visualRadius=bubbleSizeDp·density/2 → circle visibly grows leaving idle [FloatingBubbleView.kt onDraw, RECORDING_PTT + PROCESSING arms]
- [ ] [Review][Patch] Window-positioning math mixes visual bubblePx with the touch-target window width: edge-snap (ACTION_UP), savePosition side-test, setupBubble defaultX, and adjustBubbleForKeyboard maxY all use bubblePx (visual 36–44dp) while the window is touchTargetPx (≥48dp) → snapped circle not flush, side mis-saved near center, keyboard clamp under-shoots by (touchTarget−visual)px [KlarvoOverlayService.kt setupBubble/handleTouch ACTION_UP/savePosition/adjustBubbleForKeyboard]
- [ ] [Review][Patch] Keyboard jump-up never restores prior Y on hide in PREF_ALWAYS_VISIBLE mode (applyKeyboardState returns early when alwaysVisible; default keyboard-triggered mode is masked by hideBubble) → bubble stays stuck jumped-up [KlarvoOverlayService.kt applyKeyboardState/adjustBubbleForKeyboard]
- [x] [Review][Defer] getInputMethodWindowVisibleHeight() hidden-API fragility — deferred, pre-existing reflection fallback path
- [x] [Review][Defer] No onConfigurationChanged / rotation re-layout (size/side/Y not recomputed on rotate) — deferred, pre-existing; bubble re-setup on keyboard show in default mode
- [x] [Review][Defer] Drag move never clamps Y to screen bottom (only ACTION_UP x-snap + coerceAtLeast(0)) — deferred, pre-existing drag handler
- Dismissed (4): config.bubbleOpacity nullability (field is non-null — false positive); redundant double null-check in notifyKeyboardState (cosmetic); computeVisualSizeDp "near-constant" (it is the AC3 spec formula — behaving as specified); visualRadius>minOf(cx,cy) clipping (impossible: touchTarget≥48 > visual≤44)

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (story-context pass, 2026-06-15)

### Debug Log References

(none — story-context pass, no code execution)

### Completion Notes List

- All 5 tasks (21 subtasks) implemented and Kotlin compile verified (exit 0).
- **AC1/AC2:** IDLE render replaced — teal "K" + 4dp glass ring on dark Surface fill; `drawIdleIcon()`/`drawMicIconFallback()` NOT called in IDLE (kept for 9.4+). All hardcoded `Color.parseColor(...)` replaced with `KlarvoTheme` constants throughout (RECORDING_PTT→Danger, PROCESSING→Teal, bar cancel→Danger, bar confirm→TealHi, shadow→ShadowColor). No shape morphs introduced.
- **AC3:** `computeVisualSizeDp()` added — formula `clamp(36, (0.11 × min(screenW_dp, screenH_dp)).toInt(), 44)`. Used in both `setupBubble()` and `reloadBubbleAppearance()`. `bubbleSize` config scale factor superseded (no-op), documented in code comment.
- **AC4:** Touch-target expansion — `LayoutParams` set to `max(visualDp, 48)dp` × same. `FloatingBubbleView.onDraw()` IDLE uses `bubbleSizeDp` for visual radius (smaller), `w/2`, `h/2` for center → transparent padding is touch-responsive. `adjustLayoutForState()` updated: RECORDING→WRAP_CONTENT, all other states→touchTargetPx.
- **AC5:** Edge-snap on drag release: snaps to nearest edge with 8dp margin; `savePosition()` now also saves `PREF_SIDE` ("left"/"right"); `setupBubble()` restores default X from saved side.
- **AC6:** `adjustBubbleForKeyboard(keyboardHeightPx)` added to `KlarvoOverlayService` — posts Y adjustment to main handler. Called from accessibility path (`notifyKeyboardState()` now extracts IME bounds via `getBoundsInScreen()`, API 21+/minSdk 24) and reflection fallback path. `NAV_BAR_CLEARANCE_PX = 56` fixed constant (AR5d).
- **AC7:** On-device smoke is the gate — machine-verified: Kotlin compile exit 0, all JVM unit tests pass (exit 0).

### File List

- `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` (modified)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (modified)
- `android/kotlin-src/com/klarvo/voice/KlarvoAccessibilityService.kt` (modified)
- `_bmad-output/implementation-artifacts/9-3-bubble-idle-re-skin-responsive-sizing-anchoring.md` (this file)
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (status updated)

## Change Log

- 2026-06-15: Story context created (claude-sonnet-4-6). Baseline analysis: FloatingBubbleView.kt (478 LOC), KlarvoOverlayService.kt (1482 LOC), KlarvoTheme.kt (9.2 foundation). No edge-snap exists today; no keyboard-height adjustment. Touch-target expansion pattern documented. Color semantics (DT5) enforced.
- 2026-06-15: Implementation complete (claude-sonnet-4-6). FloatingBubbleView: IDLE re-skin (teal K + glass ring), all colors migrated to KlarvoTheme (DT5). KlarvoOverlayService: computeVisualSizeDp() responsive formula, touch-target expansion (≥48dp LayoutParams), edge-snap on drag release, PREF_SIDE side-memory, adjustBubbleForKeyboard() with NAV_BAR_CLEARANCE_PX=56. KlarvoAccessibilityService: notifyKeyboardState() extracts IME height via getBoundsInScreen(), calls adjustBubbleForKeyboard(). Kotlin compile: exit 0. JVM unit tests: exit 0. Status → review.
- 2026-06-15: Code-review (Opus, 3 layers) + conductor close-out (worker base impl folded into the single story commit) → 3 patches applied (AC2 PTT/PROCESSING circle now uses visual radius, not touch-box; all window-positioning math uses window width not visual px; keyboard jump-up restores prior Y on hide in always-visible mode), 3 deferred (pre-existing: hidden-API IME-height, no onConfigurationChanged/rotation, drag-Y not clamped to bottom), 4 dismissed. Compile + 24 JVM tests green; android-smoke build/install GREEN on device (v0.5.0, AI-1 gate). On-device visual render remains the human gate (secure keyguard blocks agent capture). Status stays review pending Andi's on-device visual smoke.
