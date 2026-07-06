package com.klarvo.voice

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Story 9-15 TAP-surface geometry guard.
 *
 * NOTE (Story 9-16, Andi device feedback 2026-07-01): non-HOLD recording modes were reverted
 * from this TAP surface to the compact cluster ([FloatingBubbleView.drawRecordingCluster]), so the
 * circular 2D hit-detection guarded here ([isInsideCircle]/[tapCircleCenters]) now covers DEAD code
 * for the non-HOLD path. The file is retained (still green) because:
 *   (a) it documents the TAP geometry in case the dispatch is ever flipped back, and
 *   (b) the button-size constant tests below (TAP_BUTTON_SIZE_*) still govern the LIVE HOLD Cancel
 *       target — the size slider now scales only HOLD (9-16 decision).
 * The live compact-cluster touch/layout wiring is View-bound (not unit-testable) and is verified on
 * the real device (Andi's GATE-4), as the cluster originally was.
 *
 * Design:
 * - Tests the pure [isInsideCircle] math directly (independent predicate — not calling
 *   isTouchInConfirmZone/isTouchInCancelZone which have View state guards).
 * - Each test names the AC it covers and WHY it would go RED on a regression.
 * - Inversion tests explicitly verify that a miss really is a miss.
 *
 * Geometry used in tests (mirrors adjustLayoutForState TAP window, density-independent):
 *   Window: 340dp wide × 222dp tall (TAP_VISUAL_W_DP=320 + 2×TAP_SHADOW_PAD_DP=10)
 *   Right dock (default): Send circle cx=264, cy=146, radius=66  (all in dp-equivalent units)
 *                          Cancel circle cx=76, cy=146, radius=66
 *   Left dock (mirrored): Send circle cx=76, cy=146, radius=66
 *                          Cancel circle cx=264, cy=146, radius=66
 */
class TapSurfaceTouchZoneTest {

    // ---------------------------------------------------------------------------
    // Geometry constants (dp-space — density=1 simplifies the test math)
    // ---------------------------------------------------------------------------

    // Window dimensions (density-agnostic, using dp values directly):
    //   shadowPad = 10, radius = 66 (TAP_SEND_DIAM_DP/2 = 132/2)
    //   leftCx  = shadowPad + radius = 76
    //   rightCx = windowW - shadowPad - radius = 340 - 10 - 66 = 264
    //   chipH   = 54, chipGap = 16
    //   circlesCy = shadowPad + chipH + chipGap + radius = 10+54+16+66 = 146
    private val RADIUS = 66f
    private val CY     = 146f      // both circles share the same vertical center

    private val RIGHT_CX = 264f   // right side circle center
    private val LEFT_CX  = 76f    // left side circle center

    // For right dock: Send=right, Cancel=left
    private val SEND_CX_RIGHT   = RIGHT_CX
    private val CANCEL_CX_RIGHT = LEFT_CX

    // For left dock: Send=left, Cancel=right
    private val SEND_CX_LEFT   = LEFT_CX
    private val CANCEL_CX_LEFT = RIGHT_CX

    // ---------------------------------------------------------------------------
    // Helper shorthands
    // ---------------------------------------------------------------------------

    private fun inside(touchX: Float, touchY: Float, cx: Float, cy: Float) =
        FloatingBubbleView.isInsideCircle(touchX, touchY, cx, cy, RADIUS)

    // ---------------------------------------------------------------------------
    // AC4 — Basic circular hit detection: center of circle is always inside
    // ---------------------------------------------------------------------------

    @Test
    fun center_of_send_circle_right_dock_is_inside() {
        assertTrue(
            "Touch exactly at Send circle center must be a hit (AC4)",
            inside(SEND_CX_RIGHT, CY, SEND_CX_RIGHT, CY)
        )
    }

    @Test
    fun center_of_cancel_circle_right_dock_is_inside() {
        assertTrue(
            "Touch exactly at Cancel circle center must be a hit (AC4)",
            inside(CANCEL_CX_RIGHT, CY, CANCEL_CX_RIGHT, CY)
        )
    }

    // ---------------------------------------------------------------------------
    // AC4 — Touch on Send circle does NOT register as Cancel, and vice versa
    // ---------------------------------------------------------------------------

    @Test
    fun send_circle_center_is_not_in_cancel_zone_right_dock() {
        assertFalse(
            "Send circle center must NOT be inside the Cancel zone — zones must be distinct (AC4)",
            inside(SEND_CX_RIGHT, CY, CANCEL_CX_RIGHT, CY)
        )
    }

    @Test
    fun cancel_circle_center_is_not_in_send_zone_right_dock() {
        assertFalse(
            "Cancel circle center must NOT be inside the Send zone — zones must be distinct (AC4)",
            inside(CANCEL_CX_RIGHT, CY, SEND_CX_RIGHT, CY)
        )
    }

    // ---------------------------------------------------------------------------
    // AC4 — Touch at exactly the radius boundary (edge hit — inside == true)
    // ---------------------------------------------------------------------------

    @Test
    fun edge_touch_at_radius_is_inside_send_circle() {
        // Touch at exactly radius to the right of the Send circle center.
        assertTrue(
            "Touch at exact radius boundary must be inside the circle (AC4: edge tap is a hit)",
            inside(SEND_CX_RIGHT + RADIUS, CY, SEND_CX_RIGHT, CY)
        )
    }

    @Test
    fun touch_just_outside_radius_is_outside_cancel_circle() {
        // Touch one unit beyond the radius must miss.
        assertFalse(
            "Touch at radius+1 must be outside the circle (inversion: a miss is a miss)",
            inside(CANCEL_CX_RIGHT + RADIUS + 1f, CY, CANCEL_CX_RIGHT, CY)
        )
    }

    // ---------------------------------------------------------------------------
    // AC6 — Dock-side mirroring: for left dock, Send is on the left
    // ---------------------------------------------------------------------------

    @Test
    fun left_dock_send_circle_is_on_left_side() {
        // tapCircleCenters resolves send/cancel positions from the production dock→circle binding.
        // windowW=340, shadowPad=10, radius=66 → leftCx=76, rightCx=264.
        val (sendCx, cancelCx) = FloatingBubbleView.tapCircleCenters("left", 340f, 10f, RADIUS)
        assertTrue(
            "Left-dock Send must be at leftCx=76, was $sendCx (AC6 dock mirroring)",
            sendCx == LEFT_CX
        )
        assertTrue(
            "Left-dock Cancel must be at rightCx=264, was $cancelCx (AC6 dock mirroring)",
            cancelCx == RIGHT_CX
        )
        // Integration: tap at send center hits send but not cancel — proves binding AND geometry.
        assertTrue("Left-dock: tap at send center is inside send zone", inside(sendCx, CY, sendCx, CY))
        assertFalse("Left-dock: tap at send center must NOT be inside cancel zone", inside(sendCx, CY, cancelCx, CY))
    }

    @Test
    fun left_dock_cancel_circle_is_on_right_side() {
        // For right dock (default): send is on the right, cancel is on the left.
        val (sendCx, cancelCx) = FloatingBubbleView.tapCircleCenters("right", 340f, 10f, RADIUS)
        assertTrue(
            "Right-dock Send must be at rightCx=264, was $sendCx (AC6 dock mirroring)",
            sendCx == RIGHT_CX
        )
        assertTrue(
            "Right-dock Cancel must be at leftCx=76, was $cancelCx (AC6 dock mirroring)",
            cancelCx == LEFT_CX
        )
        // Integration: tap at send center hits send but not cancel — proves binding AND geometry.
        assertTrue("Right-dock: tap at send center is inside send zone", inside(sendCx, CY, sendCx, CY))
        assertFalse("Right-dock: tap at send center must NOT be inside cancel zone", inside(sendCx, CY, cancelCx, CY))
    }

    @Test
    fun left_dock_send_and_cancel_are_swapped_vs_right_dock() {
        val (leftSendCx, leftCancelCx)   = FloatingBubbleView.tapCircleCenters("left",  340f, 10f, RADIUS)
        val (rightSendCx, rightCancelCx) = FloatingBubbleView.tapCircleCenters("right", 340f, 10f, RADIUS)
        assertFalse(
            "Send center must differ between left and right dock — swap must be real (AC6)",
            leftSendCx == rightSendCx
        )
        assertTrue(
            "Left-dock Send position must equal right-dock Cancel position (AC6 swap)",
            leftSendCx == rightCancelCx
        )
        assertTrue(
            "Left-dock Cancel position must equal right-dock Send position (AC6 swap)",
            leftCancelCx == rightSendCx
        )
    }

    // ---------------------------------------------------------------------------
    // AC2 — Configurable button size {60,72,88}dp; default 72dp (Story 9-15 Re-Scope 2026-06-30)
    // ---------------------------------------------------------------------------

    @Test
    fun recording_button_size_default_is_72dp() {
        assertTrue(
            "TAP_BUTTON_SIZE_DEFAULT must be 72 (device-scale calibrated default — AC2 re-scope)",
            FloatingBubbleView.TAP_BUTTON_SIZE_DEFAULT == 72
        )
    }

    @Test
    fun recording_button_size_min_is_at_least_48dp() {
        assertTrue(
            "TAP_BUTTON_SIZE_MIN (${FloatingBubbleView.TAP_BUTTON_SIZE_MIN}) must be ≥ 48dp (minimum comfortable tap target — AC2)",
            FloatingBubbleView.TAP_BUTTON_SIZE_MIN >= 48
        )
    }

    @Test
    fun recording_button_size_range_is_52_to_96_per_story_9_14_rescope() {
        // Story 9-14 re-scope 2026-07-01 AC3: range widened from {60,72,88} to {52,60,72,84,96},
        // Andi-decided — now also governs the HOLD Abbrechen button, not just the TAP surface.
        assertTrue(
            "TAP_BUTTON_SIZE_MIN must be 52 (Story 9-14 re-scope AC3), was ${FloatingBubbleView.TAP_BUTTON_SIZE_MIN}",
            FloatingBubbleView.TAP_BUTTON_SIZE_MIN == 52
        )
        assertTrue(
            "TAP_BUTTON_SIZE_MAX must be 96 (Story 9-14 re-scope AC3), was ${FloatingBubbleView.TAP_BUTTON_SIZE_MAX}",
            FloatingBubbleView.TAP_BUTTON_SIZE_MAX == 96
        )
    }

    @Test
    fun visual_width_at_default_is_less_than_reference_max() {
        val visualW72 = FloatingBubbleView.tapVisualWidthDp(FloatingBubbleView.TAP_BUTTON_SIZE_DEFAULT)
        assertTrue(
            "tapVisualWidthDp(72)=$visualW72 must be < TAP_VISUAL_W_DP=${FloatingBubbleView.TAP_VISUAL_W_DP} (proportional downsizing — AC2 re-scope)",
            visualW72 < FloatingBubbleView.TAP_VISUAL_W_DP
        )
    }

    @Test
    fun visual_width_scales_proportionally_with_button_size() {
        val w60 = FloatingBubbleView.tapVisualWidthDp(60)
        val w72 = FloatingBubbleView.tapVisualWidthDp(72)
        val w88 = FloatingBubbleView.tapVisualWidthDp(88)
        assertTrue(
            "tapVisualWidthDp must increase monotonically with button size: $w60 < $w72 < $w88 (AC2 proportional scaling)",
            w60 < w72 && w72 < w88
        )
    }

    // ---------------------------------------------------------------------------
    // Inversion: touch between circles (chip area / backdrop) must miss both zones
    // ---------------------------------------------------------------------------

    @Test
    fun touch_between_circles_misses_both_zones() {
        // Mid-point between left and right circle centers (rightCx-leftCx = 188; mid = leftCx + 94 = 170)
        val midX = (LEFT_CX + RIGHT_CX) / 2f
        assertFalse(
            "Touch at horizontal midpoint between circles must miss Send zone",
            inside(midX, CY, SEND_CX_RIGHT, CY)
        )
        assertFalse(
            "Touch at horizontal midpoint between circles must miss Cancel zone",
            inside(midX, CY, CANCEL_CX_RIGHT, CY)
        )
    }

    // ---------------------------------------------------------------------------
    // Inversion: touch in chip area (above circles) must miss both zones
    // ---------------------------------------------------------------------------

    @Test
    fun touch_in_chip_area_misses_both_zones() {
        // Chip area is above circlesCy: use chipCy = shadowPad + chipH/2 = 10 + 27 = 37
        val chipCy = 37f
        assertFalse(
            "Touch in chip zone (above circles) must miss Send zone (no false-positive)",
            inside(SEND_CX_RIGHT, chipCy, SEND_CX_RIGHT, CY)
        )
        assertFalse(
            "Touch in chip zone (above circles) must miss Cancel zone (no false-positive)",
            inside(CANCEL_CX_RIGHT, chipCy, CANCEL_CX_RIGHT, CY)
        )
    }
}
