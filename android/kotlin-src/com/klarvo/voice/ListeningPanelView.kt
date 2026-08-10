package com.klarvo.voice

import android.animation.ValueAnimator
import android.content.Context
import android.graphics.*
import android.graphics.drawable.GradientDrawable
import android.os.Handler
import android.os.Looper
import android.text.TextPaint
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.animation.LinearInterpolator
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView

/**
 * Klarvo listening-panel overlay for the recording / transcribing states.
 *
 * Implemented as a LinearLayout wrapper containing:
 *   - TopRowView          — Canvas-drawn row: K-badge, live-dot/spinner, waveform/"Cleaning…", timer, stop-btn
 *   - transcriptScrollView — single scrollable transcript TextView (Story 11-3 AC-3 pivot,
 *                            2026-07-08 — supersedes the 2026-07-07 fixed rolling-window/no-scroll
 *                            rendering: the WINDOW is fixed-height (Task 5), but its CONTENT is
 *                            now a bounded scroll view that auto-scrolls to the newest text and
 *                            can be manually scrolled back up)
 *   - FooterView          — Canvas-drawn footer row with caption text
 *
 * Added to WindowManager as a separate TYPE_APPLICATION_OVERLAY window (AC5). The window itself
 * is fixed-height (Story 11-3, `KlarvoOverlayService.showListeningPanel`) — the panel never
 * grows or shrinks; the transcript scrolls internally instead.
 * Panel enters with spring animation (240ms OvershootInterpolator(1.8f)) on attach.
 *
 * States:
 *   RECORDING     — amber live-dot + waveform + timer + stop button; top amber border-line
 *   TRANSCRIBING  — teal spinner + "Cleaning…" label; top Border2 line; text dimmed
 */
class ListeningPanelView(context: Context) : LinearLayout(context) {

    enum class State { RECORDING, TRANSCRIBING }

    companion object {
        private val RGBA_REGEX =
            Regex("""rgba?\(\s*(\d{1,3})\s*,\s*(\d{1,3})\s*,\s*(\d{1,3})(?:\s*,\s*([\d.]+))?\s*\)""")
        // Story 11-3 (AC-5, item 5): device feedback pass rescale 11/13/15 -> 13/15/18.
        // Android-only; desktop's FONT_PX_MAP (Story 6-3) is untouched.
        private val FONT_PX_SP = mapOf("small" to 13f, "medium" to 15f, "large" to 18f)
        // Story 11.6 DESIGN DECISION 2 (step size widened at the 2026-08-10 review gate,
        // finding D2): "medium" = today's hardcoded 1.7, so nothing changes visually until
        // the user touches the control. "small"/"large" are a symmetric ±0.25 offset here
        // (not ±0.30, unlike Desktop): `setLineSpacing(0f, mult)` multiplies the font's
        // *natural line height* (~1.2× text size), while Desktop's GDI line-stepping and
        // CSS `lineHeight` multiply the *font size* directly (see native_preview.rs's
        // `line_height_mult` comment) — ±0.25 here and ±0.30 on Desktop move the rendered
        // spacing by the same ±0.30 em on both platforms. To be confirmed at GATE-4 on the
        // real device.
        private val LINE_SPACING_MULT = mapOf("small" to 1.45f, "medium" to 1.7f, "large" to 1.95f)

        /**
         * Parses a CSS `rgba()`/`rgb()` string into an Android ARGB color int (Story 11-2,
         * Task 5.2). Mirrors `src/components/settings/previewAppearance.ts`'s
         * `rgbaToHexOpacity` regex/validation. Falls back to [fallback] on any malformed input
         * -- never throws (config values are user-editable free text).
         */
        fun parseRgba(css: String, fallback: Int): Int {
            val m = RGBA_REGEX.find(css) ?: return fallback
            val r = m.groupValues[1].toIntOrNull() ?: return fallback
            val g = m.groupValues[2].toIntOrNull() ?: return fallback
            val b = m.groupValues[3].toIntOrNull() ?: return fallback
            val a = m.groupValues[4].takeIf { it.isNotEmpty() }?.toFloatOrNull() ?: 1f
            if (listOf(r, g, b).any { it < 0 || it > 255 } || a < 0f || a > 1f) return fallback
            val alphaByte = (a * 255f).toInt().coerceIn(0, 255)
            return Color.argb(alphaByte, r, g, b)
        }

        /**
         * Maps the curated desktop font-family stack (`previewAppearance.ts` `PREVIEW_FONTS`)
         * to an Android [Typeface] (Task 5.3). No native asset is currently wired for Inter in
         * Kotlin (checked: no `R.font.*` reference exists anywhere in `android/kotlin-src`
         * today), so Inter and System UI both map to the platform default; Serif/Monospace map
         * to their Android built-ins.
         */
        fun typefaceForFontFamily(fontFamily: String): Typeface = when {
            fontFamily.contains("Georgia", ignoreCase = true) -> Typeface.SERIF
            fontFamily.contains("Cascadia", ignoreCase = true) ||
                fontFamily.contains("Fira Code", ignoreCase = true) ||
                fontFamily.contains("Consolas", ignoreCase = true) -> Typeface.MONOSPACE
            else -> Typeface.DEFAULT
        }

        /**
         * Code-review fix F4 (2026-07-01, pure/testable): decides which color the transcript
         * text should use on a [State] change. When an appearance config has been applied
         * ([appliedPreviewTextColor] non-null), it must win outright -- honoring the configured
         * preview text color across state transitions (desktop parity: `PreviewPanel.tsx` uses
         * one configured color for both states, no per-state dimming). Only when no appearance
         * has been applied (preview off) does this fall back to the pre-11-2 stock
         * Muted(RECORDING)/Dim(TRANSCRIBING) split.
         */
        fun resolveTranscriptColor(
            appliedPreviewTextColor: Int?,
            panelState: State,
            recordingColor: Int,
            transcribingColor: Int,
        ): Int =
            appliedPreviewTextColor ?: if (panelState == State.RECORDING) recordingColor else transcribingColor

        // --- Story 11-3 (AC-3 pivot, 2026-07-08): fixed-height, auto-scroll-to-newest transcript ---

        /**
         * How close (in dp) the viewport's bottom edge must be to the transcript content's
         * bottom edge to still count as "at the bottom" (Task 8.3). A small tolerance instead of
         * an exact-zero comparison absorbs rounding/animation jitter from the ScrollView's own
         * `fullScroll` calls.
         */
        const val SCROLL_STICK_THRESHOLD_DP = 24

        /**
         * Pure "stick to bottom" decision (Story 11-3, AC-3 pivot, Task 8.3/8.5). Given a
         * ScrollView's current [scrollY], its own [viewportHeight], and the scrolling child's
         * total [contentHeight], returns whether the viewport is currently showing the newest
         * (bottom-most) transcript text within [thresholdPx]. Used both to decide "is the user
         * currently following the newest text" after a manual scroll, and — read just before new
         * text lands — whether to auto-scroll to the new bottom (AC-3c) or leave a deliberately
         * scrolled-up user alone (AC-3d).
         *
         * No Context/View/I-O — directly JVM-testable (mirrors `RecordingMode.selectSilenceSecs`).
         */
        fun isScrolledToBottom(scrollY: Int, viewportHeight: Int, contentHeight: Int, thresholdPx: Int): Boolean {
            if (contentHeight <= viewportHeight) return true
            return (contentHeight - (scrollY + viewportHeight)) <= thresholdPx
        }
    }

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

    /**
     * True while the HOLD Cancel surface is active (PTT hold in progress, Story 9-14).
     * Changes the RECORDING header label to "Aufnahme · halten".
     */
    var isHoldMode: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            topRowView.invalidate()
        }

    var amplitude: Float = 0f
        set(value) {
            field = value.coerceIn(0f, 1f)
            topRowView.invalidate()
        }

    var rawTranscript: String = ""
        set(value) {
            field = value
            // Story 11-3 (AC-3 pivot, Task 8.3, review fixes F1/F2): read stickToBottom BEFORE the
            // text change lands -- it reflects the user's scroll position relative to the OLD
            // content, i.e. "was the user following the newest text just before this update". A
            // user who has deliberately scrolled up is left alone (AC-3d); this only decides
            // whether to bother scheduling a scroll at all.
            val shouldFollow = transcriptScrollView.stickToBottom
            transcriptTextView.text = value
            if (shouldFollow) {
                // F2: don't scroll on a bare post {} -- the TextView's re-layout to its new
                // (taller) height is scheduled via the Choreographer and is not guaranteed to run
                // before a Handler.post runnable, so a bare post can scroll to the OLD bottom and
                // clip the just-appended newest line. Wait for the TextView to actually re-layout
                // (one-shot listener, self-removing so it never leaks/duplicates across appends).
                // F1: re-check stickToBottom at execution time -- the user may have scrolled up
                // between enqueue and this callback firing (rapid preview appends).
                transcriptTextView.addOnLayoutChangeListener(object : View.OnLayoutChangeListener {
                    override fun onLayoutChange(
                        v: View,
                        left: Int, top: Int, right: Int, bottom: Int,
                        oldLeft: Int, oldTop: Int, oldRight: Int, oldBottom: Int
                    ) {
                        v.removeOnLayoutChangeListener(this)
                        if (transcriptScrollView.stickToBottom) {
                            transcriptScrollView.fullScroll(View.FOCUS_DOWN)
                        }
                    }
                })
            }
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
     * Code-review fix F4 (2026-07-01): the configured preview text color, set by
     * [applyAppearance]. `null` means no appearance config has been applied (preview off, or
     * panel not yet configured) -- [updateTranscriptColor] falls back to the pre-11-2
     * Muted/Dim behavior in that case. Stored so state transitions (RECORDING<->TRANSCRIBING)
     * can honor the configured color instead of hard-resetting it.
     */
    private var appliedPreviewTextColor: Int? = null

    /**
     * Applies the ported preview-appearance config fields (Story 11-2, AC-9/Task 5.2/5.3) to
     * the panel's background, border, transcript text color and font. Call at show-time with a
     * freshly-read config (mirrors the desktop 6.6 "separate-window reactive read" lesson --
     * do not cache stale values across a Settings session).
     */
    fun applyAppearance(config: KlarvoApi.Config) {
        val dp = resources.displayMetrics.density
        val bgColor = parseRgba(config.previewBgColor, Color.rgb(0x12, 0x14, 0x16))
        val borderColor = parseRgba(config.previewBorderColor, KlarvoTheme.Border2)
        background = GradientDrawable().apply {
            setColor(bgColor)
            cornerRadius = config.previewBorderRadius * dp
            setStroke((config.previewBorderWidth * dp).toInt().coerceAtLeast(0), borderColor)
        }
        val textColor = parseRgba(config.previewTextColor, KlarvoTheme.Muted)
        // Fix F4: remember the applied color so a later state change (panelState setter ->
        // updateTranscriptColor) doesn't discard it back to the hardcoded Muted/Dim.
        appliedPreviewTextColor = textColor
        val typeface = typefaceForFontFamily(config.previewFontFamily)
        val sizeSp = FONT_PX_SP[config.previewFontSize] ?: 15f
        val lineSpacingMult = LINE_SPACING_MULT[config.previewLineSpacing] ?: 1.7f
        transcriptTextView.setTextColor(textColor)
        transcriptTextView.typeface = typeface
        transcriptTextView.textSize = sizeSp
        transcriptTextView.setLineSpacing(0f, lineSpacingMult)
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
    // Story 11-3 (AC-3 pivot, 2026-07-08): single scrollable transcript TextView (replaces the
    // 2026-07-07 fixed pool of ROLLING_MAX_LINES TextViews) -- the panel WINDOW stays fixed
    // height (Task 5, unchanged), but its content now scrolls internally instead of evicting
    // older lines.
    private val transcriptTextView: TextView
    private val transcriptScrollView: TranscriptScrollView
    private val footerView: FooterView
    private val caretView: CaretView

    init {
        orientation = VERTICAL
        // Fully opaque dark background (panel is not a glass surface — no alpha needed).
        // LAYER_TYPE_SOFTWARE is intentionally absent: panel uses no BlurMaskFilter.
        setBackgroundColor(Color.rgb(0x12, 0x14, 0x16))

        val dp = resources.displayMetrics.density

        // Story 11-3 (AC-4, item 4): GripView (grab-handle) removed — it implied a resize
        // affordance that doesn't exist. Top padding tightened from 9dp so topRowView sits
        // closer to the panel's top edge with the reclaimed space (was 9dp + 4dp grip +
        // 11dp grip bottom-margin = 24dp before topRowView; now a single reduced top padding).
        setPadding(0, (6 * dp).toInt(), 0, (18 * dp).toInt())

        // Top row
        topRowView = TopRowView(context)
        val topRowParams = LayoutParams(LayoutParams.MATCH_PARENT, (26 * dp).toInt()).apply {
            leftMargin = (16 * dp).toInt()
            rightMargin = (16 * dp).toInt()
            bottomMargin = (11 * dp).toInt()
        }
        addView(topRowView, topRowParams)

        // Transcript area: single scrollable TextView + blinking amber caret, in a FrameLayout
        // overlay (Story 11-3 AC-3 pivot, 2026-07-08). The panel WINDOW is fixed-height (Task 5,
        // unchanged) so this ScrollView actually bounds/scrolls its content instead of being
        // inert (the pre-pivot 2026-07-07 rolling-window rendering existed only because a
        // WRAP_CONTENT window made a plain ScrollView useless).
        transcriptTextView = TextView(context).apply {
            setTextColor(KlarvoTheme.Muted)
            textSize = 15f  // sp — FONT_PX_SP "medium" default until applyAppearance runs
            typeface = Typeface.MONOSPACE
            setLineSpacing(0f, 1.7f)  // LINE_SPACING_MULT "medium" default until applyAppearance runs
            gravity = Gravity.TOP or Gravity.START
            text = ""
        }
        caretView = CaretView(context)

        val transcriptFrame = FrameLayout(context)
        transcriptFrame.addView(transcriptTextView, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        ))
        val caretLp = FrameLayout.LayoutParams(
            (2 * dp).toInt(),
            (15 * dp).toInt()
        ).apply { gravity = Gravity.TOP or Gravity.START }
        transcriptFrame.addView(caretView, caretLp)

        // Task 8.4: the ScrollView itself carries no touch-blocking flags, and the panel overlay
        // window (KlarvoOverlayService.showListeningPanel) does not set FLAG_NOT_TOUCHABLE (the
        // HyperOS alpha-dim quirk, see Dev Notes) -- so drag/fling gestures on the panel reach
        // this ScrollView normally, both for the stick-to-bottom auto-scroll and manual scroll-up.
        transcriptScrollView = TranscriptScrollView(context).apply {
            isVerticalScrollBarEnabled = false
        }
        transcriptScrollView.addView(transcriptFrame, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        ))

        val textParams = LayoutParams(LayoutParams.MATCH_PARENT, 0, 1f).apply {
            leftMargin = (16 * dp).toInt()
            rightMargin = (16 * dp).toInt()
        }
        addView(transcriptScrollView, textParams)

        // Footer
        footerView = FooterView(context)
        val footerParams = LayoutParams(LayoutParams.MATCH_PARENT, (20 * dp).toInt()).apply {
            leftMargin = (16 * dp).toInt()
            rightMargin = (16 * dp).toInt()
        }
        addView(footerView, footerParams)

        // Outer: draw the top border-line via dispatchDraw override
        setWillNotDraw(false)

        // Story 11-3 (Task 5.2): the pre-11-3 200dp `minimumHeight` floor (AC5/F8) is now
        // redundant — the WindowManager window itself is a FIXED 200dp height
        // (`KlarvoOverlayService.showListeningPanel`, Task 5.1), not just a minimum. Two
        // competing height mechanisms would be confusing; the window-level fix is the one that
        // actually prevents the window from growing, so it wins and this view-level floor is
        // removed.
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

    /**
     * Code-review fix F4 (2026-07-01): previously hard-reset to Muted/Dim on every
     * [panelState] change, discarding whatever [applyAppearance] configured -- so a configured
     * preview text color reverted on the very first RECORDING->TRANSCRIBING transition. Desktop
     * parity (`PreviewPanel.tsx`) uses a single configured `previewTextColor` for both states
     * (no per-state dimming), so once an appearance has been applied we honor it unconditionally
     * here too; the Muted/Dim split is only the pre-11-2 stock look for when no appearance
     * config has been applied (preview off).
     */
    private fun updateTranscriptColor() {
        val color = resolveTranscriptColor(appliedPreviewTextColor, panelState, KlarvoTheme.Muted, KlarvoTheme.Dim)
        transcriptTextView.setTextColor(color)
    }

    // -------------------------------------------------------------------------
    // Inner views
    // -------------------------------------------------------------------------

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
                    // Label: isHoldMode, then default (AC8, Story 9-14 — the isLockedMode variant
                    // was removed in the 2026-07-01 re-scope, Sperren/lock is gone).
                    // Story 11-3 (AC-1, item 1): "Aufnahme" -> "Live-Preview" identity rename.
                    val recordingLabel = when {
                        isHoldMode -> "Live-Preview · halten"
                        else       -> "Live-Preview"
                    }
                    canvas.drawText(recordingLabel, curX, labelY, textPaint)

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
     * Story 11-3 (AC-3 pivot, Task 8.2/8.3): custom [ScrollView] that tracks whether the
     * viewport currently shows the newest (bottom-most) transcript text. [onScrollChanged] fires
     * on every scroll -- both user drag/fling AND this view's own programmatic
     * `fullScroll(FOCUS_DOWN)` calls -- so [stickToBottom] always reflects the CURRENT scroll
     * position relative to the CURRENT content height, regardless of what caused the last
     * scroll. [rawTranscript]'s setter reads [stickToBottom] just before new text lands to decide
     * whether to auto-scroll to the new bottom (AC-3c) or leave a manually-scrolled-up user alone
     * (AC-3d) -- delegates the actual decision to the pure [isScrolledToBottom].
     */
    private inner class TranscriptScrollView(context: Context) : ScrollView(context) {
        private val thresholdPx = (SCROLL_STICK_THRESHOLD_DP * resources.displayMetrics.density).toInt()

        var stickToBottom: Boolean = true
            private set

        override fun onScrollChanged(l: Int, t: Int, oldl: Int, oldt: Int) {
            super.onScrollChanged(l, t, oldl, oldt)
            val content = getChildAt(0) ?: return
            stickToBottom = isScrolledToBottom(t, height, content.height, thresholdPx)
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

            // Honest status caption. The previous "Tastatur pausiert / kommt zurück" wording
            // claimed a keyboard-collapse behaviour that does NOT happen yet — an overlay cannot
            // dismiss another app's IME (it is NOT_FOCUSABLE); real keyboard collapse needs the
            // AccessibilityService (Story 9-6). Until 9-6 lands, describe only what actually
            // happens. The mic glyph replaces the misleading keyboard glyph.
            // (The locked-state footer override — "Finger losgelassen · weiter über die Knöpfe" —
            // was removed in the Story 9-14 2026-07-01 re-scope: there is no locked/TAP-handoff
            // state anymore, release always either sends or cancels.)
            // Story 11-3 (AC-2, item 2): RECORDING's "Ich höre zu …" caption removed — the
            // preview context makes it unnecessary. TRANSCRIBING's caption is unchanged (out of
            // scope per this story's decisions).
            val iconText = "🎙 "
            val captionText = when (panelState) {
                State.RECORDING    -> null
                State.TRANSCRIBING -> "Wird verarbeitet …"
            }
            if (captionText != null) {
                canvas.drawText(iconText + captionText, 0f, textY, textPaint)
            }
        }
    }
}
