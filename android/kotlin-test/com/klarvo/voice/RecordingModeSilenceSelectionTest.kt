package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * AC6 regression lock — Story 9-7: Short-press gesture modes mirror desktop.
 *
 * Guards the silence-field mapping introduced/verified in 9-7 and the prior
 * Android silence-field divergence fix ([project_android_silence_field_divergence]).
 *
 * The historic bug: AUTO and AUTOSTOP modes read only `bubbleTapSilenceSecs`
 * instead of the mode-level fields (`autoModeSilenceSecs`, `autostopSilenceSecs`).
 * This test goes RED on any regression to that old behaviour.
 *
 * The function under test is `RecordingMode.selectSilenceSecs()` — a pure companion
 * function extracted from `KlarvoOverlayService.startRecording()` specifically for
 * testability (behaviour byte-identical to the original inline block, AC6 Story 9-7).
 *
 * Design principles:
 * - Uses an **independent expected-value table** that the test itself owns.
 *   The production path is NOT called and compared against itself — that would be
 *   a vacuous green (feedback_test_must_not_judge_sut_with_itself).
 * - Explicitly tests the regression cases (bubbleTapSilenceSecs for AUTO/AUTOSTOP)
 *   by verifying the function returns a DIFFERENT value for those inputs.
 */
class RecordingModeSilenceSelectionTest {

    // ---------------------------------------------------------------------------
    // Independent expected-value table (owned by this test — NOT derived from SUT)
    // ---------------------------------------------------------------------------
    //
    //  mode       | gesture      | expected field
    //  -----------|--------------|------------------------
    //  AUTO       | any          | autoModeSilenceSecs   = 4.0f
    //  AUTOSTOP   | any          | autostopSilenceSecs   = 3.0f
    //  HOLD       | "tap"        | tapSilenceSecs        = 1.0f
    //  TOGGLE     | "tap"        | tapSilenceSecs        = 1.0f
    //  HOLD       | "longpress"  | longPressSilenceSecs  = 2.0f
    //  TOGGLE     | "longpress"  | longPressSilenceSecs  = 2.0f
    //  HOLD       | null         | tapSilenceSecs        = 1.0f  (null ≡ tap path)
    //
    // Distinct values are intentionally chosen so an accidental field-swap is detectable.

    private val TAP_SILENCE        = 1.0f
    private val LONG_PRESS_SILENCE = 2.0f
    private val AUTOSTOP_SILENCE   = 3.0f
    private val AUTO_MODE_SILENCE  = 4.0f

    private fun select(
        mode: KlarvoOverlayService.RecordingMode,
        gesture: String?
    ): Float = KlarvoOverlayService.RecordingMode.selectSilenceSecs(
        mode              = mode,
        gesture           = gesture,
        tapSilence        = TAP_SILENCE,
        longPressSilence  = LONG_PRESS_SILENCE,
        autostopSilence   = AUTOSTOP_SILENCE,
        autoModeSilence   = AUTO_MODE_SILENCE,
    )

    // ---------------------------------------------------------------------------
    // AUTO mode → autoModeSilenceSecs (regression: old code used bubbleTapSilenceSecs)
    // ---------------------------------------------------------------------------

    @Test
    fun auto_tap_uses_autoModeSilenceSecs() {
        assertEquals(
            "AUTO + tap gesture must use autoModeSilenceSecs, not bubbleTapSilenceSecs (AC6 regression)",
            AUTO_MODE_SILENCE,
            select(KlarvoOverlayService.RecordingMode.AUTO, "tap")
        )
    }

    @Test
    fun auto_longpress_uses_autoModeSilenceSecs() {
        assertEquals(
            "AUTO + longpress gesture must still use autoModeSilenceSecs (mode takes priority)",
            AUTO_MODE_SILENCE,
            select(KlarvoOverlayService.RecordingMode.AUTO, "longpress")
        )
    }

    @Test
    fun auto_nullGesture_uses_autoModeSilenceSecs() {
        assertEquals(
            "AUTO + null gesture (auto-loop restart) must use autoModeSilenceSecs",
            AUTO_MODE_SILENCE,
            select(KlarvoOverlayService.RecordingMode.AUTO, null)
        )
    }

    // ---------------------------------------------------------------------------
    // AUTOSTOP mode → autostopSilenceSecs (regression: old code used bubbleTapSilenceSecs)
    // ---------------------------------------------------------------------------

    @Test
    fun autostop_tap_uses_autostopSilenceSecs() {
        assertEquals(
            "AUTOSTOP + tap gesture must use autostopSilenceSecs, not bubbleTapSilenceSecs (AC6 regression)",
            AUTOSTOP_SILENCE,
            select(KlarvoOverlayService.RecordingMode.AUTOSTOP, "tap")
        )
    }

    @Test
    fun autostop_longpress_uses_autostopSilenceSecs() {
        assertEquals(
            "AUTOSTOP + longpress gesture must still use autostopSilenceSecs (mode takes priority)",
            AUTOSTOP_SILENCE,
            select(KlarvoOverlayService.RecordingMode.AUTOSTOP, "longpress")
        )
    }

    // ---------------------------------------------------------------------------
    // HOLD + tap gesture → tapSilenceSecs
    // ---------------------------------------------------------------------------

    @Test
    fun hold_tap_uses_tapSilenceSecs() {
        assertEquals(
            "HOLD + tap gesture must use tapSilenceSecs",
            TAP_SILENCE,
            select(KlarvoOverlayService.RecordingMode.HOLD, "tap")
        )
    }

    @Test
    fun hold_nullGesture_uses_tapSilenceSecs() {
        assertEquals(
            "HOLD + null gesture falls back to tap path (tapSilenceSecs)",
            TAP_SILENCE,
            select(KlarvoOverlayService.RecordingMode.HOLD, null)
        )
    }

    // ---------------------------------------------------------------------------
    // TOGGLE + tap gesture → tapSilenceSecs
    // ---------------------------------------------------------------------------

    @Test
    fun toggle_tap_uses_tapSilenceSecs() {
        assertEquals(
            "TOGGLE + tap gesture must use tapSilenceSecs",
            TAP_SILENCE,
            select(KlarvoOverlayService.RecordingMode.TOGGLE, "tap")
        )
    }

    // ---------------------------------------------------------------------------
    // HOLD + longpress gesture → longPressSilenceSecs
    // ---------------------------------------------------------------------------

    @Test
    fun hold_longpress_uses_longPressSilenceSecs() {
        assertEquals(
            "HOLD + longpress gesture must use longPressSilenceSecs",
            LONG_PRESS_SILENCE,
            select(KlarvoOverlayService.RecordingMode.HOLD, "longpress")
        )
    }

    // ---------------------------------------------------------------------------
    // TOGGLE + longpress gesture → longPressSilenceSecs
    // ---------------------------------------------------------------------------

    @Test
    fun toggle_longpress_uses_longPressSilenceSecs() {
        assertEquals(
            "TOGGLE + longpress gesture must use longPressSilenceSecs",
            LONG_PRESS_SILENCE,
            select(KlarvoOverlayService.RecordingMode.TOGGLE, "longpress")
        )
    }

    // ---------------------------------------------------------------------------
    // Regression inversion: confirm AUTO does NOT return tapSilenceSecs
    // This test goes RED if the code regresses to the old bubbleTapSilenceSecs path.
    // ---------------------------------------------------------------------------

    @Test
    fun auto_doesNotReturn_tapSilenceSecs() {
        val result = select(KlarvoOverlayService.RecordingMode.AUTO, "tap")
        assert(result != TAP_SILENCE) {
            "REGRESSION: AUTO mode must NOT return tapSilenceSecs ($TAP_SILENCE) — " +
            "got $result. This is the Android silence-field divergence bug re-introduced."
        }
    }

    @Test
    fun autostop_doesNotReturn_tapSilenceSecs() {
        val result = select(KlarvoOverlayService.RecordingMode.AUTOSTOP, "tap")
        assert(result != TAP_SILENCE) {
            "REGRESSION: AUTOSTOP mode must NOT return tapSilenceSecs ($TAP_SILENCE) — " +
            "got $result. This is the Android silence-field divergence bug re-introduced."
        }
    }
}
