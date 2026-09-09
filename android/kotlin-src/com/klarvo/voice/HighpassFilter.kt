package com.klarvo.voice

import kotlin.math.cos
import kotlin.math.sin
import kotlin.math.sqrt

/**
 * Ports Rust's 2nd-order Butterworth highpass biquad filter (`BiquadHighpass`,
 * `src-tauri/src/vad/mod.rs:114-184`) so the Android live VAD gate rejects bass bleed
 * (e.g. headphone music leaking into the mic) before RMS/energy-gate computation and
 * Silero inference -- parity fix for row M3, Story 7-2 (AC3).
 *
 * Direct Form I biquad, Q = 1/sqrt(2) (maximally flat Butterworth response). The filter
 * has memory (a delay line) -- call [reset] once per new recording session, never per-frame,
 * mirroring Rust's `SileroVad`-owned filter instance lifecycle (`vad/mod.rs:292-296`).
 */
class HighpassFilter(cutoffHz: Float, sampleRateHz: Float) {

    // Feed-forward / feed-back coefficients, normalised by (1 + alpha) -- see vad/mod.rs:134-149.
    private val b0: Float
    private val b1: Float
    private val b2: Float
    private val a1: Float
    private val a2: Float

    // Direct Form I delay line.
    private var x1 = 0f // x[n-1]
    private var x2 = 0f // x[n-2]
    private var y1 = 0f // y[n-1]
    private var y2 = 0f // y[n-2]

    init {
        val omega = (2.0 * Math.PI * cutoffHz / sampleRateHz).toFloat()
        val q = (sqrt(2.0) / 2.0).toFloat() // 0.7071...
        val alpha = sin(omega) / (2f * q)
        val cosOmega = cos(omega)
        val norm = 1f + alpha // common denominator

        val b0Value = (1f + cosOmega) / 2f / norm
        b0 = b0Value
        b1 = -(1f + cosOmega) / norm
        b2 = b0Value
        a1 = -2f * cosOmega / norm
        a2 = (1f - alpha) / norm
    }

    /** Processes a single sample and returns the filtered output. */
    fun process(x: Float): Float {
        val y = b0 * x + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2
        x2 = x1
        x1 = x
        y2 = y1
        y1 = y
        return y
    }

    /** Resets the filter delay line to zero (call once per new recording session). */
    fun reset() {
        x1 = 0f
        x2 = 0f
        y1 = 0f
        y2 = 0f
    }
}
