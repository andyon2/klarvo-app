package com.klarvo.voice

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * M2 — Story 7-2, AC5: `minRecordingMs` must be config-driven, not the old hardcoded `500L`
 * literal previously passed to `GroqSttBridge.nativeSilenceCheck`
 * (`KlarvoOverlayService.kt:1847-1854`, pre-Story-7-2).
 *
 * `nativeSilenceCheck` itself is an external JNI function and cannot run in a JVM unit test, so
 * this is a config round-trip test of the resolution seam
 * [KlarvoOverlayService.resolveMinRecordingMsForSilenceFilter] -- the pure function extracted
 * from the call site specifically so this is testable (mirrors
 * [RecordingModeSilenceSelectionTest]'s pattern: an independent expected-value table, the
 * production SUT is not compared against itself).
 */
class MinRecordingMsConfigTest {

    private fun baseConfig(minRecordingMs: Long) = KlarvoApi.Config(
        groqApiKey = "gsk-test",
        deepseekApiKey = "",
        language = "en",
        cleanupStyle = "verbatim",
        tursoUrl = "",
        tursoToken = "",
        deviceId = "test-device",
        minRecordingMs = minRecordingMs
    )

    @Test
    fun defaultConfig_minRecordingMs_is500_matchesRustDefault() {
        // Independent expected value: 500L, from Rust AdvancedSettings::min_recording_ms
        // default (src-tauri/src/config/mod.rs:217-219). Constructed WITHOUT an explicit
        // minRecordingMs argument (unlike the old version of this test, which passed 500L in
        // and asserted 500L back out -- a tautology that would still pass if the declared
        // default became 0L) so the assertion exercises Config's actual default value.
        val config = KlarvoApi.Config(
            groqApiKey = "gsk-test",
            deepseekApiKey = "",
            language = "en",
            cleanupStyle = "verbatim",
            tursoUrl = "",
            tursoToken = "",
            deviceId = "test-device"
        )
        assertEquals(500L, config.minRecordingMs)
    }

    /**
     * AC5's real JSON-parse path (Story 7-2 review finding): [KlarvoApi.parseMinRecordingMs]
     * previously had no test driving actual `config.json` text through `org.json.JSONObject` --
     * a wrong/misspelled JSON key would have made the setting silently inert with the feature
     * reporting green. Feeds a real config string through the production parser.
     */
    @Test
    fun jsonParse_minRecordingMs_nonDefault_parsesFromConfigJsonString_ac5() {
        val json = JSONObject("""{"advanced":{"minRecordingMs":750}}""")
        assertEquals(750L, KlarvoApi.parseMinRecordingMs(json))
    }

    @Test
    fun jsonParse_minRecordingMs_default_whenAdvancedKeyAbsent_ac5() {
        val json = JSONObject("{}")
        assertEquals(500L, KlarvoApi.parseMinRecordingMs(json))
    }

    @Test
    fun nonDefaultMinRecordingMs_inConfig_changesResolvedValue_ac5() {
        // AC5's exact regression check: a non-default minRecordingMs (750L, deliberately
        // distinct from the 500L default) must reach the resolved value passed to
        // nativeSilenceCheck.
        val config = baseConfig(minRecordingMs = 750L)
        val resolved = KlarvoOverlayService.resolveMinRecordingMsForSilenceFilter(config)
        assertEquals(
            "AC5: a non-default minRecordingMs (750L) in config must change the value resolved " +
                "for the nativeSilenceCheck call -- got $resolved. If this fails, the call site " +
                "is still ignoring config (the old hardcoded-500L regression).",
            750L,
            resolved
        )
    }

    @Test
    fun nullConfig_resolvesToNullSafeDefault_500_ac5() {
        // Null-safe fallback: if config is somehow absent, the resolved value must still be
        // 500L (the same value the old hardcoded literal used) -- not a behavior change.
        val resolved = KlarvoOverlayService.resolveMinRecordingMsForSilenceFilter(null)
        assertEquals(
            "AC5: a null config must fall back to 500L (matching the old hardcoded literal / " +
                "KlarvoApi.Config's own default), not crash or resolve to 0.",
            500L,
            resolved
        )
    }

    /**
     * Inversion: proves [KlarvoOverlayService.resolveMinRecordingMsForSilenceFilter] is not
     * secretly hardcoded to 500L regardless of input -- distinct config values must resolve to
     * distinct results.
     */
    @Test
    fun inversion_distinctConfigValues_resolveToDistinctResults() {
        val resolvedDefault = KlarvoOverlayService.resolveMinRecordingMsForSilenceFilter(baseConfig(500L))
        val resolvedTuned = KlarvoOverlayService.resolveMinRecordingMsForSilenceFilter(baseConfig(750L))
        assert(resolvedDefault != resolvedTuned) {
            "resolveMinRecordingMsForSilenceFilter must not be hardcoded -- 500L and 750L " +
                "configs resolved to the same value ($resolvedDefault)."
        }
    }
}
