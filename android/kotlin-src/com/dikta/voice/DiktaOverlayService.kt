package com.dikta.voice

import android.app.*
import android.content.*
import android.content.pm.ServiceInfo
import android.graphics.Color
import android.graphics.PixelFormat
import android.graphics.Typeface
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.*
import android.util.DisplayMetrics
import android.view.*
import android.widget.LinearLayout
import android.widget.SeekBar
import android.widget.TextView
import android.widget.Toast
import java.io.IOException
import kotlin.math.abs
import kotlin.math.sqrt

/**
 * Foreground Service that manages the floating bubble overlay.
 *
 * - Creates a WindowManager overlay with TYPE_APPLICATION_OVERLAY
 * - Detects keyboard visibility via a hidden sensor view (getWindowVisibleDisplayFrame)
 * - Shows/hides bubble when keyboard opens/closes -- works system-wide, no AccessibilityService needed
 * - Handles touch events: drag vs. tap detection (threshold: 10dp), long-press opens size menu
 * - Coordinates audio recording (AudioRecord), Groq STT, DeepSeek cleanup
 * - Saves result to history DB, pastes via AccessibilityService if available, clipboard fallback
 * - Runs as a foreground service with a persistent notification
 */
class DiktaOverlayService : Service() {

    companion object {
        private const val CHANNEL_ID = "dikta_overlay"
        private const val NOTIFICATION_ID = 1
        private const val PREFS_NAME = "dikta_bubble_prefs"
        private const val PREF_X = "bubble_x"
        private const val PREF_Y = "bubble_y"
        private const val PREF_BUBBLE_SIZE = "bubble_size"
        private const val PREF_BUBBLE_OPACITY = "bubble_opacity"
        private const val DEFAULT_BUBBLE_OPACITY = 100

        /** BroadcastReceiver action: tap on notification toggles bubble visibility. */
        const val ACTION_TOGGLE_BUBBLE = "com.dikta.voice.TOGGLE_BUBBLE"

        // Audio recording parameters
        private const val SAMPLE_RATE = 16000
        private const val CHANNEL_CONFIG = AudioFormat.CHANNEL_IN_MONO
        private const val AUDIO_FORMAT = AudioFormat.ENCODING_PCM_16BIT

        // Keyboard detection: if >15% of screen height is hidden, keyboard is likely open
        private const val KEYBOARD_HEIGHT_RATIO = 0.15f
        // Polling interval for keyboard detection (ms)
        private const val KEYBOARD_CHECK_INTERVAL = 300L

        // Long-press threshold in milliseconds
        private const val LONG_PRESS_TIMEOUT_MS = 500L

        // Available bubble sizes in dp
        val BUBBLE_SIZES_DP = intArrayOf(32, 44, 56, 72)
        val BUBBLE_SIZE_LABELS = arrayOf("Mini", "Small", "Normal", "Large")
        const val DEFAULT_BUBBLE_SIZE_DP = 56

        /** Live reference used by DiktaAccessibilityService for paste. */
        var instance: DiktaOverlayService? = null
    }

    private enum class RecordingState { IDLE, RECORDING, PROCESSING }

    private val handler = Handler(Looper.getMainLooper())
    private lateinit var windowManager: WindowManager
    private lateinit var bubbleView: FloatingBubbleView
    private lateinit var bubbleParams: WindowManager.LayoutParams
    private var overlayType = 0

    private var currentState = RecordingState.IDLE

    /** Tracks whether the bubble view is currently attached to WindowManager. */
    private var isBubbleVisible = false

    // Keyboard detection
    private var keyboardVisible = false

    // Audio
    private var audioRecord: AudioRecord? = null
    private val pcmBuffer = ArrayList<Short>()
    private var recordingThread: Thread? = null
    private var isCapturing = false

    // Touch handling
    private var dragTouchStartX = 0f
    private var dragTouchStartY = 0f
    private var bubbleStartX = 0
    private var bubbleStartY = 0
    private var isDragging = false
    private var dragThresholdPx = 0f

    // Bubble opacity (5..100). Applied to bubbleView.alpha when state is IDLE.
    // During RECORDING / PROCESSING the bubble is always fully opaque so the user can see status.
    private var bubbleOpacity = DEFAULT_BUBBLE_OPACITY

    // Long-press detection
    private var longPressTriggered = false
    private val longPressRunnable = Runnable {
        // Only open menu if the user hasn't started dragging
        if (!isDragging) {
            longPressTriggered = true
            showSizeMenu()
        }
    }

    // Size menu overlay (null when not shown)
    private var sizeMenuView: View? = null
    private var sizeMenuParams: WindowManager.LayoutParams? = null

    /**
     * Receives ACTION_TOGGLE_BUBBLE from the foreground notification's contentIntent.
     * Registered/unregistered dynamically so no manifest entry is needed.
     */
    private val toggleBubbleReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == ACTION_TOGGLE_BUBBLE) {
                toggleBubble()
            }
        }
    }

    private val keyboardCheckRunnable = object : Runnable {
        override fun run() {
            checkKeyboardVisibility()
            handler.postDelayed(this, KEYBOARD_CHECK_INTERVAL)
        }
    }

    override fun onCreate() {
        super.onCreate()
        instance = this
        dragThresholdPx = 10f * resources.displayMetrics.density

        overlayType = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
        } else {
            @Suppress("DEPRECATION")
            WindowManager.LayoutParams.TYPE_PHONE
        }

        createNotificationChannel()
        startForegroundWithNotification()

        // Register toggle receiver -- no manifest entry needed for dynamic receivers.
        val filter = IntentFilter(ACTION_TOGGLE_BUBBLE)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(toggleBubbleReceiver, filter, RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(toggleBubbleReceiver, filter)
        }

        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager

        setupBubble()
        setupKeyboardDetector()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        return START_STICKY
    }

    override fun onDestroy() {
        instance = null
        handler.removeCallbacks(keyboardCheckRunnable)
        handler.removeCallbacks(longPressRunnable)
        try { unregisterReceiver(toggleBubbleReceiver) } catch (_: Exception) {}
        dismissSizeMenu()
        super.onDestroy()
        stopCapture()
        if (::bubbleView.isInitialized && isBubbleVisible) {
            try { windowManager.removeView(bubbleView) } catch (_: Exception) {}
            isBubbleVisible = false
        }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    // --- Notification ---

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "Dikta Overlay",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Keeps the Dikta voice bubble visible"
                setShowBadge(false)
            }
            val nm = getSystemService(NotificationManager::class.java)
            nm.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(): Notification {
        val statusText = if (isBubbleVisible) "Tap to hide bubble" else "Tap to show bubble"

        // PendingIntent that sends ACTION_TOGGLE_BUBBLE to our dynamic receiver.
        val toggleIntent = Intent(ACTION_TOGGLE_BUBBLE).apply {
            setPackage(packageName)
        }
        val pendingToggle = PendingIntent.getBroadcast(
            this,
            0,
            toggleIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
        return builder
            .setContentTitle("Dikta - Voice Dictation")
            .setContentText(statusText)
            .setSmallIcon(android.R.drawable.ic_btn_speak_now)
            .setContentIntent(pendingToggle)
            .setOngoing(true)
            .build()
    }

    /**
     * Rebuilds and re-posts the notification to reflect the current bubble visibility.
     * Called after showBubble() / hideBubble() / toggleBubble().
     */
    private fun updateNotification() {
        val nm = getSystemService(NotificationManager::class.java)
        nm.notify(NOTIFICATION_ID, buildNotification())
    }

    private fun startForegroundWithNotification() {
        val notification = buildNotification()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(
                NOTIFICATION_ID,
                notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    // --- Keyboard detection via InputMethodManager ---

    /**
     * Starts polling InputMethodManager.getInputMethodWindowVisibleHeight() via reflection.
     * This internal Android API returns the keyboard height in pixels (0 = hidden).
     * Works system-wide from a Service context, no AccessibilityService needed.
     */
    private fun setupKeyboardDetector() {
        // No sensor view needed -- we query InputMethodManager directly.
        handler.post(keyboardCheckRunnable)
    }

    private fun checkKeyboardVisibility() {
        try {
            val imm = getSystemService(INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager
            val method = imm.javaClass.getMethod("getInputMethodWindowVisibleHeight")
            val height = method.invoke(imm) as Int
            val isKeyboardOpen = height > 0

            if (isKeyboardOpen != keyboardVisible) {
                keyboardVisible = isKeyboardOpen
                if (isKeyboardOpen) {
                    showBubble()
                } else if (currentState == RecordingState.IDLE) {
                    hideBubble()
                }
            }
        } catch (_: Exception) {
            // Reflection failed -- keep bubble in current state
        }
    }

    /**
     * Toggles bubble visibility.
     * Called by the BroadcastReceiver when the user taps the foreground notification.
     * Always runs on a background thread (BroadcastReceiver.onReceive), so we post to main.
     */
    private fun toggleBubble() {
        handler.post {
            if (isBubbleVisible) hideBubble() else showBubble()
        }
    }

    private fun showBubble() {
        if (!isBubbleVisible && ::bubbleView.isInitialized) {
            try {
                windowManager.addView(bubbleView, bubbleParams)
                isBubbleVisible = true
                updateNotification()
            } catch (_: Exception) {}
        }
    }

    private fun hideBubble() {
        if (isBubbleVisible && ::bubbleView.isInitialized) {
            try {
                windowManager.removeView(bubbleView)
                isBubbleVisible = false
                updateNotification()
            } catch (_: Exception) {}
        }
    }

    // --- Bubble setup ---

    /**
     * Prepares the bubble view and layout params. Bubble starts HIDDEN;
     * it appears when the keyboard detector sees the soft keyboard open.
     * Restores previously saved size from SharedPreferences.
     */
    private fun setupBubble() {
        bubbleView = FloatingBubbleView(this)

        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val savedSizeDp = prefs.getInt(PREF_BUBBLE_SIZE, DEFAULT_BUBBLE_SIZE_DP)
        bubbleOpacity = prefs.getInt(PREF_BUBBLE_OPACITY, DEFAULT_BUBBLE_OPACITY)

        // Apply saved size to the view before first layout
        bubbleView.setBubbleSize(savedSizeDp)

        // Apply saved opacity (only while IDLE -- recording/processing always fully opaque)
        bubbleView.alpha = bubbleOpacity / 100f

        val (screenW, screenH) = getScreenDimensions()
        val dp = resources.displayMetrics.density
        val bubbleSizePx = (savedSizeDp * dp).toInt()
        val marginPx = (16 * dp).toInt()

        val savedX = prefs.getInt(PREF_X, screenW - bubbleSizePx - marginPx)
        val savedY = prefs.getInt(PREF_Y, screenH / 2)

        bubbleParams = WindowManager.LayoutParams(
            WindowManager.LayoutParams.WRAP_CONTENT,
            WindowManager.LayoutParams.WRAP_CONTENT,
            overlayType,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN,
            PixelFormat.TRANSLUCENT
        ).apply {
            gravity = Gravity.TOP or Gravity.START
            x = savedX
            y = savedY
        }

        bubbleView.setOnTouchListener { _, event -> handleTouch(event) }

        // Bubble starts hidden -- keyboard detector will show it.
        isBubbleVisible = false
    }

    private fun getScreenDimensions(): Pair<Int, Int> {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            val metrics = windowManager.currentWindowMetrics
            val bounds = metrics.bounds
            Pair(bounds.width(), bounds.height())
        } else {
            val dm = DisplayMetrics()
            @Suppress("DEPRECATION")
            windowManager.defaultDisplay.getRealMetrics(dm)
            Pair(dm.widthPixels, dm.heightPixels)
        }
    }

    // --- Touch handling ---

    private fun handleTouch(event: MotionEvent): Boolean {
        when (event.action) {
            MotionEvent.ACTION_DOWN -> {
                dragTouchStartX = event.rawX
                dragTouchStartY = event.rawY
                bubbleStartX = bubbleParams.x
                bubbleStartY = bubbleParams.y
                isDragging = false
                longPressTriggered = false
                // Schedule long-press detection
                handler.postDelayed(longPressRunnable, LONG_PRESS_TIMEOUT_MS)
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                val dx = event.rawX - dragTouchStartX
                val dy = event.rawY - dragTouchStartY
                if (!isDragging && (abs(dx) > dragThresholdPx || abs(dy) > dragThresholdPx)) {
                    isDragging = true
                    // Cancel long-press if the user starts dragging
                    handler.removeCallbacks(longPressRunnable)
                }
                if (isDragging) {
                    bubbleParams.x = (bubbleStartX + dx).toInt()
                    bubbleParams.y = (bubbleStartY + dy).toInt()
                    windowManager.updateViewLayout(bubbleView, bubbleParams)
                }
                return true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                handler.removeCallbacks(longPressRunnable)
                if (event.action == MotionEvent.ACTION_UP) {
                    if (isDragging) {
                        savePosition(bubbleParams.x, bubbleParams.y)
                    } else if (!longPressTriggered) {
                        // Only fire tap if long-press menu was NOT shown
                        handleTap()
                    }
                }
                return true
            }
        }
        return false
    }

    private fun savePosition(x: Int, y: Int) {
        getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
            .putInt(PREF_X, x)
            .putInt(PREF_Y, y)
            .apply()
    }

    // --- State machine ---

    private fun handleTap() {
        // If the size menu is open, a tap outside it dismisses it (handled by the menu's own touch).
        // Taps on the bubble itself while menu is open should dismiss the menu but not start recording.
        if (sizeMenuView != null) {
            dismissSizeMenu()
            return
        }
        when (currentState) {
            RecordingState.IDLE -> startRecording()
            RecordingState.RECORDING -> stopRecording()
            RecordingState.PROCESSING -> { /* ignore taps while processing */ }
        }
    }

    // --- Size menu ---

    /**
     * Builds and attaches a size-selection overlay next to the bubble.
     *
     * The menu is a vertical LinearLayout with 4 rows (Mini / Small / Normal / Large).
     * Each row is a TextView. The currently active size is highlighted.
     * A transparent full-screen "scrim" view underneath handles outside-tap dismissal.
     *
     * Both scrim and menu are added as separate WindowManager overlays so they work
     * from a Service context (no Activity window anchor needed).
     */
    private fun showSizeMenu() {
        if (sizeMenuView != null) return  // Already visible

        val dp = resources.displayMetrics.density
        val currentSizeDp = bubbleView.getBubbleSizeDp()

        // --- Transparent full-screen scrim to catch outside taps ---
        val scrim = View(this).apply {
            setBackgroundColor(Color.TRANSPARENT)
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
        scrim.setOnClickListener { dismissSizeMenu() }

        // --- Menu card ---
        val menuCard = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.WHITE)
            // Rounded-corner background: draw as a card with shadow via elevation
            elevation = 12f * dp
            // Padding: 8dp top/bottom, 0 left/right (rows have their own horizontal padding)
            val padV = (8 * dp).toInt()
            setPadding(0, padV, 0, padV)
        }

        // Set a rounded background on the card via a programmatic drawable
        val cardBackground = android.graphics.drawable.GradientDrawable().apply {
            setColor(Color.WHITE)
            cornerRadius = 16f * dp
        }
        menuCard.background = cardBackground

        BUBBLE_SIZES_DP.forEachIndexed { index, sizeDp ->
            val row = buildMenuRow(
                label = BUBBLE_SIZE_LABELS[index],
                sizeDp = sizeDp,
                isSelected = sizeDp == currentSizeDp,
                dp = dp
            ) {
                applyBubbleSize(sizeDp)
                dismissSizeMenu()
            }
            menuCard.addView(row)
        }

        // --- Divider between size section and opacity section ---
        val divider = View(this).apply {
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

        // --- Opacity section ---
        menuCard.addView(buildOpacitySection(dp))

        // --- Position the menu card ---
        // Prefer left of bubble; if bubble is near left edge, place right of it.
        val menuWidthPx = (160 * dp).toInt()
        // Size rows + opacity section (label row ~36dp + seekbar row ~52dp) + divider + card padding
        val menuEstimatedHeightPx = (BUBBLE_SIZES_DP.size * 48 * dp + 100 * dp + 16 * dp).toInt()
        val bubbleSizePx = (currentSizeDp * dp).toInt()

        // Place menu to the left if there is room (> menuWidth + 8dp margin from left edge)
        val menuX: Int
        val menuY: Int
        val margin8 = (8 * dp).toInt()

        val (screenW, _) = getScreenDimensions()
        menuX = if (bubbleParams.x > menuWidthPx + margin8) {
            // Enough space to the left
            bubbleParams.x - menuWidthPx - margin8
        } else {
            // Place to the right of the bubble
            bubbleParams.x + bubbleSizePx + margin8
        }
        // Vertically: center the menu with the bubble, clamped to screen
        menuY = (bubbleParams.y + bubbleSizePx / 2 - menuEstimatedHeightPx / 2)
            .coerceAtLeast(margin8)

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

        // Add scrim first (lower z-order), then menu card on top
        try {
            windowManager.addView(scrim, scrimParams)
            windowManager.addView(menuCard, menuParams)
            sizeMenuView = menuCard
            sizeMenuParams = menuParams
            // Keep a reference to the scrim for cleanup
            menuCard.tag = scrim
        } catch (e: Exception) {
            // If adding fails (e.g. permission revoked), clean up gracefully
            try { windowManager.removeView(scrim) } catch (_: Exception) {}
            sizeMenuView = null
            sizeMenuParams = null
        }
    }

    /**
     * Builds a single row in the size menu.
     * Shows the label and a check mark if this size is currently selected.
     */
    private fun buildMenuRow(
        label: String,
        sizeDp: Int,
        isSelected: Boolean,
        dp: Float,
        onClick: () -> Unit
    ): View {
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            val padH = (16 * dp).toInt()
            val padV = (12 * dp).toInt()
            setPadding(padH, padV, padH, padV)
            isClickable = true
            isFocusable = true
            // Highlight selected row
            if (isSelected) {
                setBackgroundColor(Color.parseColor("#F0F0F0"))
            }
            setOnClickListener { onClick() }
        }

        // Label text (e.g. "Normal")
        val labelView = TextView(this).apply {
            text = label
            textSize = 15f
            setTextColor(if (isSelected) Color.parseColor("#1A1A1A") else Color.parseColor("#444444"))
            if (isSelected) setTypeface(null, Typeface.BOLD)
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }

        // Size hint text (e.g. "56dp")
        val sizeHint = TextView(this).apply {
            text = "${sizeDp}dp"
            textSize = 12f
            setTextColor(Color.parseColor("#999999"))
        }

        // Checkmark indicator (Unicode heavy check mark)
        val checkView = TextView(this).apply {
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
     * Builds the opacity control section for the long-press menu.
     *
     * Layout (inside a vertical LinearLayout):
     *   [Label: "Opacity"  |  current value: "75%"]
     *   [SeekBar  5..100 ]
     *
     * The SeekBar reports values 0..95 internally (offset by 5) so the minimum
     * displayed opacity is 5% and the maximum is 100%.
     * Live changes are applied immediately via bubbleView.alpha; the final value
     * is persisted to SharedPreferences on ACTION_STOP_TRACKING (finger lifted).
     */
    private fun buildOpacitySection(dp: Float): View {
        val section = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val padH = (16 * dp).toInt()
            val padTop = (4 * dp).toInt()
            val padBottom = (8 * dp).toInt()
            setPadding(padH, padTop, padH, padBottom)
        }

        // Header row: label on the left, current value on the right
        val headerRow = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
        }

        val opacityLabel = TextView(this).apply {
            text = "Opacity"
            textSize = 14f
            setTextColor(Color.parseColor("#444444"))
            layoutParams = LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f)
        }

        val opacityValueLabel = TextView(this).apply {
            text = "$bubbleOpacity%"
            textSize = 13f
            setTextColor(Color.parseColor("#666666"))
        }

        headerRow.addView(opacityLabel)
        headerRow.addView(opacityValueLabel)
        section.addView(headerRow)

        // SeekBar: internally 0..95, displayed as 5..100
        val seekBar = SeekBar(this).apply {
            max = 95  // 0 maps to 5%, 95 maps to 100%
            progress = bubbleOpacity - 5
            val marginTop = (4 * dp).toInt()
            layoutParams = LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT,
                LinearLayout.LayoutParams.WRAP_CONTENT
            ).also { it.topMargin = marginTop }
        }

        seekBar.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(sb: SeekBar, progress: Int, fromUser: Boolean) {
                if (!fromUser) return
                val newOpacity = progress + 5  // clamp to 5..100
                opacityValueLabel.text = "$newOpacity%"
                // Live preview: apply to bubble immediately (only when IDLE to avoid
                // overriding the full-opacity enforcement during recording/processing)
                if (currentState == RecordingState.IDLE) {
                    bubbleView.alpha = newOpacity / 100f
                }
            }

            override fun onStartTrackingTouch(sb: SeekBar) { /* nothing */ }

            override fun onStopTrackingTouch(sb: SeekBar) {
                // Finger lifted -- commit the new value
                val newOpacity = sb.progress + 5
                bubbleOpacity = newOpacity
                bubbleView.alpha = if (currentState == RecordingState.IDLE) {
                    newOpacity / 100f
                } else {
                    1.0f
                }
                getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
                    .putInt(PREF_BUBBLE_OPACITY, newOpacity)
                    .apply()
            }
        })

        section.addView(seekBar)
        return section
    }

    /** Removes both the menu card and its scrim from the WindowManager. */
    private fun dismissSizeMenu() {
        val menu = sizeMenuView ?: return
        val scrim = menu.tag as? View
        try { windowManager.removeView(menu) } catch (_: Exception) {}
        try { scrim?.let { windowManager.removeView(it) } } catch (_: Exception) {}
        sizeMenuView = null
        sizeMenuParams = null
    }

    /**
     * Applies a new bubble size:
     * 1. Updates FloatingBubbleView internal size
     * 2. Updates WindowManager LayoutParams (WRAP_CONTENT stays, view reports new measure)
     * 3. Adjusts bubble position so the center stays stable
     * 4. Persists the new size to SharedPreferences
     */
    private fun applyBubbleSize(newSizeDp: Int) {
        val dp = resources.displayMetrics.density
        val oldSizePx = (bubbleView.getBubbleSizeDp() * dp).toInt()
        val newSizePx = (newSizeDp * dp).toInt()

        // Keep the bubble center at the same screen position
        val centerX = bubbleParams.x + oldSizePx / 2
        val centerY = bubbleParams.y + oldSizePx / 2
        bubbleParams.x = centerX - newSizePx / 2
        bubbleParams.y = centerY - newSizePx / 2

        // Update the view (triggers requestLayout + invalidate inside setBubbleSize)
        bubbleView.setBubbleSize(newSizeDp)

        // Push new LayoutParams to WindowManager
        if (isBubbleVisible) {
            try {
                windowManager.updateViewLayout(bubbleView, bubbleParams)
            } catch (_: Exception) {}
        }

        // Persist position (shifted to keep center) and new size
        getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
            .putInt(PREF_BUBBLE_SIZE, newSizeDp)
            .putInt(PREF_X, bubbleParams.x)
            .putInt(PREF_Y, bubbleParams.y)
            .apply()
    }

    // --- Audio recording ---

    private fun startRecording() {
        val minBufSize = AudioRecord.getMinBufferSize(SAMPLE_RATE, CHANNEL_CONFIG, AUDIO_FORMAT)
        if (minBufSize == AudioRecord.ERROR || minBufSize == AudioRecord.ERROR_BAD_VALUE) {
            showToast("Cannot start recording: AudioRecord error")
            return
        }

        val bufferSize = maxOf(minBufSize, 8192)

        val recorder = AudioRecord(
            MediaRecorder.AudioSource.MIC,
            SAMPLE_RATE,
            CHANNEL_CONFIG,
            AUDIO_FORMAT,
            bufferSize
        )

        if (recorder.state != AudioRecord.STATE_INITIALIZED) {
            recorder.release()
            showToast("Microphone not available")
            return
        }

        audioRecord = recorder
        pcmBuffer.clear()
        isCapturing = true

        setState(RecordingState.RECORDING)
        recorder.startRecording()

        recordingThread = Thread {
            val buf = ShortArray(bufferSize / 2)
            while (isCapturing) {
                val read = recorder.read(buf, 0, buf.size)
                if (read > 0) {
                    for (i in 0 until read) {
                        pcmBuffer.add(buf[i])
                    }
                    // Calculate RMS amplitude for waveform visualization
                    val rms = calculateRms(buf, read)
                    val normalizedAmp = (rms / 32768f).coerceIn(0f, 1f)
                    handler.post { bubbleView.amplitude = normalizedAmp }
                }
            }
        }.also { it.start() }
    }

    private fun stopRecording() {
        isCapturing = false
        recordingThread?.join(500)
        recordingThread = null

        val recorder = audioRecord
        audioRecord = null
        recorder?.stop()
        recorder?.release()

        val pcmData = pcmBuffer.toShortArray()
        pcmBuffer.clear()

        setState(RecordingState.PROCESSING)

        // Run API calls off the main thread
        Thread { processAudio(pcmData) }.start()
    }

    private fun stopCapture() {
        isCapturing = false
        recordingThread?.interrupt()
        recordingThread = null
        audioRecord?.stop()
        audioRecord?.release()
        audioRecord = null
    }

    // --- API pipeline ---

    private fun processAudio(pcmData: ShortArray) {
        if (pcmData.isEmpty()) {
            handler.post {
                showToast("No audio recorded")
                setState(RecordingState.IDLE)
            }
            return
        }

        val config = DiktaApi.readConfig(this)
        if (config == null || config.groqApiKey.isBlank()) {
            handler.post {
                showToast("No API keys configured. Please open Dikta and add your Groq key in Settings.")
                setState(RecordingState.IDLE)
            }
            return
        }

        try {
            val wavBytes = encodeWav(pcmData, SAMPLE_RATE)

            // Step 1: STT via Groq Whisper
            val transcript = DiktaApi.transcribe(wavBytes, config.groqApiKey, config.language)

            if (transcript.isBlank()) {
                handler.post {
                    showToast("No speech detected")
                    setState(RecordingState.IDLE)
                }
                return
            }

            // Step 2: Text cleanup via DeepSeek (optional -- skip if no key)
            val finalText = if (config.deepseekApiKey.isNotBlank()) {
                try {
                    DiktaApi.cleanup(transcript, config.deepseekApiKey, config.cleanupStyle)
                } catch (e: IOException) {
                    // Cleanup failed -- use raw transcript
                    transcript
                }
            } else {
                transcript
            }

            // Step 3: Save to history DB (best-effort, runs on current background thread)
            DiktaApi.saveToHistory(
                context = this,
                finalText = finalText,
                rawText = transcript,
                style = config.cleanupStyle,
                language = config.language,
                deviceId = config.deviceId
            )

            // Step 3b: Push unsynced entries to Turso (best-effort, same background thread)
            DiktaApi.pushToTurso(this, config.tursoUrl, config.tursoToken)

            // Step 4: Copy to clipboard and paste via AccessibilityService if available
            handler.post {
                copyToClipboard(finalText)

                // Try direct paste into the focused text field
                val pasted = DiktaAccessibilityService.instance != null
                DiktaAccessibilityService.instance?.pasteIntoFocusedField()

                val preview = if (finalText.length > 50) finalText.take(50) + "..." else finalText
                if (pasted) {
                    showToast("Inserted: $preview")
                } else {
                    showToast("Copied: $preview")
                }
                setState(RecordingState.IDLE)
            }

        } catch (e: IOException) {
            handler.post {
                showToast("Error: ${e.message?.take(80)}")
                setState(RecordingState.IDLE)
            }
        }
    }

    // --- Helpers ---

    private fun setState(newState: RecordingState) {
        currentState = newState
        bubbleView.state = when (newState) {
            RecordingState.IDLE -> FloatingBubbleView.State.IDLE
            RecordingState.RECORDING -> FloatingBubbleView.State.RECORDING
            RecordingState.PROCESSING -> FloatingBubbleView.State.PROCESSING
        }
        // During active states the bubble must be fully visible so the user can see status.
        // Restore configured opacity only when returning to IDLE.
        bubbleView.alpha = when (newState) {
            RecordingState.IDLE -> bubbleOpacity / 100f
            RecordingState.RECORDING, RecordingState.PROCESSING -> 1.0f
        }
        if (newState == RecordingState.IDLE) {
            bubbleView.amplitude = 0f
        }
    }

    private fun copyToClipboard(text: String) {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val clip = ClipData.newPlainText("Dikta transcription", text)
        clipboard.setPrimaryClip(clip)
    }

    private fun showToast(message: String) {
        Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
    }

    private fun calculateRms(buffer: ShortArray, length: Int): Float {
        if (length == 0) return 0f
        var sum = 0.0
        for (i in 0 until length) {
            sum += buffer[i].toDouble() * buffer[i].toDouble()
        }
        return sqrt(sum / length).toFloat()
    }
}
