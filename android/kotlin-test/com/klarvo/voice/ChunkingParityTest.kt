package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.IOException
import java.util.concurrent.Callable
import java.util.concurrent.Executors
import java.util.concurrent.Future

/**
 * Cross-platform chunking parity tests for Epic 7 / Story 7.1.
 *
 * Locks Android `KlarvoApi` against the Desktop (Rust) `chunked_cleanup` /
 * `split_into_chunks` reference in `src-tauri/src/llm/mod.rs` on four drift points:
 *
 * - **H2** — UTF-8 byte-length indices (not UTF-16 char count)
 * - **H13** — chunk results joined with `\n` (not `\n\n`)
 * - **L4** — threshold is `< CHUNK_THRESHOLD` over **byte** length
 * - **M8** — abort on first chunk error, surfacing the original `IOException`
 *
 * **Every test here calls the production function that owns the behavior it claims to
 * lock** — `splitIntoChunks`, `shouldChunk`, `joinChunkResults`, `collectChunkResults`.
 * The first version of this suite joined/thresholded *inside the test* and therefore
 * stayed green against the unfixed code (code review 2026-08-10); H13/L4/M8 now hang on
 * the real seams instead. Each test below is red against the pre-fix implementation.
 */
class ChunkingParityTest {

    /** Compares content independently of where the splitter dropped separator whitespace. */
    private fun squeeze(s: String): String = s.filterNot { it.isWhitespace() }

    // =========================================================================
    // H2: UTF-8 byte-length chunk indices (not UTF-16 char count)
    // =========================================================================

    /**
     * "ä" is 1 char / 2 bytes. A byte-indexed splitter cuts at byte 350 = 175 chars;
     * the pre-fix char-indexed one cut at char 350 = byte 700.
     */
    @Test
    fun h2_splitUsesByteOffsets_notUtf16CharCount() {
        val text = "ä".repeat(500) // 500 chars, 1000 bytes

        val chunks = KlarvoApi.splitIntoChunks(text)

        assertTrue("500 ä (1000 bytes) must split", chunks.size >= 2)
        assertEquals(
            "first chunk must end at byte 350 = 175 ä (Rust split_into_chunks), not at char 350",
            "ä".repeat(175),
            chunks[0]
        )
    }

    /**
     * Rust `test_split_fallback_is_char_boundary_safe` port. The leading ASCII byte shifts
     * every 2-byte 'ü' to an odd byte offset, so the fallback split (start + 350) lands
     * INSIDE a 'ü'. The UTF-8 boundary floor must walk back — otherwise `decodeToString`
     * silently substitutes U+FFFD and destroys the character (it does not throw).
     */
    @Test
    fun h2_fallbackFloorsToCharBoundary_noReplacementChars() {
        val text = "a" + "ü".repeat(300) // 601 bytes, no ". " / "\n" anywhere

        val chunks = KlarvoApi.splitIntoChunks(text)

        assertTrue("601 bytes must split", chunks.size >= 2)
        chunks.forEachIndexed { i, chunk ->
            assertFalse(
                "chunk $i was cut mid-codepoint (contains U+FFFD): '$chunk'",
                chunk.contains('�')
            )
        }
        assertEquals(
            "no character may be lost or replaced across the split",
            squeeze(text),
            squeeze(chunks.joinToString(""))
        )
    }

    /**
     * Realistic German dictation: umlaut-dense, no sentence boundary in the search window,
     * so every split takes the fallback path and the ASCII-whitespace skip runs between chunks.
     */
    @Test
    fun h2_umlautDictation_survivesSplitIntact() {
        val text = "Straße über Männer schön kräftig grün Träume ".repeat(16).trim() // ~830 bytes

        val chunks = KlarvoApi.splitIntoChunks(text)

        assertTrue("~830 bytes must split", chunks.size >= 2)
        chunks.forEachIndexed { i, chunk ->
            assertFalse("chunk $i contains U+FFFD: '$chunk'", chunk.contains('�'))
            assertFalse("chunk $i is empty", chunk.isEmpty())
            assertEquals("chunk $i is not trimmed", chunk.trim(), chunk)
        }
        assertEquals(
            "no character may be lost across the split",
            squeeze(text),
            squeeze(chunks.joinToString(" "))
        )
    }

    // =========================================================================
    // L4 + H2: the threshold decides on UTF-8 byte length, strict less-than
    // =========================================================================

    /** Rust: `if raw_text.len() < CHUNK_THRESHOLD` → 400 is NOT below 400, so 400 chunks. */
    @Test
    fun l4_thresholdIsStrictLessThan() {
        assertFalse("399 bytes is below the threshold → single call", KlarvoApi.shouldChunk("a".repeat(399)))
        assertTrue("400 bytes is NOT below the threshold → chunked", KlarvoApi.shouldChunk("a".repeat(400)))
    }

    /**
     * The quantity, not just the operator: Rust compares `raw_text.len()` (UTF-8 bytes).
     * 200 "ä" are 200 UTF-16 chars but exactly 400 bytes — Desktop chunks it, so Android must too.
     */
    @Test
    fun l4_thresholdCountsUtf8Bytes_notUtf16Chars() {
        val text = "ä".repeat(200)
        assertEquals("fixture must be 200 UTF-16 chars", 200, text.length)
        assertEquals("fixture must be 400 UTF-8 bytes", 400, text.encodeToByteArray().size)

        assertTrue(
            "umlaut text at 400 bytes must take the chunked path like Desktop",
            KlarvoApi.shouldChunk(text)
        )
    }

    // =========================================================================
    // H13: results joined with a single \n, Rust's empty-accumulator guard kept
    // =========================================================================

    @Test
    fun h13_resultsJoinedWithSingleNewline() {
        assertEquals(
            "Rust pushes exactly one '\\n' between chunk results",
            "erster\nzweiter\ndritter",
            KlarvoApi.joinChunkResults(listOf("erster", "zweiter", "dritter"))
        )
    }

    /** Rust guards with `i > 0 && !combined_text.is_empty()` — an empty first result adds no line. */
    @Test
    fun h13_emptyLeadingResultProducesNoBlankLine() {
        assertEquals(
            "an empty first chunk must not produce a leading blank line",
            "Zweiter Satz.",
            KlarvoApi.joinChunkResults(listOf("", "Zweiter Satz."))
        )
    }

    // =========================================================================
    // M8: abort on first chunk error — with the exception type the caller gates on
    // =========================================================================

    /**
     * `Future.get()` wraps the worker's exception in an ExecutionException. KlarvoOverlayService
     * gates its cross-provider cleanup fallback (Epic 12) on `e is IOException`, so the cause
     * must be unwrapped — otherwise a 429 on one chunk silently disables the fallback ladder.
     */
    @Test
    fun m8_chunkFailureAbortsWithTheOriginalIOException() {
        val executor = Executors.newFixedThreadPool(2)
        try {
            val boom = IOException("Cleanup failed: HTTP 429 rate limited")
            val futures: List<Future<String>> = listOf(
                executor.submit(Callable { "erster Chunk" }),
                executor.submit(Callable<String> { throw boom })
            )

            val thrown: Throwable? = try {
                KlarvoApi.collectChunkResults(futures)
                null
            } catch (e: Throwable) {
                e
            }

            assertNotNull("a failing chunk must abort collection, not be swallowed", thrown)
            assertTrue(
                "caller gates the provider fallback on `e is IOException`, got ${thrown!!.javaClass.name}",
                thrown is IOException
            )
            assertEquals("the original failure message must survive", boom.message, thrown.message)
        } finally {
            executor.shutdownNow()
        }
    }

    @Test
    fun m8_allChunksSucceed_resultsKeepChunkOrder() {
        val executor = Executors.newFixedThreadPool(3)
        try {
            val futures: List<Future<String>> = listOf("eins", "zwei", "drei").map { value ->
                executor.submit(Callable { value })
            }

            assertEquals(listOf("eins", "zwei", "drei"), KlarvoApi.collectChunkResults(futures))
        } finally {
            executor.shutdownNow()
        }
    }

    // =========================================================================
    // Shared invariant: no trivial chunk reaches the LLM (history id=3041)
    // =========================================================================

    @Test
    fun shared_noTrivialChunkReachesLLM() {
        val testCases = listOf(
            "Hello world. This is a test of the chunking system. " +
                "More text here to ensure we exceed the target size. Final sentence. ",
            "Müller fährt über die Straße. Kräftiger Wind weht. " +
                "Die Straße ist schön und lang. Noch mehr Text zum Testen. Schluss. ",
            "The quick brown fox jumps over the lazy dog. " +
                "Pack my box with five dozen liquor jugs. " +
                "How vexingly quick daft zebras jump. "
        ).map { it.repeat(8) } // long enough that the split path actually runs

        for ((idx, text) in testCases.withIndex()) {
            val chunks = KlarvoApi.splitIntoChunks(text)
            assertTrue("[$idx] fixture must actually split", chunks.size >= 2)
            for ((i, chunk) in chunks.withIndex()) {
                assertFalse(
                    "[$idx] chunk $i is trivial (would draw an LLM refusal): '$chunk'",
                    KlarvoApi.isTrivialChunk(chunk)
                )
            }
        }
    }
}
