package com.klarvo.voice

import org.json.JSONArray
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Story 13-2 (D9 / D-M5, D-M6) — the STT sentinel ladder, Kotlin half.
 *
 * ## The rows
 * - **D-M5**: an empty or unparseable STT answer is `SttError::ResponseFormat`,
 *   which `pipeline::is_retryable_stt_error` calls NON-retryable. It had no
 *   sentinel of its own, fell into `stt/groq_jni.rs`'s `__ERROR_NETWORK:`
 *   catch-all, and Android burned three Groq calls and ~7 s of backoff on a
 *   guaranteed repeat that Desktop reports on the first attempt. It now
 *   crosses as `__ERROR_FORMAT:` and is classified NON_RETRYABLE.
 * - **D-M6**: the retry BUDGET. Desktop makes one STT attempt; Android made
 *   three with 2 s + 5 s in between, reaching the same end state up to ~70 s
 *   later. `transcribeWithRetry`'s `retryDelaysMs` is now empty.
 *
 * ## Discipline
 * The sentinel STRINGS are the contract between `stt/groq_jni.rs` and this
 * classifier. They are written here as literals, deliberately: comparing them
 * to another Kotlin constant would agree no matter what either said. The Rust
 * side's literals are in `groq_jni.rs`'s `nativeTranscribe`; the fixture
 * `TEST-STT-EMPTY-001` in `test-provider-scenario-vectors.json` records the
 * pairing, and `sentinelNamesMatchTheFixture` below reads it.
 *
 * ## What this does NOT cover
 * The backoff loop itself (it sleeps on a real thread inside a Service), the
 * local-Whisper safety net, the pending-WAV terminal path, and whether the
 * Rust side really emits these strings — that is `stt/groq_jni.rs`'s own
 * source and is not reachable from a JVM test.
 */
class SttSentinelClassificationTest {

    @Test
    fun aPlainTranscriptIsSuccess() {
        val s = KlarvoOverlayService.classifySttSentinel("Hallo Welt.")
        assertEquals(KlarvoOverlayService.SttVerdict.SUCCESS, s.verdict)
        assertEquals("Hallo Welt.", s.message)
    }

    /**
     * The guard chain returns `""` for a dropped transcript (prompt echo /
     * blocklist), which is NOT an error sentinel — it must reach the caller's
     * blank-transcript branch, not the retry ladder.
     */
    @Test
    fun anEmptyStringIsSuccess_notAnError() {
        assertEquals(
            KlarvoOverlayService.SttVerdict.SUCCESS,
            KlarvoOverlayService.classifySttSentinel("").verdict,
        )
    }

    @Test
    fun emptyAudioIsNonRetryable() {
        val s = KlarvoOverlayService.classifySttSentinel("__ERROR_EMPTY_AUDIO__")
        assertEquals(KlarvoOverlayService.SttVerdict.NON_RETRYABLE, s.verdict)
        assertEquals("Groq STT: empty audio", s.message)
    }

    /**
     * **The row.** `__ERROR_FORMAT:` is the new sentinel and it must not be
     * retried.
     */
    @Test
    fun responseFormatSentinelIsNonRetryable() {
        val s = KlarvoOverlayService.classifySttSentinel(
            "__ERROR_FORMAT:API returned empty text after segment confidence filter__"
        )
        assertEquals(KlarvoOverlayService.SttVerdict.NON_RETRYABLE, s.verdict)
        assertTrue(
            "the cause must survive into the exception message",
            s.message.contains("empty text after segment confidence filter"),
        )
    }

    /**
     * The discriminating half: the SAME failure, carried by the old
     * catch-all, is still retryable. Without this the assertion above would
     * also pass against a classifier that had simply stopped retrying
     * everything — and the ordering claim in `groq_jni.rs` (match
     * `ResponseFormat` BEFORE the catch-all) would be untested from this side.
     */
    @Test
    fun inversion_theSameMessageUnderTheNetworkSentinelIsStillRetryable() {
        val viaCatchAll = KlarvoOverlayService.classifySttSentinel(
            "__ERROR_NETWORK:API returned empty text after segment confidence filter__"
        )
        assertEquals(KlarvoOverlayService.SttVerdict.RETRYABLE, viaCatchAll.verdict)
    }

    @Test
    fun fourXxApiErrorsAreNonRetryable() {
        val s = KlarvoOverlayService.classifySttSentinel("__ERROR_API:HTTP 401: invalid key__")
        assertEquals(KlarvoOverlayService.SttVerdict.NON_RETRYABLE, s.verdict)
        assertTrue(s.message.contains("HTTP 401"))
    }

    @Test
    fun fiveXxAndUnparseableApiErrorsAreRetryable() {
        assertEquals(
            KlarvoOverlayService.SttVerdict.RETRYABLE,
            KlarvoOverlayService.classifySttSentinel("__ERROR_API:HTTP 503: overloaded__").verdict,
        )
        assertEquals(
            KlarvoOverlayService.SttVerdict.RETRYABLE,
            KlarvoOverlayService.classifySttSentinel("__ERROR_API:something odd__").verdict,
        )
    }

    @Test
    fun networkAndUnknownSentinelsAreRetryable() {
        assertEquals(
            KlarvoOverlayService.SttVerdict.RETRYABLE,
            KlarvoOverlayService.classifySttSentinel("__ERROR_NETWORK:connection refused__").verdict,
        )
        assertEquals(
            "an unknown sentinel from a newer core must fail soft, i.e. retry",
            KlarvoOverlayService.SttVerdict.RETRYABLE,
            KlarvoOverlayService.classifySttSentinel("__ERROR_SOMETHING_NEW:x__").verdict,
        )
    }

    /**
     * D-M6: the budget the fixture states. Read out of the production source
     * because the loop itself is a private method on a Service — no JVM test
     * can call it, and a number asserted against nothing would be a wish.
     */
    @Test
    fun retryBudgetIsOneAttempt() {
        val candidates = listOf(
            "../../../../android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt", // gen/android/app/ -> repo root
            "../../../android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt",
            "../../android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt",
            "../android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt",
            "android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt",
        )
        val base = File(System.getProperty("user.dir") ?: ".")
        val src = candidates.map { File(base, it) }.firstOrNull { it.exists() }
            ?.readText()
            ?: error("KlarvoOverlayService.kt not found from ${base.absolutePath}")
        assertTrue(
            "transcribeWithRetry must carry an empty retry-delay list (one attempt, like Desktop)",
            src.contains("val retryDelaysMs = emptyList<Long>()"),
        )
        assertEquals(
            "the fixture must state the same budget",
            1,
            fixtureVector("TEST-STT-EMPTY-001").getJSONObject("kotlin").getInt("retry_attempts"),
        )
    }

    /** The sentinel name is part of the cross-twin contract, so the fixture carries it. */
    @Test
    fun sentinelNamesMatchTheFixture() {
        val kotlin = fixtureVector("TEST-STT-EMPTY-001").getJSONObject("kotlin")
        val sentinel = kotlin.getString("sentinel")
        assertEquals("__ERROR_FORMAT:", sentinel)
        assertEquals(
            kotlin.getString("verdict"),
            KlarvoOverlayService.classifySttSentinel("${sentinel}empty__").verdict.name,
        )
    }

    private fun fixtureVector(id: String): JSONObject {
        val candidates = listOf(
            "../../../../test-fixtures/test-provider-scenario-vectors.json", // gen/android/app/ -> repo root
            "../../../test-fixtures/test-provider-scenario-vectors.json",
            "../../test-fixtures/test-provider-scenario-vectors.json",
            "../test-fixtures/test-provider-scenario-vectors.json",
            "test-fixtures/test-provider-scenario-vectors.json",
        )
        val base = File(System.getProperty("user.dir") ?: ".")
        val file = candidates.map { File(base, it) }.firstOrNull { it.exists() }
            ?: error("test-provider-scenario-vectors.json not found from ${base.absolutePath}")
        val arr = JSONArray(file.readText())
        return (0 until arr.length())
            .map { arr.getJSONObject(it) }
            .firstOrNull { it.getString("id") == id }
            ?: error("fixture has no vector with id=$id")
    }
}
