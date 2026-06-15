package com.klarvo.voice

import android.Manifest
import android.app.*
import android.content.*
import android.content.pm.PackageManager
import android.content.pm.ServiceInfo
import android.graphics.PixelFormat
import android.os.*
import android.util.DisplayMetrics
import android.view.*
import android.widget.Toast
import androidx.core.content.ContextCompat
import java.io.File
import java.io.IOException
import kotlin.math.abs

/**
 * Foreground Service that manages the floating bubble overlay.
 *
 * Keyboard detection -- two-tier approach:
 *   PRIMARY:  KlarvoAccessibilityService calls onKeyboardVisibilityChanged() whenever
 *             it detects a TYPE_INPUT_METHOD window appearing/disappearing system-wide.
 *             This is the most reliable mechanism and works in all apps.
 *   FALLBACK: If the accessibility service is not active, we fall back to polling
 *             InputMethodManager.getInputMethodWindowVisibleHeight() via reflection.
 *
 * Bubble visibility modes (stored in SharedPreferences):
 *   KEYBOARD_ONLY (default): bubble appears only when the soft keyboard is visible.
 *   ALWAYS_VISIBLE: bubble is always on screen, regardless of keyboard state.
 *
 * Recording modes (switchable via notification action):
 *   HOLD:     Tap -> bar with [X][waveform][✓], Long-press -> PTT (hold to record)
 *   TOGGLE:   Tap -> start (red circle), Tap again -> stop + process
 *   AUTOSTOP: Tap -> start, auto-stops after silence detected
 *   AUTO:     Tap -> start loop, auto-stops on silence then restarts, Tap -> stop loop
 *
 * Touch gestures in RECORDING state (bar mode, HOLD only):
 *   Tap left zone  (X button)  -> cancel: stop recording, discard audio
 *   Tap right zone (✓ button)  -> confirm: stop recording, start STT + cleanup pipeline
 *   Drag                       -> moves the bar (drag threshold still applies)
 */
class KlarvoOverlayService : Service() {

    companion object {
        private const val TAG = "KlarvoOverlayService"

        private const val CHANNEL_ID    = "klarvo_overlay"
        private const val NOTIFICATION_ID = 1
        private const val PREFS_NAME    = "klarvo_bubble_prefs"
        private const val PREF_X        = "bubble_x"
        private const val PREF_Y        = "bubble_y"
        private const val PREF_SIDE     = "bubble_side"  // "left" or "right"

        // Keyboard jump-up: fixed nav-bar clearance in px (AR5d — do NOT use WindowInsetsCompat
        // or env(safe-area-inset-bottom); those are unreliable for overlay Services on API 24+)
        private const val NAV_BAR_CLEARANCE_PX = 56

        /** SharedPreference key: if true the bubble is always visible, not just when keyboard is open. */
        const val PREF_ALWAYS_VISIBLE = "bubble_always_visible"

        /** BroadcastReceiver actions. */
        const val ACTION_TOGGLE_BUBBLE = "com.klarvo.voice.TOGGLE_BUBBLE"

        // Keyboard detection: poll InputMethodManager at this interval (ms)
        private const val KEYBOARD_CHECK_INTERVAL = 300L

        // Long-press threshold -- after this delay a held touch becomes push-to-talk
        private const val LONG_PRESS_TIMEOUT_MS = 500L

        // Debounce delay before showing the bubble. Gives checkForegroundBankingApp()
        // time to detect and block, preventing the brief "flash" that banking apps catch.
        private const val SHOW_DEBOUNCE_MS = 150L

        // Base bubble size in dp -- multiplied by config.bubbleSize scale factor
        private const val BASE_BUBBLE_SIZE_DP = 56

        /** Live reference used by KlarvoAccessibilityService for paste. */
        var instance: KlarvoOverlayService? = null
    }

    // Cached config -- populated by loadBubbleControls(), reused in processAudio().
    // Avoids redundant disk reads within a single dictation cycle.
    private var cachedConfig: KlarvoApi.Config? = null

    enum class RecordingMode(val label: String, val badge: String) {
        HOLD("Hold", "H"),
        TOGGLE("Toggle", "T"),
        AUTOSTOP("Auto Stop", "S"),
        AUTO("Auto", "A");

        fun next(): RecordingMode = entries[(ordinal + 1) % entries.size]

        companion object {
            /**
             * Maps a config string (case-insensitive) to a RecordingMode.
             * Falls back to HOLD for unknown values.
             */
            fun fromString(value: String): RecordingMode = when (value.lowercase()) {
                "toggle"   -> TOGGLE
                "autostop" -> AUTOSTOP
                "auto"     -> AUTO
                else       -> HOLD
            }
        }
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

    /** True while a banking/security app is in the foreground. Blocks bubble show. */
    private var bankingAppActive = false

    // Audio
    private var audioRecorder: KlarvoAudioRecorder? = null

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

    // Per-gesture recording modes: tap and long-press are configured independently.
    private var tapMode = RecordingMode.TOGGLE
    private var longPressMode = RecordingMode.HOLD

    // Per-gesture auto-send and silence-detection settings.
    private var tapAutoSend = false
    private var longPressAutoSend = false
    private var tapSilenceSecs = 2.0f
    private var longPressSilenceSecs = 2.0f
    // Mode-level silence durations (AUTO/AUTOSTOP use these, parity with desktop pipeline.rs:640/704).
    private var autostopSilenceSecs = 2.0f
    private var autoModeSilenceSecs = 2.0f

    /**
     * Tracks which gesture started the current recording session.
     * Used to select the correct silenceSecs / autoSend values when stopping.
     * "tap" or "longpress"; null when not recording.
     */
    private var activeGesture: String? = null

    // Auto-mode loop: true while the auto-loop is active (records, processes, repeats)
    private var autoLoopActive = false

    /**
     * Remembered Y position before a keyboard jump-up so we can restore it when the keyboard
     * closes in always-visible mode. Null when the bubble has not been moved for the keyboard.
     * Never written by savePosition() — a keyboard-shifted Y must not become the resting position.
     */
    private var preKeyboardY: Int? = null

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
            activeGesture      = "longpress"
            // Re-read config before deciding behavior.
            loadBubbleControls()
            // Only activate push-to-talk (stop on finger lift) when longPressMode is HOLD.
            pushToTalkActive = (longPressMode == RecordingMode.HOLD)
            // Enable auto-loop when longPressMode is AUTO.
            if (longPressMode == RecordingMode.AUTO) {
                autoLoopActive = true
            }
            startRecording()
        }
    }

    /**
     * Receives broadcast actions from the foreground notification.
     * Registered/unregistered dynamically -- no manifest entry needed.
     * Only handles ACTION_TOGGLE_BUBBLE (mode switching removed).
     */
    private val notificationActionReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context?, intent: Intent?) {
            if (intent?.action == ACTION_TOGGLE_BUBBLE) toggleBubble()
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
        KlarvoLogger.init(this)
        dragThresholdPx = 10f * resources.displayMetrics.density

        overlayType = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY
        } else {
            @Suppress("DEPRECATION")
            WindowManager.LayoutParams.TYPE_PHONE
        }

        cleanupStalePendingWavFiles()
        loadBubbleControls()
        createNotificationChannel()
        startForegroundWithNotification()

        val filter = IntentFilter().apply {
            addAction(ACTION_TOGGLE_BUBBLE)
        }
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(notificationActionReceiver, filter, RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(notificationActionReceiver, filter)
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
            unregisterReceiver(notificationActionReceiver)
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "Failed to unregister notificationActionReceiver (already unregistered?)", e)
        }
        audioRecorder?.releaseImmediately()
        audioRecorder = null
        super.onDestroy()
        if (::bubbleView.isInitialized && isBubbleVisible) {
            try {
                windowManager.removeView(bubbleView)
            } catch (e: Exception) {
                KlarvoLogger.w(TAG, "Failed to remove bubbleView on destroy", e)
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
                "Klarvo Overlay",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Keeps the Klarvo voice bubble visible"
                setShowBadge(false)
            }
            val nm = getSystemService(NotificationManager::class.java)
            nm.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(): Notification {
        // Show current per-gesture mode configuration as status text.
        val statusText = "Tap: ${tapMode.label}, Hold: ${longPressMode.label}"

        // Tap on notification body = toggle bubble visibility
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
        builder
            .setContentTitle("Klarvo")
            .setContentText(statusText)
            .setSmallIcon(android.R.drawable.ic_btn_speak_now)
            .setContentIntent(pendingToggle)
            .setOngoing(true)

        return builder.build()
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

    // --- Recording controls ---

    /**
     * Loads per-gesture recording controls from config.json.
     * Replaces the old single-mode loadRecordingMode().
     */
    private fun loadBubbleControls() {
        val config = KlarvoApi.readConfig(this)
        cachedConfig = config  // cache for processAudio() to avoid redundant disk read
        if (config != null) {
            tapMode = RecordingMode.fromString(config.bubbleTapMode)
            longPressMode = RecordingMode.fromString(config.bubbleLongPressMode)
            // Auto-send disabled on Android — Enter key rarely works in mobile apps.
            tapAutoSend = false
            longPressAutoSend = false
            tapSilenceSecs = config.bubbleTapSilenceSecs
            longPressSilenceSecs = config.bubbleLongPressSilenceSecs
            autostopSilenceSecs = config.autostopSilenceSecs
            autoModeSilenceSecs = config.autoModeSilenceSecs
            KlarvoLogger.d(TAG, "loadBubbleControls: tap=${config.bubbleTapMode}→$tapMode, lp=${config.bubbleLongPressMode}→$longPressMode, tapAutoSend=$tapAutoSend, lpAutoSend=$longPressAutoSend")
        } else {
            KlarvoLogger.w(TAG, "loadBubbleControls: config is NULL, using defaults tap=$tapMode, lp=$longPressMode")
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

    /**
     * Called by KlarvoAccessibilityService when a banking/security app enters or
     * leaves the foreground. When active, the bubble is forcefully hidden regardless
     * of keyboard state or alwaysVisible setting. This is a security feature and
     * cannot be disabled.
     */
    fun onBankingAppStateChanged(active: Boolean, packageName: String) {
        handler.post {
            KlarvoLogger.d(TAG, "Banking state change: active=$active (was=$bankingAppActive), pkg=$packageName")
            if (active == bankingAppActive) return@post
            bankingAppActive = active

            if (active) {
                KlarvoLogger.i(TAG, "Banking app detected: $packageName — hiding bubble")
                hideBubble()
                Toast.makeText(this, "Klarvo paused — you can dismiss any remaining security warning from your banking app.", Toast.LENGTH_LONG).show()
            } else {
                KlarvoLogger.i(TAG, "Banking app left foreground: $packageName — restoring bubble")
                // Re-apply normal visibility rules: show if keyboard is open or alwaysVisible.
                if (alwaysVisible || keyboardVisible) {
                    showBubble()
                }
            }
        }
    }

    private fun applyKeyboardState(isOpen: Boolean) {
        if (alwaysVisible) {
            // In always-visible mode we never show/hide, but we still need to restore the
            // bubble's Y position when the keyboard closes after a jump-up.
            if (!isOpen) {
                val saved = preKeyboardY
                if (saved != null && isBubbleVisible && ::bubbleView.isInitialized) {
                    preKeyboardY = null
                    bubbleParams.y = saved
                    updateBubbleLayout()
                } else {
                    preKeyboardY = null
                }
            }
            return
        }
        if (isOpen == keyboardVisible) return

        keyboardVisible = isOpen
        if (isOpen) {
            showBubble()
        } else {
            // Keyboard closed: clear any remembered pre-keyboard Y (not always-visible path,
            // so the bubble is about to be hidden; no restore needed).
            preKeyboardY = null
            if (currentState == RecordingState.IDLE) {
                hideBubble()
            }
        }
    }

    /**
     * Adjusts the bubble Y position upward if the keyboard would cover it.
     * Called by KlarvoAccessibilityService when the IME window bounds are known,
     * and by the reflection fallback path with the IMM-reported height.
     *
     * Nav-bar clearance: 56px fixed constant (AR5d). Do NOT use WindowInsetsCompat or
     * env(safe-area-inset-bottom) — those are unreliable for overlay Services on API 24+.
     */
    fun adjustBubbleForKeyboard(keyboardHeightPx: Int) {
        handler.post {
            if (!isBubbleVisible || !::bubbleView.isInitialized) return@post
            val (_, screenH) = getScreenDimensions()
            val dm = resources.displayMetrics
            // Use the window height (≥48dp touch target) so the window's real bottom edge
            // clears the keyboard, not just the smaller visual circle.
            val windowPx = (maxOf(bubbleView.getBubbleSizeDp(), 48) * dm.density).toInt()
            val maxY = screenH - keyboardHeightPx - NAV_BAR_CLEARANCE_PX - windowPx
            if (bubbleParams.y > maxY) {
                if (preKeyboardY == null) {
                    preKeyboardY = bubbleParams.y
                }
                bubbleParams.y = maxY.coerceAtLeast(0)
                updateBubbleLayout()
            }
        }
    }

    private fun checkKeyboardVisibility() {
        if (accessibilityServiceActive) return

        try {
            val imm = getSystemService(INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager
            val method = imm.javaClass.getMethod("getInputMethodWindowVisibleHeight")
            val height = method.invoke(imm) as Int
            applyKeyboardState(height > 0)
            // Reflection fallback: also apply keyboard jump-up with the reported height
            if (height > 0) {
                adjustBubbleForKeyboard(height)
            }
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "getInputMethodWindowVisibleHeight reflection failed: ${e.message}")
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

    /**
     * Pending show runnable for debounced bubble display. When showBubble() is called,
     * the actual WindowManager.addView is delayed by SHOW_DEBOUNCE_MS to give
     * checkForegroundBankingApp() time to detect and block. This prevents the brief
     * "flash" that banking apps like N26 catch as an active overlay.
     */
    private var pendingShowRunnable: Runnable? = null

    private fun showBubble() {
        if (bankingAppActive) return  // Never show while banking app is active
        // Cancel any previous pending show to avoid duplicates
        pendingShowRunnable?.let { handler.removeCallbacks(it) }
        val runnable = Runnable {
            pendingShowRunnable = null
            if (bankingAppActive) return@Runnable  // Re-check after delay
            if (!isBubbleVisible && ::bubbleView.isInitialized) {
                try {
                    reloadBubbleAppearance()
                    windowManager.addView(bubbleView, bubbleParams)
                    isBubbleVisible = true
                    updateNotification()
                } catch (e: Exception) {
                    KlarvoLogger.w(TAG, "Failed to add bubbleView to WindowManager", e)
                }
            }
        }
        pendingShowRunnable = runnable
        handler.postDelayed(runnable, SHOW_DEBOUNCE_MS)
    }

    private fun hideBubble() {
        // Cancel any pending show — banking detection may arrive before the debounce fires
        pendingShowRunnable?.let { handler.removeCallbacks(it) }
        pendingShowRunnable = null
        if (isBubbleVisible && ::bubbleView.isInitialized) {
            try {
                windowManager.removeView(bubbleView)
                isBubbleVisible = false
                updateNotification()
            } catch (e: Exception) {
                KlarvoLogger.w(TAG, "Failed to remove bubbleView from WindowManager", e)
            }
        }
    }

    // --- Bubble setup ---

    private fun setupBubble() {
        bubbleView = FloatingBubbleView(this)

        // Load opacity from config.json (written by the Tauri/React settings UI).
        // bubbleSize scale factor is superseded by computeVisualSizeDp() as of Story 9.3.
        val config = KlarvoApi.readConfig(this)
        bubbleOpacity = config?.bubbleOpacity ?: 0.85f

        // Responsive size formula: clamp(36, 0.11 × min(screenW_dp, screenH_dp), 44)
        val sizeDp = computeVisualSizeDp()
        bubbleView.setBubbleSize(sizeDp)
        bubbleView.alpha = bubbleOpacity

        val (screenW, screenH) = getScreenDimensions()
        val dm        = resources.displayMetrics
        val dp        = dm.density
        val bubblePx  = (sizeDp * dp).toInt()
        val marginPx  = (8 * dp).toInt()  // 8dp snap margin (tighter than startup default)

        // Touch-target expansion: LayoutParams must be ≥ 48dp to meet touch-target requirement.
        // The visual circle (bubbleSizeDp) may be smaller; FloatingBubbleView draws it centered.
        val touchTargetDp = maxOf(sizeDp, 48)
        val touchTargetPx = (touchTargetDp * dp).toInt()

        // Restore position using saved side (left/right) rather than raw pixel X.
        // This ensures the bubble lands on the correct edge after screen rotation or reinstall.
        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val savedSide = prefs.getString(PREF_SIDE, "right") ?: "right"
        // WindowManager positions the window (touchTargetPx wide), not the visual circle.
        // Use touchTargetPx for edge placement so the window fits within the screen edge.
        val defaultX = if (savedSide == "left") marginPx else screenW - touchTargetPx - marginPx
        val savedX = prefs.getInt(PREF_X, defaultX)
        val savedY = prefs.getInt(PREF_Y, screenH / 2)

        bubbleParams = WindowManager.LayoutParams(
            touchTargetPx,
            touchTargetPx,
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

    /**
     * Computes the visual bubble size in dp using the responsive formula:
     *   visualDp = clamp(36, (0.11 × min(screenW_dp, screenH_dp)).toInt(), 44)
     *
     * This supersedes the old BASE_BUBBLE_SIZE_DP × config.bubbleSize formula as of Story 9.3.
     * The bubbleSize config scale factor is no longer applied to the visual size (it becomes a
     * no-op). Document here so Story 9.5+ is aware. The config field is intentionally not
     * removed — it may be repurposed or restored in a future story.
     */
    private fun computeVisualSizeDp(): Int {
        val dm = resources.displayMetrics
        val screenWdp = dm.widthPixels / dm.density
        val screenHdp = dm.heightPixels / dm.density
        val rawDp = (0.11f * minOf(screenWdp, screenHdp)).toInt()
        return rawDp.coerceIn(36, 44)
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
            KlarvoLogger.w(TAG, "Failed to update bubble layout", e)
        }
    }

    /**
     * Adjusts the WindowManager LayoutParams to match the current view state.
     *
     * IDLE / PROCESSING  -> explicit touchTargetPx × touchTargetPx (≥48dp touch target)
     *                       FloatingBubbleView draws the smaller visual circle centered within.
     * RECORDING          -> WRAP_CONTENT (onMeasure returns BAR_WIDTH_DP × bubbleSizeDp)
     * RECORDING_PTT      -> explicit touchTargetPx (same as IDLE; scale animation via scaleX/Y)
     *
     * Also keeps the bar center aligned with the original bubble center:
     * when expanding from circle to bar we shift x left by half the extra width so
     * the center stays in place.
     */
    private fun adjustLayoutForState(newState: RecordingState, previousState: RecordingState) {
        val dp           = resources.displayMetrics.density
        val visualDp     = bubbleView.getBubbleSizeDp()
        val bubblePx     = (visualDp * dp).toInt()
        val touchTargetDp = maxOf(visualDp, 48)
        val touchTargetPx = (touchTargetDp * dp).toInt()
        val barPx         = (FloatingBubbleView.BAR_WIDTH_DP * dp).toInt()

        when {
            newState == RecordingState.RECORDING && previousState == RecordingState.IDLE -> {
                // Expand: shift left so bubble center stays under finger
                // Center of touch-target was at (bubbleParams.x + touchTargetPx/2);
                // bar center should stay there: new x = oldCenter - barPx/2
                val oldCenterX = bubbleParams.x + touchTargetPx / 2
                bubbleParams.x = (oldCenterX - barPx / 2).coerceAtLeast(0)
            }
            newState != RecordingState.RECORDING && previousState == RecordingState.RECORDING -> {
                // Collapse: restore center position
                val oldCenterX = bubbleParams.x + barPx / 2
                bubbleParams.x = (oldCenterX - touchTargetPx / 2).coerceAtLeast(0)
            }
        }

        when (newState) {
            RecordingState.RECORDING -> {
                // Bar mode: let onMeasure drive the size
                bubbleParams.width  = WindowManager.LayoutParams.WRAP_CONTENT
                bubbleParams.height = WindowManager.LayoutParams.WRAP_CONTENT
            }
            else -> {
                // IDLE / RECORDING_PTT / PROCESSING: explicit touch-target dimensions
                bubbleParams.width  = touchTargetPx
                bubbleParams.height = touchTargetPx
            }
        }

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
                        KlarvoLogger.w(TAG, "Failed to update bubble position during drag", e)
                    }
                }
                return true
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                handler.removeCallbacks(longPressRunnable)

                if (event.action == MotionEvent.ACTION_UP) {
                    when {
                        isDragging -> {
                            // Edge-snap: slide to nearest horizontal edge on drag release.
                            // 8dp margin from edge for a clean overlay feel.
                            val (screenW, _) = getScreenDimensions()
                            val dm = resources.displayMetrics
                            // WindowManager positions the window (windowPx wide), not the visual
                            // circle. All edge/midpoint math must use the window width.
                            val windowPx = (maxOf(bubbleView.getBubbleSizeDp(), 48) * dm.density).toInt()
                            val marginPx = (8 * dm.density).toInt()
                            val midScreen = screenW / 2
                            bubbleParams.x = if (bubbleParams.x + windowPx / 2 < midScreen) {
                                marginPx              // snap left
                            } else {
                                screenW - windowPx - marginPx  // snap right
                            }
                            updateBubbleLayout()
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
        val (screenW, _) = getScreenDimensions()
        val dm = resources.displayMetrics
        // Side detection uses the window width (≥48dp touch target), matching WindowManager
        // placement logic so left/right classification agrees with edge-snap math.
        val windowPx = (maxOf(bubbleView.getBubbleSizeDp(), 48) * dm.density).toInt()
        val side = if (x + windowPx / 2 < screenW / 2) "left" else "right"
        getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE).edit()
            .putInt(PREF_X, x)
            .putInt(PREF_Y, y)
            .putString(PREF_SIDE, side)
            .apply()
    }

    // --- State machine ---

    /**
     * Handles a tap (no drag, no long-press).
     *
     * Behavior depends on [tapMode]:
     *
     * HOLD mode:
     *   IDLE -> expand to bar with [X][waveform][✓]
     *   RECORDING -> tap cancel/confirm zones
     *
     * TOGGLE / AUTOSTOP mode:
     *   IDLE -> start recording (red circle, no bar)
     *   RECORDING -> stop + process
     *
     * AUTO mode:
     *   IDLE -> start auto-loop (records, processes, repeats until tapped again)
     *   RECORDING -> stop loop + process current segment
     *
     * @param touchX Touch x-coordinate relative to the view's left edge.
     */
    private fun handleTap(touchX: Float) {
        when (currentState) {
            RecordingState.IDLE -> {
                activeGesture = "tap"
                // Reload config before checking mode so Settings changes apply immediately.
                loadBubbleControls()
                if (tapMode == RecordingMode.AUTO) {
                    autoLoopActive = true
                }
                startRecording()
            }
            RecordingState.RECORDING -> {
                when (tapMode) {
                    RecordingMode.HOLD -> {
                        when {
                            bubbleView.isTouchInCancelZone(touchX)  -> cancelRecording()
                            bubbleView.isTouchInConfirmZone(touchX) -> stopAndProcessRecording()
                            // Middle zone tap: ignore
                        }
                    }
                    RecordingMode.TOGGLE, RecordingMode.AUTOSTOP -> {
                        stopAndProcessRecording()
                    }
                    RecordingMode.AUTO -> {
                        autoLoopActive = false
                        stopAndProcessRecording()
                    }
                }
            }
            RecordingState.RECORDING_PTT -> {
                if (!pushToTalkActive) {
                    // Not actual PTT -- this is TOGGLE/AUTOSTOP/AUTO using circular visual
                    when (tapMode) {
                        RecordingMode.TOGGLE, RecordingMode.AUTOSTOP -> stopAndProcessRecording()
                        RecordingMode.AUTO -> {
                            autoLoopActive = false
                            stopAndProcessRecording()
                        }
                        else -> { /* HOLD PTT: ignore taps, release handles it */ }
                    }
                }
                // If pushToTalkActive: ignore taps, finger release handles it
            }
            RecordingState.PROCESSING -> {
                // Stop auto-loop so the cycle doesn't repeat after this processing finishes.
                if (autoLoopActive) {
                    autoLoopActive = false
                    KlarvoLogger.d(TAG, "Auto-loop deactivated by tap during processing")
                }
            }
        }
    }

    // --- Audio recording ---

    private fun startRecording() {
        // Check runtime permission before starting audio capture.
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.RECORD_AUDIO)
            != PackageManager.PERMISSION_GRANTED
        ) {
            KlarvoLogger.e(TAG, "RECORD_AUDIO permission not granted at recording time")
            showToast("Microphone permission required. Please grant in app settings.")
            return
        }

        // Pre-check: verify that a valid config with API keys exists before we start recording.
        // Without this check, the pipeline would silently fail after the user has already
        // recorded audio -- which is confusing. Failing fast here gives immediate feedback.
        val preCheckConfig = cachedConfig ?: KlarvoApi.readConfig(this)
        if (preCheckConfig == null) {
            // config.json missing entirely -- app was never configured via the desktop UI.
            KlarvoLogger.w(TAG, "startRecording: config.json not found or incomplete -- aborting")
            showToast("No configuration found. Open Klarvo on your desktop and configure the app first.")
            return
        }
        if (preCheckConfig.sttProvider != "local" && preCheckConfig.groqApiKey.isBlank()) {
            // Cloud STT selected but no Groq key present.
            KlarvoLogger.w(TAG, "startRecording: Groq API key missing -- aborting")
            showToast("No API key configured. Open Klarvo Settings and add your Groq key.")
            return
        }

        // Determine which mode governs this recording session.
        val activeMode = when (activeGesture) {
            "longpress" -> longPressMode
            else        -> tapMode  // "tap" or null (auto-loop restart)
        }

        // Select the silence duration by the ACTIVE MODE, mirroring desktop
        // (pipeline.rs:640 AUTOSTOP→autostop_silence_secs, :704 AUTO→auto_mode_silence_secs)
        // and the shared settings UI, which binds the silence slider to those mode-level fields.
        // The bubble per-gesture values apply only to non-auto modes (HOLD/TOGGLE), where no
        // silence auto-stop is wired anyway.
        val activeSilenceSecs = when (activeMode) {
            RecordingMode.AUTO     -> autoModeSilenceSecs
            RecordingMode.AUTOSTOP -> autostopSilenceSecs
            else -> when (activeGesture) {
                "longpress" -> longPressSilenceSecs
                else        -> tapSilenceSecs
            }
        }
        KlarvoLogger.d(TAG, "[pipeline] silence window: mode=$activeMode → ${activeSilenceSecs}s")

        val recorder = KlarvoAudioRecorder(
            context = this,
            onAmplitude = { amplitude -> handler.post { bubbleView.amplitude = amplitude } },
            silenceSecs = activeSilenceSecs
        )

        // Wire up silence detection for AUTOSTOP / AUTO modes.
        if (activeMode == RecordingMode.AUTOSTOP || activeMode == RecordingMode.AUTO) {
            recorder.onSilenceDetected = {
                handler.post { onSilenceTriggered() }
            }
        }

        try {
            recorder.start()
        } catch (e: SecurityException) {
            KlarvoLogger.e(TAG, "Permission denied when starting audio recording", e)
            showToast("Microphone permission denied. Please grant in app settings.")
            return
        } catch (e: IllegalStateException) {
            KlarvoLogger.w(TAG, "Failed to start audio recording", e)
            showToast("Cannot start recording: ${e.message}")
            return
        }

        audioRecorder = recorder
        val previousState = currentState

        when {
            pushToTalkActive -> {
                // PTT mode: bubble stays circular (no bar expansion), just turns red + scales up.
                setState(RecordingState.RECORDING_PTT)
            }
            activeMode == RecordingMode.HOLD -> {
                // HOLD: expand to bar with cancel/confirm buttons
                setState(RecordingState.RECORDING)
                adjustLayoutForState(RecordingState.RECORDING, previousState)
            }
            else -> {
                // TOGGLE / AUTOSTOP / AUTO: red circle, no bar
                setState(RecordingState.RECORDING_PTT)
            }
        }
    }

    /**
     * Called when silence detection fires (AUTOSTOP / AUTO modes only).
     * Must be called on the main thread.
     */
    private fun onSilenceTriggered() {
        if (currentState != RecordingState.RECORDING && currentState != RecordingState.RECORDING_PTT) return

        val activeMode = when (activeGesture) {
            "longpress" -> longPressMode
            else        -> tapMode
        }

        when (activeMode) {
            RecordingMode.AUTOSTOP -> {
                stopAndProcessRecording()
            }
            RecordingMode.AUTO -> {
                // Stop current segment and process it, then start a new recording
                // if the auto-loop is still active.
                stopAndProcessRecording()
                // processAudio will call onProcessingComplete which handles the loop restart.
            }
            else -> { /* should not happen */ }
        }
    }

    /**
     * Stops recording and discards the captured audio.
     * Returns the bubble to IDLE immediately without calling the STT pipeline.
     */
    private fun cancelRecording() {
        val recorder = audioRecorder ?: return
        audioRecorder = null
        autoLoopActive = false

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
        val t0 = System.currentTimeMillis()
        if (wavBytes.isEmpty()) {
            handler.post {
                showToast("No audio recorded")
                autoLoopActive = false
                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)
            }
            return
        }

        // Pre-STT filter: discard mini-taps and silent recordings before the Groq API call.
        // Delegates to the shared Rust silence_skip via GroqSttBridge (ADR-0017, AC4).
        // Config values read here use defaults matching the Rust pipeline defaults
        // (minRecordingMs=500, silenceThreshold=0.005) so the filter runs before the full
        // config read below. These match KlarvoApi.AppConfig defaults.
        run {
            val wavBase64ForFilter = android.util.Base64.encodeToString(wavBytes, android.util.Base64.NO_WRAP)
            val silenceResult = GroqSttBridge.nativeSilenceCheck(wavBase64ForFilter, 500L, 0.005f)
            when {
                silenceResult.startsWith("TooShort:") -> {
                    val durationMs = silenceResult.removePrefix("TooShort:").toLongOrNull() ?: 0L
                    KlarvoLogger.d(TAG, "[pipeline] pre-STT filter: TooShort (${durationMs}ms < 500ms)")
                    handler.post {
                        showToast("Recording too short")
                        autoLoopActive = false
                        val prev = currentState
                        setState(RecordingState.IDLE)
                        adjustLayoutForState(RecordingState.IDLE, prev)
                    }
                    return
                }
                silenceResult.startsWith("Silent:") -> {
                    val rms = silenceResult.removePrefix("Silent:").toFloatOrNull() ?: 0f
                    KlarvoLogger.d(TAG, "[pipeline] pre-STT filter: Silent (rms=$rms < 0.005)")
                    handler.post {
                        showToast("No speech detected")
                        autoLoopActive = false
                        val prev = currentState
                        setState(RecordingState.IDLE)
                        adjustLayoutForState(RecordingState.IDLE, prev)
                    }
                    return
                }
                else -> { /* "Pass" — proceed to STT */ }
            }
        }

        // Persist WAV to disk before any network call so audio survives an app kill or
        // transient network failure.  The file is cleaned up after a successful STT call.
        val pendingWavFile = savePendingWav(wavBytes)

        // Use cached config from loadBubbleControls() (called moments ago by handleTap/longPress).
        // Fall back to a fresh read if the cache is somehow stale (e.g. auto-loop restart path).
        val config = cachedConfig ?: KlarvoApi.readConfig(this)
        val tConfig = System.currentTimeMillis()
        KlarvoLogger.d(TAG, "[pipeline] config read: ${tConfig - t0}ms")

        if (config == null || (config.sttProvider != "local" && config.groqApiKey.isBlank())) {
            handler.post {
                showToast("No API keys configured. Please open Klarvo and add your Groq key in Settings.")
                autoLoopActive = false
                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)
            }
            return
        }

        try {
            // Step 1: STT -- cloud (Groq) or local (whisper.cpp via JNI)
            val transcript = if (config.sttProvider == "local") {
                val tLocalStart = System.currentTimeMillis()

                // Resolve model file path.
                // Tauri's app_data_dir() maps to filesDir on Android,
                // so downloaded models land in filesDir/models/.
                // Fallback: also check dataDir/models/ in case of older builds.
                val filesDirModels = java.io.File(filesDir, "models")
                val dataDirModels = if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.N) {
                    java.io.File(dataDir, "models")
                } else {
                    java.io.File(applicationInfo.dataDir, "models")
                }
                KlarvoLogger.d(TAG, "[local-stt] filesDir/models: $filesDirModels exists=${filesDirModels.exists()}")
                KlarvoLogger.d(TAG, "[local-stt] dataDir/models: $dataDirModels exists=${dataDirModels.exists()}")

                val modelDir = when {
                    filesDirModels.exists() -> filesDirModels
                    dataDirModels.exists() -> dataDirModels
                    else -> filesDirModels  // default to filesDir (matches Tauri download path)
                }
                val modelFile = modelDir.resolve("ggml-small.bin")  // TODO: read model name from config

                KlarvoLogger.d(TAG, "[local-stt] model path: $modelFile, exists=${modelFile.exists()}")

                if (!modelFile.exists()) {
                    KlarvoLogger.e(TAG, "[local-stt] Whisper model not found: $modelFile")
                    handler.post {
                        showToast("Whisper model not downloaded. Please download in Settings.")
                        autoLoopActive = false
                        val prev = currentState
                        setState(RecordingState.IDLE)
                        adjustLayoutForState(RecordingState.IDLE, prev)
                    }
                    return
                }

                KlarvoLogger.d(TAG, "[local-stt] nativeAvailable=${LocalWhisperInference.isNativeAvailable()}, isModelLoaded=${LocalWhisperInference.isModelLoaded()}")

                if (!LocalWhisperInference.isModelLoaded()) {
                    KlarvoLogger.d(TAG, "[local-stt] loading model: ${modelFile.absolutePath}")
                    val loadOk = LocalWhisperInference.load(modelFile.absolutePath)
                    KlarvoLogger.d(TAG, "[local-stt] load result: $loadOk")
                    if (!loadOk) {
                        KlarvoLogger.e(TAG, "[local-stt] Failed to load whisper model: $modelFile")
                        handler.post {
                            showToast("Failed to load Whisper model")
                            autoLoopActive = false
                            val prev = currentState
                            setState(RecordingState.IDLE)
                            adjustLayoutForState(RecordingState.IDLE, prev)
                        }
                        return
                    }
                }

                val wavBase64 = android.util.Base64.encodeToString(wavBytes, android.util.Base64.NO_WRAP)
                KlarvoLogger.d(TAG, "[local-stt] calling transcribeAudio, base64 len=${wavBase64.length}, lang=${config.language}")
                val result = LocalWhisperInference.transcribeAudio(wavBase64, config.language)
                KlarvoLogger.d(TAG, "[local-stt] transcribeAudio result: '${result.take(100)}' (len=${result.length})")
                val tLocalEnd = System.currentTimeMillis()
                KlarvoLogger.d(TAG, "[pipeline] local STT: ${tLocalEnd - tLocalStart}ms (${wavBytes.size / 1024}KB audio)")

                if (result.isBlank()) {
                    KlarvoLogger.e(TAG, "Local transcription returned empty result")
                    handler.post {
                        showToast("Transcription failed")
                        autoLoopActive = false
                        val prev = currentState
                        setState(RecordingState.IDLE)
                        adjustLayoutForState(RecordingState.IDLE, prev)
                    }
                    return
                }
                result
            } else {
                transcribeWithRetry(
                    wavBytes,
                    config.groqApiKey,
                    config.language,
                    "whisper-large-v3-turbo", // H9: model comes from Rust config (sttModel not yet in Android AppConfig; default parity)
                    config.dictionaryTerms,
                    config.customPrompt,
                    pendingWavFile
                )
            }
            val tStt = System.currentTimeMillis()
            KlarvoLogger.d(TAG, "[pipeline] STT: ${tStt - tConfig}ms (${wavBytes.size / 1024}KB audio, provider=${config.sttProvider})")

            // STT succeeded -- safe to remove the pending WAV backup.
            pendingWavFile?.delete()

            if (transcript.isBlank()) {
                handler.post {
                    showToast("No speech detected")
                    autoLoopActive = false
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                }
                return
            }

            // Hallucination guard via shared Rust (ADR-0017, AC2).
            // Replaces HallucinationFilter.isHallucination() — same logic, single Rust source.
            if (GroqSttBridge.nativeIsHallucination(transcript)) {
                KlarvoLogger.d(TAG, "[pipeline] hallucination filtered (Rust): '${transcript.take(60)}'")
                handler.post {
                    showToast("Speech not recognized")
                    autoLoopActive = false
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                }
                return
            }

            // Tracks LLM cleanup latency for feedback metrics.
            // Remains null when cleanup is skipped or fails (no key, exception).
            var llmLatencyMs: Long? = null

            // Step 2: Text cleanup via configured LLM provider (optional -- skip if no key)
            val finalText = if (config.llmProvider == "local") {
                // Offline cleanup via MNN (local inference, no internet needed)
                try {
                    val result = KlarvoApi.cleanupLocal(this, transcript, config.cleanupStyle)
                    val tCleanup = System.currentTimeMillis()
                    llmLatencyMs = tCleanup - tStt
                    KlarvoLogger.d(TAG, "[pipeline] cleanup: ${tCleanup - tStt}ms (local/mnn)")
                    result
                } catch (e: Exception) {
                    KlarvoLogger.w(TAG, "Local cleanup failed -- using raw transcript", e)
                    KlarvoApi.sanitizeLlmOutput(transcript)
                }
            } else {
                val llmProvider = KlarvoApi.resolveLlmProvider(config)
                if (llmProvider != null) {
                    try {
                        val result = KlarvoApi.cleanupChunked(
                            text = transcript,
                            provider = llmProvider,
                            style = config.cleanupStyle,
                            dictionaryTerms = config.dictionaryTerms.takeIf { it.isNotBlank() },
                            customInstructions = config.customPrompt.takeIf { it.isNotBlank() }
                        )
                        val tCleanup = System.currentTimeMillis()
                        llmLatencyMs = tCleanup - tStt
                        KlarvoLogger.d(TAG, "[pipeline] cleanup: ${tCleanup - tStt}ms (${llmProvider.model})")
                        result
                    } catch (e: IOException) {
                        KlarvoLogger.w(TAG, "Text cleanup failed -- using raw transcript", e)
                        KlarvoApi.updateFeedbackMetrics(this) { m ->
                            m.copy(llmErrorCount = m.llmErrorCount + 1)
                        }
                        KlarvoApi.sanitizeLlmOutput(transcript)
                    }
                } else {
                    KlarvoLogger.d(TAG, "[pipeline] cleanup: skipped (no LLM provider key)")
                    // Notify the user that cleanup was skipped so they understand
                    // why the pasted text may still contain filler words or errors.
                    handler.post {
                        showToast("Text pasted without cleanup (no LLM key configured).")
                    }
                    KlarvoApi.sanitizeLlmOutput(transcript)
                }
            }

            // Step 3: Save to history DB
            val tBeforeHistory = System.currentTimeMillis()
            KlarvoApi.saveToHistory(
                context  = this,
                finalText = finalText,
                rawText  = transcript,
                style    = config.cleanupStyle,
                language = config.language,
                deviceId = config.deviceId
            )
            val tHistory = System.currentTimeMillis()
            KlarvoLogger.d(TAG, "[pipeline] history save: ${tHistory - tBeforeHistory}ms")
            KlarvoLogger.d(TAG, "[pipeline] total so far (after history): ${tHistory - t0}ms")

            // Step 3b: Push unsynced entries to Turso (fire-and-forget -- must not block paste)
            if (config.tursoUrl.isNotBlank() && config.tursoToken.isNotBlank()) {
                Thread {
                    try {
                        KlarvoApi.pushToTurso(this@KlarvoOverlayService, config.tursoUrl, config.tursoToken)
                    } catch (e: Exception) {
                        KlarvoLogger.w(TAG, "Turso sync failed (non-blocking)", e)
                    }
                }.start()
            }

            KlarvoLogger.d(TAG, "[pipeline] total before paste: ${System.currentTimeMillis() - t0}ms")

            // Step 4: Copy to clipboard and paste
            // Capture activeGesture before posting to main thread (it may change on next gesture).
            val gesture = activeGesture
            // Capture pipeline timing values for metrics (captured in lambda closure).
            val capturedT0          = t0
            val capturedTConfig     = tConfig
            val capturedTStt        = tStt
            val capturedLlmLatency  = llmLatencyMs
            val capturedTranscript  = transcript
            val capturedFinalText   = finalText
            handler.post {
                // DIV-04 fix: abort paste if a banking/security app is focused at paste time.
                // The pipeline may have started before the app-switch; this guard ensures
                // nothing reaches the clipboard or the accessibility paste path.
                if (BankingGuard.shouldBlockPaste(bankingAppActive)) {
                    showToast("Paste blocked — banking app active.")
                    autoLoopActive = false
                    val prev = currentState
                    setState(RecordingState.IDLE)
                    adjustLayoutForState(RecordingState.IDLE, prev)
                    return@post
                }

                copyToClipboard(finalText)

                val pasted = KlarvoAccessibilityService.instance != null
                KlarvoAccessibilityService.instance?.pasteIntoFocusedField()

                val preview = if (finalText.length > 50) finalText.take(50) + "..." else finalText
                if (pasted) showToast("Inserted: $preview") else showToast("Copied: $preview")

                // Write feedback metrics (fire-and-forget, off main thread).
                Thread {
                    KlarvoApi.updateFeedbackMetrics(this@KlarvoOverlayService) { m ->
                        m.copy(
                            lastSttLatencyMs   = capturedTStt - capturedTConfig,
                            lastLlmLatencyMs   = capturedLlmLatency,
                            lastTotalLatencyMs = System.currentTimeMillis() - capturedT0,
                            lastTargetApp      = null,   // Android has no foreground-window tracking
                            lastDictationAt    = java.text.SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss'Z'", java.util.Locale.US).also { it.timeZone = java.util.TimeZone.getTimeZone("UTC") }.format(java.util.Date()),
                            lastRawText        = capturedTranscript,
                            lastCleanedText    = capturedFinalText
                            // Error counts left unchanged -- copy() retains them.
                        )
                    }
                }.start()

                val prev = currentState
                setState(RecordingState.IDLE)
                adjustLayoutForState(RecordingState.IDLE, prev)

                // Auto-send (press Enter) if configured for this gesture.
                val shouldAutoSend = when (gesture) {
                    "tap"       -> tapAutoSend
                    "longpress" -> longPressAutoSend
                    else        -> false
                }
                if (shouldAutoSend && pasted) {
                    // Short delay so the pasted text is committed before Enter fires.
                    handler.postDelayed({
                        KlarvoAccessibilityService.instance?.performEnter()
                    }, 150)
                }

                // AUTO mode: restart recording for next segment
                val activeMode = when (gesture) {
                    "longpress" -> longPressMode
                    else        -> tapMode
                }
                if (autoLoopActive && activeMode == RecordingMode.AUTO) {
                    startRecording()
                }
            }

        } catch (e: IOException) {
            KlarvoLogger.w(TAG, "STT/API pipeline failed", e)
            // Increment STT error counter (this catch covers STT failures;
            // LLM IOException is caught earlier and increments llmErrorCount there).
            KlarvoApi.updateFeedbackMetrics(this) { m ->
                m.copy(sttErrorCount = m.sttErrorCount + 1)
            }
            // If a pending WAV still exists (retries exhausted) let the user know it was
            // preserved so no audio is silently lost.
            val savedMsg = if (pendingWavFile?.exists() == true) " Recording saved." else ""
            handler.post {
                showToast("Error: ${e.message?.take(80)}$savedMsg")
                autoLoopActive = false
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
     * Re-reads bubble size, opacity, and recording controls from config.json.
     * Called on every return to IDLE so Settings changes take effect after the next dictation.
     *
     * Bubble visual size uses the responsive formula (computeVisualSizeDp) as of Story 9.3;
     * config.bubbleSize scale factor is no longer applied here.
     */
    private fun reloadBubbleAppearance() {
        val config = KlarvoApi.readConfig(this) ?: return
        val newSizeDp = computeVisualSizeDp()
        bubbleOpacity = config.bubbleOpacity
        bubbleView.setBubbleSize(newSizeDp)
        bubbleView.alpha = bubbleOpacity

        // Touch-target expansion: keep LayoutParams in sync with the visual size
        val touchTargetDp = maxOf(newSizeDp, 48)
        val touchTargetPx = (touchTargetDp * resources.displayMetrics.density).toInt()
        bubbleParams.width  = touchTargetPx
        bubbleParams.height = touchTargetPx

        updateBubbleLayout()
        loadBubbleControls()
        updateNotification()
    }

    private fun copyToClipboard(text: String) {
        val clipboard = getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val clip      = ClipData.newPlainText("Klarvo transcription", text)
        clipboard.setPrimaryClip(clip)
    }

    private fun showToast(message: String) {
        Toast.makeText(this, message, Toast.LENGTH_SHORT).show()
    }

    // ---- Data-loss prevention helpers ----

    /**
     * Writes the WAV bytes to {dataDir}/pending/<timestamp>.wav before any network call.
     * Returns the File, or null if the write fails (non-fatal -- pipeline continues).
     */
    private fun savePendingWav(wavBytes: ByteArray): File? {
        return try {
            val pendingDir = File(dataDir, "pending")
            pendingDir.mkdirs()
            val f = File(pendingDir, "${System.currentTimeMillis()}.wav")
            f.writeBytes(wavBytes)
            KlarvoLogger.d(TAG, "[pending-wav] saved ${wavBytes.size / 1024}KB to ${f.name}")
            f
        } catch (e: IOException) {
            KlarvoLogger.w(TAG, "[pending-wav] failed to save backup WAV", e)
            null
        }
    }

    /**
     * Transcribes [wavBytes] via the shared Rust Groq STT path (GroqSttBridge.nativeTranscribe)
     * with up to 2 retries (delays: 2 s, 5 s) for network errors.
     *
     * Retry contract preserved from the old KlarvoApi.transcribe path:
     * - 4xx HTTP errors are NOT retried (bad request / auth failure).
     * - Network errors (all other failures) are retried up to 2 times.
     *
     * ADR-0017: KlarvoApi.transcribe + buildMultipartBody deleted; this method
     * now calls GroqSttBridge.nativeTranscribe which runs the shared Rust WhisperStt path.
     */
    private fun transcribeWithRetry(
        wavBytes: ByteArray,
        apiKey: String,
        language: String,
        sttModel: String,
        dictionaryTerms: String,
        customPrompt: String,
        pendingWavFile: File?
    ): String {
        val wavBase64 = android.util.Base64.encodeToString(wavBytes, android.util.Base64.NO_WRAP)
        val retryDelaysMs = listOf(2_000L, 5_000L)
        var lastErrorMsg: String? = null

        for (attempt in 0..retryDelaysMs.size) {
            val result = GroqSttBridge.nativeTranscribe(
                wavBase64 = wavBase64,
                apiKey = apiKey,
                language = language,
                dictionaryTerms = dictionaryTerms,
                customPrompt = customPrompt,
                sttModel = sttModel,
                temperature = 0.0f
            )

            when {
                // Success: non-error, non-empty result.
                !result.startsWith("__ERROR_") -> return result

                // Empty audio — not retriable.
                result == "__ERROR_EMPTY_AUDIO__" -> {
                    KlarvoLogger.w(TAG, "[stt-retry] empty audio error -- not retrying")
                    throw IOException("Groq STT: empty audio")
                }

                // API error: only 4xx is non-retriable (bad request / invalid key / quota).
                // 5xx (server error / overload) falls through to the retry path below.
                result.startsWith("__ERROR_API:") -> {
                    val msg = result.removeSurrounding("__ERROR_API:", "__")
                    // Parse the HTTP status from the embedded "HTTP <status>: ..." message.
                    val statusCode = Regex("HTTP (\\d{3})").find(msg)?.groupValues?.getOrNull(1)?.toIntOrNull()
                    val is4xx = statusCode != null && statusCode in 400..499
                    if (is4xx) {
                        KlarvoLogger.w(TAG, "[stt-retry] 4xx API error -- not retrying: $msg")
                        throw IOException("Groq STT failed: $msg")
                    } else {
                        // 5xx or unparseable status — treat as retriable (mirror network retry path).
                        lastErrorMsg = msg
                        if (attempt < retryDelaysMs.size) {
                            val delay = retryDelaysMs[attempt]
                            KlarvoLogger.w(TAG, "[stt-retry] 5xx/server error (attempt $attempt, $msg), retrying in ${delay}ms")
                            Thread.sleep(delay)
                        } else {
                            KlarvoLogger.e(TAG, "[stt-retry] all retries exhausted (5xx), pending WAV kept: ${pendingWavFile?.name}")
                        }
                    }
                }

                // Network error — retriable.
                result.startsWith("__ERROR_NETWORK:") -> {
                    val msg = result.removeSurrounding("__ERROR_NETWORK:", "__")
                    lastErrorMsg = msg
                    if (attempt < retryDelaysMs.size) {
                        val delay = retryDelaysMs[attempt]
                        KlarvoLogger.w(TAG, "[stt-retry] attempt $attempt failed ($msg), retrying in ${delay}ms")
                        Thread.sleep(delay)
                    } else {
                        KlarvoLogger.e(TAG, "[stt-retry] all retries exhausted, pending WAV kept: ${pendingWavFile?.name}")
                    }
                }

                // Unknown error code — treat as network error, retriable.
                else -> {
                    lastErrorMsg = result
                    if (attempt < retryDelaysMs.size) {
                        val delay = retryDelaysMs[attempt]
                        KlarvoLogger.w(TAG, "[stt-retry] unknown error ($result), retrying in ${delay}ms")
                        Thread.sleep(delay)
                    }
                }
            }
        }
        throw IOException("Groq STT failed after retries: $lastErrorMsg")
    }

    // shouldBlockPaste delegates to BankingGuard (see BankingGuard.kt) so that
    // unit tests can reach the real decision site without an Android context (AI-2).

    /**
     * Deletes pending WAV files older than 7 days.
     * Called once at service startup to keep the pending directory clean.
     */
    private fun cleanupStalePendingWavFiles() {
        try {
            val pendingDir = File(dataDir, "pending")
            if (!pendingDir.exists()) return
            val cutoff = System.currentTimeMillis() - 7L * 24 * 60 * 60 * 1000
            var deleted = 0
            pendingDir.listFiles()?.forEach { f ->
                if (f.isFile && f.lastModified() < cutoff) {
                    f.delete()
                    deleted++
                }
            }
            if (deleted > 0) {
                KlarvoLogger.i(TAG, "[pending-wav] cleaned up $deleted stale WAV file(s)")
            }
        } catch (e: Exception) {
            KlarvoLogger.w(TAG, "[pending-wav] cleanup failed", e)
        }
    }
}
