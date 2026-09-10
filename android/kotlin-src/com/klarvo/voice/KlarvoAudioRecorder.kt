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
import kotlin.math.ceil
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
 * The RMS energy gate is kept, and a frame counts as speech only if it clears the gate AND
 * the VAD agrees ([vadGateDecision] returns `energyAboveGate && vadSpeech`). Note the gate is
 * NOT a CPU short-circuit: the VAD model is called for every frame regardless of the gate,
 * and deliberately so -- Silero is stateful, and skipping frames would change its sliding-window
 * trajectory (see the [VadGateResult] KDoc).
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
     * RMS energy gate threshold (normalized 0..1). Frames whose filtered RMS is below this
     * cannot be marked as speech, whatever the VAD says (the VAD is still called for every
     * frame -- see the class KDoc). Defaults to 0.005, which matches the Rust
     * default_silence_threshold() in src-tauri/src/config/mod.rs:209.
     *
     * Set from config.json "advanced.silenceThreshold" by KlarvoOverlayService so
     * the user's desktop slider setting is honored on Android (AC1/AC2/AC3, Story 9-11).
     * The previous hard-coded 0.02 was 4× the desktop default, causing ~10% of quiet
     * utterances to be missed on a typical device microphone.
     */
    private var energyGateThreshold: Float = DEFAULT_ENERGY_GATE_THRESHOLD,
    /**
     * Seconds of continuous silence required to trigger the repeatable [onPreviewPause]
     * edge (Story 11-2, AC-2/Task 2.3a). Independent of [silenceSecs] -- HOLD/TOGGLE have no
     * mode-level silence window of their own (that constructor param carries the per-gesture
     * tap/long-press value, only ever *consumed* by AUTOSTOP/AUTO), so the preview signal needs
     * its own frame counter driven by the ported `previewPauseSilenceSecs` Settings slider,
     * exactly mirroring desktop where `preview_pause_silence_secs` is a distinct config field.
     * Defaults to 2.0s (matches the Rust/desktop default).
     */
    private var previewPauseSilenceSecs: Float = 2.0f
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
         * @param normalizedRms  Per-frame RMS normalized to [0, 1] (highpass-filtered signal,
         *                       divided by [VAD_RMS_NORMALIZATION_DIVISOR] = 32767, AC1/AC3/AC4).
         * @param threshold      Energy gate threshold; caller supplies the configured value.
         */
        fun isEnergyAboveGate(normalizedRms: Float, threshold: Float): Boolean =
            normalizedRms >= threshold

        /**
         * Pure RMS computation over an ALREADY-NORMALIZED (see [VAD_RMS_NORMALIZATION_DIVISOR]),
         * highpass-filtered frame (H1/M3/M4, AC1/AC3/AC4, Story 7-2). Mirrors Rust's `rms()`
         * (`src-tauri/src/vad/mod.rs:397-403`) -- same Double-accumulator-then-narrow-to-Float
         * structure as the private (display-path, raw-signal) [calculateRms], applied instead to
         * the filtered VAD-gate signal so the energy gate compares against the same signal Rust
         * does. Extracted to the companion object so a JVM test can drive it directly (AC1's test
         * note: prove the filter is actually in the RMS path, not just present in the file).
         */
        fun calculateRmsFloat(buffer: FloatArray, length: Int): Float {
            if (length == 0) return 0f
            var sum = 0.0
            for (i in 0 until length) {
                sum += buffer[i].toDouble() * buffer[i].toDouble()
            }
            return sqrt(sum / length).toFloat()
        }

        /**
         * Pure delta-slice function (Story 11-2, AC-1/Task 1.2/1.3). Given the FULL sample
         * buffer captured so far and the marker (sample-count offset of the last flush),
         * returns the new samples since [marker] -- empty if none.
         *
         * Extracted so a JVM test can verify disjoint + union == full buffer without an
         * Android [Context] (mirrors desktop's `delta_snapshot_wav`/`spec_delta_snapshot_disjoint_union`).
         */
        fun sliceSince(samples: ShortArray, marker: Int): ShortArray {
            val from = marker.coerceIn(0, samples.size)
            return samples.copyOfRange(from, samples.size)
        }

        /**
         * Pure seconds→frames conversion (Story 11-2, Task 2.3a), extracted so a JVM test can
         * verify a larger silence-seconds value yields a larger frame threshold WITHOUT an
         * Android Context. Used for both [requiredSilentFrames] (one-shot AUTOSTOP/AUTO path)
         * and [previewRequiredSilentFrames] (repeatable preview-pause edge) -- same formula,
         * different independent inputs, so the preview slider is never inert (subject to the
         * [MIN_SILENT_FRAMES] floor below, which collapses very small values together by design,
         * AC2/H17).
         *
         * L1/H17 (AC6/AC2, Story 7-2): frame count uses the EXACT 31.25 fps with [ceil]
         * (matching Rust's rounding direction, `src-tauri/src/vad/mod.rs:239-242`) -- the old
         * truncated integer 31 under-counted frames, requiring fewer silence frames than Rust
         * for the same configured duration. A [MIN_SILENT_FRAMES] floor (AC2/H17) is then
         * applied uniformly, matching Rust's `hangover_ms.max(200)` applied to BOTH the
         * autostop/auto config (`audio/mod.rs:1072-1076`) and the preview-flush config
         * (`audio/mod.rs:1180-1184`). DECIDED (GATE 1, Andi, 2026-09-09): apply the floor
         * uniformly through this shared function -- preview-pause timing is floored too,
         * exactly as Rust does; no separate autostop-only helper.
         */
        fun framesForSeconds(secs: Float): Int =
            ceil(secs * VAD_FRAMES_PER_SECOND).toInt().coerceAtLeast(MIN_SILENT_FRAMES)

        // Silero VAD requires exactly 512 samples per frame at 16 kHz (~32 ms/frame).
        private const val VAD_FRAME_SIZE = 512

        // Onset hysteresis: require this many consecutive VAD-speech frames before
        // we consider speech to have started. Prevents single-frame false positives.
        // 3 frames * 32 ms/frame = ~96 ms onset latency.
        private const val VAD_ONSET_FRAMES = 3

        // Frame duration in ms: 512 samples / 16000 Hz * 1000 = 32.0 exactly (matches Rust's
        // `frame_ms`, vad/mod.rs:239-241). VAD_FRAMES_PER_SECOND is the exact 31.25 fps this
        // implies -- previously hardcoded as the truncated integer 31 (L1/AC6).
        private const val VAD_FRAME_MS: Float = VAD_FRAME_SIZE * 1000f / SAMPLE_RATE
        private const val VAD_FRAMES_PER_SECOND: Float = 1000f / VAD_FRAME_MS

        // H17 (AC2): Rust applies a uniform 200ms hangover floor (`hangover_ms.max(200)`),
        // converted to a frame count via ceil(200 / frame_ms) = ceil(6.25) = 7 frames --
        // regardless of how small the user's configured silence window is. Android's old
        // `coerceAtLeast(1)` only floored at ~32ms (a single frame).
        private const val HANGOVER_FLOOR_MS: Float = 200f
        private val MIN_SILENT_FRAMES: Int = ceil(HANGOVER_FLOOR_MS / VAD_FRAME_MS).toInt()

        // Highpass cutoff for the VAD-gate pre-filter (M3, AC3). Matches Rust's
        // `VadConfig::highpass_cutoff_hz` default (`src-tauri/src/vad/mod.rs:88`).
        // Internal (not private) so a JVM test can pin the -3dB corner against the real
        // production value instead of a test-local duplicate (Story 7-2 review finding).
        @androidx.annotation.VisibleForTesting
        internal const val HIGHPASS_CUTOFF_HZ = 85f

        /**
         * Normalization divisor for VAD-gate RMS/highpass-filter input (M4, AC4, Story 7-2).
         * Matches Rust's `i16::MAX` exactly (`src-tauri/src/audio/mod.rs:766`) -- NOT the old
         * 32768f (2^15) previously used at the energy-gate call site. A ~0.003% scale
         * correction, smaller than the M3 filtering gap (AC3) but needed for exact parity.
         * Display-path normalization ([smoothedAmplitude]) is untouched and keeps /32768f --
         * this constant is VAD-gate-only. Internal (not private) so a JVM test can assert
         * against the real production constant instead of a test-local copy (Story 7-2 review
         * finding).
         */
        @androidx.annotation.VisibleForTesting
        internal const val VAD_RMS_NORMALIZATION_DIVISOR = 32767f

        /**
         * H1/M3/M4 (AC1/AC3/AC4): normalizes [length] samples of [frame] to [-1,1] (dividing by
         * [VAD_RMS_NORMALIZATION_DIVISOR], matching Rust's i16::MAX) THEN highpass-filters each
         * sample through [filter] -- mirrors Rust's normalize-before-filter order
         * (`audio/mod.rs:766` normalizes by `i16::MAX` before `SileroVad::feed`, which filters
         * internally). Writes into [out] and returns it (the caller passes a reused scratch
         * buffer in production to avoid a per-frame allocation on the audio thread; tests may
         * pass a fresh array).
         *
         * Extracted to the companion object so a JVM test can drive the EXACT chain
         * [processVadFrame] feeds into both [calculateRmsFloat] (AC1's energy gate) and the
         * Silero call wrapped by [vadGateDecision] (AC3) -- this is the seam Story 7-2's round-1
         * review found missing: without it, reverting the filter or the divisor left every test
         * green because tests re-implemented this sequence instead of calling it.
         */
        @androidx.annotation.VisibleForTesting
        internal fun vadGateFilteredFrame(
            frame: ShortArray,
            length: Int,
            filter: HighpassFilter,
            out: FloatArray
        ): FloatArray {
            for (i in 0 until length) {
                val normalized = frame[i] / VAD_RMS_NORMALIZATION_DIVISOR
                out[i] = filter.process(normalized)
            }
            return out
        }

        /** Result of [vadGateDecision]: the normalized (filtered) RMS, the raw Silero verdict,
         * and the combined energy-gate-AND-Silero decision [processVadFrame]'s state machine
         * consumes. Three fields, not just the combined boolean, so [processVadFrame]'s
         * diagnostics (which log the raw Silero rate separately from the gated rate) don't need a
         * second, un-seamed `isSpeech` call -- Silero's internal sliding-window state would be
         * corrupted by calling `isSpeech` twice per frame. */
        data class VadGateResult(val normalizedRms: Float, val vadSpeech: Boolean, val isSpeechFrame: Boolean)

        /**
         * The FULL VAD-gate decision for one frame (H1/AC1, M3/AC3): filters+normalizes [frame]
         * via [vadGateFilteredFrame], computes the energy gate via [calculateRmsFloat] +
         * [isEnergyAboveGate], and calls [isSpeech] -- the caller's Silero wrapper -- with the
         * SAME filtered frame the energy gate used. [isSpeech] is a function parameter (not a
         * direct `VadSilero` dependency) so a JVM test can drive this exact production sequence
         * with a fake verdict function and assert it received the FILTERED frame, without needing
         * a real Silero/ONNX instance (round-2 review finding: round 1's seam covered only the
         * filter+RMS half, leaving `vad?.isSpeech(filteredFrame)` vs. `isSpeech(frame)` -- and
         * dropping this seam call entirely -- undetected by any test).
         */
        @androidx.annotation.VisibleForTesting
        internal fun vadGateDecision(
            frame: ShortArray,
            length: Int,
            filter: HighpassFilter,
            out: FloatArray,
            threshold: Float,
            isSpeech: (FloatArray) -> Boolean
        ): VadGateResult {
            val filteredFrame = vadGateFilteredFrame(frame, length, filter, out)
            val normalizedRms = calculateRmsFloat(filteredFrame, length).coerceIn(0f, 1f)
            val energyAboveGate = isEnergyAboveGate(normalizedRms, threshold)
            val vadSpeech = isSpeech(filteredFrame)
            return VadGateResult(normalizedRms, vadSpeech, energyAboveGate && vadSpeech)
        }
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
        get() = framesForSeconds(silenceSecs)

    /**
     * Story 11-2 (AC-2/Task 2.3a): independent frame threshold for the repeatable
     * [onPreviewPause] edge, derived from [previewPauseSilenceSecs] -- NOT from [silenceSecs]/
     * [requiredSilentFrames], which govern the (possibly unset) one-shot [onSilenceDetected] path.
     */
    private val previewRequiredSilentFrames: Int
        get() = framesForSeconds(previewPauseSilenceSecs)

    /**
     * Optional callback fired once when sustained silence is detected after speech.
     * Set by KlarvoOverlayService for AUTOSTOP / AUTO modes.
     * Fires on the recording thread -- caller must post to main thread.
     */
    var onSilenceDetected: (() -> Unit)? = null

    /**
     * Optional REPEATABLE callback (Story 11-2, AC-2) fired on every silence-onset edge for
     * the whole recording -- unlike [onSilenceDetected], it is not gated by [silenceCallbackFired]
     * and does not stop VAD feeding. Set by KlarvoOverlayService for HOLD/TOGGLE when live preview
     * is enabled. Fires on the recording thread -- caller must post to main thread.
     */
    var onPreviewPause: (() -> Unit)? = null

    private var audioRecord: AudioRecord? = null
    private val pcmBuffer = ArrayList<Short>()

    /**
     * Sample-count offset into [pcmBuffer] of the last [deltaSnapshotWav] flush (Story 11-2,
     * mirrors desktop's `delta_marker: Mutex<usize>`). Reset to 0 on [start].
     */
    private var deltaMarker = 0
    private var recordingThread: Thread? = null
    private var isCapturing = false

    // Silero VAD instance -- initialized in start(), released in stop()/releaseImmediately().
    // Previously there was no VAD object; silence was detected purely by RMS comparison.
    private var vad: VadSilero? = null

    // M3 (AC3): 85 Hz highpass filter run on every VAD-gate frame before RMS (AC1) and Silero
    // (vad.isSpeech). Has memory (delay line) -- persists across frames for the life of a
    // recording session and is reset only in [start], mirroring Rust's `SileroVad`-owned filter
    // instance lifecycle (`vad/mod.rs:292-296`). The display-path RMS ([calculateRms], fed by
    // the raw unfiltered signal) is intentionally NOT routed through this filter.
    private val highpassFilter = HighpassFilter(HIGHPASS_CUTOFF_HZ, SAMPLE_RATE.toFloat())

    // Reused scratch buffer for the normalized+filtered VAD-gate frame ([vadGateFilteredFrame]
    // output), always exactly VAD_FRAME_SIZE. Avoids allocating a new 2 KB FloatArray on every
    // ~32 ms frame (~31x/s, ~64 KB/s of garbage) on the audio recording thread.
    private val filteredFrameScratch = FloatArray(VAD_FRAME_SIZE)

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

    // Story 11-2: independent hangover counter + one-shot-per-silence-period guard for the
    // repeatable onPreviewPause edge. Reset to 0/false whenever speech resumes, so it can fire
    // again on the NEXT pause (unlike silenceCallbackFired, which never resets).
    private var previewSilentFrames = 0
    private var previewFiredThisSilence = false

    // --- Auto-mode silence-fire diagnostics (Story 9-7 follow-up, 2026-06-16) ---
    // Purely observational: aggregate per ~1s window so one device repro reveals why
    // onSilenceDetected never fires (VAD sees no speech? requiredSilentFrames too big?
    // silentFrames keeps resetting?). No behavior change — logging only.
    private var dbgWindowFrames = 0
    private var dbgSpeechFrames = 0
    private var dbgRmsMax = 0f
    private var dbgVadTrue = 0

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
        synchronized(pcmBuffer) { pcmBuffer.clear() }
        isCapturing = true

        recorder.startRecording()

        // Reset VAD silence detection state.
        highpassFilter.reset()
        vadRingPos = 0
        silentFrames = 0
        silenceCallbackFired = false
        onsetFrames = 0
        speechDetected = false
        previewSilentFrames = 0
        previewFiredThisSilence = false
        deltaMarker = 0
        dbgWindowFrames = 0
        dbgSpeechFrames = 0
        dbgRmsMax = 0f
        dbgVadTrue = 0
        KlarvoLogger.d(TAG, "VAD config: silenceSecs=$silenceSecs requiredSilentFrames=$requiredSilentFrames " +
            "energyGate=$energyGateThreshold onSilenceDetected=${onSilenceDetected != null}")

        recordingThread = Thread {
            // Read in 1024-short chunks (~64 ms at 16 kHz) so onAmplitude fires ~4× more often
            // than the old bufferSize/2 (4096 shorts = 256 ms). AudioRecord bufferSize (8192)
            // is unchanged — only the per-iteration read slice shrinks.
            val buf = ShortArray(1024)
            while (isCapturing) {
                val read = recorder.read(buf, 0, buf.size)
                if (read > 0) {
                    // Accumulate PCM for WAV output. Synchronized: deltaSnapshotWav() (Story 11-2)
                    // reads/copies pcmBuffer from the main thread while this thread keeps appending.
                    synchronized(pcmBuffer) {
                        for (i in 0 until read) {
                            pcmBuffer.add(buf[i])
                        }
                    }

                    // Compute RMS for waveform visualization (unchanged from before).
                    val rms = calculateRms(buf, read)
                    val smoothedAmp = smoothedAmplitude(rms)
                    onAmplitude(smoothedAmp)

                    // Feed samples into the VAD ring buffer in 512-sample frames.
                    // Story 11-2 (AC-2a): widened so HOLD/TOGGLE also feed the VAD when the
                    // repeatable preview-flush callback is installed -- previously this gate was
                    // closed for HOLD/TOGGLE (onSilenceDetected is null there), so no pause edges
                    // were ever produced and the preview flush could never fire.
                    if ((onSilenceDetected != null && !silenceCallbackFired) || onPreviewPause != null) {
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
     *     - Energy gate below energyGateThreshold → onsetFrames = 0 (no speech)
     *     - VAD returns true                      → onsetFrames++
     *     - onsetFrames >= VAD_ONSET_FRAMES       → speechDetected = true, silentFrames = 0
     *
     *   AFTER SPEECH CONFIRMED (speechDetected == true):
     *     - Energy gate below threshold OR VAD false → silentFrames++
     *     - VAD true                                 → silentFrames = 0 (hangover reset)
     *     - silentFrames >= requiredSilentFrames     → fire onSilenceDetected
     *
     * Previously: a single RMS threshold determined speech vs. silence for entire
     * AudioRecord chunks (~8192/2 = 4096 samples), on the raw unfiltered signal.
     * Now: VAD model runs on 512-sample frames (normalized + highpass-filtered, AC1/AC3/AC4)
     * with onset and hangover hysteresis.
     */
    private fun processVadFrame(frame: ShortArray) {
        // Story 11-2 (AC-2): the old top-of-function `if (silenceCallbackFired) return` guard
        // is REMOVED here -- it used to block ALL further processing (including the new
        // repeatable preview edge) forever after the one-shot callback fired once. The one-shot
        // guard is now scoped locally to the onSilenceDetected branch below, where it belongs.

        // H1/M3/M4 (AC1/AC3/AC4): the shared companion seam (vadGateDecision) normalizes,
        // highpass-filters, computes the energy gate AND calls Silero on the SAME filtered
        // signal Rust compares against -- the display-path RMS ([calculateRms], fed by [buf]
        // above) stays on the raw, unfiltered signal (AC3 decision). Round-2 review finding:
        // the seam now covers the whole gate decision, not just the filter+RMS half, so a JVM
        // test can catch a reverted `isSpeech(filteredFrame)` -> `isSpeech(frame)` regression.
        val gateResult = vadGateDecision(
            frame,
            frame.size,
            highpassFilter,
            filteredFrameScratch,
            energyGateThreshold
        ) { filtered -> vad?.isSpeech(filtered) == true }

        val normalizedRms = gateResult.normalizedRms
        val vadSpeech = gateResult.vadSpeech
        val isSpeechFrame = gateResult.isSpeechFrame

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
                    previewSilentFrames = 0
                    previewFiredThisSilence = false
                    KlarvoLogger.d(TAG,"VAD: speech onset confirmed (onsetFrames=$onsetFrames)")
                }
            } else {
                // Any non-speech frame resets the onset counter.
                onsetFrames = 0
            }
            return
        }

        // Hangover phase: count silence frames after speech was detected.
        if (isSpeechFrame) {
            silentFrames = 0
            previewSilentFrames = 0
            previewFiredThisSilence = false
            return
        }

        // Story 11-2 (AC-2): the one-shot AUTOSTOP/AUTO path is scoped to `onSilenceDetected != null`
        // and its own `silenceCallbackFired` guard -- exactly as before, byte-identical semantics,
        // just no longer gating the repeatable preview edge below.
        if (onSilenceDetected != null && !silenceCallbackFired) {
            silentFrames++
            if (silentFrames >= requiredSilentFrames) {
                silenceCallbackFired = true
                KlarvoLogger.d(TAG,"VAD: silence detected after speech ($silentFrames frames >= $requiredSilentFrames required)")
                // Story 11-1 (spike): mark the exact pause-signal instant here (this IS
                // onSilenceDetected firing). Log-only -- the actual pause-to-text delta is
                // computed and logged in KlarvoOverlayService once the transcript returns
                // (tag "[benchmark-11-1]"); this line lets the raw fire instant be cross-
                // checked independently if the two ever need pairing on-device.
                KlarvoLogger.d(TAG, "[benchmark-11-1] onSilenceDetected fired at ${System.currentTimeMillis()}")
                onSilenceDetected?.invoke()
            }
        }

        // Story 11-2 (AC-2b/Task 2.3a): repeatable preview-pause edge -- NOT gated by
        // silenceCallbackFired, uses its own independent frame threshold
        // (previewRequiredSilentFrames, derived from previewPauseSilenceSecs), and re-arms
        // (previewFiredThisSilence = false) whenever speech resumes above, so it fires once
        // per pause for the whole recording, not just once per session.
        if (onPreviewPause != null) {
            previewSilentFrames++
            if (previewSilentFrames >= previewRequiredSilentFrames && !previewFiredThisSilence) {
                previewFiredThisSilence = true
                KlarvoLogger.d(TAG, "VAD: preview-pause edge fired ($previewSilentFrames frames >= $previewRequiredSilentFrames required)")
                onPreviewPause?.invoke()
            }
        }
    }

    /**
     * Story 11-2 (AC-1/Task 1.2): returns the audio captured **since the last flush** (not the
     * whole buffer so far), encoded as a WAV byte array, and advances the delta marker.
     * Returns `null` if no new samples have accumulated since the marker (e.g. flush fired but
     * the ring-buffer boundary landed exactly at a prior flush -- practically rare).
     *
     * Safe to call from any thread while recording is in progress (synchronizes on [pcmBuffer],
     * which the recording thread is concurrently appending to).
     */
    fun deltaSnapshotWav(): ByteArray? {
        val slice = synchronized(pcmBuffer) {
            val snapshot = pcmBuffer.toShortArray()
            val s = sliceSince(snapshot, deltaMarker)
            deltaMarker = snapshot.size
            s
        }
        if (slice.isEmpty()) return null
        return encodeWav(slice, SAMPLE_RATE)
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

        val pcmData = synchronized(pcmBuffer) { pcmBuffer.toShortArray() }
        synchronized(pcmBuffer) { pcmBuffer.clear() }

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
        synchronized(pcmBuffer) { pcmBuffer.clear() }
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
     * Converts a raw RMS value (0..32768) into a noise-gated, amplified amplitude
     * in [0, 1] suitable for waveform display.
     *
     * Recalibrated 2026-06-21 (Story 9-12 GATE-4 re-run):
     * - Old noise floor 0.04 (= raw RMS ≈1311) gated normal phone speech to ≈0.
     * - New noise floor 0.012 (= raw RMS ≈393) gates ambient hum while passing speech.
     * - Speech band [0.012..0.15] remapped and × 4.0 to fill [0..1] visibly;
     *   clamped at 1 — louder speech saturates cleanly.
     * - No rolling average: [gated] is returned directly (the per-chunk amplitude is
     *   returned as-is; temporal/visual smoothing is provided SPATIALLY by the scrolling
     *   20-deep waveLevels history in FloatingBubbleView — desktop parity). Removing the
     *   3-sample average eliminates the ~200-300 ms onset/offset lag confirmed by real-
     *   device KLARVO_AMP_DIAG data (silent rawRMS=64 still showed smoothedAmp=0.304
     *   carryover; speech rawRMS=639 showed only smoothedAmp=0.073 onset lag).
     *   Note: the method name smoothedAmplitude is now a slight misnomer — it returns the
     *   gated per-chunk amplitude, not a temporally smoothed value.
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

        return gated
    }
}
