//! JNI bridge for Groq cloud STT on Android — the consolidated STT path.
//!
//! Exposes Kotlin-callable JNI functions to the class
//! `com.klarvo.voice.GroqSttBridge`:
//!
//! - `nativeTranscribe(wavBase64, apiKey, language, dictionaryTerms, customPrompt,
//!                      sttModel, temperature, sttProvider, debugSttScenario): String`
//! - `nativeIsHallucination(text: String): Boolean`
//! - `nativeIsPromptEcho(transcription: String, sttHint: String): Boolean`
//! - `nativeStripPromptFragments(text: String, sttHint: String): String`
//! - `nativeSilenceCheck(wavBase64: String, minRecordingMs: Long, silenceThreshold: Float): String`
//!
//! ## Why this module exists
//!
//! ADR-0017 mandates a single shared Rust STT request + guard path. The Kotlin
//! twins (`KlarvoApi.transcribe`, `HallucinationFilter.kt`, `SilencePreFilter.kt`)
//! are deleted; Android now calls these JNI functions instead, consuming the exact
//! same Rust logic the desktop pipeline uses.
//!
//! ## Weg A runtime (DECIDED 2026-06-12)
//!
//! `WhisperStt::transcribe` is `async` over `reqwest`. From JNI there is no shared
//! Tokio runtime (see `stt/jni_bridge.rs:24-29`). We build a per-call throwaway
//! `current_thread` runtime and `block_on` the existing async function — no new
//! reqwest path, no new TLS dependency, no runtime lifecycle to manage. Cost is
//! negligible vs. a network round-trip. ANR is already handled because the Kotlin
//! call site runs on a background thread (`KlarvoOverlayService.kt:1347`).
//!
//! ## Panic safety
//!
//! JNI functions **must not panic** — a Rust panic propagating into the JVM causes
//! an unrecoverable crash. All `Result`s and `Option`s are handled explicitly;
//! errors are logged via `log::error!` and safe fallback values are returned.
//! Pattern mirrors `license/jni.rs` and `stt/jni_bridge.rs`.
//!
//! ## jni crate version
//!
//! This file uses `jni 0.21` (pinned in Cargo.toml). The 0.22 API is NOT available
//! (0.22 is the v2 archive). Do not add 0.22 imports.

//! ## Platform gating
//!
//! Every `extern "system"` entry point below is `#[cfg(target_os = "android")]`,
//! but the **module is not**: [`select_stt_provider`] — the one decision
//! `nativeTranscribe` makes before it hands off to the shared core — is plain
//! Rust and compiles everywhere, so the Linux `cargo test --lib` gate can reach
//! the Android debug branch. An android-gated selector would be a selector no
//! executing test can reach (story 13-1).

#[cfg(target_os = "android")]
use jni::objects::{JClass, JString};
#[cfg(target_os = "android")]
use jni::sys::{jboolean, jfloat, jlong, jstring};
#[cfg(target_os = "android")]
use jni::JNIEnv;

use super::{GroqWhisper, SttProvider};
#[cfg(target_os = "android")]
use super::{build_stt_prompt_with_hint, is_hallucination, strip_stockphrase_ghosts};
#[cfg(target_os = "android")]
use crate::pipeline::{compute_wav_rms, is_prompt_echo, silence_skip, strip_prompt_fragments};

// ---------------------------------------------------------------------------
// Provider choice — the ONE decision nativeTranscribe makes (story 13-1)
// ---------------------------------------------------------------------------

/// Picks the STT provider the Android JNI entry point will use.
///
/// Extracted out of the `extern "system"` function so it is reachable by a plain
/// Rust unit test: the Android `debug` branch was previously inside a function
/// no test could call, on a target no test runs on, so inverting its comparison
/// kept every gate green.
///
/// `debug_scenario`, `api_key`, `model` and `temperature` are all read from
/// `config.json` by Kotlin and carried through uninspected — the `"debug"`
/// decision is made here, in Rust, which is what keeps ADR-0017 intact: Android
/// gets the debug scenarios with no STT logic of its own.
///
/// Any provider name other than [`crate::llm::DEBUG_PROVIDER_NAME`] keeps the
/// production Groq path, including an empty string (the fail-soft value the JNI
/// caller substitutes when it cannot read the argument): a debug run that cannot
/// read its own arguments must degrade to the real provider, never the reverse.
///
/// Its only production caller is `nativeTranscribe` below, which is Android-only;
/// on every other target it exists solely so the `cargo test --lib` gate can
/// reach the branch. Hence the same `cfg_attr` dead-code allowance
/// `config::load_config` uses for the mirror-image situation.
#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
pub fn select_stt_provider(
    provider_name: &str,
    debug_scenario: &str,
    api_key: &str,
    model: &str,
    temperature: f32,
) -> Box<dyn SttProvider> {
    if provider_name == crate::llm::DEBUG_PROVIDER_NAME {
        log::info!("[groq_jni] DEBUG STT provider active: scenario={debug_scenario}");
        Box::new(crate::stt::DebugStt::new(debug_scenario))
    } else {
        Box::new(
            GroqWhisper::new(api_key)
                .with_model(model)
                .with_temperature(temperature),
        )
    }
}

// ---------------------------------------------------------------------------
// WAV duration helper (local — mirrors SilencePreFilter.computeDurationMs)
// ---------------------------------------------------------------------------

/// Computes WAV duration in milliseconds from the standard 44-byte PCM header.
///
/// Returns 0 if the header is malformed or too short.
/// Mirrors `SilencePreFilter.computeDurationMs` in Kotlin for boundary parity.
#[cfg(target_os = "android")]
fn compute_wav_duration_ms(wav_bytes: &[u8]) -> u64 {
    if wav_bytes.len() < 44 {
        return 0;
    }
    // WAV header: sample_rate at bytes 24-27 (LE u32), data chunk size at bytes 40-43 (LE u32).
    let sample_rate = u32::from_le_bytes([wav_bytes[24], wav_bytes[25], wav_bytes[26], wav_bytes[27]]) as u64;
    let data_size = u32::from_le_bytes([wav_bytes[40], wav_bytes[41], wav_bytes[42], wav_bytes[43]]) as u64;
    if sample_rate == 0 {
        return 0;
    }
    // 16-bit mono: 2 bytes per sample.
    (data_size / 2 * 1000) / sample_rate
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an empty Java string, falling back to null on JNI failure.
#[cfg(target_os = "android")]
fn empty_jstring(env: &mut JNIEnv) -> jstring {
    env.new_string("").map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Build a Java string from a Rust `&str`, falling back to empty string on failure.
#[cfg(target_os = "android")]
fn to_jstring(env: &mut JNIEnv, s: &str) -> jstring {
    match env.new_string(s) {
        Ok(js) => js.into_raw(),
        Err(e) => {
            log::error!("[groq_jni] to_jstring failed: {e}");
            empty_jstring(env)
        }
    }
}

/// Read a Java String argument into a Rust `String`. Returns `None` on failure
/// (caller must return a fail-soft value).
#[cfg(target_os = "android")]
fn read_jstring(env: &mut JNIEnv, arg: JString, name: &str) -> Option<String> {
    match env.get_string(&arg) {
        Ok(s) => Some(s.into()),
        Err(e) => {
            log::error!("[groq_jni] failed to read {name}: {e}");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// nativeTranscribe — the single shared Groq STT request path (AC1, Task 1)
// ---------------------------------------------------------------------------

/// Transcribes a Base64-encoded WAV using the shared Rust Groq STT path.
///
/// Parameters (from Kotlin):
/// - `wav_base64`:       Standard RFC-4648 Base64-encoded 16 kHz mono WAV.
/// - `api_key`:          Groq API Bearer token from `config.groqApiKey`.
/// - `language`:         ISO-639-1 code ("de", "en") or empty for auto-detect.
/// - `dictionary_terms`: Comma-separated user dictionary (or empty).
/// - `custom_prompt`:    User custom STT hint (or empty).
/// - `stt_model`:        Groq model name (e.g. "whisper-large-v3-turbo").
/// - `temperature`:      Whisper sampling temperature (0.0 = deterministic).
/// - `stt_provider`:     `config.sttProvider`, carried through by Kotlin without
///                       being inspected there. Story 13-1: `"debug"` selects
///                       [`crate::stt::DebugStt`]; every other value keeps the
///                       Groq path. Deciding this HERE rather than in Kotlin is
///                       what keeps ADR-0017 intact — Android gets the debug
///                       scenarios with no STT logic of its own, and the
///                       `__ERROR_*` sentinels below are still emitted by the
///                       real mapping rather than forged.
/// - `debug_stt_scenario`: `config.debugSttScenario`. Read only when
///                       `stt_provider == "debug"`.
///
/// Returns: transcribed text, or an empty string on any error.
///
/// Error codes embedded in the return string for distinguishable failures:
/// - `"__ERROR_EMPTY_AUDIO__"` — WAV decoded to zero bytes.
/// - `"__ERROR_API:<msg>__"`   — Groq API returned a non-2xx status.
/// - `"__ERROR_NETWORK:<msg>__"` — network failure (caller may retry).
///
/// The double-underscore prefix makes these machine-detectable by the Kotlin
/// retry wrapper so it can distinguish retriable from non-retriable errors.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_GroqSttBridge_nativeTranscribe(
    mut env: JNIEnv,
    _class: JClass,
    wav_base64: JString,
    api_key: JString,
    language: JString,
    dictionary_terms: JString,
    custom_prompt: JString,
    stt_model: JString,
    temperature: jfloat,
    stt_provider: JString,
    debug_stt_scenario: JString,
) -> jstring {
    // --- Unmarshal string arguments ---
    let b64 = match read_jstring(&mut env, wav_base64, "wav_base64") {
        Some(s) => s,
        None => return empty_jstring(&mut env),
    };
    let key = match read_jstring(&mut env, api_key, "api_key") {
        Some(s) => s,
        None => return empty_jstring(&mut env),
    };
    let lang = match read_jstring(&mut env, language, "language") {
        Some(s) => s,
        None => return empty_jstring(&mut env),
    };
    let dict = read_jstring(&mut env, dictionary_terms, "dictionary_terms").unwrap_or_default();
    let custom = read_jstring(&mut env, custom_prompt, "custom_prompt").unwrap_or_default();
    let model = match read_jstring(&mut env, stt_model, "stt_model") {
        Some(s) if !s.is_empty() => s,
        _ => "whisper-large-v3-turbo".to_string(),
    };
    let temp = temperature as f32;
    // Story 13-1. Both default to the production path on a read failure — a
    // debug run that cannot read its own arguments must degrade to the real
    // provider, never the other way round.
    let provider_name = read_jstring(&mut env, stt_provider, "stt_provider").unwrap_or_default();
    let debug_scenario =
        read_jstring(&mut env, debug_stt_scenario, "debug_stt_scenario").unwrap_or_default();

    // --- Decode Base64 → WAV bytes ---
    use base64::Engine as _;
    let wav_bytes = match base64::engine::general_purpose::STANDARD.decode(&b64) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("[groq_jni] Base64 decode failed: {e}");
            return empty_jstring(&mut env);
        }
    };

    if wav_bytes.is_empty() {
        log::warn!("[groq_jni] decoded WAV is empty");
        return to_jstring(&mut env, "__ERROR_EMPTY_AUDIO__");
    }

    // --- Build STT prompt (H3 / Recall #5 / L3 parity — Rust is the single source) ---
    let dict_opt = if dict.trim().is_empty() { None } else { Some(dict.trim()) };
    let custom_opt = if custom.trim().is_empty() { None } else { Some(custom.trim()) };
    let prompt = build_stt_prompt_with_hint(dict_opt, &lang, custom_opt);

    // --- Build client (H9: sttModel from config, H10: no hardcoded model literal) ---
    //
    // Story 13-1: `sttProvider == "debug"` swaps in the canned-wire provider.
    // The choice lives in [`select_stt_provider`] so a Rust test can reach it;
    // everything below is unchanged — the same runtime, the same guard chain and
    // the same `__ERROR_*` sentinel mapping — so an Android debug run produces
    // the production strings instead of imitating them.
    let client: Box<dyn SttProvider> =
        select_stt_provider(&provider_name, &debug_scenario, &key, &model, temp);

    // --- Weg A: throwaway current-thread runtime + block_on ---
    // `WhisperStt::transcribe` is async over reqwest. From JNI there is no shared
    // Tokio runtime. We create a per-call throwaway runtime (current_thread) and
    // block_on the existing async path. The runtime is dropped at end of scope.
    // ANR is already handled: Kotlin calls this from a background thread.
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            log::error!("[groq_jni] failed to build Tokio runtime: {e}");
            return empty_jstring(&mut env);
        }
    };

    let result = runtime.block_on(client.transcribe(&wav_bytes, &lang, prompt.as_deref()));

    match result {
        Ok(text) => {
            log::info!("[groq_jni] transcribe ok, len={}", text.len());

            // AC2 (Finding 4): apply is_prompt_echo (H6) and strip_prompt_fragments (H7)
            // inline here, exactly as the desktop pipeline does (pipeline.rs:501, 1032).
            // This ensures Android inherits these guards without requiring any Kotlin change.
            // The separate nativeIsPromptEcho / nativeStripPromptFragments JNI fns remain
            // but are no longer the primary path for the nativeTranscribe caller.
            let hint = prompt.as_deref().unwrap_or("");
            if is_prompt_echo(&text, hint) {
                log::info!("[groq_jni] transcript is prompt echo (H6), returning empty");
                return to_jstring(&mut env, "");
            }
            let stripped = strip_prompt_fragments(&text, hint);
            // Also strip stockphrase ghosts (AC7) from the post-strip output.
            let cleaned = strip_stockphrase_ghosts(&stripped);

            to_jstring(&mut env, &cleaned)
        }
        Err(crate::stt::SttError::EmptyAudio) => {
            log::warn!("[groq_jni] transcribe: empty audio");
            to_jstring(&mut env, "__ERROR_EMPTY_AUDIO__")
        }
        Err(crate::stt::SttError::ApiError { status, message }) => {
            let msg = format!("__ERROR_API:HTTP {status}: {message}__");
            log::warn!("[groq_jni] transcribe API error: {msg}");
            to_jstring(&mut env, &msg)
        }
        Err(e) => {
            let msg = format!("__ERROR_NETWORK:{e}__");
            log::warn!("[groq_jni] transcribe network error: {msg}");
            to_jstring(&mut env, &msg)
        }
    }
}

// ---------------------------------------------------------------------------
// nativeIsHallucination — shared Rust hallucination guard (AC2, Task 2)
// ---------------------------------------------------------------------------

/// Returns `true` (JNI_TRUE) if `text` is a Whisper hallucination artifact.
///
/// Replaces `HallucinationFilter.isHallucination()` in Kotlin. The Kotlin twin
/// is deleted after this bridge is wired in.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_GroqSttBridge_nativeIsHallucination(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) -> jboolean {
    let s = match read_jstring(&mut env, text, "text") {
        Some(s) => s,
        None => return jni::sys::JNI_TRUE, // fail-safe: unknown = treat as hallucination
    };
    if is_hallucination(&s) {
        jni::sys::JNI_TRUE
    } else {
        jni::sys::JNI_FALSE
    }
}

// ---------------------------------------------------------------------------
// nativeIsPromptEcho — shared Rust prompt-echo guard (AC2 / H6)
// ---------------------------------------------------------------------------

/// Returns `true` if `transcription` is an echo of the STT conditioning prompt.
///
/// Replaces the implicit echo check in the Kotlin pipeline path.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_GroqSttBridge_nativeIsPromptEcho(
    mut env: JNIEnv,
    _class: JClass,
    transcription: JString,
    stt_hint: JString,
) -> jboolean {
    let trans = match read_jstring(&mut env, transcription, "transcription") {
        Some(s) => s,
        None => return jni::sys::JNI_FALSE,
    };
    let hint = match read_jstring(&mut env, stt_hint, "stt_hint") {
        Some(s) => s,
        None => return jni::sys::JNI_FALSE,
    };
    if is_prompt_echo(&trans, &hint) {
        jni::sys::JNI_TRUE
    } else {
        jni::sys::JNI_FALSE
    }
}

// ---------------------------------------------------------------------------
// nativeStripPromptFragments — shared Rust fragment strip (AC2 / H7)
// ---------------------------------------------------------------------------

/// Strips conditioning-prompt fragments from the transcription.
///
/// Returns the stripped text, or the original text on any error.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_GroqSttBridge_nativeStripPromptFragments(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
    stt_hint: JString,
) -> jstring {
    let t = match read_jstring(&mut env, text, "text") {
        Some(s) => s,
        None => return empty_jstring(&mut env),
    };
    let hint = match read_jstring(&mut env, stt_hint, "stt_hint") {
        Some(s) => s,
        None => return to_jstring(&mut env, &t),
    };
    let stripped = strip_prompt_fragments(&t, &hint);
    // Also strip stockphrase ghosts (AC7) from the post-strip output.
    let result = strip_stockphrase_ghosts(&stripped);
    to_jstring(&mut env, &result)
}

// ---------------------------------------------------------------------------
// nativeSilenceCheck — shared Rust pre-STT silence filter (AC4)
// ---------------------------------------------------------------------------

/// Pre-STT silence and duration filter.
///
/// Returns a JSON-encoded result string consumed by Kotlin:
/// - `"Pass"`
/// - `"TooShort:<durationMs>"`
/// - `"Silent:<rms>"`
///
/// Replaces `SilencePreFilter.check()` in Kotlin.
///
/// `min_recording_ms` and `silence_threshold` are read from `config.json` by the Kotlin
/// caller (`KlarvoOverlayService.resolveMinRecordingMsForSilenceFilter(cachedConfig)` /
/// the `silenceThreshold` field, both config-driven since Story 7.2, AC5) and passed in here
/// as arguments -- `500L`/`0.005f` are only the Kotlin caller's null-safe fallback defaults,
/// matching the desktop pipeline defaults. This function itself stays pure / config-free.
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_GroqSttBridge_nativeSilenceCheck(
    mut env: JNIEnv,
    _class: JClass,
    wav_base64: JString,
    min_recording_ms: jlong,
    silence_threshold: jfloat,
) -> jstring {
    let b64 = match read_jstring(&mut env, wav_base64, "wav_base64") {
        Some(s) => s,
        None => return to_jstring(&mut env, "Pass"), // fail-open: unknown audio → proceed
    };

    use base64::Engine as _;
    let wav_bytes = match base64::engine::general_purpose::STANDARD.decode(&b64) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::warn!("[groq_jni] SilenceCheck: Base64 decode failed, proceeding: {e}");
            return to_jstring(&mut env, "Pass");
        }
    };

    // Duration check (mirrors SilencePreFilter.computeDurationMs).
    let duration_ms = compute_wav_duration_ms(&wav_bytes);

    // RMS check (mirrors SilencePreFilter.computeWavRms).
    let rms = compute_wav_rms(&wav_bytes);

    match silence_skip(
        duration_ms,
        min_recording_ms.max(0) as u64,
        rms,
        silence_threshold as f32,
    ) {
        Some(crate::pipeline::SilenceSkip::TooShort) => {
            let msg = format!("TooShort:{duration_ms}");
            to_jstring(&mut env, &msg)
        }
        Some(crate::pipeline::SilenceSkip::Silent) => {
            let rms_val = rms.unwrap_or(0.0);
            let msg = format!("Silent:{rms_val:.6}");
            to_jstring(&mut env, &msg)
        }
        None => to_jstring(&mut env, "Pass"),
    }
}

// ---------------------------------------------------------------------------
// Tests (pure Rust, no JVM needed)
// ---------------------------------------------------------------------------

// ⚠ This test module stays `#[cfg(target_os = "android")]`, i.e. it still never
// executes — exactly as before story 13-1, which only ungated the module's
// non-JNI half. Ungating it is a separate piece of work, not a free win:
// `test_panic_safety_is_hallucination_unusual_inputs` asserts
// `is_hallucination("\0")`, which is FALSE against today's guard. That
// assertion was written in story 7-3 and, because the whole file was
// android-gated, has never run. It is a never-executed test's wrong
// expectation, not a product defect — but whoever ungates this module has to
// adjudicate it first.
//
// The one thing this story needed to be testable — the provider choice — lives
// outside this module in [`select_stt_provider`] and is covered by
// `stt::tests::spec_android_select_stt_provider_*`, which DO run on the
// `cargo test --lib` gate.
#[cfg(target_os = "android")]
#[cfg(test)]
mod tests {
    use super::*;

    // --- R-001 proof gate: Weg A runtime can be built ---

    /// Verifies that a throwaway current-thread Tokio runtime can be created
    /// without panicking. This is the R-001 proof gate (Weg A viability).
    ///
    /// A full round-trip to Groq is not possible in CI (no live API key).
    /// The network boundary is mocked via wiremock in integration tests.
    /// This test only proves that `block_on` infrastructure is available.
    #[test]
    fn test_r001_throwaway_tokio_runtime_can_be_built() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        assert!(rt.is_ok(), "throwaway current-thread runtime must build without panic");
        let rt = rt.unwrap();
        // block_on a trivial future: verifies the executor works.
        let result = rt.block_on(async { 42u32 });
        assert_eq!(result, 42, "block_on must execute a trivial future");
    }

    /// Verifies that building a second runtime (per-call pattern) after dropping
    /// the first does not panic or deadlock.
    #[test]
    fn test_r001_two_sequential_runtimes_do_not_conflict() {
        for i in 0..3u32 {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime must build");
            let v = rt.block_on(async { i * 2 });
            assert_eq!(v, i * 2);
            // Drop rt here: verifies isolation.
        }
    }

    // --- Panic-safety tests (AC10 / R-003) ---

    /// The prompt builder must not panic on any combination of empty inputs.
    #[test]
    fn test_panic_safety_prompt_builder_empty_inputs() {
        let _ = build_stt_prompt_with_hint(None, "", None);
        let _ = build_stt_prompt_with_hint(Some(""), "de", Some(""));
        let _ = build_stt_prompt_with_hint(Some("   "), "", Some("   "));
    }

    /// is_hallucination must not panic on unusual inputs.
    #[test]
    fn test_panic_safety_is_hallucination_unusual_inputs() {
        assert!(is_hallucination(""));
        assert!(is_hallucination("\0"));
        let _ = is_hallucination(&"a".repeat(10_000)); // very long string
        let _ = is_hallucination("♪♪♪♪♪");
        let _ = is_hallucination("日本語テスト");
    }

    // --- Silence-check contract tests (AC4 boundary parity) ---

    /// Exactly MIN_RECORDING_MS → Pass (< not <=, per SilencePreFilter contract).
    #[test]
    fn test_ac4_silence_check_boundary_exactly_min_ms_is_pass() {
        // silence_skip(duration, min, rms, threshold): duration == min → Pass
        assert_eq!(
            silence_skip(500, 500, Some(0.1), 0.005),
            None,
            "exactly MIN_RECORDING_MS must be Pass (< not <=)"
        );
    }

    /// One ms below MIN → TooShort.
    #[test]
    fn test_ac4_silence_check_one_below_min_is_too_short() {
        assert_eq!(
            silence_skip(499, 500, Some(0.1), 0.005),
            Some(crate::pipeline::SilenceSkip::TooShort)
        );
    }

    /// Exactly SILENCE_THRESHOLD → Pass (< not <=).
    #[test]
    fn test_ac4_silence_check_boundary_exactly_threshold_is_pass() {
        assert_eq!(
            silence_skip(1000, 500, Some(0.005), 0.005),
            None,
            "exactly SILENCE_THRESHOLD must be Pass (< not <=)"
        );
    }

    /// Malformed WAV (rms = None) → skip RMS check → Pass.
    #[test]
    fn test_ac4_silence_check_malformed_wav_rms_none_is_pass() {
        // rms = None: SilencePreFilter contract says skip silent check → Pass.
        assert_eq!(
            silence_skip(1000, 500, None, 0.005),
            None,
            "malformed WAV (rms=None) must skip silent check and Pass"
        );
    }

    // --- AC2 (Finding 4): nativeTranscribe path applies is_prompt_echo + strip_prompt_fragments ---
    //
    // The full JNI path can't be tested without a JVM, but we can unit-test the
    // pure-Rust logic that nativeTranscribe now calls inline. This test verifies
    // that the guard chain (is_prompt_echo → strip_prompt_fragments → strip_stockphrase_ghosts)
    // behaves identically to the desktop pipeline for the same inputs.

    /// A transcript that is a prompt echo must be caught by is_prompt_echo,
    /// mirroring the desktop pipeline (pipeline.rs:501).
    #[test]
    fn test_ac2_prompt_echo_detected_by_inline_logic() {
        let hint = "Diktat auf Deutsch mit gelegentlichen englischen Fachbegriffen. Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion.";
        // A verbatim echo of the hint text should be detected as a prompt echo.
        let transcript = hint;
        assert!(
            is_prompt_echo(transcript, hint),
            "verbatim prompt echo must be detected by is_prompt_echo"
        );
        // Simulating what nativeTranscribe now does:
        // if is_prompt_echo → return empty string (not forwarded to Kotlin).
    }

    /// strip_prompt_fragments removes prompt conditioning fragments from a real
    /// transcript that has them appended — mirroring pipeline.rs:1032.
    #[test]
    fn test_ac2_strip_prompt_fragments_inline_logic() {
        let hint = "Voice dictation in English. Proper punctuation.";
        let transcript = "Here is my note. Voice dictation in English.";
        let stripped = strip_prompt_fragments(transcript, hint);
        let cleaned = strip_stockphrase_ghosts(&stripped);
        // The prompt fragment should be stripped; real content preserved.
        assert!(
            !cleaned.to_lowercase().contains("voice dictation in english"),
            "prompt fragment must be stripped by the inline guard chain"
        );
        assert!(
            cleaned.contains("Here is my note"),
            "real transcript content must be preserved"
        );
    }
}
