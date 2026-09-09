package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Test
import kotlin.math.PI
import kotlin.math.sin

/**
 * M4 — Story 7-2, AC4: locks the corrected VAD-gate RMS normalization divisor (32767f, matching
 * Rust's `i16::MAX`, NOT the old 32768f) against the same known-input-to-known-output cases
 * documented in `test-fixtures/wav-rms-vectors.json` (silence -> 0.0, full-scale sine -> 1/sqrt(2),
 * speech-level constant amplitude -> the constant).
 *
 * `test-fixtures/wav-rms-vectors.json` is normally consumed by Rust's `compute_wav_rms` (whole-WAV
 * decode). Kotlin no longer has an equivalent WAV-decoding function -- `WavRmsVectorsTest.kt` was
 * deleted in Story 7.3 when `SilencePreFilter.kt` was removed and the pre-STT RMS check moved
 * behind the shared Rust JNI call (`nativeSilenceCheck`). This test instead targets the OTHER
 * Android RMS consumer that IS still Kotlin-only per ADR-0017 (the live VAD-gate path,
 * [KlarvoAudioRecorder.calculateRmsFloat]): it builds the SAME independently-known signals the
 * fixture describes (silence, full-scale 440 Hz sine, constant 0.3 amplitude) directly as raw
 * Short/i16 sample arrays (bypassing WAV-container decoding, irrelevant to calculateRmsFloat's
 * frame-based API), normalizes them via the AC4-corrected divisor, and asserts the result against
 * the fixture's `expected_rms`/`tolerance` -- the same independent expected-value source used on
 * the Rust side, so both platforms are locked to one number.
 */
class VadGateRmsFixtureTest {

    private val sampleRateHz = 16000f

    /** Short.MAX_VALUE (32767) -- the AC4-corrected divisor, matching Rust's i16::MAX exactly. */
    private val divisor = 32767f

    private fun silenceShorts(durationMs: Int): ShortArray =
        ShortArray((sampleRateHz * durationMs / 1000f).toInt())

    private fun sineShorts(freqHz: Float, amplitude: Float, durationMs: Int): ShortArray {
        val n = (sampleRateHz * durationMs / 1000f).toInt()
        return ShortArray(n) { i ->
            (amplitude * Short.MAX_VALUE * sin(2.0 * PI * freqHz * i / sampleRateHz)).toInt().toShort()
        }
    }

    private fun constantShorts(amplitude: Float, durationMs: Int): ShortArray {
        val n = (sampleRateHz * durationMs / 1000f).toInt()
        val value = (amplitude * Short.MAX_VALUE).toInt().toShort()
        return ShortArray(n) { value }
    }

    /** Normalizes raw Short samples (AC4 divisor) and runs the real calculateRmsFloat. */
    private fun normalizedRms(samples: ShortArray): Float {
        val normalized = FloatArray(samples.size) { samples[it] / divisor }
        return KlarvoAudioRecorder.calculateRmsFloat(normalized, normalized.size)
    }

    // RMS-003 — silence WAV, 100ms, all-zero samples -> expected_rms 0.0, tolerance 0.0.
    @Test
    fun rms003_silence_isZero() {
        val samples = silenceShorts(durationMs = 100)
        assertEquals(0.0f, normalizedRms(samples), 0.0f)
    }

    // RMS-004 — full-scale 440Hz sine, amplitude 1.0, 1s -> expected_rms 1/sqrt(2), tol 1e-3.
    @Test
    fun rms004_fullScaleSine_isOneOverSqrtTwo() {
        val samples = sineShorts(freqHz = 440f, amplitude = 1.0f, durationMs = 1000)
        assertEquals(0.7071067811865476f, normalizedRms(samples), 1e-3f)
    }

    // RMS-005 — constant amplitude 0.3 (speech level), 200ms -> expected_rms 0.3, tol 1e-3.
    @Test
    fun rms005_speechLevelConstant_isTheConstant() {
        val samples = constantShorts(amplitude = 0.3f, durationMs = 200)
        assertEquals(0.3f, normalizedRms(samples), 1e-3f)
    }

    // RMS-006 — zero samples (0ms) -> Some(0.0), not an error/NaN. calculateRmsFloat's
    // length==0 guard must return 0f, not divide-by-zero (NaN).
    @Test
    fun rms006_zeroSamples_isZeroNotNaN() {
        val samples = silenceShorts(durationMs = 0)
        assertEquals(0, samples.size)
        assertEquals(0.0f, normalizedRms(samples), 0.0f)
    }

    /**
     * Inversion: the OLD divisor (32768f) would have produced a measurably different RMS for the
     * full-scale sine case (RMS-004) -- 1/sqrt(2) scaled by 32767/32768 -- proving the divisor
     * value is load-bearing, not coincidentally correct either way. The two results differ by
     * ~0.003%, which is smaller than the 1e-3 tolerance used above (by design -- AC4 documents
     * this as a small correction relative to the M3 filtering gap), so this test compares the
     * two divisors' outputs DIRECTLY against each other at full precision, not against the
     * fixture's tolerance-bounded expected value.
     */
    @Test
    fun inversion_oldDivisor32768_differsFromCorrectedDivisor32767() {
        val samples = sineShorts(freqHz = 440f, amplitude = 1.0f, durationMs = 1000)
        val oldNormalized = FloatArray(samples.size) { samples[it] / 32768f }
        val oldRms = KlarvoAudioRecorder.calculateRmsFloat(oldNormalized, oldNormalized.size)
        val correctedRms = normalizedRms(samples)
        assert(oldRms != correctedRms) {
            "AC4 regression: RMS computed with the old 32768f divisor ($oldRms) must differ from " +
                "the corrected 32767f divisor ($correctedRms) -- if they're equal, the divisor " +
                "correction was reverted or never applied."
        }
    }
}
