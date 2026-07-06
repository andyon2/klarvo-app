package com.klarvo.voice

import org.junit.Test

/**
 * Code-review fix F2 (2026-07-01) -- Story 11-2: guard governing whether
 * [ListeningPanelView.applyAppearance] may run at all. Restyling the panel's stock look
 * (translucent bg, teal border, font size/family) must be gated on the user having opted into
 * Live Preview (AC-4: byte-identical to pre-11-2 behavior when disabled).
 */
class ShouldApplyPreviewAppearanceTest {

    @Test
    fun enabled_true() {
        assert(KlarvoOverlayService.shouldApplyPreviewAppearance(true))
    }

    @Test
    fun disabled_false() {
        assert(!KlarvoOverlayService.shouldApplyPreviewAppearance(false))
    }

    /**
     * Inversion (per DoD): a guard that always returns true (i.e. ignores its argument, which is
     * exactly the pre-fix bug -- `applyAppearance` was called unconditionally) would make this
     * test go RED -- proving the gate is load-bearing rather than a rubber-stamp.
     */
    @Test
    fun inversion_alwaysTrueWouldFailThisTest() {
        val alwaysTrue = true // stand-in for "ignores livePreviewEnabled entirely" (the F2 bug)
        val disabledResult = KlarvoOverlayService.shouldApplyPreviewAppearance(false)
        assert(disabledResult != alwaysTrue) {
            "shouldApplyPreviewAppearance(false) must be false -- if it ever resolves to true, " +
                "the F2 regression (unconditional applyAppearance call) is back."
        }
    }
}
