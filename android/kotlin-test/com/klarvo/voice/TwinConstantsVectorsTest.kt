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
 * root. Rust to Kotlin twins that were unlocked and could silently re-diverge.
 *
 * Story 7-9 added the four default cleanup model IDs, and its review round 1
 * (decision D2) added the model-ID sanitisation table. One of the ten entries,
 * `TWIN-CLEANUP-MODEL-ANTHROPIC-001`, is DESKTOP-ONLY: Android has no Anthropic
 * cleanup provider (drift row H5), so this half asserts the other nine and
 * skips that one by id — deliberately, not by oversight.
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
 * Covers: the twin values as they exist in the Kotlin production tree, on the JVM.
 * Does NOT cover: the Rust side (its own test in `llm/mod.rs` reads this same fixture --
 * neither half can prove the other), any network request actually carrying these values,
 * `advanced.llmModelAnthropic` (Desktop-only, drift row H5), a NON-empty model override
 * reaching the request (that is `LlmFallbackProviderTest` / `LlmModelOverrideConfigTest`),
 * or Android runtime behaviour on a device.
 *
 * The dead-config cluster (`advanced.llmTemperature`, `llmMaxTokens`, `chunkThreshold`,
 * `sttTemperature` and friends) that story 7-8 deliberately left unlocked no longer
 * exists: story 7-9 REMOVED those keys from all three layers. The values they claimed to
 * control are still locked here; they are simply no longer settable.
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
    fun fixtureCarriesAllTenEntriesAndDescribesEach() {
        val byId = loadFixture()
        val expected = listOf(
            "TWIN-LLM-TEMPERATURE-001",
            "TWIN-LLM-MAX-TOKENS-001",
            "TWIN-CHUNK-THRESHOLD-001",
            "TWIN-CHUNK-TARGET-SIZE-001",
            "TWIN-CHUNK-JOIN-SEPARATOR-001",
            "TWIN-CLEANUP-MODEL-DEEPSEEK-001",
            "TWIN-CLEANUP-MODEL-OPENAI-001",
            "TWIN-CLEANUP-MODEL-GROQ-001",
            "TWIN-CLEANUP-MODEL-ANTHROPIC-001",
            "TWIN-CLEANUP-MODEL-SANITIZE-001",
        )
        assertEquals("the twin fixture must carry exactly the ten locked entries", expected.toSet(), byId.keys)
        for (id in expected) {
            val d = byId.getValue(id).getString("description")
            assertTrue("$id needs a description stating what it pins", d.contains("PINS:"))
            assertTrue("$id must state what it does NOT pin", d.contains("DOES NOT PIN:"))
        }
    }

    /**
     * Story 7-9: each Kotlin `DEFAULT_MODEL_*` constant against the fixture
     * literal — production symbol vs INDEPENDENT literal, never against the
     * Rust symbol (no cross-language assert exists; the Rust half reads the
     * same fixture).
     *
     * Also pins the empty-override rule through the production seam
     * `KlarvoApi.effectiveCleanupModel`, the twin of Rust's
     * `llm::effective_cleanup_model`: an empty override must resolve to
     * exactly the fixture's default.
     *
     * The Anthropic entry is skipped on purpose — see the class KDoc — and the
     * skip is ASSERTED below rather than left implicit.
     */
    @Test
    fun cleanupModelDefaultsMatchFixture() {
        val cases = listOf(
            Triple("TWIN-CLEANUP-MODEL-DEEPSEEK-001", KlarvoApi.DEFAULT_MODEL_DEEPSEEK, "DeepSeek"),
            Triple("TWIN-CLEANUP-MODEL-OPENAI-001", KlarvoApi.DEFAULT_MODEL_OPENAI, "OpenAI"),
            Triple("TWIN-CLEANUP-MODEL-GROQ-001", KlarvoApi.DEFAULT_MODEL_GROQ, "Groq"),
        )
        for ((id, productionDefault, label) in cases) {
            val fixtureLiteral = vector(id).getString("expected_string")
            assertEquals(
                "$label: the Kotlin default cleanup model drifted from the fixture literal",
                fixtureLiteral,
                productionDefault
            )
            assertEquals(
                "$label: an empty override must resolve to the fixture's default model",
                fixtureLiteral,
                KlarvoApi.effectiveCleanupModel("", productionDefault)
            )
            assertEquals(
                "$label: a whitespace-only override must resolve to the fixture's default model",
                fixtureLiteral,
                KlarvoApi.effectiveCleanupModel("   ", productionDefault)
            )
        }
    }

    /**
     * Review round 1, decision D2: the sanitisation applied to a raw model-ID
     * override, driven through the production seam
     * `KlarvoApi.effectiveCleanupModel` against the fixture's raw -> expected
     * table — the SAME table the Rust twin
     * (`llm::tests::spec_twin_constants_cleanup_model_sanitize`) feeds through
     * `llm::effective_cleanup_model`.
     *
     * PINS: drop every char below U+0020 **and U+0085 (NEL)**, then trim, then
     * "empty means the default" — in that order (review round 3). Two cases
     * force it: the lone-U+0001 case forces the empty check to come last (it
     * survives `trim()` and a strip-after-the-empty-check implementation would
     * return it verbatim as the model ID), and the edge-adjacent cases
     * (NEL-then-space, space-then-NEL, U+001C-then-space) force the filter to
     * come *before* the trim: trimming first leaves the neighbouring space
     * behind on whichever runtime does not treat that control character as
     * whitespace, and Kotlin and Rust disagree in opposite directions (U+0085
     * vs U+001C..U+001F).
     * DOES NOT PIN: which default the empty case selects (that is
     * [cleanupModelDefaultsMatchFixture]), the Desktop-only model-not-found
     * warning text (Android has no equivalent), or any network call.
     */
    @Test
    fun cleanupModelSanitizeMatchesFixture() {
        val entry = vector("TWIN-CLEANUP-MODEL-SANITIZE-001")
        val cases = entry.getJSONArray("cases")
        assertTrue("the sanitize table must not be empty", cases.length() > 0)

        // A sentinel that cannot be produced by sanitising any of the inputs, so
        // "fell back to the default" is unambiguous.
        val sentinelDefault = "SENTINEL-DEFAULT"
        for (i in 0 until cases.length()) {
            val case = cases.getJSONObject(i)
            val raw = case.getString("raw")
            val expected = case.getString("expected")
            val got = KlarvoApi.effectiveCleanupModel(raw, sentinelDefault)
            if (expected.isEmpty()) {
                assertEquals(
                    "raw ${raw.toCharArray().toList()} must sanitise to empty and select the default",
                    sentinelDefault,
                    got
                )
            } else {
                assertEquals(
                    "raw ${raw.toCharArray().toList()} sanitised to the wrong model ID",
                    expected,
                    got
                )
            }
            assertFalse(
                "raw ${raw.toCharArray().toList()} left a control character in the model ID",
                got.any { it.code < 0x20 || it.code == 0x85 }
            )
        }
    }

    /**
     * The Anthropic entry must keep declaring itself Desktop-only, so a later
     * reader cannot mistake this half's missing assert for an oversight (and
     * cannot quietly add a Kotlin symbol without touching this test).
     */
    @Test
    fun anthropicModelEntryIsDeclaredDesktopOnly() {
        val entry = vector("TWIN-CLEANUP-MODEL-ANTHROPIC-001")
        assertTrue(
            "the Anthropic entry must declare a null kotlin_symbol (Android has no Anthropic provider, drift row H5)",
            entry.isNull("kotlin_symbol")
        )
        assertTrue(
            "the Anthropic entry must say in words that it is Desktop-only",
            entry.getString("description").contains("DESKTOP-ONLY")
        )
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
        // Leading-empty case: the fixture's DOES-NOT-PIN clause names the empty-result skip
        // rule (Kotlin `if (sb.isNotEmpty())`, Rust `!combined_text.is_empty()`), so the case
        // has to exist on both sides rather than only be described (7-8 review round 1).
        // An empty first chunk must NOT produce a leading separator.
        assertEquals(
            "an empty first chunk result must not emit a leading separator",
            "beta",
            KlarvoApi.joinChunkResults(listOf("", "beta"))
        )
    }
}
