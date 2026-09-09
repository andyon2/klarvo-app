package com.klarvo.voice

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
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
 * [KlarvoAudioRecorder.calculateRmsFloat]): it ACTUALLY LOADS the fixture (Story 7-2 review
 * finding -- the previous version of this file claimed to but only held inline literal copies,
 * so fixture and test could drift silently) and, for every vector with a populated
 * `expected_rms_kotlin`, builds the SAME independently-known signal the vector's `wav_encoding`
 * describes directly as a raw Short/i16 sample array (bypassing WAV-container decoding, which is
 * irrelevant to `calculateRmsFloat`'s frame-based API), normalizes it via the AC4-corrected
 * divisor, and asserts the result against `expected_rms_kotlin`/`tolerance`.
 */
class VadGateRmsFixtureTest {

    private val sampleRateHz = 16000f

    /**
     * Short.MAX_VALUE (32767) -- the AC4-corrected divisor, matching Rust's i16::MAX exactly.
     * Asserted against the real production constant (not a test-local copy) so a reverted/wrong
     * divisor in [KlarvoAudioRecorder] fails here (Story 7-2 review finding).
     */
    private val divisor = KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR

    @Test
    fun divisor_matchesProductionConstant_notATestLocalCopy() {
        assertEquals(32767f, KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR, 0f)
    }

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

    /**
     * Loads `test-fixtures/wav-rms-vectors.json` from the repo root, resolved relative to
     * whatever CWD gradle runs the JVM test suite from (mirrors [VadGateGoldenVectorsTest]'s
     * candidate-path search).
     */
    private fun loadFixture(): JSONArray {
        val cwd = File(System.getProperty("user.dir") ?: ".")
        val name = "test-fixtures/wav-rms-vectors.json"
        val candidates = listOf(
            cwd.resolve("../../../../$name"), // gen/android/app/ -> repo root
            cwd.resolve(name),
            cwd.resolve("../$name"),
            cwd.resolve("../../../$name"),
        )
        val found = candidates.firstOrNull { it.canonicalFile.exists() }
            ?: error(
                "Cannot find $name. Tried:\n" +
                    candidates.joinToString("\n") { "  ${it.canonicalPath}" } +
                    "\nCWD=${cwd.canonicalPath}"
            )
        return JSONArray(found.readText())
    }

    /** Builds the raw Short/i16 samples a fixture vector's `wav_encoding` describes. */
    private fun samplesFor(encoding: JSONObject): ShortArray {
        val durationMs = encoding.getInt("duration_ms")
        val amplitude = encoding.optDouble("amplitude", 0.0).toFloat()
        return when (encoding.getString("type")) {
            "sine" -> sineShorts(
                freqHz = encoding.getDouble("freq_hz").toFloat(),
                amplitude = amplitude,
                durationMs = durationMs
            )
            "synthetic" -> if (amplitude == 0f) silenceShorts(durationMs) else constantShorts(amplitude, durationMs)
            else -> error("samplesFor: unsupported wav_encoding.type '${encoding.getString("type")}'")
        }
    }

    @Test
    fun fixtureVectors_matchExpectedRmsKotlin() {
        val vectors = loadFixture()
        var exercised = 0
        for (i in 0 until vectors.length()) {
            val vector = vectors.getJSONObject(i)
            if (vector.isNull("expected_rms_kotlin")) continue // RMS-007: float32, not applicable (see note below)
            val encoding = vector.getJSONObject("wav_encoding")
            if (encoding.getString("type") == "raw_bytes") continue // RMS-001/002: WAV-container-level, N/A here
            val id = vector.getString("id")
            val expected = vector.getDouble("expected_rms_kotlin").toFloat()
            val tolerance = vector.optDouble("tolerance", 0.0).toFloat()
            val samples = samplesFor(encoding)
            val actual = normalizedRms(samples)
            assertEquals("$id: normalizedRms mismatch", expected, actual, tolerance)
            exercised++
        }
        assertTrue("fixtureVectors_matchExpectedRmsKotlin exercised zero vectors", exercised > 0)
    }

    // RMS-006 — zero samples (0ms) -> Some(0.0), not an error/NaN. calculateRmsFloat's
    // length==0 guard must return 0f, not divide-by-zero (NaN). Covered by the fixture-driven
    // test above too, kept explicit here since it exercises a distinct code path (empty array).
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
