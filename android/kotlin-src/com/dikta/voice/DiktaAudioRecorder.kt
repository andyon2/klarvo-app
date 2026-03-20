package com.dikta.voice

import android.content.Context
import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.util.Log
import com.konovalov.vad.silero.VadSilero
import com.konovalov.vad.silero.config.FrameSize
import com.konovalov.vad.silero.config.Mode
import com.konovalov.vad.silero.config.SampleRate
import kotlin.math.sqrt

/**
 * Manages audio capture from the microphone.
 *
 * Usage:
 *   val recorder = DiktaAudioRecorder(context) { amplitude -> updateWaveform(amplitude) }
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
class DiktaAudioRecorder(
    private val context: Context,
    private val onAmplitude: (Float) -> Unit,
    /**
     * Seconds of continuous silence required to trigger [onSilenceDetected].
     * Defaults to 2.0s.
     */
    private val silenceSecs: Float = 2.0f
) {

    companion object {
        private const val TAG = "DiktaAudioRecorder"

        private const val SAMPLE_RATE = 16000
        private const val CHANNEL_CONFIG = AudioFormat.CHANNEL_IN_MONO
        private const val AUDIO_FORMAT = AudioFormat.ENCODING_PCM_16BIT

        // Energy gate: RMS (normalized 0..1) below this is treated as silence without
        // calling the VAD model. Previously this was the only silence threshold.
        // Kept lower than the old value (0.03) to let VAD handle borderline cases.
        private const val SILENCE_THRESHOLD = 0.02f

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

    /**
     * Computed required-silent-frames from the silenceSecs constructor parameter.
     * Each frame is 512 samples = ~32 ms at 16 kHz.
     */
    private val requiredSilentFrames: Int
        get() = (silenceSecs * VAD_FRAMES_PER_SECOND).toInt().coerceAtLeast(1)

    /**
     * Optional callback fired once when sustained silence is detected after speech.
     * Set by DiktaOverlayService for AUTOSTOP / AUTO modes.
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
     * are missing. The caller (DiktaOverlayService) handles the error and shows a toast.
     */
    fun start() {
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

        Log.d(TAG, "Recording started (bufferSize=$bufferSize, sampleRate=$SAMPLE_RATE, VAD=Silero)")
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
        // Energy gate: avoid calling the ONNX model for clearly silent frames.
        val rms = calculateRms(frame, frame.size)
        val normalizedRms = (rms / 32768f).coerceIn(0f, 1f)
        val energyAboveGate = normalizedRms >= SILENCE_THRESHOLD

        val isSpeechFrame = energyAboveGate && (vad?.isSpeech(frame) == true)

        if (!speechDetected) {
            // Onset phase: accumulate consecutive speech frames.
            if (isSpeechFrame) {
                onsetFrames++
                if (onsetFrames >= VAD_ONSET_FRAMES) {
                    speechDetected = true
                    silentFrames = 0
                    Log.d(TAG, "VAD: speech onset confirmed (onsetFrames=$onsetFrames)")
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
                    Log.d(TAG, "VAD: silence detected after speech ($silentFrames frames >= $requiredSilentFrames required)")
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
            Log.w(TAG, "Interrupted while waiting for recording thread to finish", e)
            Thread.currentThread().interrupt()
        }
        recordingThread = null

        val recorder = audioRecord
        audioRecord = null
        try {
            recorder?.stop()
        } catch (e: Exception) {
            Log.w(TAG, "Failed to stop AudioRecord cleanly", e)
        }
        try {
            recorder?.release()
        } catch (e: Exception) {
            Log.w(TAG, "Failed to release AudioRecord", e)
        }

        // Release VAD resources (closes the ONNX runtime session).
        try {
            vad?.close()
        } catch (e: Exception) {
            Log.w(TAG, "Failed to close VadSilero", e)
        }
        vad = null

        val pcmData = pcmBuffer.toShortArray()
        pcmBuffer.clear()

        Log.d(TAG, "Recording stopped (${pcmData.size} samples captured)")

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
            Log.w(TAG, "releaseImmediately: failed to stop AudioRecord", e)
        }
        try {
            audioRecord?.release()
        } catch (e: Exception) {
            Log.w(TAG, "releaseImmediately: failed to release AudioRecord", e)
        }
        audioRecord = null
        try {
            vad?.close()
        } catch (e: Exception) {
            Log.w(TAG, "releaseImmediately: failed to close VadSilero", e)
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

    /**
     * Converts a raw RMS value (0..32768) into a noise-gated, amplified, smoothed
     * amplitude in [0, 1] suitable for waveform display.
     *
     * - Values below NOISE_FLOOR_NORMALIZED are silenced (report 0).
     * - Values above the floor are remapped to [0, 1] and amplified so that
     *   normal speech peaks are clearly visible.
     * - A 3-sample rolling average removes frame-to-frame jitter.
     */
    private fun smoothedAmplitude(rawRms: Float): Float {
        val normalized = (rawRms / 32768f).coerceIn(0f, 1f)

        // Noise floor: anything below this is treated as silence.
        val noiseFloor = 0.04f

        val gated = if (normalized < noiseFloor) {
            0f
        } else {
            // Remap [noiseFloor..1] -> [0..1], then amplify to make speech peaks pop.
            val remapped = (normalized - noiseFloor) / (1f - noiseFloor)
            (remapped * 2.5f).coerceIn(0f, 1f)
        }

        // Rolling average over the last 3 samples.
        amplitudeHistory[amplitudeHistoryIndex % amplitudeHistory.size] = gated
        amplitudeHistoryIndex++
        return amplitudeHistory.average().toFloat()
    }
}
