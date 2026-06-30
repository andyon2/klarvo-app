package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * AC5/AC7 regression lock — Story 9-14: Mobile HOLD-targets (B-Sprache) touch-zone correctness.
 *
 * Mirrors TapSurfaceTouchZoneTest.kt's pattern (Story 9-15): tests the pure
 * [FloatingBubbleView.holdTargetCenters] / [FloatingBubbleView.isInsideCircle] functions directly
 * — no Android View/Robolectric needed (no-Robolectric, pure-function-under-test convention
 * established by 9-13/9-15).
 *
 * Geometry used in tests (density=1 dp-space, mirrors the production formula in
 * KlarvoOverlayService.handleTouch's ACTION_MOVE branch / FloatingBubbleView.drawHoldTargets):
 *   windowW = HOLD_VISUAL_W_DP(338) + 2×HOLD_SHADOW_PAD_DP(10) = 358
 *   shadowPad=10, bubbleEdgeGap=16, bubbleDiam=82, farInset=40
 *   restRadius=56 (HOLD_TARGET_REST_DP/2), activeRadius=74 (HOLD_TARGET_ACTIVE_DP/2)
 *   lockOffsetAbove=168, cancelOffsetBelow=74
 *   Right dock (default): Lock center=(124,84), Cancel center=(124,326)
 *   Left dock (mirrored): Lock center=(234,84), Cancel center=(234,326)
 * Hit-test radius is always [restRadius] (Task 4.2 — growing to ACTIVE is feedback, not a
 * hit-zone change).
 */
class HoldTargetTouchZoneTest {

    private val WINDOW_W = 358f
    private val SHADOW_PAD = 10f
    private val BUBBLE_EDGE_GAP = 16f
    private val BUBBLE_DIAM = 82f
    private val FAR_INSET = 40f
    private val ACTIVE_R = 74f
    private val REST_R = 56f
    private val LOCK_OFFSET_ABOVE = 168f
    private val CANCEL_OFFSET_BELOW = 74f

    private fun centers(dockSide: String) = FloatingBubbleView.holdTargetCenters(
        dockSide, WINDOW_W, SHADOW_PAD, BUBBLE_EDGE_GAP, BUBBLE_DIAM,
        FAR_INSET, ACTIVE_R, LOCK_OFFSET_ABOVE, CANCEL_OFFSET_BELOW
    )

    /** Replicates KlarvoOverlayService.handleTouch's ACTION_MOVE hit-dispatch (Task 6.1) against
     *  the REST-radius hit zone — the same combination of two isInsideCircle calls production uses. */
    private fun resolveHit(touchX: Float, touchY: Float, dockSide: String): HoldTarget {
        val (lock, cancel) = centers(dockSide)
        return when {
            FloatingBubbleView.isInsideCircle(touchX, touchY, lock.x, lock.y, REST_R) -> HoldTarget.LOCK
            FloatingBubbleView.isInsideCircle(touchX, touchY, cancel.x, cancel.y, REST_R) -> HoldTarget.CANCEL
            else -> HoldTarget.NONE
        }
    }

    // ---------------------------------------------------------------------------
    // AC7 — Dock-side mirroring: centers swap horizontally, vertical roles stay fixed
    // ---------------------------------------------------------------------------

    @Test
    fun right_dock_centers_match_mockup_derived_geometry() {
        val (lock, cancel) = centers("right")
        assertEquals("Right-dock Sperren center x must be 124 (AC7)", 124f, lock.x, 0.01f)
        assertEquals("Right-dock Sperren center y must be 84 (AC7)", 84f, lock.y, 0.01f)
        assertEquals("Right-dock Abbrechen center x must be 124 (AC7)", 124f, cancel.x, 0.01f)
        assertEquals("Right-dock Abbrechen center y must be 326 (AC7)", 326f, cancel.y, 0.01f)
    }

    @Test
    fun left_dock_centers_mirror_horizontally_only() {
        val (rightLock, rightCancel) = centers("right")
        val (leftLock, leftCancel)   = centers("left")
        assertTrue(
            "Left-dock Sperren x must differ from right-dock — horizontal mirror is real (AC7)",
            leftLock.x != rightLock.x
        )
        assertEquals(
            "Sperren y must NOT change with dock side — vertical roles are fixed (AC7)",
            rightLock.y, leftLock.y, 0.01f
        )
        assertEquals(
            "Abbrechen y must NOT change with dock side — vertical roles are fixed (AC7)",
            rightCancel.y, leftCancel.y, 0.01f
        )
        assertEquals("Left-dock Lock x must be 234", 234f, leftLock.x, 0.01f)
        assertEquals("Left-dock Cancel x must be 234", 234f, leftCancel.x, 0.01f)
    }

    @Test
    fun lock_is_always_above_cancel_regardless_of_dock_side() {
        for (dockSide in listOf("left", "right")) {
            val (lock, cancel) = centers(dockSide)
            assertTrue(
                "Sperren must sit above Abbrechen (smaller y) for dock=$dockSide (AC7 fixed vertical roles)",
                lock.y < cancel.y
            )
        }
    }

    // ---------------------------------------------------------------------------
    // AC5 — Hit-test resolution at REST radius (the function Task 6's ACTION_MOVE calls)
    // ---------------------------------------------------------------------------

    @Test
    fun touch_at_lock_center_resolves_to_lock_right_dock() {
        val (lock, _) = centers("right")
        assertEquals(HoldTarget.LOCK, resolveHit(lock.x, lock.y, "right"))
    }

    @Test
    fun touch_at_cancel_center_resolves_to_cancel_right_dock() {
        val (_, cancel) = centers("right")
        assertEquals(HoldTarget.CANCEL, resolveHit(cancel.x, cancel.y, "right"))
    }

    @Test
    fun touch_at_lock_center_resolves_to_lock_left_dock() {
        val (lock, _) = centers("left")
        assertEquals(HoldTarget.LOCK, resolveHit(lock.x, lock.y, "left"))
    }

    @Test
    fun touch_at_cancel_center_resolves_to_cancel_left_dock() {
        val (_, cancel) = centers("left")
        assertEquals(HoldTarget.CANCEL, resolveHit(cancel.x, cancel.y, "left"))
    }

    @Test
    fun touch_at_rest_radius_edge_is_a_hit() {
        val (lock, _) = centers("right")
        assertEquals(
            "Touch at exactly REST_R from Sperren center must be a hit (AC5 edge tap)",
            HoldTarget.LOCK, resolveHit(lock.x + REST_R, lock.y, "right")
        )
    }

    @Test
    fun touch_just_outside_rest_radius_is_a_miss() {
        val (lock, _) = centers("right")
        assertEquals(
            "Touch at REST_R+1 from Sperren center must miss (inversion: a miss is a miss)",
            HoldTarget.NONE, resolveHit(lock.x + REST_R + 1f, lock.y, "right")
        )
    }

    // ---------------------------------------------------------------------------
    // AC3/AC4 — ACTIVE growth does NOT change the hit zone (Task 4.2)
    // ---------------------------------------------------------------------------

    @Test
    fun touch_inside_active_radius_but_outside_rest_radius_still_misses() {
        // Between REST_R (56) and ACTIVE_R (74) from the Sperren center: a touch here would be
        // inside the visually-grown ACTIVE circle but the hit-test boundary stays REST_R — this
        // point must miss, proving growth is feedback-only, not a hit-zone expansion.
        val (lock, _) = centers("right")
        assertEquals(
            "Touch between REST_R and ACTIVE_R must miss — hit-zone stays REST size even when the target visually grows (AC3/AC4 + Task 4.2)",
            HoldTarget.NONE, resolveHit(lock.x + 65f, lock.y, "right")
        )
    }

    // ---------------------------------------------------------------------------
    // Inversion: touch outside both circles (chip area / bubble area / dead space) -> NONE
    // ---------------------------------------------------------------------------

    @Test
    fun touch_at_bubble_position_misses_both_targets() {
        // Right-dock bubble center ≈ (291, 252) — far from both target centers.
        assertEquals(HoldTarget.NONE, resolveHit(291f, 252f, "right"))
    }

    @Test
    fun touch_at_window_origin_misses_both_targets() {
        assertEquals(HoldTarget.NONE, resolveHit(0f, 0f, "right"))
    }

    @Test
    fun touch_between_lock_and_cancel_misses_both_targets() {
        val (lock, cancel) = centers("right")
        val midY = (lock.y + cancel.y) / 2f
        assertFalse(
            "Touch at vertical midpoint between targets must miss Sperren",
            FloatingBubbleView.isInsideCircle(lock.x, midY, lock.x, lock.y, REST_R)
        )
        assertEquals(HoldTarget.NONE, resolveHit(lock.x, midY, "right"))
    }
}
