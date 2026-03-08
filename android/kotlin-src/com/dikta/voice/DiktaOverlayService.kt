package com.dikta.voice

import android.app.*
import android.content.*
import android.content.pm.ServiceInfo
import android.graphics.PixelFormat
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.os.*
import android.util.DisplayMetrics
import android.view.*
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
 * - Handles touch events: drag vs. tap detection (threshold: 10dp)
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

        // Audio recording parameters
        private const val SAMPLE_RATE = 16000
        private const val CHANNEL_CONFIG = AudioFormat.CHANNEL_IN_MONO
        private const val AUDIO_FORMAT = AudioFormat.ENCODING_PCM_16BIT

        // Keyboard detection: if >15% of screen height is hidden, keyboard is likely open
        private const val KEYBOARD_HEIGHT_RATIO = 0.15f
        // Polling interval for keyboard detection (ms)
        private const val KEYBOARD_CHECK_INTERVAL = 300L

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
        val builder = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
        }
        return builder
            .setContentTitle("Dikta - Voice Dictation")
            .setContentText("Tap the bubble to start dictating")
            .setSmallIcon(android.R.drawable.ic_btn_speak_now)
            .setOngoing(true)
            .build()
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

    private fun showBubble() {
        if (!isBubbleVisible && ::bubbleView.isInitialized) {
            try {
                windowManager.addView(bubbleView, bubbleParams)
                isBubbleVisible = true
            } catch (_: Exception) {}
        }
    }

    private fun hideBubble() {
        if (isBubbleVisible && ::bubbleView.isInitialized) {
            try {
                windowManager.removeView(bubbleView)
                isBubbleVisible = false
            } catch (_: Exception) {}
        }
    }

    // --- Bubble setup ---

    /**
     * Prepares the bubble view and layout params. Bubble starts HIDDEN;
     * it appears when the keyboard detector sees the soft keyboard open.
     */
    private fun setupBubble() {
        bubbleView = FloatingBubbleView(this)

        val (screenW, screenH) = getScreenDimensions()
        val dp = resources.displayMetrics.density
        val bubbleSize = (56 * dp).toInt()
        val marginPx = (16 * dp).toInt()

        val prefs = getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE)
        val savedX = prefs.getInt(PREF_X, screenW - bubbleSize - marginPx)
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
                return true
            }
            MotionEvent.ACTION_MOVE -> {
                val dx = event.rawX - dragTouchStartX
                val dy = event.rawY - dragTouchStartY
                if (!isDragging && (abs(dx) > dragThresholdPx || abs(dy) > dragThresholdPx)) {
                    isDragging = true
                }
                if (isDragging) {
                    bubbleParams.x = (bubbleStartX + dx).toInt()
                    bubbleParams.y = (bubbleStartY + dy).toInt()
                    windowManager.updateViewLayout(bubbleView, bubbleParams)
                }
                return true
            }
            MotionEvent.ACTION_UP -> {
                if (isDragging) {
                    savePosition(bubbleParams.x, bubbleParams.y)
                } else {
                    handleTap()
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
        when (currentState) {
            RecordingState.IDLE -> startRecording()
            RecordingState.RECORDING -> stopRecording()
            RecordingState.PROCESSING -> { /* ignore taps while processing */ }
        }
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
                language = config.language
            )

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
