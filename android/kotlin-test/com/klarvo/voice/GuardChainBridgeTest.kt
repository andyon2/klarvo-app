package com.klarvo.voice

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Story 13-2 (B2 / D-H5, D-H6, D-M9 and B3 / D-H7) — Kotlin half of
 * `test-fixtures/guard-chain-vectors.json`.
 *
 * ## What this half can and cannot assert
 * The guard VERDICTS are shared Rust core (ADR-0017): Android reaches them
 * through `GroqSttBridge.nativeTranscribe` →
 * `stt::groq_jni::guard_transcript_for_jni` → `pipeline::guard_transcript`.
 * There is no Kotlin implementation to compare against, and writing one would
 * be forbidden. The fixture's `kotlin` column therefore says "n/a" for those
 * vectors, exactly as the `surface: "stt"` vectors in
 * `test-provider-scenario-vectors.json` do, and the Rust reader
 * (`pipeline::tests::spec_guard_*`) owns the verdicts.
 *
 * What this half DOES assert is the thing a Rust test cannot see: **that
 * Android delegates**. Three source-text tripwires over the production Kotlin —
 * the post-cleanup ghost strip goes through the bridge and not through a
 * Kotlin re-implementation; the JNI gets the conditioning hint and not the LLM
 * cleanup instruction; and the one vector that IS a twin
 * (`STT-HINT-SELECT-*`, config plumbing rather than STT logic) is driven
 * through the real [KlarvoApi.selectSttHintOverride].
 *
 * A source-text tripwire is a weak instrument and is used deliberately, for the
 * same reason `TestProviderScenarioTest` uses one for the ordering of
 * `testCleanupOrNull` above `URL(provider.url)`: no executing JVM test can
 * reach a JNI call site. `Adr0017BoundaryGuardTest` is the other half of the
 * net and would NOT catch a ghost-strip twin written under a new name — which
 * is exactly why this file exists.
 */
class GuardChainBridgeTest {

    private val srcDir: File by lazy {
        val candidates = listOf(
            "../../../../android/kotlin-src/com/klarvo/voice", // gen/android/app/ -> repo root
            "../../../android/kotlin-src/com/klarvo/voice",
            "../../android/kotlin-src/com/klarvo/voice",
            "../android/kotlin-src/com/klarvo/voice",
            "android/kotlin-src/com/klarvo/voice",
        )
        val base = File(System.getProperty("user.dir") ?: ".")
        candidates.map { File(base, it) }.firstOrNull { it.isDirectory }
            ?: error("kotlin-src not found from ${base.absolutePath}; tried $candidates")
    }

    private fun source(name: String): String =
        File(srcDir, name).also { assertTrue("$name must exist", it.exists()) }.readText()

    private fun loadFixture(): List<JSONObject> {
        val candidates = listOf(
            "../../../../test-fixtures/guard-chain-vectors.json", // gen/android/app/ -> repo root
            "../../../test-fixtures/guard-chain-vectors.json",
            "../../test-fixtures/guard-chain-vectors.json",
            "../test-fixtures/guard-chain-vectors.json",
            "test-fixtures/guard-chain-vectors.json",
        )
        val base = File(System.getProperty("user.dir") ?: ".")
        val file = candidates.map { File(base, it) }.firstOrNull { it.exists() }
            ?: error("guard-chain-vectors.json not found from ${base.absolutePath}")
        val arr = JSONArray(file.readText())
        return (0 until arr.length()).map { arr.getJSONObject(it) }
    }

    private fun vector(id: String): JSONObject =
        loadFixture().firstOrNull { it.getString("id") == id }
            ?: error("guard-chain-vectors.json has no vector with id=$id")

    // -----------------------------------------------------------------------
    // The fixture declares its own Kotlin coverage
    // -----------------------------------------------------------------------

    /**
     * Every guard vector must SAY it is n/a on Android and why — that is what
     * turns "skipped" into "deliberately not applicable" rather than
     * "forgotten" (the `TwinConstantsVectorsTest` skip-by-id precedent).
     */
    @Test
    fun guardVectorsDeclareThemselvesNotApplicableOnAndroid() {
        var checked = 0
        for (v in loadFixture()) {
            val id = v.getString("id")
            val kotlin = v.getJSONObject("kotlin")
            if (kotlin.getString("outcome") == "same") continue // the twin vector, driven below
            assertEquals("$id: STT guards have no Kotlin twin", "n/a", kotlin.getString("outcome"))
            assertTrue(
                "$id: must say WHY it is n/a",
                kotlin.getString("note").contains("ADR-0017"),
            )
            checked++
        }
        assertEquals("the fixture must carry nine shared-core guard vectors", 9, checked)
    }

    // -----------------------------------------------------------------------
    // B3 second half: the post-cleanup ghost strip goes through the bridge
    // -----------------------------------------------------------------------

    /**
     * The delegation, asserted at the call site. `processAudio` must run the
     * cleaned text through `GroqSttBridge.nativeStripStockphraseGhosts` before
     * anything is delivered — Desktop has always called the same Rust function
     * after `sanitize_llm_output`, Android had no post-cleanup strip at all.
     */
    @Test
    fun postCleanupGhostStripGoesThroughTheJniBridge() {
        val src = source("KlarvoOverlayService.kt")
        assertTrue(
            "processAudio must call GroqSttBridge.nativeStripStockphraseGhosts after cleanup",
            src.contains("GroqSttBridge.nativeStripStockphraseGhosts("),
        )
        assertTrue(
            "the bridge must declare the extern",
            source("GroqSttBridge.kt").contains("external fun nativeStripStockphraseGhosts"),
        )
    }

    /**
     * And there must be no Kotlin twin of it. ADR-0017 forbids a Kotlin
     * re-implementation of an STT guard; `Adr0017BoundaryGuardTest` matches
     * four fixed shapes and would miss a ghost strip written under any other
     * name, so the blocklist literals themselves are the tripwire here.
     *
     * `Klinge`/`Kleinschreibung` are the audit's own example ghosts and the
     * entries a hand-rolled Kotlin strip would have to spell out.
     */
    @Test
    fun noKotlinGhostStripTwinExists() {
        for (file in srcDir.listFiles { f: File -> f.name.endsWith(".kt") }.orEmpty()) {
            val stripped = stripComments(file.readText())
            for (needle in listOf("Kleinschreibung", "kleinschreibung", "STOCKPHRASE")) {
                assertFalse(
                    "${file.name} must not carry a Kotlin stockphrase list ($needle) — " +
                        "the ghost strip is shared Rust core (ADR-0017)",
                    stripped.contains(needle),
                )
            }
        }
    }

    // -----------------------------------------------------------------------
    // B4: the JNI gets the conditioning hint, not the cleanup instruction
    // -----------------------------------------------------------------------

    /**
     * Drift row D-H4 in one assertion: `config.customPrompt` must not be what
     * `transcribeWithRetry` sends as the Whisper prompt. It is the LLM cleanup
     * instruction and belongs to the LLM call alone; sending it replaced
     * Whisper's language hint AND became the input of both post-STT guards.
     */
    @Test
    fun theJniGetsTheSttHintAndNotTheCleanupInstruction() {
        val src = stripComments(source("KlarvoOverlayService.kt"))

        // EVERY call site, not "at least one": the dictation path and the
        // live-preview path both reach the JNI, and an inversion that reverted
        // only one of them passed a `contains` check (measured 2026-09-21 —
        // the first version of this assertion stayed GREEN).
        val callSites = Regex("""(?<!fun )transcribeWithRetry\(""").findAll(src).count()
        val hintArgs = Regex("""sttHintFor\(config\)""").findAll(src).count()
        assertTrue("transcribeWithRetry must have call sites at all", callSites >= 2)
        assertEquals(
            "every transcribeWithRetry call site must pass the conditioning hint",
            callSites,
            hintArgs,
        )
        assertTrue(
            "the hint helper must read the advanced.sttPrompt* fields via the twin selector",
            src.contains("KlarvoApi.selectSttHintOverride("),
        )

        // `config.customPrompt` may appear ONLY as the LLM's customInstructions.
        // Anywhere else it is on its way to the STT request, which is D-H4.
        val strayCustomPrompt = src.lines()
            .filter { it.contains("config.customPrompt") && !it.contains("customInstructions") }
        assertTrue(
            "config.customPrompt must reach the LLM and nothing else; stray uses: $strayCustomPrompt",
            strayCustomPrompt.isEmpty(),
        )
        // …and it must still reach the LLM, or the fix would have removed a
        // shipped feature instead of re-routing it.
        assertTrue(
            "customPrompt must still be passed to cleanup as customInstructions",
            src.contains("customInstructions = config.customPrompt"),
        )
    }

    /**
     * The one vector with a real Kotlin reader: the override SELECTION is
     * config plumbing (which string to hand the JNI), so it is a twin rather
     * than shared core — and the fall-through it pins is the half a twin
     * written from the obvious reading gets wrong.
     */
    @Test
    fun sttHintOverrideSelectionMatchesTheRustTwin() {
        val v = vector("STT-HINT-SELECT-DE-FALLTHROUGH-001")
        val input = v.getJSONObject("input")
        val selected = KlarvoApi.selectSttHintOverride(
            input.getString("language"),
            input.getString("stt_prompt_de"),
            input.getString("stt_prompt_en"),
            input.getString("stt_prompt_auto"),
        )
        assertEquals(v.getJSONObject("kotlin").getString("selected"), selected)
        assertEquals(
            "both twins must be pinned to the same literal",
            v.getJSONObject("rust").getString("selected"),
            v.getJSONObject("kotlin").getString("selected"),
        )
    }

    /** The ordinary arms, so the fall-through above is not the only case pinned. */
    @Test
    fun sttHintOverrideSelectionCoversTheOrdinaryArms() {
        assertEquals("DE", KlarvoApi.selectSttHintOverride("de", "DE", "EN", "AUTO"))
        assertEquals("EN", KlarvoApi.selectSttHintOverride("en", "DE", "EN", "AUTO"))
        assertEquals("AUTO", KlarvoApi.selectSttHintOverride("", "DE", "EN", "AUTO"))
        assertEquals(
            "with nothing configured the caller passes \"\" and Rust supplies the built-in hint",
            "",
            KlarvoApi.selectSttHintOverride("de", "", "", ""),
        )
    }

    /**
     * Inversion: the language argument is load-bearing. A selector that
     * ignored it would return the same string for both, and Whisper would be
     * conditioned in the wrong language.
     */
    @Test
    fun inversion_languageIsLoadBearing() {
        assertNotNull(KlarvoApi.selectSttHintOverride("de", "DE", "EN", ""))
        assertTrue(
            KlarvoApi.selectSttHintOverride("de", "DE", "EN", "") !=
                KlarvoApi.selectSttHintOverride("en", "DE", "EN", ""),
        )
    }

    /** Minimal, string-aware comment stripper (the `Adr0017BoundaryGuardTest` approach). */
    private fun stripComments(src: String): String {
        val out = StringBuilder()
        var i = 0
        var inString = false
        var inLine = false
        var inBlock = false
        while (i < src.length) {
            val c = src[i]
            val next = if (i + 1 < src.length) src[i + 1] else '\u0000'
            when {
                inLine -> if (c == '\n') { inLine = false; out.append(c) }
                inBlock -> if (c == '*' && next == '/') { inBlock = false; i++ }
                inString -> {
                    out.append(c)
                    if (c == '\\') { if (i + 1 < src.length) out.append(next); i++ }
                    else if (c == '"') inString = false
                }
                c == '/' && next == '/' -> { inLine = true; i++ }
                c == '/' && next == '*' -> { inBlock = true; i++ }
                c == '"' -> { inString = true; out.append(c) }
                else -> out.append(c)
            }
            i++
        }
        return out.toString()
    }
}
