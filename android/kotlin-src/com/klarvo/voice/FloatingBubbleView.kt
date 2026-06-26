package com.klarvo.voice

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.*
import android.view.View
import android.view.animation.AccelerateDecelerateInterpolator
import android.view.animation.LinearInterpolator
import android.view.animation.OvershootInterpolator

/**
 * Custom View that draws the floating voice-input bubble (and the RECORDING cluster).
 * All rendering via Canvas — no asset files needed.
 *
 * States:
 *   IDLE          -- teal-gradient squircle + dark "K" (OnTeal) + faint teal glass ring
 *                    Canon: .ab-bubble.idle
 *   RECORDING     -- depends on [holdDockActive]:
 *     holdDockActive=false: control cluster (Modell B / ADR-0019 §4′ #2):
 *                    [✗ cancel red] [amber waveform] [➤ send teal] on a dark semi-transparent
 *                    backdrop with a static amber ring. Window grows from single bubble → cluster.
 *     holdDockActive=true: HOLD dock (ADR-0019 §4′-Amendment #4):
 *                    Holdstrip (slidehint "‹" + "ziehen zum Abbrechen" + amber waveform) to the
 *                    left, heldbub (teal squircle + amber ring + finger indicator) to the right,
 *                    lockchip (▲ animated + lock icon + "hoch = sperren") above. Wider+taller
 *                    window. No tappable ➤/✗ zones — release sends, drag cancels/locks.
 *   TRANSCRIBING  -- single teal proc bubble (.ab-bubble.proc): teal squircle + rotating spinner.
 *                    Window collapses back to single-bubble size.
 *   DONE          -- success-green gradient squircle + dark check polyline (.ab-bubble.done),
 *                    then returns to IDLE (via doneFlashRunnable).
 *
 * Touch zones in RECORDING (cluster layout, left→right: cancel | waveform | send):
 *   - Left zone   -> ✗ Cancel (isTouchInCancelZone) — guarded by !holdDockActive
 *   - Dead zone   -> waveform (no action)
 *   - Right zone  -> ➤ Send  (isTouchInConfirmZone) — guarded by !holdDockActive
 *   KlarvoOverlayService reads these helpers and routes accordingly; tap on waveform/backdrop = no-op.
 *   When holdDockActive=true both zone helpers return false (release = send, drag = cancel/lock).
 *
 * Color semantics (DT5 — binding rule):
 *   KlarvoTheme.Teal    = brand / ready / processing / focus-ring
 *   KlarvoTheme.OnTeal  = dark "K" letter on teal fill (IDLE) / send-glyph stroke
 *   KlarvoTheme.TealBg  = ~12% alpha faint ring (IDLE glass-ring accent)
 *   KlarvoTheme.Amber   = waveform in RECORDING cluster ONLY (amber = live only)
 *   KlarvoTheme.Danger  = ✗ cancel glyph (ADR-0019: red = Abbrechen only)
 *   KlarvoTheme.SuccessHi / Success = done bubble gradient (150°)
 */
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
     * When true and state == RECORDING, draws the HOLD dock (ADR-0019 §4′-Amendment #4)
     * instead of the normal recording cluster. Set by KlarvoOverlayService when
     * pushToTalkActive=true (long-press + longPressMode == HOLD). Both isTouchInConfirmZone
     * and isTouchInCancelZone return false while this is true (no tappable zones during hold).
     */
    var holdDockActive: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            if (value && state == State.RECORDING) {
                holdArrowPhase = 0f
                holdArrowAnimator.start()
            } else {
                holdArrowAnimator.cancel()
            }
            requestLayout()
            invalidate()
        }

    /** Current bubble size in dp. Changed via setBubbleSize(). */
    private var bubbleSizeDp: Int = 56

    companion object {
        // Cluster visual dimensions (RECORDING state, Modell B).
        // Width breakdown: 6(pad) + 40(cancel) + 9(gap) + 40(wave) + 9(gap) + 40(send) + 6(pad) = 150dp
        // Height: 6(pad) + 40(btn) + 6(pad) = 52dp
        const val CLUSTER_VISUAL_W_DP = 150
        const val CLUSTER_VISUAL_H_DP = 52
        const val CLUSTER_SHADOW_PAD_DP = 8  // padding around visual cluster for shadow/touch

        // Internal cluster geometry constants
        private const val CLUSTER_PAD_DP = 6
        private const val CLUSTER_BTN_DP = 40
        private const val CLUSTER_GAP_DP = 9
        private const val CLUSTER_BTN_R_DP = 12   // button corner radius
        private const val CLUSTER_BACKDROP_R_DP = 18

        // Waveform bars inside the cluster (canon .hwave: 5 bars × 3dp, gap 3dp)
        private const val WAVE_BAR_W_DP = 3
        private const val WAVE_BAR_GAP_DP = 3
        private const val WAVE_BAR_COUNT = 5
        private const val WAVE_H_DP = 18   // container height per AC4

        // HOLD dock dimensions (PTT mode: .ab-holddock surfaces, ADR-0019 §4′-Amendment #4)
        // Layout (right→left): heldbub (40dp) · gap (11dp) · holdstrip (~139dp)
        // Total visual width: ~139 (holdstrip) + 11 (gap) + 40 (heldbub) = ~190dp
        const val HOLDDOCK_VISUAL_W_DP = 190       // holdstrip + gap + heldbub (without shadow pad)
        const val HOLDDOCK_VISUAL_H_DP = 52        // matches CLUSTER_VISUAL_H_DP
        const val HOLDDOCK_SHADOW_PAD_DP = 8       // matches CLUSTER_SHADOW_PAD_DP
        const val HOLDDOCK_LOCKCHIP_H_DP = 28      // lockchip area above holdstrip (incl. ~4dp bottom gap)
        // Total window height = HOLDDOCK_VISUAL_H_DP + HOLDDOCK_LOCKCHIP_H_DP + 2*HOLDDOCK_SHADOW_PAD_DP = 96dp

        // Internal HOLD dock geometry (from canon .ab-holddock CSS)
        private const val HOLD_HELDBUB_DP = 40          // .ab-heldbub width/height
        private const val HOLD_HELDBUB_R_DP = 12        // .ab-heldbub border-radius
        private const val HOLD_RING_OUTSET_DP = 4       // AmberLine box-shadow outset (4dp)
        private const val HOLD_INNER_RING_INSET_DP = 8  // .ab-heldbub .ring: inset -8dp
        private const val HOLD_INNER_RING_R_DP = 18     // .ab-heldbub .ring border-radius
        private const val HOLD_GAP_DP = 11              // gap between holdstrip and heldbub
        private const val HOLDSTRIP_L_PAD_DP = 11       // .ab-holdstrip: left padding
        private const val HOLDSTRIP_R_PAD_DP = 10       // .ab-holdstrip: right padding
        private const val HOLDSTRIP_INNER_GAP_DP = 9    // gap between slidehint and waveform zone
        private const val HOLDSTRIP_R_DP = 18           // .ab-holdstrip: border-radius
        private const val HOLD_FINGER_DP = 26           // .ab-heldbub .finger: diameter
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

    // --- HOLD dock pre-allocated paints (textSize/alpha set per draw; pre-allocated to avoid GC) ---
    private val holdArrPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Amber
        textAlign = Paint.Align.LEFT
    }
    private val holdCancelTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Dim
        textAlign = Paint.Align.LEFT
    }
    private val holdLockTextPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Muted
        textAlign = Paint.Align.LEFT
        typeface = Typeface.MONOSPACE
    }
    private val holdUpPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Amber
        textAlign = Paint.Align.CENTER
    }
    private val holdLockIconPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Muted
    }

    // --- HOLD dock arrow animation (AC1a: hint arrows pulse infinite ease-in-out ~1.1s) ---
    private var holdArrowPhase: Float = 0f  // 0..1 interpolated by holdArrowAnimator

    private val holdArrowAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
        duration = 550L  // half-period: 1100ms / 2 = 550ms for REVERSE repeat
        repeatCount = ValueAnimator.INFINITE
        repeatMode = ValueAnimator.REVERSE
        interpolator = AccelerateDecelerateInterpolator()
        addUpdateListener { anim ->
            holdArrowPhase = anim.animatedValue as Float
            invalidate()
        }
    }

    // --- HOLD dock pre-allocated scratch objects (no GC on holdArrowAnimator's hot path) ---
    // Hoisted from drawHoldDock/drawLockIcon which run on every frame of the infinite animator.
    private val holdDockStripRect   = RectF()   // holdstrip bounds
    private val holdDockHeldbubRect = RectF()   // heldbub bounds
    private val holdScratchRect     = RectF()   // reused for all inline shadow/ring RectF args
    private val holdStripBlurFilter by lazy {
        val dp = resources.displayMetrics.density
        BlurMaskFilter(HOLDDOCK_SHADOW_PAD_DP * dp * 0.8f, BlurMaskFilter.Blur.NORMAL)
    }
    private val heldbubBlurFilter by lazy {
        val dp = resources.displayMetrics.density
        BlurMaskFilter(HOLD_HELDBUB_DP * dp * 0.25f, BlurMaskFilter.Blur.NORMAL)
    }
    // Pre-allocated for drawLockIcon — replaces per-call Paint(paint).apply{…} + RectF() allocs.
    private val lockBodyRect      = RectF()
    private val lockShackleRect   = RectF()
    private val lockBodyFillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val lockShacklePaint  = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style     = Paint.Style.STROKE
        strokeCap = Paint.Cap.BUTT
    }

    // --- Touch zone boundaries (updated each draw, used by isTouchInConfirmZone / Cancel) ---
    // Send zone:   [clusterSendZoneStart, width]   (RIGHT side — thumb position)
    // Cancel zone: [0, clusterCancelZoneEnd]        (LEFT side)
    // Dead zone:   waveform area between them
    private var clusterSendZoneStart = 0f
    private var clusterCancelZoneEnd = 0f

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
        holdArrowAnimator.cancel()
    }

    private fun updateAnimators() {
        when (state) {
            State.RECORDING -> {
                // Cluster waveform motion comes from the scrolling waveLevels history (desktop parity).
                // No barAnimator needed — each amplitude push triggers invalidate().
                // No amber pulse-ring in Modell B — the static ring on the backdrop replaces it.
                // No scale pop: the cluster draws in-place, no bounce animation.
                rotationAnimator.cancel()
                // Reset history so each new recording starts flat (desktop: setLevels(new Array(20).fill(0))).
                waveLevels.fill(0f)
                // HOLD arrow animation: start when holdDockActive, stop otherwise.
                // Primary fix (finding 1): the real longPressRunnable sets holdDockActive=true
                // BEFORE startRecording() calls setState(RECORDING). The holdDockActive setter
                // checks state==RECORDING and still sees IDLE, so it cancels the animator instead
                // of starting it. This path runs during the state transition (called by the state
                // setter) and sees the already-true holdDockActive — starts the animator correctly.
                // Also covers onAttachedToWindow → updateAnimators() reattach.
                if (holdDockActive) {
                    holdArrowPhase = 0f
                    if (!holdArrowAnimator.isRunning) holdArrowAnimator.start()
                } else {
                    holdArrowAnimator.cancel()
                }
            }
            State.TRANSCRIBING -> {
                // Proc bubble: rotating spinner, no waveform.
                if (!rotationAnimator.isRunning) rotationAnimator.start()
                holdArrowAnimator.cancel()
            }
            State.DONE, State.IDLE -> {
                rotationAnimator.cancel()
                holdArrowAnimator.cancel()
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

    /** True when [touchX] hits the ➤ Send button zone (right side of the cluster).
     *  Returns false when holdDockActive=true — no tappable ➤ zone during a physical hold. */
    fun isTouchInConfirmZone(touchX: Float): Boolean =
        state == State.RECORDING && !holdDockActive && clusterSendZoneStart > 0f && touchX >= clusterSendZoneStart

    /** True when [touchX] hits the ✗ Cancel button zone (left side of the cluster).
     *  Returns false when holdDockActive=true — no tappable ✗ zone during a physical hold. */
    fun isTouchInCancelZone(touchX: Float): Boolean =
        state == State.RECORDING && !holdDockActive && clusterCancelZoneEnd > 0f && touchX <= clusterCancelZoneEnd

    // --- onDraw ---

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        when (state) {
            State.IDLE       -> drawIdleBubble(canvas)
            State.RECORDING  -> if (holdDockActive) drawHoldDock(canvas) else drawRecordingCluster(canvas)
            State.TRANSCRIBING -> drawProcBubble(canvas)
            State.DONE       -> drawDoneBubble(canvas)
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

        squircleRect.set(cx - visualRadius, cy - visualRadius, cx + visualRadius, cy + visualRadius)

        // Soft drop shadow
        shadowPaint.maskFilter = BlurMaskFilter(side * 0.14f, BlurMaskFilter.Blur.NORMAL)
        val shadowRect = RectF(
            squircleRect.left, squircleRect.top + side * 0.06f,
            squircleRect.right, squircleRect.bottom + side * 0.06f
        )
        canvas.drawRoundRect(shadowRect, cornerPx, cornerPx, shadowPaint)

        // Teal-gradient fill
        idleFillPaint.shader = LinearGradient(
            squircleRect.left, squircleRect.top,
            squircleRect.right, squircleRect.bottom,
            KlarvoTheme.TealHi, KlarvoTheme.TealLo,
            Shader.TileMode.CLAMP
        )
        canvas.drawRoundRect(squircleRect, cornerPx, cornerPx, idleFillPaint)

        // Faint teal accent ring
        val ringStrokePx = 2f * density
        idleRingPaint.strokeWidth = ringStrokePx
        val ringHalf = ringStrokePx / 2f
        val ringRect = RectF(
            squircleRect.left - ringHalf, squircleRect.top - ringHalf,
            squircleRect.right + ringHalf, squircleRect.bottom + ringHalf
        )
        canvas.drawRoundRect(ringRect, cornerPx + ringHalf, cornerPx + ringHalf, idleRingPaint)

        // Dark "K"
        kLetterPaint.textSize = visualRadius * 0.85f
        val textCy = cy - (kLetterPaint.ascent() + kLetterPaint.descent()) / 2f
        canvas.drawText("K", cx, textCy, kLetterPaint)
    }

    // =========================================================================
    // RECORDING: control cluster [✗ cancel] [amber waveform] [➤ send]
    // Cancel (left, Danger), waveform (center, amber, RMS-driven), Send (right, Teal — dock/thumb)
    // Canon .ab-cluster: r18dp backdrop, static amber ring, 6dp pad, 9dp gap (§4′-Amendment 2026-06-21 #2)
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
    // HOLD DOCK: .ab-holddock (PTT mode — ADR-0019 §4′-Amendment #4)
    // Holdstrip (slidehint + waveform) left · heldbub (teal squircle) right · lockchip above.
    // =========================================================================

    /**
     * Draws the HOLD dock when holdDockActive=true in RECORDING state.
     * Window is taller than the cluster: HOLDDOCK_LOCKCHIP_H_DP extra at the top for the lockchip
     * affordance (animated ▲ + lock icon + "hoch = sperren").
     */
    private fun drawHoldDock(canvas: Canvas) {
        val dp  = resources.displayMetrics.density
        val spd = resources.displayMetrics.scaledDensity
        val w   = width.toFloat()

        // --- Layout constants ---
        val shadowPad     = HOLDDOCK_SHADOW_PAD_DP * dp
        val lockchipZoneH = HOLDDOCK_LOCKCHIP_H_DP * dp   // 28dp at top of window
        val heldbubSide   = HOLD_HELDBUB_DP * dp           // 40dp
        val heldbubR      = HOLD_HELDBUB_R_DP * dp         // 12dp
        val holdGap       = HOLD_GAP_DP * dp               // 11dp between holdstrip and heldbub
        val holdstripR    = HOLDSTRIP_R_DP * dp            // 18dp

        // Holdstrip row: visual zone starts after lockchip zone + top shadow pad
        val visualRowTop = lockchipZoneH + shadowPad       // 36dp from window top
        val visualRowH   = HOLDDOCK_VISUAL_H_DP * dp       // 52dp
        val rowCy        = visualRowTop + visualRowH / 2f  // 62dp from window top

        // Horizontal layout (right-anchored, same right edge as idle bubble).
        // Use pre-allocated class-level RectFs — no allocation on the animator hot path.
        val heldbubRight = w - shadowPad
        val heldbubLeft  = heldbubRight - heldbubSide
        val heldbubCx    = (heldbubLeft + heldbubRight) / 2f
        holdDockHeldbubRect.set(heldbubLeft, rowCy - heldbubSide / 2f, heldbubRight, rowCy + heldbubSide / 2f)

        val holdstripRight = heldbubLeft - holdGap
        val holdstripLeft  = shadowPad
        holdDockStripRect.set(holdstripLeft, rowCy - heldbubSide / 2f, holdstripRight, rowCy + heldbubSide / 2f)

        // Non-token colors (not in KlarvoTheme — local constants)
        val holdBgColor  = 0xEB141618.toInt()
        val fingerFill   = 0x26ECEEEF.toInt()
        val fingerBorder = 0x73ECEEEF.toInt()

        // Arrow animation (AC1a): phase 0..1 driven by holdArrowAnimator
        val arrowAlpha     = 0.5f + holdArrowPhase * 0.5f  // 0.5 ↔ 1.0
        val slideArrOffset = holdArrowPhase * 4f * dp      // "‹" translate left: 0..4dp
        val lockArrOffset  = holdArrowPhase * 4f * dp      // "▲" translate up: 0..4dp

        // --- 1. Holdstrip soft shadow (pre-allocated BlurMaskFilter — finding 6) ---
        shadowPaint.maskFilter = holdStripBlurFilter
        holdScratchRect.set(holdDockStripRect.left, holdDockStripRect.top + shadowPad * 0.3f, holdDockStripRect.right, holdDockStripRect.bottom + shadowPad * 0.3f)
        canvas.drawRoundRect(holdScratchRect, holdstripR, holdstripR, shadowPaint)

        // --- 2. Holdstrip backdrop: rgba(20,22,24,.92) ---
        fillPaint.color  = holdBgColor
        fillPaint.shader = null
        canvas.drawRoundRect(holdDockStripRect, holdstripR, holdstripR, fillPaint)
        fillPaint.alpha = 0xFF  // reset

        // --- 3. Holdstrip AmberLine ring (1.5dp outset) ---
        val ringInset = 0.75f * dp
        strokePaint.color       = KlarvoTheme.AmberLine
        strokePaint.strokeWidth = 1.5f * dp
        strokePaint.style       = Paint.Style.STROKE
        holdScratchRect.set(holdDockStripRect.left - ringInset, holdDockStripRect.top - ringInset, holdDockStripRect.right + ringInset, holdDockStripRect.bottom + ringInset)
        canvas.drawRoundRect(holdScratchRect, holdstripR + ringInset, holdstripR + ringInset, strokePaint)

        // --- 4. Holdstrip content: animated "‹" + "ziehen zum Abbrechen" + waveform ---
        val lPad     = HOLDSTRIP_L_PAD_DP * dp
        val rPad     = HOLDSTRIP_R_PAD_DP * dp
        val innerGap = HOLDSTRIP_INNER_GAP_DP * dp
        val contentLeft  = holdstripLeft + lPad
        val contentRight = holdstripRight - rPad
        val contentCy    = rowCy

        // Waveform zone: fixed right-aligned inside holdstrip content
        val waveZoneW      = (WAVE_BAR_W_DP * WAVE_BAR_COUNT + WAVE_BAR_GAP_DP * (WAVE_BAR_COUNT - 1)) * dp
        val waveRight      = contentRight
        val waveLeft       = waveRight - waveZoneW           // right-anchor waveform
        val slidehintRight = waveLeft - innerGap             // slidehint zone right boundary

        // "‹" arrow (14sp, amber, animated translate-left + alpha)
        holdArrPaint.textSize = 14f * spd
        holdArrPaint.alpha    = (arrowAlpha * 255f).toInt()
        val arrMetrics = holdArrPaint.fontMetrics
        val arrY = contentCy - (arrMetrics.ascent + arrMetrics.descent) / 2f
        val arrW = holdArrPaint.measureText("‹")

        canvas.save()
        canvas.clipRect(holdstripLeft, holdDockStripRect.top, slidehintRight, holdDockStripRect.bottom)
        canvas.translate(-slideArrOffset, 0f)
        canvas.drawText("‹", contentLeft, arrY, holdArrPaint)
        canvas.restore()

        // "ziehen zum Abbrechen" (11sp, Dim, clipped to slidehint zone)
        holdCancelTextPaint.textSize = 11f * spd
        val cancelMetrics = holdCancelTextPaint.fontMetrics
        val cancelY = contentCy - (cancelMetrics.ascent + cancelMetrics.descent) / 2f
        val cancelX = contentLeft + arrW + 5f * dp

        canvas.save()
        canvas.clipRect(cancelX, holdDockStripRect.top, slidehintRight, holdDockStripRect.bottom)
        canvas.drawText("ziehen zum Abbrechen", cancelX, cancelY, holdCancelTextPaint)
        canvas.restore()

        // Waveform (reuses frozen drawClusterWaveform — same helper as normal cluster)
        drawClusterWaveform(canvas, waveLeft, waveRight, contentCy, dp)

        // --- 5. Heldbub soft shadow (pre-allocated BlurMaskFilter — finding 6) ---
        shadowPaint.maskFilter = heldbubBlurFilter
        holdScratchRect.set(holdDockHeldbubRect.left, holdDockHeldbubRect.top + heldbubSide * 0.1f, holdDockHeldbubRect.right, holdDockHeldbubRect.bottom + heldbubSide * 0.1f)
        canvas.drawRoundRect(holdScratchRect, heldbubR, heldbubR, shadowPaint)

        // --- 6. Heldbub teal gradient fill ---
        fillPaint.alpha = 0xFF
        fillPaint.shader = LinearGradient(
            holdDockHeldbubRect.left, holdDockHeldbubRect.top, holdDockHeldbubRect.right, holdDockHeldbubRect.bottom,
            KlarvoTheme.TealHi, KlarvoTheme.TealLo, Shader.TileMode.CLAMP
        )
        canvas.drawRoundRect(holdDockHeldbubRect, heldbubR, heldbubR, fillPaint)
        fillPaint.shader = null

        // --- 7. Heldbub outer AmberLine ring (box-shadow 0 0 0 4dp AmberLine) ---
        val ringOutset = HOLD_RING_OUTSET_DP * dp
        val halfOutset = ringOutset / 2f
        strokePaint.color       = KlarvoTheme.AmberLine
        strokePaint.strokeWidth = ringOutset
        strokePaint.style       = Paint.Style.STROKE
        holdScratchRect.set(holdDockHeldbubRect.left - halfOutset, holdDockHeldbubRect.top - halfOutset, holdDockHeldbubRect.right + halfOutset, holdDockHeldbubRect.bottom + halfOutset)
        canvas.drawRoundRect(holdScratchRect, heldbubR + halfOutset, heldbubR + halfOutset, strokePaint)

        // --- 8. Heldbub inner ring (.ring: Amber 2dp, r18dp, inset -8dp, alpha 0.5) ---
        val innerInset = HOLD_INNER_RING_INSET_DP * dp
        val innerRingR = HOLD_INNER_RING_R_DP * dp
        strokePaint.color       = KlarvoTheme.Amber
        strokePaint.alpha       = (255 * 0.5f).toInt()
        strokePaint.strokeWidth = 2f * dp
        holdScratchRect.set(holdDockHeldbubRect.left - innerInset, holdDockHeldbubRect.top - innerInset, holdDockHeldbubRect.right + innerInset, holdDockHeldbubRect.bottom + innerInset)
        canvas.drawRoundRect(holdScratchRect, innerRingR, innerRingR, strokePaint)
        strokePaint.alpha = 0xFF  // reset

        // --- 9. Heldbub "K" (OnTeal, centered, same formula as IDLE) ---
        kLetterPaint.textSize = heldbubSide / 2f * 0.85f  // ≈ 17sp: same as idle for bubbleSizeDp=40
        val kMetrics = kLetterPaint.fontMetrics
        canvas.drawText("K", heldbubCx, rowCy - (kMetrics.ascent + kMetrics.descent) / 2f, kLetterPaint)

        // --- 10. Heldbub finger indicator (26dp circle, CSS: right:-6px; bottom:-7px) ---
        val fingerRadius = HOLD_FINGER_DP * dp / 2f
        // CSS right:-6px means child's right extends 6dp beyond holdDockHeldbubRect.right
        // CSS bottom:-7px means child's bottom extends 7dp beyond holdDockHeldbubRect.bottom
        val fingerCx = holdDockHeldbubRect.right + 6f * dp - fingerRadius
        val fingerCy = holdDockHeldbubRect.bottom + 7f * dp - fingerRadius
        fillPaint.color  = fingerFill
        fillPaint.shader = null
        canvas.drawCircle(fingerCx, fingerCy, fingerRadius, fillPaint)
        fillPaint.alpha = 0xFF
        // strokePaint.color already encodes alpha — no redundant alpha assignment (finding 6).
        strokePaint.color       = fingerBorder
        strokePaint.strokeWidth = 1.5f * dp
        strokePaint.style       = Paint.Style.STROKE
        canvas.drawCircle(fingerCx, fingerCy, fingerRadius - 0.75f * dp, strokePaint)
        strokePaint.alpha = 0xFF  // reset
        fillPaint.color  = 0xFF000000.toInt()  // reset to avoid leaking alpha

        // --- 11. Lockchip (above holdstrip, centered horizontally over heldbub) ---
        // Zone: [0, lockchipZoneH] from window top. Content positioned near the bottom of zone.
        // Layout (column, top→bottom): ▲ (animated) → [lock icon + "hoch = sperren"]
        val lockchipCx = heldbubCx
        val zoneBottom = lockchipZoneH  // 28dp from top (includes ~4dp gap above holdstrip shadow)

        // Row 2 (bottom): lock icon + "hoch = sperren" (10sp, Muted, monospace)
        holdLockTextPaint.textSize = 10f * spd
        val lockMetrics  = holdLockTextPaint.fontMetrics
        val row2Ascent   = -lockMetrics.ascent
        val row2Descent  = lockMetrics.descent
        val row2Baseline = zoneBottom - row2Descent   // text baseline at bottom of zone
        val lockTextStr  = "hoch = sperren"
        val lockTextW    = holdLockTextPaint.measureText(lockTextStr)
        val lockIconSizePx = 13f * dp
        val lockIconGapPx  = 2f * dp
        val row2TotalW   = lockIconSizePx + lockIconGapPx + lockTextW
        val row2StartX   = lockchipCx - row2TotalW / 2f

        // Lock icon (simplified padlock Path, Muted color, ~13dp)
        val lockIconCx = row2StartX + lockIconSizePx / 2f
        val lockIconCy = row2Baseline - row2Ascent / 2f
        drawLockIcon(canvas, lockIconCx, lockIconCy, 13f, holdLockIconPaint)
        canvas.drawText(lockTextStr, row2StartX + lockIconSizePx + lockIconGapPx, row2Baseline, holdLockTextPaint)

        // Row 1 (top): animated "▲" (13sp, Amber), positioned above row 2 with 2dp gap
        holdUpPaint.textSize = 13f * spd
        holdUpPaint.alpha    = (arrowAlpha * 255f).toInt()
        val upMetrics    = holdUpPaint.fontMetrics
        val row2Height   = row2Ascent + row2Descent
        val row1Baseline = row2Baseline - row2Height - 2f * dp - upMetrics.descent

        canvas.save()
        canvas.translate(0f, -lockArrOffset)
        canvas.drawText("▲", lockchipCx, row1Baseline, holdUpPaint)
        canvas.restore()
    }

    /**
     * Draws a simplified padlock icon centered at (cx, cy) for the HOLD dock lockchip.
     * Size: approximately [sizeDp × sizeDp] dp. Uses Canvas Path (no emoji — device-independent).
     * Body: rounded rect (lower 55% of height). Shackle: arc (upper 45%).
     */
    /**
     * Draws a simplified padlock icon centered at (cx, cy) for the HOLD dock lockchip.
     * Uses pre-allocated [lockBodyRect], [lockShackleRect], [lockBodyFillPaint], [lockShacklePaint]
     * — no per-call Paint/RectF allocations (finding 6).
     */
    private fun drawLockIcon(canvas: Canvas, cx: Float, cy: Float, sizeDp: Float, paint: Paint) {
        val dp    = resources.displayMetrics.density
        val w     = sizeDp * dp
        val bodyH = w * 0.55f
        val bodyTop = cy + w * 0.0f   // body starts at center
        lockBodyRect.set(cx - w / 2f, bodyTop - bodyH / 2f, cx + w / 2f, bodyTop + bodyH / 2f)
        val cornerR = w * 0.18f

        // Body: rounded rect (fill) — reuse pre-allocated paint; copy color from param.
        lockBodyFillPaint.color = paint.color
        canvas.drawRoundRect(lockBodyRect, cornerR, cornerR, lockBodyFillPaint)

        // Shackle: semi-circular arc above body — reuse pre-allocated paint.
        val shackleW  = w * 0.55f
        val shackleCy = lockBodyRect.top
        lockShackleRect.set(cx - shackleW / 2f, shackleCy - w * 0.45f, cx + shackleW / 2f, shackleCy + shackleW * 0.4f)
        lockShacklePaint.color       = paint.color
        lockShacklePaint.strokeWidth = w * 0.18f
        canvas.drawArc(lockShackleRect, 180f, 180f, false, lockShacklePaint)
    }

    // Legacy helpers kept for API compatibility with any remaining callers.

    private fun drawWaveformBarsInZone(
        canvas: Canvas, zoneLeft: Float, zoneRight: Float, cx: Float, cy: Float, maxBarHalfHeight: Float
    ) {
        drawClusterWaveform(canvas, zoneLeft, zoneRight, cy, resources.displayMetrics.density)
    }
}
