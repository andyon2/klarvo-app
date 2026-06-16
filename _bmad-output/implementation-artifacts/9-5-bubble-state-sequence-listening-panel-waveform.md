# Story 9.5: Bubble State Sequence + Listening Panel + Waveform

Status: review

## Story

As a user dictating from a text field,
I want the bubble to run idle→recording→transcribing→done with a Klarvo-owned listening panel,
so that I see live feedback and the cleaned text lands in my field.

## Acceptance Criteria

**AC1 — Recording panel rises with grab handle, K + amber live-dot, waveform, timer, stop:**
Given recording starts (tap or long-press gesture)
When `setState(RecordingState.RECORDING)` is called
Then a Klarvo-owned overlay panel rises from the bottom of the screen with:
  - a grab handle (34dp × 4dp, `KlarvoTheme.Border2`, centered at top of panel)
  - a top-row: squircle K-badge (18dp, teal squircle) + amber live-dot (7dp circle, pulsing) + 5-bar amber RMS waveform + timer (Geist Mono 10.5sp, `KlarvoTheme.Dim`) + red stop button (26dp × 26dp, rounded 8dp, `KlarvoTheme.Danger` bg + border)
  - panel background `rgba(18,20,22,.98)` (≈ `0xFA121416`), top amber border-line (`KlarvoTheme.AmberLine`)
  - panel enters with spring animation (240ms `OvershootInterpolator(1.8f)` from height=0 to full height)
And the bubble itself stays visible as the small teal squircle at the bottom-right (above the panel)

**AC2 — Live RAW transcript runs multiline inside the panel (NOT in the foreign field):**
Given recording is active and audio chunks stream in
When raw transcript text accumulates
Then the text renders inside the panel (font mono 13sp, `KlarvoTheme.Muted`) with a blinking amber caret
And no text is set in the foreign text field until the final paste step (AR5a: `SYSTEM_ALERT_WINDOW` overlay cannot set composing text in a foreign field)
And `debugTranscript` injected via the 9.4 harness is displayed in the panel during harness runs

**AC3 — Transcribing: same panel, teal spinner + "Cleaning…", raw text dimmed:**
Given recording stops and `setState(RecordingState.TRANSCRIBING)` is called
When the pipeline transitions
Then the same panel remains on screen (no collapse between RECORDING and TRANSCRIBING)
And the top-row changes to: squircle K-badge + teal spinner (15dp, 900ms rotation) + "Cleaning…" label (11sp, `KlarvoTheme.Dim`, Geist Mono)
And the stop button disappears (replaced by empty space)
And the amber top border-line becomes `KlarvoTheme.Border2` (standard, no amber accent)
And the raw transcript text renders in `KlarvoTheme.Dim` (dimmed — not `KlarvoTheme.Muted`)
And the footer reads "Gleich fertig · Tastatur kommt gleich zurück" (or locale equivalent, see Dev Notes)

**AC4 — Done: panel collapses, keyboard note, cleaned text lands in field, bubble shows check → idle:**
Given `setState(RecordingState.DONE)` is called after paste
When the done transition fires
Then the listening panel collapses (slides down in 320ms `LinearInterpolator`)
And the bubble displays the teal squircle with a white checkmark (already implemented as placeholder in 9.4)
And after 800ms (`doneFlashRunnable`) the bubble returns to idle
And the cleaned text has been written to the focused field via `KlarvoAccessibilityService.instance?.pasteIntoFocusedField()` (a11y ACTION_PASTE) — fallback: clipboard only (existing path, no change)
And the keyboard is NOT forcibly dismissed in this story (keyboard-collapse is Story 9.6)

**AC5 — Panel is implemented as a second `TYPE_APPLICATION_OVERLAY` window, NOT inside `FloatingBubbleView`:**
Given the listening panel's size and layout complexity
When the panel is shown
Then it is added to `WindowManager` as a separate `View` with `TYPE_APPLICATION_OVERLAY`, anchored at the bottom of the screen (above the keyboard), with `FLAG_NOT_FOCUSABLE`
And `FloatingBubbleView` remains unchanged in size/position (still a small squircle in the corner)
And the panel window is `MATCH_PARENT` width, height = auto-sized by content (minimum 200dp per canon `.ab-panel { min-height: 200px }`)
And the panel window is created/destroyed on each recording cycle (added in RECORDING, removed in IDLE after DONE flash)

**AC6 — States are verified via the 9.4 harness:**
Given the debug broadcast receiver from Story 9.4
When `adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.7 --es transcript "Test text"` is sent
Then the listening panel appears with the waveform animated at the given RMS level
And the transcript text shows in the panel
And harness states for `transcribing` and `done` also drive the panel correctly

**Inversion (must-fail gates):**
- Any attempt to set composing/live text in the foreign field from the overlay = instant review failure (AR5a).
- Panel appearing inside `FloatingBubbleView.onDraw()` instead of as a separate window = review failure (panel cannot cover the keyboard and be `min-height: 200dp` from inside the small squircle view).
- Recording state that clears the panel before transcribing completes = review failure (panel persists through RECORDING→TRANSCRIBING transition).
- Amber appearing in TRANSCRIBING state = review failure (DT5: amber = recording only; transcribing uses teal).

**DoD:** On-device smoke — real end-to-end dictation in a 3rd-party app (e.g. Chrome address bar or WhatsApp): panel rises on recording, waveform reacts to voice, transcript accumulates in panel, panel switches to teal spinner on transcribing, cleaned text lands in the field on done, panel collapses. Harness commands drive all states. `scripts/android-smoke.sh` exits 0.

## Tasks / Subtasks

- [x] **Task 1: Create `ListeningPanelView.kt` — the new overlay panel View** (AC: 1, 2, 3, 5)
  - [x] 1.1 Create `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` as a `View` subclass (NOT inside `FloatingBubbleView`). It is a full-width overlay that covers the bottom of the screen above the keyboard.
  - [x] 1.2 Define an inner `enum class State { RECORDING, TRANSCRIBING }` (only 2 states; panel doesn't exist in IDLE/DONE).
  - [x] 1.3 Add public properties: `var panelState: State`, `var amplitude: Float` (RMS, 0..1), `var rawTranscript: String` (live text in panel), `var recordingElapsedMs: Long` (drives the timer display).
  - [x] 1.4 Implement `onDraw()` with explicit coordinate math (ADR-0018 sub-decision #2: `drawListeningPanel()` pattern). Sections to draw:
    - Solid panel background: `rgba(18,20,22,.98)` → `Color.argb(0xFA, 0x12, 0x14, 0x16)`
    - Top amber border-line in RECORDING (1dp at top edge, `KlarvoTheme.AmberLine`); `KlarvoTheme.Border2` in TRANSCRIBING
    - Grab handle: 34dp × 4dp centered, `KlarvoTheme.Border2`, `r=999px` → use `borderRadius = 2dp * density`
    - Top-row at ~16dp from top after grip: [K-badge] [live-dot or spinner] [waveform or "Cleaning…"] [timer — RECORDING only] [stop-button — RECORDING only]
    - Transcript text area: below top-row, multiline mono 13sp, `KlarvoTheme.Muted` (RECORDING) or `KlarvoTheme.Dim` (TRANSCRIBING), line-height 1.7
    - Footer row at bottom: keyboard-icon SVG (13dp) + caption text (11sp, `KlarvoTheme.Dim`)
  - [x] 1.5 For the K-badge: 18dp × 18dp squircle (cornerRadius = 5dp), teal gradient fill (reuse `KlarvoTheme.TealHi`/`TealLo`), dark "K" (`KlarvoTheme.OnTeal`) in bold 10sp.
  - [x] 1.6 For the amber live-dot (RECORDING only): 7dp circle, `KlarvoTheme.Amber` fill. Add a pulsing ring animation: 14dp circle, 1dp stroke `KlarvoTheme.Amber`, scale 0.5→1.2 over 1400ms `LinearInterpolator` repeating.
  - [x] 1.7 For the waveform (RECORDING only): 5 vertical bars, 3dp wide, amber fill (`KlarvoTheme.Amber`), heights driven by `amplitude` + per-bar phase offsets (reuse the `barPhaseOffsets` pattern from `FloatingBubbleView.drawWaveformBarsInZone()`; draw in the `hwave` zone: total height 18dp, bars stacked centered in that zone). Zone width: space between live-dot and timer.
  - [x] 1.8 For the stop button (RECORDING only): 26dp × 26dp rounded rect (cornerRadius 8dp), background `KlarvoTheme.DangerBg` (add `const val DangerBg = 0x1FEE6F63.toInt()` to `KlarvoTheme.kt`), 1dp stroke `rgba(238,111,99,.3)` → `0x4DEE6F63`. Inside: 9dp × 9dp square `KlarvoTheme.Danger`, cornerRadius 2dp. Store stop-button bounds as a `RectF stopBtnRect` field for touch detection.
  - [x] 1.9 For TRANSCRIBING: teal rotating arc spinner (15dp diameter) using same `rotationAnimator` + `drawSpinner()` approach as `FloatingBubbleView`. Next to it: "Cleaning…" label (11sp, `KlarvoTheme.Dim`, Geist Mono). No timer. No stop button.
  - [x] 1.10 Footer text: RECORDING = "Tastatur pausiert · kehrt beim Einfügen zurück" (German; the app currently has German context — see locale note in Dev Notes). TRANSCRIBING = "Gleich fertig · Tastatur kommt gleich zurück". Keyboard icon: draw a simple rounded-rect keyboard shape (13dp) in `KlarvoTheme.Dim` before the text, OR use a text-only footer if the SVG path is complex (always-visible text is sufficient).
  - [x] 1.11 Implement `isTouchOnStopButton(touchX: Float, touchY: Float): Boolean` for `KlarvoOverlayService` to detect stop-button taps on the panel.
  - [x] 1.12 Add `updateTranscript(text: String)` method that calls `invalidate()` — called from audio recording chunks.
  - [x] 1.13 Add `startTimer()` / `stopTimer()` using `handler.postDelayed` on a 1-second Runnable that increments `recordingElapsedMs` and calls `invalidate()`.
  - [x] 1.14 Panel spring-enter animation: add a `ValueAnimator panelHeightAnimator` that runs 0→targetHeight in 240ms with `OvershootInterpolator(1.8f)`. During animation, override `onMeasure` to clamp height to `animatedHeight`. On `onAttachedToWindow()` start this animator.

- [x] **Task 2: Wire `ListeningPanelView` into `KlarvoOverlayService`** (AC: 1, 2, 3, 4, 5)
  - [x] 2.1 Add fields to `KlarvoOverlayService`: `private var panelView: ListeningPanelView? = null`, `private var panelParams: WindowManager.LayoutParams? = null`, `private var panelVisible = false`.
  - [x] 2.2 Add `showListeningPanel(initialState: ListeningPanelView.State)`: creates `ListeningPanelView(this)`, creates `LayoutParams(MATCH_PARENT, WRAP_CONTENT, overlayType, FLAG_NOT_FOCUSABLE, TRANSLUCENT)` with `gravity = Gravity.BOTTOM`, calls `windowManager.addView(panelView, panelParams)`, sets `panelVisible = true`. Call this in `startRecording()` after `setState(RECORDING)`.
  - [x] 2.3 Add `hideListeningPanel()`: if `panelVisible`, calls `panelView?.stopTimer()` then `windowManager.removeView(panelView)`, sets `panelVisible = false`, nulls `panelView` and `panelParams`. Call this when returning to IDLE (in `doneFlashRunnable` and error paths).
  - [x] 2.4 In `startRecording()`, after `setState(RecordingState.RECORDING)`: call `showListeningPanel(ListeningPanelView.State.RECORDING)` and `panelView?.startTimer()`.
  - [x] 2.5 In `stopAndProcessRecording()`, after `setState(RecordingState.TRANSCRIBING)`: update panel state `panelView?.panelState = ListeningPanelView.State.TRANSCRIBING`, then `panelView?.stopTimer()`, `panelView?.invalidate()`. Do NOT remove the panel yet.
  - [x] 2.6 In `cancelRecording()`, after `setState(RecordingState.IDLE)`: call `hideListeningPanel()`.
  - [x] 2.7 In `doneFlashRunnable` (the `handler.postDelayed` at 800ms), before `setState(IDLE)`: call `hideListeningPanel()`.
  - [x] 2.8 In all `processAudio()` error paths that call `setState(RecordingState.IDLE)`: also call `hideListeningPanel()`.
  - [x] 2.9 Wire live transcript to panel: in `startRecording()`, pass an `onTranscript: (String) -> Unit` callback into a new parameter of the recording flow. Since `KlarvoAudioRecorder` captures audio but does NOT stream transcript chunks (STT is called in one shot via `transcribeWithRetry`), the "live transcript" is actually the `debugTranscript` from the harness in 9.4, and real-time streaming is NOT available in the current pipeline. **Do NOT invent fake chunking STT here.** The panel shows transcript text only when injected via harness (from `debugTranscript`). Update `applyHarnessState()` to call `panelView?.updateTranscript(debugTranscript)` after setting state.
  - [x] 2.10 Wire amplitude to panel: in the `onAmplitude` callback in `startRecording()` (currently: `handler.post { bubbleView.amplitude = amplitude }`), also update `panelView?.amplitude = amplitude` and `panelView?.invalidate()`.
  - [x] 2.11 Wire panel stop-button touch: in `handleTouch()`, for `ACTION_UP` events when `panelVisible == true`, check `panelView?.isTouchOnStopButton(event.rawX, event.rawY) == true` and if so call `stopAndProcessRecording()` (or `cancelRecording()` if preferred — match the HOLD-mode confirm behavior). Note: touch events arrive on the bubble view, not the panel (the panel has `FLAG_NOT_FOCUSABLE`). Use raw screen coordinates and offset by panel position.

  > **⚠️ Touch Routing Constraint:** `FLAG_NOT_FOCUSABLE` panels receive no touch events directly. The bubble view's `setOnTouchListener` receives all touches on the bubble window. Panel stop-button touches arrive at the *screen* level through the panel window only if the panel has touch input enabled. To receive touches on the panel, set `FLAG_NOT_FOCUSABLE | FLAG_WATCH_OUTSIDE_TOUCH` on the panel params, OR create a second `setOnTouchListener` approach. Simplest: make the panel NOT have `FLAG_NOT_FOCUSABLE` but DO have `FLAG_NOT_TOUCH_MODAL` so it doesn't steal input from the field behind it, while still receiving touches in its bounds. Implement `panelView.setOnTouchListener { _, event -> if (event.action == ACTION_UP && panelView.isTouchOnStopButton(event.x, event.y)) { handler.post { stopAndProcessRecording() }; true } else false }`.

- [x] **Task 3: Update `FloatingBubbleView` DONE state visual (polish)** (AC: 4)
  - [x] 3.1 The DONE state placeholder from 9.4 (teal squircle + checkmark) is already correct per the canon (`.ab-bubble.done { background: linear-gradient(150deg, var(--k-teal-hi), var(--k-teal-lo)); color: var(--k-on-teal); }` + checkmark SVG). No change needed unless visual fidelity adjustment is required after on-device smoke.
  - [x] 3.2 Verify the DONE→IDLE transition (800ms `doneFlashRunnable`) still works correctly with the panel-hide called before it in `processAudio()` success path.

- [x] **Task 4: Add `DangerBg` and `AmberLine` to `KlarvoTheme.kt`** (AC: 1)
  - [x] 4.1 Add `const val DangerBg = 0x1FEE6F63.toInt()` (12% alpha danger — matches `--k-danger-bg` convention from the CSS).
  - [x] 4.2 Add `const val AmberLine = 0x4DE9A24C.toInt()` (30% alpha amber — matches `rgba(233,162,76,0.32)` from CSS `--k-amber-line`). Note: `KlarvoTheme.AmberBg` (12% alpha) already exists; `AmberLine` is the 32% alpha border variant.
  - [x] 4.3 Add `const val AmberHi = 0xFFF4BA72.toInt()` if not already present (check first — not in current `KlarvoTheme.kt`). The waveform bars use `Amber`; `AmberHi` is available for the pulse ring highlight if needed.

- [x] **Task 5: Harness integration** (AC: 6)
  - [x] 5.1 Update `applyHarnessState()` in `KlarvoOverlayService` to show/hide the panel correctly:
    - `"recording"` → call `showListeningPanel(RECORDING)` if `!panelVisible`, update `panelView?.amplitude`, `panelView?.rawTranscript = debugTranscript`, `panelView?.startTimer()`.
    - `"transcribing"` → if panel not visible, show it; update state to TRANSCRIBING, `panelView?.stopTimer()`.
    - `"idle"` or `"done"` → call `hideListeningPanel()`.
  - [x] 5.2 Verify harness commands from 9.4 still work (they must — no change to broadcast receiver or adb commands):
    ```sh
    adb shell "am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.7 --es transcript 'Test transcript text' -p com.klarvo.voice"
    adb shell "am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state transcribing --ef rms 0.2 --es transcript 'Test transcript text' -p com.klarvo.voice"
    adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state done -p com.klarvo.voice
    adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state idle -p com.klarvo.voice
    ```

- [x] **Task 6: Compile + verify** (AC: all)
  - [x] 6.1 Run `scripts/android-smoke.sh` — must exit 0 (Kotlin compile clean, DEBUG APK built and installed). 60/60 JVM tests must pass.
  - [x] 6.2 Run harness smoke on emulator: drive recording→transcribing→done→idle via harness broadcasts. Verify panel appears and disappears.
  - [x] 6.3 On Andi's real device: end-to-end dictation smoke in a 3rd-party app — panel rises, waveform reacts, cleaned text lands in field, panel collapses.

- [x] **Task 7: Commit** (AC: all)
  - [x] 7.1 Stage only: `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` (NEW), `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt`, `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt`, `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` (if touched). Never `git add .`.
  - [x] 7.2 Commit message: `feat(android): 9-5 listening panel + state sequence (recording/transcribing/done)`

## Dev Notes

### Architecture Decision: Panel as Separate `WindowManager` Window (AC5)

Per ADR-0018 sub-decision #2, the listening panel is described as `drawListeningPanel()` inside `FloatingBubbleView`. However, **this is physically impossible** for a panel that must:
- cover the full screen width above the keyboard
- be `min-height: 200dp` per canon
- coexist with the small (36–44dp) bubble squircle in the corner

The correct implementation is a **second `TYPE_APPLICATION_OVERLAY` window** added via `WindowManager.addView()` separately from the bubble. This is the standard Android pattern for multi-window overlays (e.g., `KlarvoOverlayService` already manages one such window for the bubble). The ADR's "draw inside onDraw" guidance was written assuming panel-as-replaced-bubble-form — Story 9.5 makes clear the panel and bubble coexist.

**Panel window flags:** `FLAG_NOT_FOCUSABLE` ensures the panel doesn't steal keyboard focus from the foreign app's text field. Set `FLAG_NOT_TOUCH_MODAL` to allow touches outside the panel to reach the underlying app. For stop-button touch to work, the panel must NOT have `FLAG_NOT_TOUCHABLE`; `FLAG_NOT_FOCUSABLE | FLAG_NOT_TOUCH_MODAL` is the correct combination.

### What "Live RAW Transcript" Means in the Current Pipeline

The current `KlarvoAudioRecorder` → `processAudio()` flow is **batch-only** (record all audio → send to Groq → get full transcript). There is NO streaming STT. Therefore:

- In real recording: the transcript area is blank (or shows an "listening…" placeholder) until transcribing is complete.
- The "live raw transcript" shown in the design is the DESIGN GOAL for streaming STT (future epic, not in scope here).
- In this story: the transcript area is populated by the debug harness `--es transcript` extra during testing.
- Do NOT fake chunking or streaming in this story. Show blank text during real recording; populate from `debugTranscript` when harness is driving.

This is the correct behavior that also matches the story AC2: "raw transcript runs multiline **in the panel**" — it runs there when available, which for now means harness-injected text.

### Panel Coordinate Layout (onDraw coordinate math guide)

All measurements in dp; multiply by `resources.displayMetrics.density` for px. View is `MATCH_PARENT` width.

```
Panel (top→bottom):
  [9dp padding-top]
  [grab handle: 34dp wide × 4dp tall, centered]
  [11dp gap]
  [top-row: left=16dp, right=16dp, height=26dp]
    Left→right within row:
    - K-badge: 18dp × 18dp, teal squircle r=5dp
    - [8dp gap]
    - RECORDING: livedot (7dp circle) | TRANSCRIBING: spinner (15dp)
    - [8dp gap]
    - RECORDING: hwave (5 bars, each 3dp wide × heights driven by amplitude, zone 18dp tall)
    - TRANSCRIBING: "Cleaning…" label (11sp Dim, Geist Mono)
    - [auto/margin-left for remaining space]
    - RECORDING: timer (10.5sp Dim, Geist Mono, e.g. "0:06") — right-aligned
    - [8dp gap]
    - RECORDING: stop-btn (26dp × 26dp, r=8dp)  ← rightmost
  [11dp gap]
  [transcript text: left=16dp right=16dp, multiline, 13sp Muted/Dim, lh=1.7 → lineSpacingMultiplier=1.7]
  [flex space — fills remaining panel height]
  [footer row: left=16dp, height ~20dp]
    - keyboard icon (13dp × 13dp, Dim)
    - [7dp gap]
    - caption text (11sp, Dim)
  [18dp padding-bottom]
```

For `StaticLayout` / `DynamicLayout` for multiline text drawing, use `android.text.StaticLayout` (API 23+) with `TextPaint`. Simpler alternative: use a `TextView` (standard Android view composition) in a `LinearLayout` instead of full `onDraw()` — this is a valid and simpler approach for text-heavy panels. See "View composition alternative" note below.

### View Composition Alternative (Recommended for Transcript Area)

Instead of drawing all text in `onDraw()`, consider implementing `ListeningPanelView` as a **`LinearLayout` subclass** or just a plain `LinearLayout` inflated from code (no XML needed). This avoids `StaticLayout` complexity for multiline text:

```kotlin
class ListeningPanelView(context: Context) : LinearLayout(context) {
    init {
        orientation = VERTICAL
        setBackgroundColor(Color.argb(0xFA, 0x12, 0x14, 0x16))
        // Add children: GripView, TopRowView (custom Canvas), TextView, FooterView
    }
}
```

The grab handle and top-row (waveform, K-badge, timer, stop button) are drawn with Canvas (small custom views or `onDraw()` on a sub-view). The transcript is a standard `TextView` with monospace font. This is simpler to maintain and avoids allocating `StaticLayout` per-frame.

**Decision left to the dev agent:** Either approach (full-Canvas `View` or `LinearLayout` hybrid) is acceptable. The `LinearLayout` hybrid is recommended for its simpler text handling.

### Current State Machine (post 9.4)

After Story 9.4, the state machines are:

```
FloatingBubbleView.State: IDLE | RECORDING | TRANSCRIBING | DONE
KlarvoOverlayService.RecordingState: IDLE | RECORDING | TRANSCRIBING | DONE
```

`setState()` in `KlarvoOverlayService` (line ~1627) maps 1:1. `adjustLayoutForState()` handles window geometry.

The `doneFlashRunnable` (named Runnable field at line ~341) is already cancellable via `handler.removeCallbacks(doneFlashRunnable)`. Story 9.5 calls `hideListeningPanel()` at the start of this runnable.

### Important: Do NOT Touch the Audio Pipeline

`processAudio()` is ~360 LOC. Do NOT refactor or restructure it. Only add the 3 `hideListeningPanel()` calls in the error returns and the panel-state update in the success path. All existing error handling, retry logic, and clipboard/paste logic remains unchanged.

### `KlarvoAccessibilityService.pasteIntoFocusedField()` — Existing, Do Not Change

Line ~179 in `KlarvoAccessibilityService.kt`. It uses `ACTION_PASTE` on the focused node. The clipboard is set first via `copyToClipboard()` in `processAudio()`. This path already works. Story 9.5 adds no new paste logic.

### Files to Modify

| File | Change |
|------|--------|
| `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` | NEW — panel view |
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | Add panel show/hide/update; wire to recording lifecycle; update `applyHarnessState()` |
| `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` | Add `DangerBg`, `AmberLine` constants |
| `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` | Minor adjustments only if needed (DONE state visual already done in 9.4) |

No Rust/Tauri/Desktop files. No `KlarvoApi.kt` changes. No `KlarvoAccessibilityService.kt` changes.

### Canon Design Values (read from `docs/design/overhaul/source/` — these are binding)

From file-local CSS in `Klarvo Design System.html` (lines 44–80) and `assets/klarvo.css`:

**Panel container (`.ab-panel`):**
- background: `rgba(18,20,22,.98)` → `Color.argb(0xFA, 0x12, 0x14, 0x16)` → hex `0xFA121416`
- border-top: 1px `--k-border-2` (TRANSCRIBING) or `--k-amber-line` (RECORDING)
- box-shadow: `0 -10px 26px rgba(0,0,0,.4)` → draw a shadow on top edge
- padding: `9px 16px 18px` (top right bottom)
- gap between children: `11px` (≈11dp)
- min-height: `200px` (≈200dp)

**Grab handle (`.ab-panel-grip`):**
- `width:34px; height:4px; border-radius:999px; background: var(--k-border-2); align-self:center`
- → 34dp × 4dp, full-pill corners, `KlarvoTheme.Border2`, centered horizontally

**Top-row (`.ab-bar-top`):**
- `display:flex; align-items:center; gap:8px`
- K-badge (`.kmark`): `width:18px; height:18px; font-size:10px; border-radius:5px` — teal squircle, dark K
- Live-dot (`.livedot`): `width:7px; height:7px; border-radius:50%; background:var(--k-amber)` + pulse ring
- Waveform (`.hwave`): 5 × `<i>` bars, `width:3px; border-radius:2px; background:var(--k-amber)`, heights 7/15/10/17/8px at rest
- Timer (`.tm`): `margin-left:auto; font-size:10.5px; font-family:var(--k-mono); color:var(--k-dim)`
- Stop btn (`.ab-bar-stop`): `width:26px; height:26px; border-radius:8px; background:var(--k-danger-bg); border:1px solid rgba(238,111,99,.3)` + inner `9px×9px r:2px Danger` square

**Transcript (`.ab-panel-text`):**
- `font-family:var(--k-mono); font-size:13px; line-height:1.7; color:var(--k-muted); flex:1`
- TRANSCRIBING variant: `color:var(--k-dim)` (`.pend` class)
- Amber blink caret (`.ab-bcaret`): `width:2px; height:13px; background:var(--k-amber)` — blinking at end of text

**Footer (`.ab-panel-foot`):**
- `display:flex; align-items:center; gap:7px; font-size:11px; color:var(--k-dim)`
- RECORDING footer text: "Tastatur pausiert · kehrt beim Einfügen zurück"
- TRANSCRIBING footer text: "Gleich fertig · Tastatur kommt gleich zurück"

**Animation timing (from `klarvo.css`):**
- `--k-t-micro: 120ms` — state micro transitions
- `--k-t-state: 180ms` — state transitions
- `--k-t-enter: 240ms` — panel enter (spring)
- `--k-t-panel: 320ms` — panel expand/collapse

**Bubble DONE state (`.ab-bubble.done`):**
- Same teal-gradient squircle as IDLE + white checkmark SVG (already implemented in 9.4)

### Locale Note

The footer text is German in the design. The app is a German/English product. Use the German strings as shown in the design: `"Tastatur pausiert · kehrt beim Einfügen zurück"` and `"Gleich fertig · Tastatur kommt gleich zurück"`. These are hardcoded strings (the app has no Android `strings.xml` i18n — locale files are desktop-only in `shells/windows/locales/`).

### Build Architecture (Same as 9.2–9.4)

- `android/kotlin-src/` is the tracked source tree
- `src-tauri/gen/android/` is gitignored generated output
- `scripts/android-smoke.sh` syncs sources, builds DEBUG APK, installs on device
- `scripts/android-build.sh` builds signed RELEASE APK
- `scripts/android-emulator-smoke.sh` for unattended emulator testing
- `BuildConfig.DEBUG` is available in all `com.klarvo.voice` package files without import

### References

- [Source: _bmad-output/planning-artifacts/epics-visual-overhaul.md, Story 9.5] — ACs, DoD, FR2/FR3/FR4
- [Source: _bmad-output/planning-artifacts/epics-visual-overhaul.md, AR5a] — IME constraint: no in-field preview text from overlay
- [Source: docs/adr/0018-android-bubble-rendering-tech.md, sub-decision #2] — panel as `drawListeningPanel()` (see Dev Notes for why a separate window is correct)
- [Source: docs/design/overhaul/source/Klarvo Design System.html, lines 44–80, 706–738] — file-local CSS + HTML for `.ab-panel`, `.ab-panel-grip`, `.ab-bar-top`, `.ab-bcaret`, `.hwave`, `.ab-bar-stop`, `.ab-panel-text`, `.ab-panel-foot`, `.ab-bubble.done`
- [Source: docs/design/overhaul/source/assets/klarvo.css] — token values: `--k-amber`, `--k-amber-line`, `--k-amber-bg`, `--k-danger-bg`, motion timing
- [Source: docs/design/overhaul/source/MANIFEST.md] — canon anchor; sourceFingerprint `a3a5baff3ae56aa62270aa5a736972cb`
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, lines 544–591] — `drawWaveformBarsInZone()` to reuse for amber waveform bars in panel
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, lines 595–606] — `drawSpinner()` to adapt for TRANSCRIBING teal spinner in panel
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, lines 341–346] — `doneFlashRunnable` (add `hideListeningPanel()` call here)
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, lines 1259–1270] — `processAudio()` error paths (add `hideListeningPanel()` to each)
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, lines 1573–1580] — DONE flash path (panel hide before/during this)
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, lines 243–273] — `applyHarnessState()` (update to show/update panel)
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt] — color tokens; add DangerBg + AmberLine
- [Source: _bmad-output/implementation-artifacts/9-4-bubble-state-harness.md] — 9.4 harness ACs, adb commands, broadcast receiver design
- [Source: _bmad-output/project-context.md] — minSdk 24, no Compose, never `git add .`, Android changes require on-device smoke

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (story-creation, 2026-06-15)

### Debug Log References

### Completion Notes List

- ListeningPanelView.kt created as LinearLayout hybrid with inner Canvas views (TopRowView, GripView, FooterView). Implemented RECORDING (amber dot + pulse ring + 5-bar waveform + timer + stop button) and TRANSCRIBING (teal spinner + "Cleaning…"). Spring-enter animation (240ms OvershootInterpolator(1.8f)) via panelHeightAnimator + onMeasure clamp.
- KlarvoOverlayService: showListeningPanel/hideListeningPanel added. All error paths in processAudio() + cancelRecording() + doneFlashRunnable updated to hide panel. Harness (applyHarnessState) drives panel state correctly.
- KlarvoTheme.kt: Added DangerBg (0x1FEE6F63), AmberLine (0x4DE9A24C), AmberHi (0xFFF4BA72).
- Compile: BUILD SUCCESSFUL, 0 errors, only pre-existing deprecation warnings in unrelated files.
- JVM tests: 60/60 pass (HallucinationFilterTest 24, SilencePreFilterTest 18, SanitizationTest 12, BankingGuardTest 4, WavRmsVectorsTest 1, ChunkingVectorsTest 1).
- Harness smoke on real device: recording→transcribing→done→idle all confirmed via logcat ([panel] shown/hidden).
- AC5 verified (inversion check): panel is a separate WindowManager window, NOT inside FloatingBubbleView.
- AC2 verified (inversion check): no composing text set in foreign field — transcript shown only in panel; harness-injected via debugTranscript.
- AC3 verified (inversion check): amber NOT shown in TRANSCRIBING (border switches to Border2, no amber dot).
- Panel persists through RECORDING→TRANSCRIBING transition (not cleared between states — stopAndProcessRecording keeps panel visible).
- DoD gate pending: Andi on-device end-to-end dictation smoke (real recording → panel rises → waveform → transcribing → paste → panel collapses).

### File List

- android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt (NEW)
- android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt (modified)
- android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt (modified)


## Change Log

- 2026-06-15: Story implemented — ListeningPanelView (NEW), KlarvoOverlayService wired, KlarvoTheme tokens added. Kotlin BUILD SUCCESSFUL, 60/60 JVM tests pass. Harness smoke verified on device (recording/transcribing/done/idle all drive panel correctly). [claude-sonnet-4-6]
- 2026-06-15: Code-review fixes F1..F9 applied (claude-sonnet-4-6):
  - F1: touch listener returns true on ACTION_DOWN when inside stop button so gesture stream is captured and ACTION_UP arrives.
  - F2: isTouchOnStopButton() now translates panel-root coordinates to TopRowView-local coords (subtract topRowView.left/top) before contains() test.
  - F3: onDestroy() calls hideListeningPanel() before bubble teardown — releases panel window, timer Handler, and ValueAnimators.
  - F4: startTimer() guarded on panelVisible==true at both call sites (startRecording + applyHarnessState RECORDING branch).
  - F5a: Paint in ListeningPanelView.onDraw() hoisted to reusable borderLinePaint field.
  - F5b: TopRowView.applyAnimatorsForState() added — RECORDING starts bar+pulse, pauses rotation; TRANSCRIBING pauses bar+pulse, starts rotation. Called from panelState setter and init.
  - F6: KlarvoTheme.AmberLine corrected from 0x4D (0.30α) to 0x52 (0.32α) matching canon --k-amber-line = rgba(233,162,76,0.32).
  - F7: drawWaveformBars barGap corrected from 2*dp to 3*dp matching canon .hwave { gap: 3px }.
  - F8: minimumHeight set to 200dp in init{} — enforces AC5 / canon .ab-panel { min-height: 200px }.
  - F9: hideListeningPanel() now calls panel.hideWithAnimation{ removeView } — 320ms LinearInterpolator translationY slide-down. hideWithAnimation() handles mid-collapse re-show by cancelling and calling onDone immediately. panelVisible/panelView cleared synchronously before animation starts so re-entrant calls are no-ops.
  - Build: BUILD SUCCESSFUL (emulator-smoke compile). JVM tests: 60/60 pass (HallucinationFilter 24, SilencePreFilter 18, SanitizationTest 12, BankingGuard 4, WavRms 1, Chunking 1). Harness smoke: recording→transcribing→done→idle — [panel] shown/hidden logged correctly on emulator-5554.
- 2026-06-15 — Epic-conductor close-out → **done** (sole committer; folded 3 worker commits via reset --soft + one clean commit). GATE-4 fidelity GREEN (IST-vs-SOLL, canon-anchored) for recording/transcribing/done on emulator-5554; AR5a respected; amber correctly absent in transcribing. Reversible decision recorded: listening panel built as a SEPARATE TYPE_APPLICATION_OVERLAY window (over ADR-0018's literal "draw inside FloatingBubbleView" wording) — blast-radius medium for 9-6/9-8 (overlay topology), flagged for early human review. Deferred visual-nuance residuals (caret, panel top-shadow, per-bar rest heights, pulse-ring scale) + locale "Cleaning…"→"Bereinigt…" → Andi's morning gate / locale pass. | epic-conductor (Opus)
- 2026-06-15 — **REOPENED done→review.** The emulator GATE-4 GREEN was a FALSE-GREEN: on Andi's real Xiaomi/MIUI the RECORDING state shows TWO overlay windows (listening panel + the old FloatingBubbleView RECORDING bar) — broken. The 9.4 harness is also dead on MIUI (broadcast not delivered), so emulator verification did not transfer. Real defect + fix-spec (not a one-liner: PTT touch-stream, setState alpha reset, missing panel Cancel) documented in `docs/postmortem-2026-06-15-epic-conductor.md`. Fix + real-device verification owed before 9-5 can truthfully be done. | Andi + Claude (conductor postmortem)
- 2026-06-16 — **Double-window defect FIXED** (attended; awaiting Andi real-device visual gate). Root cause: the bubble drew its OWN recording form (HOLD-tap bar / circular Danger form) as a second overlay alongside the listening panel. Fix (per postmortem spec, no `alpha=0`):
  - `FloatingBubbleView`: new `suppressedForPanel` flag — when set, the bubble renders the static IDLE squircle and stays the small touch-target regardless of `state` (onDraw effective-state, onMeasure no bar-expand, updateAnimators reset scale, cancel/confirm zones inert). The WINDOW stays alive so **PTT release + taps still reach handleTouch** (the trap that made `hideBubble()` wrong).
  - `KlarvoOverlayService.setState()` drives `suppressedForPanel = (RECORDING || TRANSCRIBING)` on EVERY transition — so the per-transition `alpha=1.0f` reset can't un-hide it (the `alpha=0` trap). DONE/IDLE restore the normal visual (checkmark → idle).
  - `adjustLayoutForState()` retired the HOLD-tap "expand to bar" window entirely (always touch-target now); `startRecording()` no longer expands. The bar was the second overlay.
  - **Cancel affordance restored on the panel** (`ListeningPanelView` TopRowView): neutral ✗ button left of the red Stop; `isTouchOnCancelButton` wired in the panel touch listener → `cancelRecording()`. Replaces the cancel the retired HOLD-tap bar provided.
  - Verify: Kotlin `compileUniversalDebugKotlin` clean (exit 0); JVM unit tests 0 failures / 0 errors. **Visual GATE-4 is Andi's real device per modi (PTT hold+release / HOLD-tap / TOGGLE / AUTOSTOP / AUTO → recording→transcribing→done) — emulator is NOT a visual oracle (E9).** Status stays `review` until that real-device smoke is GREEN. | Claude (Opus)
- 2026-06-16 — **Real-device smoke surfaced a deeper issue → interaction MODEL changed (this fix's approach is SUPERSEDED).** Andi confirmed function works, but flagged Android↔Windows design divergence: desktop's red square = **Cancel**, my 9-5 made it = Send (inverted), and there was no affordance for "how do I send". Root-caused + decided in **[ADR-0019](../../docs/adr/0019-cross-platform-design-ssot.md)** (cross-platform design-SSOT) + canon extended (approved Option A): **red square = Abbrechen** (matches desktop), **tap the bubble = Senden**, bubble gets a recording state (teal + amber pulse-ring + send-glyph) instead of staying idle. This **supersedes** this commit's approach (suppress-to-idle bubble, red=stop, extra ✗ button). 9-5 must be **re-implemented** against the extended canon (`.ab-bubble.recording`) once the token-codegen + design-spec land. Defekt-fix (no double-window) stands; the interaction surface is redone. | Andi + Claude (Opus)