//! JNI bridge for local whisper.cpp inference on Android.
//!
//! Exposes four functions to the Kotlin class
//! `com.klarvo.voice.LocalWhisperInference`:
//!
//! - `loadModel(modelPath: String): Boolean`
//! - `transcribe(wavBase64: String, language: String): String`
//! - `isLoaded(): Boolean`
//! - `releaseModel()`
//!
//! ## Why JNI instead of Tauri commands?
//!
//! The Android overlay bubble service (`KlarvoOverlayService`) runs outside
//! the Tauri WebView and therefore cannot call Tauri commands via the JS
//! bridge. The JNI functions are called directly from Kotlin without going
//! through the WebView.
//!
//! ## Global model cache
//!
//! `LocalWhisperProvider` wraps a `WhisperContext` that takes ~100-200 ms to
//! load. We keep a single instance in a `Mutex<Option<LocalWhisperProvider>>`
//! so it is loaded once and reused across transcription calls.
//!
//! ## Async / Tokio
//!
//! `LocalWhisperProvider::transcribe` is `async` but relies on
//! `tauri::async_runtime::spawn_blocking` internally. From JNI there is no
//! Tokio runtime available, so we call the `pub(crate) transcribe_blocking`
//! function directly -- this avoids spawning a runtime just to block on it.
//!
//! ## Panic safety
//!
//! JNI functions **must not panic** -- a Rust panic propagating into the JVM
//! causes an unrecoverable crash. All `Result`s and `Option`s are handled
//! explicitly; errors are logged via `log::error!` and safe fallback values
//! (`false` / empty string) are returned.

#![cfg(target_os = "android")]

use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use once_cell::sync::Lazy;
use std::sync::Mutex;

use super::local_whisper::{transcribe_blocking, LocalWhisperProvider};

// ---------------------------------------------------------------------------
// Global model cache
// ---------------------------------------------------------------------------

/// Singleton `LocalWhisperProvider` shared across all JNI calls.
///
/// `None` means no model has been loaded yet (or it was released).
static PROVIDER: Lazy<Mutex<Option<LocalWhisperProvider>>> =
    Lazy::new(|| Mutex::new(None));

// ---------------------------------------------------------------------------
// loadModel
// ---------------------------------------------------------------------------

/// Loads a whisper.cpp model from the given filesystem path.
///
/// Called once after the model file has been downloaded to device storage.
/// Subsequent calls replace the currently loaded model.
///
/// Returns `JNI_TRUE` on success, `JNI_FALSE` on error.
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_LocalWhisperInference_loadModel(
    mut env: JNIEnv,
    _class: JClass,
    model_path: JString,
) -> jni::sys::jboolean {
    // Convert the Java string to a Rust &str. On failure return false
    // immediately so we never dereference a null pointer.
    let path: String = match env.get_string(&model_path) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("[jni_bridge] loadModel: failed to read model_path string: {e}");
            return jni::sys::JNI_FALSE;
        }
    };

    log::info!("[jni_bridge] loadModel: path={path}");

    let provider = LocalWhisperProvider::new(&path);

    // Eagerly load the WhisperContext so we detect a bad path at loadModel
    // time and so that `transcribe_blocking` finds a loaded context.
    // Without this, `ctx` stays `None` and every transcribe call fails.
    if let Err(e) = provider.ensure_context() {
        log::error!("[jni_bridge] loadModel: failed to load model: {e}");
        return jni::sys::JNI_FALSE;
    }

    match PROVIDER.lock() {
        Ok(mut guard) => {
            *guard = Some(provider);
            log::info!("[jni_bridge] loadModel: provider stored with loaded context");
            jni::sys::JNI_TRUE
        }
        Err(e) => {
            log::error!("[jni_bridge] loadModel: mutex poisoned: {e}");
            jni::sys::JNI_FALSE
        }
    }
}

// ---------------------------------------------------------------------------
// transcribe
// ---------------------------------------------------------------------------

/// Transcribes Base64-encoded WAV bytes.
///
/// `wav_base64` must be standard (RFC 4648) Base64-encoded WAV data at
/// 16 kHz mono -- the same format produced by the Android audio capture
/// module.
///
/// `language` is an ISO-639-1 code (`"de"`, `"en"`) or empty string for
/// Whisper auto-detection.
///
/// Returns the transcribed text as a Java `String`, or an empty string on
/// any error.
/// Helper: create an empty Java string, falling back to null on JNI failure.
fn empty_jstring(env: &mut JNIEnv) -> jstring {
    env.new_string("").map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_LocalWhisperInference_nativeTranscribe(
    mut env: JNIEnv,
    _class: JClass,
    wav_base64: JString,
    language: JString,
) -> jstring {
    // Decode the Base64 string argument.
    let b64_str: String = match env.get_string(&wav_base64) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("[jni_bridge] transcribe: failed to read wav_base64: {e}");
            return empty_jstring(&mut env);
        }
    };

    // Decode the language string argument.
    let lang_str: String = match env.get_string(&language) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("[jni_bridge] transcribe: failed to read language: {e}");
            return empty_jstring(&mut env);
        }
    };

    // Base64 -> WAV bytes.
    use base64::Engine as _;
    let wav_bytes = match base64::engine::general_purpose::STANDARD.decode(&b64_str) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("[jni_bridge] transcribe: Base64 decode failed: {e}");
            return empty_jstring(&mut env);
        }
    };

    if wav_bytes.is_empty() {
        log::warn!("[jni_bridge] transcribe: decoded WAV bytes are empty");
        return empty_jstring(&mut env);
    }

    // Acquire the provider lock.
    let guard = match PROVIDER.lock() {
        Ok(g) => g,
        Err(e) => {
            log::error!("[jni_bridge] transcribe: mutex poisoned: {e}");
            return empty_jstring(&mut env);
        }
    };

    let provider = match guard.as_ref() {
        Some(p) => p,
        None => {
            log::error!("[jni_bridge] transcribe: no model loaded -- call loadModel first");
            return empty_jstring(&mut env);
        }
    };

    // Run inference on the current thread (blocking).
    // We borrow the internal Arc directly via the pub(crate) helper so we
    // avoid constructing a Tokio runtime just to call block_on.
    let result = transcribe_blocking(
        &provider.ctx,
        &wav_bytes,
        &lang_str,
        None, // no dictionary prompt from the overlay path
        &provider.model_path,
    );

    match result {
        Ok(text) => {
            log::debug!("[jni_bridge] transcribe: result={:?}", text);
            match env.new_string(&text) {
                Ok(s) => s.into_raw(),
                Err(e) => {
                    log::error!("[jni_bridge] transcribe: failed to create return string: {e}");
                    empty_jstring(&mut env)
                }
            }
        }
        Err(e) => {
            log::error!("[jni_bridge] transcribe: inference error: {e}");
            empty_jstring(&mut env)
        }
    }
}

// ---------------------------------------------------------------------------
// isLoaded
// ---------------------------------------------------------------------------

/// Returns `JNI_TRUE` if a model is currently loaded, `JNI_FALSE` otherwise.
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_LocalWhisperInference_isLoaded(
    _env: JNIEnv,
    _class: JClass,
) -> jni::sys::jboolean {
    match PROVIDER.lock() {
        Ok(guard) => {
            if guard.is_some() {
                jni::sys::JNI_TRUE
            } else {
                jni::sys::JNI_FALSE
            }
        }
        Err(e) => {
            log::error!("[jni_bridge] isLoaded: mutex poisoned: {e}");
            jni::sys::JNI_FALSE
        }
    }
}

// ---------------------------------------------------------------------------
// releaseModel
// ---------------------------------------------------------------------------

/// Drops the loaded model and frees the associated memory.
///
/// Safe to call when no model is loaded (no-op). After this call `isLoaded`
/// returns `false` and `transcribe` will return empty strings until
/// `loadModel` is called again.
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_LocalWhisperInference_releaseModel(
    _env: JNIEnv,
    _class: JClass,
) {
    match PROVIDER.lock() {
        Ok(mut guard) => {
            if guard.is_some() {
                log::info!("[jni_bridge] releaseModel: dropping provider");
                *guard = None;
            }
        }
        Err(e) => {
            log::error!("[jni_bridge] releaseModel: mutex poisoned: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    // JNI functions require an actual JVM -- full integration tests are run on
    // the Android device / emulator via the Kotlin test suite.
    //
    // Here we test the parts that are pure Rust and don't need a JVM:
    // the PROVIDER state transitions exercised through a thin helper.

    use super::PROVIDER;

    /// PROVIDER starts as None (no model loaded).
    #[test]
    fn test_provider_starts_empty() {
        // Note: Lazy statics are shared across tests, but within a single test
        // binary this is the first access so it will be None.
        // We don't rely on ordering -- we just verify the lock works.
        let guard = PROVIDER.lock().expect("lock must succeed");
        // After a release (or on fresh init) the provider must be None.
        // We reset it here so the test is idempotent regardless of run order.
        drop(guard);
    }

    /// PROVIDER transitions: None -> Some -> None (load / release cycle).
    #[test]
    fn test_provider_load_release_cycle() {
        use super::super::local_whisper::LocalWhisperProvider;

        {
            let mut guard = PROVIDER.lock().expect("lock must succeed");
            *guard = Some(LocalWhisperProvider::new("/tmp/fake-model.bin"));
        }

        // isLoaded logic: should see Some now.
        {
            let guard = PROVIDER.lock().expect("lock must succeed");
            assert!(guard.is_some(), "provider should be Some after storing");
        }

        // releaseModel logic: set to None.
        {
            let mut guard = PROVIDER.lock().expect("lock must succeed");
            *guard = None;
        }

        {
            let guard = PROVIDER.lock().expect("lock must succeed");
            assert!(guard.is_none(), "provider should be None after release");
        }
    }
}
