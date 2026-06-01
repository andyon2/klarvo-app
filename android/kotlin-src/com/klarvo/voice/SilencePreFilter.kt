package com.klarvo.voice

import kotlin.math.sqrt
import java.nio.ByteBuffer
import java.nio.ByteOrder

/**
 * Pre-STT silence and duration filter — Android port of pipeline.rs::silence_skip.
 *
 * Discards mini-taps (duration < MIN_RECORDING_MS) and silent recordings
 * (RMS < SILENCE_THRESHOLD) before the Groq STT API call, preventing phantom
 * transcriptions and unnecessary API credit burn (DIV-02, robustness-audit-2026-05-30.md §3).
 *
 * Mirrors Rust:
 * - `silence_skip` logic (pipeline.rs:471-486): TooShort check before RMS check
 * - `compute_wav_rms` (pipeline.rs:413-438): 16-bit PCM, normalized to [0, 1]
 * - Defaults: min_recording_ms = 500, silence_threshold = 0.005 (config/mod.rs:201-212)
 *
 * Boundary parity with Rust:
 * - Exactly 500 ms → Pass (`< MIN_RECORDING_MS`, not `<=`)
 * - Exactly 0.005 RMS → Pass (`< SILENCE_THRESHOLD`, not `<=`)
 * - RMS == null (malformed WAV) → skip silent check, proceed to Pass
 */
object SilencePreFilter {

    const val MIN_RECORDING_MS = 500L
    const val SILENCE_THRESHOLD = 0.005f

    sealed class FilterResult {
        object Pass : FilterResult()
        data class TooShort(val durationMs: Long) : FilterResult()
        data class Silent(val rms: Float) : FilterResult()
    }

    /**
     * Parses the WAV data chunk size and sample rate from the standard 44-byte
     * PCM WAV header produced by KlarvoApi.encodeWav().
     * Returns 0L if the header is malformed or too short.
     */
    fun computeDurationMs(wavBytes: ByteArray): Long {
        if (wavBytes.size < 44) return 0L
        return try {
            val buf = ByteBuffer.wrap(wavBytes).order(ByteOrder.LITTLE_ENDIAN)
            val sampleRate = buf.getInt(24).toLong()   // bytes 24-27
            val dataSize   = buf.getInt(40).toLong()   // bytes 40-43 (data chunk size in bytes)
            if (sampleRate <= 0) return 0L
            // mono 16-bit: 2 bytes per sample
            (dataSize / 2L * 1000L) / sampleRate
        } catch (e: Exception) {
            0L
        }
    }

    /**
     * Computes RMS of the PCM samples in a WAV, normalized to [0, 1].
     * Returns null if the WAV is malformed; 0.0f if the data chunk is empty.
     * Mirrors Rust pipeline.rs::compute_wav_rms.
     */
    fun computeWavRms(wavBytes: ByteArray): Float? {
        if (wavBytes.size < 44) return null
        return try {
            val buf = ByteBuffer.wrap(wavBytes).order(ByteOrder.LITTLE_ENDIAN)
            val dataSize = buf.getInt(40)  // bytes in data chunk
            if (dataSize <= 0) return 0.0f
            val sampleCount = dataSize / 2  // 16-bit mono: 2 bytes per sample
            // Patch 3: guard degenerate sampleCount == 0 (e.g. dataSize == 1, odd byte).
            // Returns 0.0f for parity with Rust compute_wav_rms returning Some(0.0) for
            // empty samples, instead of producing sqrt(0.0 / 0) = NaN.
            if (sampleCount == 0) return 0.0f
            var sumSq = 0.0
            val dataOffset = 44
            // Patch 2: track the count of samples actually summed (loop may break early
            // on a truncated buffer where header dataSize > bytes present).
            var samplesRead = 0
            for (i in 0 until sampleCount) {
                val pos = dataOffset + i * 2
                if (pos + 1 >= wavBytes.size) break
                val sample = buf.getShort(pos).toFloat() / 32768f
                sumSq += sample * sample
                samplesRead++
            }
            // Divide by samplesRead (not header-claimed sampleCount) to avoid
            // understating RMS on truncated buffers.
            if (samplesRead == 0) return 0.0f
            sqrt(sumSq / samplesRead).toFloat()
        } catch (e: Exception) {
            null
        }
    }

    /**
     * Runs the pre-STT filter against [wavBytes].
     *
     * Order matches Rust silence_skip: TooShort is checked before RMS to
     * avoid unnecessary computation on mini-taps.
     */
    fun check(wavBytes: ByteArray): FilterResult {
        val durationMs = computeDurationMs(wavBytes)
        if (durationMs < MIN_RECORDING_MS) {
            return FilterResult.TooShort(durationMs)
        }
        val rms = computeWavRms(wavBytes)
        if (rms != null && rms < SILENCE_THRESHOLD) {
            return FilterResult.Silent(rms)
        }
        return FilterResult.Pass
    }
}
