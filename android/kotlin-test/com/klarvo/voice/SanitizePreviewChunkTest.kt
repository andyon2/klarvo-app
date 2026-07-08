package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Code-review fix P1 (11-3): guards [KlarvoOverlayService.sanitizePreviewChunk], the pure helper
 * `appendPreviewText` uses to clean an incoming preview STT chunk before accumulating it.
 *
 * The accumulated preview text is newline-joined per chunk (`KlarvoOverlayService.
 * appendPreviewText`) -- so `"\n"` must stay the ONLY inter-chunk separator. A chunk with an
 * embedded newline must collapse to a single line, and a blank/whitespace chunk must be dropped
 * entirely rather than wasting a transcript line on an empty entry.
 */
class SanitizePreviewChunkTest {

    /**
     * Core P1 assertion: an embedded `\n` inside a single STT chunk must not be miscounted as
     * two rolling-window lines.
     */
    @Test
    fun embeddedNewline_collapsesToSingleChunk() {
        val result = KlarvoOverlayService.sanitizePreviewChunk("hello\nworld")

        assertEquals("hello world", result)
        assertEquals(
            "the sanitized chunk must contain no newline -- otherwise splitting the " +
                "accumulated text on \"\\n\" would recover two lines from one chunk",
            0,
            result?.count { it == '\n' } ?: 0
        )
    }

    @Test
    fun multipleEmbeddedNewlines_collapseToSingleSpace() {
        val result = KlarvoOverlayService.sanitizePreviewChunk("a\n\n\nb")
        assertEquals("a b", result)
    }

    @Test
    fun blankChunk_isIgnored() {
        assertNull(KlarvoOverlayService.sanitizePreviewChunk(""))
        assertNull(KlarvoOverlayService.sanitizePreviewChunk("   "))
        assertNull(KlarvoOverlayService.sanitizePreviewChunk("\n\n"))
    }

    @Test
    fun ordinaryChunk_trimmedButOtherwiseUnchanged() {
        assertEquals("hello world", KlarvoOverlayService.sanitizePreviewChunk("  hello world  "))
    }

    /**
     * Inversion (per DoD): a sanitize implementation that passes the chunk through unchanged
     * (the pre-fix behavior) would fail both assertions above -- it would keep the embedded
     * newline (so the split-based rolling window miscounts it as two lines) and would not drop
     * blank chunks (wasting a rolling-window slot).
     */
    @Test
    fun inversion_passthroughWouldFailBothGuards() {
        val passthrough = { text: String -> text } // stand-in for "no sanitization" (pre-fix)

        val embeddedNewlineChunk = "hello\nworld"
        assertEquals(
            "a passthrough implementation keeps the embedded newline -- proving the collapse " +
                "in sanitizePreviewChunk is load-bearing",
            true,
            passthrough(embeddedNewlineChunk).contains("\n")
        )
        assert(KlarvoOverlayService.sanitizePreviewChunk(embeddedNewlineChunk)?.contains("\n") != true)

        val blankChunk = "   "
        assertEquals(
            "a passthrough implementation does not drop blank chunks -- proving the blank " +
                "check in sanitizePreviewChunk is load-bearing",
            blankChunk,
            passthrough(blankChunk)
        )
        assertNull(KlarvoOverlayService.sanitizePreviewChunk(blankChunk))
    }
}
