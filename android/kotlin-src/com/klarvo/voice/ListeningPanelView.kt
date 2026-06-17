package com.klarvo.voice

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.*
import android.os.Handler
import android.os.Looper
import android.text.TextPaint
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView

/**
 * Klarvo listening-panel overlay for the recording / transcribing states.
 *
 * Implemented as a LinearLayout wrapper containing:
 *   - GripView          — 34dp × 4dp centered grab handle
 *   - TopRowView        — Canvas-drawn row: K-badge, live-dot/spinner, waveform/"Cleaning…", timer, stop-btn
 *   - transcriptTextView — standard TextView for multiline raw transcript
 *   - FooterView        — Canvas-drawn footer row with caption text
 *
 * Added to WindowManager as a separate TYPE_APPLICATION_OVERLAY window (AC5).
 * Panel enters with spring animation (240ms OvershootInterpolator(1.8f)) on attach.
 *
 * States:
 *   RECORDING     — amber live-dot + waveform + timer + stop button; top amber border-line
 *   TRANSCRIBING  — teal spinner + "Cleaning…" label; top Border2 line; text dimmed
 */
class ListeningPanelView(context: Context) : LinearLayout(context) {

    enum class State { RECORDING, TRANSCRIBING }

    // --- Public properties ---

    var panelState: State = State.RECORDING
        set(value) {
            if (field == value) return
            field = value
            topRowView.applyAnimatorsForState(value)
            topRowView.invalidate()
            updateTranscriptColor()
            footerView.invalidate()
            if (value == State.RECORDING) caretView.startBlink() else caretView.stopBlink()
        }

    var amplitude: Float = 0f
        set(value) {
            field = value.coerceIn(0f, 1f)
            topRowView.invalidate()
        }

    var rawTranscript: String = ""
        set(value) {
            field = value
            transcriptTextView.text = value
        }

    var recordingElapsedMs: Long = 0L
        set(value) {
            field = value
            topRowView.invalidate()
        }

    /**
     * Hit-tests whether panel-root-local coordinates (event.x, event.y) land on the Abbrechen
     * (red square) button (ADR-0019: red = Abbrechen / cancel, Story 9.5).
     *
     * F2: translates panel-root coordinates into TopRowView-local coordinates before the
     * contains() test — stopBtnRect is computed in TopRowView's OWN coordinate space.
     * TopRowView is offset from the panel root by its left/top margins plus the panel's own
     * left-margin of the LayoutParams (leftMargin=16dp).
     */
    fun isTouchOnStopButton(touchX: Float, touchY: Float): Boolean {
        // topRowView.left and topRowView.top are TopRowView's offset within this LinearLayout
        val localX = touchX - topRowView.left
        val localY = touchY - topRowView.top
        return topRowView.isTouchOnStopButton(localX, localY)
    }

    // --- Timer ---

    private val handler = Handler(Looper.getMainLooper())
    private val timerRunnable = object : Runnable {
        override fun run() {
            recordingElapsedMs += 1000L
            handler.postDelayed(this, 1000L)
        }
    }

    fun startTimer() {
        recordingElapsedMs = 0L
        handler.removeCallbacks(timerRunnable)
        handler.postDelayed(timerRunnable, 1000L)
    }

    fun stopTimer() {
        handler.removeCallbacks(timerRunnable)
    }

    fun updateTranscript(text: String) {
        rawTranscript = text
    }

    /**
     * Collapse animation: slides the panel downward over 320ms (canon --k-t-panel),
     * then calls [onDone] so the service can removeView.
     *
     * If a collapse is already running, cancels it and calls onDone immediately
     * so a new show-request can proceed cleanly (F9).
     */
    private var collapseAnimator: ValueAnimator? = null

    fun hideWithAnimation(onDone: () -> Unit) {
        // Cancel any in-flight collapse and call done immediately (mid-collapse new-show path)
        val running = collapseAnimator
        if (running != null && running.isRunning) {
            running.cancel()
            collapseAnimator = null
            onDone()
            return
        }
        stopTimer()
        topRowView.cancelAnimators()
        val startY = translationY
        val endY   = startY + height.toFloat().coerceAtLeast(1f)
        val anim   = ValueAnimator.ofFloat(startY, endY).apply {
            duration     = 320
            interpolator = LinearInterpolator()
            addUpdateListener { translationY = it.animatedValue as Float }
            addListener(object : android.animation.AnimatorListenerAdapter() {
                override fun onAnimationEnd(a: android.animation.Animator) {
                    collapseAnimator = null
                    onDone()
                }
                override fun onAnimationCancel(a: android.animation.Animator) {
                    collapseAnimator = null
                }
            })
        }
        collapseAnimator = anim
        anim.start()
    }

    // --- Child views ---

    private val topRowView: TopRowView
    private val transcriptTextView: TextView
    private val footerView: FooterView
    private val caretView: CaretView


    init {
        orientation = VERTICAL
        // Fully opaque dark background (panel is not a glass surface — no alpha needed).
        // LAYER_TYPE_SOFTWARE is intentionally absent: panel uses no BlurMaskFilter.
        setBackgroundColor(Color.rgb(0x12, 0x14, 0x16))

        val dp = resources.displayMetrics.density

        // top padding = 9dp
        setPadding(0, (9 * dp).toInt(), 0, (18 * dp).toInt())

        // Grab handle
        val gripView = GripView(context)
        val gripParams = LayoutParams(LayoutParams.MATCH_PARENT, (4 * dp).toInt()).apply {
            bottomMargin = (11 * dp).toInt()
        }
        addView(gripView, gripParams)

        // Top row
        topRowView = TopRowView(context)
        val topRowParams = LayoutParams(LayoutParams.MATCH_PARENT, (26 * dp).toInt()).apply {
            leftMargin = (16 * dp).toInt()
            rightMargin = (16 * dp).toInt()
            bottomMargin = (11 * dp).toInt()
        }
        addView(topRowView, topRowParams)

        // Transcript area: TextView + blinking amber caret in a FrameLayout overlay
        transcriptTextView = TextView(context).apply {
            setTextColor(KlarvoTheme.Muted)
            textSize = 13f  // sp
            typeface = Typeface.MONOSPACE
            setLineSpacing(0f, 1.7f)
            gravity = Gravity.TOP or Gravity.START
            text = ""
        }
        caretView = CaretView(context)

        val transcriptFrame = FrameLayout(context)
        transcriptFrame.addView(transcriptTextView, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT
        ))
        val caretLp = FrameLayout.LayoutParams(
            (2 * dp).toInt(),
            (15 * dp).toInt()
        ).apply { gravity = Gravity.TOP or Gravity.START }
        transcriptFrame.addView(caretView, caretLp)

        val textParams = LayoutParams(LayoutParams.MATCH_PARENT, 0, 1f).apply {
            leftMargin = (16 * dp).toInt()
            rightMargin = (16 * dp).toInt()
        }
        addView(transcriptFrame, textParams)

        // Footer
        footerView = FooterView(context)
        val footerParams = LayoutParams(LayoutParams.MATCH_PARENT, (20 * dp).toInt()).apply {
            leftMargin = (16 * dp).toInt()
            rightMargin = (16 * dp).toInt()
        }
        addView(footerView, footerParams)

        // Outer: draw the top border-line via dispatchDraw override
        setWillNotDraw(false)

        // AC5 / canon .ab-panel { min-height: 200px } — enforce 200dp floor (F8)
        minimumHeight = (200 * dp).toInt()
    }

    override fun onAttachedToWindow() {
        super.onAttachedToWindow()
        if (panelState == State.RECORDING) caretView.startBlink()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        caretView.stopBlink()
        stopTimer()
        topRowView.cancelAnimators()
    }

    // Reusable Paint for onDraw — never allocate Paint inside a draw call (F5a)
    private val borderLinePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        // Draw top border-line (1dp): amber in RECORDING, Border2 in TRANSCRIBING
        val dp = resources.displayMetrics.density
        borderLinePaint.color = if (panelState == State.RECORDING) KlarvoTheme.AmberLine else KlarvoTheme.Border2
        canvas.drawRect(0f, 0f, width.toFloat(), dp, borderLinePaint)
    }

    private fun updateTranscriptColor() {
        transcriptTextView.setTextColor(
            if (panelState == State.RECORDING) KlarvoTheme.Muted else KlarvoTheme.Dim
        )
    }

    // -------------------------------------------------------------------------
    // Inner views
    // -------------------------------------------------------------------------

    /**
     * Grab handle: 34dp × 4dp, Border2 fill, full-pill corners, horizontally centered.
     */
    private inner class GripView(context: Context) : View(context) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            style = Paint.Style.FILL
            color = KlarvoTheme.Border2
        }

        override fun onDraw(canvas: Canvas) {
            val dp = resources.displayMetrics.density
            val gripW = 34 * dp
            val gripH = 4 * dp
            val cx = width / 2f
            val cy = height / 2f
            val rect = RectF(cx - gripW / 2f, cy - gripH / 2f, cx + gripW / 2f, cy + gripH / 2f)
            canvas.drawRoundRect(rect, gripH / 2f, gripH / 2f, paint)
        }
    }

    /**
     * Top row: K-badge | live-dot(RECORDING)/spinner(TRANSCRIBING) | waveform(REC)/"Cleaning…"(TRANS) |
     *          timer(REC) | stop-btn(REC)
     *
     * All drawing in Canvas — explicit coordinate math per Dev Notes.
     */
    inner class TopRowView(context: Context) : View(context) {

        // For Abbrechen-button touch detection (screen coords not needed — panel touch listener uses view.x/y)
        // Red square = Abbrechen (ADR-0019). cancelBtnRect removed — neutral ✗ button is gone (Story 9.5 re-fashion).
        private val stopBtnRect = RectF()

        // Bar animation (waveform)
        private val barAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
            duration = 600
            repeatMode = ValueAnimator.REVERSE
            repeatCount = ValueAnimator.INFINITE
            interpolator = LinearInterpolator()
            addUpdateListener { invalidate() }
        }
        private val barPhaseOffsets = floatArrayOf(0f, 0.20f, 0.40f, 0.60f, 0.80f)

        // Pulse ring animation for live-dot
        private val pulseAnimator = ValueAnimator.ofFloat(0.5f, 1.2f).apply {
            duration = 1400
            repeatCount = ValueAnimator.INFINITE
            repeatMode = ValueAnimator.RESTART
            interpolator = LinearInterpolator()
            addUpdateListener { invalidate() }
        }

        // Rotation animator for spinner
        private val rotationAnimator = ValueAnimator.ofFloat(0f, 360f).apply {
            duration = 900
            repeatCount = ValueAnimator.INFINITE
            interpolator = LinearInterpolator()
            addUpdateListener { invalidate() }
        }

        init {
            // Start only the animators relevant to the initial state (RECORDING)
            applyAnimatorsForState(panelState)
        }

        /** Start/pause animators based on panel state to avoid off-state frame waste (F5b). */
        fun applyAnimatorsForState(state: State) {
            when (state) {
                State.RECORDING -> {
                    // Panel is passive (Modell B): no waveform, no live-dot, no moving elements here.
                    if (barAnimator.isRunning)      barAnimator.pause()
                    if (pulseAnimator.isRunning)    pulseAnimator.pause()
                    if (rotationAnimator.isRunning) rotationAnimator.pause()
                }
                State.TRANSCRIBING -> {
                    if (barAnimator.isRunning)   barAnimator.pause()
                    if (pulseAnimator.isRunning) pulseAnimator.pause()
                    if (!rotationAnimator.isRunning) rotationAnimator.start()
                }
            }
        }

        fun cancelAnimators() {
            barAnimator.cancel()
            pulseAnimator.cancel()
            rotationAnimator.cancel()
        }

        /** Returns true if the given view-local coordinates hit the Abbrechen (red square) button. */
        fun isTouchOnStopButton(viewX: Float, viewY: Float): Boolean =
            stopBtnRect.contains(viewX, viewY)

        private val fillPaint   = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
        private val strokePaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
        private val textPaint   = TextPaint(Paint.ANTI_ALIAS_FLAG).apply {
            color = KlarvoTheme.Dim
            textAlign = Paint.Align.LEFT
        }

        override fun onDraw(canvas: Canvas) {
            val dp = resources.displayMetrics.density
            val h  = height.toFloat()

            // ── K-badge: 18dp × 18dp squircle, teal gradient, dark K ──────────
            val badgeSize = 18 * dp
            val badgeX    = 0f
            val badgeCy   = h / 2f
            val badgeTop  = badgeCy - badgeSize / 2f
            val badgeRect = RectF(badgeX, badgeTop, badgeX + badgeSize, badgeTop + badgeSize)
            val tealShader = LinearGradient(
                badgeX, badgeTop, badgeX, badgeTop + badgeSize,
                KlarvoTheme.TealHi, KlarvoTheme.TealLo,
                Shader.TileMode.CLAMP
            )
            fillPaint.shader = tealShader
            canvas.drawRoundRect(badgeRect, 5 * dp, 5 * dp, fillPaint)
            fillPaint.shader = null

            // "K" letter
            val kSizeSp = 10f
            val kSizePx = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, kSizeSp, resources.displayMetrics)
            textPaint.textSize = kSizePx
            textPaint.color    = KlarvoTheme.OnTeal
            textPaint.typeface = Typeface.DEFAULT_BOLD
            textPaint.textAlign = Paint.Align.CENTER
            val kMetrics = textPaint.fontMetrics
            val kCy = badgeCy - (kMetrics.ascent + kMetrics.descent) / 2f
            canvas.drawText("K", badgeX + badgeSize / 2f, kCy, textPaint)
            textPaint.textAlign = Paint.Align.LEFT  // reset

            var curX = badgeX + badgeSize + 8 * dp

            when (panelState) {
                State.RECORDING -> {
                    // Panel passive (Modell B): K-badge (already drawn above) + "Aufnahme" label + timer.
                    // Waveform, live-dot, pulse ring, and cancel button are in the cluster (bubble window).
                    stopBtnRect.setEmpty()

                    // "Aufnahme" label
                    val labelSp      = 11f
                    val labelPx      = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, labelSp, resources.displayMetrics)
                    textPaint.textSize  = labelPx
                    textPaint.typeface  = Typeface.MONOSPACE
                    textPaint.color     = KlarvoTheme.Dim
                    textPaint.textAlign = Paint.Align.LEFT
                    val labelMetrics = textPaint.fontMetrics
                    val labelY = h / 2f - (labelMetrics.ascent + labelMetrics.descent) / 2f
                    canvas.drawText("Aufnahme", curX, labelY, textPaint)

                    // Timer (right-aligned, Muted)
                    val timerSp  = 10.5f
                    val timerPx  = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, timerSp, resources.displayMetrics)
                    val timerStr = formatElapsedMs(recordingElapsedMs)
                    textPaint.textSize  = timerPx
                    textPaint.typeface  = Typeface.MONOSPACE
                    textPaint.color     = KlarvoTheme.Muted
                    textPaint.textAlign = Paint.Align.RIGHT
                    val timerMetrics = textPaint.fontMetrics
                    val timerY = h / 2f - (timerMetrics.ascent + timerMetrics.descent) / 2f
                    canvas.drawText(timerStr, width.toFloat(), timerY, textPaint)
                    textPaint.textAlign = Paint.Align.LEFT  // reset
                }

                State.TRANSCRIBING -> {
                    // ── Teal spinner: 15dp diameter ───────────────────────────────
                    val spinnerSize = 15 * dp
                    val spinCx      = curX + spinnerSize / 2f
                    val spinCy      = h / 2f
                    val spinRadius  = spinnerSize / 2f * 0.85f
                    val startAngle  = (rotationAnimator.animatedValue as? Float) ?: 0f

                    strokePaint.color       = KlarvoTheme.Teal
                    strokePaint.strokeWidth = spinnerSize * 0.12f
                    strokePaint.style       = Paint.Style.STROKE
                    strokePaint.strokeCap   = Paint.Cap.ROUND
                    val spinRect = RectF(spinCx - spinRadius, spinCy - spinRadius, spinCx + spinRadius, spinCy + spinRadius)
                    canvas.drawArc(spinRect, startAngle, 270f, false, strokePaint)
                    strokePaint.strokeCap = Paint.Cap.BUTT

                    curX = spinCx + spinnerSize / 2f + 8 * dp

                    // ── "Cleaning…" label ─────────────────────────────────────────
                    val labelSp = 11f
                    val labelPx = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, labelSp, resources.displayMetrics)
                    textPaint.textSize  = labelPx
                    textPaint.typeface  = Typeface.MONOSPACE
                    textPaint.color     = KlarvoTheme.Dim
                    textPaint.textAlign = Paint.Align.LEFT
                    val labelMetrics = textPaint.fontMetrics
                    val labelY = h / 2f - (labelMetrics.ascent + labelMetrics.descent) / 2f
                    canvas.drawText("Bereinigt…", curX, labelY, textPaint)

                    // Reset stop button rect (not visible in TRANSCRIBING — only shown in RECORDING)
                    stopBtnRect.setEmpty()
                }
            }
        }

        private fun drawWaveformBars(canvas: Canvas, zoneLeft: Float, zoneRight: Float, cy: Float, dp: Float) {
            val barW   = 3 * dp
            val barGap = 3 * dp  // canon .hwave { gap: 3px } (F7)
            val barCount = 5
            val totalW = barW * barCount + barGap * (barCount - 1)
            val startX = (zoneLeft + zoneRight) / 2f - totalW / 2f + barW / 2f

            val maxBarH = 18 * dp  // zone height
            val minBarH = maxBarH * 0.10f

            val t  = (barAnimator.animatedValue as? Float) ?: 0f
            val silenceThreshold = 0.02f
            val isSilent = amplitude < silenceThreshold
            val dynamicFactor = if (isSilent) 0f else Math.pow(amplitude.toDouble(), 0.6).toFloat()

            fillPaint.color = KlarvoTheme.Amber

            for (i in 0 until barCount) {
                val barX = startX + i * (barW + barGap)
                if (barX - barW / 2f < zoneLeft || barX + barW / 2f > zoneRight) continue

                val phase = if (isSilent) 0f else (t + barPhaseOffsets[i]) % 1f
                val barH  = (minBarH + (maxBarH - minBarH) * phase * dynamicFactor).coerceIn(minBarH, maxBarH)

                val barRect = RectF(barX - barW / 2f, cy - barH / 2f, barX + barW / 2f, cy + barH / 2f)
                canvas.drawRoundRect(barRect, barW / 2f, barW / 2f, fillPaint)
            }
        }

        private fun formatElapsedMs(ms: Long): String {
            val totalSec = ms / 1000L
            val min      = totalSec / 60
            val sec      = totalSec % 60
            return "$min:${sec.toString().padStart(2, '0')}"
        }
    }

    /**
     * 2dp × 15dp amber rect that blinks at 1Hz during RECORDING (AC5: blinking amber caret).
     * Positioned at top-left of the transcript area; correct for an empty transcript.
     */
    private inner class CaretView(context: Context) : View(context) {
        private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
            style = Paint.Style.FILL
            color = KlarvoTheme.Amber
        }
        private var caretVisible = false
        private val blinkHandler = Handler(Looper.getMainLooper())
        private val blinkRunnable = object : Runnable {
            override fun run() {
                caretVisible = !caretVisible
                invalidate()
                blinkHandler.postDelayed(this, 500)
            }
        }

        fun startBlink() {
            blinkHandler.removeCallbacks(blinkRunnable)
            caretVisible = true
            invalidate()
            blinkHandler.postDelayed(blinkRunnable, 500)
        }

        fun stopBlink() {
            blinkHandler.removeCallbacks(blinkRunnable)
            caretVisible = false
            invalidate()
        }

        override fun onDraw(canvas: Canvas) {
            if (!caretVisible) return
            val dp = resources.displayMetrics.density
            canvas.drawRect(0f, 0f, 2 * dp, 15 * dp, paint)
        }

        override fun onDetachedFromWindow() {
            super.onDetachedFromWindow()
            blinkHandler.removeCallbacks(blinkRunnable)
        }
    }

    /**
     * Footer: keyboard icon placeholder (text-based) + caption text.
     */
    private inner class FooterView(context: Context) : View(context) {

        private val textPaint = TextPaint(Paint.ANTI_ALIAS_FLAG).apply {
            textAlign = Paint.Align.LEFT
            color     = KlarvoTheme.Dim
        }

        override fun onDraw(canvas: Canvas) {
            val dp      = resources.displayMetrics.density
            val sp11px  = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, 11f, resources.displayMetrics)
            textPaint.textSize  = sp11px
            textPaint.typeface  = Typeface.DEFAULT
            val metrics = textPaint.fontMetrics
            val textY   = height / 2f - (metrics.ascent + metrics.descent) / 2f

            // Simple keyboard icon: ⌨ unicode or text prefix
            val iconText = "⌨ "
            val captionText = when (panelState) {
                State.RECORDING    -> "Tastatur pausiert · kehrt beim Einfügen zurück"
                State.TRANSCRIBING -> "Gleich fertig · Tastatur kommt gleich zurück"
            }
            canvas.drawText(iconText + captionText, 0f, textY, textPaint)
        }
    }
}
