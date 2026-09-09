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
 *   Nyquist), so these vectors pin `isEnergyAboveGate`'s `>=` boundary precisely via
 *   `amplitude_short` -- a LITERAL precomputed offline from 32767 and baked into the fixture, not
 *   derived from [KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR] at test time (a reverted
 *   32768f divisor can no longer cancel out against this fixture's own amplitude derivation).
 * - `bass_tone` ([bassToneShorts], VAD-GATE-001): a 20-60 Hz tone that the 85 Hz highpass
 *   meaningfully attenuates -- its raw (unfiltered) RMS passes the gate but its production-seam
 *   filtered RMS does not, making the filter load-bearing for this vector's outcome.
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
        fun optDouble(key: String, default: Double = 0.0) =
            (this as? Obj)?.map?.get(key)?.let { (it as? Num)?.v } ?: default
        fun optBool(key: String, default: Boolean = false) =
            (this as? Obj)?.map?.get(key)?.let { (it as? Num)?.v?.let { n -> n != 0.0 } } ?: default
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
            val threshold = v.optDouble("silence_threshold").toFloat()
            val expectedOpen = v.optBool("expected_gate_open")
            // amplitude_short is a LITERAL baked into the fixture (precomputed offline from a
            // literal 32767, e.g. ceil(target * 32767)) -- NOT derived here from
            // KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR, so a reverted production divisor
            // cannot cancel out against this test's own amplitude derivation (round-2 review
            // finding).
            val amplitudeShort = v.optDouble("amplitude_short").toInt().toShort()
            val signal = v.optString("signal", "nyquist_square")

            val samples = when (signal) {
                "bass_tone" -> {
                    val freqHz = v.optDouble("signal_freq_hz").toFloat()
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
            val silenceSecs = v.optDouble("silence_secs").toFloat()
            val expectedFrames = v.optDouble("expected_frames").toInt()
            val actualFrames = KlarvoAudioRecorder.framesForSeconds(silenceSecs)
            assertEquals(
                "$id: framesForSeconds($silenceSecs) must be $expectedFrames (AC2/AC6 floor+fps)",
                expectedFrames,
                actualFrames
            )
        }
    }
}
