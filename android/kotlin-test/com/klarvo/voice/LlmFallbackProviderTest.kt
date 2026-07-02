package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Unit tests for `KlarvoApi.resolveLlmProvider` / `KlarvoApi.resolveFallbackLlmProvider`
 * (story 12-1, AC2/AC7): Groq must NEVER be a cleanup-fallback candidate on
 * either platform -- it is the STT provider and must not have its quota
 * eaten by cleanup-fallback retries.
 *
 * Two distinct call sites share the same underlying candidate list
 * (`cleanupFallbackCandidates`, private):
 * - `resolveLlmProvider` — config-resolution fallback (the configured
 *   primary provider has no key at all).
 * - `resolveFallbackLlmProvider` — the runtime-failure fallback added by
 *   this story inside `KlarvoOverlayService`'s cleanup `catch (e:
 *   IOException)` block (the "fallback-before-raw-text" sequence): it is
 *   called with the primary's name as `excluding` right before the actual
 *   fallback `cleanupChunked` network call, so asserting its selection here
 *   covers the sequencing logic without needing a live network call.
 *
 * No Android Context is needed -- both functions operate purely on the
 * `KlarvoApi.Config` data class.
 */
class LlmFallbackProviderTest {

    private fun baseConfig(
        llmProvider: String = "deepseek",
        deepseekApiKey: String = "",
        groqApiKey: String = "",
        openaiApiKey: String = "",
        openrouterApiKey: String = ""
    ) = KlarvoApi.Config(
        groqApiKey = groqApiKey,
        deepseekApiKey = deepseekApiKey,
        language = "en",
        cleanupStyle = "verbatim",
        tursoUrl = "",
        tursoToken = "",
        deviceId = "test-device",
        llmProvider = llmProvider,
        openaiApiKey = openaiApiKey,
        openrouterApiKey = openrouterApiKey
    )

    // -----------------------------------------------------------------------
    // resolveLlmProvider: config-resolution fallback (primary has no key)
    // -----------------------------------------------------------------------

    @Test
    fun resolveLlmProvider_primaryHasNoKey_groqKeyAlone_neverSelected() {
        val config = baseConfig(llmProvider = "deepseek", groqApiKey = "gsk-test")
        val result = KlarvoApi.resolveLlmProvider(config)
        assertNull(
            "Groq must never be chosen as a config-resolution fallback, even if it's the only other key",
            result
        )
    }

    @Test
    fun resolveLlmProvider_primaryHasNoKey_skipsGroqPicksOpenai() {
        val config = baseConfig(llmProvider = "deepseek", groqApiKey = "gsk-test", openaiApiKey = "sk-openai")
        val result = KlarvoApi.resolveLlmProvider(config)
        assertNotNull(result)
        assertEquals("gpt-4o-mini", result!!.model)
    }

    @Test
    fun resolveLlmProvider_primaryConfiguredAsGroq_isAllowedAsPrimary() {
        // Groq is only excluded from the *fallback* candidate list -- a user can
        // still explicitly configure it as the primary cleanup provider.
        val config = baseConfig(llmProvider = "groq", groqApiKey = "gsk-test")
        val result = KlarvoApi.resolveLlmProvider(config)
        assertNotNull(result)
        assertEquals("llama-3.3-70b-versatile", result!!.model)
    }

    // -----------------------------------------------------------------------
    // resolveFallbackLlmProvider: runtime-failure fallback (AC2)
    // -----------------------------------------------------------------------

    @Test
    fun resolveFallbackLlmProvider_groqKeyAlone_neverSelected() {
        val config = baseConfig(deepseekApiKey = "ds-key", groqApiKey = "gsk-key")
        val result = KlarvoApi.resolveFallbackLlmProvider(config, excluding = "deepseek")
        assertNull(
            "Groq must never be selected as a runtime cleanup fallback, even if it's the only other key",
            result
        )
    }

    @Test
    fun resolveFallbackLlmProvider_skipsGroqAndFailedPrimary_picksOpenai() {
        val config = baseConfig(
            deepseekApiKey = "ds-key",
            groqApiKey = "gsk-key",
            openaiApiKey = "sk-openai",
            openrouterApiKey = "sk-or"
        )
        val result = KlarvoApi.resolveFallbackLlmProvider(config, excluding = "deepseek")
        assertNotNull(result)
        assertEquals("gpt-4o-mini", result!!.model)
    }

    @Test
    fun resolveFallbackLlmProvider_noKeysAvailable_returnsNull() {
        val config = baseConfig()
        val result = KlarvoApi.resolveFallbackLlmProvider(config, excluding = "deepseek")
        assertNull(result)
    }

    @Test
    fun resolveFallbackLlmProvider_onlyExcludedProviderHasKey_returnsNull() {
        val config = baseConfig(deepseekApiKey = "ds-key")
        val result = KlarvoApi.resolveFallbackLlmProvider(config, excluding = "deepseek")
        assertNull("The provider that just failed must not be re-selected as its own fallback", result)
    }

    // -----------------------------------------------------------------------
    // Finding C (12-1 code review): the fallback call site must exclude the
    // ACTUALLY resolved provider name, not `config.llmProvider` -- when the
    // configured provider has no key, `resolveLlmProvider` silently
    // substitutes one (see resolveLlmProvider's cleanupFallbackCandidates
    // fallthrough), and excluding the never-run configured name lets the
    // runtime fallback re-pick the exact substitute that just failed.
    // -----------------------------------------------------------------------

    @Test
    fun resolveLlmProvider_substitutesDeepseek_exposesResolvedNameForExclusion() {
        // llmProvider = "openrouter" has no key -> resolveLlmProvider falls
        // through to cleanupFallbackCandidates and actually runs DeepSeek.
        val config = baseConfig(llmProvider = "openrouter", deepseekApiKey = "ds-key")
        val primary = KlarvoApi.resolveLlmProvider(config)
        assertNotNull(primary)
        assertEquals(
            "resolveLlmProvider substituted DeepSeek, so providerName must say so (not 'openrouter')",
            "deepseek",
            primary!!.providerName
        )
    }

    @Test
    fun resolveFallbackLlmProvider_excludingResolvedSubstitute_doesNotRetryIt() {
        val config = baseConfig(
            llmProvider = "openrouter",
            deepseekApiKey = "ds-key",
            openaiApiKey = "sk-openai"
        )
        val primary = KlarvoApi.resolveLlmProvider(config)!!

        // Correct call site behavior (post-fix): exclude the resolved substitute.
        val fallback = KlarvoApi.resolveFallbackLlmProvider(config, excluding = primary.providerName)
        assertNotNull("must pick an alternative, not silently return nothing", fallback)
        assertEquals(
            "must move on to the next candidate, not re-run the DeepSeek call that just failed",
            "gpt-4o-mini",
            fallback!!.model
        )

        // Regression pin: excluding the never-run CONFIGURED name (the pre-fix
        // bug) re-selects the same DeepSeek that just failed.
        val buggyFallback = KlarvoApi.resolveFallbackLlmProvider(config, excluding = config.llmProvider)
        assertEquals("deepseek-chat", buggyFallback!!.model)
    }
}
