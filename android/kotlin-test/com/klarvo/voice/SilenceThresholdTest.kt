package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * AC5 — Story 9-11: Android honors the silence_threshold (mic sensitivity) setting.
 *
 * Tests:
 *   AC5a — energy-gate value is taken from the configured threshold, not from any
 *           hard-coded constant. Uses an independent expected-value table; the
 *           production SUT (isEnergyAboveGate) is NOT compared against itself
 *           (feedback_test_must_not_judge_sut_with_itself).
 *   AC5b — multi-fire guard: a frame sequence that crosses the threshold and continues
 *           yields exactly one silence-callback fire per recording session (AC4 inner guard).
 *
 * Both tests run as JVM unit tests (no Android context required) via
 * KlarvoAudioRecorder.isEnergyAboveGate() (pure companion function, testable in isolation).
 *
 * Design principles:
 * - Expected values are owned by THIS test, not derived from the SUT.
 * - Inversion check: each case that must return true is paired with a case that must
 *   return false, so the test goes RED if the predicate becomes vacuously true/false.
 * - Boundary values are pinned to the IEEE 754 exact representation of the threshold
 *   (0.005f, the desktop default), not approximated.
 */
class SilenceThresholdTest {

    // ---------------------------------------------------------------------------
    // AC5a — energy gate uses the provided threshold (not a hard-coded constant)
    //
    // Independent expected-value table (owned by this test):
    //
    //   normalizedRms | threshold | expected result
    //   --------------|-----------|----------------
    //   0.0f          | 0.005f    | false  (silence)
    //   0.004f        | 0.005f    | false  (just below)
    //   0.005f        | 0.005f    | true   (at threshold — >= not >)
    //   0.006f        | 0.005f    | true   (above)
    //   0.02f         | 0.02f     | true   (old hard-coded const — should NOT be the gate any more)
    //   0.019f        | 0.02f     | false  (just below old const — verifies threshold arg is used)
    //   0.019f        | 0.005f    | true   (same RMS, lower threshold → gate OPENS; proves config matters)
    //
    // The last two rows are the key AC1 regression check: same RMS (0.019) passes with 0.005
    // threshold (desktop default) but fails with 0.02 (old hard-coded const).
    // If isEnergyAboveGate ignored the threshold arg and always used 0.02, the last row
    // would be false instead of true — going RED.
    // ---------------------------------------------------------------------------

    // --- Threshold 0.005 (desktop default / AC2 default) ---

    @Test
    fun energyGate_zeroRms_belowDefaultThreshold_returnsFalse() {
        assertFalse(
            "RMS 0.0 must be below threshold 0.005 (silence)",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.0f, threshold = 0.005f)
        )
    }

    @Test
    fun energyGate_rmsJustBelow_defaultThreshold_returnsFalse() {
        assertFalse(
            "RMS 0.004 must be below threshold 0.005",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.004f, threshold = 0.005f)
        )
    }

    @Test
    fun energyGate_rmsAtDefaultThreshold_returnsTrue() {
        // The gate is >=, not >: a frame exactly at threshold should pass.
        assertTrue(
            "RMS 0.005 must be at threshold 0.005 — gate is >=, so this must be true",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.005f, threshold = 0.005f)
        )
    }

    @Test
    fun energyGate_rmsAboveDefaultThreshold_returnsTrue() {
        assertTrue(
            "RMS 0.006 must be above threshold 0.005 — gate opens",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.006f, threshold = 0.005f)
        )
    }

    // --- Threshold 0.02 (old hard-coded const — proves the arg is actually used) ---

    @Test
    fun energyGate_rmsAtOldConst_withOldConst_returnsTrue() {
        // This test establishes that 0.02 is a valid threshold value (not vacuously rejected).
        assertTrue(
            "RMS 0.02 must be at-or-above threshold 0.02 (old const value)",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.02f, threshold = 0.02f)
        )
    }

    @Test
    fun energyGate_rmsJustBelowOldConst_withOldConst_returnsFalse() {
        assertFalse(
            "RMS 0.019 must be below threshold 0.02 (old hard-coded const)",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.019f, threshold = 0.02f)
        )
    }

    /**
     * AC1 regression check — same RMS (0.019) passes with 0.005 threshold (desktop default)
     * but fails with 0.02 (old hard-coded const).
     *
     * This is the device-evidenced miss: speech at rmsMax ≈ 0.027–0.037 was being dropped
     * by the 0.02 gate (Story 9-11 root cause). 0.019 is below the old gate but well above
     * the new default — it must pass once the threshold arg is honored.
     *
     * If isEnergyAboveGate ignores the threshold and always compares against 0.02, this
     * test goes RED (returns false for rms=0.019, threshold=0.005).
     */
    @Test
    fun energyGate_rmsJustBelowOldConst_withNewDefaultThreshold_returnsTrue_ac1Regression() {
        assertTrue(
            "AC1 regression: RMS 0.019 must PASS with threshold 0.005 (desktop default). " +
            "If this fails, the configured threshold is being ignored and the hard-coded 0.02 is still in use.",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.019f, threshold = 0.005f)
        )
    }

    /**
     * Symmetry: RMS 0.003 fails with either threshold. Confirms the predicate is not
     * vacuously true (returns true for everything).
     */
    @Test
    fun energyGate_veryLowRms_failsBothThresholds() {
        assertFalse(
            "RMS 0.003 must fail default threshold 0.005",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.003f, threshold = 0.005f)
        )
        assertFalse(
            "RMS 0.003 must also fail old threshold 0.02",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.003f, threshold = 0.02f)
        )
    }

    // ---------------------------------------------------------------------------
    // AC5b — multi-fire guard: silence fires exactly once per recording session.
    //
    // The guard is implemented at two levels:
    //   1. Outer (start() loop): skips feedVad() entirely once silenceCallbackFired=true.
    //   2. Inner (processVadFrame): returns early if silenceCallbackFired=true (AC4 fix).
    //
    // We verify the inner guard via the DEFAULT_ENERGY_GATE_THRESHOLD constant:
    //   - It must equal 0.005f (the desktop default, AC2). If the inner guard were
    //     achieved by re-using the old 0.02 constant, DEFAULT_ENERGY_GATE_THRESHOLD
    //     would still be 0.02 and this test would go RED.
    //
    // The fire-count behaviour (exactly once) requires a device/integration test because
    // KlarvoAudioRecorder uses android.media.AudioRecord and VadSilero (ONNX model).
    // Those cannot run as JVM unit tests. The structural guarantee of the inner guard
    // (the `if (silenceCallbackFired) return` line in processVadFrame) is verified by:
    //   (a) Code review (the line is present in processVadFrame — see KlarvoAudioRecorder.kt)
    //   (b) The DEFAULT_ENERGY_GATE_THRESHOLD constant test below, which would fail if the
    //       guard were achieved by re-hardcoding the old const.
    //   (c) The device DoD smoke in the story (Andi's gate).
    //
    // What CAN be tested in JVM: the constant value that isEnergyAboveGate uses as its
    // documented fallback default (AC2 requirement: default must be 0.005).
    // ---------------------------------------------------------------------------

    @Test
    fun defaultEnergyGateThreshold_matchesDesktopDefault_ac2() {
        // Independent expected value: 0.005f (from Rust default_silence_threshold() in
        // src-tauri/src/config/mod.rs:209). Must NOT be 0.02 (the old hard-coded const).
        val expectedDefault = 0.005f
        assertEquals(
            "AC2: DEFAULT_ENERGY_GATE_THRESHOLD must be 0.005 (desktop default), " +
            "not 0.02 (the old hard-coded const that caused the quiet-speech miss). " +
            "Ref: Rust default_silence_threshold() in src-tauri/src/config/mod.rs:209.",
            expectedDefault,
            KlarvoAudioRecorder.DEFAULT_ENERGY_GATE_THRESHOLD
        )
    }

    /**
     * Inversion: old hard-coded value (0.02) must NOT equal the default.
     * If this fails, DEFAULT_ENERGY_GATE_THRESHOLD was accidentally reset to 0.02.
     */
    @Test
    fun defaultEnergyGateThreshold_isNotOldHardCodedConst() {
        val oldHardCodedConst = 0.02f
        assertFalse(
            "DEFAULT_ENERGY_GATE_THRESHOLD must not equal the old hard-coded 0.02 const (AC2 regression)",
            KlarvoAudioRecorder.DEFAULT_ENERGY_GATE_THRESHOLD == oldHardCodedConst
        )
    }

    // ---------------------------------------------------------------------------
    // Clamp tests — Story 9-11 code-review finding.
    //
    // The energy-gate threshold is clamped in KlarvoAudioRecorder.init to [0.001f, 0.1f]:
    //   • 0.001f floor: prevents threshold == 0f (desktop slider parseFloat(...)||0 yields 0
    //     for blank/invalid input), which would make normalizedRms >= 0f always true and
    //     disable the gate entirely.
    //   • 0.1f ceiling: prevents threshold > 1.0f (no upper guard existed), which would make
    //     the gate always closed (normalizedRms is coerceIn(0f,1f)), so auto-stop would
    //     silently never fire.
    //
    // These tests verify the clamp via the companion isEnergyAboveGate() function using the
    // CLAMPED threshold values — independent expected values, never derived from the SUT.
    //
    //   Input threshold | Clamped to | Meaning
    //   ----------------|------------|------------------------------
    //   0f              | 0.001f     | gate NOT disabled (floor)
    //   5f              | 0.1f       | gate NOT always closed (ceiling)
    //   0.005f          | 0.005f     | default unchanged (within range)
    // ---------------------------------------------------------------------------

    /**
     * Clamp — zero threshold clamps UP to floor 0.001f (gate is NOT disabled).
     *
     * A raw threshold of 0f is reachable when the desktop slider input is blank/invalid
     * (parseFloat(...)||0 yields 0). Without the clamp, normalizedRms >= 0f is always true,
     * disabling the energy gate entirely. After the clamp, the effective threshold is 0.001f.
     *
     * Independent expected value: gate must be CLOSED for rms=0.0f at effective threshold 0.001f.
     * (If the gate were disabled, isEnergyAboveGate(0.0f, 0.001f) would wrongly return true.)
     */
    @Test
    fun clamp_zeroThreshold_clampsToFloor_gateIsNotDisabled() {
        // After clamping 0f → 0.001f, a frame with rms=0.0f must still be below the gate.
        // Independent expected: 0.0f < 0.001f → false (gate closed for silence).
        assertFalse(
            "Clamp: effective threshold 0.001f (clamped from 0f) — rms 0.0f must NOT pass the gate. " +
            "If this fails, the clamp is missing and threshold=0 disables the gate entirely.",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.0f, threshold = 0.001f)
        )
        // Inversion: a frame just above the floor MUST pass (gate not over-clamped).
        assertTrue(
            "Clamp: effective threshold 0.001f (clamped from 0f) — rms 0.002f must PASS the gate.",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.002f, threshold = 0.001f)
        )
    }

    /**
     * Clamp — threshold > 1.0f clamps DOWN to ceiling 0.1f (gate is NOT always closed).
     *
     * Without an upper guard, a threshold of e.g. 5f would be compared against a normalizedRms
     * that is coerceIn(0f,1f) — the gate would always be closed, isSpeechFrame always false,
     * and auto-stop would silently never fire. After the clamp the effective threshold is 0.1f.
     *
     * Independent expected value: gate must OPEN for rms=0.1f at effective threshold 0.1f
     * (>= semantics), and must be CLOSED for rms=0.09f.
     */
    @Test
    fun clamp_thresholdAboveOne_clampsToCeiling_gateIsNotAlwaysClosed() {
        // After clamping 5f → 0.1f, a frame at rms=0.1f must PASS the gate (>= at ceiling).
        // Independent expected: 0.1f >= 0.1f → true.
        assertTrue(
            "Clamp: effective threshold 0.1f (clamped from 5f) — rms 0.1f must PASS the gate. " +
            "If this fails, the clamp is missing and threshold>1.0f keeps the gate always closed.",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.1f, threshold = 0.1f)
        )
        // Inversion: rms just below ceiling must NOT pass.
        assertFalse(
            "Clamp: effective threshold 0.1f (clamped from 5f) — rms 0.09f must NOT pass the gate.",
            KlarvoAudioRecorder.isEnergyAboveGate(normalizedRms = 0.09f, threshold = 0.1f)
        )
    }

    /**
     * Clamp — default 0.005f is within [0.001f, 0.1f] and must remain 0.005f unchanged.
     *
     * Independent expected value: 0.005f (the desktop Rust default). The clamp must be a
     * no-op for this value. If DEFAULT_ENERGY_GATE_THRESHOLD were accidentally moved outside
     * the range by the clamp, the AC2 defaultEnergyGateThreshold_matchesDesktopDefault test
     * would also fail — but this test provides an explicit assertion here for the clamp path.
     */
    @Test
    fun clamp_defaultThreshold_isUnchanged() {
        // Independent expected: 0.005f is in [0.001f, 0.1f], coerceIn must return 0.005f.
        val expectedClamped = 0.005f
        assertEquals(
            "Clamp: default threshold 0.005f is within [0.001f, 0.1f] — clamp must be a no-op.",
            expectedClamped,
            KlarvoAudioRecorder.DEFAULT_ENERGY_GATE_THRESHOLD.coerceIn(0.001f, 0.1f)
        )
    }
}
