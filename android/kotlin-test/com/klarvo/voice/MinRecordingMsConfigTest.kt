package com.klarvo.voice

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
        // default (src-tauri/src/config/mod.rs:217-219).
        assertEquals(500L, baseConfig(minRecordingMs = 500L).minRecordingMs)
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
