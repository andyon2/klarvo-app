package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Story 11-3 (AC-3/Task 4.1/4.4) -- the "Bedienbarkeits-Blocker" core fix.
 *
 * Guards [ListeningPanelView.visibleLines], the pure line-buffer eviction function that bounds
 * the listening panel's rolling-window transcript to [ListeningPanelView.ROLLING_MAX_LINES]
 * lines, regardless of how much text has accumulated (item 3: "fixed box size, not
 * grow-with-text"). This is the pure/testable half of the fix; the other half
 * (`KlarvoOverlayService.showListeningPanel`'s fixed WindowManager height) is not JVM-testable
 * (needs a real WindowManager) and is covered by GATE-4 device smoke instead.
 */
class RollingWindowVisibleLinesTest {

    @Test
    fun fewerChunksThanMax_allVisible_noneFading() {
        val chunks = listOf("eins", "zwei")
        val result = ListeningPanelView.visibleLines(chunks, maxLines = 5)

        assertEquals(listOf("eins", "zwei"), result.map { it.text })
        assertTrue("no chunk should be flagged fading -- nothing has been evicted yet",
            result.none { it.isFading })
    }

    @Test
    fun exactlyMaxChunks_allVisible_noneFading() {
        val chunks = listOf("a", "b", "c")
        val result = ListeningPanelView.visibleLines(chunks, maxLines = 3)

        assertEquals(listOf("a", "b", "c"), result.map { it.text })
        assertTrue("buffer at capacity but nothing evicted yet -- no fade expected",
            result.none { it.isFading })
    }

    /**
     * Core AC-3 assertion: the buffer NEVER exceeds maxLines, however many chunks accumulate --
     * this is the direct fix for the "fills the whole screen" defect this story exists to close.
     */
    @Test
    fun moreChunksThanMax_bufferNeverExceedsMaxLines() {
        val chunks = (1..50).map { "chunk-$it" }
        val result = ListeningPanelView.visibleLines(chunks, maxLines = 5)

        assertTrue(
            "visible line count (${result.size}) must never exceed maxLines (5), no matter " +
                "how many chunks accumulated (${chunks.size}) -- this bound is the direct fix " +
                "for the panel growing to fill the screen",
            result.size <= 5
        )
        assertEquals(5, result.size)
    }

    /**
     * Oldest-first eviction order: once over capacity, the window must contain the MOST RECENT
     * chunks (highest indices), not an arbitrary subset -- the user needs to see what they just
     * said, not stale early content.
     */
    @Test
    fun moreChunksThanMax_keepsMostRecentChunks_oldestEvictedFirst() {
        val chunks = listOf("oldest", "old", "middle", "recent", "newest")
        val result = ListeningPanelView.visibleLines(chunks, maxLines = 3)

        assertEquals(
            "with maxLines=3 over a 5-chunk buffer, only the 3 most recent chunks should " +
                "remain -- the 2 oldest ('oldest', 'old') must be evicted first",
            listOf("middle", "recent", "newest"),
            result.map { it.text }
        )
    }

    /**
     * The oldest currently-visible line is flagged fading exactly when older chunks were
     * evicted to make room for it (AC-3c: soft fade, not an abrupt cut).
     */
    @Test
    fun whenOverCapacity_oldestVisibleLineIsFlaggedFading() {
        val chunks = listOf("evicted", "middle", "recent", "newest")
        val result = ListeningPanelView.visibleLines(chunks, maxLines = 3)

        assertEquals(listOf("middle", "recent", "newest"), result.map { it.text })
        assertTrue("the oldest VISIBLE line ('middle') must be flagged fading once older " +
            "chunks were evicted", result[0].isFading)
        assertTrue("only the oldest visible line fades -- newer lines must not",
            result.drop(1).none { it.isFading })
    }

    @Test
    fun emptyChunks_returnsEmptyList() {
        val result = ListeningPanelView.visibleLines(emptyList(), maxLines = 5)
        assertTrue("no chunks accumulated yet -- nothing should be visible", result.isEmpty())
    }

    /**
     * Inversion (per DoD): an unbounded implementation (returning all chunks, no eviction) must
     * fail the capacity assertion above -- proving the maxLines bound is load-bearing, not
     * accidental. Simulated here directly (not by calling the SUT) so this test does not judge
     * the SUT with itself (feedback_test_must_not_judge_sut_with_itself).
     */
    @Test
    fun inversion_unboundedBuffer_wouldFailTheCapacityBound() {
        val chunks = (1..50).map { "chunk-$it" }
        val unboundedResult = chunks // what an eviction-less implementation would return as-is

        assertTrue(
            "an unbounded buffer keeps growing with every chunk -- exactly the pre-11-3 " +
                "defect this story fixes; it must NOT satisfy a maxLines=5 bound",
            unboundedResult.size > 5
        )
        // The real function, in contrast, always holds the bound:
        val boundedResult = ListeningPanelView.visibleLines(chunks, maxLines = 5)
        assertTrue(boundedResult.size <= 5)
    }
}
