//! JNI bridge for offline license-status computation on Android.
//!
//! Exposes one function to the Kotlin object `com.klarvo.voice.LicenseValidator`:
//!
//! - `nativeComputeStatus(key, source, lsInstanceId, lsLastValidatedAt,
//!    licenseValidatedAt, firstInstallAt): String`
//!
//! It returns the same status strings the desktop Tauri command layer returns
//! (`"licensed"`, `"trial:<until>"`, `"grace_period:<until>"`, `"unlicensed"`),
//! computed by the shared [`super::compute_cached_status`]. Android therefore
//! reuses the exact Rust HMAC/trial logic instead of reimplementing it in
//! Kotlin, so enforcement can never drift from desktop (ADR-0016).
//!
//! ## Why JNI
//!
//! The Android overlay service runs outside the Tauri WebView and cannot call
//! Tauri commands. It calls this `#[no_mangle]` function directly, the same way
//! `stt::jni_bridge` exposes whisper inference.
//!
//! ## Panic / fail-safe
//!
//! JNI functions must not panic into the JVM. Every conversion is checked and
//! any failure returns `"unlicensed"` — fail-SAFE: a broken bridge denies paid
//! features, it never grants them.

#![cfg(target_os = "android")]

use jni::objects::{JClass, JString};
use jni::sys::{jlong, jstring};
use jni::JNIEnv;

use super::{compute_cached_status, status_to_string};

/// Build the `"unlicensed"` Java string used for every error path.
fn unlicensed_jstring(env: &mut JNIEnv) -> jstring {
    env.new_string("unlicensed")
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

/// Computes the cached license status from the fields Android read out of
/// `config.json` (+ its own trial timestamp). Mirrors the desktop boot path.
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_LicenseValidator_nativeComputeStatus(
    mut env: JNIEnv,
    _class: JClass,
    key: JString,
    source: JString,
    ls_instance_id: JString,
    ls_last_validated_at: jlong,
    license_validated_at: jlong,
    first_install_at: jlong,
) -> jstring {
    let key: String = match env.get_string(&key) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("[license_jni] failed to read key: {e}");
            return unlicensed_jstring(&mut env);
        }
    };
    let source: String = match env.get_string(&source) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("[license_jni] failed to read source: {e}");
            return unlicensed_jstring(&mut env);
        }
    };
    let ls_instance_id: String = match env.get_string(&ls_instance_id) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("[license_jni] failed to read ls_instance_id: {e}");
            return unlicensed_jstring(&mut env);
        }
    };

    // `jlong` is i64; the config timestamps are u64 unix seconds. Clamp any
    // negative value to 0 (treated as "never set" by the compute functions).
    let ls_last = ls_last_validated_at.max(0) as u64;
    let lic_validated = license_validated_at.max(0) as u64;
    let first_install = first_install_at.max(0) as u64;

    let status = compute_cached_status(
        &key,
        &source,
        &ls_instance_id,
        ls_last,
        lic_validated,
        first_install,
    );
    let out = status_to_string(&status);

    match env.new_string(&out) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            log::error!("[license_jni] failed to create return string: {e}");
            unlicensed_jstring(&mut env)
        }
    }
}
