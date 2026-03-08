package com.dikta.voice

import android.app.*
import android.content.*
import android.content.pm.ServiceInfo
import android.graphics.PixelFormat
import android.os.*
import android.util.DisplayMetrics
import android.util.Log
import android.view.*
import android.widget.Toast
import java.io.IOException
import kotlin.math.abs

/**
 * Foreground Service that manages the floating bubble overlay.
 *
 * - Creates a WindowManager overlay with TYPE_APPLICATION_OVERLAY
 * - Detects keyboard visibility via InputMethodManager.getInputMethodWindowVisibleHeight()
 *   (reflection). Works system-wide; no AccessibilityService needed.
 * - Shows/hides bubble when keyboard opens/closes
 * - Handles touch events: drag vs. tap (threshold: 10dp), long-press opens settings menu
 * - Delegates audio capture to [DiktaAudioRecorder]
 * - Delegates size/opacity menu to [BubbleSettingsMenu]
 * - Coordinates Groq STT + DeepSeek cleanup pipeline via [DiktaApi]
 * - Saves result to history DB, pastes via [DiktaAccessibilityService] if available
 * - Runs as a foreground service with a persistent notification
 */
class DiktaOverlayService : Service() {

    companion object {
        private const val TAG = "DiktaOverlayService"

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

        // Keyboard detection: poll InputMethodManager at this interval (ms)
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
    private var audioRecorder: DiktaAudioRecorder? = null

    // Touch handling
    private var dragTouchStartX = 0f
    private var dragTouchStartY = 0f
    private var bubbleStartX = 0
    private var bubbleStartY = 0
    private var isDragging = false
    private var dragThresholdPx = 0f

    // Bubble opacity (5..100). Applied to bubbleView.alpha when state is IDLE.
    // During RECORDING / PROCESSING the bubble is always fully opaque.
    private var bubbleOpacity = DEFAULT_BUBBLE_OPACITY

    // Long-press detection
    private var longPressTriggered = false
    private val longPressRunnable = Runnable {
        if (!isDragging) {
            longPressTriggered = true
            showSizeMenu()
        }
    }

    // Settings menu (null when not visible)
    private var settingsMenu: BubbleSettingsMenu? = null

    /**
     * Receives ACTION_TOGGLE_BUBBLE from the foreground notification's contentIntent.
     * Registered/unregistered dynamically -- no manifest entry needed.
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
        try {
            unregisterReceiver(toggleBubbleReceiver)
        } catch (e: Exception) {
            Log.w(TAG, "Failed to unregister toggleBubbleReceiver (already unregistered?)", e)
        }
        settingsMenu?.dismiss()
        audioRecorder?.releaseImmediately()
        audioRecorder = null
        super.onDestroy()
        if (::bubbleView.isInitialized && isBubbleVisible) {
            try {
                windowManager.removeView(bubbleView)
            } catch (e: Exception) {
                Log.w(TAG, "Failed to remove bubbleView on destroy", e)
            }
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

        val toggleIntent = Intent(ACTION_TOGGLE_BUBBLE).apply { setPackage(packageName) }
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
        } catch (e: Exception) {
            // Reflection failed on this ROM variant -- bubble stays in current state.
            // Not logged at warn level because this can fire hundreds of times per session.
            Log.d(TAG, "getInputMethodWindowVisibleHeight reflection failed", e)
        }
    }

    /**
     * Toggles bubble visibility.
     * Called by the BroadcastReceiver when the user taps the foreground notification.
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
            } catch (e: Exception) {
                Log.w(TAG, "Failed to add bubbleView to WindowManager", e)
            }
        }
    }

    private fun hideBubble() {
        if (isBubbleVisible && ::bubbleView.isInitialized) {
            try {
                windowManager.removeView(bubbleView)
                isBubbleVisible = false
                updateNotification()
            } catch (e: Exception) {
                Log.w(TAG, "Failed to remove bubbleView from WindowManager", e)
            }
        }
    }

    // --- Bubble setup ---

    /**
     * Prepares the bubble view and layout params. Bubble starts HIDDEN;
     * it appears when the keyboard detector sees the soft keyboard open.
     * Restores previously saved size and opacity from SharedPreferences.
     */
    private fun setupBubble() {
        bubbleView = FloatingBubbleView(this)

        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val savedSizeDp = prefs.getInt(PREF_BUBBLE_SIZE, DEFAULT_BUBBLE_SIZE_DP)
        bubbleOpacity = prefs.getInt(PREF_BUBBLE_OPACITY, DEFAULT_BUBBLE_OPACITY)

        bubbleView.setBubbleSize(savedSizeDp)
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
                handler.postDelayed(longPressRunnable, LONG_PRESS_TIMEOUT_MS)
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                val dx = event.rawX - dragTouchStartX
                val dy = event.rawY - dragTouchStartY
                if (!isDragging && (abs(dx) > dragThresholdPx || abs(dy) > dragThresholdPx)) {
                    isDragging = true
                    handler.removeCallbacks(longPressRunnable)
                }
                if (isDragging) {
                    bubbleParams.x = (bubbleStartX + dx).toInt()
                    bubbleParams.y = (bubbleStartY + dy).toInt()
                    try {
                        windowManager.updateViewLayout(bubbleView, bubbleParams)
                    } catch (e: Exception) {
                        Log.w(TAG, "Failed to update bubble position during drag", e)
                    }
                }
                return true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                handler.removeCallbacks(longPressRunnable)
                if (event.action == MotionEvent.ACTION_UP) {
                    if (isDragging) {
                        savePosition(bubbleParams.x, bubbleParams.y)
                    } else if (!longPressTriggered) {
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
        // If the settings menu is open, a tap on the bubble dismisses it but does not
        // start recording (scrim handles outside taps via its own click listener).
        if (settingsMenu?.isVisible == true) {
            settingsMenu?.dismiss()
            return
        }
        when (currentState) {
            RecordingState.IDLE -> startRecording()
            RecordingState.RECORDING -> stopRecording()
            RecordingState.PROCESSING -> { /* ignore taps while processing */ }
        }
    }

    // --- Settings menu ---

    private fun showSizeMenu() {
        if (settingsMenu?.isVisible == true) return

        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val (screenW, _) = getScreenDimensions()
        val dp = resources.displayMetrics.density
        val bubbleSizePx = (bubbleView.getBubbleSizeDp() * dp).toInt()

        settingsMenu = BubbleSettingsMenu(
            context = this,
            windowManager = windowManager,
            overlayType = overlayType,
            prefs = prefs,
            onSizeChanged = { newSizeDp -> applyBubbleSize(newSizeDp) },
            onOpacityChanged = { newOpacity, committed ->
                if (committed) {
                    bubbleOpacity = newOpacity
                }
                if (currentState == RecordingState.IDLE) {
                    bubbleView.alpha = newOpacity / 100f
                }
                // If not committed (live drag preview): alpha is applied above.
                // On commit the field is also updated so it survives state transitions.
            }
        )

        settingsMenu?.show(
            currentSizeDp = bubbleView.getBubbleSizeDp(),
            currentOpacity = bubbleOpacity,
            bubbleX = bubbleParams.x,
            bubbleY = bubbleParams.y,
            bubbleSizePx = bubbleSizePx,
            screenWidth = screenW
        )
    }

    /**
     * Applies a new bubble size:
     * 1. Updates FloatingBubbleView internal size
     * 2. Adjusts bubble position so the center stays stable
     * 3. Pushes updated LayoutParams to WindowManager
     * 4. Persists position and size to SharedPreferences
     */
    private fun applyBubbleSize(newSizeDp: Int) {
        val dp = resources.displayMetrics.density
        val oldSizePx = (bubbleView.getBubbleSizeDp() * dp).toInt()
        val newSizePx = (newSizeDp * dp).toInt()

        val centerX = bubbleParams.x + oldSizePx / 2
        val centerY = bubbleParams.y + oldSizePx / 2
        bubbleParams.x = centerX - newSizePx / 2
        bubbleParams.y = centerY - newSizePx / 2

        bubbleView.setBubbleSize(newSizeDp)

        if (isBubbleVisible) {
            try {
                windowManager.updateViewLayout(bubbleView, bubbleParams)
            } catch (e: Exception) {
                Log.w(TAG, "Failed to update bubble layout after size change", e)
            }
        }

        getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
            .putInt(PREF_BUBBLE_SIZE, newSizeDp)
            .putInt(PREF_X, bubbleParams.x)
            .putInt(PREF_Y, bubbleParams.y)
            .apply()
    }

    // --- Audio recording ---

    private fun startRecording() {
        val recorder = DiktaAudioRecorder { amplitude ->
            // Called on the recording thread -- post to main for UI update
            handler.post { bubbleView.amplitude = amplitude }
        }

        try {
            recorder.start()
        } catch (e: IllegalStateException) {
            Log.w(TAG, "Failed to start audio recording", e)
            showToast("Cannot start recording: ${e.message}")
            return
        }

        audioRecorder = recorder
        setState(RecordingState.RECORDING)
    }

    private fun stopRecording() {
        val recorder = audioRecorder ?: return
        audioRecorder = null

        setState(RecordingState.PROCESSING)

        Thread {
            val wavBytes = recorder.stop()
            processAudio(wavBytes)
        }.start()
    }

    // --- API pipeline ---

    private fun processAudio(wavBytes: ByteArray) {
        if (wavBytes.isEmpty()) {
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
                    Log.w(TAG, "Text cleanup via DeepSeek failed -- using raw transcript", e)
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
            Log.w(TAG, "STT/API pipeline failed", e)
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
        // During active states the bubble must be fully visible.
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
}
