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
 * Keyboard detection -- two-tier approach:
 *   PRIMARY:  DiktaAccessibilityService calls onKeyboardVisibilityChanged() whenever
 *             it detects a TYPE_INPUT_METHOD window appearing/disappearing system-wide.
 *             This is the most reliable mechanism and works in all apps.
 *   FALLBACK: If the accessibility service is not active, we fall back to polling
 *             InputMethodManager.getInputMethodWindowVisibleHeight() via reflection.
 *
 * Bubble visibility modes (stored in SharedPreferences):
 *   KEYBOARD_ONLY (default): bubble appears only when the soft keyboard is visible.
 *   ALWAYS_VISIBLE: bubble is always on screen, regardless of keyboard state.
 *
 * Touch gestures in IDLE state:
 *   Single tap  -> start recording, bubble expands to pill bar with [X] [waveform] [✓]
 *   Long-press  -> push-to-talk: start recording immediately, release finger = confirm
 *
 * Touch gestures in RECORDING state (bar mode):
 *   Tap left zone  (X button)  -> cancel: stop recording, discard audio
 *   Tap right zone (✓ button)  -> confirm: stop recording, start STT + cleanup pipeline
 *   Drag                       -> moves the bar (drag threshold still applies)
 */
class DiktaOverlayService : Service() {

    companion object {
        private const val TAG = "DiktaOverlayService"

        private const val CHANNEL_ID    = "dikta_overlay"
        private const val NOTIFICATION_ID = 1
        private const val PREFS_NAME    = "dikta_bubble_prefs"
        private const val PREF_X        = "bubble_x"
        private const val PREF_Y        = "bubble_y"

        /** SharedPreference key: if true the bubble is always visible, not just when keyboard is open. */
        const val PREF_ALWAYS_VISIBLE = "bubble_always_visible"

        /** BroadcastReceiver action: tap on notification toggles bubble visibility. */
        const val ACTION_TOGGLE_BUBBLE = "com.dikta.voice.TOGGLE_BUBBLE"

        // Keyboard detection: poll InputMethodManager at this interval (ms)
        private const val KEYBOARD_CHECK_INTERVAL = 300L

        // Long-press threshold -- after this delay a held touch becomes push-to-talk
        private const val LONG_PRESS_TIMEOUT_MS = 500L

        // Base bubble size in dp -- multiplied by config.bubbleSize scale factor
        private const val BASE_BUBBLE_SIZE_DP = 56

        /** Live reference used by DiktaAccessibilityService for paste. */
        var instance: DiktaOverlayService? = null
    }

    private enum class RecordingState { IDLE, RECORDING, RECORDING_PTT, PROCESSING }

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

    /**
     * True when the bubble should be shown regardless of keyboard state.
     * Loaded from SharedPreferences; defaults to false (keyboard-only mode).
     */
    private var alwaysVisible = false

    /**
     * True once the AccessibilityService has called onKeyboardVisibilityChanged() at
     * least once. While this is false we trust the reflection-based fallback instead.
     */
    private var accessibilityServiceActive = false

    // Audio
    private var audioRecorder: DiktaAudioRecorder? = null

    // Touch handling
    private var dragTouchStartX = 0f
    private var dragTouchStartY = 0f
    private var bubbleStartX = 0
    private var bubbleStartY = 0
    private var isDragging = false
    private var dragThresholdPx = 0f

    // Bubble opacity (0.0..1.0). Applied to bubbleView.alpha when state is IDLE.
    // During RECORDING / PROCESSING the bubble is always fully opaque.
    // Loaded from config.json; defaults to 0.85 if config is unavailable.
    private var bubbleOpacity = 0.85f

    // Long-press / push-to-talk state
    private var longPressTriggered = false

    /**
     * True while the user is holding a long-press that triggered push-to-talk recording.
     * When the finger lifts we confirm (stop + process) instead of treating it as a tap.
     */
    private var pushToTalkActive = false

    private val longPressRunnable = Runnable {
        if (!isDragging && currentState == RecordingState.IDLE) {
            longPressTriggered = true
            pushToTalkActive   = true
            startRecording()
        }
    }

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

        val filter = IntentFilter(ACTION_TOGGLE_BUBBLE)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(toggleBubbleReceiver, filter, RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(toggleBubbleReceiver, filter)
        }

        windowManager = getSystemService(WINDOW_SERVICE) as WindowManager

        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        alwaysVisible = prefs.getBoolean(PREF_ALWAYS_VISIBLE, false)

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
            this, 0, toggleIntent,
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

    private fun updateNotification() {
        val nm = getSystemService(NotificationManager::class.java)
        nm.notify(NOTIFICATION_ID, buildNotification())
    }

    private fun startForegroundWithNotification() {
        val notification = buildNotification()
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
            startForeground(
                NOTIFICATION_ID, notification,
                ServiceInfo.FOREGROUND_SERVICE_TYPE_MICROPHONE
            )
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
    }

    // --- Keyboard detection ---

    private fun setupKeyboardDetector() {
        if (alwaysVisible) {
            showBubble()
        } else {
            handler.post(keyboardCheckRunnable)
        }
    }

    fun onKeyboardVisibilityChanged(visible: Boolean) {
        handler.post {
            accessibilityServiceActive = true
            applyKeyboardState(visible)
        }
    }

    private fun applyKeyboardState(isOpen: Boolean) {
        if (alwaysVisible) return
        if (isOpen == keyboardVisible) return

        keyboardVisible = isOpen
        if (isOpen) {
            showBubble()
        } else if (currentState == RecordingState.IDLE) {
            hideBubble()
        }
    }

    private fun checkKeyboardVisibility() {
        if (accessibilityServiceActive) return

        try {
            val imm = getSystemService(INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager
            val method = imm.javaClass.getMethod("getInputMethodWindowVisibleHeight")
            val height = method.invoke(imm) as Int
            applyKeyboardState(height > 0)
        } catch (e: Exception) {
            Log.d(TAG, "getInputMethodWindowVisibleHeight reflection failed", e)
        }
    }

    fun isAlwaysVisible(): Boolean = alwaysVisible

    fun setAlwaysVisible(enabled: Boolean) {
        alwaysVisible = enabled
        getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
            .putBoolean(PREF_ALWAYS_VISIBLE, enabled)
            .apply()

        if (enabled) {
            showBubble()
        } else {
            if (!keyboardVisible && currentState == RecordingState.IDLE) {
                hideBubble()
            }
        }
    }

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

    private fun setupBubble() {
        bubbleView = FloatingBubbleView(this)

        // Load bubble size and opacity from config.json (written by the Tauri/React settings UI).
        // Falls back to defaults if the config is not yet available (first launch).
        val config = DiktaApi.readConfig(this)
        val sizeScale = config?.bubbleSize ?: 1.0f
        bubbleOpacity = config?.bubbleOpacity ?: 0.85f

        val sizeDp = (BASE_BUBBLE_SIZE_DP * sizeScale).toInt().coerceAtLeast(24)
        bubbleView.setBubbleSize(sizeDp)
        bubbleView.alpha = bubbleOpacity

        val (screenW, screenH) = getScreenDimensions()
        val dp        = resources.displayMetrics.density
        val bubblePx  = (sizeDp * dp).toInt()
        val marginPx  = (16 * dp).toInt()

        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val savedX = prefs.getInt(PREF_X, screenW - bubblePx - marginPx)
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
            val bounds  = metrics.bounds
            Pair(bounds.width(), bounds.height())
        } else {
            val dm = DisplayMetrics()
            @Suppress("DEPRECATION")
            windowManager.defaultDisplay.getRealMetrics(dm)
            Pair(dm.widthPixels, dm.heightPixels)
        }
    }

    // --- WindowManager layout update ---

    /**
     * Pushes the current bubbleParams to WindowManager.
     * Must be called on the main thread whenever params change (size, position).
     */
    private fun updateBubbleLayout() {
        if (!isBubbleVisible) return
        try {
            windowManager.updateViewLayout(bubbleView, bubbleParams)
        } catch (e: Exception) {
            Log.w(TAG, "Failed to update bubble layout", e)
        }
    }

    /**
     * Adjusts the WindowManager LayoutParams width to match the current view state.
     *
     * IDLE / PROCESSING  -> WRAP_CONTENT (square = bubble diameter)
     * RECORDING          -> WRAP_CONTENT (the view's onMeasure returns BAR_WIDTH_DP)
     *
     * WRAP_CONTENT is sufficient because FloatingBubbleView.onMeasure returns different
     * dimensions depending on state. We just need to force a layout pass after the state
     * change so WindowManager picks up the new measured size.
     *
     * Also keeps the bar center aligned with the original bubble center:
     * when expanding from circle to bar we shift x left by half the extra width so
     * the center stays in place.
     */
    private fun adjustLayoutForState(newState: RecordingState, previousState: RecordingState) {
        val dp       = resources.displayMetrics.density
        val bubblePx = (bubbleView.getBubbleSizeDp() * dp).toInt()
        val barPx    = (FloatingBubbleView.BAR_WIDTH_DP * dp).toInt()

        when {
            newState == RecordingState.RECORDING && previousState == RecordingState.IDLE -> {
                // Expand: shift left so bubble center stays under finger
                val extraW = barPx - bubblePx
                bubbleParams.x = (bubbleParams.x - extraW / 2).coerceAtLeast(0)
            }
            newState != RecordingState.RECORDING && previousState == RecordingState.RECORDING -> {
                // Collapse: shift right to restore original center position
                val extraW = barPx - bubblePx
                bubbleParams.x += extraW / 2
            }
        }

        // WRAP_CONTENT in both directions; onMeasure drives the actual size
        bubbleParams.width  = WindowManager.LayoutParams.WRAP_CONTENT
        bubbleParams.height = WindowManager.LayoutParams.WRAP_CONTENT

        updateBubbleLayout()
    }

    // --- Touch handling ---

    private fun handleTouch(event: MotionEvent): Boolean {
        when (event.action) {
            MotionEvent.ACTION_DOWN -> {
                dragTouchStartX = event.rawX
                dragTouchStartY = event.rawY
                bubbleStartX    = bubbleParams.x
                bubbleStartY    = bubbleParams.y
                isDragging         = false
                longPressTriggered = false
                pushToTalkActive   = false

                // Only arm long-press in IDLE state (push-to-talk)
                if (currentState == RecordingState.IDLE) {
                    handler.postDelayed(longPressRunnable, LONG_PRESS_TIMEOUT_MS)
                }
                return true
            }

            MotionEvent.ACTION_MOVE -> {
                // During push-to-talk the bubble must stay locked in place.
                // Ignore all movement -- no drag, no cancel, no position update.
                if (pushToTalkActive) return true

                val dx = event.rawX - dragTouchStartX
                val dy = event.rawY - dragTouchStartY
                if (!isDragging && (abs(dx) > dragThresholdPx || abs(dy) > dragThresholdPx)) {
                    isDragging = true
                    // Moved too much -- cancel long-press
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
                    when {
                        isDragging -> {
                            savePosition(bubbleParams.x, bubbleParams.y)
                        }
                        pushToTalkActive -> {
                            // Push-to-talk release: confirm recording
                            pushToTalkActive = false
                            stopAndProcessRecording()
                        }
                        !longPressTriggered -> {
                            handleTap(event.x)
                        }
                    }
                } else {
                    // ACTION_CANCEL while push-to-talk -> cancel recording
                    if (pushToTalkActive) {
                        pushToTalkActive = false
                        cancelRecording()
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

    /**
     * Handles a tap (no drag, no long-press).
     *
     * In IDLE state:
     *   - Single tap -> start recording
     *
     * In RECORDING state:
     *   - Tap in left zone (X) -> cancel recording
     *   - Tap in right zone (✓) -> confirm recording
     *   - Tap in middle zone  -> ignored
     *
     * In PROCESSING state: taps are ignored.
     *
     * @param touchX Touch x-coordinate relative to the view's left edge.
     */
    private fun handleTap(touchX: Float) {
        when (currentState) {
            RecordingState.IDLE -> startRecording()
            RecordingState.RECORDING -> {
                when {
                    bubbleView.isTouchInCancelZone(touchX)  -> cancelRecording()
                    bubbleView.isTouchInConfirmZone(touchX) -> stopAndProcessRecording()
                    // Middle zone tap: ignore
                }
            }
            RecordingState.RECORDING_PTT -> { /* PTT: ignore taps, release handles it */ }
            RecordingState.PROCESSING -> { /* ignore */ }
        }
    }

    // --- Audio recording ---

    private fun startRecording() {
        val recorder = DiktaAudioRecorder { amplitude ->
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
        val previousState = currentState

        if (pushToTalkActive) {
            // PTT mode: bubble stays circular (no bar expansion), just turns red + scales up.
            // adjustLayoutForState is intentionally skipped -- view size does not change.
            setState(RecordingState.RECORDING_PTT)
        } else {
            setState(RecordingState.RECORDING)
            adjustLayoutForState(RecordingState.RECORDING, previousState)
        }
    }

    /**
     * Stops recording and discards the captured audio.
     * Returns the bubble to IDLE immediately without calling the STT pipeline.
     */
    private fun cancelRecording() {
        val recorder = audioRecorder ?: return
        audioRecorder = null

        // Release the recorder on a background thread (stop() can block briefly)
        Thread {
            recorder.releaseImmediately()
        }.start()

        val previousState = currentState
        setState(RecordingState.IDLE)
        // Only adjust layout if we were in bar mode (tap-to-record), not PTT mode.
        if (previousState == RecordingState.RECORDING) {
            adjustLayoutForState(RecordingState.IDLE, previousState)
        }
    }

    /**
     * Stops recording and starts the STT + cleanup pipeline.
     * This is the "confirm" action -- used by the ✓ button and push-to-talk release.
     */
    private fun stopAndProcessRecording() {
        val recorder = audioRecorder ?: return
        audioRecorder = null

        val previousState = currentState
        setState(RecordingState.PROCESSING)
        // Only adjust layout if we were in bar mode (tap-to-record), not PTT mode.
        if (previousState == RecordingState.RECORDING) {
            adjustLayoutForState(RecordingState.PROCESSING, previousState)
        }

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
                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)
            }
            return
        }

        val config = DiktaApi.readConfig(this)
        if (config == null || config.groqApiKey.isBlank()) {
            handler.post {
                showToast("No API keys configured. Please open Dikta and add your Groq key in Settings.")
                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)
            }
            return
        }

        try {
            // Step 1: STT via Groq Whisper
            val transcript = DiktaApi.transcribe(wavBytes, config.groqApiKey, config.language)

            if (transcript.isBlank()) {
                handler.post {
                    showToast("No speech detected")
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                }
                return
            }

            // Step 2: Text cleanup via DeepSeek (optional -- skip if no key)
            val finalText = if (config.deepseekApiKey.isNotBlank()) {
                try {
                    DiktaApi.cleanupChunked(transcript, config.deepseekApiKey, config.cleanupStyle)
                } catch (e: IOException) {
                    Log.w(TAG, "Text cleanup via DeepSeek failed -- using raw transcript", e)
                    transcript
                }
            } else {
                transcript
            }

            // Step 3: Save to history DB
            DiktaApi.saveToHistory(
                context  = this,
                finalText = finalText,
                rawText  = transcript,
                style    = config.cleanupStyle,
                language = config.language,
                deviceId = config.deviceId
            )

            // Step 3b: Push unsynced entries to Turso (best-effort)
            DiktaApi.pushToTurso(this, config.tursoUrl, config.tursoToken)

            // Step 4: Copy to clipboard and paste
            handler.post {
                copyToClipboard(finalText)

                val pasted = DiktaAccessibilityService.instance != null
                DiktaAccessibilityService.instance?.pasteIntoFocusedField()

                val preview = if (finalText.length > 50) finalText.take(50) + "..." else finalText
                if (pasted) showToast("Inserted: $preview") else showToast("Copied: $preview")

                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)
            }

        } catch (e: IOException) {
            Log.w(TAG, "STT/API pipeline failed", e)
            handler.post {
                showToast("Error: ${e.message?.take(80)}")
                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)
            }
        }
    }

    // --- Helpers ---

    private fun setState(newState: RecordingState) {
        currentState   = newState
        bubbleView.state = when (newState) {
            RecordingState.IDLE          -> FloatingBubbleView.State.IDLE
            RecordingState.RECORDING     -> FloatingBubbleView.State.RECORDING
            RecordingState.RECORDING_PTT -> FloatingBubbleView.State.RECORDING_PTT
            RecordingState.PROCESSING    -> FloatingBubbleView.State.PROCESSING
        }
        bubbleView.alpha = when (newState) {
            RecordingState.IDLE -> bubbleOpacity
            RecordingState.RECORDING, RecordingState.RECORDING_PTT,
            RecordingState.PROCESSING -> 1.0f
        }
        if (newState == RecordingState.IDLE) {
            bubbleView.amplitude = 0f
            // Re-read config so bubble size/opacity changes from Settings take effect
            // without requiring a full app restart.
            reloadBubbleAppearance()
        }
    }

    /**
     * Re-reads bubble size and opacity from config.json and applies them live.
     * Called on every return to IDLE so Settings changes take effect after the next dictation.
     */
    private fun reloadBubbleAppearance() {
        val config = DiktaApi.readConfig(this) ?: return
        val newSizeDp = (BASE_BUBBLE_SIZE_DP * config.bubbleSize).toInt().coerceAtLeast(24)
        bubbleOpacity = config.bubbleOpacity
        bubbleView.setBubbleSize(newSizeDp)
        bubbleView.alpha = bubbleOpacity
        updateBubbleLayout()
    }

    private fun copyToClipboard(text: String) {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val clip      = ClipData.newPlainText("Dikta transcription", text)
        clipboard.setPrimaryClip(clip)
    }

    private fun showToast(message: String) {
        Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
    }
}
