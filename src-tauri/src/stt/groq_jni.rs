//! JNI bridge for Groq cloud STT on Android — the consolidated STT path.
//!
//! Exposes Kotlin-callable JNI functions to the class
//! `com.klarvo.voice.GroqSttBridge`:
//!
//! - `nativeTranscribe(wavBase64, apiKey, language, dictionaryTerms, customPrompt,
//!                      sttModel, temperature, testProviderStt): String`
//! - `nativeIsHallucination(text: String): Boolean`
//! - `nativeIsPromptEcho(transcription: String, sttHint: String): Boolean`
//! - `nativeStripPromptFragments(text: String, sttHint: String): String`
//! - `nativeStripStockphraseGhosts(text: String): String`
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
//! but the **module is not**. Every DECISION `nativeTranscribe` makes is a
//! plain-Rust helper that compiles everywhere, so the Linux `cargo test --lib`
//! gate can reach it: [`select_stt_provider`] (story 13-1), and — added by
//! story 13-2 and completed at its review — [`guard_hint_for_jni`],
//! [`stt_error_sentinel`] and [`guard_transcript_for_jni`].
//!
//! The rule, learned twice: an android-gated decision is a decision no
//! executing test can reach, so it can be reverted with every gate green. The
//! `extern` functions keep only unmarshalling, the runtime and the return.

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
// The shared guard chain, reachable from `cargo test --lib` (story 13-2)
// ---------------------------------------------------------------------------

/// The conditioning hint `nativeTranscribe` feeds the guard chain, rebuilt
/// from the two arguments that already cross the JNI boundary.
///
/// Story 13-2 (B2 / D-H5, D-H6). The pre-13-2 code fed the guards
/// `prompt.as_deref().unwrap_or("")` — the **full built prompt**, hint plus the
/// comma-joined dictionary plus `customPrompt` — which made a dictionary-word
/// utterance look like a prompt echo and deleted any ≥10-byte dictionary term
/// from every transcript. The desktop pipeline feeds `stt::stt_hint_text`; so
/// does this.
///
/// Plain Rust, not `#[cfg(target_os = "android")]`, for the reason the whole
/// story keeps repeating: a decision no executing test can reach is a decision
/// that can be reverted with every gate green. This one WAS exactly that —
/// review finding, 2026-09-21. Same `cfg_attr` shape as [`select_stt_provider`].
#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
pub fn guard_hint_for_jni(language: &str, custom_prompt: &str) -> String {
    let custom_opt = if custom_prompt.trim().is_empty() {
        None
    } else {
        Some(custom_prompt.trim())
    };
    crate::stt::stt_hint_text(language, custom_opt)
}

/// Maps an [`crate::stt::SttError`] to the `__ERROR_*` sentinel Kotlin's retry
/// ladder reads.
///
/// Story 13-2 (D9 / D-M5). `ResponseFormat` is matched **before** the
/// catch-all: an empty or unparseable STT answer is non-retryable
/// (`pipeline::is_retryable_stt_error`), and until this story it reached Kotlin
/// as `__ERROR_NETWORK:` and burned three Groq calls plus ~7 s of backoff on a
/// guaranteed repeat. Written as a `match` over the enum rather than as arms
/// inside the `extern "system"` function, so the ORDER is a property of a
/// function a Linux test can call — moving an arm here now fails a gate
/// instead of passing one (review finding, 2026-09-21).
///
/// Kotlin twin of the classification: `KlarvoOverlayService.classifySttSentinel`,
/// pinned against `TEST-STT-EMPTY-001`'s `kotlin.sentinel` literal on both sides.
#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
pub fn stt_error_sentinel(err: &crate::stt::SttError) -> String {
    match err {
        crate::stt::SttError::EmptyAudio => "__ERROR_EMPTY_AUDIO__".to_string(),
        crate::stt::SttError::ApiError { status, message } => {
            format!("__ERROR_API:HTTP {status}: {message}__")
        }
        crate::stt::SttError::ResponseFormat(message) => format!("__ERROR_FORMAT:{message}__"),
        other => format!("__ERROR_NETWORK:{other}__"),
    }
}

/// Runs the ONE post-STT guard chain for the Android JNI and maps its verdict
/// to what `nativeTranscribe` hands back to Kotlin.
///
/// `Some(text)` — the transcript survived; `None` — it was dropped by the echo
/// or blocklist guard, which Kotlin sees as the empty string (the shipped
/// "nothing recognised" ending, silent on both platforms after D11).
///
/// Story 13-2 (B2 / D-H5, D-H6, D-M9 and B3 / D-H7). Until this story
/// `nativeTranscribe` re-implemented the chain inline, in the wrong order, fed
/// with the **full built prompt** (hint + dictionary + the LLM cleanup
/// instruction) where the desktop feeds the hint alone. Those were not three
/// bugs but one: a second copy of a decision that already lived in one pure
/// function. There is no copy any more — this delegates to
/// [`crate::pipeline::guard_transcript`], which is what the desktop pipeline
/// calls.
///
/// Plain Rust, not `#[cfg(target_os = "android")]`, following the
/// [`select_stt_provider`] precedent: an android-gated chain is a chain no
/// executing test can reach, and every B2/B3 claim would be agent-only.
#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
pub fn guard_transcript_for_jni(text: &str, stt_hint: &str) -> Option<String> {
    let outcome = crate::pipeline::guard_transcript(text, stt_hint);
    match outcome.skip {
        Some(skip) => {
            log::info!("[groq_jni] transcript dropped by the shared guard chain: {skip:?}");
            None
        }
        None => Some(outcome.text),
    }
}

// ---------------------------------------------------------------------------
// Provider choice — the ONE decision nativeTranscribe makes (story 13-1)
// ---------------------------------------------------------------------------

/// Picks the STT provider the Android JNI entry point will use.
///
/// Extracted out of the `extern "system"` function so it is reachable by a plain
/// Rust unit test: the Android test branch was previously inside a function no
/// test could call, on a target no test runs on, so inverting its comparison
/// kept every gate green.
///
/// Story 13-1b: there is no `provider_name` argument any more. The value IS the
/// state — `test_provider_stt` is `advanced.testProviderStt`, and anything but
/// `"off"` both switches the test provider on and names the canned answer. That
/// removes the pair of arguments that could disagree with each other.
///
/// `test_provider_stt`, `api_key`, `model` and `temperature` are all read from
/// `config.json` by Kotlin and carried through uninspected — the decision is
/// made here, in Rust, which is what keeps ADR-0017 intact: Android gets the
/// test scenarios with no STT logic of its own.
///
/// `"off"` and the empty string (the fail-soft value the JNI caller substitutes
/// when it cannot read the argument) both keep the production Groq path: a test
/// run that cannot read its own arguments must degrade to the real provider,
/// never the reverse.
///
/// Its only production caller is `nativeTranscribe` below, which is Android-only;
/// on every other target it exists solely so the `cargo test --lib` gate can
/// reach the branch. Hence the same `cfg_attr` dead-code allowance
/// `config::load_config` uses for the mirror-image situation.
#[cfg_attr(not(any(test, target_os = "android")), allow(dead_code))]
pub fn select_stt_provider(
    test_provider_stt: &str,
    api_key: &str,
    model: &str,
    temperature: f32,
) -> Box<dyn SttProvider> {
    if !test_provider_stt.is_empty() && test_provider_stt != crate::config::TEST_PROVIDER_OFF {
        log::info!("[groq_jni] TEST STT provider active: scenario={test_provider_stt}");
        Box::new(crate::stt::TestStt::new(test_provider_stt))
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
/// - `custom_prompt`:    The Whisper CONDITIONING hint — `advanced.sttPromptDe`
///                       / `…En` / `…Auto`, selected for `language` by
///                       `KlarvoApi.selectSttHintOverride`, or empty for the
///                       built-in hint. Story 13-2 (B4 / D-H4): Kotlin used to
///                       pass `config.customPrompt`, the **LLM cleanup
///                       instruction**, which replaced the language hint in the
///                       Whisper request and poisoned both guards.
///                       `customPrompt` now reaches the LLM and nothing else.
/// - `stt_model`:        Groq model name (e.g. "whisper-large-v3-turbo").
/// - `temperature`:      Whisper sampling temperature (0.0 = deterministic).
/// - `test_provider_stt`: `config.advanced.testProviderStt`, carried through by
///                       Kotlin without being inspected there. Story 13-1b:
///                       anything but `"off"` selects [`crate::stt::TestStt`]
///                       with that value as its scenario; `"off"` (and an
///                       unreadable argument) keeps the Groq path. Deciding this
///                       HERE rather than in Kotlin is what keeps ADR-0017
///                       intact — Android gets the test scenarios with no STT
///                       logic of its own, and the `__ERROR_*` sentinels below
///                       are still emitted by the real mapping rather than
///                       forged.
///
/// ⚠️ The parameter list is 8 wide since story 13-1b (was 9). `#[no_mangle]`
/// exports the short JNI name with no signature suffix, so this declaration and
/// `GroqSttBridge.nativeTranscribe` must change in the SAME commit — a one-sided
/// edit, or a stale `libklarvo_lib.so`, misbinds silently instead of throwing.
///
/// Returns: transcribed text, or an empty string on any error.
///
/// Error codes embedded in the return string for distinguishable failures:
/// - `"__ERROR_EMPTY_AUDIO__"` — WAV decoded to zero bytes. NOT retryable.
/// - `"__ERROR_API:<msg>__"`   — Groq API returned a non-2xx status. Retryable
///   iff the embedded status is not 4xx.
/// - `"__ERROR_FORMAT:<msg>__"` — the answer arrived but is empty or does not
///   parse (`SttError::ResponseFormat`). **NOT retryable**, matching
///   `pipeline::is_retryable_stt_error`. Added by story 13-2 (D9 / D-M5):
///   without its own sentinel this class fell into the `__ERROR_NETWORK:`
///   catch-all and Android burned three Groq calls and ~7 s of backoff on a
///   guaranteed repeat that Desktop reports on the first attempt.
/// - `"__ERROR_NETWORK:<msg>__"` — network failure (caller may retry).
///
/// The double-underscore prefix makes these machine-detectable by the Kotlin
/// retry wrapper so it can distinguish retriable from non-retriable errors.
/// The classification itself is `KlarvoOverlayService.classifySttSentinel`.
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
    test_provider_stt: JString,
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
    // Story 13-1b. ONE argument now, and it defaults to the production path on a
    // read failure — a test run that cannot read its own arguments must degrade
    // to the real provider, never the other way round. `unwrap_or_default()`
    // yields `""`, which `select_stt_provider` treats exactly like `"off"`.
    let test_provider =
        read_jstring(&mut env, test_provider_stt, "test_provider_stt").unwrap_or_default();

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

    // Story 13-2 (B2): the guards need the conditioning hint ALONE, not the
    // built prompt. Rather than widen the JNI signature — `#[no_mangle]`
    // exports the short name, so an arity change can misbind a stale `.so`
    // silently — the hint is rebuilt from the two arguments that already cross
    // the boundary. The rebuild lives in [`guard_hint_for_jni`] so a Linux test
    // can reach it; `custom_prompt` carries `advanced.sttPrompt*` since this
    // story (B4), so the two sides compute the identical string.
    let hint = guard_hint_for_jni(&lang, &custom);

    // --- Build client (H9: sttModel from config, H10: no hardcoded model literal) ---
    //
    // Story 13-1b: a non-`off` `advanced.testProviderStt` swaps in the
    // canned-wire provider. The choice lives in [`select_stt_provider`] so a Rust
    // test can reach it; everything below is unchanged — the same runtime, the
    // same guard chain and the same `__ERROR_*` sentinel mapping — so an Android
    // test run produces the production strings instead of imitating them.
    let client: Box<dyn SttProvider> =
        select_stt_provider(&test_provider, &key, &model, temp);

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

            // Story 13-2 (B2/B3): ONE chain, the desktop's own. See
            // [`guard_transcript_for_jni`]. A dropped transcript comes back as
            // the empty string, exactly as before.
            match guard_transcript_for_jni(&text, &hint) {
                Some(cleaned) => to_jstring(&mut env, &cleaned),
                None => to_jstring(&mut env, ""),
            }
        }
        // Story 13-2 (D9 / D-M5): the whole error→sentinel mapping, including
        // the ordering claim that `ResponseFormat` is decided before the
        // catch-all, lives in [`stt_error_sentinel`] — a function a Linux test
        // can call. Inline arms here could be reordered with every gate green.
        Err(e) => {
            let msg = stt_error_sentinel(&e);
            log::warn!("[groq_jni] transcribe failed: {msg}");
            to_jstring(&mut env, &msg)
        }
    }
}

// ---------------------------------------------------------------------------
// nativeStripStockphraseGhosts — post-cleanup ghost strip (story 13-2, B3)
// ---------------------------------------------------------------------------

/// Strips stockphrase ghosts from already-cleaned text.
///
/// The second half of B3 / D-H7: the desktop runs `strip_stockphrase_ghosts`
/// once more **after** LLM cleanup (`pipeline.rs`, right after
/// `sanitize_llm_output`), because cleanup rationalises a recognisable ghost
/// (`"Klinge"`) into a convincing full stockphrase (`"Kleinschreibung"`) —
/// detectable junk turned fluent. Android had no post-cleanup strip at all.
///
/// A sibling of [`Java_com_klarvo_voice_GroqSttBridge_nativeStripPromptFragments`]
/// rather than a reuse of it: that one also applies `strip_prompt_fragments`,
/// which the desktop does NOT do after cleanup. Same job, same function, same
/// position — no Kotlin ghost-strip twin (ADR-0017).
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_klarvo_voice_GroqSttBridge_nativeStripStockphraseGhosts(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) -> jstring {
    let t = match read_jstring(&mut env, text, "text") {
        Some(s) => s,
        None => return empty_jstring(&mut env),
    };
    let result = strip_stockphrase_ghosts(&t);
    to_jstring(&mut env, &result)
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
// Everything story 13-2 needed to be testable lives OUTSIDE this module, in
// the plain-Rust helpers at the top of the file, and is covered by tests that
// DO run on the `cargo test --lib` gate:
// [`select_stt_provider`] (13-1) → `stt::tests::spec_android_select_stt_provider_*`,
// [`guard_hint_for_jni`] → `spec_jni_guard_hint_is_the_hint_and_not_the_built_prompt`,
// [`stt_error_sentinel`] → `spec_jni_response_format_sentinel_precedes_the_network_catch_all`,
// [`guard_transcript_for_jni`] → `pipeline::tests::spec_jni_guard_wrapper_*`.
// That is the rule this file keeps learning: a decision left inside
// `nativeTranscribe` is a decision that can be reverted with every gate green.
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

    // --- AC2 (Finding 4), story 7-3: the individual guards ---
    //
    // HISTORICAL. These two date from when `nativeTranscribe` re-implemented
    // the chain inline, in the order `is_prompt_echo → strip_prompt_fragments
    // → strip_stockphrase_ghosts`. Story 13-2 deleted that inline chain: there
    // is ONE chain now, `pipeline::guard_transcript` (fragment strip → ghost
    // strip → echo/blocklist verdict), reached from here through
    // [`guard_transcript_for_jni`] and covered by `pipeline::tests::spec_guard_*`
    // against `test-fixtures/guard-chain-vectors.json` — tests that actually
    // run. What is left below is two smoke checks on the individual guard
    // functions, and like the rest of this module they have never executed.

    /// A verbatim echo of the hint is caught by `is_prompt_echo`. The chain
    /// that calls it lives in `pipeline::guard_transcript`.
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

    /// `strip_prompt_fragments` removes leaked conditioning fragments. In the
    /// shipped chain it runs BEFORE the ghost strip and before the verdict —
    /// see `pipeline::guard_transcript`, which owns the order.
    #[test]
    fn test_ac2_strip_prompt_fragments_inline_logic() {
        let hint = "Voice dictation in English. Proper punctuation.";
        let transcript = "Here is my note. Voice dictation in English.";
        let stripped = strip_prompt_fragments(transcript, hint);
        let cleaned = strip_stockphrase_ghosts(&stripped);
        // The prompt fragment should be stripped; real content preserved.
        assert!(
            !cleaned.to_lowercase().contains("voice dictation in english"),
            "prompt fragment must be stripped by the shared guard chain"
        );
        assert!(
            cleaned.contains("Here is my note"),
            "real transcript content must be preserved"
        );
    }
}

// ---------------------------------------------------------------------------
// The composition tripwire (story 13-2, follow-up review 2026-09-21)
// ---------------------------------------------------------------------------

/// Source-text contract over `nativeTranscribe`'s body, because no gate compiles it.
///
/// ## Why this exists
/// The review round lifted every DECISION out of the `extern "system"` function
/// into [`guard_hint_for_jni`], [`stt_error_sentinel`] and
/// [`guard_transcript_for_jni`], each with its own executing test. What it did
/// not lift — because it cannot be — is the **composition**: which value is fed
/// to which helper, inside a `#[cfg(target_os = "android")]` function that
/// `cargo test --lib` never compiles and the JVM gate never builds.
///
/// Measured at this review: replacing `guard_hint_for_jni(&lang, &custom)` with
/// the pre-13-2 `prompt.as_deref().unwrap_or("")` — the exact feed that made a
/// dictionary-word utterance look like a prompt echo (B2 / D-H5, D-H6) — leaves
/// `cargo test --lib` at **774 passed**. The helpers stay green because they are
/// still called by their own tests. That is this story's own defect class, one
/// level up, on the side that got tripwires last: the Kotlin half has
/// `OverlayServiceSourceContractTest` and `GuardChainBridgeTest`, the Rust half
/// had nothing.
///
/// ## The instrument, and its honest weight
/// This reads the file as text, the sanctioned instrument in this repo for a
/// claim no gate can execute (`Adr0017BoundaryGuardTest` is the precedent). It
/// proves the code SAYS the right thing, never that the device does it. The
/// device half stays on the H+ list in `gate4-evidence/13-2/verdict.md`.
///
/// Every assertion is order-anchored or two-sided, never a bare `contains` —
/// four of this story's first tripwires came back GREEN under inversion because
/// an unrelated second occurrence satisfied them.
#[cfg(test)]
mod composition_contract {
    /// `nativeTranscribe`'s body, by brace matching from its parameter list.
    fn native_transcribe_body() -> String {
        let src = include_str!("groq_jni.rs");
        let decl = src
            .find("pub extern \"system\" fn Java_com_klarvo_voice_GroqSttBridge_nativeTranscribe(")
            .expect("nativeTranscribe must exist");
        let brace = src[decl..]
            .find(") -> jstring {")
            .map(|i| decl + i + ") -> jstring ".len())
            .expect("nativeTranscribe must have a body");
        let bytes = src.as_bytes();
        let mut depth = 0usize;
        for (i, _) in src[brace..].char_indices() {
            match bytes[brace + i] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        return src[brace..=brace + i].to_string();
                    }
                }
                _ => {}
            }
        }
        panic!("unbalanced braces in nativeTranscribe");
    }

    /// B2 / D-H5, D-H6: the guards are fed the conditioning HINT, and the wire
    /// prompt is fed to the provider — never the other way round.
    ///
    /// Two-sided on purpose. An assertion that only demanded the
    /// `guard_hint_for_jni` call would pass against a body that computed the
    /// hint and then handed `prompt` to the chain anyway.
    #[test]
    fn spec_jni_feeds_the_guards_the_hint_and_the_provider_the_prompt() {
        let body = native_transcribe_body();

        let hint = body
            .find("let hint = guard_hint_for_jni(&lang, &custom);")
            .expect("the guard hint must be rebuilt by guard_hint_for_jni, not inlined");
        let wire = body
            .find("let prompt = build_stt_prompt_with_hint(dict_opt, &lang, custom_opt);")
            .expect("the wire prompt must still be the full built prompt");
        let transcribe = body
            .find("client.transcribe(&wav_bytes, &lang, prompt.as_deref())")
            .expect("the provider must receive the built prompt");
        let chain = body
            .find("guard_transcript_for_jni(&text, &hint)")
            .expect("the guard chain must be fed `hint`, the value guard_hint_for_jni produced");

        assert!(
            wire < hint && hint < transcribe && transcribe < chain,
            "order: build the wire prompt, rebuild the hint, transcribe, then guard \
             (wire@{wire}, hint@{hint}, transcribe@{transcribe}, chain@{chain})"
        );
        assert!(
            !body.contains("guard_transcript_for_jni(&text, prompt"),
            "the pre-13-2 feed is back: the guards must never see the built prompt"
        );
    }

    /// D9 / D-M5: the error→sentinel mapping is the helper's, not re-inlined.
    ///
    /// [`super::stt_error_sentinel`] owns the ordering claim (`ResponseFormat`
    /// before the catch-all) and a Linux test drives it — but only while the
    /// `extern` actually calls it. Re-inlining the four arms here would restore
    /// the three-Groq-call burn with every gate green.
    #[test]
    fn spec_jni_maps_errors_through_the_sentinel_helper() {
        let body = native_transcribe_body();
        assert!(
            body.contains("let msg = stt_error_sentinel(&e);"),
            "the sentinel mapping must go through stt_error_sentinel"
        );
        // NOT `__ERROR_EMPTY_AUDIO__`: that literal has a second, legitimate
        // site in this body -- the pre-request early return for a WAV that
        // decoded to zero bytes, which is not an error MAPPING. Asserting it
        // here would have been exactly the unrelated-second-occurrence trap
        // that turned four of this story's first tripwires green.
        for inlined in [
            "__ERROR_API:HTTP",
            "__ERROR_FORMAT:{message}",
            "__ERROR_NETWORK:{e}",
        ] {
            assert!(
                !body.contains(inlined),
                "an arm was re-inlined into nativeTranscribe ({inlined}); the ORDER would \
                 stop being a property of a function cargo test --lib can call"
            );
        }
    }
}
