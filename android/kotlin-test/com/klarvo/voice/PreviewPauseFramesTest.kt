package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
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

    // ---------------------------------------------------------------------------
    // AC6 (L1) — exact 31.25 fps (ceil), not truncated integer 31.
    //
    // Independent expected value: ceil(2.0 * 31.25) = ceil(62.5) = 63, matching Rust's
    // frame_ms=32.0-derived fps exactly (src-tauri/src/vad/mod.rs:239-242). The OLD truncated
    // constant (31) would give (2.0 * 31).toInt() = 62 -- one frame short.
    // ---------------------------------------------------------------------------

    @Test
    fun framesForSeconds_2Point0_yields63Frames_ac6() {
        assertEquals(
            "AC6: framesForSeconds(2.0f) must be ceil(2.0 * 31.25) = 63, not the old " +
                "truncated-fps value of 62. If this fails, VAD_FRAMES_PER_SECOND regressed to " +
                "the truncated integer 31.",
            63,
            KlarvoAudioRecorder.framesForSeconds(2.0f)
        )
    }

    /**
     * Inversion: the OLD truncated fps (31, via toInt() truncation instead of ceil at the exact
     * 31.25) would have produced 62, not 63. This pins the exact regression AC6 closes.
     */
    @Test
    fun inversion_oldTruncatedFps_wouldHaveGiven62_ac6() {
        val oldTruncatedResult = (2.0f * 31).toInt() // old: hardcoded int 31, no ceil
        assertEquals(
            "Sanity: the OLD formula (truncated fps=31, no ceil) computes 62, confirming 63 " +
                "(the new correct value) is a real behavior change, not a coincidental match.",
            62,
            oldTruncatedResult
        )
        assert(KlarvoAudioRecorder.framesForSeconds(2.0f) != oldTruncatedResult) {
            "framesForSeconds(2.0f) must NOT equal the old truncated-fps result (62)."
        }
    }

    // ---------------------------------------------------------------------------
    // AC2 (H17) — 200ms-equivalent hangover floor, applied uniformly through framesForSeconds
    // (DECIDED, GATE 1, Andi, 2026-09-09: shared with preview-pause, not autostop-only).
    //
    // Independent expected value: ceil(200ms / 32ms) = ceil(6.25) = 7 frames -- the floor Rust
    // applies via hangover_ms.max(200) to BOTH the autostop/auto config (audio/mod.rs:1072-1076)
    // and the preview-flush config (audio/mod.rs:1180-1184).
    // ---------------------------------------------------------------------------

    @Test
    fun framesForSeconds_verySmallSilenceSecs_stillYieldsFloorOf7Frames_ac2() {
        // 0.05s -> ceil(0.05 * 31.25) = ceil(1.5625) = 2 frames WITHOUT the floor -- well below
        // the 7-frame (200ms-equivalent) floor AC2 requires.
        assertEquals(
            "AC2: framesForSeconds(0.05f) must be floored to 7 frames (200ms-equivalent), not " +
                "the unfloored ceil(0.05 * 31.25) = 2. If this fails, the H17 hangover floor is " +
                "missing or was removed.",
            7,
            KlarvoAudioRecorder.framesForSeconds(0.05f)
        )
    }

    @Test
    fun framesForSeconds_zeroSilenceSecs_stillYieldsFloorOf7Frames_ac2() {
        assertEquals(
            "AC2: framesForSeconds(0.0f) must still be floored to 7 frames, never 0 or 1.",
            7,
            KlarvoAudioRecorder.framesForSeconds(0.0f)
        )
    }

    /**
     * Inversion: without the floor, framesForSeconds(0.05f) would be ceil(0.05*31.25)=2 (or,
     * under the OLD pre-AC6 code, coerceAtLeast(1) would floor it at just 1 frame =~32ms --
     * nowhere near the 200ms Rust requires). This test goes RED if the floor is reverted.
     */
    @Test
    fun inversion_revertingFloor_wouldGiveOnly2Frames_ac2() {
        val unflooredResult = kotlin.math.ceil(0.05f * 31.25f).toInt() // = 2, no floor applied
        assertEquals(2, unflooredResult)
        assert(KlarvoAudioRecorder.framesForSeconds(0.05f) != unflooredResult) {
            "framesForSeconds(0.05f) must NOT equal the unfloored result (2) -- the AC2 floor " +
                "must be active."
        }
    }

    /**
     * AC2's GATE-1 decision: the floor applies uniformly through the SHARED framesForSeconds,
     * so it also governs the preview-pause path (previewRequiredSilentFrames derives from the
     * same function) -- not just autostop. This is proven structurally: both call sites already
     * route through framesForSeconds (see KlarvoAudioRecorder.kt requiredSilentFrames /
     * previewRequiredSilentFrames), and this test confirms the shared function itself enforces
     * the floor for ANY caller, autostop or preview alike.
     */
    @Test
    fun framesForSeconds_floorAppliesUniformly_noAutostopOnlyCarveOut() {
        val smallSecsValues = listOf(0.0f, 0.01f, 0.05f, 0.1f)
        for (secs in smallSecsValues) {
            val frames = KlarvoAudioRecorder.framesForSeconds(secs)
            assertTrue(
                "AC2 GATE-1: framesForSeconds($secs) must be >= 7 (the shared floor) -- got " +
                    "$frames. The floor must apply uniformly, not just to an autostop-specific path.",
                frames >= 7
            )
        }
    }
}
