package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * AC7 (epic DoD) — Story 7-2: seeds the golden-vector fixtures the Test-Architect note requires
 * ("energy-floor + stop-latency at default config and at one tuned config") for the Story 7.7
 * golden-vector parity net to consolidate later. Does NOT build the full 7.7 net -- just proves
 * this story's own AC1/AC2/AC3/AC4/AC6 fixes agree with the documented Rust behavior at both a
 * default and a deliberately tuned configuration.
 *
 * Reads `test-fixtures/vad-gate-golden-vectors-7-2.json` -- expected values are independently
 * hand-derived from the formulas documented in the ACs (energy gate: `normalizedRms >= threshold`;
 * frame count: `ceil(secs * 31.25).coerceAtLeast(7)`), not from calling
 * [KlarvoAudioRecorder.isEnergyAboveGate]/[KlarvoAudioRecorder.framesForSeconds] and recording
 * whatever they return (feedback_test_must_not_judge_sut_with_itself).
 *
 * The energy-floor vectors feed RAW i16 sample frames through the actual RMS computation this
 * story changed -- [KlarvoAudioRecorder.vadGateFilteredFrame] (normalize + highpass-filter) then
 * [KlarvoAudioRecorder.calculateRmsFloat] -- rather than asserting a pre-computed `normalized_rms`
 * straight into `isEnergyAboveGate`'s bare `>=` (Story 7-2 review finding round 1: the latter
 * never exercised the filtered signal or the 32767 divisor this story actually changed).
 *
 * Two signal shapes are used (round-2 review finding: a Nyquist-only fixture is provably
 * insensitive to both the highpass filter AND the divisor, since a Butterworth highpass has exact
 * unity gain at Nyquist and the amplitude derivation used to cancel against a reverted divisor):
 * - `nyquist_square` ([nyquistSquareWaveShorts]): unaffected by the highpass filter (unity gain at
 *   Nyquist). `amplitude_short` is a LITERAL baked into the fixture, not derived from
 *   [KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR] at test time, so the test no longer
 *   derives its own input from the symbol under test.
 * - `bass_tone` ([bassToneShorts], VAD-GATE-001): a 20-60 Hz tone that the 85 Hz highpass
 *   meaningfully attenuates -- its raw (unfiltered) RMS passes the gate but its production-seam
 *   filtered RMS does not, making the filter load-bearing for this vector's outcome.
 *
 * ## What these vectors do NOT pin (story 7-8 / 7-2 round-3 finding R3-P6)
 * An earlier version of this KDoc claimed more than the vectors deliver. Corrected:
 * - **Not the `>=` boundary.** The amplitudes are computed with `ceil`, which places even the
 *   "at threshold" vectors STRICTLY above their threshold (VAD-GATE-002: 0.00500504 vs 0.005;
 *   VAD-GATE-005: 0.02002014 vs 0.02). Flipping [KlarvoAudioRecorder.isEnergyAboveGate] from
 *   `>=` to `>` leaves all six green. The `>=` boundary is pinned by `SilenceThresholdTest`, not
 *   here.
 * - **Not the normalization divisor.** A reverted 32768f divisor is not caught by these vectors;
 *   the divisor is pinned by `VadGateRmsFixtureTest`. Baking the amplitude as a literal only
 *   removed the cancellation that would have masked such a revert — it did not add a divisor
 *   assertion.
 * - **Not Silero.** No VAD model runs here; only the energy-gate half of the decision is
 *   exercised. The Silero-input half is covered by `HighpassFilterTest`'s `vadGateDecision` tests.
 * What they DO pin: that a given raw i16 signal, pushed through the real
 * [KlarvoAudioRecorder.vadGateFilteredFrame] + [KlarvoAudioRecorder.calculateRmsFloat] path,
 * lands on the documented side of the configured threshold — and, for VAD-GATE-001 specifically,
 * that the highpass is load-bearing in reaching that verdict.
 */
class VadGateGoldenVectorsTest {

    // --- Minimal dependency-free JSON (mirrors ChunkingVectorsTest/WavRmsVectorsTest) ---

    private sealed class JsonVal {
        data class Obj(val map: Map<String, JsonVal>) : JsonVal()
        data class Arr(val list: List<JsonVal>) : JsonVal()
        data class Str(val v: String) : JsonVal()
        data class Num(val v: Double) : JsonVal()
        object Null : JsonVal()

        fun asArray() = (this as? Arr)?.list ?: error("Expected array, got $this")
        fun optString(key: String, default: String = "") =
            (this as? Obj)?.map?.get(key)?.let { (it as? Str)?.v } ?: default
        /**
         * Throwing numeric accessor (story 7-8, AC5 / 7-2 round-3 finding R3-P2).
         *
         * A `default = 0.0` accessor turns a typo'd or schema-drifted key into a SILENT
         * zero. For this fixture that is not a harmless default: amplitude 0 or frequency 0
         * yields an all-zero signal, RMS 0, gate closed — so every `expected_gate_open: false`
         * vector passes VACUOUSLY while reporting green. A 0.0 silence_threshold is likewise the
         * most permissive threshold possible, and a 0.0 silence_secs floors to 7 frames.
         *
         * Every numeric key in this file reads through this accessor; there is no `optDouble`
         * left to reach for. The one defaulting reader that remains is [optString] for
         * `signal`, whose default (`nyquist_square`) is a real schema default and cannot fake
         * a pass — a wrong signal type changes the measured RMS rather than zeroing it.
         */
        fun getDouble(key: String): Double =
            (this as? Obj)?.map?.get(key)?.let { (it as? Num)?.v }
                ?: error(
                    "fixture vector is missing required numeric key '$key' (or it is not a number) -- " +
                        "a silent default here would let this vector pass vacuously"
                )

        /**
         * Throwing boolean accessor, same rationale (7-8 review round 1).
         *
         * An `optBool(default = false)` reader is the [getDouble] defect in the EXPECTATION
         * column: the three `expected_gate_open: false` vectors would keep passing if the key
         * were dropped or misspelled, because the default agrees with them. The expected value
         * is never optional — a vector without it is a broken vector.
         */
        fun getBool(key: String): Boolean =
            (this as? Obj)?.map?.get(key)?.let { (it as? Num)?.v?.let { n -> n != 0.0 } }
                ?: error(
                    "fixture vector is missing required boolean key '$key' -- " +
                        "a silent default here would let this vector pass vacuously"
                )
    }

    private fun parseJson(src: String): JsonVal {
        var pos = 0
        fun skipWs() { while (pos < src.length && src[pos] in " \t\n\r") pos++ }
        fun parseString(): JsonVal.Str {
            check(src[pos] == '"') { "Expected '\"' at $pos" }
            pos++
            val sb = StringBuilder()
            while (pos < src.length && src[pos] != '"') {
                if (src[pos] == '\\') { pos++; sb.append(src[pos]) } else sb.append(src[pos])
                pos++
            }
            pos++
            return JsonVal.Str(sb.toString())
        }
        fun parseNumber(): JsonVal.Num {
            val start = pos
            if (pos < src.length && src[pos] == '-') pos++
            while (pos < src.length && (src[pos].isDigit() || src[pos] in ".eE+-")) pos++
            return JsonVal.Num(src.substring(start, pos).toDouble())
        }
        fun parseValue(): JsonVal {
            skipWs()
            return when {
                pos >= src.length -> error("Unexpected end of input")
                src[pos] == '"' -> parseString()
                src[pos] == '{' -> {
                    pos++; skipWs()
                    val map = mutableMapOf<String, JsonVal>()
                    while (pos < src.length && src[pos] != '}') {
                        skipWs()
                        val key = parseString().v
                        skipWs(); check(src[pos] == ':') { "Expected ':'" }; pos++
                        map[key] = parseValue()
                        skipWs()
                        if (pos < src.length && src[pos] == ',') pos++
                        skipWs()
                    }
                    pos++
                    JsonVal.Obj(map)
                }
                src[pos] == '[' -> {
                    pos++; skipWs()
                    val list = mutableListOf<JsonVal>()
                    while (pos < src.length && src[pos] != ']') {
                        list.add(parseValue())
                        skipWs()
                        if (pos < src.length && src[pos] == ',') pos++
                        skipWs()
                    }
                    pos++
                    JsonVal.Arr(list)
                }
                src.startsWith("null", pos) -> { pos += 4; JsonVal.Null }
                src.startsWith("true", pos) -> { pos += 4; JsonVal.Num(1.0) }
                src.startsWith("false", pos) -> { pos += 5; JsonVal.Num(0.0) }
                src[pos] == '-' || src[pos].isDigit() -> parseNumber()
                else -> error("Unexpected char '${src[pos]}' at $pos")
            }
        }
        return parseValue()
    }

    private fun loadFixture(): List<JsonVal> {
        val cwd = File(System.getProperty("user.dir") ?: ".")
        val name = "test-fixtures/vad-gate-golden-vectors-7-2.json"
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
        return (parseJson(found.readText()) as JsonVal.Arr).list
    }

    /**
     * A raw i16 square wave alternating +[amplitudeShort]/-[amplitudeShort] every sample --
     * exactly the Nyquist frequency (8 kHz at a 16 kHz sample rate). A 2nd-order Butterworth
     * highpass has EXACT unity gain at Nyquist (the frequency-response numerator/denominator
     * both reduce to the same value there, independent of the cutoff), so this signal's RMS is
     * unchanged by [KlarvoAudioRecorder.vadGateFilteredFrame]'s highpass step -- letting us target
     * a precise post-filter RMS without fighting the filter's frequency response.
     */
    private fun nyquistSquareWaveShorts(amplitudeShort: Short, frameCount: Int): ShortArray {
        val n = frameCount * 512
        return ShortArray(n) { i -> if (i % 2 == 0) amplitudeShort else (-amplitudeShort).toShort() }
    }

    /**
     * A raw i16 sine wave at [freqHz] -- unlike [nyquistSquareWaveShorts], this signal is
     * meaningfully attenuated by the 85 Hz highpass at bass frequencies (20-60 Hz), making it
     * load-bearing for VAD-GATE-001's gate-closed outcome (round-2 review finding).
     */
    private fun bassToneShorts(freqHz: Float, amplitudeShort: Float, seconds: Float): ShortArray {
        val sampleRateHz = 16000f
        val n = (sampleRateHz * seconds).toInt()
        return ShortArray(n) { i ->
            (amplitudeShort * kotlin.math.sin(2.0 * Math.PI * freqHz * i / sampleRateHz)).toInt().toShort()
        }
    }

    /**
     * Feeds [samples] through the real production seam ([KlarvoAudioRecorder.vadGateFilteredFrame]
     * then [KlarvoAudioRecorder.calculateRmsFloat]) in 512-sample frames, skipping the first
     * [settleFrames] so the highpass filter's delay line has settled before RMS is measured.
     */
    private fun productionFilteredRms(samples: ShortArray, settleFrames: Int = 4): Float {
        val filter = HighpassFilter(KlarvoAudioRecorder.HIGHPASS_CUTOFF_HZ, 16000f)
        val frameSize = 512
        val scratch = FloatArray(frameSize)
        val tail = ArrayList<Float>()
        var frameIndex = 0
        var pos = 0
        while (pos + frameSize <= samples.size) {
            val frame = samples.copyOfRange(pos, pos + frameSize)
            val filtered = KlarvoAudioRecorder.vadGateFilteredFrame(frame, frameSize, filter, scratch)
            if (frameIndex >= settleFrames) {
                for (value in filtered) tail.add(value)
            }
            pos += frameSize
            frameIndex++
        }
        return KlarvoAudioRecorder.calculateRmsFloat(tail.toFloatArray(), tail.size)
    }

    @Test
    fun energyFloorVectors_matchIsEnergyAboveGate_atDefaultAndTunedThreshold() {
        val vectors = loadFixture().filter { (it as JsonVal.Obj).optString("category") == "energy-floor" }
        assertTrue("energy-floor golden vectors must not be empty", vectors.isNotEmpty())
        for (v in vectors) {
            val id = v.optString("id")
            val threshold = v.getDouble("silence_threshold").toFloat()
            val expectedOpen = v.getBool("expected_gate_open")
            // amplitude_short is a LITERAL baked into the fixture (precomputed offline from a
            // literal 32767, e.g. ceil(target * 32767)) -- NOT derived here from
            // KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR, so a reverted production divisor
            // cannot cancel out against this test's own amplitude derivation (round-2 review
            // finding).
            val amplitudeShort = v.getDouble("amplitude_short").toInt().toShort()
            val signal = v.optString("signal", "nyquist_square")

            val samples = when (signal) {
                "bass_tone" -> {
                    val freqHz = v.getDouble("signal_freq_hz").toFloat()
                    bassToneShorts(freqHz, amplitudeShort.toFloat(), seconds = 1.0f)
                }
                "nyquist_square" -> nyquistSquareWaveShorts(amplitudeShort, frameCount = 16)
                else -> error("$id: unsupported signal type '$signal'")
            }
            val actualRms = productionFilteredRms(samples)
            val actualOpen = KlarvoAudioRecorder.isEnergyAboveGate(actualRms, threshold)

            assertEquals(
                "$id: production-seam RMS ($actualRms, signal=$signal amplitude_short=$amplitudeShort) " +
                    "vs threshold=$threshold -- isEnergyAboveGate must be $expectedOpen",
                expectedOpen,
                actualOpen
            )
        }
    }

    @Test
    fun stopLatencyVectors_matchFramesForSeconds_atDefaultAndTunedSilenceSecs() {
        val vectors = loadFixture().filter { (it as JsonVal.Obj).optString("category") == "stop-latency" }
        assertTrue("stop-latency golden vectors must not be empty", vectors.isNotEmpty())
        for (v in vectors) {
            val id = v.optString("id")
            val silenceSecs = v.getDouble("silence_secs").toFloat()
            // expected_frames follows the same accessor mechanically now that getDouble exists;
            // it was not one of the named R3-P2 sites.
            val expectedFrames = v.getDouble("expected_frames").toInt()
            val actualFrames = KlarvoAudioRecorder.framesForSeconds(silenceSecs)
            assertEquals(
                "$id: framesForSeconds($silenceSecs) must be $expectedFrames (AC2/AC6 floor+fps)",
                expectedFrames,
                actualFrames
            )
        }
    }
}
