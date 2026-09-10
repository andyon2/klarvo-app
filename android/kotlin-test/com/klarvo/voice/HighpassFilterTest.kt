package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.PI
import kotlin.math.sin

/**
 * M3 — Story 7-2, AC3: the ported 85 Hz Butterworth highpass filter ([HighpassFilter]) must
 * behave like Rust's `BiquadHighpass` (`src-tauri/src/vad/mod.rs:114-184`) -- reject bass,
 * pass speech-band frequencies -- and must actually sit in the VAD-gate RMS/Silero path, not
 * just exist in the file.
 *
 * The first two tests are a direct port of Rust's own reference unit tests
 * (`test_biquad_highpass_attenuates_dc`, `test_biquad_highpass_passes_high_frequency`,
 * `vad/mod.rs:622-658`) -- same cutoff (85 Hz), same sample rate (16 kHz), same thresholds.
 * These anchor to filter theory (an ideal highpass fully blocks DC and fully passes high
 * frequencies), not to "whatever this implementation happens to return."
 *
 * The remaining tests exercise the real production seams used by
 * [KlarvoAudioRecorder.processVadFrame] -- [HighpassFilter], [KlarvoAudioRecorder.calculateRmsFloat]
 * and [KlarvoAudioRecorder.isEnergyAboveGate] -- wired together the same way, proving AC3's claim
 * ("a bass-tone-only synthetic frame should fail the energy gate after filtering even if its raw
 * RMS would pass") against the real functions, not a parallel reimplementation.
 */
class HighpassFilterTest {

    private val sampleRateHz = 16000f

    /**
     * Production cutoff constant ([KlarvoAudioRecorder.HIGHPASS_CUTOFF_HZ]) -- NOT a test-local
     * `85f` duplicate. Filter instances below are built with this so a drifted production value
     * (e.g. 300 Hz, which the pre-fix tests couldn't distinguish from 85 Hz) is caught by
     * [cutoff_isAt85Hz_minus3dbCorner] below (Story 7-2 review finding).
     */
    private val cutoffHz = KlarvoAudioRecorder.HIGHPASS_CUTOFF_HZ

    // ---------------------------------------------------------------------------
    // Direct port of Rust's BiquadHighpass reference tests (vad/mod.rs:622-658).
    // ---------------------------------------------------------------------------

    @Test
    fun highpass_attenuatesDc_toNearZero() {
        val hp = HighpassFilter(cutoffHz, sampleRateHz)
        // Prime the filter with DC to reach steady state (mirrors Rust's 1000-sample priming).
        repeat(1000) { hp.process(1.0f) }
        val out = hp.process(1.0f)
        assertTrue(
            "Highpass filter must attenuate DC to near zero, got $out (Rust reference: < 0.01)",
            kotlin.math.abs(out) < 0.01f
        )
    }

    @Test
    fun highpass_passesHighFrequency_nearUnityGain() {
        val hp = HighpassFilter(cutoffHz, sampleRateHz)
        val freq = 1000f
        val n = 2000
        var peak = 0f
        for (i in 0 until n) {
            val x = sin(2.0 * PI * freq * i / sampleRateHz).toFloat()
            val y = hp.process(x)
            if (i > 500) peak = maxOf(peak, kotlin.math.abs(y))
        }
        assertTrue(
            "1 kHz signal must pass through highpass with amplitude > 0.95, got peak=$peak " +
                "(Rust reference: vad/mod.rs test_biquad_highpass_passes_high_frequency)",
            peak > 0.95f
        )
    }

    /**
     * Story 7-2 review finding: the two tests above (DC blocked, 1 kHz passed) are satisfied by
     * ANY cutoff roughly in [40, 300] Hz -- they don't pin the exact 85 Hz value AC3 requires.
     * This test drives a filter built from the PRODUCTION [cutoffHz] constant with a probe tone
     * at the literal AC3-specified 85 Hz (an independent value, not derived from [cutoffHz]) and
     * asserts the Butterworth -3dB corner gain (1/sqrt(2) ~= 0.7071, +/-0.02). If
     * [KlarvoAudioRecorder.HIGHPASS_CUTOFF_HZ] drifted (e.g. to 300 Hz), the 85 Hz probe would sit
     * deep in that filter's stopband and measure a much lower gain, failing this assertion.
     */
    @Test
    fun cutoff_isAt85Hz_minus3dbCorner() {
        val hp = HighpassFilter(cutoffHz, sampleRateHz)
        val probeFreqHz = 85f
        val n = 8000
        val settleSamples = 4000
        var peak = 0f
        for (i in 0 until n) {
            val x = sin(2.0 * PI * probeFreqHz * i / sampleRateHz).toFloat()
            val y = hp.process(x)
            if (i > settleSamples) peak = maxOf(peak, kotlin.math.abs(y))
        }
        val expectedGain = 0.7071067811865476f
        assertTrue(
            "At production HIGHPASS_CUTOFF_HZ=$cutoffHz Hz, an 85 Hz probe tone must show the " +
                "Butterworth -3dB corner gain ~$expectedGain (+/-0.02), got peak=$peak. A drifted " +
                "cutoff (e.g. 300 Hz) would fail this -- 85 Hz would sit in that filter's stopband.",
            kotlin.math.abs(peak - expectedGain) < 0.02f
        )
    }

    // ---------------------------------------------------------------------------
    // AC3 behavioral test: bass-tone fails the energy gate after filtering even though its
    // raw RMS would pass; a speech-band tone is attenuated negligibly (does not falsely close
    // the gate). Uses the REAL production functions (HighpassFilter, calculateRmsFloat,
    // isEnergyAboveGate) wired the same way processVadFrame wires them, not a reimplementation.
    //
    // 20 Hz is used as the bass tone (well below the 85 Hz cutoff -- Butterworth 2nd-order
    // highpass theory: |H(20/85)| ~= 0.055, i.e. ~5.5% passthrough at steady state) -- a more
    // decisive example than the AC text's illustrative "e.g. 60 Hz", chosen so the gate-flip
    // assertion has a comfortable margin rather than sitting near the filter's -3dB corner.
    // ---------------------------------------------------------------------------

    /**
     * Generates [seconds] of a mono sine wave at [freqHz], peak amplitude [amplitudeShortScale]
     * (in raw Short/i16 scale, e.g. 3277 =~ 0.1 normalized), at 16 kHz.
     */
    private fun sineToneShorts(freqHz: Float, amplitudeShortScale: Float, seconds: Float): ShortArray {
        val n = (sampleRateHz * seconds).toInt()
        return ShortArray(n) { i ->
            (amplitudeShortScale * sin(2.0 * PI * freqHz * i / sampleRateHz)).toInt().toShort()
        }
    }

    /**
     * Feeds [samples] through the REAL production chain -- [KlarvoAudioRecorder.vadGateFilteredFrame]
     * (normalize by [KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR] THEN highpass-filter, the
     * exact seam [KlarvoAudioRecorder.processVadFrame] uses) -- in 512-sample frames (mirroring
     * [KlarvoAudioRecorder]'s VAD_FRAME_SIZE). Returns (rawNormalizedRms, filteredRms) computed
     * over the TAIL of the signal only (skips the first 8 frames =~ 256 ms so the filter's delay
     * line has settled, mirroring Rust's own settle-then-measure test pattern). The raw side has
     * no production equivalent post-Story-7-2 (the VAD gate is always filtered now) so it stays a
     * plain normalize-only loop here, purely to compute the "what the old unfiltered code would
     * have seen" comparison value for the inversion checks below.
     */
    private fun rawAndFilteredRms(samples: ShortArray): Pair<Float, Float> {
        val filter = HighpassFilter(cutoffHz, sampleRateHz)
        val frameSize = 512
        val settleFrames = 8
        val scratch = FloatArray(frameSize)
        val rawTail = ArrayList<Float>()
        val filteredTail = ArrayList<Float>()

        var frameIndex = 0
        var pos = 0
        while (pos + frameSize <= samples.size) {
            val frame = samples.copyOfRange(pos, pos + frameSize)
            val filteredFrame = KlarvoAudioRecorder.vadGateFilteredFrame(frame, frameSize, filter, scratch)
            if (frameIndex >= settleFrames) {
                for (i in 0 until frameSize) {
                    rawTail.add(frame[i] / KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR)
                    filteredTail.add(filteredFrame[i])
                }
            }
            pos += frameSize
            frameIndex++
        }

        val rawRms = KlarvoAudioRecorder.calculateRmsFloat(rawTail.toFloatArray(), rawTail.size)
        val filteredRms = KlarvoAudioRecorder.calculateRmsFloat(filteredTail.toFloatArray(), filteredTail.size)
        return rawRms to filteredRms
    }

    @Test
    fun bassTone_passesRawGate_butFailsFilteredGate() {
        // Peak amplitude 3277 (~0.1 normalized) -- a plausible "loud" mic signal.
        val samples = sineToneShorts(freqHz = 20f, amplitudeShortScale = 3277f, seconds = 1.0f)
        val (rawRms, filteredRms) = rawAndFilteredRms(samples)

        val threshold = 0.02f // a representative tuned threshold, above the desktop default 0.005f.

        assertTrue(
            "Raw (unfiltered) RMS of a 20 Hz bass tone at 0.1 amplitude must PASS the energy " +
                "gate at threshold=$threshold -- got rawRms=$rawRms. If this fails, the fixture " +
                "amplitude needs adjusting, independent of the filter under test.",
            KlarvoAudioRecorder.isEnergyAboveGate(rawRms, threshold)
        )
        assertFalse(
            "AC3: after highpass filtering, the SAME 20 Hz bass tone must FAIL the energy gate " +
                "at threshold=$threshold -- got filteredRms=$filteredRms (raw was $rawRms). If " +
                "this fails, the highpass filter is not actually in the RMS path.",
            KlarvoAudioRecorder.isEnergyAboveGate(filteredRms, threshold)
        )
    }

    @Test
    fun speechBandTone_attenuatedNegligibly_stillPassesGate() {
        // 1 kHz is well within the speech band (300-3000 Hz per AC3's test note) and far above
        // the 85 Hz cutoff -- must pass through with only negligible attenuation.
        val samples = sineToneShorts(freqHz = 1000f, amplitudeShortScale = 3277f, seconds = 1.0f)
        val (rawRms, filteredRms) = rawAndFilteredRms(samples)

        val threshold = 0.02f

        assertTrue(
            "AC3: a speech-band (1 kHz) tone must still PASS the energy gate after filtering -- " +
                "got filteredRms=$filteredRms (raw was $rawRms). The highpass filter must not " +
                "meaningfully attenuate speech-band content.",
            KlarvoAudioRecorder.isEnergyAboveGate(filteredRms, threshold)
        )
        assertTrue(
            "Speech-band attenuation must be negligible: filteredRms ($filteredRms) must stay " +
                "above 95% of rawRms ($rawRms).",
            filteredRms > rawRms * 0.95f
        )
    }

    /**
     * Inversion: without the highpass filter (i.e. feeding the RAW, unfiltered frame straight
     * into calculateRmsFloat/isEnergyAboveGate, as the pre-Story-7-2 code did), the 20 Hz bass
     * tone would WRONGLY pass the gate. This is the exact regression AC1/AC3 close.
     */
    @Test
    fun inversion_unfilteredBassTone_wouldWronglyPassTheGate() {
        val samples = sineToneShorts(freqHz = 20f, amplitudeShortScale = 3277f, seconds = 1.0f)
        val (rawRms, _) = rawAndFilteredRms(samples)
        assertTrue(
            "Sanity/inversion: the OLD unfiltered code path would have let this bass tone pass " +
                "the gate (rawRms=$rawRms >= 0.02) -- proving the filter, not fixture choice, is " +
                "what makes bassTone_passesRawGate_butFailsFilteredGate's filtered case fail.",
            KlarvoAudioRecorder.isEnergyAboveGate(rawRms, 0.02f)
        )
    }

    // ---------------------------------------------------------------------------
    // Story 7-2 round-2 review finding: round 1's seam (vadGateFilteredFrame) covers only the
    // filter+RMS half of the gate decision. These tests drive KlarvoAudioRecorder.vadGateDecision
    // -- the seam that ALSO owns the Silero call -- directly, with a spy `isSpeech` lambda, and
    // assert it receives the FILTERED frame, not the raw one.
    //
    // WHAT THEY CATCH: reverting `isSpeech(filteredFrame)` to `isSpeech(frame)` INSIDE the seam,
    // a wrong `normalizedRms`, and a broken energy-gate AND-combination.
    //
    // KNOWN REMAINING GAP -- do not re-inflate this claim (story 7-8, closing 7-2 round-3 finding
    // R3-P1): these tests do NOT catch a DROPPED `vadGateDecision(...)` call in `processVadFrame`.
    // `processVadFrame` is private, stateful, and referenced by no test, so deleting the seam call
    // there -- or rewriting its lambda to `{ _ -> vad?.isSpeech(frame) == true }`, since the
    // `short[]` overload exists and `frame` is in scope -- still compiles and leaves every test
    // green. Closing that gap needs processVadFrame's per-frame body made callable from a test;
    // that is deliberately not done here. Three places previously claimed the drop WAS caught.
    // ---------------------------------------------------------------------------

    @Test
    fun vadGateDecision_feedsFilteredFrameToIsSpeech_notRawFrame() {
        val filter = HighpassFilter(cutoffHz, sampleRateHz)
        val scratch = FloatArray(512)
        // A DC-like constant frame (~0.1 normalized amplitude). Within a single fresh-filter frame
        // the 85 Hz highpass attenuates this to ~0.0144 -- about 14.4 % of the raw 0.10001, a 6.9x
        // margin. (R3-P4: an earlier comment said "drives this toward zero", which overstated it;
        // the margin is large but the residual is not near zero.) Caveat: the DC choice makes THIS
        // test insensitive to HIGHPASS_CUTOFF_HZ -- it passes at 20, 85 and 300 Hz alike. The
        // cutoff value itself is pinned by cutoff_isAt85Hz_minus3dbCorner above, not here.
        val amplitudeShortScale: Short = 3277
        val frame = ShortArray(512) { amplitudeShortScale }
        var capturedFrame: FloatArray? = null

        val result = KlarvoAudioRecorder.vadGateDecision(
            frame, frame.size, filter, scratch, threshold = 0f
        ) { f ->
            capturedFrame = f.copyOf()
            true
        }

        val rawNormalizedRms = KlarvoAudioRecorder.calculateRmsFloat(
            FloatArray(frame.size) { frame[it] / KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR },
            frame.size
        )
        val captured = capturedFrame ?: error("isSpeech was never called")
        val capturedRms = KlarvoAudioRecorder.calculateRmsFloat(captured, captured.size)

        assertTrue(
            "vadGateDecision must pass the FILTERED frame to isSpeech, not the raw one -- " +
                "captured RMS ($capturedRms) must be well below the raw normalized RMS " +
                "($rawNormalizedRms). If this fails, isSpeech(filteredFrame) was reverted to " +
                "isSpeech(frame).",
            capturedRms < rawNormalizedRms * 0.5f
        )
        // R3-P3 lower bound: without this, an all-zero captured buffer (e.g. `out` never written,
        // or length 0) would satisfy the upper bound above and pass while proving nothing.
        assertTrue(
            "captured RMS ($capturedRms) must be a real, non-degenerate signal -- an all-zero " +
                "buffer also satisfies the '< 0.5x raw' bound above",
            capturedRms > 1e-4f
        )
        // R3-P3: the returned VadGateResult was previously discarded entirely. normalizedRms must
        // be the FILTERED rms the gate actually measured, not the raw one.
        assertEquals(
            "VadGateResult.normalizedRms must be the filtered RMS the energy gate measured",
            capturedRms.toDouble(),
            result.normalizedRms.toDouble(),
            1e-6
        )
        assertTrue("the spy verdict must be surfaced as vadSpeech", result.vadSpeech)
        assertTrue(
            "threshold=0f opens the energy gate and the spy says speech, so the combined " +
                "decision must be true",
            result.isSpeechFrame
        )
    }

    @Test
    fun vadGateDecision_combinesEnergyGateAndVadWithAnd_notOr() {
        // R3-P3: pins `energyAboveGate && vadSpeech`. The older test passed threshold = 0f, which
        // made energyAboveGate trivially true -- so changing the combination to `vadSpeech` alone,
        // or to `||`, stayed green. Both operands are exercised here.
        val frame = ShortArray(512) { 3277 }

        // Energy gate CLOSED (threshold above any achievable normalized RMS), VAD says speech.
        // `&&` -> false; `||` or `vadSpeech`-alone -> true.
        val gateClosed = KlarvoAudioRecorder.vadGateDecision(
            frame, frame.size, HighpassFilter(cutoffHz, sampleRateHz), FloatArray(512), threshold = 1f
        ) { true }
        assertTrue("the spy must still report speech", gateClosed.vadSpeech)
        assertFalse(
            "a closed energy gate must veto the frame even when the VAD says speech " +
                "(energyAboveGate && vadSpeech)",
            gateClosed.isSpeechFrame
        )

        // Energy gate OPEN, VAD says NOT speech. `&&` -> false; `||` -> true.
        val vadSilent = KlarvoAudioRecorder.vadGateDecision(
            frame, frame.size, HighpassFilter(cutoffHz, sampleRateHz), FloatArray(512), threshold = 0f
        ) { false }
        assertFalse("the VAD verdict must be surfaced as-is", vadSilent.vadSpeech)
        assertFalse(
            "an open energy gate must not by itself mark the frame as speech",
            vadSilent.isSpeechFrame
        )
    }

    @Test
    fun inversion_bypassedHighpass_makesTheFilteredFrameAssertionFail() {
        // R3-P4: a GENUINE inversion, replacing a tautology. The previous version never touched
        // HighpassFilter or vadGateDecision -- it recomputed 3277/32767 and asserted it was
        // > 0.09f, i.e. a statement about integer division and calculateRmsFloat, both of which the
        // test above already depends on. It could not go red if the filter were bypassed.
        //
        // This drives the REAL seam with a near-all-pass filter (1 Hz cutoff): over one 512-sample
        // frame at 16 kHz the DC transient has barely decayed, so ~87 % of the raw amplitude
        // survives. It then shows the discriminating assertion from
        // vadGateDecision_feedsFilteredFrameToIsSpeech_notRawFrame FAILS under that bypass --
        // proving that assertion is sensitive to the filter actually doing its job, not to the
        // signal choice.
        val frame = ShortArray(512) { 3277 }
        var capturedFrame: FloatArray? = null
        KlarvoAudioRecorder.vadGateDecision(
            frame, frame.size, HighpassFilter(1f, sampleRateHz), FloatArray(512), threshold = 0f
        ) { f ->
            capturedFrame = f.copyOf()
            true
        }
        val rawNormalizedRms = KlarvoAudioRecorder.calculateRmsFloat(
            FloatArray(frame.size) { frame[it] / KlarvoAudioRecorder.VAD_RMS_NORMALIZATION_DIVISOR },
            frame.size
        )
        val bypassedRms = KlarvoAudioRecorder.calculateRmsFloat(
            capturedFrame ?: error("isSpeech was never called"),
            frame.size
        )
        assertFalse(
            "with the highpass effectively bypassed (1 Hz cutoff) the captured RMS ($bypassedRms) " +
                "must NOT satisfy the '< 0.5x raw ($rawNormalizedRms)' assertion -- if it does, " +
                "that assertion no longer discriminates a filtered frame from a raw one",
            bypassedRms < rawNormalizedRms * 0.5f
        )
        // The stated ~87 % is pinned with a real band, not merely with the complement of the
        // assertion above (7-8 review round 1: `assertTrue(x > y * 0.5f)` directly after
        // `assertFalse(x < y * 0.5f)` differs only at exact equality and so cannot fail).
        // Computed from the RBJ biquad coefficients at fc = 1 Hz, fs = 16 kHz over one
        // 512-sample DC frame: 0.086799 / 0.100009 = 0.8679. The band is wide enough for
        // f32 rounding and narrow enough that a real cutoff (85 Hz -> ~14 %) falls out of it.
        val survivingFraction = bypassedRms / rawNormalizedRms
        assertTrue(
            "the ~87 % figure this test's comment states must hold: expected 0.85..0.89 of the " +
                "raw amplitude to survive a 1 Hz cutoff, got $survivingFraction " +
                "(bypassedRms=$bypassedRms, rawNormalizedRms=$rawNormalizedRms)",
            survivingFraction in 0.85f..0.89f
        )
    }
}
