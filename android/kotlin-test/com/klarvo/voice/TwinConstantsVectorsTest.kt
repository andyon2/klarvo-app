package com.klarvo.voice

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Twin-constant lock (story 7-8, AC3) — Kotlin half.
 *
 * Reads the SAME file as the Rust half (`llm::tests::spec_twin_constants_*` in
 * src-tauri/src/llm/mod.rs): `test-fixtures/twin-constants-vectors.json` at the repo
 * root. Five Rust to Kotlin twins that were unlocked and could silently re-diverge.
 *
 * ## Discipline
 * Every assertion binds to a PRODUCTION symbol or a production seam
 * (`KlarvoApi.CLEANUP_TEMPERATURE`, `CLEANUP_MAX_TOKENS`, `shouldChunk`,
 * `splitIntoChunks`, `joinChunkResults`) and compares it to the FIXTURE literal —
 * never to another production symbol, which would agree no matter what both said.
 * That is the "SUT must not judge itself" rule already carried by
 * `ChunkingVectorsTest`, applied to constants.
 *
 * Uses `org.json` with the THROWING accessors (`getDouble`/`getInt`/`getString`), not the
 * hand-rolled `JsonVal` parser: a typo'd or missing fixture key must fail loudly, never
 * fall back to a default and pass vacuously. That vacuous-default shape is exactly the
 * defect story 7-2 round 3 recorded as R3-P2.
 *
 * ## What this covers — and what it does NOT
 * Covers: the five twin values as they exist in the Kotlin production tree, on the JVM.
 * Does NOT cover: the Rust side (its own test in `llm/mod.rs` reads this same fixture --
 * neither half can prove the other), any network request actually carrying these values,
 * the dead-config cluster (`advanced.llmTemperature` and friends -- deliberately NOT
 * locked, see the scope guard in story 7-8 AC3), or Android runtime behaviour on a device.
 */
class TwinConstantsVectorsTest {

    private fun loadFixture(): Map<String, JSONObject> {
        val cwd = File(System.getProperty("user.dir") ?: ".")
        val name = "test-fixtures/twin-constants-vectors.json"
        val candidates = listOf(
            cwd.resolve("../../../../$name"), // gen/android/app/ to repo root
            cwd.resolve(name),
            cwd.resolve("../$name"),
            cwd.resolve("../../../$name"),
        )
        val found = candidates.firstOrNull { it.canonicalFile.exists() }
            ?: error(
                "Cannot find $name. Tried:" +
                    candidates.joinToString("") { System.lineSeparator() + "  " + it.canonicalPath } +
                    System.lineSeparator() + "CWD=" + cwd.canonicalPath
            )
        val arr = JSONArray(found.readText())
        val byId = LinkedHashMap<String, JSONObject>()
        for (i in 0 until arr.length()) {
            val o = arr.getJSONObject(i)
            byId[o.getString("id")] = o
        }
        return byId
    }

    /** Throwing lookup: an unknown id must fail loudly, not silently skip the assertion. */
    private fun vector(id: String): JSONObject =
        loadFixture()[id] ?: error("fixture has no vector with id=$id")

    @Test
    fun fixtureCarriesAllFiveTwinsAndDescribesEach() {
        val byId = loadFixture()
        val expected = listOf(
            "TWIN-LLM-TEMPERATURE-001",
            "TWIN-LLM-MAX-TOKENS-001",
            "TWIN-CHUNK-THRESHOLD-001",
            "TWIN-CHUNK-TARGET-SIZE-001",
            "TWIN-CHUNK-JOIN-SEPARATOR-001",
        )
        assertEquals("the twin fixture must carry exactly the five locked twins", expected.toSet(), byId.keys)
        for (id in expected) {
            val d = byId.getValue(id).getString("description")
            assertTrue("$id needs a description stating what it pins", d.contains("PINS:"))
            assertTrue("$id must state what it does NOT pin", d.contains("DOES NOT PIN:"))
        }
    }

    @Test
    fun cleanupTemperatureMatchesFixture() {
        val expected = vector("TWIN-LLM-TEMPERATURE-001").getDouble("expected_double")
        assertEquals(
            "Kotlin cleanup temperature must equal the Rust OpenAiCompatibleCleanup twin",
            expected,
            KlarvoApi.CLEANUP_TEMPERATURE,
            0.0
        )
    }

    @Test
    fun cleanupMaxTokensMatchesFixture() {
        val expected = vector("TWIN-LLM-MAX-TOKENS-001").getInt("expected_int")
        assertEquals(
            "Kotlin cleanup max_tokens must equal the Rust OpenAiCompatibleCleanup twin",
            expected,
            KlarvoApi.CLEANUP_MAX_TOKENS
        )
    }

    @Test
    fun chunkThresholdBoundaryMatchesFixture() {
        val threshold = vector("TWIN-CHUNK-THRESHOLD-001").getInt("expected_int")
        // Probe the public seam rather than the private constant: this also catches a
        // flipped comparison operator, which reading the constant would not.
        // ASCII only, so one char == one UTF-8 byte.
        assertTrue(
            "text of exactly $threshold bytes must chunk (Rust: raw_text.len() < CHUNK_THRESHOLD is the single-call case)",
            KlarvoApi.shouldChunk("a".repeat(threshold))
        )
        assertFalse(
            "text of ${threshold - 1} bytes must NOT chunk",
            KlarvoApi.shouldChunk("a".repeat(threshold - 1))
        )
    }

    @Test
    fun chunkTargetSizeFallbackSplitMatchesFixture() {
        val target = vector("TWIN-CHUNK-TARGET-SIZE-001").getInt("expected_int")
        // A boundary-free input: no '.', '!', '?' or newline anywhere, so bestSplit stays
        // null and the fallback offset (start + CHUNK_TARGET_SIZE) is what decides the cut.
        val chunks = KlarvoApi.splitIntoChunks("a".repeat(target * 3))
        assertTrue("input well above the target must produce more than one chunk", chunks.size > 1)
        assertEquals(
            "with no sentence boundary to find, the first cut must land exactly on CHUNK_TARGET_SIZE",
            target,
            chunks[0].length
        )
    }

    @Test
    fun chunkJoinSeparatorMatchesFixture() {
        val code = vector("TWIN-CHUNK-JOIN-SEPARATOR-001").getInt("expected_char_code")
        val sep = code.toChar()
        val joined = KlarvoApi.joinChunkResults(listOf("alpha", "beta"))
        assertEquals(
            "chunk results must be joined with a single separator char (code $code)",
            "alpha" + sep + "beta",
            joined
        )
        // Three parts: proves it is one separator PER seam, not a trailing or doubled one.
        assertEquals(
            "three chunks must be joined with exactly two separators",
            "a" + sep + "b" + sep + "c",
            KlarvoApi.joinChunkResults(listOf("a", "b", "c"))
        )
    }
}
