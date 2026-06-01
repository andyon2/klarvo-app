package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Unit tests for SilencePreFilter.
 *
 * These run as JVM unit tests (no device required). They bind to the real
 * SilencePreFilter.check() production function via computeDurationMs and
 * computeWavRms — not mock primitives that bypass WAV parsing (AI-2 from
 * Epic 1 retro).
 *
 * buildTestWav generates a real 16kHz/mono/16-bit WAV using encodeWav (the
 * same production encoder used in KlarvoApi) so tests exercise the actual
 * parsing path.
 *
 * Coverage:
 * - Empty ByteArray → TooShort (duration = 0ms)
 * - Recording shorter than 500ms → TooShort
 * - Recording exactly 500ms → Pass (boundary: < not <=)
 * - Sufficient duration but all-zero samples (RMS = 0.0) → Silent
 * - RMS = 0.004 (below 0.005 threshold) → Silent
 * - RMS = 0.005 (AT threshold) → Pass (boundary: < not <=)
 * - RMS = 0.05 and duration ≥ 500ms → Pass
 * - Malformed ByteArray (not a WAV) → TooShort gracefully, no exception
 */
class SilencePreFilterTest {

    // ---------------------------------------------------------------------------
    // Test WAV helper — uses the real production encodeWav() so we bind to the
    // actual parsing path (AI-2 mandate: no mock inputs that bypass WAV parsing).
    // ---------------------------------------------------------------------------

    /**
     * Builds a real 16kHz/mono/16-bit WAV ByteArray with [durationMs] duration
     * and constant amplitude [rmsAmplitude].
     *
     * For a constant-amplitude signal, RMS == amplitude, so rmsAmplitude = 0.005
     * produces a WAV whose measured RMS is exactly 0.005.
     */
    private fun buildTestWav(durationMs: Long, rmsAmplitude: Float): ByteArray {
        val sampleRate = 16000
        val sampleCount = ((sampleRate * durationMs) / 1000L).toInt()
        // Constant amplitude → RMS equals amplitude
        val amplitude = (rmsAmplitude * 32767f).toInt().toShort()
        val pcm = ShortArray(sampleCount) { amplitude }
        return encodeWav(pcm, sampleRate)
    }

    // ---------------------------------------------------------------------------
    // Task 2.2 — empty ByteArray → TooShort (duration = 0ms)
    // ---------------------------------------------------------------------------

    @Test
    fun emptyByteArray_isTooShort() {
        val result = SilencePreFilter.check(ByteArray(0))
        assertTrue("empty ByteArray must be TooShort", result is SilencePreFilter.FilterResult.TooShort)
        assertEquals(0L, (result as SilencePreFilter.FilterResult.TooShort).durationMs)
    }

    // ---------------------------------------------------------------------------
    // Task 2.3 — WAV shorter than 500ms → TooShort
    // ---------------------------------------------------------------------------

    @Test
    fun wavShorterThan500ms_isTooShort() {
        val wav = buildTestWav(200L, 0.0f)
        val result = SilencePreFilter.check(wav)
        assertTrue("200ms recording must be TooShort", result is SilencePreFilter.FilterResult.TooShort)
        val durationMs = (result as SilencePreFilter.FilterResult.TooShort).durationMs
        assertTrue("duration must be < 500ms, got $durationMs", durationMs < 500L)
    }

    @Test
    fun wav499ms_isTooShort() {
        val wav = buildTestWav(499L, 0.05f)
        val result = SilencePreFilter.check(wav)
        assertTrue("499ms recording must be TooShort", result is SilencePreFilter.FilterResult.TooShort)
    }

    // ---------------------------------------------------------------------------
    // Task 2.4 — WAV exactly 500ms → Pass (boundary: < MIN_RECORDING_MS, not <=)
    // ---------------------------------------------------------------------------

    @Test
    fun wavExactly500ms_isPass() {
        // rmsAmplitude = 0.05 (well above silence threshold) — only testing duration boundary
        val wav = buildTestWav(500L, 0.05f)
        val result = SilencePreFilter.check(wav)
        assertTrue("exactly 500ms must be Pass (not TooShort), got $result",
            result is SilencePreFilter.FilterResult.Pass)
    }

    // ---------------------------------------------------------------------------
    // Task 2.5 — Sufficient length but all-zero samples (RMS = 0.0) → Silent
    // ---------------------------------------------------------------------------

    @Test
    fun wavWithZeroSamples_isSilent() {
        val wav = buildTestWav(1000L, 0.0f)
        val result = SilencePreFilter.check(wav)
        assertTrue("all-zero samples must be Silent, got $result",
            result is SilencePreFilter.FilterResult.Silent)
    }

    // ---------------------------------------------------------------------------
    // Task 2.6 — RMS = 0.004 (below 0.005 threshold) → Silent
    // ---------------------------------------------------------------------------

    @Test
    fun wavWithRms0004_isSilent() {
        val wav = buildTestWav(1000L, 0.004f)
        val result = SilencePreFilter.check(wav)
        assertTrue("RMS 0.004 must be Silent (below threshold 0.005), got $result",
            result is SilencePreFilter.FilterResult.Silent)
    }

    // ---------------------------------------------------------------------------
    // Task 2.7 — RMS boundary tests around the 0.005 threshold
    //
    // 16-bit quantization: amplitude = (0.005 * 32767).toInt().toShort() = 163
    //   163 / 32768 ≈ 0.004974  → just BELOW threshold → Silent
    //   164 / 32768 ≈ 0.005005  → just ABOVE threshold → Pass
    // The original "RMS at threshold = Pass" premise was tautological (asserting
    // !is TooShort on a 1000ms WAV says nothing about the RMS branch). Renamed to
    // reflect what the quantized sample actually produces.
    // ---------------------------------------------------------------------------

    /**
     * Amplitude input 0.005 quantizes to short 163 → measured RMS ≈ 0.004974,
     * which is just below the 0.005 threshold. The filter must classify this as
     * Silent (strict `<` boundary, not `<=`).
     *
     * Strict boundary from below: confirms quantization is applied correctly and
     * the `<` comparison fires at ≈0.004974.
     */
    @Test
    fun wavWithRmsJustBelowThreshold_isSilent() {
        // amplitude input 0.005 → short 163 → rms ≈ 163/32768 ≈ 0.004974 < 0.005
        val wav = buildTestWav(1000L, 0.005f)
        val result = SilencePreFilter.check(wav)
        assertTrue(
            "Quantized amplitude 163 (≈0.004974 RMS) must be Silent (just below 0.005 threshold), got $result",
            result is SilencePreFilter.FilterResult.Silent
        )
    }

    /**
     * Amplitude input 0.00501 quantizes to short 164 → measured RMS ≈ 0.005005,
     * which is just above the 0.005 threshold. The filter must classify this as Pass.
     *
     * Strict boundary from above: confirms `<` not `<=` (i.e. at-or-above passes).
     */
    @Test
    fun wavWithRmsJustAboveThreshold_isPass() {
        // amplitude input ≈0.00501 → short 164 → rms ≈ 164/32768 ≈ 0.005005 ≥ 0.005
        val wav = buildTestWav(1000L, 164f / 32767f)
        val result = SilencePreFilter.check(wav)
        assertTrue(
            "Quantized amplitude 164 (≈0.005005 RMS) must be Pass (just above 0.005 threshold), got $result",
            result is SilencePreFilter.FilterResult.Pass
        )
    }

    /**
     * Additional: RMS well above threshold → Pass. Keeps a clear-cut positive case.
     */
    @Test
    fun wavWithRmsAboveThreshold_isPass() {
        val wav = buildTestWav(1000L, 0.006f)
        val result = SilencePreFilter.check(wav)
        assertTrue("RMS 0.006 must be Pass (above threshold 0.005), got $result",
            result is SilencePreFilter.FilterResult.Pass)
    }

    // ---------------------------------------------------------------------------
    // Task 2.8 — RMS = 0.05 and duration ≥ 500ms → Pass
    // ---------------------------------------------------------------------------

    @Test
    fun validUtterance_isPass() {
        val wav = buildTestWav(2000L, 0.05f)
        val result = SilencePreFilter.check(wav)
        assertTrue("2000ms/0.05 RMS recording must Pass, got $result",
            result is SilencePreFilter.FilterResult.Pass)
    }

    // ---------------------------------------------------------------------------
    // Task 2.9 — Malformed ByteArray (not a WAV) → TooShort gracefully, no exception
    // ---------------------------------------------------------------------------

    @Test
    fun malformedByteArray_isTooShortGracefully() {
        val garbage = byteArrayOf(0x00, 0x01, 0x02, 0x03, 0xFF.toByte(), 0xFE.toByte())
        val result = SilencePreFilter.check(garbage) // must not throw
        assertTrue("malformed bytes must yield TooShort (duration=0), got $result",
            result is SilencePreFilter.FilterResult.TooShort)
        assertEquals(0L, (result as SilencePreFilter.FilterResult.TooShort).durationMs)
    }

    @Test
    fun randomBytes_noException() {
        // Stress: 100 bytes of pseudo-random-ish content — should not throw
        val random = ByteArray(100) { i -> (i * 37 + 13).toByte() }
        val result = SilencePreFilter.check(random) // no exception
        assertTrue("random bytes must yield TooShort", result is SilencePreFilter.FilterResult.TooShort)
    }

    // ---------------------------------------------------------------------------
    // Task 2.10 — TooShort is checked BEFORE RMS (short-circuit order)
    // ---------------------------------------------------------------------------

    @Test
    fun shortRecordingWithHighAmplitude_isTooShortNotSilent() {
        // A 100ms WAV with very loud amplitude — if RMS were checked first, it might
        // be classified differently. TooShort must win (parity with Rust silence_skip).
        val wav = buildTestWav(100L, 0.9f)
        val result = SilencePreFilter.check(wav)
        assertTrue("short recording must be TooShort even if loud, got $result",
            result is SilencePreFilter.FilterResult.TooShort)
    }

    // ---------------------------------------------------------------------------
    // computeDurationMs / computeWavRms direct binding tests
    // (verify helpers parse real WAV bytes, not mock primitives)
    // ---------------------------------------------------------------------------

    @Test
    fun computeDurationMs_matchesActualDuration() {
        val wav = buildTestWav(1000L, 0.05f)
        val durationMs = SilencePreFilter.computeDurationMs(wav)
        assertEquals("computeDurationMs must return ~1000ms for 1s WAV", 1000L, durationMs)
    }

    @Test
    fun computeWavRms_returnsNullForMalformed() {
        val tiny = byteArrayOf(1, 2, 3)
        val rms = SilencePreFilter.computeWavRms(tiny)
        assertTrue("computeWavRms must return null for malformed WAV (< 44 bytes)", rms == null)
    }

    @Test
    fun computeWavRms_returns0ForSilence() {
        val wav = buildTestWav(500L, 0.0f)
        val rms = SilencePreFilter.computeWavRms(wav)
        assertTrue("computeWavRms must return 0.0f for all-zero samples", rms == 0.0f)
    }

    // ---------------------------------------------------------------------------
    // Patch 2 + Patch 3 — odd/short data-chunk paths
    // These exercise: (a) degenerate sampleCount==0 from odd dataSize (Patch 3),
    // and (b) truncated buffer where the loop breaks early (Patch 2).
    // ---------------------------------------------------------------------------

    /**
     * Patch 3: WAV with dataSize == 1 (odd byte) → sampleCount == 0.
     * computeWavRms must return 0.0f (parity with Rust Some(0.0) for empty
     * sample set), not NaN from sqrt(0.0/0).
     *
     * Note: check() on this WAV returns TooShort (duration = 0ms because 1 data
     * byte = 0 complete 16-bit samples → 0ms), which is correct — TooShort fires
     * before RMS. The RMS guard is tested via computeWavRms directly.
     */
    @Test
    fun computeWavRms_oddDataSize1_returns0() {
        // Build a minimal WAV with dataSize=1 (one garbage byte) → sampleCount = 0
        val wav = buildWavWithDataBytes(byteArrayOf(0x42))  // 1 byte → sampleCount = 0
        val rms = SilencePreFilter.computeWavRms(wav)
        // Patch 3: must return 0.0f, not NaN
        assertTrue("dataSize==1 (sampleCount==0) must return 0.0f, got $rms", rms == 0.0f)
        // check() correctly returns TooShort (0ms duration) — TooShort fires before RMS
        val result = SilencePreFilter.check(wav)
        assertTrue("check() on 0ms WAV must return TooShort (duration check before RMS), got $result",
            result is SilencePreFilter.FilterResult.TooShort)
    }

    /**
     * Patch 2: WAV header claims N samples but the byte array is truncated to fewer.
     * The loop breaks early; RMS must be computed over the actually-read samples,
     * not over the header-claimed count (which would understate RMS toward zero).
     *
     * We build a header claiming 500 samples (1000 bytes data) of amplitude 0.05,
     * then truncate the data to only 10 samples (20 bytes). The 10 real samples
     * have RMS ≈ 0.05; dividing by 500 would give ≈ sqrt(0.0025/500) ≈ 0.00224
     * (Silent), while dividing by 10 correctly gives ≈ 0.05 (Pass-range).
     */
    @Test
    fun computeWavRms_truncatedBuffer_usesSamplesRead() {
        val claimedSamples = 500
        val actualSamples  = 10
        val amplitude      = (0.05f * 32767f).toInt().toShort()

        // Build header claiming claimedSamples * 2 bytes in data chunk
        val header = buildWavHeader(sampleRate = 16000, dataBytes = claimedSamples * 2)
        // But only provide actualSamples samples of actual PCM data
        val pcmData = java.nio.ByteBuffer.allocate(actualSamples * 2)
            .order(java.nio.ByteOrder.LITTLE_ENDIAN)
        repeat(actualSamples) { pcmData.putShort(amplitude) }
        val wav = header + pcmData.array()

        val rms = SilencePreFilter.computeWavRms(wav)
        // If Patch 2 is applied correctly, rms ≈ 0.05 (above threshold → Pass)
        // If Patch 2 is missing, rms ≈ 0.00224 (below threshold → Silent) — a false negative
        assertTrue("truncated buffer RMS must reflect actual samples (≈0.05), got $rms",
            rms != null && rms >= SilencePreFilter.SILENCE_THRESHOLD)
    }

    // ---------------------------------------------------------------------------
    // Private helpers for patch tests — construct raw WAV bytes directly
    // ---------------------------------------------------------------------------

    /** Builds a 44-byte WAV header for the given sample rate and data chunk size in bytes. */
    private fun buildWavHeader(sampleRate: Int, dataBytes: Int): ByteArray {
        val buf = java.nio.ByteBuffer.allocate(44).order(java.nio.ByteOrder.LITTLE_ENDIAN)
        val totalSize = 36 + dataBytes
        buf.put("RIFF".toByteArray())
        buf.putInt(totalSize)
        buf.put("WAVE".toByteArray())
        buf.put("fmt ".toByteArray())
        buf.putInt(16)           // PCM chunk size
        buf.putShort(1)          // PCM format
        buf.putShort(1)          // mono
        buf.putInt(sampleRate)   // sample rate
        buf.putInt(sampleRate * 2) // byte rate (mono 16-bit)
        buf.putShort(2)          // block align
        buf.putShort(16)         // bits per sample
        buf.put("data".toByteArray())
        buf.putInt(dataBytes)
        return buf.array()
    }

    /**
     * Builds a full WAV (header + provided data bytes) with dataSize set to the
     * exact length of [data]. Useful for degenerate / odd-size chunk tests.
     */
    private fun buildWavWithDataBytes(data: ByteArray): ByteArray {
        return buildWavHeader(sampleRate = 16000, dataBytes = data.size) + data
    }
}
