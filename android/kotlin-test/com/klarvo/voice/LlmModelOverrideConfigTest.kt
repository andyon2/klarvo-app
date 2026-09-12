package com.klarvo.voice

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Story 7-9 — the JSON half of the model-ID override, plus the Kotlin half of
 * AC4 (an old `config.json` must still parse).
 *
 * Same reason and same shape as [MinRecordingMsConfigTest]: real `config.json`
 * text goes through the production `org.json` seam
 * ([KlarvoApi.parseLlmModelOverride]) rather than a hand-built
 * [KlarvoApi.Config] being asserted against itself. A misspelled JSON key here
 * would make the override silently inert while the feature reports green.
 *
 * Expected values are independent test literals, not references to the
 * production `DEFAULT_MODEL_*` constants, except where the assertion is
 * explicitly "the default is used" — there the production symbol IS the claim.
 * The literal↔symbol binding is pinned separately by [TwinConstantsVectorsTest]
 * against `test-fixtures/twin-constants-vectors.json`.
 *
 * What this does NOT exercise: the Rust side (no cross-language assert), the
 * provider-selection logic (that is [LlmFallbackProviderTest]), any network
 * call, `llmModelAnthropic` (Desktop-only — Android has no Anthropic provider,
 * drift row H5), or `readConfig`'s file I/O and license gating.
 */
class LlmModelOverrideConfigTest {

    // -----------------------------------------------------------------------
    // The parse seam
    // -----------------------------------------------------------------------

    @Test
    fun jsonParse_modelOverride_readsNestedAdvancedKey() {
        val json = JSONObject(
            """{"advanced":{"llmModelDeepseek":"deepseek-reasoner","llmModelOpenai":"gpt-4o","llmModelGroq":"llama-3.1-8b-instant"}}"""
        )
        assertEquals("deepseek-reasoner", KlarvoApi.parseLlmModelOverride(json, "llmModelDeepseek"))
        assertEquals("gpt-4o", KlarvoApi.parseLlmModelOverride(json, "llmModelOpenai"))
        assertEquals("llama-3.1-8b-instant", KlarvoApi.parseLlmModelOverride(json, "llmModelGroq"))
    }

    @Test
    fun jsonParse_modelOverride_emptyWhenAdvancedObjectAbsent() {
        val json = JSONObject("{}")
        assertEquals("", KlarvoApi.parseLlmModelOverride(json, "llmModelDeepseek"))
    }

    @Test
    fun jsonParse_modelOverride_emptyWhenKeyAbsentFromAdvanced() {
        val json = JSONObject("""{"advanced":{"minRecordingMs":750}}""")
        assertEquals("", KlarvoApi.parseLlmModelOverride(json, "llmModelDeepseek"))
    }

    /**
     * Review round 1 (P8): a non-string value must NOT be coerced into a model
     * ID. `optString` stringifies whatever it finds, so `"llmModelDeepseek": 42`
     * became the model ID `"42"` and would have gone to the provider.
     *
     * PINS: number, boolean, JSON null, object and array all yield `""`, which
     * [KlarvoApi.effectiveCleanupModel] then reads as "use the default" — so a
     * type-confused config falls back instead of inventing an ID. This is the
     * Kotlin end of a deliberate asymmetry; the Rust twin
     * (`config::tests::spec_non_string_model_override_takes_corrupt_recovery_path`)
     * rejects the whole file instead, which is pre-existing serde behaviour
     * across every `AdvancedSettings` String field (recorded as deferred).
     * DOES NOT PIN: `readConfig`'s file I/O or its license gating (this drives
     * the pure `org.json` seam), or what the provider would do with a bad ID.
     */
    @Test
    fun jsonParse_modelOverride_nonStringValueIsNotCoerced() {
        val cases = listOf(
            """{"advanced":{"llmModelDeepseek":42}}""" to "a JSON number",
            """{"advanced":{"llmModelDeepseek":3.5}}""" to "a JSON float",
            """{"advanced":{"llmModelDeepseek":true}}""" to "a JSON boolean",
            """{"advanced":{"llmModelDeepseek":null}}""" to "a JSON null",
            """{"advanced":{"llmModelDeepseek":{"id":"x"}}}""" to "a JSON object",
            """{"advanced":{"llmModelDeepseek":["x"]}}""" to "a JSON array",
        )
        for ((raw, label) in cases) {
            val parsed = KlarvoApi.parseLlmModelOverride(JSONObject(raw), "llmModelDeepseek")
            assertEquals("$label must not be coerced into a model ID", "", parsed)
            assertEquals(
                "$label must end at the built-in default, not at a stringified value",
                KlarvoApi.DEFAULT_MODEL_DEEPSEEK,
                KlarvoApi.effectiveCleanupModel(parsed, KlarvoApi.DEFAULT_MODEL_DEEPSEEK)
            )
        }
    }

    // -----------------------------------------------------------------------
    // The empty/whitespace predicate (Q5) — twin of Rust
    // `llm::effective_cleanup_model`
    // -----------------------------------------------------------------------

    @Test
    fun effectiveCleanupModel_nonEmptyOverrideWins() {
        assertEquals(
            "deepseek-reasoner",
            KlarvoApi.effectiveCleanupModel("deepseek-reasoner", KlarvoApi.DEFAULT_MODEL_DEEPSEEK)
        )
    }

    @Test
    fun effectiveCleanupModel_blankOverrideUsesDefault() {
        for (blank in listOf("", " ", "   ", "\t", "\n", " \t\n ")) {
            assertEquals(
                "a blank override (${blank.length} chars) must resolve to the default",
                KlarvoApi.DEFAULT_MODEL_DEEPSEEK,
                KlarvoApi.effectiveCleanupModel(blank, KlarvoApi.DEFAULT_MODEL_DEEPSEEK)
            )
        }
    }

    @Test
    fun effectiveCleanupModel_trimsPaddedOverride() {
        assertEquals("gpt-4o", KlarvoApi.effectiveCleanupModel("  gpt-4o  ", KlarvoApi.DEFAULT_MODEL_OPENAI))
        assertEquals("gpt-4o", KlarvoApi.effectiveCleanupModel("\n gpt-4o \t", KlarvoApi.DEFAULT_MODEL_OPENAI))
    }

    // -----------------------------------------------------------------------
    // AC4 (Kotlin half): an old config.json carrying the 13 keys story 7-9
    // removed still parses, and the live values still come out.
    // -----------------------------------------------------------------------

    /**
     * AC4: `config.json` written by a pre-7.9 build carries all 13 removed keys
     * with non-default values. Android only ever READS `config.json`
     * (ADR-0015 single writer), and `org.json` ignores keys nobody asks for —
     * but that is the claim, so it is pinned rather than assumed.
     *
     * The discriminating part is that the LIVE values still parse out of the
     * same object: if the removed keys somehow broke the parse, `readConfig`'s
     * `catch (e: Exception)` would swallow it and return `null`, and the bubble
     * would silently fall back to its defaults.
     */
    @Test
    fun oldConfigJson_withRemovedDeadKeys_stillYieldsLiveValues() {
        val oldConfig = """
            {
              "language": "de",
              "groqApiKey": "gsk-old",
              "bubbleTapAutoSend": true,
              "bubbleLongPressAutoSend": true,
              "advanced": {
                "sttTemperature": 0.42,
                "llmTemperature": 0.77,
                "llmMaxTokens": 1234,
                "chunkThreshold": 1111,
                "chunkTargetSize": 999,
                "autoPaste": false,
                "autoCapitalize": false,
                "llmSystemPromptPolished": "old polished prompt",
                "llmSystemPromptVerbatim": "old verbatim prompt",
                "llmSystemPromptChat": "old chat prompt",
                "llmCommandModePrompt": "old command prompt",
                "minRecordingMs": 750,
                "silenceThreshold": 0.012,
                "llmModelDeepseek": "deepseek-reasoner"
              }
            }
        """.trimIndent()

        val json = JSONObject(oldConfig)

        // Live values survive alongside the removed keys.
        assertEquals(
            "live advanced.minRecordingMs must parse out of an old config",
            750L,
            KlarvoApi.parseMinRecordingMs(json)
        )
        assertEquals(
            "live advanced.llmModelDeepseek must parse out of an old config",
            "deepseek-reasoner",
            KlarvoApi.parseLlmModelOverride(json, "llmModelDeepseek")
        )
        assertEquals(
            "live advanced.silenceThreshold must parse out of an old config",
            0.012,
            json.optJSONObject("advanced")!!.optDouble("silenceThreshold", 0.005),
            1e-9
        )
        assertEquals("live top-level language must parse", "de", json.optString("language", ""))
    }

    /**
     * AC4 + AC5: the old config's non-default DeepSeek override reaches the
     * resolved provider — the end-to-end claim (JSON text → `Config` →
     * `LlmProviderInfo.model`) for the one field this story makes live.
     */
    @Test
    fun oldConfigJson_modelOverride_reachesResolvedProvider() {
        val json = JSONObject(
            """{"advanced":{"llmModelDeepseek":"deepseek-reasoner","autoPaste":false,"sttTemperature":0.42}}"""
        )
        val config = KlarvoApi.Config(
            groqApiKey = "gsk-test",
            deepseekApiKey = "ds-key",
            language = "de",
            cleanupStyle = "verbatim",
            tursoUrl = "",
            tursoToken = "",
            deviceId = "test-device",
            llmProvider = "deepseek",
            llmModelDeepseek = KlarvoApi.parseLlmModelOverride(json, "llmModelDeepseek")
        )
        assertEquals("deepseek-reasoner", KlarvoApi.resolveLlmProvider(config)!!.model)
    }
}
