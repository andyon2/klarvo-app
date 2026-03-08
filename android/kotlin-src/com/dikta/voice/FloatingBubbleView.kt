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
 *   RECORDING  -- red circle + animated waveform bars (3 bars)
 *   PROCESSING -- amber circle + rotating arc spinner
 *
 * Size:
 *   Call setBubbleSize(dp) to resize the bubble at runtime.
 *   The canvas-relative drawing code uses width/height, so everything scales automatically.
 */
class FloatingBubbleView(context: Context) : View(context) {

    enum class State { IDLE, RECORDING, PROCESSING }

    var state: State = State.IDLE
        set(value) {
            if (field == value) return
            field = value
            updateAnimators()
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

    // --- Colours ---
    private val colorIdleBackground = Color.parseColor("#F5F5F5")  // light grey/white
    private val colorRecording = Color.parseColor("#EF4444")       // red
    private val colorProcessing = Color.parseColor("#F59E0B")      // amber

    // --- App icon ---
    // Loaded once; null if the resource doesn't resolve (fallback to mic path)
    private val appIconDrawable: Drawable? =
        ContextCompat.getDrawable(context, R.mipmap.ic_launcher)

    // --- Paint objects ---
    private val circlePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }
    private val shadowPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = Color.parseColor("#33000000")  // subtle shadow
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

    // --- Bar animation ---
    private val barAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
        duration = 600
        repeatMode = ValueAnimator.REVERSE
        repeatCount = ValueAnimator.INFINITE
        interpolator = LinearInterpolator()
        addUpdateListener { invalidate() }
    }
    // Phase offsets so the bars pulse independently
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
            State.RECORDING -> barAnimator.start()
            State.PROCESSING -> rotationAnimator.start()
            State.IDLE -> { /* no animation */ }
        }
    }

    /**
     * Changes the bubble size at runtime.
     * Caller (DiktaOverlayService) is responsible for updating WindowManager LayoutParams
     * and calling windowManager.updateViewLayout() after this.
     */
    fun setBubbleSize(sizeDp: Int) {
        bubbleSizeDp = sizeDp
        // requestLayout triggers a new onMeasure pass, which returns the new size.
        requestLayout()
        invalidate()
    }

    fun getBubbleSizeDp(): Int = bubbleSizeDp

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val size = (bubbleSizeDp * resources.displayMetrics.density).toInt()
        setMeasuredDimension(size, size)
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val w = width.toFloat()
        val h = height.toFloat()
        val cx = w / 2f
        val cy = h / 2f
        val radius = minOf(cx, cy)

        when (state) {
            State.IDLE -> {
                // Subtle shadow circle slightly offset for elevation effect
                canvas.drawCircle(cx, cy + radius * 0.06f, radius * 0.92f, shadowPaint)
                // White/light background circle
                circlePaint.color = colorIdleBackground
                canvas.drawCircle(cx, cy, radius, circlePaint)
                drawIdleIcon(canvas, cx, cy, radius)
            }
            State.RECORDING -> {
                circlePaint.color = colorRecording
                canvas.drawCircle(cx, cy, radius, circlePaint)
                drawWaveformBars(canvas, cx, cy, radius)
            }
            State.PROCESSING -> {
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
            // Icon fills ~70% of the circle diameter, centered
            val iconRadius = (radius * 0.70f).toInt()
            val left = (cx - iconRadius).toInt()
            val top = (cy - iconRadius).toInt()
            val right = (cx + iconRadius).toInt()
            val bottom = (cy + iconRadius).toInt()
            icon.setBounds(left, top, right, bottom)
            icon.draw(canvas)
        } else {
            // Fallback: draw a simple mic path if the icon resource is unavailable
            drawMicIconFallback(canvas, cx, cy, radius)
        }
    }

    // Fallback mic icon used only when the launcher icon resource fails to load
    private fun drawMicIconFallback(canvas: Canvas, cx: Float, cy: Float, radius: Float) {
        val scale = radius * 0.5f

        val micW = scale * 0.55f
        val micLeft = cx - micW / 2f
        val micTop = cy - scale * 0.65f
        val micRight = cx + micW / 2f
        val micBottom = cy + scale * 0.05f

        val bodyPath = Path().apply {
            val cornerR = micW / 2f
            moveTo(micLeft + cornerR, micTop)
            arcTo(
                RectF(micLeft, micTop, micRight, micTop + cornerR * 2),
                180f, -180f
            )
            lineTo(micRight, micBottom)
            lineTo(micLeft, micBottom)
            close()
        }
        // Use a dark colour so it's visible on the light background
        val darkPaint = Paint(whitePaint).apply { color = Color.parseColor("#555555") }
        canvas.drawPath(bodyPath, darkPaint)

        val arcStrokeW = radius * 0.08f
        val fallbackArcPaint = Paint(arcPaint).apply {
            color = Color.parseColor("#555555")
            strokeWidth = arcStrokeW
        }
        val arcRadius = scale * 0.7f
        val arcTop = cy - arcRadius * 0.1f
        val arcRect = RectF(cx - arcRadius, arcTop, cx + arcRadius, arcTop + arcRadius * 1.5f)
        canvas.drawArc(arcRect, 0f, 180f, false, fallbackArcPaint)

        val lineBottom = cy + scale * 0.65f
        canvas.drawLine(cx, arcTop + arcRadius * 0.75f, cx, lineBottom, fallbackArcPaint)
        val baseW = scale * 0.5f
        canvas.drawLine(cx - baseW / 2f, lineBottom, cx + baseW / 2f, lineBottom, fallbackArcPaint)
    }

    // --- RECORDING: three animated bars ---

    private fun drawWaveformBars(canvas: Canvas, cx: Float, cy: Float, radius: Float) {
        val dp = resources.displayMetrics.density
        val barW = 4f * dp
        val barGap = 2f * dp
        val maxBarH = radius * 1.1f
        val minBarH = radius * 0.25f

        val totalW = barW * 3 + barGap * 2
        val startX = cx - totalW / 2f + barW / 2f

        val t = (barAnimator.animatedValue as? Float) ?: 0f

        for (i in 0..2) {
            // Each bar oscillates at a slightly different phase
            val phase = (t + barPhaseOffsets[i]) % 1f
            // Combine with live amplitude for responsiveness
            val dynamicFactor = 0.5f + amplitude * 0.5f
            val barH = (minBarH + (maxBarH - minBarH) * phase * dynamicFactor)
                .coerceIn(minBarH, maxBarH)

            val barX = startX + i * (barW + barGap)
            val top = cy - barH / 2f
            val bottom = cy + barH / 2f
            val cornerR = barW / 2f

            val barRect = RectF(barX - barW / 2f, top, barX + barW / 2f, bottom)
            canvas.drawRoundRect(barRect, cornerR, cornerR, whitePaint)
        }
    }

    // --- PROCESSING: rotating arc spinner ---

    private fun drawSpinner(canvas: Canvas, cx: Float, cy: Float, radius: Float) {
        val spinRadius = radius * 0.55f
        val strokeW = radius * 0.12f
        arcPaint.strokeWidth = strokeW
        arcPaint.style = Paint.Style.STROKE

        val startAngle = (rotationAnimator.animatedValue as? Float) ?: 0f
        val sweepAngle = 270f

        val rect = RectF(cx - spinRadius, cy - spinRadius, cx + spinRadius, cy + spinRadius)
        canvas.drawArc(rect, startAngle, sweepAngle, false, arcPaint)
    }
}
