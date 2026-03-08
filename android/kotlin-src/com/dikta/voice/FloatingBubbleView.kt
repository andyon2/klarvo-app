package com.dikta.voice

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.*
import android.graphics.drawable.Drawable
import android.view.View
import android.view.animation.LinearInterpolator
import androidx.core.content.ContextCompat

/**
 * Custom View that draws the floating voice-input bubble.
 * All rendering via Canvas -- no asset files needed.
 *
 * States:
 *   IDLE       -- white circle + Dikta app launcher icon
 *   RECORDING  -- pill/bar shape with [X] [waveform] [checkmark]
 *   PROCESSING -- amber circle + rotating arc spinner
 *
 * Size:
 *   Call setBubbleSize(dp) to resize the bubble at runtime.
 *   In RECORDING state the view widens to BAR_WIDTH_DP; height stays at bubbleSizeDp.
 *
 * Touch zones in RECORDING bar:
 *   - Left ~25% of width  -> cancel zone  (X button)
 *   - Right ~25% of width -> confirm zone (checkmark button)
 *   - Middle               -> waveform (no action)
 *   DiktaOverlayService reads isTouchInCancelZone() / isTouchInConfirmZone() to route taps.
 */
class FloatingBubbleView(context: Context) : View(context) {

    enum class State { IDLE, RECORDING, PROCESSING }

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

    // --- Colours ---
    private val colorIdleBackground = Color.parseColor("#F5F5F5")  // light grey/white
    private val colorRecordingBar   = Color.parseColor("#EF4444")  // red
    private val colorCancelBtn      = Color.parseColor("#CC2222")  // darker red for X circle
    private val colorConfirmBtn     = Color.parseColor("#22C55E")  // green for checkmark circle
    private val colorProcessing     = Color.parseColor("#F59E0B")  // amber

    // --- App icon ---
    private val appIconDrawable: Drawable? =
        ContextCompat.getDrawable(context, R.mipmap.ic_launcher)

    // --- Paint objects ---
    private val circlePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }
    private val shadowPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = Color.parseColor("#33000000")
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

    // --- Bar animation ---
    private val barAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
        duration = 600
        repeatMode = ValueAnimator.REVERSE
        repeatCount = ValueAnimator.INFINITE
        interpolator = LinearInterpolator()
        addUpdateListener { invalidate() }
    }
    private val barPhaseOffsets = floatArrayOf(0f, 0.33f, 0.66f)

    // --- Rotation animation ---
    private val rotationAnimator = ValueAnimator.ofFloat(0f, 360f).apply {
        duration = 900
        repeatCount = ValueAnimator.INFINITE
        interpolator = LinearInterpolator()
        addUpdateListener { invalidate() }
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        updateAnimators()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        barAnimator.cancel()
        rotationAnimator.cancel()
    }

    private fun updateAnimators() {
        barAnimator.cancel()
        rotationAnimator.cancel()
        when (state) {
            State.RECORDING  -> barAnimator.start()
            State.PROCESSING -> rotationAnimator.start()
            State.IDLE       -> { /* no animation */ }
        }
    }

    /**
     * Changes the bubble size at runtime.
     * Caller (DiktaOverlayService) is responsible for updating WindowManager LayoutParams
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
            State.RECORDING -> (BAR_WIDTH_DP * density).toInt()
            else            -> heightPx  // square == circle
        }
        setMeasuredDimension(widthPx, heightPx)
    }

    // --- Touch zone helpers (used by DiktaOverlayService) ---

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
                val cx = w / 2f
                val cy = h / 2f
                val radius = minOf(cx, cy)
                canvas.drawCircle(cx, cy + radius * 0.06f, radius * 0.92f, shadowPaint)
                circlePaint.color = colorIdleBackground
                canvas.drawCircle(cx, cy, radius, circlePaint)
                drawIdleIcon(canvas, cx, cy, radius)
            }
            State.RECORDING -> {
                drawRecordingBar(canvas, w, h)
            }
            State.PROCESSING -> {
                val cx = w / 2f
                val cy = h / 2f
                val radius = minOf(cx, cy)
                circlePaint.color = colorProcessing
                canvas.drawCircle(cx, cy, radius, circlePaint)
                drawSpinner(canvas, cx, cy, radius)
            }
        }
    }

    // --- IDLE: Dikta app launcher icon, centered in the bubble ---

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

        // Background pill (red)
        circlePaint.color = colorRecordingBar
        canvas.drawRoundRect(RectF(0f, 0f, w, h), radius, radius, circlePaint)

        val btnRadius = h * BTN_RADIUS_FRACTION

        // --- Cancel button (X, left) ---
        val cancelCx = h / 2f   // center is half a bubble-height from left
        val cancelCy = h / 2f
        btnPaint.color = colorCancelBtn
        canvas.drawCircle(cancelCx, cancelCy, btnRadius, btnPaint)
        drawXMark(canvas, cancelCx, cancelCy, btnRadius * 0.5f)

        // --- Confirm button (checkmark, right) ---
        val confirmCx = w - h / 2f
        val confirmCy = h / 2f
        btnPaint.color = colorConfirmBtn
        canvas.drawCircle(confirmCx, confirmCy, btnRadius, btnPaint)
        drawCheckMark(canvas, confirmCx, confirmCy, btnRadius * 0.55f)

        // --- Waveform in the middle zone ---
        val waveLeft  = cancelCx + btnRadius + h * 0.05f
        val waveRight = confirmCx - btnRadius - h * 0.05f
        val waveMidX  = (waveLeft + waveRight) / 2f
        drawWaveformBarsInZone(canvas, waveLeft, waveRight, waveMidX, h / 2f, h / 2f)
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
        val dp   = resources.displayMetrics.density
        val barW = 4f * dp
        val barGap = 3f * dp

        val totalW  = barW * 3 + barGap * 2
        val startX  = cx - totalW / 2f + barW / 2f

        val maxBarH = maxBarHalfHeight * 1.0f
        val minBarH = maxBarHalfHeight * 0.25f

        val t = (barAnimator.animatedValue as? Float) ?: 0f

        for (i in 0..2) {
            val phase         = (t + barPhaseOffsets[i]) % 1f
            val dynamicFactor = 0.5f + amplitude * 0.5f
            val barH          = (minBarH + (maxBarH - minBarH) * phase * dynamicFactor)
                .coerceIn(minBarH, maxBarH)

            val barX = startX + i * (barW + barGap)

            // Clamp bars inside waveform zone
            if (barX < zoneLeft || barX > zoneRight) continue

            val top      = cy - barH / 2f
            val bottom   = cy + barH / 2f
            val cornerR  = barW / 2f
            val barRect  = RectF(barX - barW / 2f, top, barX + barW / 2f, bottom)
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
