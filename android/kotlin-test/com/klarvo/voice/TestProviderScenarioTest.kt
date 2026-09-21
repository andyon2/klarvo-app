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
 * Story 13-1/13-1b — test provider, Kotlin half.
 *
 * Reads the SAME file as the Rust halves (`llm::tests::spec_debug_llm_*` in
 * src-tauri/src/llm/mod.rs and `stt::tests::spec_debug_stt_*` in
 * src-tauri/src/stt/mod.rs): `test-fixtures/test-provider-scenario-vectors.json`
 * at the repo root.
 *
 * ## Why this test exists at all
 * The test provider is an ENABLER: story 13-2 needs four audit rows
 * (D2/D-H19 empty answer, D3/D-M16 truncated answer, D9 empty STT result,
 * D10/D-M2 malformed answer) reproducible on a real device. The mechanism only
 * works if the SAME canned wire bytes go through each twin's own mapping — so
 * this half locks two things: the bytes [KlarvoApi.testCannedWire] puts on the
 * wire, and the verdict [KlarvoApi.mapCleanupResponse] returns for them.
 *
 * Three of those verdicts are DELIBERATE DIVERGENCES from the Rust twin and are
 * asserted as such below. They are defects story 13-2 fixes; this story only
 * makes them observable. A future change that "fixes" one here must change the
 * fixture's `expected_divergence` entry and the Rust half in the same commit.
 *
 * ## Discipline
 * Every assertion binds to a PRODUCTION symbol ([KlarvoApi.testCannedWire],
 * [KlarvoApi.mapCleanupResponse], [KlarvoApi.parseTestProvider],
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
 * [KlarvoApi.collectChunkResults]); the COMPOSITION [KlarvoApi.testCleanupOrNull]
 * that [KlarvoApi.cleanup] takes before it builds a URL, including its `null` for
 * every real provider, AND — as a source-text tripwire, because no executing test
 * can see it — the PLACEMENT of that branch above `URL(provider.url)`; the
 * config-parse seam; the reachability of the `"debug"` arm in
 * `resolveLlmProvider`; and its absence from the cleanup fallback ladder.
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
 *   `android.util.Log`, which throws "not mocked" outside Robolectric. Its debug
 *   branch is driven through [KlarvoApi.testCleanupOrNull], which is that branch
 *   minus the one log line; the real HTTP path below it is not exercised here.
 * - `readConfig`'s file I/O. Its license GATE is covered, but through the pure
 *   extraction [KlarvoApi.gateProvidersForLicense], not through `readConfig`
 *   itself: that function reads a file and logs through `android.util.Log`,
 *   which throws "not mocked" outside Robolectric. The gate is why the test
 *   provider only resolves on a licensed/trial device (story 13-4 owns the gate
 *   decision itself; 13-1b only carried its observable effect onto the two new
 *   keys).
 * - `KlarvoOverlayService.isRetryableCleanupFailure` (private) and therefore
 *   whether the fallback ladder really fires — only the message SHAPE its regex
 *   keys on is asserted here.
 * - anything on a real device.
 */
class TestProviderScenarioTest {

    // -----------------------------------------------------------------------
    // Fixture plumbing
    // -----------------------------------------------------------------------

    private fun loadFixture(): Map<String, JSONObject> {
        val cwd = File(System.getProperty("user.dir") ?: ".")
        val name = "test-fixtures/test-provider-scenario-vectors.json"
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
                    val actual = KlarvoApi.testCannedWire(scenario)
                    assertNotNull("$id: $scenario must produce a canned wire", actual)
                    assertEquals("$id: canned status", wire.getInt("status"), actual!!.first)
                    assertEquals("$id: canned body", wire.getString("body"), actual.second)
                }
                "loopback" -> {
                    assertNull(
                        "$id: $scenario must have NO canned wire (it performs a real loopback request)",
                        KlarvoApi.testCannedWire(scenario)
                    )
                    assertEquals(
                        "$id: loopback URL",
                        wire.getString("url"),
                        KlarvoApi.TEST_TRANSPORT_URL
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
            KlarvoApi.testCannedWire("ok"),
            KlarvoApi.testCannedWire("no-such-scenario")
        )
        // `truncated` IS offered on the LLM side, so it must NOT collapse to ok.
        assertTrue(KlarvoApi.testCannedWire("truncated") != KlarvoApi.testCannedWire("ok"))
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
                KlarvoApi.mapCleanupResponse(status, body, KlarvoApi.TEST_MODEL)
            )
            "throws" -> when (val thrown = expected.getString("throws")) {
                "IOException" -> {
                    val e = assertThrows(IOException::class.java) {
                        KlarvoApi.mapCleanupResponse(status, body, KlarvoApi.TEST_MODEL)
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
                    KlarvoApi.mapCleanupResponse(status, body, KlarvoApi.TEST_MODEL)
                }
                else -> error("$id: unknown kotlin.throws $thrown")
            }
            else -> error("$id: unknown kotlin.outcome $outcome")
        }
    }

    @Test
    fun okScenarioReturnsTheCannedAnswer() {
        assertKotlinVerdict("TEST-LLM-OK-001")
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
        assertKotlinVerdict("TEST-LLM-EMPTY-001")
        assertTrue(
            "the fixture must record this as an expected divergence",
            vector("TEST-LLM-EMPTY-001").getString("expected_divergence").contains("D-H19")
        )
    }

    /**
     * DIVERGENCE (drift row D3 / D-M16): Kotlin never inspects `finish_reason`,
     * so a truncated answer is returned as the half sentence. Rust maps it to
     * `LlmError::OutputTruncated`.
     */
    @Test
    fun truncatedAnswerIsReturnedAsThePartialText_divergesFromRust() {
        assertKotlinVerdict("TEST-LLM-TRUNCATED-001")
        assertTrue(
            "the fixture must record this as an expected divergence",
            vector("TEST-LLM-TRUNCATED-001").getString("expected_divergence").contains("D-M16")
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
        assertKotlinVerdict("TEST-LLM-MALFORMED-001")
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
        val v = vector("TEST-LLM-MALFORMED-001")
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
                Callable { KlarvoApi.mapCleanupResponse(status, body, KlarvoApi.TEST_MODEL) }
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
            val rateLimited = KlarvoApi.testCannedWire("http429")!!
            val httpFuture: Future<String> = executor.submit(
                Callable {
                    KlarvoApi.mapCleanupResponse(rateLimited.first, rateLimited.second, KlarvoApi.TEST_MODEL)
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

    /**
     * The COMPOSITION, not the two halves separately: [KlarvoApi.testCleanupOrNull]
     * is the branch [KlarvoApi.cleanup] takes before it builds a URL, and without
     * this test nothing executes it — moving the branch below `URL(provider.url)`
     * would keep every gate green while `URL("")` threw a MalformedURLException
     * (an IOException with no `HTTP nnn`) and the fallback ladder fired into the
     * user's real DeepSeek key.
     *
     * Three cases: the benign scenario, one error scenario, and — the
     * discriminating half — a NON-test provider, which must come back `null` so
     * the real HTTP path below it still runs.
     */
    @Test
    fun testCleanupOrNull_answersForTheTestProviderAndOnlyForIt() {
        val ok = vector("TEST-LLM-OK-001")
        assertEquals(
            "the ok scenario must come back through the composed branch",
            ok.getJSONObject("kotlin").getString("text"),
            KlarvoApi.testCleanupOrNull(testProvider("ok"))
        )

        // One error scenario, through the same composition: a 429 must still
        // arrive as the IOException the retry regex reads.
        val e = assertThrows(IOException::class.java) {
            KlarvoApi.testCleanupOrNull(testProvider("http429"))
        }
        assertTrue(
            "the composed branch must keep the HTTP status: ${e.message}",
            Regex("HTTP 429").containsMatchIn(e.message ?: "")
        )

        // Discriminating half: every real provider must fall through to the HTTP
        // path. Without this, a branch that swallowed all providers would pass.
        for (name in listOf("deepseek", "openai", "groq", "openrouter", "anthropic", "", "Debug")) {
            assertNull(
                "provider '$name' must fall through to the real HTTP path",
                KlarvoApi.testCleanupOrNull(
                    LlmProviderInfo(
                        url = "https://api.deepseek.com/chat/completions",
                        model = KlarvoApi.DEFAULT_MODEL_DEEPSEEK,
                        apiKey = "ds-key",
                        providerName = name,
                        testScenario = "empty"
                    )
                )
            )
        }
    }

    /**
     * SOURCE-TEXT TRIPWIRE over the branch's PLACEMENT, in the
     * [Adr0017BoundaryGuardTest] style — because
     * [testCleanupOrNull_answersForTheTestProviderAndOnlyForIt] cannot see it.
     *
     * That test drives the extracted function directly, so moving its CALL SITE
     * in [KlarvoApi.cleanup] below `val url = URL(provider.url)` would leave
     * every gate green while the test provider's `url` — `""`, as
     * `resolveLlmProvider` builds it — made `URL("")` throw a
     * MalformedURLException. That is an IOException carrying no `HTTP nnn`, so
     * `KlarvoOverlayService.isRetryableCleanupFailure` would read it as retryable
     * and fire the cleanup-fallback ladder into the user's real DeepSeek key —
     * a real network call, real tokens, from a provider chosen precisely to make
     * none.
     *
     * Comment lines are skipped, so the prose around the branch (which names both
     * anchors on purpose) cannot satisfy or defeat the check. Exactly one code
     * occurrence of each anchor is required: zero means renamed, reformatted or
     * deleted, more than one means the ordering claim is ambiguous. Either way it
     * fails loudly with the line numbers it did find.
     */
    @Test
    fun testProviderBranchIsTakenBeforeTheProviderUrlIsBuilt() {
        val src = kotlinSrcFile("KlarvoApi.kt").readText()

        val callSite = Regex("""^\s*testCleanupOrNull\(provider\)\?\.let\s*\{\s*return it\s*}\s*$""")
        val urlBuild = Regex("""^\s*val\s+url\s*=\s*URL\(provider\.url\)\s*$""")

        val callLines = codeLineNumbersMatching(src, callSite)
        val urlLines = codeLineNumbersMatching(src, urlBuild)

        assertEquals(
            "KlarvoApi.kt: expected exactly one `testCleanupOrNull(provider)?.let { return it }` " +
                "statement, found $callLines — renamed, reformatted or duplicated",
            1,
            callLines.size
        )
        assertEquals(
            "KlarvoApi.kt: expected exactly one `val url = URL(provider.url)` statement, " +
                "found $urlLines — renamed, reformatted or duplicated",
            1,
            urlLines.size
        )
        assertTrue(
            "KlarvoApi.kt: the test branch must be taken BEFORE the provider URL is built, " +
                "but testCleanupOrNull is at line ${callLines[0]} and URL(provider.url) at " +
                "line ${urlLines[0]}. The test provider's url is \"\", so URL(\"\") throws a " +
                "MalformedURLException — an IOException with no `HTTP nnn` — and the cleanup " +
                "fallback ladder would fire into the user's real API key.",
            callLines[0] < urlLines[0]
        )
    }

    /**
     * 1-based line numbers of CODE lines matching [pattern]. Lines that are pure
     * comments (`//`, `/*`, `*`, `*/`) are skipped, so documentation naming an
     * anchor can neither satisfy nor defeat a placement assertion.
     */
    private fun codeLineNumbersMatching(src: String, pattern: Regex): List<Int> =
        src.lines().mapIndexedNotNull { i, line ->
            val t = line.trim()
            val isComment = t.startsWith("//") || t.startsWith("/*") || t.startsWith("*")
            if (!isComment && pattern.containsMatchIn(line)) i + 1 else null
        }

    /** Resolves a production Kotlin source file, loudly. Mirrors [Adr0017BoundaryGuardTest]. */
    private fun kotlinSrcFile(name: String): File {
        val cwd = File(System.getProperty("user.dir") ?: ".")
        val rel = "android/kotlin-src/com/klarvo/voice/$name"
        val candidates = listOf(
            cwd.resolve("../../../../$rel"), // gen/android/app/ → repo root
            cwd.resolve(rel),
            cwd.resolve("../$rel"),
            cwd.resolve("../../../$rel"),
        )
        return candidates.firstOrNull { it.canonicalFile.isFile }?.canonicalFile
            ?: error(
                "Cannot find $rel. Tried:" +
                    candidates.joinToString("") { System.lineSeparator() + "  " + it.canonicalPath } +
                    System.lineSeparator() + "CWD=" + cwd.canonicalPath
            )
    }

    /**
     * The test provider as `resolveLlmProvider` builds it: empty url, empty key.
     * The empty url is deliberate — it is what makes the branch's placement in
     * [KlarvoApi.cleanup] load-bearing.
     */
    private fun testProvider(scenario: String) = LlmProviderInfo(
        url = "",
        model = KlarvoApi.TEST_MODEL,
        apiKey = "",
        providerName = KlarvoApi.TEST_PROVIDER_NAME,
        testScenario = scenario
    )

    @Test
    fun http429IsAnIoExceptionCarryingTheStatusTheRetryRegexReads() {
        assertKotlinVerdict("TEST-LLM-HTTP429-001")
    }

    @Test
    fun http5xxIsAnIoExceptionCarryingTheStatusTheRetryRegexReads() {
        assertKotlinVerdict("TEST-LLM-HTTP5XX-001")
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
        val v = vector("TEST-LLM-TRANSPORT-001")
        assertTrue(
            "the fixture must state that the message carries no HTTP status",
            v.getJSONObject("kotlin").getBoolean("message_excludes_http_status")
        )
        assertNull(KlarvoApi.testCannedWire("transport"))

        // Every 200 mapping result is free of the literal the regex keys on …
        val httpStatus = Regex("HTTP \\d{3}")
        val ok = KlarvoApi.testCannedWire("ok")!!
        assertFalse(httpStatus.containsMatchIn(KlarvoApi.mapCleanupResponse(ok.first, ok.second, KlarvoApi.TEST_MODEL)))
        // … and only the non-200 branch introduces it.
        val rateLimited = KlarvoApi.testCannedWire("http429")!!
        val e = assertThrows(IOException::class.java) {
            KlarvoApi.mapCleanupResponse(rateLimited.first, rateLimited.second, KlarvoApi.TEST_MODEL)
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
    fun jsonParse_testProvider_readsNestedAdvancedKeys() {
        val json = JSONObject(
            """{"advanced":{"testProviderLlm":"truncated","testProviderStt":"http429"}}"""
        )
        assertEquals("truncated", KlarvoApi.parseTestProvider(json, "testProviderLlm"))
        assertEquals("http429", KlarvoApi.parseTestProvider(json, "testProviderStt"))
    }

    @Test
    fun jsonParse_testProvider_defaultsToOffWhenAbsentBlankOrWrongType() {
        assertEquals(
            KlarvoApi.TEST_PROVIDER_OFF,
            KlarvoApi.parseTestProvider(JSONObject("{}"), "testProviderLlm")
        )
        assertEquals(
            KlarvoApi.TEST_PROVIDER_OFF,
            KlarvoApi.parseTestProvider(JSONObject("""{"advanced":{"minRecordingMs":750}}"""), "testProviderLlm")
        )
        assertEquals(
            KlarvoApi.TEST_PROVIDER_OFF,
            KlarvoApi.parseTestProvider(JSONObject("""{"advanced":{"testProviderLlm":"   "}}"""), "testProviderLlm")
        )
        // A non-String value must not be coerced (the parseLlmModelOverride P8 rule).
        for (bad in listOf("42", "true", "null", """{"a":1}""", "[1,2]")) {
            assertEquals(
                "a non-string testProviderLlm ($bad) must fall back to the default",
                KlarvoApi.TEST_PROVIDER_OFF,
                KlarvoApi.parseTestProvider(
                    JSONObject("""{"advanced":{"testProviderLlm":$bad}}"""),
                    "testProviderLlm"
                )
            )
        }
    }

    /**
     * Story 13-1b: a non-`off` `advanced.testProviderLlm` must reach the test
     * provider UNCONDITIONALLY — no API key check, and BEFORE `llmProvider` is
     * read at all. A key check would drop through to the fallback ladder and
     * silently turn a test run into a real DeepSeek call.
     *
     * The discriminating half is the same config with the key `off`: it must
     * resolve to the REAL provider, so the assertion above cannot pass with a
     * resolver that always returns the test provider.
     */
    @Test
    fun resolveLlmProvider_testProviderWinsWithoutAnyApiKey() {
        val cfg = baseConfig().copy(
            llmProvider = "deepseek",
            testProviderLlm = "empty"
        )
        val resolved = KlarvoApi.resolveLlmProvider(cfg)
        assertNotNull("the test provider must resolve without a key", resolved)
        assertEquals(KlarvoApi.TEST_PROVIDER_NAME, resolved!!.providerName)
        assertEquals(KlarvoApi.TEST_MODEL, resolved.model)
        assertEquals("the configured scenario must reach the provider", "empty", resolved.testScenario)
        assertEquals(
            "the test provider's url must stay empty -- its emptiness is what makes " +
                "the branch placement in cleanup() load-bearing",
            "",
            resolved.url
        )

        // Even with a DeepSeek key present, the test provider still wins: a
        // silent fall-through would return a usable provider and hide itself.
        assertEquals(
            KlarvoApi.TEST_PROVIDER_NAME,
            KlarvoApi.resolveLlmProvider(cfg.copy(deepseekApiKey = "ds-key"))!!.providerName
        )

        // Discriminating half: `off` resolves to the REAL provider.
        val off = baseConfig().copy(llmProvider = "deepseek", deepseekApiKey = "ds-key")
        assertEquals(KlarvoApi.TEST_PROVIDER_OFF, off.testProviderLlm)
        assertEquals("deepseek", KlarvoApi.resolveLlmProvider(off)!!.providerName)
    }

    /**
     * Story 13-1b, the old-shape row: a config file written by story 13-1 carries
     * `llmProvider = "debug"` and no `testProvider*` keys. It must resolve to a
     * REAL working provider, never a dead one — the one outcome the story
     * forbids. `resolveLlmProvider`'s `else ->` is what does it, which is why
     * that arm is load-bearing and not just a default.
     */
    @Test
    fun resolveLlmProvider_oldShapeDebugNameResolvesToDeepSeekNotADeadProvider() {
        val oldShape = baseConfig().copy(llmProvider = "debug", deepseekApiKey = "ds-key")
        val resolved = KlarvoApi.resolveLlmProvider(oldShape)
        assertNotNull("an old-shape config must still resolve a provider", resolved)
        assertEquals("deepseek", resolved!!.providerName)
        assertTrue(
            "a real provider must carry a real url -- an empty one is the test provider's marker",
            resolved.url.startsWith("https://")
        )
        assertEquals(
            "the old name must not switch the test provider on",
            KlarvoApi.TEST_PROVIDER_OFF,
            oldShape.testProviderLlm
        )

        // …and with no DeepSeek key it falls to the ladder rather than to a
        // provider that cannot work.
        val keyless = baseConfig().copy(llmProvider = "debug", openaiApiKey = "sk-openai")
        assertEquals("openai", KlarvoApi.resolveLlmProvider(keyless)!!.providerName)
    }

    /**
     * Story 13-1b: the ANDROID LICENSE GATE, across the shape change.
     *
     * Story 13-1's gate forced `llmProvider` / `sttProvider` to `"groq"` when
     * unlicensed, which also neutralised the test provider — it was a provider
     * NAME. Now that it is a separate key, leaving it ungated would SILENTLY
     * REMOVE a gate Andi decided keeps. The gate ITSELF is story 13-4's; this
     * test only proves its observable effect survived the reshape.
     *
     * Driven through [KlarvoApi.gateProvidersForLicense], the pure extraction of
     * the decision `readConfig` makes inline — `readConfig` itself does file I/O
     * and logs through `android.util.Log` ("not mocked" outside Robolectric), so
     * it is unreachable from a plain JVM test. Andi cannot un-license his phone
     * either, so this row is machine-verified BY CONSTRUCTION rather than handed
     * to him.
     *
     * Inversion (verified RED at writing time): dropping the `if (licensed)`
     * from either test-provider line in `gateProvidersForLicense` makes the
     * unlicensed half return the stored scenario.
     */
    @Test
    fun licenseGate_forcesBothTestKeysOffWhenUnlicensed() {
        val gated = KlarvoApi.gateProvidersForLicense(
            licensed = false,
            llmProvider = "deepseek",
            sttProvider = "groq",
            testProviderLlm = "empty",
            testProviderStt = "http429"
        )
        assertEquals(KlarvoApi.TEST_PROVIDER_OFF, gated.testProviderLlm)
        assertEquals(KlarvoApi.TEST_PROVIDER_OFF, gated.testProviderStt)
        assertTrue("the [license] log line must still fire", gated.gated)
        // The rest of the gate is unchanged.
        assertEquals("groq", gated.llmProvider)
        assertEquals("groq", gated.sttProvider)

        // Discriminating half: LICENSED leaves both values exactly as stored, so
        // the assertions above cannot pass with a helper that always says `off`.
        val licensed = KlarvoApi.gateProvidersForLicense(
            licensed = true,
            llmProvider = "deepseek",
            sttProvider = "groq",
            testProviderLlm = "empty",
            testProviderStt = "http429"
        )
        assertEquals("empty", licensed.testProviderLlm)
        assertEquals("http429", licensed.testProviderStt)
        assertEquals("deepseek", licensed.llmProvider)
        assertFalse("nothing was gated, so nothing is logged", licensed.gated)
    }

    /**
     * The gate's pre-13-1b behaviour, pinned so the extraction is provably
     * behaviour-PRESERVING rather than merely behaviour-compatible: the STT half
     * keeps its ALLOWLIST shape (anything that is neither `groq` nor `local` is
     * alternative), and an already-free configuration is not logged as gated.
     */
    @Test
    fun licenseGate_keepsItsPreviousDecisionForEveryOtherInput() {
        // `local` is NOT rewritten by this gate (its own gate is deferred).
        val localStt = KlarvoApi.gateProvidersForLicense(
            licensed = false,
            llmProvider = "groq",
            sttProvider = "local",
            testProviderLlm = KlarvoApi.TEST_PROVIDER_OFF,
            testProviderStt = KlarvoApi.TEST_PROVIDER_OFF
        )
        assertEquals("local", localStt.sttProvider)
        assertFalse("an already-free configuration is not gated", localStt.gated)

        // An alternative STT provider IS rewritten, and logged.
        val altStt = KlarvoApi.gateProvidersForLicense(
            licensed = false,
            llmProvider = "groq",
            sttProvider = "openai",
            testProviderLlm = KlarvoApi.TEST_PROVIDER_OFF,
            testProviderStt = KlarvoApi.TEST_PROVIDER_OFF
        )
        assertEquals("groq", altStt.sttProvider)
        assertTrue(altStt.gated)

        // Licensed: nothing is touched at all.
        val free = KlarvoApi.gateProvidersForLicense(
            licensed = true,
            llmProvider = "openrouter",
            sttProvider = "openai",
            testProviderLlm = KlarvoApi.TEST_PROVIDER_OFF,
            testProviderStt = KlarvoApi.TEST_PROVIDER_OFF
        )
        assertEquals("openrouter", free.llmProvider)
        assertEquals("openai", free.sttProvider)
        assertFalse(free.gated)
    }

    /**
     * The test provider must never appear in the cleanup fallback ladder
     * (deepseek → openai → openrouter), so a test 429/5xx fires the PRODUCTION
     * ladder and the test provider can never rescue itself.
     */
    @Test
    fun testProviderIsNeverACleanupFallbackCandidate() {
        val cfg = baseConfig().copy(
            llmProvider = "deepseek",
            testProviderLlm = "http429",
            deepseekApiKey = "ds-key",
            openaiApiKey = "sk-openai",
            openrouterApiKey = "sk-or"
        )
        for (excluding in listOf("test", "debug", "deepseek", "openai", "openrouter", "")) {
            val fallback = KlarvoApi.resolveFallbackLlmProvider(cfg, excluding)
            if (fallback != null) {
                assertFalse(
                    "excluding=$excluding: the test provider must never be the selected fallback",
                    fallback.providerName == KlarvoApi.TEST_PROVIDER_NAME
                )
            }
        }
        assertEquals(
            "the production ladder starts at DeepSeek",
            "deepseek",
            KlarvoApi.resolveFallbackLlmProvider(cfg, KlarvoApi.TEST_PROVIDER_NAME)!!.providerName
        )

        // Discriminating half: with NO real key at all there is no candidate —
        // a test-provider entry in the ladder would make this non-null.
        val keyless = baseConfig().copy(testProviderLlm = "http429")
        assertNull(
            "the test provider must not be able to nominate itself",
            KlarvoApi.resolveFallbackLlmProvider(keyless, KlarvoApi.TEST_PROVIDER_NAME)
        )
    }

    /**
     * Minimal [KlarvoApi.Config]: only the seven positional fields the data
     * class requires. Everything else keeps its declared default, including the
     * two story-13-1b test-provider fields appended last (both `off`).
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
