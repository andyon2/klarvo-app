package com.klarvo.voice

/**
 * Computes the license status on Android by reusing the Rust HMAC/trial logic
 * over JNI -- the SAME `compute_cached_status` the desktop uses at boot. Android
 * never reimplements license math (ADR-0016: a Kotlin reimplementation would
 * itself become a cross-platform drift source, and would leak the HMAC secret
 * into the easily-decompiled DEX instead of keeping it in the .so).
 *
 * Native function: `Java_com_klarvo_voice_LicenseValidator_nativeComputeStatus`
 * in `src-tauri/src/license/jni.rs`, inside `libklarvo_lib.so`.
 *
 * Fail-SAFE: if the native library is unavailable or anything throws, the status
 * is treated as NOT allowed -- a broken bridge denies paid features, never grants.
 */
object LicenseValidator {
    private const val TAG = "LicenseValidator"

    // Set to false permanently if System.loadLibrary fails.
    private var nativeAvailable = false

    init {
        try {
            // Same library as LocalWhisperInference; loading twice is a no-op (JLS).
            System.loadLibrary("klarvo_lib")
            nativeAvailable = true
            KlarvoLogger.i(TAG, "Native library libklarvo_lib loaded for license validation")
        } catch (e: Throwable) {
            KlarvoLogger.e(TAG, "Failed to load native library for license: ${e.javaClass.simpleName}: ${e.message}")
            // nativeAvailable stays false -- computeStatus returns "unlicensed".
        }
    }

    // Must match Java_com_klarvo_voice_LicenseValidator_nativeComputeStatus in Rust.
    // Returns "licensed" | "trial:<until>" | "grace_period:<until>" | "unlicensed".
    private external fun nativeComputeStatus(
        key: String,
        source: String,
        lsInstanceId: String,
        lsLastValidatedAt: Long,
        licenseValidatedAt: Long,
        firstInstallAt: Long
    ): String

    /**
     * Returns the raw status string from Rust, or "unlicensed" if the native
     * bridge is unavailable or throws (fail-safe).
     */
    fun computeStatus(
        key: String,
        source: String,
        lsInstanceId: String,
        lsLastValidatedAt: Long,
        licenseValidatedAt: Long,
        firstInstallAt: Long
    ): String {
        if (!nativeAvailable) return "unlicensed"
        return try {
            nativeComputeStatus(key, source, lsInstanceId, lsLastValidatedAt, licenseValidatedAt, firstInstallAt)
        } catch (e: Throwable) {
            KlarvoLogger.e(TAG, "nativeComputeStatus threw: ${e.javaClass.simpleName}: ${e.message}")
            "unlicensed"
        }
    }

    /**
     * True if the user may use paid features: Licensed, or an unexpired Trial /
     * GracePeriod. Mirrors Rust `is_feature_allowed`. The Rust side already
     * resolves expiry, so an expired trial returns "unlicensed"; the `until > now`
     * re-check here is defense-in-depth against clock skew.
     */
    fun isAllowed(
        key: String,
        source: String,
        lsInstanceId: String,
        lsLastValidatedAt: Long,
        licenseValidatedAt: Long,
        firstInstallAt: Long
    ): Boolean {
        val status = computeStatus(key, source, lsInstanceId, lsLastValidatedAt, licenseValidatedAt, firstInstallAt)
        if (status == "licensed") return true
        val colon = status.indexOf(':')
        if (colon > 0) {
            val prefix = status.substring(0, colon)
            if (prefix == "trial" || prefix == "grace_period") {
                val until = status.substring(colon + 1).toLongOrNull() ?: return false
                val nowSecs = System.currentTimeMillis() / 1000L
                return until > nowSecs
            }
        }
        return false
    }
}
