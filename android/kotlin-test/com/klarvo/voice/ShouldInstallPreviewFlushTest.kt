package com.klarvo.voice

import org.junit.Test

/**
 * Task 2.2 — Story 11-2 (AC-3/AC-4): guard governing whether the repeatable preview-flush
 * callback gets installed at all. HOLD/TOGGLE + enabled -> true; every other combination -> false
 * (Auto/AutoStop never get a preview flush, mirrors desktop FR4 parity; disabled means byte-
 * identical existing behavior, AC-4).
 */
class ShouldInstallPreviewFlushTest {

    private fun check(mode: KlarvoOverlayService.RecordingMode, enabled: Boolean) =
        KlarvoOverlayService.RecordingMode.shouldInstallPreviewFlush(mode, enabled)

    @Test
    fun hold_enabled_true() {
        assert(check(KlarvoOverlayService.RecordingMode.HOLD, true))
    }

    @Test
    fun toggle_enabled_true() {
        assert(check(KlarvoOverlayService.RecordingMode.TOGGLE, true))
    }

    @Test
    fun hold_disabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.HOLD, false))
    }

    @Test
    fun toggle_disabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.TOGGLE, false))
    }

    @Test
    fun autostop_enabled_false() {
        // AC-3 scope guard: Auto/AutoStop never install a preview flush, even when enabled.
        assert(!check(KlarvoOverlayService.RecordingMode.AUTOSTOP, true))
    }

    @Test
    fun auto_enabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.AUTO, true))
    }

    @Test
    fun autostop_disabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.AUTOSTOP, false))
    }

    @Test
    fun auto_disabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.AUTO, false))
    }

    /**
     * Inversion (per DoD): flip one branch (e.g. treat AUTOSTOP like HOLD) and this test would
     * go RED -- proving the guard's mode-check is load-bearing, documented empirically here
     * rather than merely asserted.
     */
    @Test
    fun inversion_autostopMustNotEqualHoldBehavior() {
        val autostopResult = check(KlarvoOverlayService.RecordingMode.AUTOSTOP, true)
        val holdResult = check(KlarvoOverlayService.RecordingMode.HOLD, true)
        assert(autostopResult != holdResult) {
            "AUTOSTOP and HOLD must resolve differently for the same enabled=true input -- " +
                "if they ever match, the mode guard has regressed to ignore RecordingMode entirely."
        }
    }
}
