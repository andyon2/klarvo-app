package com.klarvo.voice

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Cross-platform chunking parity test.
 *
 * Reads test-fixtures/chunking-cleanup-vectors.json from the repo root — the
 * SAME fixture the Rust `spec_chunking_vectors_*` tests consume — so Kotlin
 * `KlarvoApi.splitIntoChunks` and Rust `split_into_chunks` cannot silently drift
 * on this contract. The JSON is the single source of truth for both consumers.
 *
 * Born from the history id=3041 meta-refusal leak: `splitIntoChunks` orphaned a
 * trailing "." into its own chunk, which the LLM cleanup then "refused" to clean
 * with a conversational reply that leaked into the user's pasted text.
 *
 * Scope note: this asserts the SPLIT-level invariant (no trivial chunk reaches
 * the LLM) — the structural root-cause guard. The cleanupChunked per-chunk
 * passthrough is defense-in-depth verified by the Rust trait-mock integration
 * test (`spec_chunking_vectors_no_meta_refusal`); Android has no injectable LLM
 * seam to mock the network here.
 */
class ChunkingVectorsTest {

    // --- Minimal dependency-free JSON (mirrors WavRmsVectorsTest) ---

    private sealed class JsonVal {
        data class Obj(val map: Map<String, JsonVal>) : JsonVal()
        data class Arr(val list: List<JsonVal>) : JsonVal()
        data class Str(val v: String) : JsonVal()
        data class Num(val v: Double) : JsonVal()
        object Null : JsonVal()

        fun asString() = (this as? Str)?.v ?: error("Expected string, got $this")
        fun asArray() = (this as? Arr)?.list ?: error("Expected array, got $this")
        fun get(key: String) = (this as? Obj)?.map?.get(key) ?: Null
        fun optString(key: String, default: String = "") =
            (this as? Obj)?.map?.get(key)?.let { (it as? Str)?.v } ?: default
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
        val name = "test-fixtures/chunking-cleanup-vectors.json"
        val candidates = listOf(
            cwd.resolve("../../../../$name"), // gen/android/app/ → repo root
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

    // --- The parity assertions ---

    // Independent triviality predicate — must NOT call the production
    // KlarvoApi.isTrivialChunk, or flipping that guard would also blind this test
    // (the SUT must not judge itself).
    private fun looksTrivial(c: String): Boolean = c.none { it.isLetterOrDigit() }

    @Test
    fun splitProducesNoTrivialChunkAndKeepsTrailingPunctuation() {
        val vectors = loadFixture()
        assertTrue("fixture must not be empty", vectors.isNotEmpty())
        for (v in vectors) {
            val id = v.optString("id", "?")
            val input = v.get("input").asString()
            val chunks = KlarvoApi.splitIntoChunks(input)

            if (v.optBool("expect_no_trivial_chunk")) {
                chunks.forEachIndexed { i, c ->
                    assertFalse(
                        "[$id] chunk $i is a lone trivial fragment (would draw an LLM refusal): '$c'",
                        looksTrivial(c)
                    )
                }
            }

            val suffix = v.optString("expect_last_chunk_ends_with")
            if (suffix.isNotEmpty()) {
                val last = chunks.lastOrNull() ?: ""
                assertTrue(
                    "[$id] last chunk must end with '$suffix', got: '$last'",
                    last.endsWith(suffix)
                )
            }
        }
    }
}
