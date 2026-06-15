package com.klarvo.voice

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.*
import android.graphics.drawable.Drawable
import android.view.View
import android.view.animation.LinearInterpolator
import android.view.animation.OvershootInterpolator
import androidx.core.content.ContextCompat

/**
 * Custom View that draws the floating voice-input bubble.
 * All rendering via Canvas -- no asset files needed.
 *
 * States:
 *   IDLE          -- teal "K" letter + glass ring (4dp Teal stroke on dark Surface fill)
 *   RECORDING     -- pill/bar shape with [X] [waveform] [checkmark]
 *                    Used for tap-to-record where the user needs cancel/confirm buttons.
 *   RECORDING_PTT -- circular bubble, scaled up + Danger color, waveform inside.
 *                    Used for push-to-talk: user just holds and releases, no bar needed.
 *   PROCESSING    -- Teal circle + rotating arc spinner
 *
 * Size:
 *   Call setBubbleSize(dp) to resize the bubble at runtime (controls the visual radius).
 *   The WindowManager LayoutParams may be larger than bubbleSizeDp (touch-target expansion
 *   to ≥48dp); the visual circle is drawn centered within the larger touch-target bounds.
 *   In RECORDING state the view widens to BAR_WIDTH_DP; height stays at bubbleSizeDp.
 *   In RECORDING_PTT state the view stays circular but animates scale via scaleX/scaleY.
 *
 * Touch zones in RECORDING bar:
 *   - Left ~25% of width  -> cancel zone  (X button)
 *   - Right ~25% of width -> confirm zone (checkmark button)
 *   - Middle               -> waveform (no action)
 *   KlarvoOverlayService reads isTouchInCancelZone() / isTouchInConfirmZone() to route taps.
 *
 * Color semantics (DT5 — binding rule):
 *   KlarvoTheme.Teal   = brand / processing / focus-ring / idle "K"
 *   KlarvoTheme.Danger = stop / recording / cancel button
 *   No amber in IDLE — amber = recording tally (Story 9.5+ scope).
 */
class FloatingBubbleView(context: Context) : View(context) {

    enum class State { IDLE, RECORDING, RECORDING_PTT, PROCESSING }

    var state: State = State.IDLE
        set(value) {
            if (field == value) return
            field = value
            updateAnimators()
            requestLayout()   // width changes between circle and bar
            invalidate()
        }

    /** Amplitude 0..1 for waveform bar height during RECORDING */
    var amplitude: Float = 0f
        set(value) {
            field = value.coerceIn(0f, 1f)
            invalidate()
        }

    /** Current bubble size in dp. Changed via setBubbleSize(). */
    private var bubbleSizeDp: Int = 56

    companion object {
        /** Width of the recording bar in dp. */
        const val BAR_WIDTH_DP = 220

        /** Button circle radius as fraction of bubble height. */
        private const val BTN_RADIUS_FRACTION = 0.35f
    }

    // --- App icon (kept for 9.4+ states; not used in IDLE since 9.3 re-skin) ---
    private val appIconDrawable: Drawable? =
        ContextCompat.getDrawable(context, R.mipmap.ic_launcher)

    // --- Paint objects ---
    private val circlePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }
    private val shadowPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.ShadowColor
    }
    private val whitePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        style = Paint.Style.FILL
    }
    private val arcPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        style = Paint.Style.STROKE
        strokeCap = Paint.Cap.ROUND
    }
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        style = Paint.Style.FILL
        textAlign = Paint.Align.CENTER
    }
    private val btnPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }

    // --- Idle re-skin paints (Story 9.3 — teal "K" + glass ring) ---
    // idleCirclePaint: dark Surface fill (simulates glass — no RenderEffect blur, AR5/DT4)
    private val idleCirclePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Surface
    }
    // idleRingPaint: 4dp Teal stroke ring; strokeWidth is set in onDraw after density is known
    private val idleRingPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        color = KlarvoTheme.Teal
        strokeCap = Paint.Cap.BUTT
    }
    // kLetterPaint: bold "K" in Teal; textSize and center alignment set in onDraw
    private val kLetterPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.Teal
        textAlign = Paint.Align.CENTER
        typeface = Typeface.DEFAULT_BOLD
    }

    // --- Bar animation (drives waveform bars in both RECORDING and RECORDING_PTT) ---
    private val barAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
        duration = 600
        repeatMode = ValueAnimator.REVERSE
        repeatCount = ValueAnimator.INFINITE
        interpolator = LinearInterpolator()
        addUpdateListener { invalidate() }
    }
    // 5 bars: evenly distributed phase offsets across the 0..1 cycle
    private val barPhaseOffsets = floatArrayOf(0f, 0.20f, 0.40f, 0.60f, 0.80f)

    // --- Rotation animation ---
    private val rotationAnimator = ValueAnimator.ofFloat(0f, 360f).apply {
        duration = 900
        repeatCount = ValueAnimator.INFINITE
        interpolator = LinearInterpolator()
        addUpdateListener { invalidate() }
    }

    /**
     * PTT scale-up animator: smoothly grows the bubble from 1.0 to 1.3 with a slight
     * overshoot for a tactile "pop" feel when push-to-talk activates.
     */
    private val pttScaleUpAnimator = ValueAnimator.ofFloat(1.0f, 1.3f).apply {
        duration = 200
        interpolator = OvershootInterpolator(2.0f)
        addUpdateListener { anim ->
            val s = anim.animatedValue as Float
            scaleX = s
            scaleY = s
        }
    }

    /**
     * PTT scale-down animator: shrinks back to 1.0 when push-to-talk is released.
     * No overshoot -- just a clean snap back.
     */
    private val pttScaleDownAnimator = ValueAnimator.ofFloat(1.3f, 1.0f).apply {
        duration = 150
        interpolator = LinearInterpolator()
        addUpdateListener { anim ->
            val s = anim.animatedValue as Float
            scaleX = s
            scaleY = s
        }
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        updateAnimators()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        barAnimator.cancel()
        rotationAnimator.cancel()
        pttScaleUpAnimator.cancel()
        pttScaleDownAnimator.cancel()
    }

    private fun updateAnimators() {
        barAnimator.cancel()
        rotationAnimator.cancel()
        when (state) {
            State.RECORDING     -> barAnimator.start()
            State.RECORDING_PTT -> {
                barAnimator.start()
                // Cancel any in-progress scale-down, then animate scale-up
                pttScaleDownAnimator.cancel()
                // Read current scale so the animator starts from wherever we are
                pttScaleUpAnimator.setFloatValues(scaleX, 1.3f)
                pttScaleUpAnimator.start()
            }
            State.PROCESSING -> {
                // If we just left PTT, scale back down
                if (scaleX != 1.0f) {
                    pttScaleUpAnimator.cancel()
                    pttScaleDownAnimator.setFloatValues(scaleX, 1.0f)
                    pttScaleDownAnimator.start()
                }
                rotationAnimator.start()
            }
            State.IDLE -> {
                // Ensure scale is reset (e.g. cancel was called directly from PTT)
                if (scaleX != 1.0f) {
                    pttScaleUpAnimator.cancel()
                    pttScaleDownAnimator.setFloatValues(scaleX, 1.0f)
                    pttScaleDownAnimator.start()
                }
            }
        }
    }

    /**
     * Changes the bubble size at runtime.
     * Caller (KlarvoOverlayService) is responsible for updating WindowManager LayoutParams
     * and calling windowManager.updateViewLayout() after this.
     */
    fun setBubbleSize(sizeDp: Int) {
        bubbleSizeDp = sizeDp
        requestLayout()
        invalidate()
    }

    fun getBubbleSizeDp(): Int = bubbleSizeDp

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val density = resources.displayMetrics.density
        val heightPx = (bubbleSizeDp * density).toInt()
        val widthPx = when (state) {
            // Bar mode only for tap-to-record; PTT stays circular
            State.RECORDING -> (BAR_WIDTH_DP * density).toInt()
            else            -> heightPx  // square == circle
        }
        setMeasuredDimension(widthPx, heightPx)
    }

    // --- Touch zone helpers (used by KlarvoOverlayService) ---

    /**
     * Returns true if [touchX] (relative to this view's left edge) falls inside
     * the cancel button zone on the left side of the recording bar.
     * Only meaningful in RECORDING state.
     */
    fun isTouchInCancelZone(touchX: Float): Boolean {
        if (state != State.RECORDING) return false
        val density = resources.displayMetrics.density
        val barW = BAR_WIDTH_DP * density
        return touchX < barW * 0.30f
    }

    /**
     * Returns true if [touchX] falls inside the confirm button zone on the right.
     * Only meaningful in RECORDING state.
     */
    fun isTouchInConfirmZone(touchX: Float): Boolean {
        if (state != State.RECORDING) return false
        val density = resources.displayMetrics.density
        val barW = BAR_WIDTH_DP * density
        return touchX > barW * 0.70f
    }

    // --- Draw ---

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val w = width.toFloat()
        val h = height.toFloat()

        when (state) {
            State.IDLE -> {
                // Touch-target expansion: the view may be larger than the visual bubble.
                // bubbleSizeDp controls the visual radius; the view is sized to the touch target.
                // Draw centered within the touch-target bounds.
                val density = resources.displayMetrics.density
                val visualRadius = (bubbleSizeDp * density) / 2f
                val cx = w / 2f
                val cy = h / 2f

                // Shadow (slightly offset downward for depth)
                canvas.drawCircle(cx, cy + visualRadius * 0.06f, visualRadius * 0.92f, shadowPaint)

                // Dark Surface fill (glass effect without blur — AR5/DT4)
                canvas.drawCircle(cx, cy, visualRadius, idleCirclePaint)

                // 4dp Teal ring, inset by half stroke width so ring is fully inside circle bounds
                val ringStrokePx = 4f * density
                idleRingPaint.strokeWidth = ringStrokePx
                val ringStrokeHalf = ringStrokePx / 2f
                canvas.drawCircle(cx, cy, visualRadius - ringStrokeHalf, idleRingPaint)

                // Teal "K" centered in the bubble; +0.22f*radius baseline correction for capital letter
                kLetterPaint.textSize = visualRadius * 0.65f
                canvas.drawText("K", cx, cy + visualRadius * 0.22f, kLetterPaint)
            }
            State.RECORDING -> {
                drawRecordingBar(canvas, w, h)
            }
            State.RECORDING_PTT -> {
                // Circular bubble -- stays same size as IDLE, scale animation via scaleX/scaleY.
                // Drawn Danger (red/stop) with waveform bars centered inside.
                // Use the same visual radius derivation as IDLE so the circle diameter never
                // grows to fill the touch-target window when leaving IDLE.
                val cx = w / 2f
                val cy = h / 2f
                val visualRadius = (bubbleSizeDp * resources.displayMetrics.density) / 2f
                // Shadow (slightly offset downward for depth)
                canvas.drawCircle(cx, cy + visualRadius * 0.06f, visualRadius * 0.92f, shadowPaint)
                // Danger filled circle (stop/recording — DT5)
                circlePaint.color = KlarvoTheme.Danger
                canvas.drawCircle(cx, cy, visualRadius, circlePaint)
                // Waveform bars inside the circle.
                // Half-height limit: 70% of visual radius so bars are tall and clearly visible.
                // The bar draws ±halfHeight from center, so this is nearly full diameter.
                val waveHalfH = visualRadius * 0.70f
                // Left/right bounds: 75% of visual radius each side for a wide waveform footprint.
                val waveLeft  = cx - visualRadius * 0.75f
                val waveRight = cx + visualRadius * 0.75f
                drawWaveformBarsInZone(canvas, waveLeft, waveRight, cx, cy, waveHalfH)
            }
            State.PROCESSING -> {
                val cx = w / 2f
                val cy = h / 2f
                // Use the same visual radius derivation as IDLE so the circle diameter never
                // grows to fill the touch-target window when leaving IDLE.
                val visualRadius = (bubbleSizeDp * resources.displayMetrics.density) / 2f
                // Teal fill (brand/processing — DT5; NOT amber)
                circlePaint.color = KlarvoTheme.Teal
                canvas.drawCircle(cx, cy, visualRadius, circlePaint)
                drawSpinner(canvas, cx, cy, visualRadius)
            }
        }
    }

    // --- IDLE: Klarvo app launcher icon, centered in the bubble ---

    private fun drawIdleIcon(canvas: Canvas, cx: Float, cy: Float, radius: Float) {
        val icon = appIconDrawable
        if (icon != null) {
            val iconRadius = (radius * 0.70f).toInt()
            val left   = (cx - iconRadius).toInt()
            val top    = (cy - iconRadius).toInt()
            val right  = (cx + iconRadius).toInt()
            val bottom = (cy + iconRadius).toInt()
            icon.setBounds(left, top, right, bottom)
            icon.draw(canvas)
        } else {
            drawMicIconFallback(canvas, cx, cy, radius)
        }
    }

    private fun drawMicIconFallback(canvas: Canvas, cx: Float, cy: Float, radius: Float) {
        val scale = radius * 0.5f
        val micW   = scale * 0.55f
        val micLeft   = cx - micW / 2f
        val micTop    = cy - scale * 0.65f
        val micRight  = cx + micW / 2f
        val micBottom = cy + scale * 0.05f

        val bodyPath = Path().apply {
            val cornerR = micW / 2f
            moveTo(micLeft + cornerR, micTop)
            arcTo(RectF(micLeft, micTop, micRight, micTop + cornerR * 2), 180f, -180f)
            lineTo(micRight, micBottom)
            lineTo(micLeft, micBottom)
            close()
        }
        val darkPaint = Paint(whitePaint).apply { color = Color.parseColor("#555555") }
        canvas.drawPath(bodyPath, darkPaint)

        val arcStrokeW = radius * 0.08f
        val fallbackArcPaint = Paint(arcPaint).apply {
            color = Color.parseColor("#555555")
            strokeWidth = arcStrokeW
        }
        val arcRadius = scale * 0.7f
        val arcTop  = cy - arcRadius * 0.1f
        val arcRect = RectF(cx - arcRadius, arcTop, cx + arcRadius, arcTop + arcRadius * 1.5f)
        canvas.drawArc(arcRect, 0f, 180f, false, fallbackArcPaint)

        val lineBottom = cy + scale * 0.65f
        canvas.drawLine(cx, arcTop + arcRadius * 0.75f, cx, lineBottom, fallbackArcPaint)
        val baseW = scale * 0.5f
        canvas.drawLine(cx - baseW / 2f, lineBottom, cx + baseW / 2f, lineBottom, fallbackArcPaint)
    }

    // --- RECORDING: pill bar with [X] [waveform] [checkmark] ---

    private fun drawRecordingBar(canvas: Canvas, w: Float, h: Float) {
        val radius = h / 2f

        // Background pill (Danger = stop/recording — DT5)
        circlePaint.color = KlarvoTheme.Danger
        canvas.drawRoundRect(RectF(0f, 0f, w, h), radius, radius, circlePaint)

        val btnRadius = h * BTN_RADIUS_FRACTION

        // --- Cancel button (X, left) ---
        val cancelCx = h / 2f   // center is half a bubble-height from left
        val cancelCy = h / 2f
        btnPaint.color = KlarvoTheme.Danger
        canvas.drawCircle(cancelCx, cancelCy, btnRadius, btnPaint)
        drawXMark(canvas, cancelCx, cancelCy, btnRadius * 0.5f)

        // --- Confirm button (checkmark, right) ---
        val confirmCx = w - h / 2f
        val confirmCy = h / 2f
        btnPaint.color = KlarvoTheme.TealHi
        canvas.drawCircle(confirmCx, confirmCy, btnRadius, btnPaint)
        drawCheckMark(canvas, confirmCx, confirmCy, btnRadius * 0.55f)

        // --- Waveform in the middle zone ---
        // Cancel zone: left 0..h (button centered at h/2, radius btnRadius)
        // Confirm zone: right (w-h)..w (button centered at w-h/2, radius btnRadius)
        // Waveform zone: the middle 65-70% between the two button edges.
        // Each button occupies h px (one bubble-diameter), so the middle span is w - 2*h.
        // We shrink it slightly (5% each side) so bars never overlap the button circles.
        val middleSpan = w - 2f * h
        val waveLeft  = h + middleSpan * 0.08f
        val waveRight = w - h - middleSpan * 0.08f
        val waveMidX  = (waveLeft + waveRight) / 2f
        // Use 80% of half-height so bars nearly fill the bar height
        drawWaveformBarsInZone(canvas, waveLeft, waveRight, waveMidX, h / 2f, h / 2f * 0.80f)
    }

    private fun drawXMark(canvas: Canvas, cx: Float, cy: Float, arm: Float) {
        val xPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.WHITE
            style = Paint.Style.STROKE
            strokeWidth = arm * 0.4f
            strokeCap = Paint.Cap.ROUND
        }
        canvas.drawLine(cx - arm, cy - arm, cx + arm, cy + arm, xPaint)
        canvas.drawLine(cx + arm, cy - arm, cx - arm, cy + arm, xPaint)
    }

    private fun drawCheckMark(canvas: Canvas, cx: Float, cy: Float, size: Float) {
        val checkPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            color = Color.WHITE
            style = Paint.Style.STROKE
            strokeWidth = size * 0.35f
            strokeCap = Paint.Cap.ROUND
            strokeJoin = Paint.Join.ROUND
        }
        val path = Path().apply {
            // Start at bottom-left of tick, go down-right then up-right
            moveTo(cx - size * 0.55f, cy)
            lineTo(cx - size * 0.1f,  cy + size * 0.45f)
            lineTo(cx + size * 0.6f,  cy - size * 0.45f)
        }
        canvas.drawPath(path, checkPaint)
    }

    private fun drawWaveformBarsInZone(
        canvas: Canvas,
        zoneLeft: Float,
        zoneRight: Float,
        cx: Float,
        cy: Float,
        maxBarHalfHeight: Float
    ) {
        val dp     = resources.displayMetrics.density
        val barW   = 7f * dp   // 7dp wide bars for strong visual presence
        val barGap = 4f * dp   // 4dp gap keeps bars distinct without wasting space

        val barCount = barPhaseOffsets.size   // 5
        val totalW   = barW * barCount + barGap * (barCount - 1)
        val startX   = cx - totalW / 2f + barW / 2f

        // maxBarHalfHeight is the half-height limit; full bar draws ±maxBarHalfHeight
        val maxBarH = maxBarHalfHeight * 2f  // total bar height (symmetric, top+bottom)
        val minBarH = maxBarH * 0.10f        // nearly flat during silence -- max contrast

        val t = (barAnimator.animatedValue as? Float) ?: 0f

        // Silence gate: below this threshold bars stay static at min height.
        val silenceThreshold = 0.02f
        val isSilent = amplitude < silenceThreshold

        // Power curve: amplitude.pow(0.6) boosts mid-range values so moderate
        // speech looks dramatically more visible than a linear mapping would.
        val dynamicFactor = if (isSilent) 0f else Math.pow(amplitude.toDouble(), 0.6).toFloat()

        for (i in 0 until barCount) {
            val barX = startX + i * (barW + barGap)

            // Skip bars that would overflow the zone (e.g. if zone is narrower than totalW)
            if (barX - barW / 2f < zoneLeft || barX + barW / 2f > zoneRight) continue

            val phase = if (isSilent) 0f else (t + barPhaseOffsets[i]) % 1f
            // Height oscillates with the animation phase, scaled by the boosted amplitude
            val barH = (minBarH + (maxBarH - minBarH) * phase * dynamicFactor)
                .coerceIn(minBarH, maxBarH)

            val top     = cy - barH / 2f
            val bottom  = cy + barH / 2f
            val cornerR = barW / 2f
            val barRect = RectF(barX - barW / 2f, top, barX + barW / 2f, bottom)
            canvas.drawRoundRect(barRect, cornerR, cornerR, whitePaint)
        }
    }

    // --- PROCESSING: rotating arc spinner ---

    private fun drawSpinner(canvas: Canvas, cx: Float, cy: Float, radius: Float) {
        val spinRadius = radius * 0.55f
        val strokeW    = radius * 0.12f
        arcPaint.strokeWidth = strokeW
        arcPaint.style = Paint.Style.STROKE

        val startAngle = (rotationAnimator.animatedValue as? Float) ?: 0f
        val sweepAngle = 270f

        val rect = RectF(cx - spinRadius, cy - spinRadius, cx + spinRadius, cy + spinRadius)
        canvas.drawArc(rect, startAngle, sweepAngle, false, arcPaint)
    }
}
