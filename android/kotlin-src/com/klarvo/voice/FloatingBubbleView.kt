package com.klarvo.voice

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.*
import android.view.View
import android.view.animation.LinearInterpolator
import android.view.animation.OvershootInterpolator

/**
 * Custom View that draws the floating voice-input bubble (and the RECORDING cluster).
 * All rendering via Canvas — no asset files needed.
 *
 * States:
 *   IDLE          -- teal-gradient squircle + dark "K" (OnTeal) + faint teal glass ring
 *                    Canon: .ab-bubble.idle
 *   RECORDING     -- control cluster at the dock spot (Modell B / ADR-0019 §4′ #2):
 *                    [✗ cancel red] [amber waveform] [➤ send teal] on a dark semi-transparent
 *                    backdrop with a static amber ring. Window grows from single bubble → cluster.
 *   TRANSCRIBING  -- single teal proc bubble (.ab-bubble.proc): teal squircle + rotating spinner.
 *                    Window collapses back to single-bubble size.
 *   DONE          -- success-green gradient squircle + dark check polyline (.ab-bubble.done),
 *                    then returns to IDLE (via doneFlashRunnable).
 *
 * Touch zones in RECORDING (cluster layout, left→right: cancel | waveform | send):
 *   - Left zone   -> ✗ Cancel (isTouchInCancelZone)
 *   - Dead zone   -> waveform (no action)
 *   - Right zone  -> ➤ Send  (isTouchInConfirmZone)
 *   KlarvoOverlayService reads these helpers and routes accordingly; tap on waveform/backdrop = no-op.
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

    /** True when [touchX] hits the ➤ Send button zone (right side of the cluster). */
    fun isTouchInConfirmZone(touchX: Float): Boolean =
        state == State.RECORDING && clusterSendZoneStart > 0f && touchX >= clusterSendZoneStart

    /** True when [touchX] hits the ✗ Cancel button zone (left side of the cluster). */
    fun isTouchInCancelZone(touchX: Float): Boolean =
        state == State.RECORDING && clusterCancelZoneEnd > 0f && touchX <= clusterCancelZoneEnd

    // --- onDraw ---

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        when (state) {
            State.IDLE       -> drawIdleBubble(canvas)
            State.RECORDING  -> drawRecordingCluster(canvas)
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

    // Legacy helpers kept for API compatibility with any remaining callers.

    private fun drawWaveformBarsInZone(
        canvas: Canvas, zoneLeft: Float, zoneRight: Float, cx: Float, cy: Float, maxBarHalfHeight: Float
    ) {
        drawClusterWaveform(canvas, zoneLeft, zoneRight, cy, resources.displayMetrics.density)
    }
}
