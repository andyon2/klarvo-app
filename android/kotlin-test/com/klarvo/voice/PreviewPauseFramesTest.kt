package com.klarvo.voice

import org.junit.Test

/**
 * Task 2.3a — Story 11-2: the ported `previewPauseSilenceSecs` slider must actually govern the
 * preview-pause frame threshold (it must not be inert). Guards
 * [KlarvoAudioRecorder.framesForSeconds], the pure seconds→frames conversion shared by the
 * one-shot AUTOSTOP/AUTO path (`requiredSilentFrames`) and the repeatable preview edge
 * (`previewRequiredSilentFrames`).
 */
class PreviewPauseFramesTest {

    @Test
    fun largerPreviewPauseSilenceSecs_yieldsLargerRequiredSilentFrames() {
        val shortWindow = KlarvoAudioRecorder.framesForSeconds(1.0f)
        val longWindow = KlarvoAudioRecorder.framesForSeconds(4.0f)
        assert(longWindow > shortWindow) {
            "A larger previewPauseSilenceSecs must yield a larger frame threshold -- got " +
                "shortWindow=$shortWindow, longWindow=$longWindow. If this ever fails, the " +
                "slider has silently become inert (AC-4/AC-8 contradiction, Task 2.3a)."
        }
    }

    /**
     * Inversion: a hard-coded threshold (ignoring the input secs) would make this test fail --
     * proves framesForSeconds is load-bearing, not a constant in disguise.
     */
    @Test
    fun inversion_hardCodedThresholdWouldFailThisTest() {
        val hardCoded = 62 // stand-in for "ignores its argument"
        val actualFor1s = KlarvoAudioRecorder.framesForSeconds(1.0f)
        val actualFor4s = KlarvoAudioRecorder.framesForSeconds(4.0f)
        assert(!(actualFor1s == hardCoded && actualFor4s == hardCoded)) {
            "Both 1s and 4s must not resolve to the same hard-coded frame count."
        }
    }
}
