package com.klarvo.voice

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import androidx.core.content.ContextCompat
import com.konovalov.vad.silero.VadSilero
import com.konovalov.vad.silero.config.FrameSize
import com.konovalov.vad.silero.config.Mode
import com.konovalov.vad.silero.config.SampleRate
import kotlin.math.sqrt

/**
 * Manages audio capture from the microphone.
 *
 * Usage:
 *   val recorder = KlarvoAudioRecorder(context) { amplitude -> updateWaveform(amplitude) }
 *   recorder.start()
 *   ...
 *   val wavBytes = recorder.stop()  // returns WAV-encoded bytes, ready for STT API
 *
 * The [onAmplitude] callback fires on the recording thread for every audio chunk,
 * delivering a normalized RMS value in [0, 1]. Callers must post UI updates to
 * the main thread themselves (e.g. via Handler.post).
 *
 * [stop] blocks for up to 500 ms waiting for the recording thread to finish, then
 * releases the [AudioRecord] and returns the captured data as a WAV byte array.
 * If no audio was captured, an empty ByteArray is returned.
 *
 * Silence detection: previously RMS-based (compare chunk RMS against SILENCE_THRESHOLD).
 * Now uses Silero VAD v5 (android-vad library) for neural voice activity detection.
 * The RMS energy gate is kept as a pre-filter: frames below SILENCE_THRESHOLD are
 * treated as silence without even calling the VAD model, saving CPU.
 */
class KlarvoAudioRecorder(
    private val context: Context,
    private val onAmplitude: (Float) -> Unit,
    /**
     * Seconds of continuous silence required to trigger [onSilenceDetected].
     * Defaults to 2.0s.
     */
    private val silenceSecs: Float = 2.0f,
    /**
     * RMS energy gate threshold (normalized 0..1). Frames below this are treated
     * as silence without calling the VAD model. Defaults to 0.005, which matches
     * the Rust default_silence_threshold() in src-tauri/src/config/mod.rs:209.
     *
     * Set from config.json "advanced.silenceThreshold" by KlarvoOverlayService so
     * the user's desktop slider setting is honored on Android (AC1/AC2/AC3, Story 9-11).
     * The previous hard-coded 0.02 was 4× the desktop default, causing ~10% of quiet
     * utterances to be missed on a typical device microphone.
     */
    private var energyGateThreshold: Float = DEFAULT_ENERGY_GATE_THRESHOLD
) {

    companion object {
        private const val TAG = "KlarvoAudioRecorder"

        private const val SAMPLE_RATE = 16000
        private const val CHANNEL_CONFIG = AudioFormat.CHANNEL_IN_MONO
        private const val AUDIO_FORMAT = AudioFormat.ENCODING_PCM_16BIT

        /**
         * Default energy gate threshold (normalized RMS 0..1).
         * Matches Rust default_silence_threshold() = 0.005 in src-tauri/src/config/mod.rs:209.
         * The old hard-coded 0.02 was 4× the desktop default and missed quiet speech on
         * a typical device microphone (Story 9-11 root cause). This constant is kept as the
         * default for [energyGateThreshold] but SHOULD NOT be used directly in logic — always
         * use the instance field [energyGateThreshold] so config overrides work (AC1/AC3).
         */
        const val DEFAULT_ENERGY_GATE_THRESHOLD = 0.005f

        /**
         * Pure energy-gate predicate. Extracted from [processVadFrame] so JVM unit tests can
         * verify the gate uses the provided threshold rather than a hard-coded value (AC5a).
         *
         * Returns true when [normalizedRms] is at or above [threshold], meaning the frame has
         * enough energy to be passed to the VAD model.
         *
         * @param normalizedRms  Per-frame RMS normalized to [0, 1] (raw RMS / 32768).
         * @param threshold      Energy gate threshold; caller supplies the configured value.
         */
        fun isEnergyAboveGate(normalizedRms: Float, threshold: Float): Boolean =
            normalizedRms >= threshold

        // Silero VAD requires exactly 512 samples per frame at 16 kHz (~32 ms/frame).
        private const val VAD_FRAME_SIZE = 512

        // Onset hysteresis: require this many consecutive VAD-speech frames before
        // we consider speech to have started. Prevents single-frame false positives.
        // 3 frames * 32 ms/frame = ~96 ms onset latency.
        private const val VAD_ONSET_FRAMES = 3

        // Hangover frames: VAD frames per second at 16 kHz / 512 samples = ~31.25 fps.
        // Used to convert silenceSecs into a frame count.
        private const val VAD_FRAMES_PER_SECOND = 31
    }

    init {
        // Clamp the configured energy-gate threshold to the slider's declared range
        // [0.001f, 0.1f] so neither failure mode is reachable:
        //   • threshold == 0f  → normalizedRms >= 0f is always true → gate fully disabled
        //     (reachable: desktop "Silence threshold" slider uses parseFloat(...)||0, so a
        //     blank/invalid input yields 0).
        //   • threshold > 1.0f → normalizedRms is coerceIn(0f,1f), so gate always closed
        //     → isSpeechFrame always false → auto-stop silently dies.
        // 0.001f floor keeps at least minimal filtering; 0.1f == desktop slider max.
        // Default 0.005f is within [0.001f, 0.1f], so default behavior is unchanged.
        energyGateThreshold = energyGateThreshold.coerceIn(0.001f, 0.1f)
    }

    /**
     * Computed required-silent-frames from the silenceSecs constructor parameter.
     * Each frame is 512 samples = ~32 ms at 16 kHz.
     */
    private val requiredSilentFrames: Int
        get() = (silenceSecs * VAD_FRAMES_PER_SECOND).toInt().coerceAtLeast(1)

    /**
     * Optional callback fired once when sustained silence is detected after speech.
     * Set by KlarvoOverlayService for AUTOSTOP / AUTO modes.
     * Fires on the recording thread -- caller must post to main thread.
     */
    var onSilenceDetected: (() -> Unit)? = null

    private var audioRecord: AudioRecord? = null
    private val pcmBuffer = ArrayList<Short>()
    private var recordingThread: Thread? = null
    private var isCapturing = false

    // Silero VAD instance -- initialized in start(), released in stop()/releaseImmediately().
    // Previously there was no VAD object; silence was detected purely by RMS comparison.
    private var vad: VadSilero? = null

    // Ring buffer for feeding exactly VAD_FRAME_SIZE samples to Silero.
    // AudioRecord delivers variable-size chunks; we accumulate them here.
    private val vadRingBuffer = ShortArray(VAD_FRAME_SIZE)
    private var vadRingPos = 0

    // VAD silence detection state (replaces the old chunk-count-based state)
    private var silentFrames = 0
    private var silenceCallbackFired = false
    // Consecutive speech frames seen since last silence period (onset hysteresis).
    private var onsetFrames = 0
    // True once enough consecutive speech frames have been seen (onset confirmed).
    private var speechDetected = false

    // --- Auto-mode silence-fire diagnostics (Story 9-7 follow-up, 2026-06-16) ---
    // Purely observational: aggregate per ~1s window so one device repro reveals why
    // onSilenceDetected never fires (VAD sees no speech? requiredSilentFrames too big?
    // silentFrames keeps resetting?). No behavior change — logging only.
    private var dbgWindowFrames = 0
    private var dbgSpeechFrames = 0
    private var dbgRmsMax = 0f
    private var dbgVadTrue = 0

    // Rolling average for amplitude smoothing (last 3 values).
    private val amplitudeHistory = FloatArray(3) { 0f }
    private var amplitudeHistoryIndex = 0

    /**
     * Returns true if [start] has been called and [stop] has not yet returned.
     */
    val isRecording: Boolean
        get() = isCapturing

    /**
     * Starts capturing audio from the microphone.
     *
     * Throws [IllegalStateException] if the microphone is unavailable or permissions
     * are missing. The caller (KlarvoOverlayService) handles the error and shows a toast.
     */
    fun start() {
        // Verify runtime permission before accessing the microphone.
        if (ContextCompat.checkSelfPermission(context, Manifest.permission.RECORD_AUDIO)
            != PackageManager.PERMISSION_GRANTED
        ) {
            throw SecurityException("RECORD_AUDIO permission not granted")
        }

        val minBufSize = AudioRecord.getMinBufferSize(SAMPLE_RATE, CHANNEL_CONFIG, AUDIO_FORMAT)
        if (minBufSize == AudioRecord.ERROR || minBufSize == AudioRecord.ERROR_BAD_VALUE) {
            throw IllegalStateException("AudioRecord.getMinBufferSize returned error: $minBufSize")
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
            throw IllegalStateException("AudioRecord failed to initialize -- microphone not available")
        }

        // Initialize Silero VAD.
        // Previously: no VAD object, silence was detected by comparing chunk RMS to SILENCE_THRESHOLD.
        // Now: VadSilero processes 512-sample frames; we implement onset/hangover hysteresis manually
        // (the library's built-in speechDurationMs/silenceDurationMs would be an alternative but
        // manual control keeps the logic consistent with the Desktop Rust implementation).
        vad = VadSilero(
            context,
            sampleRate = SampleRate.SAMPLE_RATE_16K,
            frameSize = FrameSize.FRAME_SIZE_512,
            mode = Mode.NORMAL
        )

        audioRecord = recorder
        pcmBuffer.clear()
        isCapturing = true

        recorder.startRecording()

        // Reset smoothing state for the new recording session.
        amplitudeHistory.fill(0f)
        amplitudeHistoryIndex = 0

        // Reset VAD silence detection state.
        vadRingPos = 0
        silentFrames = 0
        silenceCallbackFired = false
        onsetFrames = 0
        speechDetected = false
        dbgWindowFrames = 0
        dbgSpeechFrames = 0
        dbgRmsMax = 0f
        dbgVadTrue = 0
        KlarvoLogger.d(TAG, "VAD config: silenceSecs=$silenceSecs requiredSilentFrames=$requiredSilentFrames " +
            "energyGate=$energyGateThreshold onSilenceDetected=${onSilenceDetected != null}")

        recordingThread = Thread {
            val buf = ShortArray(bufferSize / 2)
            while (isCapturing) {
                val read = recorder.read(buf, 0, buf.size)
                if (read > 0) {
                    // Accumulate PCM for WAV output.
                    for (i in 0 until read) {
                        pcmBuffer.add(buf[i])
                    }

                    // Compute RMS for waveform visualization (unchanged from before).
                    val rms = calculateRms(buf, read)
                    val smoothedAmp = smoothedAmplitude(rms)
                    onAmplitude(smoothedAmp)

                    // Feed samples into the VAD ring buffer in 512-sample frames.
                    if (onSilenceDetected != null && !silenceCallbackFired) {
                        feedVad(buf, read)
                    }
                }
            }
        }.also { it.start() }

        KlarvoLogger.d(TAG,"Recording started (bufferSize=$bufferSize, sampleRate=$SAMPLE_RATE, VAD=Silero)")
    }

    /**
     * Feeds [count] samples from [buf] into the VAD ring buffer.
     * Whenever the ring buffer is full (512 samples), runs a VAD inference step
     * and updates the silence detection state machine.
     *
     * Previously: silence was detected by comparing per-chunk RMS to SILENCE_THRESHOLD.
     * Now: samples are batched into 512-sample frames; each frame goes through the
     * energy gate first, then through VadSilero if the gate passes.
     */
    private fun feedVad(buf: ShortArray, count: Int) {
        var srcPos = 0
        while (srcPos < count) {
            // How many samples fit until the ring buffer is full?
            val space = VAD_FRAME_SIZE - vadRingPos
            val toCopy = minOf(space, count - srcPos)
            System.arraycopy(buf, srcPos, vadRingBuffer, vadRingPos, toCopy)
            vadRingPos += toCopy
            srcPos += toCopy

            if (vadRingPos == VAD_FRAME_SIZE) {
                // Ring buffer full: process one 512-sample VAD frame.
                vadRingPos = 0
                processVadFrame(vadRingBuffer)
            }
        }
    }

    /**
     * Runs the VAD on one complete 512-sample frame and updates the silence state machine.
     *
     * State machine (mirrors Desktop Rust implementation):
     *
     *   BEFORE SPEECH CONFIRMED (speechDetected == false):
     *     - Energy gate below SILENCE_THRESHOLD → onsetFrames = 0 (no speech)
     *     - VAD returns true                   → onsetFrames++
     *     - onsetFrames >= VAD_ONSET_FRAMES     → speechDetected = true, silentFrames = 0
     *
     *   AFTER SPEECH CONFIRMED (speechDetected == true):
     *     - Energy gate below threshold OR VAD false → silentFrames++
     *     - VAD true                                 → silentFrames = 0 (hangover reset)
     *     - silentFrames >= requiredSilentFrames     → fire onSilenceDetected
     *
     * Previously: a single RMS threshold (SILENCE_THRESHOLD = 0.03) determined
     * speech vs. silence for entire AudioRecord chunks (~8192/2 = 4096 samples).
     * Now: VAD model runs on 512-sample frames with onset and hangover hysteresis.
     */
    private fun processVadFrame(frame: ShortArray) {
        // AC4 guard: silenceCallbackFired is also checked in the start() loop, but that
        // outer check fires only between audio buffers. Within a single buffer, feedVad()
        // may call processVadFrame() multiple times after the first fire (remaining frames
        // in the same batch). This inner guard ensures the callback fires at most once per
        // session, regardless of how many frames cross the threshold in the same buffer.
        if (silenceCallbackFired) return

        // Energy gate: avoid calling the ONNX model for clearly silent frames.
        // Uses the instance energyGateThreshold (from config.json "advanced.silenceThreshold")
        // instead of the old hard-coded 0.02 constant, so user preferences are honored (AC1/AC3).
        val rms = calculateRms(frame, frame.size)
        val normalizedRms = (rms / 32768f).coerceIn(0f, 1f)
        val energyAboveGate = isEnergyAboveGate(normalizedRms, energyGateThreshold)

        val vadSpeech = vad?.isSpeech(frame) == true
        val isSpeechFrame = energyAboveGate && vadSpeech

        // --- diagnostics: aggregate per ~1s window (Story 9-7 follow-up) ---
        dbgWindowFrames++
        if (normalizedRms > dbgRmsMax) dbgRmsMax = normalizedRms
        if (vadSpeech) dbgVadTrue++
        if (isSpeechFrame) dbgSpeechFrames++
        if (dbgWindowFrames >= VAD_FRAMES_PER_SECOND) {
            KlarvoLogger.d(TAG, "VAD ~1s: rmsMax=${"%.3f".format(dbgRmsMax)} vadTrue=$dbgVadTrue " +
                "speechFrames=$dbgSpeechFrames speechDetected=$speechDetected " +
                "onsetFrames=$onsetFrames silentFrames=$silentFrames/$requiredSilentFrames")
            dbgWindowFrames = 0
            dbgSpeechFrames = 0
            dbgRmsMax = 0f
            dbgVadTrue = 0
        }

        if (!speechDetected) {
            // Onset phase: accumulate consecutive speech frames.
            if (isSpeechFrame) {
                onsetFrames++
                if (onsetFrames >= VAD_ONSET_FRAMES) {
                    speechDetected = true
                    silentFrames = 0
                    KlarvoLogger.d(TAG,"VAD: speech onset confirmed (onsetFrames=$onsetFrames)")
                }
            } else {
                // Any non-speech frame resets the onset counter.
                onsetFrames = 0
            }
        } else {
            // Hangover phase: count silence frames after speech was detected.
            if (isSpeechFrame) {
                silentFrames = 0
            } else {
                silentFrames++
                if (silentFrames >= requiredSilentFrames) {
                    silenceCallbackFired = true
                    KlarvoLogger.d(TAG,"VAD: silence detected after speech ($silentFrames frames >= $requiredSilentFrames required)")
                    onSilenceDetected?.invoke()
                }
            }
        }
    }

    /**
     * Stops capturing, releases [AudioRecord] and [VadSilero], and returns the recorded audio
     * encoded as a WAV byte array (16-bit mono, 16 kHz).
     *
     * Blocks for up to 500 ms waiting for the recording thread to finish cleanly.
     * Safe to call from any thread.
     *
     * Returns an empty [ByteArray] if [start] was never called or no samples were captured.
     */
    fun stop(): ByteArray {
        isCapturing = false

        try {
            recordingThread?.join(500)
        } catch (e: InterruptedException) {
            KlarvoLogger.w(TAG,"Interrupted while waiting for recording thread to finish", e)
            Thread.currentThread().interrupt()
        }
        recordingThread = null

        val recorder = audioRecord
        audioRecord = null
        try {
            recorder?.stop()
        } catch (e: Exception) {
            KlarvoLogger.w(TAG,"Failed to stop AudioRecord cleanly", e)
        }
        try {
            recorder?.release()
        } catch (e: Exception) {
            KlarvoLogger.w(TAG,"Failed to release AudioRecord", e)
        }

        // Release VAD resources (closes the ONNX runtime session).
        try {
            vad?.close()
        } catch (e: Exception) {
            KlarvoLogger.w(TAG,"Failed to close VadSilero", e)
        }
        vad = null

        val pcmData = pcmBuffer.toShortArray()
        pcmBuffer.clear()

        KlarvoLogger.d(TAG,"Recording stopped (${pcmData.size} samples captured)")

        if (pcmData.isEmpty()) return ByteArray(0)

        return encodeWav(pcmData, SAMPLE_RATE)
    }

    /**
     * Emergency release -- called from Service.onDestroy when we need to tear down
     * without waiting for a clean stop. Does not return WAV data.
     */
    fun releaseImmediately() {
        isCapturing = false
        recordingThread?.interrupt()
        recordingThread = null
        try {
            audioRecord?.stop()
        } catch (e: Exception) {
            KlarvoLogger.w(TAG,"releaseImmediately: failed to stop AudioRecord", e)
        }
        try {
            audioRecord?.release()
        } catch (e: Exception) {
            KlarvoLogger.w(TAG,"releaseImmediately: failed to release AudioRecord", e)
        }
        audioRecord = null
        try {
            vad?.close()
        } catch (e: Exception) {
            KlarvoLogger.w(TAG,"releaseImmediately: failed to close VadSilero", e)
        }
        vad = null
        pcmBuffer.clear()
    }

    private fun calculateRms(buffer: ShortArray, length: Int): Float {
        if (length == 0) return 0f
        var sum = 0.0
        for (i in 0 until length) {
            sum += buffer[i].toDouble() * buffer[i].toDouble()
        }
        return sqrt(sum / length).toFloat()
    }

    // TEMPORARY diagnostic log state — removed before close-out (Story 9-12 GATE-4 re-run).
    // One log line per ~1 second of recording (throttled by buffer-read count).
    // Buffer = 4096 shorts at 16000 Hz ≈ 4 reads/sec → fire every 4 calls.
    private var diagSampleCount = 0
    private val diagLogIntervalSamples = 4  // ~1 s at ~4 buffer reads/sec

    /**
     * Converts a raw RMS value (0..32768) into a noise-gated, amplified, smoothed
     * amplitude in [0, 1] suitable for waveform display.
     *
     * Recalibrated 2026-06-21 (Story 9-12 GATE-4 re-run):
     * - Old noise floor 0.04 (= raw RMS ≈1311) gated normal phone speech to ≈0.
     * - New noise floor 0.005 (= raw RMS ≈164) sits well below typical speech;
     *   aligns with the VAD energy-gate DEFAULT_ENERGY_GATE_THRESHOLD (0.005).
     * - Speech band [0.005..0.15] remapped and × 4.0 to fill [0..1] visibly;
     *   clamped at 1 — louder speech saturates cleanly.
     * - 3-sample rolling average retained to remove frame-to-frame jitter.
     *
     * Call sites: DISPLAY ONLY (onAmplitude → bubbleView/panelView). The VAD path
     * uses normalizedRms + isEnergyAboveGate independently — this function is safe
     * to tune without touching silence detection.
     */
    private fun smoothedAmplitude(rawRms: Float): Float {
        val normalized = (rawRms / 32768f).coerceIn(0f, 1f)

        // Noise floor: anything below this is treated as silence for display.
        // 0.012 ≈ raw RMS 393 — gates steady background hum to 0 while staying well
        // below normal/quiet speech (typically ≥0.03 normalized on phone mics).
        val noiseFloor = 0.012f

        val gated = if (normalized < noiseFloor) {
            0f
        } else {
            // Remap [noiseFloor..0.15] -> [0..1] with ×4 gain so normal speech (which
            // sits roughly in the 0.01..0.10 normalized band on phone mics) maps to a
            // clearly visible range. Louder speech saturates at 1 — that's fine.
            val remapped = (normalized - noiseFloor) / (0.15f - noiseFloor)
            (remapped * 4.0f).coerceIn(0f, 1f)
        }

        // Rolling average over the last 3 samples.
        amplitudeHistory[amplitudeHistoryIndex % amplitudeHistory.size] = gated
        amplitudeHistoryIndex++
        val smoothed = amplitudeHistory.average().toFloat()

        // TEMPORARY diagnostic log — tag: KLARVO_AMP_DIAG — remove before close-out (9-12).
        // Format: rawRMS | normalized | smoothedAmp  (throttled to ~1× per second)
        diagSampleCount += 1  // increment by 1 per call; interval = ~4 reads/sec
        if (diagSampleCount >= diagLogIntervalSamples) {
            diagSampleCount = 0
            KlarvoLogger.d(
                "KLARVO_AMP_DIAG",
                "rawRMS=%.1f normalized=%.4f smoothedAmp=%.3f".format(rawRms, normalized, smoothed)
            )
        }

        return smoothed
    }
}
