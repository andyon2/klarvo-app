package com.klarvo.voice

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.*
import android.view.View
import android.view.animation.LinearInterpolator
import android.view.animation.OvershootInterpolator

/**
 * Custom View that draws the floating voice-input bubble (and the RECORDING surface).
 * All rendering via Canvas — no asset files needed.
 *
 * States:
 *   IDLE          -- teal-gradient squircle + dark "K" (OnTeal) + faint teal glass ring
 *                    Canon: .ab-bubble.idle
 *   RECORDING     -- depends on [holdDockActive]:
 *     holdDockActive=false: TAP surface (B-Sprache — ADR-0019 Amendment 2026-06-26):
 *                    Two large tappable circles (Send teal ➤ at dock side · Cancel dark+red ring ✕
 *                    on the opposite side) + a calm amber waveform chip above. Window size is
 *                    computed from [recordingButtonSizeDp] (default 72dp, ∈ {52,60,72,84,96}).
 *                    Dock side tracked in [dockSide] ("left"/"right").
 *     holdDockActive=true: HOLD Cancel surface — vereinfacht (ADR-0019 Amendment 2026-07-01,
 *                    Story 9-14 re-scope): the anchor bubble is the IDLE bubble itself (same size
 *                    + on-screen position, [bubbleSizeDp] — no separate HOLD bubble size) + ONE
 *                    round Abbrechen target (✕, red) growing diagonally up-and-toward-center from
 *                    it, scaled by [recordingButtonSizeDp], + a calm waveform chip hugging the
 *                    bubble. On finger hit ([holdTargetHit]) the target grows to ACTIVE size and
 *                    glows — no continuous animator, a redraw per hit-state change. Dragging away
 *                    from the bubble ([holdDragging]) fades the origin bubble (~.32 alpha) and
 *                    shows a ghost squircle following the live finger ([holdFingerX]/[holdFingerY]).
 *                    Release-to-commit: lands on Abbrechen → cancel, anywhere else → sends
 *                    (no Sperren/lock target — sending is the default, only Abbrechen needs a
 *                    deliberate target). Supersedes the prior two-target (Sperren+Abbrechen) build
 *                    (commits `ce20bb0`/`c431ba5`), which GATE-4-failed on device — see story
 *                    Dev Notes.
 *   TRANSCRIBING  -- single teal proc bubble (.ab-bubble.proc): teal squircle + rotating spinner.
 *                    Window collapses back to single-bubble size.
 *   DONE          -- success-green gradient squircle + dark check polyline (.ab-bubble.done),
 *                    then returns to IDLE (via doneFlashRunnable).
 *
 * Touch zones in RECORDING (TAP surface, Story 9-15):
 *   - Send circle   -> ➤ Send  (isTouchInConfirmZone) — circular hit, dock side
 *   - Cancel circle -> ✗ Cancel (isTouchInCancelZone) — circular hit, opposite side
 *   - Between/outside circles → no-op (chip area, backdrop)
 *   When holdDockActive=true both zone helpers return false (release = send by default; release on
 *   the Abbrechen target = cancel — no tappable zones during a physical HOLD, hit-tracking only).
 *
 * Color semantics (DT5 — binding rule):
 *   KlarvoTheme.Teal    = brand / ready / processing / focus-ring
 *   KlarvoTheme.OnTeal  = dark "K" letter on teal fill (IDLE) / send-glyph stroke
 *   KlarvoTheme.TealBg  = ~12% alpha faint ring (IDLE glass-ring accent)
 *   KlarvoTheme.Amber   = waveform in RECORDING surface ONLY (amber = live only)
 *   KlarvoTheme.Danger  = ✗ cancel glyph (ADR-0019: red = Abbrechen only)
 *   KlarvoTheme.SuccessHi / Success = done bubble gradient (150°)
 */
/**
 * Whether the finger currently sits over the single HOLD Abbrechen target, or not (Story 9-14
 * re-scope 2026-07-01 — the Sperren/lock target is gone, see story Dev Notes "vereinfachtes HOLD").
 * Top-level (not nested) so [KlarvoOverlayService]'s release-to-commit dispatch can reference
 * it unqualified (same package) — mirrors how RecordingState/RecordingMode are package-visible.
 */
enum class HoldTarget { NONE, CANCEL }

/**
 * Plain Kotlin (x, y) pair — deliberately NOT android.graphics.PointF. The Android unit-test stub
 * jar (no Robolectric, same convention as TapSurfaceTouchZoneTest) replaces PointF's constructor
 * with a no-op that silently leaves x/y at 0f instead of throwing, which would make
 * [FloatingBubbleView.holdCancelCenter] pass-but-wrong in JVM tests. Mirrors how
 * [FloatingBubbleView.tapCircleCenters] returns a plain `Pair<Float, Float>` for the same reason.
 */
data class HoldPoint(val x: Float, val y: Float)

class FloatingBubbleView(context: Context) : View(context) {

    enum class State { IDLE, RECORDING, TRANSCRIBING, DONE }

    var state: State = State.IDLE
        set(value) {
            if (field == value) return
            field = value
            updateAnimators()
            requestLayout()
            invalidate()
        }

    /**
     * Scrolling level-history ring buffer, depth 20 — mirrors desktop `useState(new Array(20).fill(0))`.
     * Each `amplitude` set pushes a new value (shift-left, append-newest).
     * The cluster waveform samples 5 positions across this buffer for the smooth scroll/fade effect.
     * Reset to all-0 on recording start and on return to idle.
     */
    private val waveLevels = FloatArray(20) { 0f }

    /** Amplitude 0..1 for waveform bar height during RECORDING */
    var amplitude: Float = 0f
        set(value) {
            field = value.coerceIn(0f, 1f)
            // Push new level into the scrolling history (shift left, append newest) — desktop parity.
            System.arraycopy(waveLevels, 1, waveLevels, 0, waveLevels.size - 1)
            waveLevels[waveLevels.size - 1] = field
            invalidate()
        }

    /**
     * Fill ALL waveLevels slots with [v] and invalidate.
     * Used by the debug harness (which sets one static level, not a stream) so a forced
     * harness recording shows a uniform waveform at that level rather than one filled slot.
     * Does NOT re-enter the amplitude push-setter — the cluster waveform reads waveLevels
     * directly, so touching amplitude is unnecessary here.
     */
    fun setStaticWaveLevel(v: Float) {
        waveLevels.fill(v.coerceIn(0f, 1f))
        invalidate()
    }

    /**
     * No-op — kept so KlarvoOverlayService compiles without change.
     * Previously suppressed the bubble visual; now the state-based draw handles everything.
     */
    @Suppress("UNUSED_PARAMETER")
    var suppressedForPanel: Boolean = false
        set(_) { /* no-op */ }

    /**
     * When true and state == RECORDING, draws the HOLD targets (B-Sprache, Story 9-14)
     * instead of the TAP surface. Set by KlarvoOverlayService when pushToTalkActive=true
     * (long-press + longPressMode == HOLD). Both isTouchInConfirmZone and isTouchInCancelZone
     * return false while this is true (no tappable zones during hold — release-to-commit
     * via [holdTargetHit] instead).
     */
    var holdDockActive: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            requestLayout()
            invalidate()
        }

    /**
     * Whether the finger currently sits over the single Abbrechen target, while [holdDockActive].
     * Drives both the grow-on-target redraw (AC4) and KlarvoOverlayService's release-to-commit
     * dispatch on ACTION_UP (AC5). No animator — setting this just triggers a single invalidate();
     * the grow/glow is a static redraw, not a continuous tween.
     */
    var holdTargetHit: HoldTarget = HoldTarget.NONE
        set(value) {
            if (field == value) return
            field = value
            invalidate()
        }

    /**
     * True once the finger has moved more than ~10dp away from the anchor bubble during a HOLD
     * gesture (Task 3.5/4, AC6) — gates the ghost-bubble + origin-fade dynamics. Reuses
     * KlarvoOverlayService's existing `dragThresholdPx` convention (same ~10dp already tuned for
     * the free-drag-the-idle-bubble gesture) rather than inventing a new threshold.
     */
    var holdDragging: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            invalidate()
        }

    /**
     * Live finger position (view-local px), forwarded by KlarvoOverlayService on every ACTION_MOVE
     * while [holdDockActive] (Task 4). Only meaningful while [holdDragging] — drives the ghost
     * squircle's position so it tracks wherever the finger actually is, not a derived/interpolated
     * point (AC6, canon: the ghost's mockup position is illustrative, not a formula input).
     */
    var holdFingerX: Float = 0f
        set(value) { field = value; if (holdDragging) invalidate() }
    var holdFingerY: Float = 0f
        set(value) { field = value; if (holdDragging) invalidate() }

    /**
     * Which screen edge the bubble is docked on: "left" or "right".
     * Set by KlarvoOverlayService.getDockSide() before entering RECORDING state.
     * Controls which circle (Send/Cancel) appears on which side of the TAP surface.
     */
    var dockSide: String = "right"

    /**
     * Epoch-ms timestamp when the current recording started.
     * Used by drawTapSurface() to render the elapsed timer in the waveform chip.
     * Set by KlarvoOverlayService.startRecording(). Reset to 0L when leaving RECORDING.
     */
    var recordingStartMs: Long = 0L

    /**
     * Diameter of the Send/Cancel circles on the TAP surface in dp — AND (Story 9-14 re-scope
     * 2026-07-01) the REST diameter of the single HOLD Abbrechen target.
     * User-configurable: ∈ {52, 60, 72, 84, 96}, default 72. Read from config.json key
     * "recordingButtonSizeDp" and applied via KlarvoOverlayService.reloadBubbleAppearance().
     * All TAP-surface layout dimensions (chip, gap, glyphs, labels) scale proportionally.
     * Also drives the window size via adjustLayoutForState (AC10 / Story 9-14 AC3).
     */
    var recordingButtonSizeDp: Int = TAP_BUTTON_SIZE_DEFAULT
        set(value) {
            field = value.coerceIn(TAP_BUTTON_SIZE_MIN, TAP_BUTTON_SIZE_MAX)
            invalidate()
        }

    /** Current bubble size in dp. Changed via setBubbleSize(). */
    private var bubbleSizeDp: Int = 56

    companion object {
        // ---- TAP surface reference dimensions (B-Sprache — Story 9-15, ADR-0019 Amendment 2026-06-26) ----
        // These constants define the reference layout at the ORIGINAL 132dp circle size (from the
        // browser-px render .ztap{width:132px}) and are used ONLY as proportional scaling anchors.
        // The ACTUAL rendered size is [recordingButtonSizeDp] (default 72dp, ∈ {60,72,88}).
        // All TAP-surface layout values scale as: actual = reference × (recordingButtonSizeDp / 132f).
        const val TAP_SEND_DIAM_DP   = 132   // reference circle diameter (scaling anchor; NOT the displayed size)
        const val TAP_INNER_GAP_DP   = 56    // horizontal gap at reference size
        const val TAP_CHIP_H_DP      = 54    // .statuschip at reference size
        const val TAP_CHIP_GAP_DP    = 16    // gap between chip bottom and circles top at reference size
        const val TAP_SHADOW_PAD_DP  = 10    // shadow/clip margin (fixed, not scaled)
        // Reference visual dimensions (used by tapVisualWidthDp / tapVisualHeightDp for proportional scaling).
        const val TAP_VISUAL_W_DP    = 320   // 132+56+132 at reference
        const val TAP_VISUAL_H_DP    = 202   // 54+16+132 at reference

        // ---- User-configurable button size constants (AC2/AC8 — Story 9-15 Re-Scope 2026-06-30;
        // range widened to {52,60,72,84,96} by Story 9-14 re-scope 2026-07-01 AC3, Andi-decided —
        // now also governs the HOLD Abbrechen target's REST diameter, not just the TAP surface). ----
        const val TAP_BUTTON_SIZE_MIN     = 52   // minimum allowed recordingButtonSizeDp
        const val TAP_BUTTON_SIZE_DEFAULT = 72   // default recordingButtonSizeDp (device-scale approved, unchanged)
        const val TAP_BUTTON_SIZE_MAX     = 96   // maximum allowed recordingButtonSizeDp

        /**
         * Compute TAP surface visual width in dp for the given [buttonSizeDp] (AC10).
         * Scales proportionally from the 132dp reference: result = buttonSizeDp × 320/132.
         * Does NOT include shadow padding (TAP_SHADOW_PAD_DP × 2 is added in adjustLayoutForState).
         */
        @JvmStatic
        fun tapVisualWidthDp(buttonSizeDp: Int): Int =
            (buttonSizeDp * TAP_VISUAL_W_DP.toFloat() / TAP_SEND_DIAM_DP.toFloat()).toInt()

        /**
         * Compute TAP surface visual height in dp for the given [buttonSizeDp] (AC10).
         * Scales proportionally from the 132dp reference: result = buttonSizeDp × 202/132.
         * Does NOT include shadow padding (TAP_SHADOW_PAD_DP × 2 is added in adjustLayoutForState).
         */
        @JvmStatic
        fun tapVisualHeightDp(buttonSizeDp: Int): Int =
            (buttonSizeDp * TAP_VISUAL_H_DP.toFloat() / TAP_SEND_DIAM_DP.toFloat()).toInt()

        // Non-KlarvoTheme colors for the TAP surface (mockup-specific alpha blends not in canon CSS).
        // These are NOT generated and NOT diffed by the drift gate — local Canvas constants only.
        private const val TAP_CANCEL_FILL      = 0xF2141212.toInt()  // .ztap.cancel fill: rgba(20,18,18,.95)
        private const val TAP_CANCEL_BORDER    = 0x80EE6F63.toInt()  // .ztap.cancel border: rgba(238,111,99,.5)
        private const val TAP_CANCEL_DANGER_HI = 0xFFF4897E.toInt()  // .ztap.cancel color: var(--k-danger-hi)=#F4897E (glyph+label)
        private const val TAP_CHIP_BG       = 0xF5121416.toInt()  // .statuschip bg: rgba(18,20,22,.96)
        private const val TAP_SEND_HINT     = 0xB305201B.toInt()  // OnTeal @70% for "tippen" hint

        // Non-KlarvoTheme colors for the HOLD Abbrechen target (mockup-specific alpha blends not in
        // canon CSS, same convention as TAP_CANCEL_* above — Story 9-14, re-scoped 2026-07-01 to a
        // single target).
        private const val HOLD_DANGER_HI   = 0xFFF4897E.toInt()  // --k-danger-hi: rest-cancel icon/label, active-cancel border
        private const val HOLD_DANGER_LINE = 0x73EE6F63.toInt()  // --k-danger-line rgba(238,111,99,.45): rest-cancel border
        private const val HOLD_ZONE_REST_BG = 0xEB121416.toInt() // .zone.rest background: rgba(18,20,22,.92)

        // ---- Old Klein-Cluster dimensions (kept as dead constants; CLUSTER_VISUAL_W/H_DP still
        // referenced by the retained dead drawRecordingCluster() code path below). ----
        const val CLUSTER_VISUAL_W_DP = 150
        const val CLUSTER_VISUAL_H_DP = 52
        const val CLUSTER_SHADOW_PAD_DP = 8

        // Internal cluster geometry (still used by drawRecordingCluster which is now dead code —
        // kept so the companion object compiles and any future reference is traceable).
        private const val CLUSTER_PAD_DP = 6
        private const val CLUSTER_BTN_DP = 40
        private const val CLUSTER_GAP_DP = 9
        private const val CLUSTER_BTN_R_DP = 12
        private const val CLUSTER_BACKDROP_R_DP = 18

        // Waveform bars (shared by both the old cluster dead code and the new TAP chip)
        private const val WAVE_BAR_W_DP = 3
        private const val WAVE_BAR_GAP_DP = 3
        private const val WAVE_BAR_COUNT = 5
        private const val WAVE_H_DP = 18

        // ---- HOLD Abbrechen target — vereinfacht (Story 9-14 re-scope, ADR-0019 Amendment
        // 2026-07-01). Values derived from mockup-mobile-hold-simple.html's `sRest`/`sHit` frames
        // (the "SIMPLIFIED HOLD CANON" override block), rendered+approved at device scale
        // 1080×2460@2.75 — these CSS px ARE dp, same convention as the 9-15 device-scale mockups.
        // Not exact-pixel law — proportions matter, GATE-4 visual fidelity is Andi's real-device
        // call (see story Dev Notes "Calibration caveat"). No fixed HOLD bubble size anymore — the
        // anchor bubble reads the live [bubbleSizeDp] via [getBubbleSizeDp] (AC2: same value
        // [drawIdleBubble] uses, so it can never drift from the idle bubble's size).
        const val HOLD_CHIP_H_DP        = 52   // .statuschip approx height (11px*2 pad + 30px wave) — unchanged
        const val HOLD_SHADOW_PAD_DP    = 10   // window shadow/clip margin (TAP_SHADOW_PAD_DP convention) — unchanged
        const val HOLD_CHIP_BUBBLE_GAP_DP = 14 // chip's near edge -> bubble inner edge (mockup: ≈18dp) — unchanged

        // REST diameter of the Abbrechen target = [recordingButtonSizeDp] dp directly (mirrors the
        // TAP surface's circle-diameter convention). ACTIVE = REST × this scale (mockup:
        // .zone.rest{width:96px} -> .zone.active{width:120px}, 120/96 = 1.25).
        const val HOLD_CANCEL_ACTIVE_SCALE = 1.25f

        // Bubble-center -> Abbrechen-center offset (mockup-derived, sRest/sHit center ≈(165,300)
        // both frames — confirms center-fixed growth, only the radius changes). Magnitudes only:
        // Δx mirrors sign by dock side (cancel always grows toward screen-center, i.e. AWAY from
        // the dock edge), Δy is always upward regardless of dock side. Fixed dp, NOT scaled by
        // recordingButtonSizeDp — no canon text addresses scaling the gap itself, and fixed-offset
        // matches the codebase's existing convention (gaps were always fixed dp, never user-scaled).
        const val HOLD_CANCEL_OFFSET_X_DP = 178f
        const val HOLD_CANCEL_OFFSET_Y_DP = 160f

        /**
         * Compute the HOLD window's visual width in dp for the given [buttonSizeDp] and the live
         * [bubbleSizeDp] (Task 1.6). The window must span from the bubble's dock-side edge to the
         * Abbrechen target's far edge at ACTIVE size: bubbleR + [HOLD_CANCEL_OFFSET_X_DP] +
         * activeR. Mirrors [tapVisualWidthDp]'s naming/shape; does NOT include
         * [HOLD_SHADOW_PAD_DP] × 2 (added separately in adjustLayoutForState, TAP convention).
         */
        @JvmStatic
        fun holdVisualWidthDp(buttonSizeDp: Int, bubbleSizeDp: Int): Int {
            val activeRDp = buttonSizeDp * HOLD_CANCEL_ACTIVE_SCALE / 2f
            return (bubbleSizeDp / 2f + HOLD_CANCEL_OFFSET_X_DP + activeRDp).toInt()
        }

        /**
         * Compute the HOLD window's visual height in dp — same derivation as [holdVisualWidthDp]
         * but along the (always-upward) Y offset: bubbleR + [HOLD_CANCEL_OFFSET_Y_DP] + activeR.
         */
        @JvmStatic
        fun holdVisualHeightDp(buttonSizeDp: Int, bubbleSizeDp: Int): Int {
            val activeRDp = buttonSizeDp * HOLD_CANCEL_ACTIVE_SCALE / 2f
            return (bubbleSizeDp / 2f + HOLD_CANCEL_OFFSET_Y_DP + activeRDp).toInt()
        }

        /**
         * Pure function: true if (touchX, touchY) lies within the circle at (cx, cy) of [radius].
         * Extracted from isTouchInConfirmZone / isTouchInCancelZone for JVM testability
         * (no Android View context needed). Tested by TapSurfaceTouchZoneTest.
         */
        @JvmStatic
        fun isInsideCircle(touchX: Float, touchY: Float, cx: Float, cy: Float, radius: Float): Boolean {
            val dx = touchX - cx
            val dy = touchY - cy
            return Math.sqrt((dx * dx + dy * dy).toDouble()) <= radius
        }

        /**
         * Pure function: resolves Send/Cancel circle centers for the TAP surface given dock side.
         * Returns Pair(sendCx, cancelCx) in the same coordinate space as [windowW].
         *   dockSide=="left"  → Send on left  (shadowPad+radius),  Cancel on right (windowW-shadowPad-radius)
         *   dockSide=="right" → Send on right (windowW-shadowPad-radius), Cancel on left (shadowPad+radius)
         * Extracted from drawTapSurface for JVM testability (no Android View context needed).
         * Tested by TapSurfaceTouchZoneTest (AC6).
         */
        @JvmStatic
        fun tapCircleCenters(dockSide: String, windowW: Float, shadowPad: Float, radius: Float): Pair<Float, Float> {
            val leftCx  = shadowPad + radius
            val rightCx = windowW - shadowPad - radius
            return if (dockSide == "left") Pair(leftCx, rightCx) else Pair(rightCx, leftCx)
        }

        /**
         * Pure function: resolves the anchor bubble's center for the HOLD surface given dock side
         * and window geometry (Story 9-14, re-scoped 2026-07-01). Mirrors [tapCircleCenters]'s
         * convention — all inputs share one coordinate space (px in production via
         * drawHoldTargets/adjustLayoutForState, dp-equivalent in HoldTargetTouchZoneTest using
         * density=1).
         *
         * The bubble sits [shadowPad] from the dock-side window edge (Task 1.5 — no separate
         * HOLD-specific edge inset anymore; reuses the same shadow-pad-as-edge-inset convention
         * [tapCircleCenters] already uses) and [shadowPad] from the BOTTOM window edge — the
         * Abbrechen target always grows upward from it (AC2/AC7), so the window's vertical budget
         * is spent above the bubble, not symmetrically around it.
         */
        @JvmStatic
        fun holdBubbleCenter(
            dockSide: String,
            windowW: Float,
            windowH: Float,
            shadowPad: Float,
            bubbleDiam: Float,
        ): HoldPoint {
            val bubbleR = bubbleDiam / 2f
            val bubbleCx = if (dockSide == "left") {
                shadowPad + bubbleR
            } else {
                windowW - shadowPad - bubbleR
            }
            val bubbleCy = windowH - shadowPad - bubbleR
            return HoldPoint(bubbleCx, bubbleCy)
        }

        /**
         * Pure function: resolves the single Abbrechen target's center, offset from [bubbleCenter]
         * by ([offsetXDp], [offsetYDp]) (Story 9-14 re-scope 2026-07-01). Mirrors [tapCircleCenters]'s
         * pattern for JVM testability (no Android View instance needed) — this is the function
         * KlarvoOverlayService's ACTION_MOVE hit-tracking (Task 6) calls every move event.
         *
         * [offsetXDp] mirrors sign by dock side (the target always grows toward screen-center, i.e.
         * away from the dock edge: right-docked → leftward / negative dx, left-docked → rightward /
         * positive dx). [offsetYDp] is always upward (AC7: vertical role is fixed regardless of
         * dock side).
         */
        @JvmStatic
        fun holdCancelCenter(
            dockSide: String,
            bubbleCenter: HoldPoint,
            offsetXDp: Float,
            offsetYDp: Float,
        ): HoldPoint {
            val dx = if (dockSide == "left") offsetXDp else -offsetXDp
            return HoldPoint(bubbleCenter.x + dx, bubbleCenter.y - offsetYDp)
        }
    }

    init {
        // Software layer so BlurMaskFilter (soft drop shadow) actually renders.
        setLayerType(LAYER_TYPE_SOFTWARE, null)
    }

    // --- Shared paints ---
    private val circlePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val shadowPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.ShadowColor
    }
    private val fillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }

    // --- Idle re-skin paints ---
    private val idleFillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val idleRingPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        color = KlarvoTheme.TealBg
        strokeCap = Paint.Cap.BUTT
    }
    private val kLetterPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.OnTeal
        textAlign = Paint.Align.CENTER
        typeface = Typeface.DEFAULT_BOLD
    }

    // Reusable RectF
    private val squircleRect = RectF()

    // --- Pre-allocated paints for the hot path (cluster drawing) ---
    private val sendGlyphPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        color = KlarvoTheme.OnTeal
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
    }
    private val cancelGlyphPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        color = KlarvoTheme.Danger
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
    }
    private val amberBarPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Amber
    }
    private val arcPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = KlarvoTheme.OnTeal
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
    }

    // Cached send-glyph path (rebuilt when size changes)
    private val sendGlyphPath = Path()
    private var lastSendGlyphSize = -1f

    // --- HOLD targets paints (B-Sprache — Story 9-14; color/textSize set per draw call since
    // rest/active state differs — same lightweight local-RectF convention as drawTapSurface,
    // not the old animator-hot-path pre-allocation style (no continuous animator anymore). ---
    private val holdZoneLabelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        textAlign = Paint.Align.CENTER
        typeface = Typeface.MONOSPACE
    }
    private val holdCancelGlyphPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        textAlign = Paint.Align.CENTER
    }
    private val holdCaptionPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Muted
        textAlign = Paint.Align.LEFT
    }

    // --- Pre-allocated for drawHoldChip/drawHoldZone (code review finding C, Story 9-14) — the
    // chip and the Abbrechen target's geometry is stable for the duration of a single hold
    // gesture, so their shaders/rects are rebuilt only when that geometry actually changes instead
    // of on every amplitude-driven invalidate(). The shared squircle helper (drawTealSquircle,
    // Task 3.1) deliberately does NOT cache — it now also serves the ghost bubble, whose position
    // changes every ACTION_MOVE while dragging (a cache keyed on position would rarely hit), and
    // matches the already-uncached style drawIdleBubble/drawProcBubble/drawDoneBubble use. ---
    private val holdBubbleRingRect   = RectF()
    private val holdChipRect         = RectF()
    private val holdChipShadowRect   = RectF()

    private var holdChipShadowBlurR = -1f
    private var holdChipShadowBlur: BlurMaskFilter? = null

    private var holdCancelGradientCx = Float.NaN
    private var holdCancelGradientCy = Float.NaN
    private var holdCancelGradientR  = Float.NaN
    private var holdCancelGradient: RadialGradient? = null

    // --- Touch zone boundaries (updated each onDraw) ---
    // TAP surface (Story 9-15): circular zones — 2D hit test via hypot(dx,dy) <= tapZoneRadius.
    // Legacy 1-D cluster fields below are still used by the now-dead drawRecordingCluster code
    // path and kept to avoid removing unused-variable warnings in the retained dead code.
    private var tapSendCx     = 0f
    private var tapSendCy     = 0f
    private var tapCancelCx   = 0f
    private var tapCancelCy   = 0f
    private var tapZoneRadius = 0f

    // Legacy cluster 1-D touch zone fields (kept — used by drawRecordingCluster dead code).
    private var clusterSendZoneStart = 0f
    private var clusterCancelZoneEnd = 0f

    // --- TAP surface pre-allocated paints (Story 9-15) ---
    // Pre-alloc to avoid GC on each amplitude-driven invalidate() during recording.

    /** Label inside Send circle: "Senden" — 15sp, weight 600, OnTeal */
    private val tapSendLabelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.OnTeal
        textAlign = Paint.Align.CENTER
        typeface = Typeface.DEFAULT_BOLD
    }

    /** Label inside Cancel circle: "Abbrechen" — 13sp, Danger-Hi tint */
    private val tapCancelLabelPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = TAP_CANCEL_DANGER_HI
        textAlign = Paint.Align.CENTER
        typeface = Typeface.DEFAULT_BOLD
    }

    /** Hint text inside circles: "tippen" — 11sp, monospace, subdued */
    private val tapSendHintPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = TAP_SEND_HINT
        textAlign = Paint.Align.CENTER
        typeface = Typeface.MONOSPACE
    }

    /** Hint text inside cancel circle: "tippen" — 11sp, monospace, Dim */
    private val tapCancelHintPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Dim
        textAlign = Paint.Align.CENTER
        typeface = Typeface.MONOSPACE
    }

    /** Timer text in waveform chip: "0:08" — 13sp, monospace, Muted */
    private val tapTimerPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Muted
        textAlign = Paint.Align.LEFT
        typeface = Typeface.MONOSPACE
    }

    /** Cancel glyph (✗ cross) in the TAP surface cancel circle — Danger-Hi tint, rounded cap */
    private val tapCancelGlyphPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        color = TAP_CANCEL_DANGER_HI
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
    }

    // --- Animations ---
    private val rotationAnimator = ValueAnimator.ofFloat(0f, 360f).apply {
        duration = 900
        repeatCount = ValueAnimator.INFINITE
        interpolator = LinearInterpolator()
        addUpdateListener { invalidate() }
    }

    private val pttScaleUpAnimator = ValueAnimator.ofFloat(1.0f, 1.3f).apply {
        duration = 200
        interpolator = OvershootInterpolator(2.0f)
        addUpdateListener { anim ->
            val s = anim.animatedValue as Float
            scaleX = s; scaleY = s
        }
    }
    private val pttScaleDownAnimator = ValueAnimator.ofFloat(1.3f, 1.0f).apply {
        duration = 150
        interpolator = LinearInterpolator()
        addUpdateListener { anim ->
            val s = anim.animatedValue as Float
            scaleX = s; scaleY = s
        }
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        updateAnimators()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        rotationAnimator.cancel()
        pttScaleUpAnimator.cancel()
        pttScaleDownAnimator.cancel()
    }

    private fun updateAnimators() {
        when (state) {
            State.RECORDING -> {
                // Cluster waveform motion comes from the scrolling waveLevels history (desktop parity).
                // No barAnimator needed — each amplitude push triggers invalidate().
                // No amber pulse-ring in Modell B — the static ring on the backdrop replaces it.
                // No scale pop: the cluster draws in-place, no bounce animation.
                // HOLD targets (Story 9-14, B-Sprache) have no continuous animator either — grow-on-target
                // (AC3/AC4) is a static redraw driven by the holdTargetHit setter, not a tween.
                rotationAnimator.cancel()
                // Reset history so each new recording starts flat (desktop: setLevels(new Array(20).fill(0))).
                waveLevels.fill(0f)
            }
            State.TRANSCRIBING -> {
                // Proc bubble: rotating spinner, no waveform.
                if (!rotationAnimator.isRunning) rotationAnimator.start()
            }
            State.DONE, State.IDLE -> {
                rotationAnimator.cancel()
                // Reset history on return to idle (desktop: setLevels(new Array(20).fill(0))).
                waveLevels.fill(0f)
                if (scaleX != 1.0f) {
                    pttScaleUpAnimator.cancel()
                    pttScaleDownAnimator.setFloatValues(scaleX, 1.0f)
                    pttScaleDownAnimator.start()
                }
            }
        }
    }

    fun setBubbleSize(sizeDp: Int) {
        bubbleSizeDp = sizeDp
        requestLayout()
        invalidate()
    }

    fun getBubbleSizeDp(): Int = bubbleSizeDp

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        // Always fill the window dimensions supplied by KlarvoOverlayService.
        // For IDLE/TRANSCRIBING/DONE: square touch-target window.
        // For RECORDING: wider cluster window (set by adjustLayoutForState).
        setMeasuredDimension(
            MeasureSpec.getSize(widthMeasureSpec),
            MeasureSpec.getSize(heightMeasureSpec)
        )
    }

    // --- Touch zone helpers ---

    /**
     * True when the touch point ([touchX], [touchY]) lands inside the ➤ Send circle of the
     * TAP surface (Story 9-15). Uses 2D circular hit detection via [isInsideCircle].
     * Returns false when holdDockActive=true — no tappable zones during a physical HOLD.
     */
    fun isTouchInConfirmZone(touchX: Float, touchY: Float): Boolean {
        if (state != State.RECORDING || holdDockActive || tapZoneRadius <= 0f) return false
        return isInsideCircle(touchX, touchY, tapSendCx, tapSendCy, tapZoneRadius)
    }

    /**
     * True when the touch point ([touchX], [touchY]) lands inside the ✗ Cancel circle of the
     * TAP surface (Story 9-15). Uses 2D circular hit detection via [isInsideCircle].
     * Returns false when holdDockActive=true — no tappable zones during a physical HOLD.
     */
    fun isTouchInCancelZone(touchX: Float, touchY: Float): Boolean {
        if (state != State.RECORDING || holdDockActive || tapZoneRadius <= 0f) return false
        return isInsideCircle(touchX, touchY, tapCancelCx, tapCancelCy, tapZoneRadius)
    }

    // --- onDraw ---

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        when (state) {
            State.IDLE         -> drawIdleBubble(canvas)
            State.RECORDING    -> if (holdDockActive) drawHoldTargets(canvas) else drawTapSurface(canvas)
            State.TRANSCRIBING -> drawProcBubble(canvas)
            State.DONE         -> drawDoneBubble(canvas)
        }
    }

    // =========================================================================
    // IDLE: teal-gradient squircle + dark "K" + faint ring
    // =========================================================================

    private fun drawIdleBubble(canvas: Canvas) {
        val density = resources.displayMetrics.density
        val side = bubbleSizeDp * density
        val visualRadius = side / 2f
        val cx = width / 2f
        val cy = height / 2f
        val cornerPx = side * 0.30f

        drawTealSquircle(canvas, cx, cy, side, alpha = 0xFF)

        // Faint teal accent ring (idle-only — the HOLD anchor uses an amber holding-ring instead)
        squircleRect.set(cx - visualRadius, cy - visualRadius, cx + visualRadius, cy + visualRadius)
        val ringStrokePx = 2f * density
        idleRingPaint.strokeWidth = ringStrokePx
        val ringHalf = ringStrokePx / 2f
        val ringRect = RectF(
            squircleRect.left - ringHalf, squircleRect.top - ringHalf,
            squircleRect.right + ringHalf, squircleRect.bottom + ringHalf
        )
        canvas.drawRoundRect(ringRect, cornerPx + ringHalf, cornerPx + ringHalf, idleRingPaint)

        drawKLetter(canvas, cx, cy, visualRadius, alpha = 0xFF)
    }

    /**
     * Shared squircle base draw (soft shadow + teal-gradient fill + rounded-rect), extracted from
     * the old per-state-duplicated block (Task 3.1, Story 9-14 re-scope). Now has 2 real consumers
     * — [drawIdleBubble] and the HOLD anchor/ghost bubbles in [drawHoldTargets] — justifying the
     * extraction per the project's "factor out only on proven duplication" rule. Using the SAME
     * code path for the idle bubble and the HOLD anchor makes AC2 ("kein Größen-/Orts-Sprung")
     * true by construction, not by copy-pasted, hopefully-matching constants.
     * [alpha] (0-255) supports the HOLD origin bubble's fade-on-drag (AC6, canon `opacity:.32`).
     */
    private fun drawTealSquircle(canvas: Canvas, cx: Float, cy: Float, diamPx: Float, alpha: Int) {
        val r = diamPx / 2f
        val cornerPx = diamPx * 0.30f
        val rect = RectF(cx - r, cy - r, cx + r, cy + r)

        shadowPaint.maskFilter = BlurMaskFilter(diamPx * 0.14f, BlurMaskFilter.Blur.NORMAL)
        shadowPaint.alpha = alpha
        val shadowRect = RectF(rect.left, rect.top + diamPx * 0.06f, rect.right, rect.bottom + diamPx * 0.06f)
        canvas.drawRoundRect(shadowRect, cornerPx, cornerPx, shadowPaint)
        shadowPaint.alpha = 0xFF

        idleFillPaint.shader = LinearGradient(
            rect.left, rect.top, rect.right, rect.bottom,
            KlarvoTheme.TealHi, KlarvoTheme.TealLo, Shader.TileMode.CLAMP
        )
        idleFillPaint.alpha = alpha
        canvas.drawRoundRect(rect, cornerPx, cornerPx, idleFillPaint)
        idleFillPaint.alpha = 0xFF
    }

    /** Shared dark "K" letter draw, centered at (cx, cy) for a squircle of [radius]. */
    private fun drawKLetter(canvas: Canvas, cx: Float, cy: Float, radius: Float, alpha: Int) {
        kLetterPaint.textSize = radius * 0.85f
        kLetterPaint.alpha = alpha
        val textCy = cy - (kLetterPaint.ascent() + kLetterPaint.descent()) / 2f
        canvas.drawText("K", cx, textCy, kLetterPaint)
        kLetterPaint.alpha = 0xFF
    }

    // =========================================================================
    // RECORDING: TAP surface (B-Sprache — Story 9-15, ADR-0019 Amendment 2026-06-26)
    // Two large tappable circles (Send teal ➤ + Cancel dark/red ✗) + waveform chip above.
    // Window: TAP_VISUAL_W_DP × TAP_VISUAL_H_DP + 2×TAP_SHADOW_PAD_DP on each side.
    // Dock side: dockSide="right" → Send on right, Cancel on left; mirrored for "left".
    // =========================================================================

    private fun drawTapSurface(canvas: Canvas) {
        val dp    = resources.displayMetrics.density
        val spd   = resources.displayMetrics.scaledDensity
        val w     = width.toFloat()
        // Proportional scale factor relative to the 132dp reference size (AC2/AC10).
        val scale = recordingButtonSizeDp / TAP_SEND_DIAM_DP.toFloat()

        val shadowPadPx = TAP_SHADOW_PAD_DP * dp
        val radPx       = recordingButtonSizeDp * dp / 2f   // actual circle radius (AC2)
        val chipH       = TAP_CHIP_H_DP   * scale * dp      // chip scales with button size
        val chipGap     = TAP_CHIP_GAP_DP * scale * dp      // gap scales with button size

        // Circles vertical center: below chip area + gap
        val circlesCy = shadowPadPx + chipH + chipGap + radPx

        // Assign Send/Cancel based on dock side (AC3 + AC6)
        val (sendCx, cancelCx) = tapCircleCenters(dockSide, w, shadowPadPx, radPx)

        // Store 2D touch zones for isTouchInConfirmZone / isTouchInCancelZone (AC4)
        tapSendCx     = sendCx
        tapSendCy     = circlesCy
        tapCancelCx   = cancelCx
        tapCancelCy   = circlesCy
        tapZoneRadius = radPx

        // --- 1. Waveform chip (amber wave + timer, above the circles) ---
        val chipCy = shadowPadPx + chipH / 2f
        drawTapChip(canvas, w / 2f, chipCy, dp, spd, scale)

        // --- 2. Cancel circle (drawn first so Send renders on top if they overlap) ---
        drawTapCancelCircle(canvas, cancelCx, circlesCy, radPx, dp, spd, scale)

        // --- 3. Send circle ---
        drawTapSendCircle(canvas, sendCx, circlesCy, radPx, dp, spd, scale)
    }

    /**
     * Draws the waveform chip centered at (cx, cy).
     * Layout: [amber wave bars] [9dp gap] [mm:ss timer] — inside a dark rounded chip.
     * Matches .statuschip CSS: padding 11dp×16dp, border-radius 18dp, rgba(18,20,22,.96) fill.
     * [scale] = recordingButtonSizeDp / 132f — all layout dims scale proportionally (AC2).
     */
    private fun drawTapChip(canvas: Canvas, cx: Float, cy: Float, dp: Float, spd: Float, scale: Float) {
        val chipH    = TAP_CHIP_H_DP * scale * dp
        val halfH    = chipH / 2f
        val padW     = 16f * scale * dp
        val padH     = 11f * scale * dp   // top/bottom padding inside chip
        val waveTimerGap = 9f * scale * dp
        val chipR    = 18f * scale * dp

        // Wave zone width (same as cluster: 5 bars × 3dp + 4 gaps × 3dp = 27dp)
        val waveW = (WAVE_BAR_W_DP * WAVE_BAR_COUNT + WAVE_BAR_GAP_DP * (WAVE_BAR_COUNT - 1)).toFloat() * dp

        // Elapsed time for timer
        val elapsedMs = if (recordingStartMs > 0L) System.currentTimeMillis() - recordingStartMs else 0L
        val totalSecs = (elapsedMs / 1000L).coerceAtLeast(0L)
        val mm = totalSecs / 60L
        val ss = totalSecs % 60L
        val timerStr = "%d:%02d".format(mm, ss)
        tapTimerPaint.textSize = 13f * scale * spd
        val timerW = tapTimerPaint.measureText(timerStr)

        // Chip dimensions
        val contentW = waveW + waveTimerGap + timerW
        val chipW    = contentW + 2f * padW
        val chipLeft = cx - chipW / 2f
        val chipRight = cx + chipW / 2f
        val chipTop  = cy - halfH
        val chipBot  = cy + halfH
        val chipRect = RectF(chipLeft, chipTop, chipRight, chipBot)

        // Shadow
        shadowPaint.maskFilter = BlurMaskFilter(halfH * 0.5f, BlurMaskFilter.Blur.NORMAL)
        val shadowR = RectF(chipLeft, chipTop + halfH * 0.2f, chipRight, chipBot + halfH * 0.2f)
        canvas.drawRoundRect(shadowR, chipR, chipR, shadowPaint)

        // Fill (opaque dark — AC2 "blickdichte Fläche")
        fillPaint.color  = TAP_CHIP_BG
        fillPaint.shader = null
        canvas.drawRoundRect(chipRect, chipR, chipR, fillPaint)
        fillPaint.alpha = 0xFF

        // Border (1dp, KlarvoTheme.Border)
        strokePaint.color       = KlarvoTheme.Border
        strokePaint.strokeWidth = 1f * dp
        strokePaint.style       = Paint.Style.STROKE
        canvas.drawRoundRect(chipRect, chipR, chipR, strokePaint)

        // Wave bars (amber, RMS-driven — AC5: reuse drawClusterWaveform unchanged)
        val waveLeft  = chipLeft + padW
        val waveRight = waveLeft + waveW
        val waveCy    = cy  // vertically centered inside chip
        drawClusterWaveform(canvas, waveLeft, waveRight, waveCy, dp)

        // Timer text (right of wave, vertically centered)
        val timerMetrics = tapTimerPaint.fontMetrics
        val timerBaseline = cy - (timerMetrics.ascent + timerMetrics.descent) / 2f
        canvas.drawText(timerStr, waveRight + waveTimerGap, timerBaseline, tapTimerPaint)
    }

    /**
     * Draws the Send (teal) circle centered at (cx, cy) with radius r.
     * Content: ➤ glyph (paper-plane, OnTeal) + "Senden" label + "tippen" hint (stacked, centered).
     * [scale] = recordingButtonSizeDp / 132f — all layout dims scale proportionally (AC2).
     */
    private fun drawTapSendCircle(canvas: Canvas, cx: Float, cy: Float, r: Float, dp: Float, spd: Float, scale: Float) {
        // Shadow
        shadowPaint.maskFilter = BlurMaskFilter(r * 0.22f, BlurMaskFilter.Blur.NORMAL)
        canvas.drawCircle(cx, cy + r * 0.08f, r, shadowPaint)

        // Teal gradient fill (150° → approximate: TealHi top-left → TealLo bottom-right)
        fillPaint.alpha  = 0xFF
        fillPaint.shader = LinearGradient(
            cx - r * 0.7f, cy - r * 0.7f,
            cx + r * 0.7f, cy + r * 0.7f,
            KlarvoTheme.TealHi, KlarvoTheme.TealLo,
            Shader.TileMode.CLAMP
        )
        canvas.drawCircle(cx, cy, r, fillPaint)
        fillPaint.shader = null

        // Content column scales proportionally with button size.
        // At 132dp: colHalf=45.5dp, glyph=46dp, gap=7dp.
        val colHalf    = 45.5f * scale * dp
        val colTop     = cy - colHalf

        // ➤ Paper-plane glyph (scales with circle)
        val glyphSize  = 46f * scale * dp
        val glyphCy    = colTop + glyphSize / 2f
        drawSendGlyph(canvas, cx, glyphCy, glyphSize)

        // "Senden" label (scales with circle)
        tapSendLabelPaint.textSize = 15f * scale * spd
        val labelGap   = 7f * scale * dp
        val labelTopY  = colTop + glyphSize + labelGap
        val labelBaseline = labelTopY - tapSendLabelPaint.ascent()
        canvas.drawText("Senden", cx, labelBaseline, tapSendLabelPaint)

        // "tippen" hint (scales with circle)
        tapSendHintPaint.textSize = 11f * scale * spd
        val hintTopY  = labelBaseline + tapSendLabelPaint.descent() + labelGap
        val hintBaseline = hintTopY - tapSendHintPaint.ascent()
        canvas.drawText("tippen", cx, hintBaseline, tapSendHintPaint)
    }

    /**
     * Draws the Cancel (dark + red-ring) circle centered at (cx, cy) with radius r.
     * Content: ✗ cross glyph (Danger) + "Abbrechen" label + "tippen" hint.
     * [scale] = recordingButtonSizeDp / 132f — all layout dims scale proportionally (AC2).
     */
    private fun drawTapCancelCircle(canvas: Canvas, cx: Float, cy: Float, r: Float, dp: Float, spd: Float, scale: Float) {
        // Shadow
        shadowPaint.maskFilter = BlurMaskFilter(r * 0.22f, BlurMaskFilter.Blur.NORMAL)
        canvas.drawCircle(cx, cy + r * 0.08f, r, shadowPaint)

        // Dark fill (rgba(20,18,18,.95) — AC2 opaque dark, AC2 non-transparent)
        fillPaint.color  = TAP_CANCEL_FILL
        fillPaint.shader = null
        canvas.drawCircle(cx, cy, r, fillPaint)
        fillPaint.alpha = 0xFF

        // Red ring border (scales with button size)
        strokePaint.color       = TAP_CANCEL_BORDER
        strokePaint.strokeWidth = 2f * scale * dp
        strokePaint.style       = Paint.Style.STROKE
        canvas.drawCircle(cx, cy, r - 1f * scale * dp, strokePaint)

        // Content column scales proportionally with button size.
        // At 132dp: colHalf=42.5dp, glyph=42dp, gap=7dp.
        val colHalf    = 42.5f * scale * dp
        val colTop     = cy - colHalf

        // ✗ cross glyph (scales with circle; internal path uses 24×24 viewBox)
        val glyphSize  = 42f * scale * dp
        val glyphCy    = colTop + glyphSize / 2f
        val glyphScale = glyphSize / 24f
        val left       = cx - glyphSize / 2f
        val top        = glyphCy - glyphSize / 2f
        tapCancelGlyphPaint.strokeWidth = 2.8f * scale * dp
        val crossPath = Path().apply {
            moveTo(left + 18f * glyphScale, top + 6f  * glyphScale)
            lineTo(left + 6f  * glyphScale, top + 18f * glyphScale)
            moveTo(left + 6f  * glyphScale, top + 6f  * glyphScale)
            lineTo(left + 18f * glyphScale, top + 18f * glyphScale)
        }
        canvas.drawPath(crossPath, tapCancelGlyphPaint)

        // "Abbrechen" label (scales with circle)
        tapCancelLabelPaint.textSize = 13f * scale * spd
        val labelGap  = 7f * scale * dp
        val labelTopY = colTop + glyphSize + labelGap
        val labelBaseline = labelTopY - tapCancelLabelPaint.ascent()
        canvas.drawText("Abbrechen", cx, labelBaseline, tapCancelLabelPaint)

        // "tippen" hint (scales with circle)
        tapCancelHintPaint.textSize = 11f * scale * spd
        val hintTopY     = labelBaseline + tapCancelLabelPaint.descent() + labelGap
        val hintBaseline = hintTopY - tapCancelHintPaint.ascent()
        canvas.drawText("tippen", cx, hintBaseline, tapCancelHintPaint)
    }

    // =========================================================================
    // RECORDING: old Klein-Cluster [✗ cancel] [amber waveform] [➤ send] — SUPERSEDED by drawTapSurface
    // Kept as dead code so the private helpers (drawSendButton, drawCancelButton) remain reachable
    // from the compiler. drawRecordingCluster() is no longer called from onDraw().
    // =========================================================================

    private fun drawRecordingCluster(canvas: Canvas) {
        val dp = resources.displayMetrics.density
        val w = width.toFloat()
        val h = height.toFloat()

        // Cluster visual bounds (centered within the window which has CLUSTER_SHADOW_PAD_DP on each side)
        val padPx = CLUSTER_SHADOW_PAD_DP * dp
        val clusterVisualW = CLUSTER_VISUAL_W_DP * dp
        val clusterVisualH = CLUSTER_VISUAL_H_DP * dp
        val clusterLeft = (w - clusterVisualW) / 2f
        val clusterTop  = (h - clusterVisualH) / 2f
        val clusterRight  = clusterLeft + clusterVisualW
        val clusterBottom = clusterTop  + clusterVisualH
        val backdropR = CLUSTER_BACKDROP_R_DP * dp

        // --- Soft backdrop shadow ---
        shadowPaint.maskFilter = BlurMaskFilter(padPx * 0.8f, BlurMaskFilter.Blur.NORMAL)
        val shadowRect = RectF(clusterLeft, clusterTop + padPx * 0.4f, clusterRight, clusterBottom + padPx * 0.4f)
        canvas.drawRoundRect(shadowRect, backdropR, backdropR, shadowPaint)

        // --- Backdrop fill: rgba(20,22,24,.55) = 0x8C141618 ---
        fillPaint.color = 0x8C141618.toInt()
        fillPaint.shader = null
        val backdropRect = RectF(clusterLeft, clusterTop, clusterRight, clusterBottom)
        canvas.drawRoundRect(backdropRect, backdropR, backdropR, fillPaint)

        // --- Static amber ring: 0 0 0 1.5dp AmberLine (NOT animated) ---
        strokePaint.color = KlarvoTheme.AmberLine
        strokePaint.strokeWidth = 1.5f * dp
        strokePaint.style = Paint.Style.STROKE
        val ringInset = 0.75f * dp  // half strokeWidth keeps ring outside the fill
        val ringRect = RectF(
            clusterLeft - ringInset, clusterTop - ringInset,
            clusterRight + ringInset, clusterBottom + ringInset
        )
        canvas.drawRoundRect(ringRect, backdropR + ringInset, backdropR + ringInset, strokePaint)

        // Cluster internal geometry
        val innerPad = CLUSTER_PAD_DP * dp
        val btnPx    = CLUSTER_BTN_DP * dp
        val gapPx    = CLUSTER_GAP_DP * dp
        val btnR     = CLUSTER_BTN_R_DP * dp
        val btnCy    = (clusterTop + clusterBottom) / 2f

        val cancelCx = clusterLeft + innerPad + btnPx / 2f    // LEFT  (was sendCx)
        val sendCx   = clusterRight - innerPad - btnPx / 2f   // RIGHT (was cancelCx)

        // --- ✗ Cancel button (LEFT) ---
        drawCancelButton(canvas, cancelCx, btnCy, btnPx / 2f, btnR, dp)

        // --- ➤ Send button (RIGHT / dock-thumb) ---
        drawSendButton(canvas, sendCx, btnCy, btnPx / 2f, btnR, dp)

        // --- Amber waveform (between buttons) ---
        val waveLeft  = cancelCx + btnPx / 2f + gapPx   // right edge of Cancel + gap
        val waveRight = sendCx - btnPx / 2f - gapPx     // left edge of Send - gap
        drawClusterWaveform(canvas, waveLeft, waveRight, btnCy, dp)

        // --- Update touch zones (half-gap extends the zones over the dead area) ---
        // Cancel zone right boundary: midpoint between cancel right edge and waveform left
        val cancelZoneRight = waveLeft - gapPx / 2f
        // Send zone left boundary: midpoint between waveform right edge and send left
        val sendZoneLeft = waveRight + gapPx / 2f
        clusterCancelZoneEnd = cancelZoneRight
        clusterSendZoneStart = sendZoneLeft
    }

    private fun drawSendButton(canvas: Canvas, cx: Float, cy: Float, r: Float, cornerR: Float, dp: Float) {
        val btnRect = RectF(cx - r, cy - r, cx + r, cy + r)

        // Button shadow
        shadowPaint.maskFilter = BlurMaskFilter(r * 0.4f, BlurMaskFilter.Blur.NORMAL)
        val btnShadowRect = RectF(btnRect.left, btnRect.top + r * 0.15f, btnRect.right, btnRect.bottom + r * 0.15f)
        canvas.drawRoundRect(btnShadowRect, cornerR, cornerR, shadowPaint)

        // Teal gradient fill — reset alpha (backdrop set fillPaint to 0x8C; buttons must be opaque)
        fillPaint.alpha = 0xFF
        fillPaint.shader = LinearGradient(
            btnRect.left, btnRect.top, btnRect.right, btnRect.bottom,
            KlarvoTheme.TealHi, KlarvoTheme.TealLo, Shader.TileMode.CLAMP
        )
        canvas.drawRoundRect(btnRect, cornerR, cornerR, fillPaint)
        fillPaint.shader = null

        // Paper-plane glyph (19dp, OnTeal, stroke 2.2dp)
        val glyphSize = 19 * dp
        drawSendGlyph(canvas, cx, cy, glyphSize)
    }

    private fun drawCancelButton(canvas: Canvas, cx: Float, cy: Float, r: Float, cornerR: Float, dp: Float) {
        val btnRect = RectF(cx - r, cy - r, cx + r, cy + r)

        // Button shadow
        shadowPaint.maskFilter = BlurMaskFilter(r * 0.4f, BlurMaskFilter.Blur.NORMAL)
        val btnShadowRect = RectF(btnRect.left, btnRect.top + r * 0.15f, btnRect.right, btnRect.bottom + r * 0.15f)
        canvas.drawRoundRect(btnShadowRect, cornerR, cornerR, shadowPaint)

        // Fill: DangerBg (0x1FEE6F63)
        fillPaint.color = KlarvoTheme.DangerBg
        canvas.drawRoundRect(btnRect, cornerR, cornerR, fillPaint)

        // Border: full-opacity Danger (mockup: border:1px solid var(--k-danger))
        strokePaint.color = KlarvoTheme.Danger
        strokePaint.strokeWidth = dp
        strokePaint.style = Paint.Style.STROKE
        canvas.drawRoundRect(btnRect, cornerR, cornerR, strokePaint)

        // ✗ glyph: M18 6 6 18 / M6 6 l12 12 on 24×24 → scaled to 19dp, stroke 2.4dp Danger
        val glyphSize = 19 * dp
        val scale = glyphSize / 24f
        val left = cx - glyphSize / 2f
        val top  = cy - glyphSize / 2f

        cancelGlyphPaint.strokeWidth = 2.4f * dp

        val cancelPath = Path().apply {
            // Line 1: (18,6) → (6,18)
            moveTo(left + 18f * scale, top + 6f * scale)
            lineTo(left + 6f * scale,  top + 18f * scale)
            // Line 2: (6,6) → (18,18)
            moveTo(left + 6f * scale,  top + 6f * scale)
            lineTo(left + 18f * scale, top + 18f * scale)
        }
        canvas.drawPath(cancelPath, cancelGlyphPaint)
    }

    private fun drawClusterWaveform(canvas: Canvas, zoneLeft: Float, zoneRight: Float, cy: Float, dp: Float) {
        val barW   = WAVE_BAR_W_DP * dp
        val barGap = WAVE_BAR_GAP_DP * dp
        val totalW = barW * WAVE_BAR_COUNT + barGap * (WAVE_BAR_COUNT - 1)
        val startX = (zoneLeft + zoneRight) / 2f - totalW / 2f + barW / 2f
        val maxBarH = WAVE_H_DP * dp
        val minBarH = maxBarH * 0.10f

        // Scrolling-history waveform — mirrors desktop FloatingBar.tsx Waveform component:
        //   levelIdx = round(i / (BAR_COUNT-1) * (levels.length-1))
        //   amplitude = max(0.12, levels[levelIdx])
        //   heightPx  = max(3, amplitude * 19)
        //
        // Motion source: each amplitude push shifts waveLevels left and appends the newest value.
        // Silence → all slots 0 → all bars at minBarH (flat/still).
        // Speech fills the buffer; stopping → loud values scroll off over 20 pushes → smooth fade.
        // No synthetic animation — the motion is purely the scrolling history.
        // Canon mandate: hwave is RMS-driven (ADR-0019 §4′ #1-Anker, Story 9-12).
        for (i in 0 until WAVE_BAR_COUNT) {
            val barX = startX + i * (barW + barGap)
            if (barX - barW / 2f < zoneLeft || barX + barW / 2f > zoneRight) continue
            val levelIdx = Math.round(i.toFloat() / (WAVE_BAR_COUNT - 1) * (waveLevels.size - 1))
            val level = waveLevels[levelIdx]
            val barH = maxOf(minBarH, level * maxBarH)
            val barRect = RectF(barX - barW / 2f, cy - barH / 2f, barX + barW / 2f, cy + barH / 2f)
            canvas.drawRoundRect(barRect, barW / 2f, barW / 2f, amberBarPaint)
        }
    }

    // =========================================================================
    // TRANSCRIBING: single teal proc bubble (.ab-bubble.proc) + rotating spinner
    // =========================================================================

    private fun drawProcBubble(canvas: Canvas) {
        val density = resources.displayMetrics.density
        val side = bubbleSizeDp * density
        val visualRadius = side / 2f
        val cx = width / 2f
        val cy = height / 2f
        val cornerPx = side * 0.30f

        squircleRect.set(cx - visualRadius, cy - visualRadius, cx + visualRadius, cy + visualRadius)

        // Soft drop shadow
        shadowPaint.maskFilter = BlurMaskFilter(side * 0.14f, BlurMaskFilter.Blur.NORMAL)
        val shadowRect = RectF(
            squircleRect.left, squircleRect.top + side * 0.06f,
            squircleRect.right, squircleRect.bottom + side * 0.06f
        )
        canvas.drawRoundRect(shadowRect, cornerPx, cornerPx, shadowPaint)

        // Teal-gradient fill (same as IDLE — proc bubble is teal squircle)
        idleFillPaint.shader = LinearGradient(
            squircleRect.left, squircleRect.top,
            squircleRect.right, squircleRect.bottom,
            KlarvoTheme.TealHi, KlarvoTheme.TealLo,
            Shader.TileMode.CLAMP
        )
        canvas.drawRoundRect(squircleRect, cornerPx, cornerPx, idleFillPaint)

        // Rotating spinner: 20dp, OnTeal stroke, 900ms — canon .ab-bubble.proc .spinner
        val spinnerSizePx = 20f * density
        val spinnerR = spinnerSizePx / 2f * 0.85f
        val startAngle = (rotationAnimator.animatedValue as? Float) ?: 0f
        arcPaint.color = KlarvoTheme.OnTeal
        arcPaint.strokeWidth = spinnerSizePx * 0.12f
        arcPaint.style = Paint.Style.STROKE
        arcPaint.strokeCap = Paint.Cap.ROUND
        val spinRect = RectF(cx - spinnerR, cy - spinnerR, cx + spinnerR, cy + spinnerR)
        canvas.drawArc(spinRect, startAngle, 270f, false, arcPaint)
    }

    // =========================================================================
    // DONE: success-green gradient (.ab-bubble.done) + dark check → back to IDLE
    // Canon: linear-gradient(150°, SuccessHi #62E0A4, Success #4FC58A) + dark check 20dp
    // =========================================================================

    private fun drawDoneBubble(canvas: Canvas) {
        val density = resources.displayMetrics.density
        val side = bubbleSizeDp * density
        val visualRadius = side / 2f
        val cx = width / 2f
        val cy = height / 2f
        val cornerPx = side * 0.30f

        squircleRect.set(cx - visualRadius, cy - visualRadius, cx + visualRadius, cy + visualRadius)

        // Soft drop shadow
        shadowPaint.maskFilter = BlurMaskFilter(side * 0.14f, BlurMaskFilter.Blur.NORMAL)
        val shadowRect = RectF(
            squircleRect.left, squircleRect.top + side * 0.06f,
            squircleRect.right, squircleRect.bottom + side * 0.06f
        )
        canvas.drawRoundRect(shadowRect, cornerPx, cornerPx, shadowPaint)

        // Success-green gradient: SuccessHi (#62E0A4) → Success (#4FC58A) at ~150°
        idleFillPaint.shader = LinearGradient(
            squircleRect.left, squircleRect.top,
            squircleRect.right, squircleRect.bottom,
            KlarvoTheme.SuccessHi, KlarvoTheme.Success,
            Shader.TileMode.CLAMP
        )
        canvas.drawRoundRect(squircleRect, cornerPx, cornerPx, idleFillPaint)

        // Dark check polyline 20dp, stroke 3dp, OnTeal (dark on green)
        // Canon check: similar to existing drawCheckMark but with OnTeal and 3dp stroke
        val checkSize = 20f * density / 2f  // half-size for the helper
        drawCheckMark(canvas, cx, cy, checkSize, KlarvoTheme.OnTeal, 3f * density)
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    private fun drawCheckMark(canvas: Canvas, cx: Float, cy: Float, size: Float, color: Int, strokeWidthPx: Float) {
        val checkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            this.color = color
            style = Paint.Style.STROKE
            strokeWidth = strokeWidthPx
            strokeCap = Paint.Cap.ROUND
            strokeJoin = Paint.Join.ROUND
        }
        val path = Path().apply {
            moveTo(cx - size * 0.55f, cy)
            lineTo(cx - size * 0.1f,  cy + size * 0.45f)
            lineTo(cx + size * 0.6f,  cy - size * 0.45f)
        }
        canvas.drawPath(path, checkPaint)
    }

    /**
     * Draws the send (paper-plane) glyph centered at (cx, cy).
     * Two sub-paths on a 24×24 viewBox, scaled to [glyphSizePx] and centered at origin:
     *   Sub-path 1: M22,2 → L11,13  (diagonal tail)
     *   Sub-path 2: M22,2 → L15,22 → L11,13 → L2,9 → close  (body polygon)
     */
    private fun drawSendGlyph(canvas: Canvas, cx: Float, cy: Float, glyphSizePx: Float) {
        val dp = resources.displayMetrics.density
        if (glyphSizePx != lastSendGlyphSize) {
            val scale = glyphSizePx / 24f
            val ox = -glyphSizePx / 2f  // offset to center the 24×24 box at origin
            val oy = -glyphSizePx / 2f
            sendGlyphPath.reset()
            // Sub-path 1: diagonal M22,2 L11,13
            sendGlyphPath.moveTo(22f * scale + ox, 2f  * scale + oy)
            sendGlyphPath.lineTo(11f * scale + ox, 13f * scale + oy)
            // Sub-path 2: body polygon M22,2 L15,22 L11,13 L2,9 Z
            sendGlyphPath.moveTo(22f * scale + ox, 2f  * scale + oy)
            sendGlyphPath.lineTo(15f * scale + ox, 22f * scale + oy)
            sendGlyphPath.lineTo(11f * scale + ox, 13f * scale + oy)
            sendGlyphPath.lineTo( 2f * scale + ox,  9f * scale + oy)
            sendGlyphPath.close()
            lastSendGlyphSize = glyphSizePx
        }
        sendGlyphPaint.strokeWidth = 2.2f * dp
        canvas.save()
        canvas.translate(cx, cy)
        canvas.drawPath(sendGlyphPath, sendGlyphPaint)
        canvas.restore()
    }

    // =========================================================================
    // RECORDING: HOLD Cancel surface — vereinfacht (Story 9-14 re-scope, ADR-0019 Amendment
    // 2026-07-01). Anchor bubble = the idle bubble itself (same draw path, same size, same
    // on-screen position — AC2) + ONE round Abbrechen target growing diagonally up-and-toward-
    // center from it (AC1/AC3/AC4) + a calm waveform chip hugging the bubble. Dragging away from
    // the bubble fades it (~.32 alpha) and shows a ghost squircle following the live finger (AC6).
    // Supersedes the prior two-target (Sperren+Abbrechen) build — rejected at GATE-4, see Dev Notes.
    // =========================================================================

    private fun drawHoldTargets(canvas: Canvas) {
        val dp  = resources.displayMetrics.density
        val spd = resources.displayMetrics.scaledDensity
        val w   = width.toFloat()
        val h   = height.toFloat()

        val shadowPad    = HOLD_SHADOW_PAD_DP * dp
        val bubbleDiamPx = bubbleSizeDp * dp
        val bubbleRPx    = bubbleDiamPx / 2f
        val restRPx      = recordingButtonSizeDp * dp / 2f
        val activeRPx    = recordingButtonSizeDp * HOLD_CANCEL_ACTIVE_SCALE * dp / 2f
        val offsetXPx    = HOLD_CANCEL_OFFSET_X_DP * dp
        val offsetYPx    = HOLD_CANCEL_OFFSET_Y_DP * dp

        val bubbleCenter = holdBubbleCenter(dockSide, w, h, shadowPad, bubbleDiamPx)
        val cancelCenter = holdCancelCenter(dockSide, bubbleCenter, offsetXPx, offsetYPx)

        // --- 1. Abbrechen target (drawn first — bubble/chip sit visually on top) ---
        drawHoldZone(canvas, cancelCenter.x, cancelCenter.y, restRPx, activeRPx,
            active = holdTargetHit == HoldTarget.CANCEL, dp = dp, spd = spd)

        // --- 2. Waveform chip hugging the bubble (inner side, same vertical level) ---
        val chipGapPx = HOLD_CHIP_BUBBLE_GAP_DP * dp
        val chipCx = if (dockSide == "left") {
            bubbleCenter.x + bubbleRPx + chipGapPx  // chip to the right of a left-docked bubble
        } else {
            bubbleCenter.x - bubbleRPx - chipGapPx  // chip to the left of a right-docked bubble
        }
        drawHoldChip(canvas, chipCx, bubbleCenter.y, dockSide, dp, spd)

        // --- 3. Anchor bubble + ghost dynamics (AC6, Task 3.5) ---
        if (holdDragging) {
            // Origin fades to ~.32 alpha (canon `opacity:.32`) — the anchor never moves, only
            // fades; no amber ring while faded (matches the canon's `sHit` origin styling).
            drawTealSquircle(canvas, bubbleCenter.x, bubbleCenter.y, bubbleDiamPx, alpha = 0x52)
            drawKLetter(canvas, bubbleCenter.x, bubbleCenter.y, bubbleRPx, alpha = 0x52)

            // Ghost squircle follows the LIVE finger position — not a derived/interpolated point
            // (canon: the mockup's ghost position is illustrative, the formula input is the real
            // touch). ~0.92× the anchor's size (canon ghost/heldbub ratio 44/48).
            val ghostDiamPx = bubbleDiamPx * 0.92f
            drawTealSquircle(canvas, holdFingerX, holdFingerY, ghostDiamPx, alpha = 0xFF)
            drawKLetter(canvas, holdFingerX, holdFingerY, ghostDiamPx / 2f, alpha = 0xFF)
        } else {
            drawTealSquircle(canvas, bubbleCenter.x, bubbleCenter.y, bubbleDiamPx, alpha = 0xFF)

            // Amber holding-ring (box-shadow 0 0 0 5dp AmberLine — .heldbub), rest-state only.
            val cornerPx = bubbleDiamPx * 0.30f
            val ringOutset = 5f * dp
            strokePaint.color       = KlarvoTheme.AmberLine
            strokePaint.strokeWidth = ringOutset
            strokePaint.style       = Paint.Style.STROKE
            val halfOutset = ringOutset / 2f
            holdBubbleRingRect.set(
                bubbleCenter.x - bubbleRPx - halfOutset, bubbleCenter.y - bubbleRPx - halfOutset,
                bubbleCenter.x + bubbleRPx + halfOutset, bubbleCenter.y + bubbleRPx + halfOutset
            )
            canvas.drawRoundRect(holdBubbleRingRect, cornerPx + halfOutset, cornerPx + halfOutset, strokePaint)

            drawKLetter(canvas, bubbleCenter.x, bubbleCenter.y, bubbleRPx, alpha = 0xFF)
        }

        // --- 4. Live caption above the (origin) bubble, mirrors .reccap — text swaps to the
        //        "Finger auf Abbrechen" variant only when the finger is actually on the target
        //        (AC6, canon `sHit` text), independent of the broader holdDragging dead-zone. ---
        drawHoldCaption(canvas, bubbleCenter.x, bubbleCenter.y - bubbleRPx - 24f * dp,
            hit = holdTargetHit == HoldTarget.CANCEL, dp = dp, spd = spd)
    }

    /**
     * Draws the single Abbrechen target centered at (cx, cy) (Task 3.2 — `isLock` branch removed,
     * Story 9-14 re-scope 2026-07-01: there is only one target now).
     * REST: dark fill, red-line 2dp border, ✕ icon+label muted (AC1).
     * ACTIVE (grow-on-target, AC4): radial-gradient red fill at [activeRadius], glow ring, white
     * label, "loslassen = abbrechen" text. Center stays fixed — only the drawn radius/style changes.
     */
    private fun drawHoldZone(
        canvas: Canvas, cx: Float, cy: Float, restRadius: Float, activeRadius: Float,
        active: Boolean, dp: Float, spd: Float
    ) {
        val r = if (active) activeRadius else restRadius

        if (active) {
            // Glow ring (box-shadow 0 0 0 8dp @22% + soft outer glow)
            strokePaint.color       = HOLD_DANGER_LINE
            strokePaint.strokeWidth = 8f * dp
            strokePaint.style       = Paint.Style.STROKE
            canvas.drawCircle(cx, cy, r + 4f * dp, strokePaint)

            // Radial gradient (finding C): rebuild only when center/radius actually changed —
            // stable while the target stays ACTIVE across repeated invalidate() calls.
            val gradCy = cy - r * 0.24f
            if (holdCancelGradient == null || holdCancelGradientCx != cx || holdCancelGradientCy != gradCy || holdCancelGradientR != r) {
                holdCancelGradientCx = cx
                holdCancelGradientCy = gradCy
                holdCancelGradientR  = r
                holdCancelGradient = RadialGradient(
                    cx, gradCy, r, HOLD_DANGER_HI, KlarvoTheme.Danger, Shader.TileMode.CLAMP
                )
            }
            fillPaint.shader = holdCancelGradient
            canvas.drawCircle(cx, cy, r, fillPaint)
            fillPaint.shader = null
            fillPaint.alpha  = 0xFF
        } else {
            fillPaint.color  = HOLD_ZONE_REST_BG
            fillPaint.shader = null
            canvas.drawCircle(cx, cy, r, fillPaint)
            fillPaint.alpha = 0xFF

            strokePaint.color       = HOLD_DANGER_LINE
            strokePaint.strokeWidth = 2f * dp
            strokePaint.style       = Paint.Style.STROKE
            canvas.drawCircle(cx, cy, r - dp, strokePaint)
        }

        // ✕ glyph (drawn as text, mirrors mockup's <span>✕</span>)
        val iconSizeDp = if (active) 46f else 34f
        val iconCy = cy - r * 0.18f
        val labelColor = if (active) 0xFFFFFFFF.toInt() else HOLD_DANGER_HI
        holdCancelGlyphPaint.color    = labelColor
        holdCancelGlyphPaint.textSize = iconSizeDp * dp
        val glyphMetrics = holdCancelGlyphPaint.fontMetrics
        canvas.drawText("✕", cx, iconCy - (glyphMetrics.ascent + glyphMetrics.descent) / 2f, holdCancelGlyphPaint)

        // Two-line label
        val labelStr = if (active) "loslassen\n= abbrechen" else "ziehen zum\nAbbrechen"
        holdZoneLabelPaint.color    = labelColor
        holdZoneLabelPaint.textSize = (if (active) 13f else 12f) * spd
        holdZoneLabelPaint.isFakeBoldText = active
        val lines = labelStr.split("\n")
        val lineMetrics = holdZoneLabelPaint.fontMetrics
        val lineH = lineMetrics.descent - lineMetrics.ascent
        val labelTopY = iconCy + iconSizeDp * dp / 2f + 7f * dp
        for ((i, line) in lines.withIndex()) {
            val baseline = labelTopY - lineMetrics.ascent + i * lineH
            canvas.drawText(line, cx, baseline, holdZoneLabelPaint)
        }
    }

    /**
     * Draws the waveform chip hugging the bubble: [amber wave bars] [gap] [mm:ss timer] inside a
     * dark rounded chip, same amplitude-driven bars as drawTapChip but more compact (smaller HOLD
     * window budget — Dev Notes). [chipNearX] = the chip's edge closest to the bubble.
     */
    private fun drawHoldChip(canvas: Canvas, chipNearX: Float, cy: Float, dockSide: String, dp: Float, spd: Float) {
        val halfH = HOLD_CHIP_H_DP * dp / 2f
        val padW  = 10f * dp
        val waveTimerGap = 7f * dp
        val chipR = 16f * dp

        val waveW = (WAVE_BAR_W_DP * WAVE_BAR_COUNT + WAVE_BAR_GAP_DP * (WAVE_BAR_COUNT - 1)).toFloat() * dp

        val elapsedMs = if (recordingStartMs > 0L) System.currentTimeMillis() - recordingStartMs else 0L
        val totalSecs = (elapsedMs / 1000L).coerceAtLeast(0L)
        val timerStr = "%d:%02d".format(totalSecs / 60L, totalSecs % 60L)
        tapTimerPaint.textSize = 12f * spd
        val timerW = tapTimerPaint.measureText(timerStr)

        val contentW = waveW + waveTimerGap + timerW
        val chipW    = contentW + 2f * padW
        // Chip extends AWAY from the bubble (chipNearX is the bubble-facing edge).
        val chipLeft  = if (dockSide == "left") chipNearX else chipNearX - chipW
        val chipRight = chipLeft + chipW
        holdChipRect.set(chipLeft, cy - halfH, chipRight, cy + halfH)

        // Shadow blur (finding C): halfH is a fixed constant (HOLD_CHIP_H_DP * dp / 2f) —
        // rebuild only if it changed, not on every amplitude-driven invalidate() during the hold.
        if (holdChipShadowBlur == null || holdChipShadowBlurR != halfH) {
            holdChipShadowBlurR = halfH
            holdChipShadowBlur  = BlurMaskFilter(halfH * 0.5f, BlurMaskFilter.Blur.NORMAL)
        }
        shadowPaint.maskFilter = holdChipShadowBlur
        holdChipShadowRect.set(holdChipRect.left, holdChipRect.top + halfH * 0.2f, holdChipRect.right, holdChipRect.bottom + halfH * 0.2f)
        canvas.drawRoundRect(holdChipShadowRect, chipR, chipR, shadowPaint)

        fillPaint.color  = TAP_CHIP_BG
        fillPaint.shader = null
        canvas.drawRoundRect(holdChipRect, chipR, chipR, fillPaint)
        fillPaint.alpha = 0xFF

        strokePaint.color       = KlarvoTheme.Border
        strokePaint.strokeWidth = 1f * dp
        strokePaint.style       = Paint.Style.STROKE
        canvas.drawRoundRect(holdChipRect, chipR, chipR, strokePaint)

        val waveLeft  = chipLeft + padW
        val waveRight = waveLeft + waveW
        drawClusterWaveform(canvas, waveLeft, waveRight, cy, dp)

        val timerMetrics = tapTimerPaint.fontMetrics
        val timerBaseline = cy - (timerMetrics.ascent + timerMetrics.descent) / 2f
        canvas.drawText(timerStr, waveRight + waveTimerGap, timerBaseline, tapTimerPaint)
    }

    /**
     * Live caption + amber dot with halo — mirrors .reccap. [hit] swaps the text to the
     * "Finger auf Abbrechen" variant while the finger sits on the Abbrechen target (AC6, Task 3.4,
     * canon `sHit` text) — at rest it reads "Aufnahme · loslassen = senden".
     */
    private fun drawHoldCaption(canvas: Canvas, cx: Float, cy: Float, hit: Boolean, dp: Float, spd: Float) {
        holdCaptionPaint.textSize = 13f * spd
        val text = if (hit) "Finger auf Abbrechen · loslassen löst aus" else "Aufnahme · loslassen = senden"
        val textW = holdCaptionPaint.measureText(text)
        val dotR  = 4f * dp
        val gap   = 8f * dp
        val totalW = dotR * 2f + gap + textW
        val startX = cx - totalW / 2f

        val dotCx = startX + dotR
        strokePaint.color       = KlarvoTheme.AmberLine
        strokePaint.strokeWidth = 4f * dp
        strokePaint.style       = Paint.Style.STROKE
        canvas.drawCircle(dotCx, cy, dotR, strokePaint)
        fillPaint.color  = KlarvoTheme.Amber
        fillPaint.shader = null
        canvas.drawCircle(dotCx, cy, dotR, fillPaint)
        fillPaint.alpha = 0xFF

        val metrics = holdCaptionPaint.fontMetrics
        canvas.drawText(text, dotCx + dotR + gap, cy - (metrics.ascent + metrics.descent) / 2f, holdCaptionPaint)
    }

    // Legacy helpers kept for API compatibility with any remaining callers.

    private fun drawWaveformBarsInZone(
        canvas: Canvas, zoneLeft: Float, zoneRight: Float, cx: Float, cy: Float, maxBarHalfHeight: Float
    ) {
        drawClusterWaveform(canvas, zoneLeft, zoneRight, cy, resources.displayMetrics.density)
    }
}
