package com.klarvo.voice

import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Story 11-3 (AC-3 pivot, 2026-07-08, Task 8.3/8.5) -- guards
 * [ListeningPanelView.isScrolledToBottom], the pure decision behind the transcript's
 * "stick to newest text unless the user scrolled away" behavior (AC-3c/AC-3d). This is the
 * pure/testable half of the fix; the ScrollView's actual scroll/fling behavior is not
 * JVM-testable (needs a real View/WindowManager) and is covered by GATE-4 device smoke instead.
 */
class IsScrolledToBottomTest {

    private val threshold = 24

    @Test
    fun contentShorterThanViewport_alwaysAtBottom() {
        // Nothing to scroll -- trivially "at the bottom" (no rolled-off text possible).
        assertTrue(ListeningPanelView.isScrolledToBottom(scrollY = 0, viewportHeight = 200, contentHeight = 150, thresholdPx = threshold))
    }

    @Test
    fun scrolledExactlyToBottom_isAtBottom() {
        // viewport bottom edge (scrollY + viewportHeight) exactly meets content bottom edge
        assertTrue(ListeningPanelView.isScrolledToBottom(scrollY = 300, viewportHeight = 200, contentHeight = 500, thresholdPx = threshold))
    }

    @Test
    fun withinThresholdOfBottom_stillCountsAsAtBottom() {
        // 10px short of the exact bottom, threshold is 24px -- still "at the bottom"
        assertTrue(ListeningPanelView.isScrolledToBottom(scrollY = 290, viewportHeight = 200, contentHeight = 500, thresholdPx = threshold))
    }

    /**
     * Core AC-3d assertion: a user who has deliberately scrolled up past the threshold must NOT
     * be reported as "at the bottom" -- this is what stops the auto-scroll from yanking them back
     * down to the newest text against their will.
     */
    @Test
    fun scrolledFarUp_isNotAtBottom() {
        assertFalse(ListeningPanelView.isScrolledToBottom(scrollY = 0, viewportHeight = 200, contentHeight = 500, thresholdPx = threshold))
    }

    /**
     * Boundary (per DoD): content exactly fills the viewport -- scrollY=0 with
     * contentHeight == viewportHeight leaves nothing to scroll, so this counts as "at the bottom".
     */
    @Test
    fun contentHeightEqualsViewportHeight_isAtBottom() {
        assertTrue(ListeningPanelView.isScrolledToBottom(scrollY = 0, viewportHeight = 300, contentHeight = 300, thresholdPx = threshold))
    }

    /**
     * Inversion (per DoD): demonstrates the guard is load-bearing, not a tautology. A naive
     * "always auto-scroll" stand-in (the AC-3 pivot's predecessor defect, where the box showed
     * the beginning and never followed new text) would wrongly report a far-scrolled-up user as
     * being at the bottom. The REAL [ListeningPanelView.isScrolledToBottom] must disagree with
     * that naive stand-in for exactly this input -- if a future change made the real function
     * always return true, [assertNotEquals] below would fail, proving this test actually exercises
     * the guard rather than asserting a hard-coded literal.
     */
    @Test
    fun inversion_naiveAlwaysAtBottomDivergesFromRealFunctionForScrolledUpUser() {
        fun naiveAlwaysAtBottom(scrollY: Int, viewportHeight: Int, contentHeight: Int, thresholdPx: Int) = true

        val scrollY = 0
        val viewportHeight = 200
        val contentHeight = 500

        val naiveResult = naiveAlwaysAtBottom(scrollY, viewportHeight, contentHeight, threshold)
        val realResult = ListeningPanelView.isScrolledToBottom(scrollY, viewportHeight, contentHeight, threshold)

        assertTrue(
            "the naive stand-in wrongly claims a scrolled-up user is at the bottom",
            naiveResult
        )
        assertFalse(
            "the real function must correctly identify this user has scrolled away from the bottom",
            realResult
        )
        assertNotEquals(
            "the guard exists precisely because naive and real disagree for a scrolled-up user",
            naiveResult,
            realResult
        )
    }
}
