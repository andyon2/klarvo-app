package com.dikta.voice

import android.content.Context
import android.content.SharedPreferences
import android.graphics.Color
import android.graphics.PixelFormat
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.util.Log
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.widget.LinearLayout
import android.widget.SeekBar
import android.widget.TextView

/**
 * Manages the long-press settings overlay for the floating bubble.
 *
 * The overlay consists of two WindowManager layers:
 *  1. A transparent full-screen scrim that dismisses the menu on outside taps.
 *  2. A card-style menu showing size options (Mini/Small/Normal/Large) and an
 *     opacity slider.
 *
 * Lifecycle: call [show] to display, [dismiss] to remove. The menu is fully
 * self-contained -- it holds references to [windowManager] and [prefs] but does
 * not own the bubble view itself.
 *
 * @param context         Service context used to build views.
 * @param windowManager   WindowManager to add/remove overlay views.
 * @param overlayType     WindowManager overlay type (TYPE_APPLICATION_OVERLAY or legacy).
 * @param prefs           SharedPreferences for persisting size and opacity.
 * @param onSizeChanged   Invoked with the new size in dp when the user picks a size row.
 * @param onOpacityChanged Invoked with the new opacity (5..100) when the user lifts
 *                         their finger from the SeekBar; also called live during drag for
 *                         preview (fromUser=true, fingerDown=true).
 */
class BubbleSettingsMenu(
    private val context: Context,
    private val windowManager: WindowManager,
    private val overlayType: Int,
    private val prefs: SharedPreferences,
    private val onSizeChanged: (newSizeDp: Int) -> Unit,
    private val onOpacityChanged: (newOpacity: Int, committed: Boolean) -> Unit
) {

    companion object {
        private const val TAG = "BubbleSettingsMenu"
        private const val PREF_BUBBLE_SIZE = "bubble_size"
        private const val PREF_BUBBLE_OPACITY = "bubble_opacity"
    }

    /** True while the menu overlay is attached to the WindowManager. */
    val isVisible: Boolean
        get() = menuCardView != null

    private var menuCardView: View? = null
    private var scrimView: View? = null

    /**
     * Builds and attaches the settings menu next to the bubble.
     *
     * @param currentSizeDp   Currently active bubble size in dp.
     * @param currentOpacity  Currently active bubble opacity (5..100).
     * @param bubbleX         Bubble WindowManager x-offset in pixels.
     * @param bubbleY         Bubble WindowManager y-offset in pixels.
     * @param bubbleSizePx    Current bubble size in pixels (for positioning).
     * @param screenWidth     Full screen width in pixels (for left/right placement).
     */
    fun show(
        currentSizeDp: Int,
        currentOpacity: Int,
        bubbleX: Int,
        bubbleY: Int,
        bubbleSizePx: Int,
        screenWidth: Int
    ) {
        if (isVisible) return

        val dp = context.resources.displayMetrics.density

        // --- Scrim (transparent, catches outside taps) ---
        val scrim = View(context).apply {
            setBackgroundColor(Color.TRANSPARENT)
            setOnClickListener { dismiss() }
        }
        val scrimParams = WindowManager.LayoutParams(
            WindowManager.LayoutParams.MATCH_PARENT,
            WindowManager.LayoutParams.MATCH_PARENT,
            overlayType,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.TOP or Gravity.START
        }

        // --- Menu card ---
        val menuCard = buildMenuCard(dp, currentSizeDp, currentOpacity)

        // --- Positioning ---
        val menuWidthPx = (160 * dp).toInt()
        // Estimated height: 4 size rows (48dp each) + opacity section (88dp) + divider + padding
        val menuEstimatedHeightPx = (DiktaOverlayService.BUBBLE_SIZES_DP.size * 48 * dp + 100 * dp + 16 * dp).toInt()
        val margin8 = (8 * dp).toInt()

        val menuX = if (bubbleX > menuWidthPx + margin8) {
            bubbleX - menuWidthPx - margin8
        } else {
            bubbleX + bubbleSizePx + margin8
        }
        val menuY = (bubbleY + bubbleSizePx / 2 - menuEstimatedHeightPx / 2).coerceAtLeast(margin8)

        val menuParams = WindowManager.LayoutParams(
            menuWidthPx,
            WindowManager.LayoutParams.WRAP_CONTENT,
            overlayType,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.TOP or Gravity.START
            x = menuX
            y = menuY
        }

        // Add scrim first (lower z-order), menu card on top
        try {
            windowManager.addView(scrim, scrimParams)
            windowManager.addView(menuCard, menuParams)
            scrimView = scrim
            menuCardView = menuCard
        } catch (e: Exception) {
            Log.w(TAG, "Failed to attach settings menu to WindowManager", e)
            try { windowManager.removeView(scrim) } catch (ex: Exception) {
                Log.w(TAG, "Failed to remove scrim after attach error", ex)
            }
            scrimView = null
            menuCardView = null
        }
    }

    /** Removes both the menu card and its scrim from the WindowManager. */
    fun dismiss() {
        try {
            menuCardView?.let { windowManager.removeView(it) }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to remove menu card from WindowManager", e)
        }
        try {
            scrimView?.let { windowManager.removeView(it) }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to remove scrim from WindowManager", e)
        }
        menuCardView = null
        scrimView = null
    }

    // --- Private helpers ---

    private fun buildMenuCard(dp: Float, currentSizeDp: Int, currentOpacity: Int): LinearLayout {
        val menuCard = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            elevation = 12f * dp
            val padV = (8 * dp).toInt()
            setPadding(0, padV, 0, padV)
            background = GradientDrawable().apply {
                setColor(Color.WHITE)
                cornerRadius = 16f * dp
            }
        }

        // Size rows
        DiktaOverlayService.BUBBLE_SIZES_DP.forEachIndexed { index, sizeDp ->
            val row = buildSizeRow(
                label = DiktaOverlayService.BUBBLE_SIZE_LABELS[index],
                sizeDp = sizeDp,
                isSelected = sizeDp == currentSizeDp,
                dp = dp
            ) {
                onSizeChanged(sizeDp)
                prefs.edit().putInt(PREF_BUBBLE_SIZE, sizeDp).apply()
                dismiss()
            }
            menuCard.addView(row)
        }

        // Divider between size section and opacity section
        val divider = View(context).apply {
            val dividerHeight = (1 * dp).toInt()
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                dividerHeight
            ).also {
                val marginH = (16 * dp).toInt()
                val marginV = (4 * dp).toInt()
                it.setMargins(marginH, marginV, marginH, marginV)
            }
            setBackgroundColor(Color.parseColor("#E0E0E0"))
        }
        menuCard.addView(divider)

        // Opacity section
        menuCard.addView(buildOpacitySection(dp, currentOpacity))

        return menuCard
    }

    /**
     * Builds a single row in the size-selection list.
     * Shows the label, size hint, and a checkmark for the selected entry.
     */
    private fun buildSizeRow(
        label: String,
        sizeDp: Int,
        isSelected: Boolean,
        dp: Float,
        onClick: () -> Unit
    ): View {
        val row = LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            val padH = (16 * dp).toInt()
            val padV = (12 * dp).toInt()
            setPadding(padH, padV, padH, padV)
            isClickable = true
            isFocusable = true
            if (isSelected) setBackgroundColor(Color.parseColor("#F0F0F0"))
            setOnClickListener { onClick() }
        }

        val labelView = TextView(context).apply {
            text = label
            textSize = 15f
            setTextColor(if (isSelected) Color.parseColor("#1A1A1A") else Color.parseColor("#444444"))
            if (isSelected) setTypeface(null, Typeface.BOLD)
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }

        val sizeHint = TextView(context).apply {
            text = "${sizeDp}dp"
            textSize = 12f
            setTextColor(Color.parseColor("#999999"))
        }

        val checkView = TextView(context).apply {
            text = if (isSelected) "\u2713" else ""
            textSize = 16f
            setTextColor(Color.parseColor("#4CAF50"))
            val marginStart = (8 * dp).toInt()
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.WRAP_CONTENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.marginStart = marginStart }
        }

        row.addView(labelView)
        row.addView(sizeHint)
        row.addView(checkView)
        return row
    }

    /**
     * Builds the opacity control section.
     *
     * Layout:
     *   [Label: "Opacity"  |  current value: "75%"]
     *   [SeekBar  5..100 ]
     *
     * SeekBar progress range is 0..95 (offset +5 = displayed 5..100%).
     * Live changes fire [onOpacityChanged] with committed=false for preview.
     * On finger-lift (ACTION_STOP_TRACKING) fires with committed=true and
     * persists the value to SharedPreferences.
     */
    private fun buildOpacitySection(dp: Float, currentOpacity: Int): View {
        val section = LinearLayout(context).apply {
            orientation = LinearLayout.VERTICAL
            val padH = (16 * dp).toInt()
            val padTop = (4 * dp).toInt()
            val padBottom = (8 * dp).toInt()
            setPadding(padH, padTop, padH, padBottom)
        }

        val headerRow = LinearLayout(context).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }

        val opacityLabel = TextView(context).apply {
            text = "Opacity"
            textSize = 14f
            setTextColor(Color.parseColor("#444444"))
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }

        val opacityValueLabel = TextView(context).apply {
            text = "$currentOpacity%"
            textSize = 13f
            setTextColor(Color.parseColor("#666666"))
        }

        headerRow.addView(opacityLabel)
        headerRow.addView(opacityValueLabel)
        section.addView(headerRow)

        val seekBar = SeekBar(context).apply {
            max = 95  // 0 maps to 5%, 95 maps to 100%
            progress = currentOpacity - 5
            val marginTop = (4 * dp).toInt()
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.topMargin = marginTop }
        }

        seekBar.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(sb: SeekBar, progress: Int, fromUser: Boolean) {
                if (!fromUser) return
                val newOpacity = progress + 5
                opacityValueLabel.text = "$newOpacity%"
                // Live preview -- not yet committed
                onOpacityChanged(newOpacity, false)
            }

            override fun onStartTrackingTouch(sb: SeekBar) { /* nothing */ }

            override fun onStopTrackingTouch(sb: SeekBar) {
                val newOpacity = sb.progress + 5
                // Commit: persist and update bubble
                prefs.edit().putInt(PREF_BUBBLE_OPACITY, newOpacity).apply()
                onOpacityChanged(newOpacity, true)
            }
        })

        section.addView(seekBar)
        return section
    }
}
