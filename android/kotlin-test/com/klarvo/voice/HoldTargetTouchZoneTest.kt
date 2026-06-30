package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * AC2/AC4/AC5/AC7 regression lock — Story 9-14 re-scope (2026-07-01): vereinfachtes HOLD
 * (ein Abbrechen-Button, kein Sperren) touch-zone correctness.
 *
 * Mirrors TapSurfaceTouchZoneTest.kt's pattern (Story 9-15): tests the pure
 * [FloatingBubbleView.holdBubbleCenter] / [FloatingBubbleView.holdCancelCenter] /
 * [FloatingBubbleView.isInsideCircle] / [FloatingBubbleView.holdVisualWidthDp] /
 * [FloatingBubbleView.holdVisualHeightDp] functions directly — no Android View/Robolectric
 * needed (no-Robolectric, pure-function-under-test convention established by 9-13/9-15).
 *
 * Replaces the prior two-target (Sperren+Abbrechen) test file in full (not a patch) — every old
 * test referenced LOCK directly or via Lock-vs-Cancel relative positioning, none of which survives
 * the single-target re-scope.
 *
 * Geometry used in tests (density=1 dp-space, mirrors the production formula in
 * KlarvoOverlayService.handleTouch's ACTION_MOVE branch / FloatingBubbleView.drawHoldTargets):
 *   buttonSizeDp=72 (default), bubbleSizeDp=44 (max responsive idle bubble size)
 *   activeR = 72×1.25/2 = 45, restR = 72/2 = 36
 *   holdVisualWidthDp(72,44)  = 44/2 + 178 + 45 = 245
 *   holdVisualHeightDp(72,44) = 44/2 + 160 + 45 = 227
 *   windowW = 245 + 2×10(shadowPad) = 265, windowH = 227 + 2×10 = 247
 *   Right dock (default): bubble center=(233,215), Abbrechen center=(55,55)
 *   Left dock (mirrored): bubble center=(32,215),  Abbrechen center=(210,55)
 * Hit-test radius is always [REST_R] (Task 4.2 — growing to ACTIVE is feedback, not a hit-zone
 * change).
 */
class HoldTargetTouchZoneTest {

    private val BUTTON_SIZE_DP = 72
    private val BUBBLE_SIZE_DP = 44
    private val SHADOW_PAD = 10f
    private val OFFSET_X = FloatingBubbleView.HOLD_CANCEL_OFFSET_X_DP
    private val OFFSET_Y = FloatingBubbleView.HOLD_CANCEL_OFFSET_Y_DP
    private val REST_R = BUTTON_SIZE_DP / 2f
    private val ACTIVE_R = BUTTON_SIZE_DP * FloatingBubbleView.HOLD_CANCEL_ACTIVE_SCALE / 2f

    private val WINDOW_W = (FloatingBubbleView.holdVisualWidthDp(BUTTON_SIZE_DP, BUBBLE_SIZE_DP) + 2 * SHADOW_PAD).toFloat()
    private val WINDOW_H = (FloatingBubbleView.holdVisualHeightDp(BUTTON_SIZE_DP, BUBBLE_SIZE_DP) + 2 * SHADOW_PAD).toFloat()

    private fun bubbleCenter(dockSide: String) = FloatingBubbleView.holdBubbleCenter(
        dockSide, WINDOW_W, WINDOW_H, SHADOW_PAD, BUBBLE_SIZE_DP.toFloat()
    )

    private fun cancelCenter(dockSide: String) = FloatingBubbleView.holdCancelCenter(
        dockSide, bubbleCenter(dockSide), OFFSET_X, OFFSET_Y
    )

    /** Replicates KlarvoOverlayService.handleTouch's ACTION_MOVE hit-dispatch (Task 6.1) against
     *  the REST-radius hit zone — the same isInsideCircle call production uses. */
    private fun resolveHit(touchX: Float, touchY: Float, dockSide: String): HoldTarget {
        val cancel = cancelCenter(dockSide)
        return if (FloatingBubbleView.isInsideCircle(touchX, touchY, cancel.x, cancel.y, REST_R)) {
            HoldTarget.CANCEL
        } else {
            HoldTarget.NONE
        }
    }

    // ---------------------------------------------------------------------------
    // AC2 — Anchor bubble center: idle position, no HOLD-specific edge inset (Task 1.5)
    // ---------------------------------------------------------------------------

    @Test
    fun right_dock_bubble_center_matches_derived_geometry() {
        val bubble = bubbleCenter("right")
        assertEquals("Right-dock bubble center x must be 233 (AC2)", 233f, bubble.x, 0.01f)
        assertEquals("Right-dock bubble center y must be 215 (AC2)", 215f, bubble.y, 0.01f)
    }

    @Test
    fun left_dock_bubble_center_mirrors_horizontally_only() {
        val right = bubbleCenter("right")
        val left  = bubbleCenter("left")
        assertTrue("Left-dock bubble x must differ from right-dock (AC2/AC7)", left.x != right.x)
        assertEquals("Bubble y must NOT change with dock side", right.y, left.y, 0.01f)
        assertEquals("Left-dock bubble x must be 32", 32f, left.x, 0.01f)
    }

    // ---------------------------------------------------------------------------
    // AC7 — Dock-side mirroring of the Abbrechen target: center swaps horizontally,
    // vertical role (always above the bubble) stays fixed
    // ---------------------------------------------------------------------------

    @Test
    fun right_dock_cancel_center_matches_mockup_derived_geometry() {
        val cancel = cancelCenter("right")
        assertEquals("Right-dock Abbrechen center x must be 55 (AC7)", 55f, cancel.x, 0.01f)
        assertEquals("Right-dock Abbrechen center y must be 55 (AC7)", 55f, cancel.y, 0.01f)
    }

    @Test
    fun left_dock_cancel_center_mirrors_horizontally_only() {
        val right = cancelCenter("right")
        val left  = cancelCenter("left")
        assertTrue(
            "Left-dock Abbrechen x must differ from right-dock — horizontal mirror is real (AC7)",
            left.x != right.x
        )
        assertEquals(
            "Abbrechen y must NOT change with dock side — vertical role is fixed (AC7)",
            right.y, left.y, 0.01f
        )
        assertEquals("Left-dock Abbrechen x must be 210", 210f, left.x, 0.01f)
    }

    @Test
    fun cancel_target_is_always_above_the_bubble_regardless_of_dock_side() {
        for (dockSide in listOf("left", "right")) {
            val bubble = bubbleCenter(dockSide)
            val cancel = cancelCenter(dockSide)
            assertTrue(
                "Abbrechen must sit above the bubble (smaller y) for dock=$dockSide (AC7 fixed vertical role)",
                cancel.y < bubble.y
            )
        }
    }

    @Test
    fun cancel_target_grows_toward_screen_center_away_from_dock_edge() {
        // Right-docked bubble is near the right window edge; the target must sit further LEFT
        // (toward center), not further right (toward the edge it's already pinned to).
        val rightBubble = bubbleCenter("right")
        val rightCancel = cancelCenter("right")
        assertTrue("Right-dock Abbrechen must sit left of the bubble", rightCancel.x < rightBubble.x)

        // Left-docked bubble is near the left edge; the target must sit further RIGHT (toward center).
        val leftBubble = bubbleCenter("left")
        val leftCancel = cancelCenter("left")
        assertTrue("Left-dock Abbrechen must sit right of the bubble", leftCancel.x > leftBubble.x)
    }

    // ---------------------------------------------------------------------------
    // AC5 — Hit-test resolution at REST radius (the function Task 6's ACTION_MOVE calls)
    // ---------------------------------------------------------------------------

    @Test
    fun touch_at_cancel_center_resolves_to_cancel_right_dock() {
        val cancel = cancelCenter("right")
        assertEquals(HoldTarget.CANCEL, resolveHit(cancel.x, cancel.y, "right"))
    }

    @Test
    fun touch_at_cancel_center_resolves_to_cancel_left_dock() {
        val cancel = cancelCenter("left")
        assertEquals(HoldTarget.CANCEL, resolveHit(cancel.x, cancel.y, "left"))
    }

    @Test
    fun touch_at_rest_radius_edge_is_a_hit() {
        val cancel = cancelCenter("right")
        assertEquals(
            "Touch at exactly REST_R from Abbrechen center must be a hit (AC5 edge tap)",
            HoldTarget.CANCEL, resolveHit(cancel.x + REST_R, cancel.y, "right")
        )
    }

    @Test
    fun touch_just_outside_rest_radius_is_a_miss() {
        val cancel = cancelCenter("right")
        assertEquals(
            "Touch at REST_R+1 from Abbrechen center must miss (inversion: a miss is a miss)",
            HoldTarget.NONE, resolveHit(cancel.x + REST_R + 1f, cancel.y, "right")
        )
    }

    // ---------------------------------------------------------------------------
    // AC4 — ACTIVE growth does NOT change the hit zone (Task 4.2)
    // ---------------------------------------------------------------------------

    @Test
    fun touch_inside_active_radius_but_outside_rest_radius_still_misses() {
        // Between REST_R (36) and ACTIVE_R (45) from the Abbrechen center: a touch here would be
        // inside the visually-grown ACTIVE circle but the hit-test boundary stays REST_R — this
        // point must miss, proving growth is feedback-only, not a hit-zone expansion.
        val cancel = cancelCenter("right")
        assertEquals(
            "Touch between REST_R and ACTIVE_R must miss — hit-zone stays REST size even when the target visually grows (AC4 + Task 4.2)",
            HoldTarget.NONE, resolveHit(cancel.x + 40f, cancel.y, "right")
        )
    }

    // ---------------------------------------------------------------------------
    // Inversion: touch outside the target circle (bubble area / dead space) -> NONE
    // ---------------------------------------------------------------------------

    @Test
    fun touch_at_bubble_position_misses_the_target() {
        val bubble = bubbleCenter("right")
        assertEquals(HoldTarget.NONE, resolveHit(bubble.x, bubble.y, "right"))
    }

    @Test
    fun touch_at_window_origin_misses_the_target() {
        assertEquals(HoldTarget.NONE, resolveHit(0f, 0f, "right"))
    }

    // ---------------------------------------------------------------------------
    // AC3/AC4 — Active-scale and visual-size geometry functions (Task 8.2, mirrors
    // TapSurfaceTouchZoneTest.visual_width_scales_proportionally_with_button_size)
    // ---------------------------------------------------------------------------

    @Test
    fun hold_cancel_active_scale_is_1_25() {
        assertEquals(
            "HOLD_CANCEL_ACTIVE_SCALE must be 1.25 (mockup .zone.active 120px / .zone.rest 96px — Task 1.3)",
            1.25f, FloatingBubbleView.HOLD_CANCEL_ACTIVE_SCALE, 0.001f
        )
    }

    @Test
    fun hold_visual_width_scales_proportionally_with_button_size() {
        val w52 = FloatingBubbleView.holdVisualWidthDp(52, BUBBLE_SIZE_DP)
        val w72 = FloatingBubbleView.holdVisualWidthDp(72, BUBBLE_SIZE_DP)
        val w96 = FloatingBubbleView.holdVisualWidthDp(96, BUBBLE_SIZE_DP)
        assertTrue(
            "holdVisualWidthDp must increase monotonically with button size: $w52 < $w72 < $w96 (AC3 proportional scaling)",
            w52 < w72 && w72 < w96
        )
    }

    @Test
    fun hold_visual_height_scales_proportionally_with_button_size() {
        val h52 = FloatingBubbleView.holdVisualHeightDp(52, BUBBLE_SIZE_DP)
        val h72 = FloatingBubbleView.holdVisualHeightDp(72, BUBBLE_SIZE_DP)
        val h96 = FloatingBubbleView.holdVisualHeightDp(96, BUBBLE_SIZE_DP)
        assertTrue(
            "holdVisualHeightDp must increase monotonically with button size: $h52 < $h72 < $h96 (AC3 proportional scaling)",
            h52 < h72 && h72 < h96
        )
    }

    @Test
    fun hold_visual_dims_at_default_match_derived_geometry() {
        assertEquals(245, FloatingBubbleView.holdVisualWidthDp(BUTTON_SIZE_DP, BUBBLE_SIZE_DP))
        assertEquals(227, FloatingBubbleView.holdVisualHeightDp(BUTTON_SIZE_DP, BUBBLE_SIZE_DP))
    }
}
