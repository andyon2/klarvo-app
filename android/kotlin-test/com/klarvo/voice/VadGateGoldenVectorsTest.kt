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

    @Test
    fun energyFloorVectors_matchIsEnergyAboveGate_atDefaultAndTunedThreshold() {
        val vectors = loadFixture().filter { (it as JsonVal.Obj).optString("category") == "energy-floor" }
        assertTrue("energy-floor golden vectors must not be empty", vectors.isNotEmpty())
        for (v in vectors) {
            val id = v.optString("id")
            val threshold = v.optDouble("silence_threshold").toFloat()
            val rms = v.optDouble("normalized_rms").toFloat()
            val expectedOpen = v.optBool("expected_gate_open")
            val actualOpen = KlarvoAudioRecorder.isEnergyAboveGate(rms, threshold)
            assertEquals(
                "$id: isEnergyAboveGate(rms=$rms, threshold=$threshold) must be $expectedOpen",
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
