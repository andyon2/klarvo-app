# Story 9.13: Recording-Cluster-Reihenfolge tauschen

Status: done

## Story

As a user dictating on Android,
I want the **➤ Send** control to sit at the dock/thumb position of the recording cluster (where the idle K-bubble sits) and **✗ Cancel** on the opposite (left) side,
so that the most-used action (send) is under my thumb and matches human habit, while the destructive action (cancel) is deliberately off the thumb path.

## Scope (locked — cluster-order/interaction change only, do NOT expand)

Swap the recording-state control-cluster order on Android from the current
`[➤ Send (left) · waveform (center) · ✗ Cancel (right/thumb)]`
to
`[✗ Cancel (left) · waveform (center) · ➤ Send (right/thumb)]`.

➤ Send (teal) moves to the dock/thumb anchor of the idle K-bubble; ✗ Cancel (red) moves to the left; the amber waveform stays centered. Color semantics are binding (ADR-0019): **red = Cancel, teal = Send** — both platforms.

**Hard scope boundaries:**
- **No** RMS/waveform behavior change (Story 9-12, done — do not touch `drawClusterWaveform`, `waveLevels`, `setStaticWaveLevel`, `amplitude` setter).
- **No** HOLD-mode surfaces (Story #4 — separate; do not build `.ab-holddock`, `.ab-holdstrip`, `.ab-slidehint`, `.ab-heldbub`, `.ab-lockchip`).
- **No** new tokens, **no** new states, **no** gesture-mode logic change.
- **Do not** silently expand Story 9.7 (gesture modes are already correct).
- **No** `KlarvoOverlayService.kt` changes — the public API (`isTouchInConfirmZone` / `isTouchInCancelZone`) is preserved as-is; the service remains unaware of button positions.
- **No** `KlarvoAudioRecorder.kt` changes.
- **No** Rust changes, **no** config key changes, **no** token changes.

**Only file to touch:** `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt`

## Acceptance Criteria

**AC1 — Send button is on the RIGHT (dock/thumb side) of the cluster.**
Given the RECORDING state cluster is visible
When the cluster renders
Then ➤ Send (teal gradient fill, paper-plane glyph, OnTeal stroke) is drawn at `clusterRight - innerPad - btnPx / 2f` — the same position the idle K-bubble's right edge occupies.

**AC2 — Cancel button is on the LEFT of the cluster.**
Given the RECORDING state cluster is visible
When the cluster renders
Then ✗ Cancel (DangerBg fill, Danger border, ✗ glyph) is drawn at `clusterLeft + innerPad + btnPx / 2f`.

**AC3 — Waveform stays centered between the two buttons.**
Given the new button positions
When the waveform zone is computed
Then `waveLeft = cancelCx + btnPx / 2f + gapPx` and `waveRight = sendCx - btnPx / 2f - gapPx` — identical geometry between buttons; waveform bar count, spacing, and amber color are unchanged.

**AC4 — Touch zones are correct for the new positions.**
Given the cluster is in RECORDING state
When tapping the RIGHT side of the cluster (≈ 40dp right zone)
Then `isTouchInConfirmZone(touchX)` returns true → `KlarvoOverlayService` calls `stopAndProcessRecording()`.
Given the cluster is in RECORDING state
When tapping the LEFT side of the cluster (≈ 40dp left zone)
Then `isTouchInCancelZone(touchX)` returns true → `KlarvoOverlayService` calls `cancelRecording()`.
Given tapping on the waveform dead zone or backdrop
Then neither zone returns true → no-op (existing AC2 from Story 9.5 preserved).

**AC5 — Harness confirms zone routing.**
Given the debug harness in RECORDING state (`adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording`)
When `touchX` is synthetically set to a point in the RIGHT third of the cluster window
Then `isTouchInConfirmZone(touchX)` == true (machine-checkable via unit test or logcat).
When `touchX` is synthetically set to a point in the LEFT third
Then `isTouchInCancelZone(touchX)` == true.

**AC6 — All other states are unaffected.**
Given any state other than RECORDING (IDLE, TRANSCRIBING, DONE)
When the view renders
Then no cluster elements are drawn, touch zone fields hold their reset values (0f), and all non-RECORDING behavior is unchanged.

**AC7 — KDoc and comments updated to reflect the new layout.**
Given the swap is complete
When a developer reads `FloatingBubbleView.kt`
Then the class-level KDoc, the section comment above `drawRecordingCluster`, the field comments, and the `isTouchInConfirmZone` / `isTouchInCancelZone` KDocs all describe the correct `[✗ cancel · waveform · ➤ send]` order.

**Inversion (must-fail gates):**
- Calling `isTouchInConfirmZone` with a touchX in the LEFT quarter of the cluster window returning true = review failure (Send is NOT on the left).
- Calling `isTouchInCancelZone` with a touchX in the RIGHT quarter returning true = review failure (Cancel is NOT on the right).
- Any change to `drawClusterWaveform`, `waveLevels`, `amplitude`, `setStaticWaveLevel` = scope violation.
- Any change to `KlarvoOverlayService.kt` = scope violation.
- Any `FLAG_NOT_TOUCHABLE` added anywhere in the file = HyperOS hard-dims overlays to 0.8 alpha.

**DoD:**
- DEBUG APK builds (`scripts/android-smoke.sh` exits 0).
- Existing 24+ JVM unit tests pass (none cover cluster geometry directly, but build must be clean).
- Emulator structural smoke green via `scripts/android-smoke.sh` under `BMAD_CONDUCTOR=1` — overlay-window structure intact; AC4/AC5 zone logic is machine-checkable without pixel inspection.
- **GATE-4 visual = Andi's real device (batched gate):** final pixel/placement verdict is Andi's real-device sight. The emulator is a structural oracle only — never a pixel oracle.

## Tasks / Subtasks

- [x] **Task 1: Rename private touch zone fields in `FloatingBubbleView.kt` (AC: 4, 7)**
  - [x] 1.1 Rename `clusterSendZoneEnd` → `clusterSendZoneStart` (leftmost X where Send zone begins — Send is now on the RIGHT, so the zone starts from mid-waveform rightward).
  - [x] 1.2 Rename `clusterCancelZoneStart` → `clusterCancelZoneEnd` (rightmost X where Cancel zone ends — Cancel is now on the LEFT, so the zone ends at mid-waveform leftward).
  - [x] 1.3 Update the field-comment block above them:
    ```kotlin
    // Send zone:   [clusterSendZoneStart, width]   (RIGHT side — thumb position)
    // Cancel zone: [0, clusterCancelZoneEnd]        (LEFT side)
    // Dead zone:   waveform area between them
    ```

- [x] **Task 2: Update `isTouchInConfirmZone` and `isTouchInCancelZone` (AC: 4, 7)**
  - [x] 2.1 Change `isTouchInConfirmZone` predicate:
    ```kotlin
    /** True when [touchX] hits the ➤ Send button zone (right side of the cluster). */
    fun isTouchInConfirmZone(touchX: Float): Boolean =
        state == State.RECORDING && clusterSendZoneStart > 0f && touchX >= clusterSendZoneStart
    ```
  - [x] 2.2 Change `isTouchInCancelZone` predicate:
    ```kotlin
    /** True when [touchX] hits the ✗ Cancel button zone (left side of the cluster). */
    fun isTouchInCancelZone(touchX: Float): Boolean =
        state == State.RECORDING && clusterCancelZoneEnd > 0f && touchX <= clusterCancelZoneEnd
    ```

- [x] **Task 3: Swap button draw positions in `drawRecordingCluster` (AC: 1, 2, 3, 6)**
  - [x] 3.1 In `drawRecordingCluster`, swap the cx assignments so Cancel occupies the LEFT position and Send occupies the RIGHT position:
    ```kotlin
    val cancelCx = clusterLeft + innerPad + btnPx / 2f    // LEFT  (was sendCx)
    val sendCx   = clusterRight - innerPad - btnPx / 2f   // RIGHT (was cancelCx)
    ```
  - [x] 3.2 Draw Cancel first, then Send (draw order left→right; visually unambiguous for the reader):
    ```kotlin
    // --- ✗ Cancel button (LEFT) ---
    drawCancelButton(canvas, cancelCx, btnCy, btnPx / 2f, btnR, dp)

    // --- ➤ Send button (RIGHT / dock-thumb) ---
    drawSendButton(canvas, sendCx, btnCy, btnPx / 2f, btnR, dp)
    ```
  - [x] 3.3 Update waveform zone bounds to use the new positions:
    ```kotlin
    val waveLeft  = cancelCx + btnPx / 2f + gapPx   // right edge of Cancel + gap
    val waveRight = sendCx - btnPx / 2f - gapPx     // left edge of Send - gap
    drawClusterWaveform(canvas, waveLeft, waveRight, btnCy, dp)
    ```
  - [x] 3.4 Update touch zone boundary assignments using renamed fields:
    ```kotlin
    // Cancel zone right boundary: midpoint between cancel right edge and waveform left
    val cancelZoneRight = waveLeft - gapPx / 2f
    // Send zone left boundary: midpoint between waveform right edge and send left
    val sendZoneLeft = waveRight + gapPx / 2f
    clusterCancelZoneEnd = cancelZoneRight
    clusterSendZoneStart = sendZoneLeft
    ```
  - [x] 3.5 Update the section comment above `drawRecordingCluster`:
    ```kotlin
    // RECORDING: control cluster [✗ cancel] [amber waveform] [➤ send]
    // Cancel (left, Danger), waveform (center, amber, RMS-driven), Send (right, Teal — dock/thumb)
    // Canon .ab-cluster: r18dp backdrop, static amber ring, 6dp pad, 9dp gap (§4′-Amendment 2026-06-21 #2)
    ```

- [x] **Task 4: Update class-level KDoc (AC: 7)**
  - [x] 4.1 Update the RECORDING line in the states KDoc block:
    ```
    *   RECORDING     -- control cluster at the dock spot (Modell B / ADR-0019 §4′ #2):
    *                    [✗ cancel red] [amber waveform] [➤ send teal] on a dark semi-transparent
    *                    backdrop with a static amber ring. Window grows from single bubble → cluster.
    ```
  - [x] 4.2 Update the "Touch zones in RECORDING" KDoc block:
    ```
    * Touch zones in RECORDING (cluster layout, left→right: cancel | waveform | send):
    *   - Left zone   -> ✗ Cancel (isTouchInCancelZone)
    *   - Dead zone   -> waveform (no action)
    *   - Right zone  -> ➤ Send  (isTouchInConfirmZone)
    *   KlarvoOverlayService reads these helpers and routes accordingly; tap on waveform/backdrop = no-op.
    ```

- [x] **Task 5: Build + structural smoke (AC: 5, DoD)**
  - [x] 5.1 `scripts/android-smoke.sh` exits 0 (build + drift gate + 24+ JVM unit tests green).
  - [x] 5.2 Structural verify on emulator: harness to RECORDING state, confirm `isTouchInConfirmZone` with X in right third returns true, `isTouchInCancelZone` with X in left third returns true.
  - [x] 5.3 Confirm APK is fresh via build script timestamp gate (no in-UI version screen).

- [x] **Task 6: Commit (AC: scope)**
  - [x] 6.1 Stage only `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt`.
  - [x] 6.2 Never `git add .`.

## Dev Notes

### The one-file change

**Only `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` needs to change.** No `KlarvoOverlayService.kt`, no `KlarvoAudioRecorder.kt`, no Rust, no tokens, no tests, no config.

The public touch-zone API (`isTouchInConfirmZone` / `isTouchInCancelZone`) is preserved — `KlarvoOverlayService` does not know which screen side each button occupies; it only asks "did the user tap the confirm zone / cancel zone." That indirection is intentional and means zero service-side changes.

### Current cluster layout (before this story)

In `drawRecordingCluster()` (currently lines ~381–401):

```kotlin
val sendCx   = clusterLeft + innerPad + btnPx / 2f   // LEFT position
val cancelCx = clusterRight - innerPad - btnPx / 2f  // RIGHT position

drawSendButton(canvas, sendCx, ...)    // draws ➤ on LEFT
drawCancelButton(canvas, cancelCx, ...)  // draws ✗ on RIGHT

val waveLeft  = sendCx + btnPx / 2f + gapPx
val waveRight = cancelCx - btnPx / 2f - gapPx

clusterSendZoneEnd = waveLeft - gapPx / 2f       // RIGHT end of the LEFT send zone
clusterCancelZoneStart = waveRight + gapPx / 2f  // LEFT start of the RIGHT cancel zone
```

Touch predicates:
```kotlin
isTouchInConfirmZone: touchX <= clusterSendZoneEnd   // LEFT side hit
isTouchInCancelZone:  touchX >= clusterCancelZoneStart  // RIGHT side hit
```

### Target cluster layout (after this story)

```kotlin
val cancelCx = clusterLeft + innerPad + btnPx / 2f   // LEFT position (was sendCx)
val sendCx   = clusterRight - innerPad - btnPx / 2f  // RIGHT position (was cancelCx)

drawCancelButton(canvas, cancelCx, ...)  // draws ✗ on LEFT
drawSendButton(canvas, sendCx, ...)      // draws ➤ on RIGHT

val waveLeft  = cancelCx + btnPx / 2f + gapPx
val waveRight = sendCx - btnPx / 2f - gapPx

clusterCancelZoneEnd = waveLeft - gapPx / 2f     // RIGHT end of the LEFT cancel zone
clusterSendZoneStart = waveRight + gapPx / 2f    // LEFT start of the RIGHT send zone
```

Touch predicates:
```kotlin
isTouchInConfirmZone: touchX >= clusterSendZoneStart  // RIGHT side hit
isTouchInCancelZone:  touchX <= clusterCancelZoneEnd  // LEFT side hit
```

### Why the window anchoring is still correct

`KlarvoOverlayService.adjustLayoutForState()` uses a right-edge-anchor expansion:
```kotlin
// Right-edge-anchor: shift X left by the extra width so the dock-spot right edge stays fixed.
bubbleParams.x = maxOf(0, bubbleParams.x + touchTargetPx - clusterW)
```

This means the WINDOW's right edge stays at the idle bubble's dock position. After the swap, ➤ Send sits at `clusterRight - innerPad - btnPx / 2f`, which is the rightmost button — directly at the dock/thumb position that the idle K-bubble occupied. The geometry is correct without touching `adjustLayoutForState`.

Exception: when the bubble is left-docked (`bubbleParams.x` ≈ 0), the maxOf clamp pins the window to screen-left, so the cluster expands rightward. In this case Send is still on the cluster's RIGHT visual edge — just the window starts at 0 rather than shift-left. This is the same clamping behavior as today; do NOT change `adjustLayoutForState`.

### What the cluster geometry constants mean

From `companion object` (DO NOT CHANGE these values — they are ADR-0019 §4′ canon):
```
CLUSTER_VISUAL_W_DP = 150  // total visual width
CLUSTER_VISUAL_H_DP = 52   // total visual height
CLUSTER_SHADOW_PAD_DP = 8  // shadow/touch pad per side → total window = 166×68dp
CLUSTER_PAD_DP = 6         // inner horizontal padding (each side)
CLUSTER_BTN_DP = 40        // each button square
CLUSTER_GAP_DP = 9         // gap between button and waveform zone
CLUSTER_BTN_R_DP = 12      // button corner radius
CLUSTER_BACKDROP_R_DP = 18 // backdrop corner radius
```

Width breakdown: `6(pad) + 40(cancel) + 9(gap) + 40(wave) + 9(gap) + 40(send) + 6(pad) = 150dp` — identical to current, only the label "cancel" and "send" swap sides.

### Do NOT touch the waveform machinery

`drawClusterWaveform`, `waveLevels`, `setStaticWaveLevel`, `amplitude` setter — these are 9-12 deliverables, DONE, and working. Pass `waveLeft` and `waveRight` computed from the new button positions; the method signature is unchanged.

### Harness commands (Story 9.4 — unchanged)

```bash
# Force RECORDING state (harness)
adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state recording --ef rms 0.7

# Reset to IDLE
adb shell am broadcast -a com.klarvo.voice.DEBUG_SET_STATE --es state idle
```

In RECORDING, the RIGHT third of the cluster window should trigger `isTouchInConfirmZone` (➤ Send) and the LEFT third `isTouchInCancelZone` (✗ Cancel). The waveform center dead zone should trigger neither.

### Anti-patterns — do NOT do

- **Do NOT swap the visual colors** (Teal always stays Send / Danger always stays Cancel; color semantics are binding per ADR-0019 DT5 — only the positions swap).
- **Do NOT change `drawClusterWaveform` or `waveLevels`** — the waveform only changes its X bounds (`waveLeft`/`waveRight`), not its rendering logic.
- **Do NOT change `KlarvoOverlayService.kt`** — the `when { bubbleView.isTouchInConfirmZone(touchX) -> ... }` block is correct; only the internal zone coordinates in `FloatingBubbleView` change.
- **Do NOT change `CLUSTER_VISUAL_W_DP` or any companion object constants** — the cluster width is the same; only the draw order within it changes.
- **Do NOT add `FLAG_NOT_TOUCHABLE`** — HyperOS dims such overlays to 0.8 alpha (bug found in Epic 9, `reference_hyperos_overlay_quirks.md`).
- **Do NOT call `adjustLayoutForState` or window params** from `FloatingBubbleView` — the view draws within whatever window dimensions the service sets.
- **Do NOT change the `suppressedForPanel` no-op** — kept for `KlarvoOverlayService` API compatibility.

### Canon source references

| Element | Canon location | Key spec |
|---------|---------------|----------|
| Cluster order | `docs/design/overhaul/source/Klarvo Design System.html` l.744–751 | `<!-- Cluster-Reihenfolge: ✗ Abbrechen links · Waveform · ➤ Senden RECHTS -->` |
| Approval render | `docs/design/overhaul/mockup-9-5-followups-2-4.html` section #2 | Cancel button first (left), Send button last (right) in DOM order |
| ADR mandate | `docs/adr/0019-cross-platform-design-ssot.md` §4′-Amendment 2026-06-21 (#2) | `#2 = Cluster getauscht [✗ links · Waveform · ➤ Senden rechts]` |
| Color semantics | ADR-0019 DT5 + project-context.md | `red = Abbrechen only`, `teal = brand/send` — never swap colors |

### Files to touch

| File | Change |
|------|--------|
| `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` | Swap button positions, rename touch zone fields, update predicates and KDoc |

No other files.

### Project Structure Notes

- `FloatingBubbleView.kt` is at `android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt` — always use this path, never `android/app/src/...` (the project copies `.kt` files into the Gradle tree via `android-build.sh`).
- Android JVM tests are in `android/kotlin-test/com/klarvo/voice/` — none cover canvas/cluster geometry, but the build must still be clean.
- The token file `KlarvoTheme.kt` is auto-generated from canon CSS (Story 9-10 drift gate). Do NOT hand-edit it. This story does not touch tokens.
- Smoke script: `scripts/android-smoke.sh` — run to get the build + drift gate + test gate in one step.

### References

- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.17–28] — Class KDoc (RECORDING state + touch zone description).
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.173–178] — Private touch zone fields `clusterSendZoneEnd` / `clusterCancelZoneStart`.
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.265–271] — `isTouchInConfirmZone` / `isTouchInCancelZone` predicates.
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, l.337–401] — `drawRecordingCluster()`, current button cx assignments, waveform bounds, touch zone updates.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.966–992] — `adjustLayoutForState()` — right-edge-anchor expansion; do NOT touch.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt, l.1132–1148] — `when { isTouchInConfirmZone ... }` dispatch; do NOT touch.
- [Source: docs/design/overhaul/source/Klarvo Design System.html, l.744–751] — Canon cluster order comment + HTML render.
- [Source: docs/design/overhaul/mockup-9-5-followups-2-4.html, section #2] — Andi-approved approval render of the swapped cluster.
- [Source: docs/adr/0019-cross-platform-design-ssot.md, §4′-Amendment 2026-06-21, (#2)] — Canon mandate for the order swap.
- [Source: docs/backlog.md, §"Story 9-5 GATE-4 green" point (2)] — Origin of this follow-up.
- [Source: _bmad-output/project-context.md] — No `git add .`; Android changes require on-device smoke; minSdk 24; no Compose; `android-smoke.sh` has drift gate.
- [Source: _bmad-output/implementation-artifacts/9-12-cluster-waveform-rms-reactive.md] — Previous story; waveform machinery (`waveLevels`, `setStaticWaveLevel`, `drawClusterWaveform`) is DONE and must not be touched.

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

Smoke run 2026-06-26: `BMAD_CONDUCTOR=1 scripts/android-smoke.sh` — exit 0.
- Drift gate: KlarvoTheme.kt in sync with canon klarvo.css.
- JVM tests: 24 tests, 0 failures.
- APK built in 7s, installed on emulator-5554.
- Emulator structural: cluster window 435×178px (=166×68dp at 420dpi/2.625 density) visible in RECORDING state. No FLAG_NOT_TOUCHABLE.
- Zone geometry confirmed: clusterCancelZoneEnd≈153px (left zone), clusterSendZoneStart≈282px (right zone). isTouchInCancelZone(72)=true, isTouchInConfirmZone(362)=true. Inversion: isTouchInConfirmZone(54)=false, isTouchInCancelZone(380)=false.

### Completion Notes List

- Only `FloatingBubbleView.kt` changed, no other files touched (scope boundary respected).
- Private fields `clusterSendZoneEnd`/`clusterCancelZoneStart` renamed to `clusterSendZoneStart`/`clusterCancelZoneEnd` — names now match their semantic (start/end of the zone, not the button side).
- `drawRecordingCluster`: `cancelCx` now at clusterLeft+innerPad+btnPx/2f (LEFT), `sendCx` at clusterRight-innerPad-btnPx/2f (RIGHT). Waveform bounds (`waveLeft`, `waveRight`) recomputed from the new positions — `drawClusterWaveform` signature unchanged.
- `isTouchInConfirmZone` changed from `touchX <= clusterSendZoneEnd` → `touchX >= clusterSendZoneStart` (RIGHT side check). `isTouchInCancelZone` changed from `touchX >= clusterCancelZoneStart` → `touchX <= clusterCancelZoneEnd` (LEFT side check).
- `KlarvoOverlayService.kt` unchanged — public API (`isTouchInConfirmZone`/`isTouchInCancelZone`) preserved.
- GATE-4 visual = Andi's real device (batched gate). Emulator confirms structural layout only.

### File List

android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-26 | Swapped recording cluster order: Cancel LEFT, Send RIGHT (dock/thumb). Renamed `clusterSendZoneEnd`→`clusterSendZoneStart`, `clusterCancelZoneStart`→`clusterCancelZoneEnd`. Updated predicates, draw order, waveform bounds, and all KDoc/comments in `FloatingBubbleView.kt`. | claude-sonnet-4-6 |
| 2026-06-26 | Review cleared — 3 independent Opus reviewers (Blind/Edge/Auditor), clean: all ACs confirmed, no scope violation, canon verified at source. GATE-4: build/drift/24-tests + dumpsys structure green (2 overlay windows, no regression); L/R order visually corroborated OS-independently (`gate4-evidence/9-13/`). Andi real-device acceptance ("an sich läuft es" — cluster + routing OK). Status → done. | story-conductor (Opus) |
