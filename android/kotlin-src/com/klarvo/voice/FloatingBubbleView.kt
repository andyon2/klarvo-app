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
 *   RECORDING     -- pill/bar shape with [X] [waveform] [checkmark] (HOLD mode)
 *                    OR circular Danger form with waveform (all other modes).
 *   TRANSCRIBING  -- Teal squircle + rotating arc spinner (was PROCESSING)
 *   DONE          -- Teal squircle + white checkmark (placeholder; Story 9.5 animates)
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
 *   KlarvoTheme.OnTeal      = dark "K" letter on teal fill (IDLE only)
 *   KlarvoTheme.TealBg      = ~12% alpha faint ring (IDLE glass-ring accent)
 *   KlarvoTheme.Danger      = stop / recording / cancel button
 *   No amber in IDLE — amber = recording tally (Story 9.5+ scope).
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
     * Story 9.5 fix: when true, the bubble renders the static IDLE squircle and stays the small
     * touch-target, regardless of [state]. KlarvoOverlayService.setState() sets this true for
     * RECORDING/TRANSCRIBING — the states where the listening panel owns the UI. This prevents the
     * bubble from drawing its OWN recording bar/circle as a SECOND overlay window for the same
     * state (the real-device double-window defect: a stray red recording pill floating over foreign
     * app content next to the panel). The bubble WINDOW stays alive so push-to-talk release and taps
     * still route through KlarvoOverlayService.handleTouch(); only the VISUAL is suppressed. Because
     * setState() drives this flag on every transition, the `alpha = 1.0f` reset there can no longer
     * un-hide a half-suppressed visual (the old `alpha = 0` approach failed for exactly that reason).
     */
    var suppressedForPanel: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            updateAnimators()   // stop recording/transcribing animators + reset scale to 1.0
            requestLayout()     // onMeasure: keep the touch-target size, never the bar width
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
        if (suppressedForPanel) {
            // Panel owns the active-state UI — keep the bubble a static idle squircle: no
            // waveform/spinner animation, no PTT scale-up. Reset any in-flight scale to 1.0.
            pttScaleUpAnimator.cancel()
            pttScaleDownAnimator.cancel()
            if (scaleX != 1.0f || scaleY != 1.0f) { scaleX = 1.0f; scaleY = 1.0f }
            return
        }
        when (state) {
            State.RECORDING -> {
                barAnimator.start()
                // Circular recording mode (non-HOLD): animate scale-up for tactile feedback.
                // Bar mode (HOLD) does not use scale animation (bar expands instead).
                pttScaleDownAnimator.cancel()
                pttScaleUpAnimator.setFloatValues(scaleX, 1.3f)
                pttScaleUpAnimator.start()
            }
            State.TRANSCRIBING -> {
                // Reset scale if we came from RECORDING circular mode
                if (scaleX != 1.0f) {
                    pttScaleUpAnimator.cancel()
                    pttScaleDownAnimator.setFloatValues(scaleX, 1.0f)
                    pttScaleDownAnimator.start()
                }
                rotationAnimator.start()
            }
            State.DONE, State.IDLE -> {
                // Ensure scale is reset
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
        if (state == State.RECORDING && !suppressedForPanel) {
            // Bar mode (WRAP_CONTENT window): compute our own size.
            // (Retired in practice by the Story 9.5 fix — suppressedForPanel is true in RECORDING —
            //  but kept for any non-panel RECORDING use.)
            val heightPx = (bubbleSizeDp * density).toInt()
            setMeasuredDimension((BAR_WIDTH_DP * density).toInt(), heightPx)
        } else {
            // IDLE / TRANSCRIBING / DONE: the window LayoutParams are an EXACT size
            // (visual squircle + shadow padding, set by KlarvoOverlayService.bubbleWindowPx()).
            // FILL it so the canvas/software-layer bitmap includes the padding — otherwise the
            // soft shadow + outer ring clip at the view edge (the square cutoff). The squircle
            // itself is drawn at bubbleSizeDp, centered, by onDraw.
            // RECORDING (circular form): window is also EXACT (touch-target); scale via scaleX/Y.
            setMeasuredDimension(
                MeasureSpec.getSize(widthMeasureSpec),
                MeasureSpec.getSize(heightMeasureSpec)
            )
        }
    }

    // --- Touch zone helpers (used by KlarvoOverlayService) ---

    /**
     * Returns true if [touchX] (relative to this view's left edge) falls inside
     * the cancel button zone on the left side of the recording bar.
     * Only meaningful in RECORDING state.
     */
    fun isTouchInCancelZone(touchX: Float): Boolean {
        // Suppressed (panel-owned recording): the bar isn't drawn, so its zones are inert —
        // cancel/confirm live on the panel now.
        if (state != State.RECORDING || suppressedForPanel) return false
        val density = resources.displayMetrics.density
        val barW = BAR_WIDTH_DP * density
        return touchX < barW * 0.30f
    }

    /**
     * Returns true if [touchX] falls inside the confirm button zone on the right.
     * Only meaningful in RECORDING state.
     */
    fun isTouchInConfirmZone(touchX: Float): Boolean {
        if (state != State.RECORDING || suppressedForPanel) return false
        val density = resources.displayMetrics.density
        val barW = BAR_WIDTH_DP * density
        return touchX > barW * 0.70f
    }

    // --- Draw ---

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)
        val w = width.toFloat()
        val h = height.toFloat()

        // Story 9.5 fix: while the listening panel owns the recording/transcribing UI, the bubble
        // renders the static idle squircle (AC1: "the bubble stays visible as the small teal
        // squircle above the panel") instead of its own recording bar/circle or spinner — which
        // would be a second overlay for the same state.
        val effectiveState = if (suppressedForPanel) State.IDLE else state

        when (effectiveState) {
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
            State.RECORDING -> {
                // HOLD mode (bar): KlarvoOverlayService sets RECORDING + adjustLayoutForState
                // to WRAP_CONTENT width — onMeasure returns BAR_WIDTH_DP × bubbleSizeDp.
                // All other modes (TOGGLE/AUTOSTOP/AUTO/PTT): window stays EXACT touch-target;
                // draw a circular Danger form with waveform bars (scale animation via scaleX/Y).
                if (width > height) {
                    // Bar mode: wide window → draw pill bar
                    drawRecordingBar(canvas, w, h)
                } else {
                    // Circular mode: square window → draw Danger circle with waveform
                    val cx = w / 2f
                    val cy = h / 2f
                    val visualRadius = (bubbleSizeDp * resources.displayMetrics.density) / 2f
                    // Shadow (slightly offset downward for depth)
                    canvas.drawCircle(cx, cy + visualRadius * 0.06f, visualRadius * 0.92f, shadowPaint)
                    // Danger filled circle (stop/recording — DT5)
                    circlePaint.color = KlarvoTheme.Danger
                    canvas.drawCircle(cx, cy, visualRadius, circlePaint)
                    // Waveform bars inside the circle
                    val waveHalfH = visualRadius * 0.70f
                    val waveLeft  = cx - visualRadius * 0.75f
                    val waveRight = cx + visualRadius * 0.75f
                    drawWaveformBarsInZone(canvas, waveLeft, waveRight, cx, cy, waveHalfH)
                }
            }
            State.TRANSCRIBING -> {
                val cx = w / 2f
                val cy = h / 2f
                // Use the same visual radius derivation as IDLE so the size never
                // grows to fill the touch-target window when leaving IDLE.
                val visualRadius = (bubbleSizeDp * resources.displayMetrics.density) / 2f
                val side = visualRadius * 2f
                // Rounded-square (squircle) form: same as IDLE — canon cornerRadius = 0.30 × side.
                val cornerPx = side * 0.30f
                squircleRect.set(cx - visualRadius, cy - visualRadius, cx + visualRadius, cy + visualRadius)
                // Teal fill (brand/transcribing — DT5; NOT amber)
                circlePaint.color = KlarvoTheme.Teal
                canvas.drawRoundRect(squircleRect, cornerPx, cornerPx, circlePaint)
                drawSpinner(canvas, cx, cy, visualRadius)
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
