//! Data-plane: raw-jni callback invocation.
//!
//! The Kotlin shell registers a listener object via a raw JNI call; Rust stores
//! a `Global<JObject>` and invokes the listener's `onLevel(F, J)V` method for
//! each audio-level event produced by the tokio broadcast pipeline.

use std::sync::Mutex;

use jni::errors::{Result as JniResult, ThrowRuntimeExAndDefault};
use jni::objects::{Global, JClass, JObject, JValue};
use jni::{Env, EnvUnowned, JavaVM, jni_sig, jni_str};

use crate::audio_level::AudioLevel;

static LISTENER: Mutex<Option<Global<JObject<'static>>>> = Mutex::new(None);

/// Register the Kotlin-side listener. Stores a global reference; the previous
/// listener (if any) is dropped (its `DeleteGlobalRef` fires in `Global::drop`).
///
/// Public so integration-tests can register a listener without going through
/// the FFI symbol path. Production path is the `Java_..._registerAudioLevelListener`
/// JNI entry point below.
pub fn register_listener(env: &mut Env, listener: &JObject) -> JniResult<()> {
    let global = env.new_global_ref(listener)?;
    *LISTENER.lock().unwrap() = Some(global);
    Ok(())
}

pub fn unregister_listener() {
    *LISTENER.lock().unwrap() = None;
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_klarvo_bridge_Bridge_registerAudioLevelListener<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    listener: JObject<'caller>,
) {
    unowned_env
        .with_env(|env| -> JniResult<()> { register_listener(env, &listener) })
        .resolve::<ThrowRuntimeExAndDefault>()
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_klarvo_bridge_Bridge_unregisterAudioLevelListener<'caller>(
    _unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
) {
    unregister_listener();
}

/// Called from the bridge task per broadcast-event. Returns `true` if a
/// listener received the call successfully.
pub(crate) fn emit_audio_level(level: &AudioLevel) -> bool {
    let vm = match JavaVM::singleton() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let outcome: JniResult<bool> = vm.attach_current_thread(|env| emit_inner(env, level));
    matches!(outcome, Ok(true))
}

fn emit_inner(env: &mut Env, level: &AudioLevel) -> JniResult<bool> {
    let guard = LISTENER.lock().unwrap();
    let Some(listener) = guard.as_ref() else {
        return Ok(false);
    };
    env.call_method(
        listener,
        jni_str!("onLevel"),
        jni_sig!("(FJ)V"),
        &[
            JValue::Float(level.rms),
            JValue::Long(level.ts_ms as i64),
        ],
    )?;
    Ok(true)
}
