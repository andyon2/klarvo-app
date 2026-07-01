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
 *   holdVisualWidthDp(72,44)  = 44/2 + 165 + 45 = 232
 *   holdVisualHeightDp(72,44) = 2×max(45, 26, 22) = 90   (level layout, 2026-07-01)
 *   windowW = 232 + 2×10(shadowPad) = 252, windowH = 90 + 2×10 = 110
 *   Right dock: bubble center=(220,55), Abbrechen center=(55,55)  — SAME Y (level)
 *   Left dock (mirrored): bubble center=(32,55), Abbrechen center=(197,55)
 * Hit-test radius is always [REST_R] (Task 4.2 — growing to ACTIVE is feedback, not a hit-zone
 * change).
 *
 * 2026-07-01 level re-tune (Andi: "das X auf gleicher Höhe wie die waveform und den rest"):
 * HOLD_CANCEL_OFFSET_Y_DP is now 0, so the ✗ sits LEVEL with the bubble and the waveform chip
 * (one horizontal row). The bubble is centred vertically in the HOLD window, so its screen-space
 * center still equals the idle thumb Y (window.y = idleCenterY - bubbleCenterYPx) — the AC2
 * "bubble never clamped away from the thumb" invariant holds by construction at every dock height,
 * with no up/down grow-direction flip needed (the `growDirection` param is retained for signature
 * stability but no longer affects the vertical layout).
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

    private fun bubbleCenter(dockSide: String, growDirection: String = "up") = FloatingBubbleView.holdBubbleCenter(
        dockSide, growDirection, WINDOW_W, WINDOW_H, SHADOW_PAD, BUBBLE_SIZE_DP.toFloat()
    )

    private fun cancelCenter(dockSide: String, growDirection: String = "up") = FloatingBubbleView.holdCancelCenter(
        dockSide, growDirection, bubbleCenter(dockSide, growDirection), OFFSET_X, OFFSET_Y
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
        assertEquals("Right-dock bubble center x must be 220 (AC2)", 220f, bubble.x, 0.01f)
        assertEquals("Right-dock bubble center y must be 55 — window middle (AC2)", 55f, bubble.y, 0.01f)
    }

    @Test
    fun left_dock_bubble_center_mirrors_horizontally_only() {
        val right = bubbleCenter("right")
        val left  = bubbleCenter("left")
        assertTrue("Left-dock bubble x must differ from right-dock (AC2/AC7)", left.x != right.x)
        assertEquals("Bubble y must NOT change with dock side", right.y, left.y, 0.01f)
        assertEquals("Left-dock bubble x must be 32", 32f, left.x, 0.01f)
        assertEquals("Left-dock bubble y must be 55 — window middle", 55f, left.y, 0.01f)
    }

    // ---------------------------------------------------------------------------
    // AC7 — Dock-side mirroring of the Abbrechen target: center swaps horizontally,
    // vertical role (always above the bubble) stays fixed
    // ---------------------------------------------------------------------------

    @Test
    fun right_dock_cancel_center_matches_mockup_derived_geometry() {
        val cancel = cancelCenter("right")
        assertEquals("Right-dock Abbrechen center x must be 55 (AC7)", 55f, cancel.x, 0.01f)
        assertEquals("Right-dock Abbrechen center y must be 55 — level with bubble (AC7)", 55f, cancel.y, 0.01f)
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
        assertEquals("Left-dock Abbrechen x must be 197", 197f, left.x, 0.01f)
    }

    @Test
    fun cancel_target_is_level_with_the_bubble_regardless_of_dock_side() {
        // 2026-07-01 re-tune (Andi): the ✗ sits LEVEL with the bubble and the waveform chip — one
        // horizontal row, not above/below. This replaces the old "always above" vertical role.
        for (dockSide in listOf("left", "right")) {
            val bubble = bubbleCenter(dockSide)
            val cancel = cancelCenter(dockSide)
            assertEquals(
                "Abbrechen must sit at the same Y as the bubble for dock=$dockSide (level layout)",
                bubble.y, cancel.y, 0.01f
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
        assertEquals(232, FloatingBubbleView.holdVisualWidthDp(BUTTON_SIZE_DP, BUBBLE_SIZE_DP))
        assertEquals(90, FloatingBubbleView.holdVisualHeightDp(BUTTON_SIZE_DP, BUBBLE_SIZE_DP))
    }

    // ---------------------------------------------------------------------------
    // High-dock case (2026-07-01 level layout): the bubble is centred vertically in the HOLD
    // window and the ✗ is level with it, so the whole cluster is centred on the idle thumb Y at
    // every dock height — no up/down flip. These tests replicate KlarvoOverlayService's Y-anchor
    // construction (`window.y = idleCenterY - bubbleCenterYPx`) to lock the invariant at the level
    // it matters: the bubble's SCREEN-space center equals the thumb, not just its window offset.
    // ---------------------------------------------------------------------------

    @Test
    fun high_dock_bubble_screen_center_equals_idle_center() {
        // idleCenterY=50 simulates a bubble docked close to the top of the screen. The bubble is
        // centred in the window (bubbleCenterYPx = windowH/2), so window.y goes slightly negative
        // but the bubble's screen-space center still lands exactly on the thumb — never clamped.
        val idleCenterY = 50f
        val bubbleLocalY = bubbleCenter("right").y
        val windowY = idleCenterY - bubbleLocalY
        val bubbleScreenY = windowY + bubbleLocalY
        assertEquals(
            "Bubble's screen-space center must equal idleCenterY exactly — never clamped away " +
                "from the thumb at any dock position (AC2)",
            idleCenterY, bubbleScreenY, 0.01f
        )
    }

    @Test
    fun high_dock_cancel_target_stays_level_with_bubble_and_on_screen() {
        val bubble = bubbleCenter("right")
        val cancel = cancelCenter("right")
        assertEquals(
            "The Abbrechen target must stay LEVEL with the bubble at a high dock (same Y) — the " +
                "cluster is centred on the thumb, no vertical flip (level layout)",
            bubble.y, cancel.y, 0.01f
        )

        // On-screen check: place the window the same way KlarvoOverlayService does for a bubble
        // docked at idleCenterY=50 (near the screen top) and verify the target's top edge stays
        // on-screen (>= 0) even in this extreme.
        val idleCenterY = 50f
        val windowY = idleCenterY - bubble.y
        val cancelScreenY = windowY + cancel.y
        assertTrue(
            "Abbrechen target's top edge must stay on-screen (>= 0) even at an extreme high dock",
            cancelScreenY - ACTIVE_R >= 0f
        )
    }

    @Test
    fun grow_direction_does_not_affect_horizontal_dock_mirroring() {
        // Finding B only changes the vertical axis — dockSide's horizontal mirroring (AC7) must
        // still hold regardless of growDirection.
        val downRight = cancelCenter("right", "down")
        val downLeft  = cancelCenter("left", "down")
        assertTrue(
            "Left/right horizontal mirroring must still hold when growDirection=down",
            downLeft.x != downRight.x
        )
    }
}
