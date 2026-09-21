package com.klarvo.voice

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Story 13-2 (E2 / G2a) — the ONE offline rule, Kotlin half.
 *
 * Reads the SAME file as the Rust half
 * (`pipeline::tests::spec_offline_rule_matches_the_fixture_matrix`):
 * `test-fixtures/offline-rule-vectors.json` at the repo root.
 *
 * ## Why this exists
 * "Offline" meant three different things. The desktop hotkey read
 * `stt == "local" && llm != "local"`; `commands::recording::cleanup_text` read
 * `stt == "local"` alone (drift row D-M20, and it runs on Android too, inside
 * `TauriActivity`); Kotlin's overlay path read `llmProvider == "local"` alone
 * (D-H10), so after a LOCAL transcript the cloud cleanup arm still ran for any
 * cloud provider. And on any platform without a local LLM, `"local"` fell
 * through to `_ => deepseek` — a setting whose promise is on-device making a
 * cloud call (D-M21). Grundsatzurteil G2a: no byte leaves the device.
 *
 * ## Discipline
 * Every assertion drives the PRODUCTION predicate
 * ([KlarvoOverlayService.skipsCloudCleanup]) against the FIXTURE literal, never
 * against another production symbol. `org.json`'s throwing accessors are used
 * so a typo'd or missing key fails loudly instead of passing vacuously.
 *
 * ## What this does NOT cover
 * - that `processAudio` really calls the predicate at the right place (a
 *   `processAudio` branch — on-device smoke territory, like the rest of
 *   [CleanupFailureDeliveryTest]'s "does not cover" list);
 * - the desktop in-app-button path, which is Rust;
 * - whether a local cleanup provider *should* exist on Android — G3b says not
 *   in this epic, and [KlarvoOverlayService.LOCAL_CLEANUP_AVAILABLE] records
 *   that as a constant rather than a hope.
 */
class OfflineRuleVectorsTest {

    private fun loadFixture(): List<JSONObject> {
        val candidates = listOf(
            "../../../../test-fixtures/offline-rule-vectors.json", // gen/android/app/ -> repo root
            "../../../test-fixtures/offline-rule-vectors.json",
            "../../test-fixtures/offline-rule-vectors.json",
            "../test-fixtures/offline-rule-vectors.json",
            "test-fixtures/offline-rule-vectors.json",
        )
        val base = File(System.getProperty("user.dir") ?: ".")
        val file = candidates
            .map { File(base, it) }
            .firstOrNull { it.exists() }
            ?: error(
                "offline-rule-vectors.json not found from ${base.absolutePath}; tried $candidates"
            )
        val arr = JSONArray(file.readText())
        return (0 until arr.length()).map { arr.getJSONObject(it) }
    }

    /** The whole 2x2x2 matrix, driven through the production predicate. */
    @Test
    fun offlineRuleMatchesTheFixtureMatrix() {
        var checked = 0
        for (v in loadFixture()) {
            if (!v.has("expected_skips_cleanup")) continue // the platform-constants row
            val id = v.getString("id")
            val actual = KlarvoOverlayService.skipsCloudCleanup(
                sttProvider = v.getString("stt_provider"),
                llmProvider = v.getString("llm_provider"),
                localCleanupAvailable = v.getBoolean("local_cleanup_available"),
            )
            assertEquals(
                "$id: stt=${v.getString("stt_provider")} llm=${v.getString("llm_provider")} " +
                    "localAvailable=${v.getBoolean("local_cleanup_available")}",
                v.getBoolean("expected_skips_cleanup"),
                actual,
            )
            checked++
        }
        assertEquals("the 2x2x2 matrix must be complete", 8, checked)
    }

    /**
     * The platform constant the matrix is parameterised on. Without this the
     * rows above would be a hypothesis rather than a statement about this app.
     */
    @Test
    fun androidReportsNoLocalCleanupProvider() {
        val v = loadFixture().first { it.getString("id") == "OFFLINE-PLATFORM-AVAILABILITY-001" }
        assertEquals(
            v.getJSONObject("platform_constants").getBoolean("android_kotlin"),
            KlarvoOverlayService.LOCAL_CLEANUP_AVAILABLE,
        )
    }

    /**
     * D-M21 on this platform, spelled out: with [KlarvoOverlayService.LOCAL_CLEANUP_AVAILABLE]
     * false, a stored `llmProvider = "local"` means NO cleanup — whatever the
     * STT provider is. 13-3 hides the control; the stored value survives it
     * (ADR-0016's gate definition), which is why this is not 13-3's problem.
     */
    @Test
    fun storedLocalCleanupNeverBecomesACloudCallOnAndroid() {
        for (stt in listOf("groq", "openai", "local", "")) {
            assertTrue(
                "stt=$stt: a local cleanup Android does not have must skip cleanup",
                KlarvoOverlayService.skipsCloudCleanup(
                    stt, "local", KlarvoOverlayService.LOCAL_CLEANUP_AVAILABLE
                ),
            )
        }
    }

    /**
     * Inversion: the availability parameter is load-bearing. Flip it and the
     * same config resolves the other way — if these ever agree, the predicate
     * has stopped reading platform availability and D-M21 is back.
     */
    @Test
    fun inversion_platformAvailabilityIsLoadBearing() {
        val withLocal = KlarvoOverlayService.skipsCloudCleanup("groq", "local", true)
        val withoutLocal = KlarvoOverlayService.skipsCloudCleanup("groq", "local", false)
        assertFalse("with a local provider, cleanup runs on-device", withLocal)
        assertTrue("without one, there is no cleanup", withoutLocal)
    }

    /**
     * The other inversion: `sttProvider` is load-bearing too. The pre-13-2
     * Kotlin branch read `llmProvider` alone, so this pair agreed — that WAS
     * drift row D-H10.
     */
    @Test
    fun inversion_sttProviderIsLoadBearing() {
        val cloudStt = KlarvoOverlayService.skipsCloudCleanup("groq", "deepseek", false)
        val localStt = KlarvoOverlayService.skipsCloudCleanup("local", "deepseek", false)
        assertFalse(cloudStt)
        assertTrue("local STT must not be followed by a cloud cleanup call", localStt)
    }

    /**
     * The test provider must stay reachable (13-1b): the predicate is fed the
     * EFFECTIVE provider name, so a run with `advanced.testProviderLlm` set is
     * not classified offline by a `llmProvider` that never runs.
     */
    @Test
    fun activeTestProviderIsTheEffectiveProvider() {
        val config = config(llmProvider = "local", testProviderLlm = "empty")
        assertEquals(KlarvoApi.TEST_PROVIDER_NAME, KlarvoApi.effectiveLlmProviderName(config))
        assertFalse(
            "an active test provider must still be called",
            KlarvoOverlayService.skipsCloudCleanup(
                config.sttProvider,
                KlarvoApi.effectiveLlmProviderName(config),
                KlarvoOverlayService.LOCAL_CLEANUP_AVAILABLE,
            ),
        )
        val off = config(llmProvider = "local")
        assertEquals("local", KlarvoApi.effectiveLlmProviderName(off))
    }

    /** The seven positional fields [KlarvoApi.Config] has no default for. */
    private fun config(
        llmProvider: String = "deepseek",
        sttProvider: String = "groq",
        testProviderLlm: String = KlarvoApi.TEST_PROVIDER_OFF,
    ) = KlarvoApi.Config(
        groqApiKey = "gsk-test",
        deepseekApiKey = "sk-test",
        language = "de",
        cleanupStyle = "polished",
        tursoUrl = "",
        tursoToken = "",
        deviceId = "test-device",
        llmProvider = llmProvider,
        sttProvider = sttProvider,
        testProviderLlm = testProviderLlm,
    )
}
