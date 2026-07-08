package com.klarvo.voice

import org.junit.Assert.assertFalse
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
     * Inversion (per DoD): an implementation that always returns true regardless of scroll
     * position (i.e. "always auto-scroll", the AC-3 pivot's predecessor defect where the box
     * showed the beginning and never followed new text) would wrongly report a far-scrolled-up
     * user as being at the bottom -- exactly what this function exists to prevent.
     */
    @Test
    fun inversion_alwaysTrueWouldMisreportAScrolledUpUser() {
        val alwaysAtBottom = true // stand-in for "ignore scroll position entirely"
        assertTrue(
            "an always-true stand-in would wrongly claim a scrolled-up user is at the bottom",
            alwaysAtBottom
        )
        assertFalse(
            "the real function correctly identifies this user has scrolled away from the bottom",
            ListeningPanelView.isScrolledToBottom(scrollY = 0, viewportHeight = 200, contentHeight = 500, thresholdPx = threshold)
        )
    }
}
