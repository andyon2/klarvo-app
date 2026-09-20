package com.klarvo.voice

import org.json.JSONArray
import org.json.JSONException
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File
import java.io.IOException
import java.util.concurrent.Callable
import java.util.concurrent.Executors
import java.util.concurrent.Future

/**
 * Story 13-1 — debug test provider, Kotlin half.
 *
 * Reads the SAME file as the Rust halves (`llm::tests::spec_debug_llm_*` in
 * src-tauri/src/llm/mod.rs and `stt::tests::spec_debug_stt_*` in
 * src-tauri/src/stt/mod.rs): `test-fixtures/debug-provider-scenario-vectors.json`
 * at the repo root.
 *
 * ## Why this test exists at all
 * The debug provider is an ENABLER: story 13-2 needs four audit rows
 * (D2/D-H19 empty answer, D3/D-M16 truncated answer, D9 empty STT result,
 * D10/D-M2 malformed answer) reproducible on a real device. The mechanism only
 * works if the SAME canned wire bytes go through each twin's own mapping — so
 * this half locks two things: the bytes [KlarvoApi.debugCannedWire] puts on the
 * wire, and the verdict [KlarvoApi.mapCleanupResponse] returns for them.
 *
 * Three of those verdicts are DELIBERATE DIVERGENCES from the Rust twin and are
 * asserted as such below. They are defects story 13-2 fixes; this story only
 * makes them observable. A future change that "fixes" one here must change the
 * fixture's `expected_divergence` entry and the Rust half in the same commit.
 *
 * ## Discipline
 * Every assertion binds to a PRODUCTION symbol ([KlarvoApi.debugCannedWire],
 * [KlarvoApi.mapCleanupResponse], [KlarvoApi.parseDebugScenario],
 * [KlarvoApi.resolveLlmProvider], [KlarvoApi.resolveFallbackLlmProvider]) and
 * compares it to the FIXTURE literal — never to another production symbol,
 * which would agree no matter what both said. Uses `org.json`'s THROWING
 * accessors so a typo'd or missing fixture key fails loudly instead of passing
 * vacuously (the 7-2 round-3 R3-P2 defect).
 *
 * ## What this covers — and what it does NOT
 * Covers: the seven `surface: "llm"` vectors, on the JVM, through the Kotlin
 * production seams named above; BOTH Android halves of drift row D10 / D-M2
 * (the silent single-call path and the rewrapped chunked path, through the real
 * [KlarvoApi.collectChunkResults]); the config-parse seam; the reachability of
 * the `"debug"` arm in `resolveLlmProvider`; and its absence from the cleanup
 * fallback ladder.
 *
 * Does NOT cover:
 * - the two `surface: "provider-options"` vectors. They pin the option lists of
 *   the two provider rows in the React settings against the Rust config
 *   allowlists; there is no Kotlin option list (the React bundle is the only
 *   settings surface on Android), so their readers are a Rust test and the
 *   throwaway desktop proxy harness.
 * - the seven `surface: "stt"` vectors. They are n/a on Android BY
 *   CONSTRUCTION, not by oversight: ADR-0017 makes the STT request and its
 *   guards shared Rust core, consumed over `GroqSttBridge.nativeTranscribe`, so
 *   there is no Kotlin STT mapping to assert against. They are skipped by
 *   `surface`, and [sttVectorsDeclareThemselvesNotApplicableOnAndroid] proves
 *   each one says so rather than being silently dropped.
 * - the `transport` scenario's actual request. [KlarvoApi.cleanup]'s debug
 *   branch performs a real `HttpURLConnection` call to the loopback discard
 *   port; only the "there is no canned wire for it" half is checked here.
 * - the Rust side (each half reads this fixture; neither can prove the other).
 * - [KlarvoApi.cleanup] itself end to end: it logs through `KlarvoLogger` →
 *   `android.util.Log`, which throws "not mocked" outside Robolectric. The two
 *   seams the debug branch composes are driven directly instead.
 * - `readConfig`'s file I/O and its license gate. The gate is why the debug
 *   provider only resolves on a licensed/trial device (story 13-4 owns it).
 * - `KlarvoOverlayService.isRetryableCleanupFailure` (private) and therefore
 *   whether the fallback ladder really fires — only the message SHAPE its regex
 *   keys on is asserted here.
 * - anything on a real device.
 */
class DebugProviderScenarioTest {

    // -----------------------------------------------------------------------
    // Fixture plumbing
    // -----------------------------------------------------------------------

    private fun loadFixture(): Map<String, JSONObject> {
        val cwd = File(System.getProperty("user.dir") ?: ".")
        val name = "test-fixtures/debug-provider-scenario-vectors.json"
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

    private fun llmVectors(): List<JSONObject> =
        loadFixture().values.filter { it.getString("surface") == "llm" }

    // -----------------------------------------------------------------------
    // (a) the canned wire bytes — the shared contract with the Rust twin
    // -----------------------------------------------------------------------

    /**
     * The bytes are the whole point: if the two twins canned different bodies, the
     * "same wire, two mappings" premise collapses and the reproduction proves
     * nothing about the drift.
     */
    @Test
    fun cannedWireBytesMatchTheFixtureForEveryLlmScenario() {
        var checked = 0
        for (v in llmVectors()) {
            val id = v.getString("id")
            val scenario = v.getString("scenario")
            val wire = v.getJSONObject("wire")
            when (val kind = wire.getString("kind")) {
                "canned" -> {
                    val actual = KlarvoApi.debugCannedWire(scenario)
                    assertNotNull("$id: $scenario must produce a canned wire", actual)
                    assertEquals("$id: canned status", wire.getInt("status"), actual!!.first)
                    assertEquals("$id: canned body", wire.getString("body"), actual.second)
                }
                "loopback" -> {
                    assertNull(
                        "$id: $scenario must have NO canned wire (it performs a real loopback request)",
                        KlarvoApi.debugCannedWire(scenario)
                    )
                    assertEquals(
                        "$id: loopback URL",
                        wire.getString("url"),
                        KlarvoApi.DEBUG_TRANSPORT_URL
                    )
                }
                else -> error("$id: unknown wire.kind $kind")
            }
            checked++
        }
        assertEquals("all seven LLM scenarios must be covered", 7, checked)
    }

    /**
     * An unrecognised scenario must resolve like `ok` (fail-soft), matching the
     * Rust twin's `_ =>` arm — a stored value from a newer build must never
     * produce an invented outcome.
     */
    @Test
    fun unknownScenarioFallsBackToOk() {
        assertEquals(
            KlarvoApi.debugCannedWire("ok"),
            KlarvoApi.debugCannedWire("no-such-scenario")
        )
        // `truncated` IS offered on the LLM side, so it must NOT collapse to ok.
        assertTrue(KlarvoApi.debugCannedWire("truncated") != KlarvoApi.debugCannedWire("ok"))
    }

    // -----------------------------------------------------------------------
    // (b) the Kotlin mapping's verdict — including the three divergences
    // -----------------------------------------------------------------------

    /** Drives the real mapping for one canned vector and asserts the fixture's Kotlin column. */
    private fun assertKotlinVerdict(id: String) {
        val v = vector(id)
        assertEquals("$id is not an llm vector", "llm", v.getString("surface"))
        val wire = v.getJSONObject("wire")
        if (wire.getString("kind") != "canned") {
            error("$id: only canned vectors have a drivable Kotlin verdict")
        }
        val status = wire.getInt("status")
        val body = wire.getString("body")
        val expected = v.getJSONObject("kotlin")

        when (val outcome = expected.getString("outcome")) {
            "ok" -> assertEquals(
                "$id: Kotlin mapping result",
                expected.getString("text"),
                KlarvoApi.mapCleanupResponse(status, body, KlarvoApi.DEBUG_MODEL)
            )
            "throws" -> when (val thrown = expected.getString("throws")) {
                "IOException" -> {
                    val e = assertThrows(IOException::class.java) {
                        KlarvoApi.mapCleanupResponse(status, body, KlarvoApi.DEBUG_MODEL)
                    }
                    val needle = expected.optString("message_contains", "")
                    if (needle.isNotEmpty()) {
                        assertTrue(
                            "$id: message ${e.message} must contain $needle",
                            e.message!!.contains(needle)
                        )
                    }
                }
                "JSONException" -> assertThrows(JSONException::class.java) {
                    KlarvoApi.mapCleanupResponse(status, body, KlarvoApi.DEBUG_MODEL)
                }
                else -> error("$id: unknown kotlin.throws $thrown")
            }
            else -> error("$id: unknown kotlin.outcome $outcome")
        }
    }

    @Test
    fun okScenarioReturnsTheCannedAnswer() {
        assertKotlinVerdict("DEBUG-LLM-OK-001")
    }

    /**
     * DIVERGENCE (drift row D2 / D-H19), asserted on purpose: Rust rejects an
     * empty answer with `LlmError::ResponseFormat`; Kotlin returns `""`, which
     * today is pasted into the focused field and stored as the dictation. Story
     * 13-2 closes this; until then the fixture records it and this test keeps it
     * visible instead of letting it be "fixed" by accident.
     */
    @Test
    fun emptyAnswerIsReturnedAsAnEmptyString_divergesFromRust() {
        assertKotlinVerdict("DEBUG-LLM-EMPTY-001")
        assertTrue(
            "the fixture must record this as an expected divergence",
            vector("DEBUG-LLM-EMPTY-001").getString("expected_divergence").contains("D-H19")
        )
    }

    /**
     * DIVERGENCE (drift row D3 / D-M16): Kotlin never inspects `finish_reason`,
     * so a truncated answer is returned as the half sentence. Rust maps it to
     * `LlmError::OutputTruncated`.
     */
    @Test
    fun truncatedAnswerIsReturnedAsThePartialText_divergesFromRust() {
        assertKotlinVerdict("DEBUG-LLM-TRUNCATED-001")
        assertTrue(
            "the fixture must record this as an expected divergence",
            vector("DEBUG-LLM-TRUNCATED-001").getString("expected_divergence").contains("D-M16")
        )
    }

    /**
     * DIVERGENCE (drift row D10 / D-M2, Android half), SINGLE-CALL path: the
     * canned `malformed` body does not parse at all, so `JSONObject(body)`
     * throws a bare [JSONException], which is NOT an [IOException] — and
     * `KlarvoOverlayService` gates its cleanup-fallback ladder on
     * `e is IOException` first. So on a short dictation (< CHUNK_THRESHOLD)
     * the ladder never runs and the dictation degrades straight to
     * clipboard-only, while the Rust twin fires its ladder for the same bytes.
     */
    @Test
    fun malformedAnswerThrowsAJsonException_notAnIoException() {
        assertKotlinVerdict("DEBUG-LLM-MALFORMED-001")
        // The discriminating half: it really is NOT an IOException. Without this
        // the assertion above would also pass if JSONException were one.
        assertFalse(
            "a JSONException must not be an IOException, or the ladder would fire and D-M2 would not reproduce",
            IOException::class.java.isAssignableFrom(JSONException::class.java)
        )
    }

    /**
     * The OTHER half of D10 / D-M2 on Android, pinned because the two halves
     * disagree and a reproduction that only knew one of them would mislead:
     * on the CHUNKED path (text >= [KlarvoApi.CHUNK_THRESHOLD] UTF-8 bytes) the
     * same [JSONException] passes through [KlarvoApi.collectChunkResults], which
     * rewraps every non-IOException cause into an [IOException] whose message is
     * the cause's own message — carrying no `HTTP <code>`. So
     * `isRetryableCleanupFailure`'s regex finds no status, returns "retryable",
     * and the ladder DOES fire there.
     *
     * Drives the real production seam with a real [java.util.concurrent.Future]
     * rather than asserting the rewrap from the outside.
     */
    @Test
    fun malformedAnswerOnTheChunkedPathIsRewrappedAsAStatuslessIoException() {
        val v = vector("DEBUG-LLM-MALFORMED-001")
        val chunked = v.getJSONObject("kotlin").getJSONObject("chunked_path")
        assertEquals(
            "the fixture must state the chunked verdict",
            "IOException",
            chunked.getString("throws")
        )
        assertTrue(
            "the fixture must state that the rewrapped message carries no HTTP status",
            chunked.getBoolean("message_excludes_http_status")
        )

        val wire = v.getJSONObject("wire")
        val status = wire.getInt("status")
        val body = wire.getString("body")

        val executor = Executors.newSingleThreadExecutor()
        try {
            val future: Future<String> = executor.submit(
                Callable { KlarvoApi.mapCleanupResponse(status, body, KlarvoApi.DEBUG_MODEL) }
            )
            val rewrapped = assertThrows(IOException::class.java) {
                KlarvoApi.collectChunkResults(listOf(future))
            }
            assertFalse(
                "the rewrap must produce a plain IOException, not a JSONException",
                rewrapped is JSONException
            )
            assertFalse(
                "the rewrapped message ${rewrapped.message} must carry no HTTP status, or the ladder would stop firing",
                Regex("HTTP \\d{3}").containsMatchIn(rewrapped.message ?: "")
            )

            // Discriminating half: an HTTP failure DOES keep its status through
            // the same seam, so the assertion above is not vacuously true of
            // everything collectChunkResults touches.
            val rateLimited = KlarvoApi.debugCannedWire("http429")!!
            val httpFuture: Future<String> = executor.submit(
                Callable {
                    KlarvoApi.mapCleanupResponse(rateLimited.first, rateLimited.second, KlarvoApi.DEBUG_MODEL)
                }
            )
            val kept = assertThrows(IOException::class.java) {
                KlarvoApi.collectChunkResults(listOf(httpFuture))
            }
            assertTrue(
                "an HTTP failure must keep its status through collectChunkResults",
                Regex("HTTP \\d{3}").containsMatchIn(kept.message ?: "")
            )
        } finally {
            executor.shutdown()
        }
    }

    @Test
    fun http429IsAnIoExceptionCarryingTheStatusTheRetryRegexReads() {
        assertKotlinVerdict("DEBUG-LLM-HTTP429-001")
    }

    @Test
    fun http5xxIsAnIoExceptionCarryingTheStatusTheRetryRegexReads() {
        assertKotlinVerdict("DEBUG-LLM-HTTP5XX-001")
    }

    /**
     * The `transport` scenario's shape claim: whatever it throws must carry NO
     * `HTTP <code>`, or `isRetryableCleanupFailure`'s regex would read a status
     * and stop treating it as retryable. The request itself is not made here
     * (see the class KDoc); what is checked is that the ONLY thing that puts
     * that literal into a message is the non-200 branch of the mapping.
     */
    @Test
    fun transportScenarioHasNoCannedWireAndNoStatusInItsMessage() {
        val v = vector("DEBUG-LLM-TRANSPORT-001")
        assertTrue(
            "the fixture must state that the message carries no HTTP status",
            v.getJSONObject("kotlin").getBoolean("message_excludes_http_status")
        )
        assertNull(KlarvoApi.debugCannedWire("transport"))

        // Every 200 mapping result is free of the literal the regex keys on …
        val httpStatus = Regex("HTTP \\d{3}")
        val ok = KlarvoApi.debugCannedWire("ok")!!
        assertFalse(httpStatus.containsMatchIn(KlarvoApi.mapCleanupResponse(ok.first, ok.second, KlarvoApi.DEBUG_MODEL)))
        // … and only the non-200 branch introduces it.
        val rateLimited = KlarvoApi.debugCannedWire("http429")!!
        val e = assertThrows(IOException::class.java) {
            KlarvoApi.mapCleanupResponse(rateLimited.first, rateLimited.second, KlarvoApi.DEBUG_MODEL)
        }
        assertTrue(httpStatus.containsMatchIn(e.message!!))
    }

    // -----------------------------------------------------------------------
    // The STT half is n/a on Android — stated, not silently skipped
    // -----------------------------------------------------------------------

    /**
     * ADR-0017 keeps the STT request and its guards in the shared Rust core, so
     * there is no Kotlin mapping for the STT vectors to assert against. This
     * test proves each one SAYS so, which is what turns "skipped" into
     * "deliberately not applicable" (the `TwinConstantsVectorsTest`
     * skip-by-id precedent).
     */
    @Test
    fun sttVectorsDeclareThemselvesNotApplicableOnAndroid() {
        val stt = loadFixture().values.filter { it.getString("surface") == "stt" }
        assertEquals("the fixture must carry seven STT vectors", 7, stt.size)
        for (v in stt) {
            val id = v.getString("id")
            val kotlin = v.getJSONObject("kotlin")
            assertEquals("$id: STT has no Kotlin twin", "n/a", kotlin.getString("outcome"))
            assertTrue(
                "$id: must say WHY it is n/a",
                kotlin.getString("note").contains("ADR-0017")
            )
        }
    }

    // -----------------------------------------------------------------------
    // Config parse + reachability
    // -----------------------------------------------------------------------

    /**
     * The JSON half: a real `config.json` string goes through the production
     * `org.json` seam. A misspelled key here would make the whole debug
     * mechanism silently inert while the UI reported green.
     */
    @Test
    fun jsonParse_debugScenario_readsNestedAdvancedKeys() {
        val json = JSONObject(
            """{"advanced":{"debugLlmScenario":"truncated","debugSttScenario":"http429"}}"""
        )
        assertEquals("truncated", KlarvoApi.parseDebugScenario(json, "debugLlmScenario"))
        assertEquals("http429", KlarvoApi.parseDebugScenario(json, "debugSttScenario"))
    }

    @Test
    fun jsonParse_debugScenario_defaultsToOkWhenAbsentBlankOrWrongType() {
        assertEquals(
            KlarvoApi.DEBUG_SCENARIO_DEFAULT,
            KlarvoApi.parseDebugScenario(JSONObject("{}"), "debugLlmScenario")
        )
        assertEquals(
            KlarvoApi.DEBUG_SCENARIO_DEFAULT,
            KlarvoApi.parseDebugScenario(JSONObject("""{"advanced":{"minRecordingMs":750}}"""), "debugLlmScenario")
        )
        assertEquals(
            KlarvoApi.DEBUG_SCENARIO_DEFAULT,
            KlarvoApi.parseDebugScenario(JSONObject("""{"advanced":{"debugLlmScenario":"   "}}"""), "debugLlmScenario")
        )
        // A non-String value must not be coerced (the parseLlmModelOverride P8 rule).
        for (bad in listOf("42", "true", "null", """{"a":1}""", "[1,2]")) {
            assertEquals(
                "a non-string debugLlmScenario ($bad) must fall back to the default",
                KlarvoApi.DEBUG_SCENARIO_DEFAULT,
                KlarvoApi.parseDebugScenario(
                    JSONObject("""{"advanced":{"debugLlmScenario":$bad}}"""),
                    "debugLlmScenario"
                )
            )
        }
    }

    /**
     * `llmProvider = "debug"` must reach the debug provider UNCONDITIONALLY —
     * no API key check. Without the explicit arm, `resolveLlmProvider`'s
     * `else ->` maps it to DeepSeek, which is the silent substitution this
     * story exists to avoid.
     */
    @Test
    fun resolveLlmProvider_debugArmIsReachableWithoutAnyApiKey() {
        val cfg = baseConfig().copy(
            llmProvider = KlarvoApi.DEBUG_PROVIDER_NAME,
            debugLlmScenario = "empty"
        )
        val resolved = KlarvoApi.resolveLlmProvider(cfg)
        assertNotNull("the debug provider must resolve without a key", resolved)
        assertEquals(KlarvoApi.DEBUG_PROVIDER_NAME, resolved!!.providerName)
        assertEquals(KlarvoApi.DEBUG_MODEL, resolved.model)
        assertEquals("the configured scenario must reach the provider", "empty", resolved.debugScenario)

        // Discriminating half: with a DeepSeek key present, a silent fall-through
        // to the `else ->` arm would still return a usable provider and hide
        // itself — so assert the name is NOT deepseek.
        val withKey = cfg.copy(deepseekApiKey = "ds-key")
        assertEquals(
            KlarvoApi.DEBUG_PROVIDER_NAME,
            KlarvoApi.resolveLlmProvider(withKey)!!.providerName
        )
    }

    /**
     * The debug provider must never appear in the cleanup fallback ladder
     * (deepseek → openai → openrouter), so a debug 429/5xx fires the PRODUCTION
     * ladder and the debug provider can never rescue itself.
     */
    @Test
    fun debugIsNeverACleanupFallbackCandidate() {
        val cfg = baseConfig().copy(
            llmProvider = KlarvoApi.DEBUG_PROVIDER_NAME,
            deepseekApiKey = "ds-key",
            openaiApiKey = "sk-openai",
            openrouterApiKey = "sk-or"
        )
        for (excluding in listOf("debug", "deepseek", "openai", "openrouter", "")) {
            val fallback = KlarvoApi.resolveFallbackLlmProvider(cfg, excluding)
            if (fallback != null) {
                assertFalse(
                    "excluding=$excluding: debug must never be the selected fallback",
                    fallback.providerName == KlarvoApi.DEBUG_PROVIDER_NAME
                )
            }
        }
        assertEquals(
            "the production ladder starts at DeepSeek",
            "deepseek",
            KlarvoApi.resolveFallbackLlmProvider(cfg, "debug")!!.providerName
        )

        // Discriminating half: with NO real key at all there is no candidate —
        // a `debug` entry in the ladder would make this non-null.
        val debugOnly = baseConfig().copy(llmProvider = KlarvoApi.DEBUG_PROVIDER_NAME)
        assertNull(
            "the debug provider must not be able to nominate itself",
            KlarvoApi.resolveFallbackLlmProvider(debugOnly, "debug")
        )
    }

    /**
     * Minimal [KlarvoApi.Config]: only the seven positional fields the data
     * class requires. Everything else keeps its declared default, including the
     * two story-13-1 fields appended last.
     */
    private fun baseConfig() = KlarvoApi.Config(
        groqApiKey = "",
        deepseekApiKey = "",
        language = "de",
        cleanupStyle = "polished",
        tursoUrl = "",
        tursoToken = "",
        deviceId = ""
    )
}
