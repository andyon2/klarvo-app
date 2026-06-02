package com.klarvo.voice

import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.math.PI
import kotlin.math.sin

/**
 * Cross-platform WAV-RMS contract test — Story 3.3, AC-4.
 *
 * Reads test-fixtures/wav-rms-vectors.json from the repo root and validates
 * SilencePreFilter.computeWavRms() against each vector. The JSON fixture is
 * the single source of truth for both Rust and Kotlin consumers.
 *
 * Per-platform expectations: some vectors carry an `expected_rms_kotlin` field
 * that overrides `expected_rms` for the Kotlin consumer, together with a
 * `divergence_reason` documenting the asymmetry. For example RMS-007 (float32
 * WAV) has `expected_rms_kotlin: null` because SilencePreFilter.computeWavRms
 * returns null for audioFormat=3 (IEEE float) after the audioFormat guard added
 * in Story 3.3, while Rust's compute_wav_rms returns Some(0.5) via its
 * SampleFormat::Float path. The JSON fixture is the first-class contract for
 * this cross-platform divergence.
 *
 * AI-2 mandate: all assertions call the real SilencePreFilter.computeWavRms()
 * with real WAV bytes — no mock inputs that bypass WAV parsing.
 */
class WavRmsVectorsTest {

    // ---------------------------------------------------------------------------
    // Fixture loading — reads JSON without Android or extra dependencies
    // ---------------------------------------------------------------------------

    /**
     * A minimal JSON value representation for the fixture vectors.
     */
    private sealed class JsonVal {
        data class Obj(val map: Map<String, JsonVal>) : JsonVal()
        data class Arr(val list: List<JsonVal>) : JsonVal()
        data class Str(val v: String) : JsonVal()
        data class Num(val v: Double) : JsonVal()
        object Null : JsonVal()

        fun asString() = (this as? Str)?.v ?: error("Expected string, got $this")
        fun asDouble() = (this as? Num)?.v ?: error("Expected number, got $this")
        fun asLong() = asDouble().toLong()
        fun asInt() = asDouble().toInt()
        fun asFloat() = asDouble().toFloat()
        fun asArray() = (this as? Arr)?.list ?: error("Expected array, got $this")
        fun asObject() = (this as? Obj)?.map ?: error("Expected object, got $this")
        fun isNull() = this is Null
        fun get(key: String) = (this as? Obj)?.map?.get(key) ?: Null
        fun optString(key: String, default: String = "") = (this as? Obj)?.map?.get(key)?.let { (it as? Str)?.v } ?: default
        fun optDouble(key: String, default: Double = 0.0) = (this as? Obj)?.map?.get(key)?.let { (it as? Num)?.v } ?: default
        fun optInt(key: String, default: Int = 0) = optDouble(key, default.toDouble()).toInt()
        fun optLong(key: String, default: Long = 0L) = optDouble(key, default.toDouble()).toLong()
    }

    /** Minimal recursive-descent JSON parser for the fixture format. */
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
            pos++ // closing "
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
                        skipWs()
                        val v = parseValue()
                        map[key] = v
                        skipWs()
                        if (pos < src.length && src[pos] == ',') pos++
                        skipWs()
                    }
                    pos++ // }
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
                    pos++ // ]
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

    /**
     * Resolves the shared fixture file from the repo root.
     *
     * Gradle JVM unit test working directory is the :app module dir
     * (src-tauri/gen/android/app/). From there, repo root is 4 levels up.
     * Falls back to other likely CWDs if the primary path is missing.
     */
    private fun loadFixture(): List<JsonVal> {
        val cwd = File(System.getProperty("user.dir") ?: ".")
        val candidates = listOf(
            // Gradle :app CWD: gen/android/app/ → 4 levels up = repo root
            cwd.resolve("../../../../test-fixtures/wav-rms-vectors.json"),
            // Fallback: CWD is already the repo root
            cwd.resolve("test-fixtures/wav-rms-vectors.json"),
            // Fallback: CWD is android/
            cwd.resolve("../test-fixtures/wav-rms-vectors.json"),
            // Fallback: CWD is src-tauri/gen/android/
            cwd.resolve("../../../test-fixtures/wav-rms-vectors.json"),
        )
        val found = candidates.firstOrNull { it.canonicalFile.exists() }
            ?: error(
                "Cannot find test-fixtures/wav-rms-vectors.json. Tried:\n" +
                    candidates.joinToString("\n") { "  ${it.canonicalPath}" } +
                    "\nCWD=${cwd.canonicalPath}"
            )
        return (parseJson(found.readText()) as JsonVal.Arr).list
    }

    // ---------------------------------------------------------------------------
    // WAV builder helpers — mirrors Rust make_wav / make_float_wav
    // ---------------------------------------------------------------------------

    /**
     * Builds WAV bytes for a test vector's wav_encoding specification.
     *
     * - "raw_bytes"  → return those bytes directly (RMS-001/RMS-002)
     * - "synthetic"  → constant-amplitude i16 PCM WAV (RMS-003/RMS-005/RMS-006)
     * - "sine"       → sine-wave i16 PCM WAV (RMS-004)
     * - "synthetic" + sample_format=float → float32 WAV (RMS-007)
     */
    private fun buildVectorWav(encoding: JsonVal): ByteArray {
        return when (val type = encoding.optString("type")) {
            "raw_bytes" -> {
                val arr = encoding.get("bytes").asArray()
                ByteArray(arr.size) { i -> arr[i].asInt().toByte() }
            }
            "synthetic" -> {
                val sampleRate = encoding.optInt("sample_rate", 16000)
                val durationMs = encoding.optLong("duration_ms", 0L)
                val amplitude = encoding.optDouble("amplitude", 0.0).toFloat()
                val bits = encoding.optInt("bits_per_sample", 16)
                val sampleFormat = encoding.optString("sample_format", "int")
                val nSamples = (sampleRate * durationMs / 1000L).toInt()
                if (sampleFormat == "float" || bits == 32) {
                    buildFloat32Wav(FloatArray(nSamples) { amplitude }, sampleRate)
                } else {
                    val pcm = ShortArray(nSamples) { (amplitude * 32767f).toInt().toShort() }
                    encodeWav(pcm, sampleRate)
                }
            }
            "sine" -> {
                val sampleRate = encoding.optInt("sample_rate", 16000)
                val durationMs = encoding.optLong("duration_ms", 1000L)
                val freqHz = encoding.optDouble("freq_hz", 440.0).toFloat()
                val amplitude = encoding.optDouble("amplitude", 1.0).toFloat()
                val nSamples = (sampleRate * durationMs / 1000L).toInt()
                val pcm = ShortArray(nSamples) { i ->
                    val v = amplitude * sin(2.0 * PI * freqHz * i / sampleRate).toFloat()
                    (v.coerceIn(-1.0f, 1.0f) * 32767f).toInt().toShort()
                }
                encodeWav(pcm, sampleRate)
            }
            else -> error("Unknown wav_encoding type: $type")
        }
    }

    /**
     * Builds a float32 (audioFormat=3) WAV from raw f32 samples.
     * Used for RMS-007 (float32 WAV construction for the cross-platform delta test).
     */
    private fun buildFloat32Wav(samples: FloatArray, sampleRate: Int = 16000): ByteArray {
        val dataBytes = samples.size * 4  // 32-bit float = 4 bytes each
        val buf = ByteBuffer.allocate(44 + dataBytes).order(ByteOrder.LITTLE_ENDIAN)
        buf.put("RIFF".toByteArray(Charsets.US_ASCII))
        buf.putInt(36 + dataBytes)
        buf.put("WAVE".toByteArray(Charsets.US_ASCII))
        buf.put("fmt ".toByteArray(Charsets.US_ASCII))
        buf.putInt(16)              // fmt chunk size
        buf.putShort(3)             // audioFormat = 3 (IEEE float)
        buf.putShort(1)             // channels = 1 (mono)
        buf.putInt(sampleRate)      // sample rate
        buf.putInt(sampleRate * 4)  // byte rate (32-bit mono)
        buf.putShort(4)             // block align
        buf.putShort(32)            // bits per sample
        buf.put("data".toByteArray(Charsets.US_ASCII))
        buf.putInt(dataBytes)
        for (s in samples) buf.putFloat(s)
        return buf.array()
    }

    // ---------------------------------------------------------------------------
    // Parametric spec test (AC-4)
    // ---------------------------------------------------------------------------

    /**
     * Iterates all vectors from test-fixtures/wav-rms-vectors.json and asserts
     * SilencePreFilter.computeWavRms() against the expected value for this platform.
     *
     * For each vector the expected value is resolved as follows:
     * - If the vector contains an `expected_rms_kotlin` field (even if null), that
     *   value is used as the Kotlin expectation.
     * - Otherwise `expected_rms` is used.
     *
     * This allows the JSON fixture to act as first-class contract data for
     * cross-platform divergences (e.g. RMS-007 float32 WAV) without hardcoding
     * platform branches in test code. See each vector's `divergence_reason` field
     * for explanation of any asymmetry.
     */
    @Test
    fun vectors_matchExpectedRms() {
        val vectors = loadFixture()
        var tested = 0

        for (v in vectors) {
            val id = v.get("id").asString()
            val wav = buildVectorWav(v.get("wav_encoding"))
            val result = SilencePreFilter.computeWavRms(wav)

            // Resolve the platform-specific expected value: prefer expected_rms_kotlin
            // if present in the fixture (it may be null to document a known asymmetry),
            // otherwise fall back to expected_rms.
            val kotlinExpectationNode = v.get("expected_rms_kotlin")
            val expectedNode = if (kotlinExpectationNode !is JsonVal.Null || v.asObject().containsKey("expected_rms_kotlin")) {
                kotlinExpectationNode
            } else {
                v.get("expected_rms")
            }

            if (expectedNode.isNull()) {
                assertNull("[$id] expected null but got $result", result)
            } else {
                val expected = expectedNode.asFloat()
                val tolerance = v.get("tolerance").let { if (it.isNull()) 1e-3f else it.asFloat() }
                assertNotNull("[$id] expected Some($expected) but got null", result)
                val rms = result!!
                assertTrue(
                    "[$id] RMS $rms not within $tolerance of expected $expected " +
                        "(diff=${Math.abs(rms - expected)})",
                    Math.abs(rms - expected) <= tolerance
                )
            }
            tested++
        }

        assertTrue("Expected at least 7 vectors, got $tested", tested >= 7)
    }
}
