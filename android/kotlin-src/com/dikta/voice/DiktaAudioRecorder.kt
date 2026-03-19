package com.dikta.voice

import android.media.AudioFormat
import android.media.AudioRecord
import android.media.MediaRecorder
import android.util.Log
import kotlin.math.sqrt

/**
 * Manages audio capture from the microphone.
 *
 * Usage:
 *   val recorder = DiktaAudioRecorder { amplitude -> updateWaveform(amplitude) }
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
 */
class DiktaAudioRecorder(
    private val onAmplitude: (Float) -> Unit,
    /**
     * Seconds of continuous silence required to trigger [onSilenceDetected].
     * Defaults to 2.0s. Roughly 15 audio chunks per second at 16 kHz.
     */
    private val silenceSecs: Float = 2.0f
) {

    companion object {
        private const val TAG = "DiktaAudioRecorder"

        private const val SAMPLE_RATE = 16000
        private const val CHANNEL_CONFIG = AudioFormat.CHANNEL_IN_MONO
        private const val AUDIO_FORMAT = AudioFormat.ENCODING_PCM_16BIT

        // Silence detection: amplitude below this for requiredSilentChunks triggers callback.
        private const val SILENCE_THRESHOLD = 0.03f
        // Approximate audio chunks per second at 16 kHz with bufferSize ~8192 shorts.
        private const val CHUNKS_PER_SECOND = 15
    }

    /** Computed required-silent-chunks from the silenceSecs constructor parameter. */
    private val requiredSilentChunks: Int
        get() = (silenceSecs * CHUNKS_PER_SECOND).toInt().coerceAtLeast(1)

    /**
     * Optional callback fired once when sustained silence is detected.
     * Set by DiktaOverlayService for AUTOSTOP / AUTO modes.
     * Fires on the recording thread -- caller must post to main thread.
     */
    var onSilenceDetected: (() -> Unit)? = null

    private var audioRecord: AudioRecord? = null
    private val pcmBuffer = ArrayList<Short>()
    private var recordingThread: Thread? = null
    private var isCapturing = false

    // Silence detection state
    private var silentChunks = 0
    private var silenceCallbackFired = false
    // Require at least 1s of audio before silence detection activates (avoids instant trigger)
    private var totalChunks = 0
    private val minChunksBeforeSilence = 15  // ~1s
    // True once speech has been detected -- silence before first speech is ignored.
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

        audioRecord = recorder
        pcmBuffer.clear()
        isCapturing = true

        recorder.startRecording()
        // Reset smoothing state for the new recording session.
        amplitudeHistory.fill(0f)
        amplitudeHistoryIndex = 0
        // Reset silence detection state.
        silentChunks = 0
        silenceCallbackFired = false
        totalChunks = 0
        speechDetected = false

        recordingThread = Thread {
            val buf = ShortArray(bufferSize / 2)
            while (isCapturing) {
                val read = recorder.read(buf, 0, buf.size)
                if (read > 0) {
                    for (i in 0 until read) {
                        pcmBuffer.add(buf[i])
                    }
                    val rms = calculateRms(buf, read)
                    val smoothedAmp = smoothedAmplitude(rms)
                    onAmplitude(smoothedAmp)

                    // Silence detection
                    totalChunks++
                    if (onSilenceDetected != null && !silenceCallbackFired) {
                        if (smoothedAmp >= SILENCE_THRESHOLD) {
                            // Speech detected: mark it, reset consecutive-silence counter.
                            speechDetected = true
                            silentChunks = 0
                        } else if (speechDetected) {
                            // Silence after speech has been detected: count it.
                            silentChunks++
                            if (silentChunks >= requiredSilentChunks && totalChunks >= minChunksBeforeSilence) {
                                silenceCallbackFired = true
                                Log.d(TAG, "Silence detected after speech ($silentChunks chunks, totalChunks=$totalChunks)")
                                onSilenceDetected?.invoke()
                            }
                        }
                        // else: silence before any speech -- ignore pre-speech ambient noise
                    }
                }
            }
        }.also { it.start() }

        Log.d(TAG, "Recording started (bufferSize=$bufferSize, sampleRate=$SAMPLE_RATE)")
    }

    /**
     * Stops capturing, releases [AudioRecord], and returns the recorded audio
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
