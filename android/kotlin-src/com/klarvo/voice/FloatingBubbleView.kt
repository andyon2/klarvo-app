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
 *   IDLE          -- teal-gradient squircle + dark "K" (OnTeal) + faint teal glass ring
 *                    Canon: .ab-bubble.idle — background: linear-gradient(150deg, TealHi, TealLo)
 *                    NOT a dark Surface fill; NOT a teal "K". See Story 9.3 AC1/AC2.
 *   RECORDING     -- canon .ab-bubble.recording form: teal-gradient squircle + amber pulse-ring +
 *                    send-glyph (paper-plane) instead of "K". Tap = Senden (ADR-0019).
 *   TRANSCRIBING  -- same recording form (teal + amber pulse + send-glyph) — panel owns the
 *                    spinner/label; bubble stays in send-state so tap still sends.
 *   DONE          -- Teal squircle + white checkmark (Story 9.5)
 *
 * Size:
 *   Call setBubbleSize(dp) to resize the bubble at runtime (controls the visual radius).
 *   The WindowManager LayoutParams may be larger than bubbleSizeDp (touch-target expansion
 *   to ≥48dp); the visual circle is drawn centered within the larger touch-target bounds.
 *   In RECORDING state (bar mode, HOLD only) the view widens to BAR_WIDTH_DP; height stays
 *   at bubbleSizeDp. In RECORDING state (circular mode) scaleX/scaleY animation is used.
 *
 * Touch zones in RECORDING bar (HOLD mode only):
 *   - Left ~25% of width  -> cancel zone  (X button)
 *   - Right ~25% of width -> confirm zone (checkmark button)
 *   - Middle               -> waveform (no action)
 *   KlarvoOverlayService reads isTouchInCancelZone() / isTouchInConfirmZone() to route taps.
 *
 * Color semantics (DT5 — binding rule):
 *   KlarvoTheme.Teal        = brand / transcribing / focus-ring
 *   KlarvoTheme.OnTeal      = dark "K" letter on teal fill (IDLE) / send-glyph stroke (RECORDING)
 *   KlarvoTheme.TealBg      = ~12% alpha faint ring (IDLE glass-ring accent)
 *   KlarvoTheme.Danger      = cancel / abort (ADR-0019: red = Abbrechen only)
 *   KlarvoTheme.Amber       = recording pulse-ring (RECORDING / TRANSCRIBING)
 *   Tap in RECORDING = Senden (ADR-0019 core). Red square on panel = Abbrechen.
 */
class FloatingBubbleView(context: Context) : View(context) {

    enum class State { IDLE, RECORDING, TRANSCRIBING, DONE }

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

    /**
     * Story 9.5 (ADR-0019 rebuild): suppressedForPanel is no longer used. The bubble now renders
     * the canon `.ab-bubble.recording` form (teal + amber pulse-ring + send-glyph) during
     * RECORDING and TRANSCRIBING instead of a suppressed idle view. Kept as a no-op property so
     * KlarvoOverlayService can set it without compile errors; will be cleaned up in a later story.
     */
    @Suppress("UNUSED_PARAMETER")
    var suppressedForPanel: Boolean = false
        set(_) { /* no-op: replaced by recording-state visual in Story 9.5 */ }

    /** Current bubble size in dp. Changed via setBubbleSize(). */
    private var bubbleSizeDp: Int = 56

    companion object {
        /** Width of the recording bar in dp. */
        const val BAR_WIDTH_DP = 220

        /** Button circle radius as fraction of bubble height. */
        private const val BTN_RADIUS_FRACTION = 0.35f
    }

    init {
        // Software layer so BlurMaskFilter (the soft drop shadow) actually renders — it is a
        // no-op under hardware acceleration. The bubble is tiny and redraws rarely, so the
        // software-layer cost is negligible. (Story 9.3 polish: clean edges + soft shadow.)
        setLayerType(LAYER_TYPE_SOFTWARE, null)
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

    // --- Idle re-skin paints (Story 9.3 — teal-gradient squircle + dark K, canon-anchored) ---
    // idleFillPaint: teal LinearGradient fill (TealHi→TealLo at ~150°).
    // The LinearGradient shader is rebuilt in onDraw whenever the box size is known (see below),
    // because the shader coordinates depend on the view's pixel dimensions.
    private val idleFillPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        // shader is set dynamically in onDraw after we know the squircle dimensions
    }
    // idleRingPaint: ~3dp faint teal ring at ~12% alpha (TealBg) — "dezenter Glas-Ring".
    // This is a SUBTLE accent, NOT the bubble's primary color. strokeWidth set in onDraw.
    private val idleRingPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        color = KlarvoTheme.TealBg    // 0x1F29C7AC — ~12% alpha teal
        strokeCap = Paint.Cap.BUTT
    }
    // kLetterPaint: dark "K" in OnTeal (#05201B) — NOT teal, NOT white.
    // Canon: color: var(--k-on-teal). textSize set in onDraw.
    private val kLetterPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.FILL
        color = KlarvoTheme.OnTeal    // 0xFF05201B — dark on teal
        textAlign = Paint.Align.CENTER
        typeface = Typeface.DEFAULT_BOLD
    }

    // Reusable RectF for squircle drawing to avoid allocation in onDraw
    private val squircleRect = RectF()

    // --- Pre-allocated paints and path for the 60fps RECORDING hot path (Finding 2) ---
    // Avoids per-frame allocation in drawAmberPulseRings() and drawSendGlyph().
    private val amberRingPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        color = KlarvoTheme.Amber
    }
    private val sendGlyphPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        style = Paint.Style.STROKE
        color = KlarvoTheme.OnTeal
        strokeCap = Paint.Cap.ROUND
        strokeJoin = Paint.Join.ROUND
        // strokeWidth is set in drawSendGlyph() because it depends on dp density
    }
    // sendGlyphPath geometry is static (same 24×24 SVG path always); rebuilt only when size
    // changes, not every frame. Built lazily in drawSendGlyph() and cached by lastSendGlyphSize.
    private val sendGlyphPath = Path()
    private var lastSendGlyphSize = -1f   // cached size in px; -1 = not yet built

    // --- Bar animation (drives waveform bars in RECORDING state, both bar and circular modes) ---
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

    /**
     * Amber pulse-ring animator — canon `@keyframes abbubblepulse` (1400ms ease-out, infinite).
     * Drives the expanding amber ring(s) on `.ab-bubble.recording` (RECORDING + TRANSCRIBING).
     * Value: 0.0 → 1.0 represents one full pulse cycle (0%→100% in the keyframe spec).
     */
    private val amberPulseAnimator = ValueAnimator.ofFloat(0f, 1f).apply {
        duration = 1400
        repeatCount = ValueAnimator.INFINITE
        repeatMode = ValueAnimator.RESTART
        interpolator = android.view.animation.AccelerateDecelerateInterpolator()
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
        amberPulseAnimator.cancel()
        pttScaleUpAnimator.cancel()
        pttScaleDownAnimator.cancel()
    }

    private fun updateAnimators() {
        when (state) {
            State.RECORDING -> {
                // Canon .ab-bubble.recording: amber pulse-ring (1400ms, infinite).
                // ADR-0019: amber = RECORDING only; TRANSCRIBING shows teal squircle + send-glyph
                // with NO amber animation.
                barAnimator.cancel()
                rotationAnimator.cancel()
                if (!amberPulseAnimator.isRunning) amberPulseAnimator.start()
                // Tactile pop on recording start
                pttScaleDownAnimator.cancel()
                pttScaleUpAnimator.setFloatValues(scaleX, 1.1f)  // subtle pop for recording form
                pttScaleUpAnimator.start()
            }
            State.TRANSCRIBING -> {
                // TRANSCRIBING: teal squircle + send-glyph, NO amber pulse (ADR-0019).
                barAnimator.cancel()
                rotationAnimator.cancel()
                amberPulseAnimator.cancel()
            }
            State.DONE, State.IDLE -> {
                // Stop amber pulse + any recording animations; reset scale.
                barAnimator.cancel()
                amberPulseAnimator.cancel()
                rotationAnimator.cancel()
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
        // Story 9.5 (ADR-0019): the HOLD-tap bar is retired. All states use the EXACT touch-target
        // window (visual squircle + shadow padding), never the wide bar. The window LayoutParams
        // are an EXACT size set by KlarvoOverlayService.bubbleWindowPx(). FILL the measured dimensions
        // so the canvas/software-layer bitmap includes the padding (no shadow/ring clipping).
        setMeasuredDimension(
            MeasureSpec.getSize(widthMeasureSpec),
            MeasureSpec.getSize(heightMeasureSpec)
        )
    }

    // --- Touch zone helpers (used by KlarvoOverlayService) ---

    /**
     * Returns false — the recording bar is retired in Story 9.5.
     * Cancel is the red square on the panel; confirm is a tap anywhere on the recording bubble.
     * Kept for API compatibility; callers in handleTap() are effectively dead code paths since
     * tapMode==HOLD now also routes to the recording form (no bar expansion).
     */
    fun isTouchInCancelZone(@Suppress("UNUSED_PARAMETER") touchX: Float): Boolean = false

    /**
     * Returns false — see isTouchInCancelZone() note above.
     */
    fun isTouchInConfirmZone(@Suppress("UNUSED_PARAMETER") touchX: Float): Boolean = false

    // --- Draw ---

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val w = width.toFloat()
        val h = height.toFloat()

        when (state) {
            State.IDLE -> {
                // Touch-target expansion: the view may be larger than the visual bubble.
                // bubbleSizeDp controls the visual side length; the view is sized to the touch target.
                // Draw centered within the touch-target bounds.
                //
                // Canon: .ab-bubble.idle — background: linear-gradient(150deg, TealHi, TealLo)
                //   border-radius: 12px on a 40px box → cornerRadius ≈ 0.30 × side
                //   color: var(--k-on-teal) — dark "K" on teal fill
                //   ring: box-shadow 0 0 0 3px rgba(41,199,172,.13) — faint teal accent ~3dp
                // Shape: rounded-square (squircle), NOT circle. See AC2 / MANIFEST C1.
                val density = resources.displayMetrics.density
                val side = bubbleSizeDp * density            // visual side in px
                val visualRadius = side / 2f                 // "radius" = half the side
                val cx = w / 2f
                val cy = h / 2f

                // --- Squircle rect (centered in the touch-target view) ---
                // cornerRadius = 0.30 × side  (canon: 12px / 40px box)
                val cornerPx = side * 0.30f
                squircleRect.set(cx - visualRadius, cy - visualRadius, cx + visualRadius, cy + visualRadius)

                // Soft drop shadow (canon: 0 6px 18px rgba(0,0,0,.5)). A BlurMaskFilter gives a
                // smooth shadow instead of a hard grey squircle (which showed as a dirty edge on
                // light backgrounds). Needs the software layer (set in init). Radius + offset
                // scale with the bubble size.
                shadowPaint.maskFilter = BlurMaskFilter(side * 0.14f, BlurMaskFilter.Blur.NORMAL)
                val shadowDy = side * 0.06f
                val shadowRect = RectF(
                    squircleRect.left,
                    squircleRect.top    + shadowDy,
                    squircleRect.right,
                    squircleRect.bottom + shadowDy
                )
                canvas.drawRoundRect(shadowRect, cornerPx, cornerPx, shadowPaint)

                // Teal-gradient fill: LinearGradient at ~150° (top-left → bottom-right).
                // Rebuild shader each draw pass; squircleRect coordinates may change when
                // bubbleSizeDp is updated (responsive sizing). Allocation is acceptable here
                // since the bubble size only changes at setup / orientation change, not per frame.
                val gradX0 = squircleRect.left
                val gradY0 = squircleRect.top
                val gradX1 = squircleRect.right
                val gradY1 = squircleRect.bottom
                idleFillPaint.shader = LinearGradient(
                    gradX0, gradY0, gradX1, gradY1,
                    KlarvoTheme.TealHi, KlarvoTheme.TealLo,
                    Shader.TileMode.CLAMP
                )
                canvas.drawRoundRect(squircleRect, cornerPx, cornerPx, idleFillPaint)

                // Faint teal accent ring — drawn OUTSIDE the fill edge (canon: 0 0 0 3px
                // rgba(.13) spread), so it never overlaps the gradient and creates an inner seam.
                // Very subtle; the vibrant fill + soft shadow carry the look.
                val ringStrokePx = 2f * density
                idleRingPaint.strokeWidth = ringStrokePx
                val ringHalf = ringStrokePx / 2f
                val ringRect = RectF(
                    squircleRect.left   - ringHalf,
                    squircleRect.top    - ringHalf,
                    squircleRect.right  + ringHalf,
                    squircleRect.bottom + ringHalf
                )
                canvas.drawRoundRect(ringRect, cornerPx + ringHalf, cornerPx + ringHalf, idleRingPaint)

                // Dark "K" centered in the squircle.
                // textSize ≈ 0.85 × visualRadius  (canon: 17px on 40px box = 0.85 × 20px radius)
                // Vertical center: drawText baseline = cy - (ascent+descent)/2 centers the glyph.
                kLetterPaint.textSize = visualRadius * 0.85f
                val textCy = cy - (kLetterPaint.ascent() + kLetterPaint.descent()) / 2f
                canvas.drawText("K", cx, textCy, kLetterPaint)
            }
            State.RECORDING, State.TRANSCRIBING -> {
                // Canon .ab-bubble.recording (ADR-0019, Story 9.5):
                //   - teal-gradient squircle (same shape as IDLE, same 40dp/r=12dp)
                //   - amber pulse-ring (@keyframes abbubblepulse) — RECORDING only (ADR-0019)
                //   - send-glyph (paper-plane) instead of "K" — both RECORDING and TRANSCRIBING
                // The panel owns the spinner/label distinction.
                // Tap = Senden; panel red square = Abbrechen.
                val density = resources.displayMetrics.density
                val side = bubbleSizeDp * density
                val visualRadius = side / 2f
                val cx = w / 2f
                val cy = h / 2f
                val cornerPx = side * 0.30f

                squircleRect.set(cx - visualRadius, cy - visualRadius, cx + visualRadius, cy + visualRadius)

                // Soft drop shadow (same as IDLE)
                shadowPaint.maskFilter = BlurMaskFilter(side * 0.14f, BlurMaskFilter.Blur.NORMAL)
                val shadowDy = side * 0.06f
                val shadowRect = RectF(
                    squircleRect.left,
                    squircleRect.top    + shadowDy,
                    squircleRect.right,
                    squircleRect.bottom + shadowDy
                )
                canvas.drawRoundRect(shadowRect, cornerPx, cornerPx, shadowPaint)

                // Teal-gradient fill (same gradient as IDLE — canon .ab-bubble.recording reuses it)
                idleFillPaint.shader = LinearGradient(
                    squircleRect.left, squircleRect.top,
                    squircleRect.right, squircleRect.bottom,
                    KlarvoTheme.TealHi, KlarvoTheme.TealLo,
                    Shader.TileMode.CLAMP
                )
                canvas.drawRoundRect(squircleRect, cornerPx, cornerPx, idleFillPaint)

                // Amber pulse-ring: RECORDING only (ADR-0019 — amber = recording state only).
                // In TRANSCRIBING the bubble shows teal squircle + send-glyph with no amber ring.
                if (state == State.RECORDING) {
                    drawAmberPulseRings(canvas, cx, cy, visualRadius)
                }

                // Send-glyph: paper-plane path, ~20dp, OnTeal stroke ~2.2dp
                // Canon path: "m22 2-7 20-4-9-9-4 20-7z" (24×24 SVG viewBox)
                // Shown in both RECORDING and TRANSCRIBING (AC6: recording form retained).
                drawSendGlyph(canvas, cx, cy, side * 0.50f)
            }

            State.DONE -> {
                val cx = w / 2f
                val cy = h / 2f
                val visualRadius = (bubbleSizeDp * resources.displayMetrics.density) / 2f
                val side = visualRadius * 2f
                val cornerPx = side * 0.30f
                squircleRect.set(cx - visualRadius, cy - visualRadius, cx + visualRadius, cy + visualRadius)
                // Teal squircle (same as IDLE / TRANSCRIBING base form)
                circlePaint.color = KlarvoTheme.Teal
                canvas.drawRoundRect(squircleRect, cornerPx, cornerPx, circlePaint)
                // White checkmark centered — placeholder for Story 9.5 animated transition
                drawCheckMark(canvas, cx, cy, visualRadius * 0.35f)
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

    // --- RECORDING (.ab-bubble.recording): amber pulse-ring ---

    /**
     * Draws the canon `@keyframes abbubblepulse` amber rings (Story 9.5 / ADR-0019).
     *
     * Keyframe spec (rings in Amber rgba(233,162,76,…)):
     *   0%/100%: ring1 = spread 2px alpha .95, ring2 = spread 4px alpha .35
     *   70%:     ring1 = spread 3px alpha .55, ring2 = spread 15px alpha 0
     *
     * Two ValueAnimator-driven rings interpolated from 0→1:
     *   - Inner ring: spread 2→3→2dp, alpha 0.95→0.55→0.95
     *   - Outer ring: spread 4→15→4dp, alpha 0.35→0→0.35
     */
    private fun drawAmberPulseRings(canvas: Canvas, cx: Float, cy: Float, visualRadius: Float) {
        val dp = resources.displayMetrics.density
        val t = (amberPulseAnimator.animatedValue as? Float) ?: 0f

        // Pulse position in keyframe space: t=0→0%, t=0.7→70%, t=1→100%
        // Using a triangle wave: peaks at t=0.7, returns to start at t=1.
        val phase = if (t <= 0.7f) t / 0.7f else (1f - t) / 0.3f  // 0..1..0

        // Inner ring: spread 2dp (phase=0) → 3dp (phase=1), alpha 0.95→0.55
        val innerSpread = (2f + phase * 1f) * dp
        val innerAlpha = (0.95f - phase * 0.40f).coerceIn(0f, 1f)

        // Outer ring: spread 4dp (phase=0) → 15dp (phase=1), alpha 0.35→0
        val outerSpread = (4f + phase * 11f) * dp
        val outerAlpha = (0.35f * (1f - phase)).coerceIn(0f, 1f)

        // Reuse pre-allocated amberRingPaint — no per-frame allocation.
        // Outer ring (drawn first, behind inner)
        amberRingPaint.strokeWidth = outerSpread
        amberRingPaint.alpha = (outerAlpha * 255).toInt()
        val outerR = visualRadius + outerSpread / 2f
        canvas.drawCircle(cx, cy, outerR, amberRingPaint)

        // Inner ring
        amberRingPaint.strokeWidth = innerSpread
        amberRingPaint.alpha = (innerAlpha * 255).toInt()
        val innerR = visualRadius + innerSpread / 2f
        canvas.drawCircle(cx, cy, innerR, amberRingPaint)
    }

    // --- RECORDING (.ab-bubble.recording): send-glyph (paper-plane) ---

    /**
     * Draws the send (paper-plane) glyph centered in the squircle.
     *
     * Canon SVG path: "m22 2-7 20-4-9-9-4 20-7z" on a 24×24 viewBox.
     * Stroke: OnTeal color, ~2.2dp width. Size: ~20dp × 20dp.
     *
     * @param size  The glyph bounding box side in px (caller passes side × 0.50f).
     */
    private fun drawSendGlyph(canvas: Canvas, cx: Float, cy: Float, size: Float) {
        val dp = resources.displayMetrics.density

        // Scale the 24×24 SVG path into the [size × size] bounding box.
        // The path geometry is static — rebuild only when size changes (e.g. bubbleSizeDp update),
        // not every frame. cx/cy are always the view centre, so we rebuild with a translate trick:
        // build the path relative to (0,0), then draw with a canvas save/translate.
        // This avoids both per-frame Path allocation and per-frame coordinate re-computation.
        if (size != lastSendGlyphSize) {
            // Rebuild path in centred coordinates (origin = centre of glyph)
            val scale = size / 24f
            sendGlyphPath.reset()
            // Path points derived from "m22 2-7 20-4-9-9-4 20-7z"
            // Absolute coords from the relative 'm': (22,2)→(15,22)→(11,13)→(2,9)→close
            sendGlyphPath.moveTo(22f * scale - size / 2f, 2f  * scale - size / 2f)
            sendGlyphPath.lineTo(15f * scale - size / 2f, 22f * scale - size / 2f)
            sendGlyphPath.lineTo(11f * scale - size / 2f, 13f * scale - size / 2f)
            sendGlyphPath.lineTo( 2f * scale - size / 2f,  9f * scale - size / 2f)
            sendGlyphPath.close()
            lastSendGlyphSize = size
        }

        // Reuse pre-allocated sendGlyphPaint — only strokeWidth depends on runtime dp value.
        sendGlyphPaint.strokeWidth = 2.2f * dp

        // Translate canvas so (0,0) aligns with the bubble centre, draw, then restore.
        canvas.save()
        canvas.translate(cx, cy)
        canvas.drawPath(sendGlyphPath, sendGlyphPaint)
        canvas.restore()
    }

    // --- TRANSCRIBING: rotating arc spinner ---

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
