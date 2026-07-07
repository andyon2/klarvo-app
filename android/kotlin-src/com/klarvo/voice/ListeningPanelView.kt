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
import android.widget.TextView

/**
 * Klarvo listening-panel overlay for the recording / transcribing states.
 *
 * Implemented as a LinearLayout wrapper containing:
 *   - TopRowView        — Canvas-drawn row: K-badge, live-dot/spinner, waveform/"Cleaning…", timer, stop-btn
 *   - linesContainer    — fixed pool of [ROLLING_MAX_LINES] TextViews, rolling-window transcript
 *                         (Story 11-3, item 3 — replaces the pre-11-3 single-TextView/ScrollView)
 *   - FooterView        — Canvas-drawn footer row with caption text
 *
 * Added to WindowManager as a separate TYPE_APPLICATION_OVERLAY window (AC5). The window itself
 * is fixed-height (Story 11-3, `KlarvoOverlayService.showListeningPanel`) — the panel never grows
 * or scrolls; older transcript lines roll out the top with a soft fade instead (item 3).
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

        // --- Story 11-3 (AC-3, item 3): fixed-size rolling-window transcript ---

        /**
         * Rolling-window capacity in lines (Task 4.2 first-pass value — OPEN ITEM #1, not
         * pixel-pinned by Andi; device-tunable at GATE-4, mirrors Story 9-15's
         * `recordingButtonSizeDp` precedent). Paired with the panel's fixed 200dp window height
         * (`KlarvoOverlayService.showListeningPanel`, Task 5.1) and the FONT_PX_SP medium
         * default (15sp * 1.7 line-spacing), 5 lines comfortably fits the fixed box without the
         * box ever growing.
         */
        const val ROLLING_MAX_LINES = 5

        /**
         * Fade duration for the topmost (about-to-be-evicted) line's soft dim (Task 4.2) —
         * matches the panel's existing 320ms collapse-animation convention ([hideWithAnimation]).
         */
        const val ROLLING_FADE_MS = 280L

        /** Alpha the topmost line dims to once flagged fading (soft fade, not an abrupt cut — AC-3c). */
        const val ROLLING_FADE_ALPHA = 0.35f

        /** One rolling-window transcript line, with whether it is about to roll off the top. */
        data class Line(val text: String, val isFading: Boolean)

        /**
         * Pure line-buffer eviction (Story 11-3, AC-3/Task 4.1). [chunks] is every accumulated
         * preview line in arrival order (oldest first) — one entry per `appendPreviewText` flush
         * (`KlarvoOverlayService`, newline-joined; see Task 4.2 Completion Notes). Returns only
         * the most recent [maxLines] of them; this bound is the direct fix for the "fills the
         * screen" defect (item 3) — the window itself never needs to grow because this function
         * never returns more than [maxLines] lines, regardless of how much has accumulated.
         *
         * The oldest of the returned lines is flagged [Line.isFading] whenever older chunks
         * exist beyond the window (i.e. eviction has begun), so the caller can render a soft
         * fade instead of an abrupt cut for that line (AC-3c).
         *
         * No Context/View/I-O — directly JVM-testable (mirrors `RecordingMode.selectSilenceSecs`).
         */
        fun visibleLines(chunks: List<String>, maxLines: Int): List<Line> {
            require(maxLines > 0) { "maxLines must be positive (got $maxLines)" }
            if (chunks.isEmpty()) return emptyList()
            val start = (chunks.size - maxLines).coerceAtLeast(0)
            return chunks.subList(start, chunks.size).mapIndexed { i, text ->
                Line(text, isFading = i == 0 && start > 0)
            }
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
            renderRollingLines(value)
        }

    /**
     * Story 11-3 (AC-3, item 3): rebuilds the fixed [ROLLING_MAX_LINES]-slot line pool from the
     * accumulated preview text. [accumulated] is newline-joined by `KlarvoOverlayService.
     * appendPreviewText` (Task 4.2 — one `\n` per preview-flush chunk, replacing the pre-11-3
     * single-space join), so splitting on `\n` recovers the chunk list [visibleLines] expects.
     * No scrolling: unused trailing slots are simply left blank, so the fixed-height box never
     * grows or shrinks with content (replaces the pre-11-3 ScrollView/fullScroll auto-scroll).
     */
    private fun renderRollingLines(accumulated: String) {
        val chunks = if (accumulated.isBlank()) emptyList() else accumulated.split("\n")
        val visible = visibleLines(chunks, ROLLING_MAX_LINES)
        for (i in lineViews.indices) {
            val slot = lineViews[i]
            val line = visible.getOrNull(i)
            slot.text = line?.text ?: ""
            val targetAlpha = if (line?.isFading == true) ROLLING_FADE_ALPHA else 1f
            // P4 (11-3 review): epsilon comparison -- exact float equality can miss-fire (or
            // fail to fire) under residual animation float drift, re-triggering redundant
            // animations.
            if (kotlin.math.abs(slot.alpha - targetAlpha) > 0.01f) {
                slot.animate().cancel()
                slot.animate().alpha(targetAlpha).setDuration(ROLLING_FADE_MS).start()
            }
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
        lineViews.forEach {
            it.setTextColor(textColor)
            it.typeface = typeface
            it.textSize = sizeSp
        }
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
    // Story 11-3 (AC-3, item 3): fixed pool of ROLLING_MAX_LINES TextViews (replaces the pre-11-3
    // single transcriptTextView + ScrollView). Index 0 = oldest visible line (fades first).
    private val lineViews: List<TextView>
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

        // Transcript area: fixed pool of ROLLING_MAX_LINES line-TextViews (rolling window,
        // Story 11-3 item 3) + blinking amber caret, in a FrameLayout overlay. No ScrollView —
        // the box is fixed-height and never scrolls; content is bounded by visibleLines().
        // P2 (11-3 review): gravity BOTTOM so overflow (a long wrapped chunk, or the "large"
        // 18sp font) clips the TOP (oldest) line, never the bottom (newest) -- the rolling
        // model is newest-at-bottom, older rolls out the top.
        val linesContainer = LinearLayout(context).apply {
            orientation = VERTICAL
            gravity = Gravity.BOTTOM
        }
        lineViews = (0 until ROLLING_MAX_LINES).map {
            TextView(context).apply {
                setTextColor(KlarvoTheme.Muted)
                textSize = 15f  // sp — FONT_PX_SP "medium" default until applyAppearance runs
                typeface = Typeface.MONOSPACE
                setLineSpacing(0f, 1.7f)
                gravity = Gravity.TOP or Gravity.START
                text = ""
            }.also { line -> linesContainer.addView(line, LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            )) }
        }
        caretView = CaretView(context)

        val transcriptFrame = FrameLayout(context)
        transcriptFrame.addView(linesContainer, FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        ).apply { gravity = Gravity.BOTTOM })
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
        lineViews.forEach { it.setTextColor(color) }
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
