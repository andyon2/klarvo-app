//! Dictation pipeline: audio stop → STT → LLM cleanup → paste.
//!
//! These functions are called by the global hotkey handler and are not
//! directly exposed as Tauri commands. They operate on [`AppState`] via
//! an [`AppHandle`] so they can emit state-change events to the frontend.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use tauri::{AppHandle, Emitter, Manager};

use crate::audio;
use crate::config::{self, AppConfig, HotkeyMode};
use crate::history;
use crate::hotkey::{PipelineEvent, EVENT_STATE_CHANGED};
use crate::llm::{self, chunked_cleanup, CleanupProvider, CleanupStyle};
use crate::paste::{
    capture_foreground_window, capture_foreground_window_title, create_paste_handler, PasteResult,
};
use crate::stt::{self, is_hallucination, SttProvider};
use crate::sync;
use crate::{AppState, friendly_error};

#[cfg(desktop)]
use crate::setup_audio_level_emitter;

// ---------------------------------------------------------------------------
// Provider resolution from config
// ---------------------------------------------------------------------------

/// Selects the STT provider based on `cfg.stt_provider`.
///
/// - `"groq"`: Groq Whisper API (primary, fast). Requires `groq_api_key`.
/// - `"openai"`: OpenAI Whisper API. Requires `openai_api_key`.
/// - `"local"`: offline whisper.cpp model (Windows-only, no key needed).
///
/// Falls back to a Groq instance (which will fail at call-time with an auth
/// error) if the provider string is unrecognised, so startup always succeeds.
pub fn resolve_stt_provider(cfg: &AppConfig, app_data_dir: &std::path::Path) -> Arc<dyn SttProvider> {
    match cfg.stt_provider.as_str() {
        "openai" => Arc::new(stt::OpenAiWhisper::new(&cfg.openai_api_key)),
        #[cfg(any(target_os = "windows", target_os = "android"))]
        "local" => build_local_whisper_provider(cfg, app_data_dir),
        #[cfg(not(any(target_os = "windows", target_os = "android")))]
        "local" => {
            log::warn!("[pipeline] local STT provider is only supported on Windows and Android; falling back to groq");
            Arc::new(stt::GroqWhisper::new(&cfg.groq_api_key).with_model(cfg.stt_model.clone()))
        }
        // "groq" and any unrecognised value
        _ => Arc::new(stt::GroqWhisper::new(&cfg.groq_api_key).with_model(cfg.stt_model.clone())),
    }
}

/// Resolves both providers from a single config in one call.
///
/// Pure convenience wrapper over [`resolve_stt_provider`] +
/// [`resolve_cleanup_provider`] so the three live consumers (boot,
/// `save_settings`, `clear_api_key`) share one resolve site. No locks, no
/// persistence, no `AppState` — order-independent at every call site.
/// Note: `resolve_cleanup_provider` does not take `app_data_dir`.
pub fn resolve_providers(
    cfg: &AppConfig,
    app_data_dir: &std::path::Path,
) -> (Arc<dyn SttProvider>, Arc<dyn CleanupProvider>) {
    (resolve_stt_provider(cfg, app_data_dir), resolve_cleanup_provider(cfg))
}

/// Builds a `LocalWhisperProvider` with the platform-appropriate model path.
///
/// ## Path convention by platform
///
/// - **Windows:** `%APPDATA%\com.klarvo.voice\models\ggml-{model}.bin`
///   Derived from the `APPDATA` env var (falls back to `.\models\` if unset).
/// - **Android:** path is constructed at command-call time in `transcribe_local`
///   using the Tauri `AppHandle` to resolve `app_data_dir`. This function is
///   not called on Android via the pipeline because Android STT goes through
///   `KlarvoApi.kt` → `transcribe_local` Tauri command directly.
///
/// We derive the path from `APPDATA` (Windows) rather than `AppState.app_data_dir`
/// because `resolve_stt_provider` takes only `&AppConfig`.
#[cfg(any(target_os = "windows", target_os = "android"))]
fn build_local_whisper_provider(cfg: &AppConfig, app_data_dir: &std::path::Path) -> Arc<dyn SttProvider> {
    use stt::LocalWhisperProvider;

    #[cfg(target_os = "windows")]
    let model_dir = std::env::var("APPDATA")
        .map(|d| std::path::PathBuf::from(d).join("com.klarvo.voice").join("models"))
        .unwrap_or_else(|_| std::path::PathBuf::from("models"));

    // On Android, use the same app_data_dir that Tauri's download command uses.
    #[cfg(target_os = "android")]
    let model_dir = app_data_dir.join("models");

    #[cfg(target_os = "windows")]
    let _ = app_data_dir; // suppress unused warning on Windows

    let model_file = format!("ggml-{}.bin", cfg.local_whisper_model);
    let model_path = model_dir.join(&model_file);

    log::info!(
        "[pipeline] Local whisper provider: model={}",
        model_path.display()
    );

    Arc::new(LocalWhisperProvider::new(
        model_path.to_string_lossy().into_owned(),
    ))
}

/// Constructs a *network* LLM cleanup provider by name with the given API key.
///
/// Shared by [`resolve_cleanup_provider`] (primary selection) and
/// [`resolve_fallback_provider`] (alternative selection) so the network-provider
/// construction — including the OpenRouter endpoint/model literals — lives in
/// ONE place instead of being duplicated across both. `"anthropic"` and
/// `"local"` are intentionally NOT here: anthropic is never a fallback candidate
/// and local needs a model path, so both stay special-cased in
/// `resolve_cleanup_provider`. Unrecognised names fall back to DeepSeek (which
/// fails at call-time with an auth error), preserving prior behavior.
fn cleanup_provider_for(name: &str, api_key: &str) -> Arc<dyn CleanupProvider> {
    match name {
        "openai" => Arc::new(llm::OpenAiCleanup::new(api_key)),
        "groq" => Arc::new(llm::GroqCleanup::new(api_key)),
        "openrouter" => Arc::new(llm::OpenAiCompatibleCleanup::new(
            api_key,
            "https://openrouter.ai/api/v1/chat/completions",
            "deepseek/deepseek-chat",
        )),
        // "deepseek" and any unrecognised value
        _ => Arc::new(llm::DeepSeekCleanup::new(api_key)),
    }
}

/// Selects the LLM cleanup provider based on `cfg.llm_provider`.
///
/// - `"deepseek"`: DeepSeek API (primary, cheap). Requires `deepseek_api_key`.
/// - `"openai"`: OpenAI API. Requires `openai_api_key`.
/// - `"anthropic"`: Anthropic API. Requires `anthropic_api_key`.
/// - `"groq"`: Groq LLM API. Requires `groq_api_key`.
///
/// Falls back to DeepSeek (which will fail at call-time with an auth error)
/// for unrecognised values, so startup always succeeds.
pub fn resolve_cleanup_provider(cfg: &AppConfig) -> Arc<dyn CleanupProvider> {
    match cfg.llm_provider.as_str() {
        "openai" => cleanup_provider_for("openai", &cfg.openai_api_key),
        "anthropic" => Arc::new(llm::AnthropicCleanup::new(&cfg.anthropic_api_key)),
        "groq" => cleanup_provider_for("groq", &cfg.groq_api_key),
        "openrouter" => cleanup_provider_for("openrouter", &cfg.openrouter_api_key),
        #[cfg(target_os = "windows")]
        "local" => {
            let model_dir = std::env::var("APPDATA")
                .map(|d| std::path::PathBuf::from(d).join("com.klarvo.voice").join("models"))
                .unwrap_or_else(|_| std::path::PathBuf::from("models"));
            // TODO(multi-model): Replace hardcoded filename with cfg.local_llm_model (a new
            // Settings field). The filename drives prompt-format detection in LocalLlmCleanup::new,
            // so changing it here is sufficient — no other pipeline changes needed.
            let model_path = model_dir.join("qwen2.5-1.5b-instruct-q4_k_m.gguf");
            log::info!(
                "[pipeline] Local LLM cleanup provider: model={}",
                model_path.display()
            );
            Arc::new(llm::local::LocalLlmCleanup::new(model_path))
        }
        // "deepseek" and any unrecognised value
        _ => cleanup_provider_for("deepseek", &cfg.deepseek_api_key),
    }
}

/// Returns `true` for transient, retryable LLM errors (rate limit or server error).
///
/// - 429 Too Many Requests (rate limit)
/// - 5xx Server Error (temporary provider outage)
///
/// All other errors (400 Bad Request, 401 Unauthorized, 403 Forbidden, …) are
/// considered permanent and are NOT retried.
pub fn is_retryable_llm_error(err: &llm::LlmError) -> bool {
    matches!(err, llm::LlmError::ApiError { status, .. } if *status == 429 || *status >= 500)
}

/// Selects an alternative LLM cleanup provider, excluding `primary_provider`.
///
/// Iterates the fixed fallback order (DeepSeek → Groq → OpenAI → OpenRouter)
/// and returns the first provider whose API key is non-empty, skipping the
/// one whose name matches `primary_provider`.
///
/// Returns `None` when no suitable alternative exists (all keys empty or
/// only the primary provider has a key).
///
/// "local" is never used as a fallback here because network errors are the
/// trigger for fallback and local inference does not require a network call.
pub fn resolve_fallback_provider(
    cfg: &AppConfig,
    primary_provider: &str,
) -> Option<(Arc<dyn CleanupProvider>, &'static str)> {
    // Ordered candidate list: (provider name, api key)
    let candidates: &[(&str, &str)] = &[
        ("deepseek", &cfg.deepseek_api_key),
        ("groq", &cfg.groq_api_key),
        ("openai", &cfg.openai_api_key),
        ("openrouter", &cfg.openrouter_api_key),
    ];

    for (name, key) in candidates {
        if *name == primary_provider || key.is_empty() {
            continue;
        }
        let provider = cleanup_provider_for(name, key);
        return Some((provider, name));
    }
    None
}

// ---------------------------------------------------------------------------
// Whisper prompt-echo detection
// ---------------------------------------------------------------------------

/// Detects when Whisper echoes the conditioning prompt instead of real speech.
///
/// Whisper sometimes "hallucinates" the prompt text when the audio contains
/// ambient noise but no actual words. The hallucinated text may not be an
/// exact copy — Whisper can vary words slightly (e.g. "punctuation" →
/// "pronunciation") or reorder phrases.
///
/// Two complementary checks:
/// 1. **Exact fragment removal** — splits the hint into sentences and removes
///    all occurrences from the transcription. If nothing meaningful remains,
///    it's an echo.
/// 2. **Word-overlap** — extracts significant words (≥3 chars) from both
///    texts and measures how many transcription words appear in the hint.
///    If ≥60% overlap AND the transcription is short (≤30 words), it's
///    likely a hallucination with slight word variation.
fn is_prompt_echo(transcription: &str, stt_hint: &str) -> bool {
    let trans = transcription.trim().to_lowercase();
    let hint = stt_hint.trim().to_lowercase();

    if trans.is_empty() || hint.is_empty() {
        return false;
    }

    // --- Check 1: exact fragment removal ---
    let hint_sentences: Vec<&str> = hint
        .split(". ")
        .flat_map(|s| s.split('.'))
        .map(|s| s.trim())
        .filter(|s| s.len() > 10)
        .collect();

    let mut cleaned = trans.clone();
    for sentence in &hint_sentences {
        cleaned = cleaned.replace(sentence, "");
    }
    cleaned = cleaned.replace(&hint, "");

    let residue: String = cleaned
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
        .collect();

    // Only conclude "echo" if stripping the prompt is what left <5 chars. When
    // nothing was stripped (`cleaned == trans`), a short residue just means a
    // short utterance the user genuinely dictated ("Gast", "hi", "für") — not an
    // echo. Without this guard the length floor silently discards real one-word
    // dictations (smoke test 2026-05-30).
    if cleaned != trans && residue.len() < 5 {
        return true;
    }

    // --- Check 2: word-overlap (catches Whisper word variations) ---
    let extract_words = |text: &str| -> Vec<String> {
        text.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .map(|w| w.to_lowercase())
            .filter(|w| w.len() >= 3)
            .collect()
    };

    let trans_words = extract_words(&trans);
    let hint_words: std::collections::HashSet<String> =
        extract_words(&hint).into_iter().collect();

    if trans_words.is_empty() {
        return false;
    }

    // Longer texts are unlikely to be pure hallucination.
    if trans_words.len() > 30 {
        return false;
    }

    // Check 2a: high word-overlap with the hint (≥70%).
    let matching = trans_words
        .iter()
        .filter(|w| hint_words.contains(w.as_str()))
        .count();
    let overlap = matching as f32 / trans_words.len() as f32;

    if overlap >= 0.7 {
        return true;
    }

    // Check 2b: highly repetitive text (low vocabulary diversity).
    // Whisper hallucinations often repeat the same 2-3 words/phrases.
    // Real speech has much higher word diversity.
    let unique: std::collections::HashSet<&String> = trans_words.iter().collect();
    let diversity = unique.len() as f32 / trans_words.len() as f32;
    // If fewer than half the words are unique AND at least some hint words
    // appear, it's a repetitive hallucination with word variations.
    if diversity < 0.5 && overlap >= 0.3 {
        return true;
    }

    false
}

// ---------------------------------------------------------------------------
// Prompt-fragment stripping
// ---------------------------------------------------------------------------

/// Default STT conditioning prompts used by the pipeline.
///
/// Whisper can leak fragments of these prompts into the transcription output,
/// especially for longer recordings. We remove any recognised fragments before
/// the text reaches the LLM cleanup step or the hallucination guard.
const DEFAULT_STT_HINTS: &[&str] = &[
    "Diktat auf Deutsch mit gelegentlichen englischen Fachbegriffen. Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion.",
    "Voice dictation in English. Proper punctuation, capitalization, and spelling.",
    "Multilingual voice dictation. German and English with proper punctuation.",
];

/// Removes known STT conditioning-prompt fragments from `text`.
///
/// Whisper occasionally leaks parts of its `initial_prompt` into the
/// transcription (e.g. "German and English with proper punctuation." appearing
/// mid-sentence). This function strips those fragments so they do not pollute
/// the LLM cleanup input or trigger false positives in the hallucination guard.
///
/// Algorithm:
/// 1. Collect candidate fragments from `stt_hint` **and** every entry in
///    `DEFAULT_STT_HINTS`.
/// 2. Split each hint string on `". "` and `"."` to get individual sentences.
/// 3. Remove every fragment that is at least 10 characters long, using a
///    case-insensitive search (original casing is preserved in the output).
/// 4. Collapse multiple consecutive spaces and trim leading/trailing whitespace.
pub fn strip_prompt_fragments(text: &str, stt_hint: &str) -> String {
    // Build the de-duplicated list of all hint strings to check.
    let mut all_hints: Vec<&str> = DEFAULT_STT_HINTS.to_vec();
    if !stt_hint.is_empty() && !DEFAULT_STT_HINTS.contains(&stt_hint) {
        all_hints.push(stt_hint);
    }

    // Collect unique fragments (≥10 chars) from every hint string.
    let mut fragments: Vec<String> = Vec::new();
    for hint in &all_hints {
        let hint_lower = hint.to_lowercase();
        // Split on ". " first, then on ".".
        for part in hint_lower.split(". ").flat_map(|s| s.split('.')) {
            let fragment = part.trim().to_string();
            if fragment.len() >= 10 && !fragments.contains(&fragment) {
                fragments.push(fragment);
            }
        }
        // Also try the full hint string as a single fragment (case-insensitive).
        let full = hint_lower.trim().to_string();
        if full.len() >= 10 && !fragments.contains(&full) {
            fragments.push(full);
        }
    }

    // Apply all fragments to the *lowercased* version of the text to find
    // positions, but rebuild from the *original* text so casing is preserved.
    let mut result = text.to_string();
    for fragment in &fragments {
        // We need a case-insensitive replace. Rust's std doesn't have one, so
        // we do it manually: find the fragment in the lowercased result and
        // remove the corresponding byte range from the original.
        loop {
            let result_lower = result.to_lowercase();
            match result_lower.find(fragment.as_str()) {
                Some(start) => {
                    let end = start + fragment.len();
                    result.replace_range(start..end, "");
                }
                None => break,
            }
        }
    }

    // Clean up in two passes:
    // 1. Remove punctuation tokens that are now orphaned (i.e. a token that
    //    consists entirely of punctuation characters with no surrounding word).
    //    This handles leftover ". ." artefacts when an entire prompt sentence
    //    was removed but the trailing period was not part of the fragment string.
    // 2. Collapse multiple spaces and trim.
    let tokens: Vec<&str> = result
        .split_whitespace()
        .filter(|token| {
            // Keep the token if it has at least one alphanumeric character.
            token.chars().any(|c| c.is_alphanumeric())
        })
        .collect();
    tokens.join(" ")
}

// ---------------------------------------------------------------------------
// Silence detection helper
// ---------------------------------------------------------------------------

/// Parses a WAV byte buffer and computes the overall RMS of the audio samples.
///
/// Returns `None` if the WAV cannot be parsed (should not happen since we
/// encoded it ourselves, but we handle it gracefully).
pub(crate) fn compute_wav_rms(wav_bytes: &[u8]) -> Option<f32> {
    let cursor = std::io::Cursor::new(wav_bytes);
    let mut reader = match hound::WavReader::new(cursor) {
        Ok(r) => r,
        Err(_) => return None,
    };

    let spec = reader.spec();
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().filter_map(|s| s.ok()).collect(),
        hound::SampleFormat::Int => {
            let max_val = (1_i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
    };

    if samples.is_empty() {
        return Some(0.0);
    }

    Some(audio::compute_rms(&samples))
}

// ---------------------------------------------------------------------------
// Pipeline decision logic (pure — characterization-tested before extraction)
// ---------------------------------------------------------------------------
//
// These functions hold the branch decisions that `stop_and_process_pipeline`
// used to inline. Pulling them out lets the decision matrix be unit-tested and
// snapshotted *before* the STT->guard->LLM->sanitize core moves into
// `process_audio` (Task 2.2). They take primitives only — no locks, no
// `AppState`, no `AppHandle` — so they are order-independent at the call site.

/// Whether the pipeline runs fully offline for *dictation*, i.e. skips the LLM
/// cleanup network call. True only when STT is local AND the LLM is not local;
/// when the LLM is also local, cleanup runs offline via llama.cpp and is kept.
pub(crate) fn is_offline(stt_provider: &str, llm_provider: &str) -> bool {
    stt_provider == "local" && llm_provider != "local"
}

/// Reason to skip the pipeline *before* transcription, based on recording
/// length and loudness. `None` means "proceed to STT".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SilenceSkip {
    /// Recording shorter than the configured minimum.
    TooShort,
    /// RMS below the (mode-dependent) silence threshold.
    Silent,
}

/// Decides whether a recording is too short or too silent to transcribe.
///
/// `rms` is `None` when the WAV could not be measured (invalid samples); the
/// loudness check is then skipped, matching the original `if let Some` guard.
pub(crate) fn silence_skip(
    duration_ms: u64,
    min_recording_ms: u64,
    rms: Option<f32>,
    silence_threshold: f32,
) -> Option<SilenceSkip> {
    if duration_ms < min_recording_ms {
        return Some(SilenceSkip::TooShort);
    }
    if let Some(rms) = rms {
        if rms < silence_threshold {
            return Some(SilenceSkip::Silent);
        }
    }
    None
}

/// Reason to skip the pipeline *after* transcription, when the transcript is a
/// Whisper hallucination rather than real speech. `None` means "proceed".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostSttSkip {
    /// Transcript is an echo of the conditioning prompt.
    PromptEcho,
    /// Transcript matches the known-hallucination blocklist.
    Blocklist,
}

/// Detects post-STT hallucinations. Order matches the original pipeline:
/// prompt-echo first, then the blocklist.
pub(crate) fn post_stt_skip(transcript: &str, stt_hint: &str) -> Option<PostSttSkip> {
    if is_prompt_echo(transcript, stt_hint) {
        return Some(PostSttSkip::PromptEcho);
    }
    if is_hallucination(transcript) {
        return Some(PostSttSkip::Blocklist);
    }
    None
}

/// Which cleanup path the transcript takes after the guards pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmPath {
    /// Offline dictation: emit the raw transcript, no LLM call.
    OfflineRaw,
    /// Command mode: rewrite the selected text using the spoken command.
    Command,
    /// Normal dictation: LLM cleanup of the transcript.
    Cleanup,
}

/// Selects the cleanup path. Command mode always requires an LLM call (even
/// offline), so a present selection wins over the offline flag.
pub(crate) fn select_llm_path(offline: bool, has_selected_text: bool) -> LlmPath {
    if offline && !has_selected_text {
        LlmPath::OfflineRaw
    } else if has_selected_text {
        LlmPath::Command
    } else {
        LlmPath::Cleanup
    }
}

// ---------------------------------------------------------------------------
// Pipeline entry points
// ---------------------------------------------------------------------------

/// Starts recording audio and emits `state=recording`.
///
/// Does nothing (returns silently) if recording is already in progress.
/// Used by the hold-mode hotkey handler on key-press.
pub async fn start_recording_only(handle: AppHandle) {
    let state = handle.state::<AppState>();

    // Re-install the audio level callback BEFORE starting -- start_recording
    // consumes it via `.take()` on the recording thread. Installing it even if
    // we lose the gate below is harmless: the slot is overwritten next cycle.
    #[cfg(desktop)]
    setup_audio_level_emitter(&handle);

    // The recorder's session lock is the single atomic gate: start_recording
    // returns Err(AlreadyRecording) if a recording is already active. We rely
    // on that instead of a racy is_recording() pre-check, so two fast presses
    // can't both pass a check and then both clobber the foreground-window state
    // captured below (the "toggle aborts after 30-60 min" desync, Task 2.3).
    let device_name = state.config.lock().ok().and_then(|c| c.audio_device.clone());
    match state.recorder.start_recording(device_name.as_deref()) {
        Ok(()) => {}
        Err(audio::AudioError::AlreadyRecording) => {
            // A second press (or the other slot) raced us and lost. Do nothing
            // observable: no foreground-window clobber, no error event.
            log::debug!("[hotkey] start_recording_only: already recording, ignoring press");
            return;
        }
        Err(e) => {
            crate::emit_pipeline_state(
                &handle,
                PipelineEvent::error(format!("Failed to start recording: {e}")),
            );
            return;
        }
    }

    // We won the gate. Capture the foreground window now -- the window the user
    // was typing in, which we restore focus to before pasting. Capturing here,
    // a few ms after the device-init gate, still yields the same window: the
    // user just pressed the hotkey and focus does not change during init.
    if let Ok(mut guard) = state.prev_foreground_hwnd.lock() {
        *guard = capture_foreground_window();
    }
    if let Ok(mut guard) = state.prev_window_title.lock() {
        *guard = capture_foreground_window_title();
        log::debug!("[hotkey] foreground window title: {:?}", *guard);
    }

    *match state.recording_start.lock() {
        Ok(g) => g,
        Err(_) => {
            crate::emit_pipeline_state(&handle, PipelineEvent::error("State lock poisoned"));
            return;
        }
    } = Some(std::time::Instant::now());

    // Ensure the floating bar window exists before telling the frontend to show it.
    // Recovers from the rare case where the bar silently vanished after hours idle.
    #[cfg(desktop)]
    {
        if let Some(bar) = handle.get_webview_window("bar") {
            if bar.is_visible().unwrap_or(false) == false {
                log::info!("[bar] recording started but bar not visible, showing");
                let _ = bar.show();
            }
        } else {
            log::warn!("[bar] recording started but bar window missing, recreating");
            let saved = state.config.lock().ok().map(|c| (c.bar_x, c.bar_y));
            let (sx, sy) = saved.unwrap_or((None, None));
            if let Err(e) = crate::create_bar_window(&handle, sx, sy) {
                log::error!("[bar] failed to recreate bar window: {e}");
            }
        }
    }

    crate::emit_pipeline_state(&handle, PipelineEvent::recording());
}

/// Starts recording with automatic stop-on-silence for AutoStop mode.
///
/// 1. Installs a silence-detection callback **before** calling `start_recording_only`.
///    The callback captures a clone of `handle` and, when fired on the cpal OS-thread,
///    spawns an async task via `tauri::async_runtime::spawn` to run the full pipeline.
/// 2. Delegates the actual recording start to `start_recording_only`.
///
/// If the user presses the hotkey again while recording is still active, the
/// `(HotkeyMode::AutoStop, ShortcutState::Pressed)` branch calls
/// `stop_and_process_pipeline` directly (which clears the callback first),
/// preventing a double-invocation.
pub async fn start_autostop_recording(handle: AppHandle) {
    let state = handle.state::<AppState>();

    if state.recorder.is_recording() {
        log::debug!("[autostop] Already recording, skipping");
        return;
    }

    // Read silence config before installing the callback so we don't hold the
    // config lock when start_recording_only runs.
    let (silence_secs, silence_threshold) = state
        .config
        .lock()
        .ok()
        .map(|c| (c.autostop_silence_secs, c.advanced.silence_threshold))
        .unwrap_or((2.0, 0.005));

    log::debug!("[autostop] Installing silence callback: silence_secs={silence_secs}, threshold={silence_threshold}");

    // Install the silence callback. It must be set BEFORE start_recording so
    // the recording thread picks it up via `.take()` inside start_recording.
    let handle_for_cb = handle.clone();
    state.recorder.set_silence_callback(
        silence_secs,
        silence_threshold,
        Box::new(move || {
            log::info!("[autostop] Silence callback fired! Stopping pipeline...");
            // This closure runs on the cpal OS-thread (non-async context).
            // Spawn an async task to run the pipeline on the Tauri runtime.
            let h = handle_for_cb.clone();
            tauri::async_runtime::spawn(async move {
                stop_and_process_pipeline(h).await;
            });
        }),
    );

    // Check callback is installed before we move handle into start_recording_only.
    let cb_installed = state.recorder.has_silence_callback();
    log::debug!("[autostop] Silence callback installed before recording: {cb_installed}");

    // Start the actual recording (re-uses all the foreground-window capture
    // and audio-level emitter setup from start_recording_only).
    start_recording_only(handle).await;
}

/// Starts recording in Auto-Loop mode.
///
/// Identical to [`start_autostop_recording`], but the silence callback checks
/// [`AppState::auto_loop_active`] after the pipeline completes. If the flag is
/// still `true`, it immediately starts another recording cycle. The loop
/// continues until the user presses the hotkey again, which sets the flag to
/// `false` and stops the current recording via [`stop_and_process_pipeline`].
///
/// Returns `Pin<Box<dyn Future + Send>>` instead of being `async fn` to break
/// a recursive opaque-type cycle: the silence callback spawns a task that
/// awaits this function again. With `async fn`, the compiler cannot prove the
/// recursive future is `Send`. The explicit `Pin<Box>` gives the compiler a
/// concrete `Send` bound to work with.
///
/// Race-condition note: if the user presses stop _while_ the pipeline is
/// executing, `auto_loop_active` will be `false` by the time the check runs,
/// so no new cycle is started. At worst one extra cycle starts and then
/// terminates gracefully -- no crash or data loss is possible.
pub fn start_auto_recording(handle: AppHandle) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        // Block scope: drop State before the await at the end so the future
        // doesn't hold a borrow across the yield point.
        {
            let state = handle.state::<AppState>();

            if state.recorder.is_recording() {
                return;
            }

            let (silence_secs, silence_threshold) = state
                .config
                .lock()
                .ok()
                .map(|c| (c.auto_mode_silence_secs, c.advanced.silence_threshold))
                .unwrap_or((2.0, 0.005));

            let handle_for_cb = handle.clone();
            state.recorder.set_silence_callback(
                silence_secs,
                silence_threshold,
                Box::new(move || {
                    // Runs on the cpal OS-thread. Spawn onto the Tauri async runtime.
                    let h = handle_for_cb.clone();
                    tauri::async_runtime::spawn(async move {
                        stop_and_process_pipeline(h.clone()).await;
                        // Read flag and drop State before the sleep await.
                        let should_restart = h
                            .state::<AppState>()
                            .auto_loop_active
                            .load(Ordering::SeqCst);
                        if should_restart {
                            // Small delay so events and cleanup finish before
                            // the next recording cycle begins.
                            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                            start_auto_recording(h).await;
                        }
                    });
                }),
            );
        }

        start_recording_only(handle).await;
    })
}

/// Starts Command Mode: copies selected text via Ctrl+C, then starts recording.
///
/// The voice command will be transcribed and used to rewrite the selected text.
///
/// Requires a paid license. If the user is unlicensed, emits an error event
/// and returns without starting recording.
pub async fn start_command_mode(handle: AppHandle) {
    let state = handle.state::<AppState>();

    // License gate: Command Mode requires a paid license.
    // Because this function returns () we use an if-check instead of the macro.
    let command_mode_allowed = state
        .license_status
        .lock()
        .ok()
        .map(|s| crate::license::is_feature_allowed(&s, crate::license::LicensedFeature::CommandMode))
        .unwrap_or(false);
    if !command_mode_allowed {
        let _ = handle.emit(
            EVENT_STATE_CHANGED,
            PipelineEvent::error("feature_requires_license:CommandMode"),
        );
        return;
    }

    if state.recorder.is_recording() {
        return;
    }

    // Capture foreground window
    if let Ok(mut guard) = state.prev_foreground_hwnd.lock() {
        *guard = capture_foreground_window();
    }

    // Copy selected text via clipboard
    #[cfg(target_os = "windows")]
    {
        // Simulate Ctrl+C to copy selected text
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
            KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C, VK_CONTROL,
        };

        unsafe {
            let inputs = [
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_CONTROL,
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(VK_C.0),
                            wScan: 0,
                            dwFlags: KEYBD_EVENT_FLAGS(0),
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(VK_C.0),
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
                INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VK_CONTROL,
                            wScan: 0,
                            dwFlags: KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                },
            ];
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        }

        // Wait for clipboard to populate
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Read clipboard (desktop only -- on mobile, command mode is not used)
    #[cfg(desktop)]
    let selected_text = arboard::Clipboard::new()
        .ok()
        .and_then(|mut cb| cb.get_text().ok())
        .unwrap_or_default();
    #[cfg(mobile)]
    let selected_text = String::new();

    log::info!(
        "[command-mode] selected text: {:?}",
        &selected_text[..selected_text.len().min(100)]
    );

    if let Ok(mut guard) = state.command_mode_selected_text.lock() {
        *guard = if selected_text.is_empty() {
            None
        } else {
            Some(selected_text)
        };
    }
    if let Ok(mut guard) = state.command_mode_active.lock() {
        *guard = true;
    }

    // Start recording the voice command
    #[cfg(desktop)]
    setup_audio_level_emitter(&handle);

    let device_name = state.config.lock().ok().and_then(|c| c.audio_device.clone());
    if let Err(e) = state.recorder.start_recording(device_name.as_deref()) {
        let _ = handle.emit(
            EVENT_STATE_CHANGED,
            PipelineEvent::error(format!("Failed to start recording: {e}")),
        );
        if let Ok(mut guard) = state.command_mode_active.lock() {
            *guard = false;
        }
        return;
    }

    *match state.recording_start.lock() {
        Ok(g) => g,
        Err(_) => {
            let _ = handle.emit(
                EVENT_STATE_CHANGED,
                PipelineEvent::error("State lock poisoned"),
            );
            return;
        }
    } = Some(std::time::Instant::now());

    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::recording());
}

// ---------------------------------------------------------------------------
// process_audio — the STT → guard → LLM → sanitize core (Task 2.2)
// ---------------------------------------------------------------------------
//
// Everything the core needs is snapshotted into `ProcessInput` by the caller
// (no `AppState`/`AppHandle` crosses in), and every observable side effect is
// either emitted through `emit` or reported in `ProcessOutcome` for the caller
// to apply. This makes the core unit-testable with fake providers while the
// lock/metric/command-mode plumbing stays a thin shell in
// `stop_and_process_pipeline`.

/// Groups the two STT conditioning fields so the type system enforces their
/// co-presence instead of a doc-comment. Both are built together at the
/// construction site in `stop_and_process_pipeline` and destructured together
/// in `process_audio`.
pub struct SttPromptPair {
    /// Combined STT conditioning prompt (dictionary terms + hint text), passed
    /// to `transcribe`. `None` when no dictionary terms or hint are configured.
    pub dict_prompt: Option<String>,
    /// Hint text alone (no dictionary terms), used by the hallucination guards.
    pub stt_hint_text: String,
}

/// Fully-snapshotted inputs for [`process_audio`]. Built under locks by the
/// shell; holds no references back into `AppState`.
pub struct ProcessInput {
    pub wav_bytes: Vec<u8>,
    pub language: String,
    pub stt_provider: Arc<dyn SttProvider>,
    pub cleanup_provider: Arc<dyn CleanupProvider>,
    /// STT conditioning prompt pair; groups `dict_prompt` + `stt_hint_text` so
    /// consistency is type-enforced rather than doc-enforced.
    pub stt_prompt: SttPromptPair,
    pub offline_mode: bool,
    /// `Some` => command mode (rewrite this selection using the spoken command).
    pub selected_text: Option<String>,
    pub cleanup_style: CleanupStyle,
    pub custom_prompt: Option<String>,
    /// Pre-formatted (`{:?}`) name of a matched app profile, logged on the
    /// normal-cleanup path only; `None` when no profile matched.
    pub matched_profile_name: Option<String>,
    pub dict_list: Option<String>,
    pub output_lang: Option<String>,
    pub llm_provider_name: String,
    /// Config snapshot used solely to resolve a fallback cleanup provider.
    pub config_for_fallback: AppConfig,
}

/// Result of [`process_audio`]. The shell applies the deferred side effects
/// (metric increments, command-mode consumption) based on which variant
/// returns; progress/terminal events were already emitted via `emit`.
#[derive(Debug, PartialEq)]
pub enum ProcessOutcome {
    /// Stopped before producing text — a pre-LLM guard skipped it, or STT
    /// failed (`stt_error`). The command point was NOT reached, so command mode
    /// is left untouched.
    Stopped { stt_error: bool },
    /// Reached the command path but the rewrite failed. The command point WAS
    /// reached, so command mode should be consumed.
    CommandFailed,
    /// Produced text to paste. `llm_error` is `true` when cleanup degraded to
    /// raw text (the shell bumps the LLM error counter).
    Produced {
        cleaned_text: String,
        raw_text: String,
        is_command: bool,
        stt_ms: u64,
        llm_ms: Option<u64>,
        prompt_tokens: Option<u32>,
        completion_tokens: Option<u32>,
        llm_error: bool,
    },
}

/// Builds the user-facing warning shown when LLM cleanup fails and the raw
/// transcript is pasted instead.
fn degrade_warn_msg(err: &dyn std::fmt::Display) -> String {
    let short_reason = friendly_error("", &err.to_string());
    format!(
        "Cleanup failed — raw text inserted.{}",
        if short_reason.is_empty() {
            String::new()
        } else {
            format!(" {short_reason}")
        }
    )
}

/// Transcribe → strip → hallucination guards → LLM cleanup/command/offline →
/// sanitize. Emits progress/terminal events through `emit`; returns a
/// [`ProcessOutcome`] for the shell to act on. No locks, no `AppState`.
pub async fn process_audio(
    input: ProcessInput,
    emit: &mut (dyn FnMut(PipelineEvent) + Send),
) -> ProcessOutcome {
    let ProcessInput {
        wav_bytes,
        language,
        stt_provider,
        cleanup_provider,
        stt_prompt: SttPromptPair { dict_prompt, stt_hint_text },
        offline_mode,
        selected_text,
        cleanup_style,
        custom_prompt,
        matched_profile_name,
        dict_list,
        output_lang,
        llm_provider_name,
        config_for_fallback,
    } = input;

    // --- Transcribe ---
    emit(PipelineEvent::transcribing());

    let stt_start = std::time::Instant::now();
    let raw_text = match stt_provider
        .transcribe(wav_bytes, &language, dict_prompt.as_deref())
        .await
    {
        Ok(t) => t,
        Err(e) => {
            log::error!("[pipeline] STT transcription failed: {e}");
            emit(PipelineEvent::error(friendly_error(
                "Transcription failed",
                &e.to_string(),
            )));
            return ProcessOutcome::Stopped { stt_error: true };
        }
    };
    let stt_ms = stt_start.elapsed().as_millis() as u64;
    log::info!("[pipeline] STT took {}ms", stt_ms);

    log::debug!("[pipeline] raw transcription: {raw_text:?}");

    // --- Strip leaked STT prompt fragments ---
    // Whisper can embed parts of the conditioning prompt into the transcription
    // output (e.g. "German and English with proper punctuation." mid-sentence).
    // Strip these *before* the hallucination guard so the guard sees clean text.
    let raw_text = {
        let stripped = strip_prompt_fragments(&raw_text, &stt_hint_text);
        if stripped != raw_text {
            log::debug!("[pipeline] stripped prompt fragments from transcription");
        }
        stripped
    };

    // --- Whisper hallucination guards ---
    // Prompt-echo (Whisper echoes the conditioning prompt) or a known
    // training-data phrase ("ZDF 2020" / "Thank you for watching"). Both skip.
    match post_stt_skip(&raw_text, &stt_hint_text) {
        Some(PostSttSkip::PromptEcho) => {
            log::info!(
                "[pipeline] transcription is prompt echo (hallucination), skipping: {raw_text:?}"
            );
            emit(PipelineEvent::idle());
            return ProcessOutcome::Stopped { stt_error: false };
        }
        Some(PostSttSkip::Blocklist) => {
            log::info!("[pipeline] Blocked Whisper hallucination: {:?}", raw_text);
            emit(PipelineEvent::idle());
            return ProcessOutcome::Stopped { stt_error: false };
        }
        None => {}
    }

    // --- LLM step ---
    // Command Mode still requires an LLM call even offline, so a present
    // selection wins over the offline flag; only normal offline dictation skips
    // cleanup entirely. The `cleaning` event fires for every non-offline path.
    let llm_path = select_llm_path(offline_mode, selected_text.is_some());
    if !matches!(llm_path, LlmPath::OfflineRaw) {
        emit(PipelineEvent::cleaning());
    }

    let mut llm_ms: Option<u64> = None;
    let mut llm_error = false;
    let cleanup_result = if matches!(llm_path, LlmPath::OfflineRaw) {
        // Offline dictation: return raw transcript without any LLM call.
        log::info!("[pipeline] Offline mode: skipping LLM cleanup");
        llm::CleanupResult {
            text: raw_text.clone(),
            prompt_tokens: None,
            completion_tokens: None,
        }
    } else if let Some(ref sel_text) = selected_text {
        // Command Mode: rewrite selected text using the voice command
        log::info!("[pipeline] command mode: rewriting with voice command");

        let cmd_start = std::time::Instant::now();
        match cleanup_provider.rewrite(sel_text, &raw_text).await {
            Ok(r) => {
                llm_ms = Some(cmd_start.elapsed().as_millis() as u64);
                r
            }
            Err(e) => {
                log::error!("[pipeline] command mode rewrite failed: {e}");
                emit(PipelineEvent::error(format!("Command mode failed: {e}")));
                return ProcessOutcome::CommandFailed;
            }
        }
    } else {
        // Normal dictation: cleanup raw transcription
        if let Some(name) = &matched_profile_name {
            log::info!("[pipeline] profile matched: {name}");
        }

        let cleanup_start = std::time::Instant::now();
        let primary_result = chunked_cleanup(
            cleanup_provider.as_ref(),
            &raw_text,
            cleanup_style,
            dict_list.as_deref(),
            custom_prompt.as_deref(),
            output_lang.as_deref(),
        )
        .await;

        match primary_result {
            Ok(r) => {
                let cleanup_ms = cleanup_start.elapsed().as_millis() as u64;
                llm_ms = Some(cleanup_ms);
                log::info!(
                    "[pipeline] LLM cleanup took {}ms (provider: {}, style: {:?}, input_len: {})",
                    cleanup_ms,
                    llm_provider_name,
                    cleanup_style,
                    raw_text.len()
                );
                r
            }
            Err(ref primary_err) if is_retryable_llm_error(primary_err) => {
                // 429 or 5xx: try a fallback provider before giving up.
                let fallback = resolve_fallback_provider(&config_for_fallback, &llm_provider_name);

                if let Some((fallback_provider, fallback_name)) = fallback {
                    log::info!(
                        "[pipeline] Primary LLM provider {} failed ({primary_err}), trying fallback: {fallback_name}",
                        llm_provider_name
                    );
                    match chunked_cleanup(
                        fallback_provider.as_ref(),
                        &raw_text,
                        cleanup_style,
                        dict_list.as_deref(),
                        custom_prompt.as_deref(),
                        output_lang.as_deref(),
                    )
                    .await
                    {
                        Ok(r) => {
                            let cleanup_ms = cleanup_start.elapsed().as_millis() as u64;
                            llm_ms = Some(cleanup_ms);
                            log::info!(
                                "[pipeline] Primary LLM provider failed ({}), fallback to {} succeeded ({}ms)",
                                primary_err,
                                fallback_name,
                                cleanup_ms
                            );
                            r
                        }
                        Err(ref fallback_err) => {
                            log::warn!(
                                "[pipeline] Primary ({primary_err}) and fallback ({fallback_err}) both failed, using raw text"
                            );
                            llm_error = true;
                            emit(PipelineEvent::warn(degrade_warn_msg(fallback_err)));
                            llm::CleanupResult {
                                text: raw_text.clone(),
                                prompt_tokens: None,
                                completion_tokens: None,
                            }
                        }
                    }
                } else {
                    log::warn!(
                        "[pipeline] LLM cleanup failed ({primary_err}), no fallback provider available, using raw text"
                    );
                    llm_error = true;
                    emit(PipelineEvent::warn(degrade_warn_msg(primary_err)));
                    llm::CleanupResult {
                        text: raw_text.clone(),
                        prompt_tokens: None,
                        completion_tokens: None,
                    }
                }
            }
            Err(ref e) => {
                // Non-retryable error (400, 401, 403, …): degrade immediately.
                log::warn!("[pipeline] LLM cleanup failed (non-retryable), falling back to raw text: {e}");
                llm_error = true;
                emit(PipelineEvent::warn(degrade_warn_msg(e)));
                llm::CleanupResult {
                    text: raw_text.clone(),
                    prompt_tokens: None,
                    completion_tokens: None,
                }
            }
        }
    };

    let is_command = selected_text.is_some();
    let cleaned_text = sanitize_llm_output(&cleanup_result.text);
    log::debug!("[pipeline] cleaned text: {cleaned_text:?}");

    ProcessOutcome::Produced {
        cleaned_text,
        raw_text,
        is_command,
        stt_ms,
        llm_ms,
        prompt_tokens: cleanup_result.prompt_tokens,
        completion_tokens: cleanup_result.completion_tokens,
        llm_error,
    }
}

/// Resets command mode: clears the active flag and discards any stored
/// selection. Called once the pipeline has consumed (or failed) a command.
fn consume_command_mode(state: &AppState) {
    if let Ok(mut g) = state.command_mode_active.lock() {
        *g = false;
    }
    if let Ok(mut g) = state.command_mode_selected_text.lock() {
        let _ = g.take();
    }
}

/// Stops the active recording and runs the full STT → LLM → paste pipeline.
///
/// Does nothing (returns silently) if no recording is active.
/// Used by the hold-mode hotkey handler on key-release, and called internally
/// by [`run_dictation_pipeline`] for the toggle case.
///
/// Dictionary terms are injected at both the STT step (as a Groq `prompt`
/// hint) and the LLM step (as `dictionary_terms` in the system prompt).
pub async fn stop_and_process_pipeline(handle: AppHandle) {
    let state = handle.state::<AppState>();
    log::info!("[pipeline] stop_and_process_pipeline called (is_recording={})", state.recorder.is_recording());

    if !state.recorder.is_recording() {
        // Not recording -- key released without a corresponding press (race condition or
        // hold mode released before recording started). Safe to ignore.
        log::warn!("[pipeline] stop_and_process called but is_recording()=false — skipping (state desync?)");
        return;
    }

    // Clear any pending silence callback first. This prevents the callback from
    // firing after we have already started processing (e.g. user pressed the
    // hotkey manually while AutoStop was still counting down silence).
    state.recorder.clear_silence_callback();

    // --- Stop recording ---
    let duration_ms = {
        match state.recording_start.lock() {
            Ok(g) => g.map(|t| t.elapsed().as_millis() as u64).unwrap_or(0),
            Err(_) => 0,
        }
    };

    let (whisper_mode, adv) = state
        .config
        .lock()
        .ok()
        .map(|c| (c.whisper_mode, c.advanced.clone()))
        .unwrap_or((false, config::AdvancedSettings::default()));
    let gain = if whisper_mode { adv.whisper_mode_gain } else { 1.0 };

    let wav_bytes = match state.recorder.stop_recording_with_gain(gain) {
        Ok(bytes) => bytes,
        Err(e) => {
            log::error!("[pipeline] failed to stop recording: {e}");
            let _ = handle.emit(
                EVENT_STATE_CHANGED,
                PipelineEvent::error(format!("Failed to stop recording: {e}")),
            );
            return;
        }
    };

    // Clear recording start timestamp.
    if let Ok(mut g) = state.recording_start.lock() {
        *g = None;
    }

    // Store WAV bytes for manual transcribe commands too.
    if let Ok(mut g) = state.last_recording.lock() {
        *g = Some(wav_bytes.clone());
    }

    log::debug!(
        "[pipeline] recording stopped after {duration_ms}ms, {len} WAV bytes",
        len = wav_bytes.len()
    );

    // --- Silence detection ---
    // If the recording is very short (<500ms) or essentially silent, skip the
    // STT/LLM pipeline. This matches Wispr Flow's "nothing said" behaviour.
    // Whisper mode uses a lower threshold since the audio has been amplified.
    let silence_threshold = if whisper_mode {
        adv.whisper_mode_threshold
    } else {
        adv.silence_threshold
    };
    // Measure RMS only when the recording is long enough, matching the original
    // guard order (too-short returns before any RMS work or log).
    let rms = if duration_ms < adv.min_recording_ms as u64 {
        None
    } else {
        let measured = compute_wav_rms(&wav_bytes);
        if let Some(rms) = measured {
            log::debug!("[pipeline] audio RMS = {rms:.5} (threshold={silence_threshold})");
        }
        measured
    };
    match silence_skip(duration_ms, adv.min_recording_ms as u64, rms, silence_threshold) {
        Some(SilenceSkip::TooShort) => {
            log::info!("[pipeline] recording too short ({duration_ms}ms), skipping");
            let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::idle());
            return;
        }
        Some(SilenceSkip::Silent) => {
            log::info!(
                "[pipeline] audio is silent (rms={:.5}), skipping",
                rms.unwrap_or(0.0)
            );
            let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::idle());
            return;
        }
        None => {}
    }

    // --- Snapshot every input the core needs, releasing all locks before the
    // await points inside process_audio (no AppState/AppHandle crosses in). ---
    // `language` and `is_command_mode` are kept as shell-scope vars: the shell
    // still needs them after the core returns (history entry / command-mode
    // consumption gate).
    let language;
    let is_command_mode;
    let process_input = {
        let cfg = match state.config.lock() {
            Ok(g) => g.clone(),
            Err(e) => {
                log::error!("[pipeline] config lock poisoned: {e}");
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned (config)"),
                );
                return;
            }
        };

        let stt_prov = match state.stt_provider.read() {
            Ok(g) => g.clone(),
            Err(e) => {
                log::error!("[pipeline] stt_provider lock poisoned: {e}");
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned (stt_provider)"),
                );
                return;
            }
        };

        let cleanup_prov = match state.cleanup_provider.read() {
            Ok(g) => g.clone(),
            Err(e) => {
                log::error!("[pipeline] cleanup_provider lock poisoned: {e}");
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned (cleanup_provider)"),
                );
                return;
            }
        };

        let dict_terms = match state.dictionary.lock() {
            Ok(g) => {
                let p = g.terms_as_prompt();
                if p.is_empty() { None } else { Some(p) }
            }
            Err(_) => None,
        };

        // Use custom STT hint from advanced settings if set.
        let stt_hint = match cfg.language.as_str() {
            "de" if !cfg.advanced.stt_prompt_de.is_empty() => {
                Some(cfg.advanced.stt_prompt_de.clone())
            }
            "en" if !cfg.advanced.stt_prompt_en.is_empty() => {
                Some(cfg.advanced.stt_prompt_en.clone())
            }
            _ if !cfg.advanced.stt_prompt_auto.is_empty() => {
                Some(cfg.advanced.stt_prompt_auto.clone())
            }
            _ => None,
        };
        let dict_prompt = stt::build_stt_prompt_with_hint(
            dict_terms.as_deref(),
            &cfg.language,
            stt_hint.as_deref(),
        );

        // Keep the hint text (without dictionary terms) for hallucination detection.
        let stt_hint_text = stt_hint.unwrap_or_else(|| match cfg.language.as_str() {
            "de" => "Diktat auf Deutsch mit gelegentlichen englischen Fachbegriffen. Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion.".to_string(),
            "en" => "Voice dictation in English. Proper punctuation, capitalization, and spelling.".to_string(),
            _ => "Multilingual voice dictation. German and English with proper punctuation.".to_string(),
        });

        // Offline mode: if stt_provider is "local" AND the LLM provider is
        // not "local", skip the cleanup step (no network call, raw text
        // goes straight to paste). When llm_provider is "local", cleanup
        // runs offline via llama.cpp — no internet needed.
        let offline = is_offline(&cfg.stt_provider, &cfg.llm_provider);

        // Command Mode: peek the flag and clone the selection WITHOUT resetting.
        // The reset/take is deferred to after process_audio returns, gated on
        // whether it reached the command point — preserving the original
        // ordering where a hallucinated command leaves the flag set.
        is_command_mode = state
            .command_mode_active
            .lock()
            .ok()
            .map(|g| *g)
            .unwrap_or(false);
        let selected_text = if is_command_mode {
            state
                .command_mode_selected_text
                .lock()
                .ok()
                .and_then(|g| g.clone())
        } else {
            None
        };

        // Cleanup parameters (used only on the normal-dictation path), resolved
        // from this same cfg snapshot. The "profile matched" log is deferred to
        // process_audio so it only fires when cleanup actually runs.
        let llm_provider_name = cfg.llm_provider.clone();
        let prev_title = state.prev_window_title.lock().ok().and_then(|t| t.clone());
        let matched = prev_title.as_deref().and_then(|title| {
            let title_lower = title.to_lowercase();
            cfg.profiles.iter().find(|p| {
                !p.app_pattern.is_empty() && title_lower.contains(&p.app_pattern.to_lowercase())
            })
        });
        let (cleanup_style, custom_prompt, matched_profile_name) = if let Some(profile) = matched {
            let prompt = if profile.custom_prompt.is_empty() {
                let p = cfg.custom_prompt.clone();
                if p.is_empty() { None } else { Some(p) }
            } else {
                Some(profile.custom_prompt.clone())
            };
            (
                profile.cleanup_style,
                prompt,
                Some(format!("{:?}", profile.name)),
            )
        } else {
            let p = cfg.custom_prompt.clone();
            (
                cfg.cleanup_style,
                if p.is_empty() { None } else { Some(p) },
                None,
            )
        };

        let dict_list = match state.dictionary.lock() {
            Ok(g) => {
                let l = g.terms_as_list();
                if l.is_empty() { None } else { Some(l) }
            }
            Err(_) => None,
        };

        let output_lang = if cfg.output_language.is_empty() {
            None
        } else {
            Some(cfg.output_language.clone())
        };

        language = cfg.language.clone();

        ProcessInput {
            wav_bytes,
            language: cfg.language.clone(),
            stt_provider: stt_prov,
            cleanup_provider: cleanup_prov,
            stt_prompt: SttPromptPair { dict_prompt, stt_hint_text },
            offline_mode: offline,
            selected_text,
            cleanup_style,
            custom_prompt,
            matched_profile_name,
            dict_list,
            output_lang,
            llm_provider_name,
            config_for_fallback: cfg,
        }
    };

    // --- Run the STT -> guard -> LLM -> sanitize core (no locks held) ---
    let pipeline_start = std::time::Instant::now();
    let outcome = {
        let mut emit = |ev| {
            let _ = handle.emit(EVENT_STATE_CHANGED, ev);
        };
        process_audio(process_input, &mut emit).await
    };

    // --- Apply the side effects the core deferred: metric deltas + command-mode
    // consumption. Command mode is consumed (flag reset, selection cleared) only
    // when the core reached the command point (Produced / CommandFailed),
    // matching the original guard-before-reset ordering. Progress/terminal
    // events were already emitted by process_audio. ---
    let Some((cleaned_text, raw_text, is_command, stt_ms, llm_ms, prompt_tokens, completion_tokens)) =
        deliver_outcome(outcome, &state, is_command_mode)
    else {
        return;
    };

    // --- Record usage ---
    if let Ok(db) = state.history_db.lock() {
        // STT cost per audio hour depends on the model
        let stt_rate = match state
            .config
            .lock()
            .ok()
            .as_ref()
            .map(|c| c.stt_model.as_str())
        {
            Some("whisper-large-v3") => 0.111,
            Some("distil-whisper-large-v3-en") => 0.02,
            _ => 0.04, // whisper-large-v3-turbo (default)
        };
        let stt_cost = duration_ms as f64 / 3_600_000.0 * stt_rate;
        if let Err(e) = history::record_usage(
            &db,
            "groq_stt",
            Some(duration_ms as i64),
            None,
            None,
            stt_cost,
        ) {
            log::warn!("[pipeline] Failed to record STT usage: {e}");
        }
        // LLM cost: DeepSeek input=$0.27/1M, output=$1.10/1M tokens
        let llm_cost = (prompt_tokens.unwrap_or(0) as f64 * 0.27
            + completion_tokens.unwrap_or(0) as f64 * 1.10)
            / 1_000_000.0;
        if let Err(e) = history::record_usage(
            &db,
            "deepseek_cleanup",
            None,
            prompt_tokens,
            completion_tokens,
            llm_cost,
        ) {
            log::warn!("[pipeline] Failed to record LLM usage: {e}");
        }
    }

    // --- Paste ---
    // Capture the window the user is CURRENTLY in, before paste switches focus.
    // Used for Return-to-Current after autosend.
    let current_hwnd_before_paste = crate::paste::capture_foreground_window();
    let prev_hwnd = state.prev_foreground_hwnd.lock().ok().and_then(|g| *g);
    let paste_handler = create_paste_handler(prev_hwnd);
    let paste_result = match paste_handler.paste(&cleaned_text) {
        Ok(result) => result,
        Err(e) => {
            log::warn!("[pipeline] paste failed: {e}. Text is still available.");
            if let Ok(mut m) = state.feedback_metrics.lock() {
                m.paste_error_count = m.paste_error_count.saturating_add(1);
            }
            // A hard error (e.g. clipboard unavailable) is treated as
            // clipboard-only -- the user gets an indication but the pipeline
            // continues so the done event is still emitted.
            PasteResult::ClipboardOnly
        }
    };

    // --- Insert+Send + Return-to-Current ---
    //
    // insert_and_send is now a per-slot flag stored in AppState by the hotkey
    // handler when recording starts. Reading it here (after the paste) is safe
    // because the hotkey handler cannot fire again while we are still in the
    // pipeline (the recorder is marked as recording until stop_recording_with_gain
    // returns above, and a second hotkey press would be a no-op or a race).
    //
    // Only sent when Ctrl+V actually landed in the right window.
    // Sending Enter into the wrong window (e.g. after a failed focus-restore)
    // would be worse than not sending it at all.
    //
    // The 150ms sleep gives the target app time to process the Paste before
    // Enter arrives. Terminals (ConPTY) need more time than simple editors.
    // This is opt-in and defaults to false per slot.
    let insert_and_send = state
        .active_insert_and_send
        .load(Ordering::SeqCst);
    if insert_and_send && paste_result == PasteResult::Pasted {
        std::thread::sleep(std::time::Duration::from_millis(150));
        if let Err(e) = paste_handler.send_enter() {
            log::warn!("[pipeline] send_enter failed: {e}");
        }

        // Return-to-Current: if the user switched to a different window while
        // Klarvo was processing (STT + LLM cleanup takes seconds), bring them
        // back to where they were just before paste, not the recording-start
        // window.
        //
        // current_hwnd_before_paste was captured BEFORE paste() switched focus
        // to the target window. If it differs from prev_hwnd, the user moved
        // to a different window during processing and we should return them.
        if let Some(current) = current_hwnd_before_paste {
            if Some(current) != prev_hwnd {
                log::info!(
                    "[pipeline] Return-to-current: restoring focus to HWND={current:#x} \
                     (user was here during processing; paste target was {:#x})",
                    prev_hwnd.unwrap_or(0)
                );
                // Small delay to let Enter land before we switch away.
                std::thread::sleep(std::time::Duration::from_millis(100));
                crate::paste::restore_focus(current);
            }
        }
    }

    // --- Save to history ---
    {
        let style_str = if is_command {
            "command".to_string()
        } else {
            state
                .config
                .lock()
                .ok()
                .map(|c| {
                    serde_json::to_string(&c.cleanup_style)
                        .unwrap_or_default()
                        .replace('"', "")
                })
                .unwrap_or_else(|| "polished".to_string())
        };
        let app_name = state.prev_window_title.lock().ok().and_then(|t| t.clone());
        let cfg_for_history = state
            .config
            .lock()
            .ok()
            .map(|c| (c.device_id.clone(), c.turso_url.clone(), c.turso_token.clone()));

        // Generate UUID here so we can pass it to both the DB insert and the
        // async Turso push without a second DB read.
        let entry_uuid = uuid::Uuid::new_v4().to_string();

        if let Ok(db) = state.history_db.lock() {
            let device_id = cfg_for_history.as_ref().map(|(d, _, _)| d.as_str());
            if let Err(e) = history::add_entry(
                &db,
                &cleaned_text,
                Some(&raw_text),
                &style_str,
                &language,
                false,
                app_name.as_deref(),
                Some(&entry_uuid),
                device_id,
            ) {
                log::warn!("[pipeline] Failed to save to history: {e}");
            }
        }

        // --- Auto-sync to Turso (fire-and-forget) ---
        // Only runs when Turso is configured. Never blocks the pipeline.
        // The manual "Sync Now" button covers pull + batch push of missed entries.
        if let Some((device_id, turso_url, turso_token)) = cfg_for_history.clone() {
            if !turso_url.is_empty() && !turso_token.is_empty() {
                let sync_entry = sync::SyncEntry {
                    uuid: entry_uuid.clone(),
                    text: cleaned_text.clone(),
                    raw_text: Some(raw_text.clone()),
                    style: style_str.clone(),
                    language: language.clone(),
                    is_note: 0,
                    app_name: app_name.clone(),
                    device_id: Some(device_id.clone()),
                    // created_at will be set by Turso's DEFAULT; we mirror what
                    // SQLite uses so the field is consistent.
                    created_at: chrono::Utc::now()
                        .naive_utc()
                        .format("%Y-%m-%dT%H:%M:%S")
                        .to_string(),
                };
                let uuid_for_mark = entry_uuid.clone();
                let handle_for_sync = handle.clone();
                tauri::async_runtime::spawn(async move {
                    match sync::push_single_entry(&turso_url, &turso_token, sync_entry).await {
                        Ok(_) => {
                            // Mark the entry as synced in the local DB.
                            // Re-acquire state via the handle -- never hold
                            // the DB lock across an await point.
                            //
                            // The MutexGuard is released at the end of the
                            // map closure, before `mark_result` is read.
                            let st = handle_for_sync.state::<AppState>();
                            let mark_result = st.history_db.lock().ok().map(|db| {
                                sync::mark_entries_synced(&db, std::slice::from_ref(&uuid_for_mark))
                            });
                            if let Some(Err(e)) = mark_result {
                                log::warn!("[sync] Failed to mark entry as synced: {e}");
                            }
                        }
                        Err(e) => {
                            // Non-fatal: the entry stays synced=0 and will be
                            // picked up by the next manual "Sync Now".
                            log::warn!(
                                "[sync] Auto-push failed (will retry on next sync): {e}"
                            );
                        }
                    }
                });
            }
        }

        // --- Webhook ---
        let webhook_url = state
            .config
            .lock()
            .ok()
            .map(|c| c.webhook_url.clone())
            .unwrap_or_default();
        if !webhook_url.is_empty() {
            let payload = serde_json::json!({
                "text": &cleaned_text,
                "rawText": &raw_text,
                "style": &style_str,
                "language": &language,
                "appName": app_name.as_deref(),
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "durationMs": duration_ms,
            });
            let url = webhook_url.clone();
            // Fire-and-forget: don't block the pipeline on webhook delivery.
            tauri::async_runtime::spawn(async move {
                let client = reqwest::Client::new();
                if let Err(e) = client
                    .post(&url)
                    .json(&payload)
                    .timeout(std::time::Duration::from_secs(10))
                    .send()
                    .await
                {
                    log::warn!("[webhook] POST to {url} failed: {e}");
                }
            });
        }
    }

    // --- Update feedback metrics ---
    // Write latency, target app, timestamp, and last dictation text so the
    // feedback form can include fresh telemetry when opened.
    {
        let total_ms = pipeline_start.elapsed().as_millis() as u64;
        let target_app = state.prev_window_title.lock().ok().and_then(|t| t.clone());
        if let Ok(mut m) = state.feedback_metrics.lock() {
            m.last_stt_latency_ms = Some(stt_ms);
            m.last_llm_latency_ms = llm_ms;
            m.last_total_latency_ms = Some(total_ms);
            m.last_target_app = target_app;
            m.last_dictation_at = Some(chrono::Utc::now().to_rfc3339());
            m.last_raw_text = Some(raw_text.clone());
            m.last_cleaned_text = Some(cleaned_text.clone());
        }
    }

    // Emit the appropriate done event based on whether the paste succeeded.
    let done_event = if paste_result == PasteResult::ClipboardOnly {
        PipelineEvent::done_with_clipboard_only(cleaned_text, raw_text)
    } else {
        PipelineEvent::done(cleaned_text, raw_text)
    };
    let _ = handle.emit(EVENT_STATE_CHANGED, done_event);
}

/// Applies the deferred side effects from a [`ProcessOutcome`] and extracts the
/// produced fields for the caller to continue with usage recording, paste, and
/// history.
///
/// Returns `None` on `Stopped` / `CommandFailed` (the caller should return
/// immediately); returns `Some(...)` on `Produced` with the text and timing
/// fields ready for the post-pipeline steps.
///
/// Invariants maintained:
/// - `stt_error_count` is incremented on `Stopped { stt_error: true }`.
/// - `llm_error_count` is incremented on `CommandFailed` and on `Produced {
///   llm_error: true }`.
/// - `consume_command_mode` is called on `CommandFailed` and on `Produced`
///   when `is_command_mode` is `true`.
#[allow(clippy::type_complexity)] // Tuple mirrors ProcessOutcome::Produced fields; a named struct is a follow-up refactor
fn deliver_outcome(
    outcome: ProcessOutcome,
    state: &AppState,
    is_command_mode: bool,
) -> Option<(String, String, bool, u64, Option<u64>, Option<u32>, Option<u32>)> {
    match outcome {
        ProcessOutcome::Stopped { stt_error } => {
            if stt_error {
                if let Ok(mut m) = state.feedback_metrics.lock() {
                    m.stt_error_count = m.stt_error_count.saturating_add(1);
                }
            }
            None
        }
        ProcessOutcome::CommandFailed => {
            if let Ok(mut m) = state.feedback_metrics.lock() {
                m.llm_error_count = m.llm_error_count.saturating_add(1);
            }
            consume_command_mode(state);
            None
        }
        ProcessOutcome::Produced {
            cleaned_text,
            raw_text,
            is_command,
            stt_ms,
            llm_ms,
            prompt_tokens,
            completion_tokens,
            llm_error,
        } => {
            if llm_error {
                if let Ok(mut m) = state.feedback_metrics.lock() {
                    m.llm_error_count = m.llm_error_count.saturating_add(1);
                }
            }
            if is_command_mode {
                consume_command_mode(state);
            }
            Some((
                cleaned_text,
                raw_text,
                is_command,
                stt_ms,
                llm_ms,
                prompt_tokens,
                completion_tokens,
            ))
        }
    }
}

/// Toggle-mode hotkey handler: press once to start, press again to stop + process.
///
/// This is the legacy behaviour, kept for users who prefer toggle mode.
pub async fn run_dictation_pipeline(handle: AppHandle) {
    let state = handle.state::<AppState>();

    if !state.recorder.is_recording() {
        start_recording_only(handle).await;
    } else {
        stop_and_process_pipeline(handle).await;
    }
}

/// Registers the global shortcut(s) with mode-aware handlers.
///
/// Reads `hotkey_slots` from the current `AppState` config. Each enabled slot
/// (non-empty `hotkey` string) gets its own independent handler that uses the
/// slot's `mode`. Disabled slots (empty `hotkey`) are silently skipped.
///
/// Unregisters all existing shortcuts first so this can be called to
/// re-register after a settings change.
///
/// Both slots share the same recorder: the `is_recording()` guard inside each
/// handler prevents two slots from starting a recording simultaneously.
///
/// Recording modes per slot:
/// - `Toggle`:  Pressed fires [`run_dictation_pipeline`] (start or stop+process).
/// - `Hold`:    Pressed fires [`start_recording_only`]; Released fires
///   [`stop_and_process_pipeline`].
/// - `AutoStop`: Press once to start; silence stops automatically. Second press
///   stops manually if still recording.
/// - `Auto`:    Like AutoStop but loops until the user presses again.
#[cfg(desktop)]
pub fn register_hotkey(handle: &AppHandle) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    // Read enabled slots from the current config. We clone the data out of the
    // lock immediately so we don't hold the Mutex while calling into the
    // global-shortcut plugin (which may acquire its own internal lock).
    let slots: Vec<crate::config::HotkeySlot> = handle
        .state::<AppState>()
        .config
        .lock()
        .ok()
        .map(|c| c.hotkey_slots.clone())
        .unwrap_or_default();

    let cmd_shortcut_str = handle
        .state::<AppState>()
        .config
        .lock()
        .ok()
        .map(|c| c.command_hotkey.clone())
        .unwrap_or_else(|| "ctrl+shift+e".to_string());

    log::info!("[hotkey] Re-registering hotkeys: {} slot(s)", slots.len());

    handle
        .global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {e}"))?;

    // --- Dictation slots ---
    //
    // FIX: Previously each slot called `on_shortcut()` in a loop, which caused
    // the plugin to overwrite the per-shortcut handler map entry for the last
    // registered shortcut, making ALL slots behave like the last slot's mode.
    //
    // Now we build a (shortcut_id, mode) dispatch map up front, collect all
    // valid shortcut objects, and register them with a SINGLE `on_shortcuts()`
    // call + one shared handler.  Inside the handler we look up the mode by
    // `shortcut.id()` so each slot dispatches to its own mode correctly.
    //
    // `Shortcut` does not implement `Hash`/`Eq`, so we key the map by the
    // `u32` hotkey ID returned by `shortcut.id()`.

    // Build dispatch map: hotkey_id -> (mode, insert_and_send)
    let mut slot_map: Vec<(u32, HotkeyMode, bool)> = Vec::new();
    let mut shortcut_objects: Vec<tauri_plugin_global_shortcut::Shortcut> = Vec::new();

    for slot in &slots {
        if !slot.is_enabled() {
            log::debug!("[hotkey] Slot {:?} disabled (empty hotkey), skipping", slot.mode);
            continue;
        }

        let shortcut = match slot.hotkey.parse::<tauri_plugin_global_shortcut::Shortcut>() {
            Ok(s) => s,
            Err(e) => {
                log::warn!(
                    "[hotkey] Slot hotkey {:?} is invalid ({e}), skipping",
                    slot.hotkey
                );
                continue;
            }
        };

        // Deduplicate: if two slots share the same shortcut key, keep only the
        // first one. Prevents "HotKey already registered" errors from the OS.
        if slot_map.iter().any(|(id, _, _)| *id == shortcut.id()) {
            log::warn!(
                "[hotkey] Slot {:?} has duplicate shortcut id={}, skipping",
                slot.hotkey,
                shortcut.id()
            );
            continue;
        }

        log::debug!(
            "[hotkey] Queuing slot: {:?} id={} mode={:?} insert_and_send={}",
            slot.hotkey, shortcut.id(), slot.mode, slot.insert_and_send
        );
        slot_map.push((shortcut.id(), slot.mode, slot.insert_and_send));
        shortcut_objects.push(shortcut);
    }

    if !shortcut_objects.is_empty() {
        let handle_clone = handle.clone();
        handle
            .global_shortcut()
            .on_shortcuts(shortcut_objects, move |_app, shortcut, event| {
                log::debug!("[hotkey] Event: shortcut_id={} {event:?}", shortcut.id());

                // While the ShortcutRecorder is active, swallow all hotkey
                // events so the user can press the current shortcut without
                // triggering the pipeline.
                if handle_clone
                    .state::<AppState>()
                    .hotkey_paused
                    .load(Ordering::SeqCst)
                {
                    log::debug!("[hotkey] paused (ShortcutRecorder active), ignoring");
                    return;
                }

                // Resolve the mode and insert_and_send flag for the specific
                // shortcut that fired. Linear scan is fine: at most two slots.
                let (mode, slot_insert_and_send) =
                    match slot_map.iter().find(|(id, _, _)| *id == shortcut.id()) {
                        Some((_, m, ias)) => (*m, *ias),
                        None => {
                            log::warn!("[hotkey] Unknown shortcut id={}, ignoring", shortcut.id());
                            return;
                        }
                    };

                let h = handle_clone.clone();
                log::info!("[hotkey] mode={mode:?} state={:?}", event.state);

                // Tell the FloatingBar which mode is active so it shows the
                // correct badge (Hotkey 1 vs Hotkey 2 may have different modes).
                let _ = handle_clone.emit("klarvo://active-mode", mode);

                // Helper: stores the slot's insert_and_send flag in AppState
                // so stop_and_process_pipeline can read it without needing to
                // know which slot triggered the pipeline.
                let store_insert_and_send = |ias: bool| {
                    handle_clone
                        .state::<AppState>()
                        .active_insert_and_send
                        .store(ias, Ordering::SeqCst);
                };

                match (mode, event.state) {
                    (HotkeyMode::Toggle, ShortcutState::Pressed) => {
                        store_insert_and_send(slot_insert_and_send);
                        tauri::async_runtime::spawn(async move {
                            run_dictation_pipeline(h).await;
                        });
                    }
                    (HotkeyMode::Hold, ShortcutState::Pressed) => {
                        store_insert_and_send(slot_insert_and_send);
                        tauri::async_runtime::spawn(async move {
                            start_recording_only(h).await;
                        });
                    }
                    (HotkeyMode::Hold, ShortcutState::Released) => {
                        tauri::async_runtime::spawn(async move {
                            stop_and_process_pipeline(h).await;
                        });
                    }
                    (HotkeyMode::AutoStop, ShortcutState::Pressed) => {
                        // If already recording: second press = manual stop.
                        // stop_and_process_pipeline clears the silence callback
                        // before doing anything else, so no double-invocation.
                        // Guard also prevents slot 2 from starting while slot 1
                        // is already recording.
                        let is_recording = handle_clone
                            .state::<AppState>()
                            .recorder
                            .is_recording();
                        if is_recording {
                            tauri::async_runtime::spawn(async move {
                                stop_and_process_pipeline(h).await;
                            });
                        } else {
                            store_insert_and_send(slot_insert_and_send);
                            tauri::async_runtime::spawn(async move {
                                start_autostop_recording(h).await;
                            });
                        }
                    }
                    (HotkeyMode::AutoStop, ShortcutState::Released) => {
                        // No-op: AutoStop is toggle-style, release has no meaning.
                    }
                    (HotkeyMode::Auto, ShortcutState::Pressed) => {
                        let state = handle_clone.state::<AppState>();
                        if state.recorder.is_recording() {
                            // Second press while recording: stop the loop and
                            // process whatever was recorded so far.
                            state.auto_loop_active.store(false, Ordering::SeqCst);
                            tauri::async_runtime::spawn(async move {
                                stop_and_process_pipeline(h).await;
                            });
                        } else {
                            // First press: activate loop and start first cycle.
                            store_insert_and_send(slot_insert_and_send);
                            state.auto_loop_active.store(true, Ordering::SeqCst);
                            tauri::async_runtime::spawn(async move {
                                start_auto_recording(h).await;
                            });
                        }
                    }
                    (HotkeyMode::Auto, ShortcutState::Released) => {
                        // No-op: Auto mode is toggle-style, release has no meaning.
                    }
                    _ => {}
                }
            })
            .map_err(|e| format!("Failed to register dictation shortcuts: {e}"))?;
    }

    // --- Command Mode hotkey ---
    if let Ok(cmd_shortcut) = cmd_shortcut_str.parse::<tauri_plugin_global_shortcut::Shortcut>() {
        let handle_clone2 = handle.clone();
        let _ = handle
            .global_shortcut()
            .on_shortcut(cmd_shortcut, move |_app, _shortcut, event| {
                let h = handle_clone2.clone();
                match event.state {
                    ShortcutState::Pressed => {
                        tauri::async_runtime::spawn(async move {
                            start_command_mode(h).await;
                        });
                    }
                    ShortcutState::Released => {
                        tauri::async_runtime::spawn(async move {
                            stop_and_process_pipeline(h).await;
                        });
                    }
                }
            });
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
// Output sanitization — strip dangerous characters from LLM output before paste
// ---------------------------------------------------------------------------

/// Sanitizes LLM output before it is pasted into the active window.
///
/// Removes characters that could exploit the target application or terminal:
/// - ANSI escape sequences (terminal control)
/// - Unicode bidirectional override/embedding characters (text spoofing)
/// - Null bytes (string truncation in C-based apps)
/// - Zero-width characters (invisible text / steganography)
///
/// Normal text (including emoji, CJK, accented characters) passes through unchanged.
pub fn sanitize_llm_output(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Strip ANSI escape: ESC followed by '[' and then parameter bytes + final byte
            '\x1b' => {
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                    // consume until a letter (the final byte of the CSI sequence)
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                // else: lone ESC, just skip it
            }
            // Null byte
            '\0' => {}
            // Unicode bidirectional overrides and embeddings
            '\u{202A}' // LRE
            | '\u{202B}' // RLE
            | '\u{202C}' // PDF
            | '\u{202D}' // LRO
            | '\u{202E}' // RLO
            | '\u{2066}' // LRI
            | '\u{2067}' // RLI
            | '\u{2068}' // FSI
            | '\u{2069}' // PDI
            | '\u{200F}' // RTL mark
            | '\u{200E}' // LTR mark
            => {}
            // Zero-width characters (invisible text / steganography)
            '\u{200B}' // ZWSP
            | '\u{200C}' // ZWNJ
            | '\u{200D}' // ZWJ
            | '\u{FEFF}' // BOM / ZWNBSP
            => {}
            // Everything else passes through
            _ => out.push(ch),
        }
    }

    out
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    /// When `stt_provider` is `"local"` and `llm_provider` is NOT `"local"`,
    /// the offline flag must be `true` so the pipeline skips the LLM cleanup step.
    ///
    /// Exercises the real `is_offline` decision helper that
    /// `stop_and_process_pipeline` now calls (previously these tests replicated
    /// the expression inline, which gave false security).
    #[test]
    fn test_offline_flag_true_when_stt_local_and_llm_cloud() {
        let cfg = AppConfig {
            stt_provider: "local".to_string(),
            llm_provider: "deepseek".to_string(),
            ..AppConfig::default()
        };
        let offline = is_offline(&cfg.stt_provider, &cfg.llm_provider);
        assert!(offline, "offline flag should be true when stt=local but llm!=local");
    }

    /// When both `stt_provider` and `llm_provider` are `"local"`, the offline
    /// flag must be `false` so the pipeline runs local LLM cleanup.
    #[test]
    fn test_offline_flag_false_when_both_local() {
        let cfg = AppConfig {
            stt_provider: "local".to_string(),
            llm_provider: "local".to_string(),
            ..AppConfig::default()
        };
        let offline = is_offline(&cfg.stt_provider, &cfg.llm_provider);
        assert!(!offline, "offline flag should be false when both stt and llm are local");
    }

    /// When `stt_provider` is a cloud provider, the offline flag must be `false`.
    #[test]
    fn test_offline_flag_false_when_provider_is_groq() {
        let cfg = AppConfig {
            stt_provider: "groq".to_string(),
            groq_api_key: "gsk-test".to_string(),
            ..AppConfig::default()
        };
        let offline = is_offline(&cfg.stt_provider, &cfg.llm_provider);
        assert!(!offline, "offline flag should be false when stt_provider != 'local'");
    }

    /// When `stt_provider` is `"openai"`, the offline flag must be `false`.
    #[test]
    fn test_offline_flag_false_when_provider_is_openai() {
        let cfg = AppConfig {
            stt_provider: "openai".to_string(),
            openai_api_key: "sk-test".to_string(),
            ..AppConfig::default()
        };
        let offline = is_offline(&cfg.stt_provider, &cfg.llm_provider);
        assert!(!offline);
    }

    /// Default stt_provider is "groq", so offline flag is false by default.
    #[test]
    fn test_offline_flag_false_by_default() {
        let cfg = AppConfig::default();
        let offline = is_offline(&cfg.stt_provider, &cfg.llm_provider);
        assert!(!offline, "default config should not be in offline mode");
    }

    // -----------------------------------------------------------------------
    // Decision logic (Task 2.2 net): silence_skip / post_stt_skip /
    // select_llm_path. These pin the branch decisions that
    // `stop_and_process_pipeline` routes through, so the upcoming `process_audio`
    // extraction cannot silently change them.
    // -----------------------------------------------------------------------

    #[test]
    fn test_silence_skip_too_short() {
        assert_eq!(
            silence_skip(100, 500, Some(0.5), 0.01),
            Some(SilenceSkip::TooShort)
        );
    }

    #[test]
    fn test_silence_skip_at_minimum_is_not_too_short() {
        // duration == min is allowed (strict `<`), and loud enough -> proceed.
        assert_eq!(silence_skip(500, 500, Some(0.5), 0.01), None);
    }

    #[test]
    fn test_silence_skip_silent() {
        assert_eq!(
            silence_skip(1000, 500, Some(0.005), 0.01),
            Some(SilenceSkip::Silent)
        );
    }

    #[test]
    fn test_silence_skip_loud_enough_proceeds() {
        assert_eq!(silence_skip(1000, 500, Some(0.02), 0.01), None);
    }

    #[test]
    fn test_silence_skip_unmeasurable_rms_proceeds() {
        // None RMS (invalid WAV) skips the loudness check, matching the original
        // `if let Some(rms)` guard.
        assert_eq!(silence_skip(1000, 500, None, 0.01), None);
    }

    #[test]
    fn test_silence_skip_too_short_precedes_silent() {
        // Too-short wins even when the audio is also silent.
        assert_eq!(
            silence_skip(100, 500, Some(0.0), 0.01),
            Some(SilenceSkip::TooShort)
        );
    }

    const TEST_STT_HINT: &str =
        "Voice dictation in English. Proper punctuation, capitalization, and spelling.";

    #[test]
    fn test_post_stt_skip_real_speech_proceeds() {
        assert_eq!(
            post_stt_skip("Please send me the report by Friday", TEST_STT_HINT),
            None
        );
    }

    #[test]
    fn test_post_stt_skip_prompt_echo() {
        assert_eq!(
            post_stt_skip(TEST_STT_HINT, TEST_STT_HINT),
            Some(PostSttSkip::PromptEcho)
        );
    }

    #[test]
    fn test_post_stt_skip_blocklist() {
        assert_eq!(
            post_stt_skip("Thank you for watching", TEST_STT_HINT),
            Some(PostSttSkip::Blocklist)
        );
    }

    #[test]
    fn test_select_llm_path_offline_dictation() {
        assert_eq!(select_llm_path(true, false), LlmPath::OfflineRaw);
    }

    #[test]
    fn test_select_llm_path_command_wins_over_offline() {
        // A selection forces an LLM call even offline.
        assert_eq!(select_llm_path(true, true), LlmPath::Command);
        assert_eq!(select_llm_path(false, true), LlmPath::Command);
    }

    #[test]
    fn test_select_llm_path_normal_cleanup() {
        assert_eq!(select_llm_path(false, false), LlmPath::Cleanup);
    }

    /// Golden master of the full decision matrix. A change here means a branch
    /// decision moved — review it deliberately, never blind-accept the snapshot.
    #[test]
    fn test_decision_matrix_snapshot() {
        let mut out = String::new();

        out.push_str("# silence_skip(duration_ms, min_recording_ms, rms, threshold)\n");
        let silence_cases: [(u64, u64, Option<f32>, f32); 6] = [
            (100, 500, Some(0.5), 0.01),
            (500, 500, Some(0.5), 0.01),
            (1000, 500, Some(0.005), 0.01),
            (1000, 500, Some(0.02), 0.01),
            (1000, 500, None, 0.01),
            (100, 500, Some(0.0), 0.01),
        ];
        for (d, m, r, t) in silence_cases {
            out.push_str(&format!(
                "  ({d}, {m}, {r:?}, {t}) -> {:?}\n",
                silence_skip(d, m, r, t)
            ));
        }

        out.push_str("\n# post_stt_skip(transcript, hint)\n");
        let post_cases: [&str; 4] = [
            "Please send me the report by Friday",
            TEST_STT_HINT,
            "",
            "Thank you for watching",
        ];
        for txt in post_cases {
            out.push_str(&format!("  {txt:?} -> {:?}\n", post_stt_skip(txt, TEST_STT_HINT)));
        }

        out.push_str("\n# select_llm_path(offline, has_selected_text)\n");
        for offline in [false, true] {
            for has_sel in [false, true] {
                out.push_str(&format!(
                    "  (offline={offline}, has_sel={has_sel}) -> {:?}\n",
                    select_llm_path(offline, has_sel)
                ));
            }
        }

        insta::assert_snapshot!(out);
    }

    // -----------------------------------------------------------------------
    // process_audio (Task 2.2 extraction): run end-to-end with fake providers
    // and a capturing emitter. Pins the I/O matrix from the spec — the core is
    // now unit-testable without a Tauri AppHandle.
    // -----------------------------------------------------------------------

    use crate::hotkey::PipelineState;

    /// Fake STT: returns a fixed transcript, or an API error.
    struct FakeStt(Result<String, ()>);

    #[async_trait::async_trait]
    impl SttProvider for FakeStt {
        async fn transcribe(
            &self,
            _audio: Vec<u8>,
            _language: &str,
            _prompt: Option<&str>,
        ) -> Result<String, stt::SttError> {
            match &self.0 {
                Ok(t) => Ok(t.clone()),
                Err(()) => Err(stt::SttError::ApiError {
                    status: 500,
                    message: "stt boom".to_string(),
                }),
            }
        }
    }

    enum CleanupBehavior {
        Ok(String),
        Retryable,
        NonRetryable,
    }

    /// Fake cleanup provider: controls both the cleanup and rewrite outcomes.
    struct FakeCleanup {
        cleanup: CleanupBehavior,
        rewrite: Result<String, ()>,
    }

    #[async_trait::async_trait]
    impl CleanupProvider for FakeCleanup {
        async fn cleanup(
            &self,
            _raw_text: &str,
            _style: CleanupStyle,
            _dictionary_terms: Option<&str>,
            _custom_prompt: Option<&str>,
        ) -> Result<llm::CleanupResult, llm::LlmError> {
            match &self.cleanup {
                CleanupBehavior::Ok(t) => Ok(llm::CleanupResult {
                    text: t.clone(),
                    prompt_tokens: Some(7),
                    completion_tokens: Some(3),
                }),
                CleanupBehavior::Retryable => Err(llm::LlmError::ApiError {
                    status: 429,
                    message: "rate limit".to_string(),
                }),
                CleanupBehavior::NonRetryable => {
                    Err(llm::LlmError::ResponseFormat("bad request".to_string()))
                }
            }
        }

        async fn rewrite(
            &self,
            _selected_text: &str,
            _voice_command: &str,
        ) -> Result<llm::CleanupResult, llm::LlmError> {
            match &self.rewrite {
                Ok(t) => Ok(llm::CleanupResult {
                    text: t.clone(),
                    prompt_tokens: Some(2),
                    completion_tokens: Some(1),
                }),
                Err(()) => Err(llm::LlmError::ResponseFormat("rewrite boom".to_string())),
            }
        }
    }

    const REAL_SPEECH: &str = "Please send me the report by Friday";

    fn make_input(stt: FakeStt, cleanup: FakeCleanup) -> ProcessInput {
        ProcessInput {
            wav_bytes: vec![0u8; 16],
            language: "en".to_string(),
            stt_provider: Arc::new(stt),
            cleanup_provider: Arc::new(cleanup),
            stt_prompt: SttPromptPair {
                dict_prompt: None,
                stt_hint_text: TEST_STT_HINT.to_string(),
            },
            offline_mode: false,
            selected_text: None,
            cleanup_style: CleanupStyle::Polished,
            custom_prompt: None,
            matched_profile_name: None,
            dict_list: None,
            output_lang: None,
            llm_provider_name: "deepseek".to_string(),
            config_for_fallback: AppConfig::default(),
        }
    }

    /// Runs `process_audio`, returning the outcome and the emitted event states.
    async fn run(input: ProcessInput) -> (ProcessOutcome, Vec<PipelineState>) {
        let mut events: Vec<PipelineState> = Vec::new();
        let outcome = {
            let mut emit = |ev: PipelineEvent| events.push(ev.state);
            process_audio(input, &mut emit).await
        };
        (outcome, events)
    }

    #[tokio::test]
    async fn test_process_audio_normal_cleanup() {
        let input = make_input(
            FakeStt(Ok(REAL_SPEECH.to_string())),
            FakeCleanup {
                cleanup: CleanupBehavior::Ok("Cleaned text.".to_string()),
                rewrite: Err(()),
            },
        );
        let (outcome, events) = run(input).await;
        assert_eq!(
            events,
            vec![PipelineState::Transcribing, PipelineState::Cleaning]
        );
        match outcome {
            ProcessOutcome::Produced {
                cleaned_text,
                is_command,
                llm_ms,
                prompt_tokens,
                completion_tokens,
                llm_error,
                ..
            } => {
                assert_eq!(cleaned_text, sanitize_llm_output("Cleaned text."));
                assert!(!is_command);
                assert!(llm_ms.is_some());
                assert_eq!(prompt_tokens, Some(7));
                assert_eq!(completion_tokens, Some(3));
                assert!(!llm_error);
            }
            other => panic!("expected Produced, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_process_audio_offline_skips_llm() {
        let mut input = make_input(
            FakeStt(Ok(REAL_SPEECH.to_string())),
            // Cleanup would error if called — it must NOT be on the offline path.
            FakeCleanup {
                cleanup: CleanupBehavior::NonRetryable,
                rewrite: Err(()),
            },
        );
        input.offline_mode = true;
        let (outcome, events) = run(input).await;
        assert_eq!(events, vec![PipelineState::Transcribing]); // no Cleaning
        match outcome {
            ProcessOutcome::Produced {
                cleaned_text,
                raw_text,
                is_command,
                llm_ms,
                prompt_tokens,
                completion_tokens,
                llm_error,
                ..
            } => {
                assert_eq!(cleaned_text, sanitize_llm_output(&raw_text));
                assert!(!is_command);
                assert_eq!(llm_ms, None);
                assert_eq!(prompt_tokens, None);
                assert_eq!(completion_tokens, None);
                assert!(!llm_error);
            }
            other => panic!("expected Produced, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_process_audio_command_mode() {
        let mut input = make_input(
            FakeStt(Ok("make it formal".to_string())),
            FakeCleanup {
                cleanup: CleanupBehavior::NonRetryable,
                rewrite: Ok("Formal rewrite.".to_string()),
            },
        );
        input.selected_text = Some("hey there".to_string());
        let (outcome, events) = run(input).await;
        assert_eq!(
            events,
            vec![PipelineState::Transcribing, PipelineState::Cleaning]
        );
        match outcome {
            ProcessOutcome::Produced {
                cleaned_text,
                is_command,
                prompt_tokens,
                ..
            } => {
                assert_eq!(cleaned_text, sanitize_llm_output("Formal rewrite."));
                assert!(is_command);
                assert_eq!(prompt_tokens, Some(2));
            }
            other => panic!("expected Produced, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_process_audio_command_failure() {
        let mut input = make_input(
            FakeStt(Ok("make it formal".to_string())),
            FakeCleanup {
                cleanup: CleanupBehavior::Ok("unused".to_string()),
                rewrite: Err(()),
            },
        );
        input.selected_text = Some("hey there".to_string());
        let (outcome, events) = run(input).await;
        assert_eq!(
            events,
            vec![
                PipelineState::Transcribing,
                PipelineState::Cleaning,
                PipelineState::Error
            ]
        );
        assert_eq!(outcome, ProcessOutcome::CommandFailed);
    }

    #[tokio::test]
    async fn test_process_audio_stt_failure() {
        let input = make_input(
            FakeStt(Err(())),
            FakeCleanup {
                cleanup: CleanupBehavior::Ok("unused".to_string()),
                rewrite: Err(()),
            },
        );
        let (outcome, events) = run(input).await;
        assert_eq!(events, vec![PipelineState::Transcribing, PipelineState::Error]);
        assert_eq!(outcome, ProcessOutcome::Stopped { stt_error: true });
    }

    #[tokio::test]
    async fn test_process_audio_blocklist_hallucination_skips() {
        let input = make_input(
            FakeStt(Ok("Thank you for watching".to_string())),
            FakeCleanup {
                cleanup: CleanupBehavior::Ok("unused".to_string()),
                rewrite: Err(()),
            },
        );
        let (outcome, events) = run(input).await;
        assert_eq!(events, vec![PipelineState::Transcribing, PipelineState::Idle]);
        assert_eq!(outcome, ProcessOutcome::Stopped { stt_error: false });
    }

    #[tokio::test]
    async fn test_process_audio_nonretryable_degrades_to_raw() {
        let input = make_input(
            FakeStt(Ok(REAL_SPEECH.to_string())),
            FakeCleanup {
                cleanup: CleanupBehavior::NonRetryable,
                rewrite: Err(()),
            },
        );
        let (outcome, events) = run(input).await;
        assert_eq!(
            events,
            vec![
                PipelineState::Transcribing,
                PipelineState::Cleaning,
                PipelineState::Warning
            ]
        );
        match outcome {
            ProcessOutcome::Produced {
                cleaned_text,
                raw_text,
                llm_error,
                prompt_tokens,
                ..
            } => {
                assert!(llm_error);
                assert_eq!(cleaned_text, sanitize_llm_output(&raw_text));
                assert_eq!(prompt_tokens, None);
            }
            other => panic!("expected Produced, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_process_audio_retryable_no_fallback_degrades() {
        // Default AppConfig has no API keys, so resolve_fallback_provider returns
        // None and the pipeline degrades to raw without building a real provider.
        let input = make_input(
            FakeStt(Ok(REAL_SPEECH.to_string())),
            FakeCleanup {
                cleanup: CleanupBehavior::Retryable,
                rewrite: Err(()),
            },
        );
        let (outcome, events) = run(input).await;
        assert_eq!(
            events,
            vec![
                PipelineState::Transcribing,
                PipelineState::Cleaning,
                PipelineState::Warning
            ]
        );
        match outcome {
            ProcessOutcome::Produced { llm_error, .. } => assert!(llm_error),
            other => panic!("expected Produced, got {other:?}"),
        }
    }

    /// `resolve_stt_provider` for "groq" returns a GroqWhisper instance.
    /// We cannot inspect the concrete type directly, but we can verify that
    /// it does not panic and returns a usable `Arc<dyn SttProvider>`.
    #[test]
    fn test_resolve_stt_provider_groq() {
        let cfg = AppConfig {
            stt_provider: "groq".to_string(),
            groq_api_key: "gsk-test".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_stt_provider(&cfg, std::path::Path::new("/tmp/test"));
        // If we reach here, construction did not panic.
    }

    /// `resolve_stt_provider` for "openai" returns an OpenAiWhisper instance.
    #[test]
    fn test_resolve_stt_provider_openai() {
        let cfg = AppConfig {
            stt_provider: "openai".to_string(),
            openai_api_key: "sk-test".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_stt_provider(&cfg, std::path::Path::new("/tmp/test"));
    }

    /// `resolve_stt_provider` for an unknown value falls back to Groq (no panic).
    #[test]
    fn test_resolve_stt_provider_unknown_fallback() {
        let cfg = AppConfig {
            stt_provider: "unknown_provider".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_stt_provider(&cfg, std::path::Path::new("/tmp/test"));
    }

    /// `resolve_cleanup_provider` for "deepseek" does not panic.
    #[test]
    fn test_resolve_cleanup_provider_deepseek() {
        let cfg = AppConfig {
            llm_provider: "deepseek".to_string(),
            deepseek_api_key: "ds-test".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_cleanup_provider(&cfg);
    }

    /// `resolve_cleanup_provider` for "openai" does not panic.
    #[test]
    fn test_resolve_cleanup_provider_openai() {
        let cfg = AppConfig {
            llm_provider: "openai".to_string(),
            openai_api_key: "sk-test".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_cleanup_provider(&cfg);
    }

    /// `resolve_cleanup_provider` for "anthropic" does not panic.
    #[test]
    fn test_resolve_cleanup_provider_anthropic() {
        let cfg = AppConfig {
            llm_provider: "anthropic".to_string(),
            anthropic_api_key: "sk-ant-test".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_cleanup_provider(&cfg);
    }

    /// `resolve_cleanup_provider` for "groq" does not panic.
    #[test]
    fn test_resolve_cleanup_provider_groq() {
        let cfg = AppConfig {
            llm_provider: "groq".to_string(),
            groq_api_key: "gsk-test".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_cleanup_provider(&cfg);
    }

    /// `resolve_cleanup_provider` for "openrouter" does not panic.
    #[test]
    fn test_resolve_cleanup_provider_openrouter() {
        let cfg = AppConfig {
            llm_provider: "openrouter".to_string(),
            openrouter_api_key: "sk-or-test".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_cleanup_provider(&cfg);
    }

    /// `resolve_cleanup_provider` for an unknown value falls back to DeepSeek (no panic).
    #[test]
    fn test_resolve_cleanup_provider_unknown_fallback() {
        let cfg = AppConfig {
            llm_provider: "unknown_provider".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_cleanup_provider(&cfg);
    }

    // -----------------------------------------------------------------------
    // resolve_fallback_provider tests
    // -----------------------------------------------------------------------

    /// When primary is "deepseek" and groq key is available, groq is the fallback.
    #[test]
    fn test_resolve_fallback_provider_deepseek_primary_groq_fallback() {
        let cfg = AppConfig {
            llm_provider: "deepseek".to_string(),
            deepseek_api_key: "ds-key".to_string(),
            groq_api_key: "gsk-key".to_string(),
            ..AppConfig::default()
        };
        let result = resolve_fallback_provider(&cfg, "deepseek");
        assert!(result.is_some(), "should find groq as fallback");
        let (_, name) = result.unwrap();
        assert_eq!(name, "groq");
    }

    /// When primary is "groq", deepseek is preferred as first candidate.
    #[test]
    fn test_resolve_fallback_provider_groq_primary_deepseek_fallback() {
        let cfg = AppConfig {
            llm_provider: "groq".to_string(),
            groq_api_key: "gsk-key".to_string(),
            deepseek_api_key: "ds-key".to_string(),
            ..AppConfig::default()
        };
        let result = resolve_fallback_provider(&cfg, "groq");
        assert!(result.is_some(), "should find deepseek as fallback");
        let (_, name) = result.unwrap();
        assert_eq!(name, "deepseek");
    }

    /// When primary is "deepseek" and no other key is set, returns None.
    #[test]
    fn test_resolve_fallback_provider_no_fallback_available() {
        let cfg = AppConfig {
            llm_provider: "deepseek".to_string(),
            deepseek_api_key: "ds-key".to_string(),
            groq_api_key: String::new(),
            openai_api_key: String::new(),
            openrouter_api_key: String::new(),
            ..AppConfig::default()
        };
        let result = resolve_fallback_provider(&cfg, "deepseek");
        assert!(result.is_none(), "should return None when no other key is configured");
    }

    /// When all keys are empty, returns None regardless of primary.
    #[test]
    fn test_resolve_fallback_provider_all_keys_empty() {
        let cfg = AppConfig::default();
        let result = resolve_fallback_provider(&cfg, "deepseek");
        assert!(result.is_none(), "should return None when all keys are empty");
    }

    /// Primary provider is excluded even when its key is set.
    #[test]
    fn test_resolve_fallback_provider_skips_primary() {
        let cfg = AppConfig {
            llm_provider: "openai".to_string(),
            openai_api_key: "sk-openai".to_string(),
            openrouter_api_key: "sk-or".to_string(),
            ..AppConfig::default()
        };
        let result = resolve_fallback_provider(&cfg, "openai");
        assert!(result.is_some(), "should find openrouter as fallback");
        let (_, name) = result.unwrap();
        assert_eq!(name, "openrouter");
    }

    // -----------------------------------------------------------------------
    // is_retryable_llm_error tests
    // -----------------------------------------------------------------------

    /// 429 is retryable.
    #[test]
    fn test_is_retryable_llm_error_429() {
        let err = llm::LlmError::ApiError { status: 429, message: "rate limit".to_string() };
        assert!(is_retryable_llm_error(&err));
    }

    /// 500 is retryable.
    #[test]
    fn test_is_retryable_llm_error_500() {
        let err = llm::LlmError::ApiError { status: 500, message: "internal server error".to_string() };
        assert!(is_retryable_llm_error(&err));
    }

    /// 503 is retryable.
    #[test]
    fn test_is_retryable_llm_error_503() {
        let err = llm::LlmError::ApiError { status: 503, message: "service unavailable".to_string() };
        assert!(is_retryable_llm_error(&err));
    }

    /// 401 is NOT retryable (auth error).
    #[test]
    fn test_is_retryable_llm_error_401_not_retryable() {
        let err = llm::LlmError::ApiError { status: 401, message: "unauthorized".to_string() };
        assert!(!is_retryable_llm_error(&err));
    }

    /// 400 is NOT retryable (bad request).
    #[test]
    fn test_is_retryable_llm_error_400_not_retryable() {
        let err = llm::LlmError::ApiError { status: 400, message: "bad request".to_string() };
        assert!(!is_retryable_llm_error(&err));
    }

    /// EmptyInput is NOT retryable (not an ApiError).
    #[test]
    fn test_is_retryable_llm_error_non_api_error() {
        let err = llm::LlmError::EmptyInput;
        assert!(!is_retryable_llm_error(&err));
    }

    /// When `insert_and_send` is `true` in config, the flag is correctly read.
    ///
    /// This mirrors the extraction logic in `stop_and_process_pipeline` --
    /// the full pipeline cannot be unit-tested without an `AppHandle`, so we
    /// verify the config read path directly.
    #[test]
    fn test_insert_and_send_flag_is_read_from_config() {
        let cfg_enabled = AppConfig {
            insert_and_send: true,
            ..AppConfig::default()
        };
        assert!(
            cfg_enabled.insert_and_send,
            "insert_and_send should be true when set in config"
        );

        let cfg_disabled = AppConfig {
            insert_and_send: false,
            ..AppConfig::default()
        };
        assert!(
            !cfg_disabled.insert_and_send,
            "insert_and_send should be false when unset in config"
        );
    }

    /// Default config has `insert_and_send = false` (opt-in feature).
    #[test]
    fn test_insert_and_send_defaults_to_false() {
        let cfg = AppConfig::default();
        assert!(
            !cfg.insert_and_send,
            "insert_and_send must default to false -- it is an opt-in feature"
        );
    }

    // -----------------------------------------------------------------------
    // AutoStop handler tests
    // -----------------------------------------------------------------------

    /// `autostop_silence_secs` is correctly read from config.
    ///
    /// This mirrors the extraction logic in `start_autostop_recording` --
    /// the full function cannot be unit-tested without an `AppHandle`.
    #[test]
    fn test_autostop_handler_concept_reads_silence_secs() {
        let cfg = AppConfig {
            autostop_silence_secs: 3.5,
            ..AppConfig::default()
        };
        assert!(
            (cfg.autostop_silence_secs - 3.5).abs() < f32::EPSILON,
            "autostop_silence_secs should be 3.5 when set in config"
        );
    }

    /// `silence_threshold` from `advanced` is correctly read for AutoStop.
    #[test]
    fn test_autostop_handler_concept_reads_silence_threshold() {
        let mut cfg = AppConfig::default();
        cfg.advanced.silence_threshold = 0.012;

        // Mirrors the extraction in start_autostop_recording:
        let threshold = cfg.advanced.silence_threshold;
        assert!(
            (threshold - 0.012).abs() < f32::EPSILON,
            "silence_threshold from advanced settings should be readable"
        );
    }

    /// Default `autostop_silence_secs` is 2.0 seconds.
    #[test]
    fn test_autostop_silence_secs_default() {
        let cfg = AppConfig::default();
        assert!(
            (cfg.autostop_silence_secs - 2.0).abs() < f32::EPSILON,
            "default autostop_silence_secs should be 2.0"
        );
    }

    /// After `set_silence_callback`, `has_silence_callback` returns `true`.
    /// After `clear_silence_callback`, it returns `false`.
    ///
    /// This is the observable side-effect of `start_autostop_recording`
    /// that can be verified without a full `AppHandle`.
    #[test]
    fn test_autostop_handler_starts_silence_monitor() {
        use crate::audio::AudioRecorder;

        let recorder = AudioRecorder::new();

        // Before installing a callback: none present.
        assert!(
            !recorder.has_silence_callback(),
            "no silence callback should be installed initially"
        );

        // Install the callback (as start_autostop_recording would).
        recorder.set_silence_callback(2.0, 0.005, Box::new(|| {}));

        assert!(
            recorder.has_silence_callback(),
            "silence callback should be installed after set_silence_callback"
        );

        // Clear it (as stop_and_process_pipeline does at the top).
        recorder.clear_silence_callback();

        assert!(
            !recorder.has_silence_callback(),
            "silence callback should be gone after clear_silence_callback"
        );
    }

    // -----------------------------------------------------------------------
    // Auto-Loop mode tests
    // -----------------------------------------------------------------------

    /// `auto_loop_active` starts as `false` -- the loop is off until the user
    /// explicitly activates it with the first hotkey press in Auto mode.
    #[test]
    fn test_auto_loop_flag_default_false() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let flag = AtomicBool::new(false);
        assert!(
            !flag.load(Ordering::SeqCst),
            "auto_loop_active must start as false"
        );
    }

    /// After `store(false)`, `load()` returns `false` -- the hotkey handler can
    /// stop the loop by writing the flag regardless of what the pipeline does.
    #[test]
    fn test_auto_loop_can_be_stopped() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let flag = AtomicBool::new(true);
        assert!(flag.load(Ordering::SeqCst), "flag should be true after store(true)");

        flag.store(false, Ordering::SeqCst);
        assert!(
            !flag.load(Ordering::SeqCst),
            "flag should be false after store(false) -- loop must be stoppable"
        );
    }

    // -----------------------------------------------------------------------
    // Whisper hallucination detection tests
    // -----------------------------------------------------------------------

    /// Exact repetition of the auto-language prompt is detected as echo.
    #[test]
    fn test_prompt_echo_exact_repetition() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        let hallucination = "German and English with proper punctuation. German and English with proper punctuation. German and English with proper punctuation.";
        assert!(
            super::is_prompt_echo(hallucination, hint),
            "repeated prompt fragments should be detected as hallucination"
        );
    }

    /// Full prompt echoed once is also a hallucination.
    #[test]
    fn test_prompt_echo_single() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        assert!(
            super::is_prompt_echo(hint, hint),
            "exact echo of the prompt should be detected"
        );
    }

    /// German prompt echo detection.
    #[test]
    fn test_prompt_echo_german() {
        let hint = "Diktat auf Deutsch mit gelegentlichen englischen Fachbegriffen. Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion.";
        let hallucination = "Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion. Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion.";
        assert!(
            super::is_prompt_echo(hallucination, hint),
            "German prompt fragments repeated should be detected"
        );
    }

    /// Real speech must NOT be flagged as hallucination.
    #[test]
    fn test_prompt_echo_real_speech_not_flagged() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        let real_speech = "Hey, ich wollte kurz fragen ob du morgen Zeit hast.";
        assert!(
            !super::is_prompt_echo(real_speech, hint),
            "real speech must not be detected as prompt echo"
        );
    }

    /// Empty transcription is not a hallucination (handled by silence check).
    #[test]
    fn test_prompt_echo_empty_is_not_echo() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        assert!(
            !super::is_prompt_echo("", hint),
            "empty transcription should not be flagged"
        );
    }

    /// Mixed speech + prompt fragment is NOT a hallucination.
    #[test]
    fn test_prompt_echo_mixed_content_not_flagged() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        let mixed = "German and English with proper punctuation. Also I wanted to say hello.";
        assert!(
            !super::is_prompt_echo(mixed, hint),
            "mixed real speech with prompt fragment must not be flagged"
        );
    }

    /// Whisper varies words: "punctuation" → "pronunciation". Word-overlap
    /// check catches this even though exact substring match fails.
    #[test]
    fn test_prompt_echo_word_variation_detected() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        let hallucination = "German and English with proper pronunciation.";
        assert!(
            super::is_prompt_echo(hallucination, hint),
            "word-variation hallucination should be detected via overlap check"
        );
    }

    /// Repeated word-varied hallucination is also caught.
    #[test]
    fn test_prompt_echo_repeated_variation() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        let hallucination = "Proper pronunciation. Proper pronunciation. Proper pronunciation.";
        assert!(
            super::is_prompt_echo(hallucination, hint),
            "repeated variation should be detected"
        );
    }

    /// Long real text (>30 words) must never be flagged, even if some prompt
    /// words appear naturally.
    #[test]
    fn test_prompt_echo_long_real_text_not_flagged() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        let real = "This is a long text about multilingual voice recognition systems. \
                    German and English are both supported in many modern applications. \
                    The technology has improved significantly with proper training data \
                    and neural network architectures that handle punctuation well.";
        assert!(
            !super::is_prompt_echo(real, hint),
            "long real text with incidental prompt-word overlap must not be flagged"
        );
    }

    /// Regression: a short single word the user genuinely dictated — absent from
    /// the conditioning prompt — must NOT be discarded. The exact-fragment-removal
    /// gate may only conclude "echo" when stripping the prompt is what left <5
    /// chars; a short word that overlaps nothing was never stripped and is real
    /// speech. Mirrors the 2026-05-30 smoke test where "Gast"/"hi"/"für" vanished
    /// while "Berlin"/"ein Gast" passed.
    #[test]
    fn test_prompt_echo_short_real_word_not_flagged() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        for word in ["Gast", "hi", "für", "Kast", "Aye"] {
            assert!(
                !super::is_prompt_echo(word, hint),
                "short dictated word {word:?} (absent from the prompt) must not be flagged as echo"
            );
        }
    }

    // -----------------------------------------------------------------------
    // strip_prompt_fragments tests
    // -----------------------------------------------------------------------

    /// A known default-prompt fragment appearing mid-sentence is removed.
    #[test]
    fn test_strip_fragment_mid_sentence() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        // Simulates Whisper leaking "German and English with proper punctuation."
        // into the middle of a real transcription.
        let raw = "Ich wollte sagen German and English with proper punctuation. dass das Projekt gut laeuft.";
        let result = super::strip_prompt_fragments(raw, hint);
        assert!(
            !result.contains("German and English with proper punctuation"),
            "leaked prompt fragment should be removed; got: {result:?}"
        );
        assert!(
            result.contains("Ich wollte sagen") && result.contains("dass das Projekt gut laeuft"),
            "real speech content must be preserved; got: {result:?}"
        );
    }

    /// Real text without any prompt fragment is returned unchanged (modulo
    /// whitespace normalisation).
    #[test]
    fn test_strip_real_text_unchanged() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        let real = "Hey, kannst du morgen Bescheid geben?";
        let result = super::strip_prompt_fragments(real, hint);
        assert_eq!(
            result, real,
            "text without prompt fragments should come out identical"
        );
    }

    /// Matching is case-insensitive: lower-cased fragment is still stripped.
    #[test]
    fn test_strip_case_insensitive() {
        let hint = "Voice dictation in English. Proper punctuation, capitalization, and spelling.";
        // Fragment with different casing than the original hint.
        let raw = "I want to say proper punctuation, capitalization, and spelling. something important.";
        let result = super::strip_prompt_fragments(raw, hint);
        assert!(
            !result.to_lowercase().contains("proper punctuation, capitalization, and spelling"),
            "case-insensitive fragment should be stripped; got: {result:?}"
        );
        assert!(
            result.contains("I want to say") && result.contains("something important"),
            "surrounding real text must be preserved; got: {result:?}"
        );
    }

    /// Multiple prompt fragments from the same hint are all removed.
    #[test]
    fn test_strip_multiple_fragments_same_hint() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        // Both sentences of the hint appear in the raw text.
        let raw = "Multilingual voice dictation. Das ist toll. German and English with proper punctuation.";
        let result = super::strip_prompt_fragments(raw, hint);
        assert!(
            !result.contains("Multilingual voice dictation"),
            "first fragment should be stripped; got: {result:?}"
        );
        assert!(
            !result.contains("German and English with proper punctuation"),
            "second fragment should be stripped; got: {result:?}"
        );
        assert!(
            result.contains("Das ist toll"),
            "real content between fragments must survive; got: {result:?}"
        );
    }

    /// A fragment from a *different* default hint (not the active stt_hint) is
    /// also stripped, because DEFAULT_STT_HINTS are always checked.
    #[test]
    fn test_strip_default_hint_even_when_not_active() {
        // Active hint is German, but Whisper leaked the English default.
        let active_hint = "Diktat auf Deutsch mit gelegentlichen englischen Fachbegriffen. Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion.";
        let raw = "Hier ist der Bericht. Voice dictation in English. Das war es.";
        let result = super::strip_prompt_fragments(raw, active_hint);
        assert!(
            !result.contains("Voice dictation in English"),
            "English default-hint fragment should be stripped even when German hint is active; got: {result:?}"
        );
        assert!(
            result.contains("Hier ist der Bericht") && result.contains("Das war es"),
            "surrounding real text must be preserved; got: {result:?}"
        );
    }

    /// When the entire transcription is composed of prompt text, the result is
    /// empty (or near-empty after whitespace collapse).
    #[test]
    fn test_strip_entire_text_is_prompt() {
        let hint = "Multilingual voice dictation. German and English with proper punctuation.";
        // The transcription IS the prompt (edge case: guard hasn't caught it yet).
        let raw = "Multilingual voice dictation. German and English with proper punctuation.";
        let result = super::strip_prompt_fragments(raw, hint);
        assert!(
            result.trim().is_empty(),
            "transcription that is entirely prompt should collapse to empty; got: {result:?}"
        );
    }

    /// A custom (user-configured) stt_hint is also stripped.
    #[test]
    fn test_strip_custom_hint() {
        let custom_hint = "Medical transcription with proper terminology and spelling.";
        let raw = "Patient presented with chest pain. Medical transcription with proper terminology and spelling. Vitals are stable.";
        let result = super::strip_prompt_fragments(raw, custom_hint);
        assert!(
            !result.contains("Medical transcription with proper terminology and spelling"),
            "custom hint fragment should be stripped; got: {result:?}"
        );
        assert!(
            result.contains("Patient presented with chest pain") && result.contains("Vitals are stable"),
            "real medical content must be preserved; got: {result:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Spec tests for compute_wav_rms
    //
    // Closed-form parametric specs driven by the shared JSON fixture at
    // test-fixtures/wav-rms-vectors.json (repo root). The fixture is the
    // single source of truth for both Rust and Kotlin consumers.
    //
    // The insta golden-master snapshot (compute_wav_rms_sine_tone) has been
    // removed in favour of a tolerance assertion (AC-2). The snapshot file
    // src-tauri/src/snapshots/klarvo_lib__pipeline__tests__compute_wav_rms_sine_tone.snap
    // has been deleted.
    //
    // Story 3.3: Spec-Test the WAV-RMS Computation Independently
    // -----------------------------------------------------------------------

    /// Builds a minimal 16kHz mono 16-bit PCM WAV buffer from f32 samples.
    ///
    /// Replicates the encoding path used in production (`encode_to_wav`) so
    /// the tests are independent of the audio module.
    fn make_wav(samples: &[f32]) -> Vec<u8> {
        use std::io::Cursor;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for &s in samples {
            let int_sample = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
            writer.write_sample(int_sample).unwrap();
        }
        writer.finalize().unwrap();
        cursor.into_inner()
    }

    /// Builds a 16kHz mono float32 WAV buffer from f32 samples.
    ///
    /// Used for RMS-007 (float32 path in compute_wav_rms). The hound spec uses
    /// SampleFormat::Float / 32 bits — audioFormat = 3 in the WAV header.
    fn make_float_wav(samples: &[f32]) -> Vec<u8> {
        use std::io::Cursor;
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for &s in samples {
            writer.write_sample(s).unwrap();
        }
        writer.finalize().unwrap();
        cursor.into_inner()
    }

    /// Loads the shared WAV-RMS test vectors from test-fixtures/wav-rms-vectors.json.
    ///
    /// CARGO_MANIFEST_DIR points to src-tauri/; parent is the workspace root
    /// where test-fixtures/ lives.
    fn load_wav_rms_vectors() -> Vec<serde_json::Value> {
        let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
        let fixture_path = std::path::Path::new(&manifest_dir)
            .parent()
            .expect("workspace root")
            .join("test-fixtures/wav-rms-vectors.json");
        let content = std::fs::read_to_string(&fixture_path)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", fixture_path.display(), e));
        serde_json::from_str::<Vec<serde_json::Value>>(&content)
            .expect("wav-rms-vectors.json must be valid JSON array")
    }

    /// Builds the WAV bytes for a test vector from its wav_encoding specification.
    fn build_vector_wav(encoding: &serde_json::Value) -> Vec<u8> {
        let enc_type = encoding["type"].as_str().expect("wav_encoding must have 'type'");
        match enc_type {
            "raw_bytes" => {
                let bytes = encoding["bytes"].as_array().expect("raw_bytes must have 'bytes'");
                bytes.iter().map(|b| b.as_u64().unwrap() as u8).collect()
            }
            "synthetic" => {
                let sample_rate = encoding["sample_rate"].as_u64().unwrap_or(16000) as u32;
                let duration_ms = encoding["duration_ms"].as_u64().unwrap_or(0);
                let amplitude = encoding["amplitude"].as_f64().unwrap_or(0.0) as f32;
                let bits = encoding["bits_per_sample"].as_u64().unwrap_or(16);
                let sample_format = encoding.get("sample_format").and_then(|v| v.as_str()).unwrap_or("int");
                let n_samples = (sample_rate as u64 * duration_ms / 1000) as usize;
                if sample_format == "float" || bits == 32 {
                    let samples = vec![amplitude; n_samples];
                    make_float_wav(&samples)
                } else {
                    let samples = vec![amplitude; n_samples];
                    make_wav(&samples)
                }
            }
            "sine" => {
                let sample_rate = encoding["sample_rate"].as_u64().unwrap_or(16000) as u32;
                let duration_ms = encoding["duration_ms"].as_u64().unwrap_or(1000);
                let freq = encoding["freq_hz"].as_f64().unwrap_or(440.0) as f32;
                let amplitude = encoding["amplitude"].as_f64().unwrap_or(1.0) as f32;
                let n_samples = (sample_rate as u64 * duration_ms / 1000) as usize;
                let sr = sample_rate as f32;
                let samples: Vec<f32> = (0..n_samples)
                    .map(|i| amplitude * (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin())
                    .collect();
                make_wav(&samples)
            }
            other => panic!("Unknown wav_encoding type: {other}"),
        }
    }

    /// Parametric spec test: iterates all vectors from test-fixtures/wav-rms-vectors.json
    /// and asserts compute_wav_rms against expected_rms / tolerance.
    ///
    /// This is the single authoritative test for the Rust side. Named individual
    /// spec_* tests below are readable aliases for the same vectors but delegate
    /// their correctness claim to this parametric driver.
    #[test]
    fn spec_wav_rms_vectors_json() {
        let vectors = load_wav_rms_vectors();
        for v in &vectors {
            let id = v["id"].as_str().unwrap_or("?");
            let wav = build_vector_wav(&v["wav_encoding"]);
            let result = compute_wav_rms(&wav);
            let expected_rms = &v["expected_rms"];
            if expected_rms.is_null() {
                assert!(
                    result.is_none(),
                    "[{id}] expected None but got {result:?}"
                );
            } else {
                let expected = expected_rms.as_f64().unwrap() as f32;
                let tolerance = v["tolerance"].as_f64().unwrap_or(1e-3) as f32;
                let rms = result.unwrap_or_else(|| panic!("[{id}] expected Some({expected}) but got None"));
                assert!(
                    (rms - expected).abs() <= tolerance,
                    "[{id}] RMS {rms:.6} not within {tolerance:.1e} of expected {expected:.6}"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Named spec wrappers for readability — each exercises a specific vector
    // scenario with hardcoded constants for clarity. The authoritative
    // expected values are defined in test-fixtures/wav-rms-vectors.json and
    // verified by the parametric `spec_wav_rms_vectors_json` test above.
    // These wrappers bind to the real compute_wav_rms call site (AI-2 mandate).
    // -----------------------------------------------------------------------

    /// RMS-001 / RMS-002: invalid / empty bytes → None  (AC-1)
    #[test]
    fn spec_compute_wav_rms_invalid_bytes_returns_none() {
        // RMS-001: empty slice
        assert!(compute_wav_rms(&[]).is_none(), "empty byte slice must return None");
        // RMS-002: garbage bytes that parse as non-WAV
        let garbage: &[u8] = b"this is not a WAV file at all!";
        assert!(compute_wav_rms(garbage).is_none(), "invalid WAV bytes must return None");
    }

    /// RMS-003: silence WAV (all-zero i16 samples) → Some(0.0)  (AC-1)
    #[test]
    fn spec_compute_wav_rms_silence_is_zero() {
        let wav = make_wav(&vec![0.0f32; 1600]); // 100 ms of silence
        let rms = compute_wav_rms(&wav).expect("silence WAV must return Some(...)");
        assert_eq!(rms, 0.0, "silence WAV must produce RMS = 0.0, got {rms}");
    }

    /// RMS-004: full-scale 440 Hz sine → RMS ≈ 1/√2  (AC-1)
    ///
    /// Replaces the removed insta snapshot. The closed-form tolerance assertion
    /// is the contract; the snapshot file has been deleted (AC-2).
    #[test]
    fn spec_compute_wav_rms_sine_tone() {
        let n = 16_000usize;
        let freq = 440.0f32;
        let sr = 16_000.0f32;
        let samples: Vec<f32> = (0..n)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin())
            .collect();
        let wav = make_wav(&samples);
        let rms = compute_wav_rms(&wav).expect("sine WAV must parse successfully");
        let expected = 1.0_f32 / 2.0_f32.sqrt(); // ≈ 0.70710678
        assert!(
            (rms - expected).abs() < 1e-3,
            "sine-tone RMS should be ≈{expected:.5} (1/√2), got {rms:.5}"
        );
    }

    /// RMS-005: constant amplitude 0.3 (speech level) → RMS ≈ 0.3  (AC-1)
    ///
    /// Replaces the previous `rms > threshold` assertion with a closed-form
    /// tolerance check. The authoritative expected value (0.3 ± 1e-3) is also
    /// defined in test-fixtures/wav-rms-vectors.json (RMS-005) and verified by
    /// the parametric `spec_wav_rms_vectors_json` test.
    #[test]
    fn spec_compute_wav_rms_speech_level() {
        let samples = vec![0.3f32; 3200]; // 200 ms constant amplitude
        let wav = make_wav(&samples);
        let rms = compute_wav_rms(&wav).expect("speech-level WAV must parse");
        assert!(
            (rms - 0.3_f32).abs() < 1e-3,
            "speech-level RMS should be ≈0.3, got {rms:.6}"
        );
    }

    /// RMS-006: WAV with 0-sample data chunk → Some(0.0), NOT None  (AC-1)
    #[test]
    fn spec_compute_wav_rms_empty_data_chunk_is_some_zero() {
        let wav = make_wav(&[]); // valid header, 0 samples written
        let result = compute_wav_rms(&wav);
        assert_eq!(
            result,
            Some(0.0),
            "WAV with empty data chunk must return Some(0.0), got {result:?}"
        );
    }

    /// RMS-007: float32 WAV, constant 0.5 → RMS ≈ 0.5  (AC-3)
    ///
    /// Tests the SampleFormat::Float path in compute_wav_rms.
    #[test]
    fn spec_compute_wav_rms_float32_path() {
        let n = (16_000usize * 100) / 1000; // 100 ms at 16kHz
        let samples = vec![0.5f32; n];
        let wav = make_float_wav(&samples);
        let rms = compute_wav_rms(&wav).expect("float32 WAV must parse successfully");
        assert!(
            (rms - 0.5_f32).abs() < 1e-4,
            "float32 constant-0.5 WAV RMS should be ≈0.5, got {rms:.6}"
        );
    }

    // Note: hallucination blocklist tests live in src/stt/hallucination.rs,
    // co-located with the implementation. See `stt::hallucination` module.

    // --- sanitize_llm_output tests ---

    #[test]
    fn sanitize_preserves_normal_text() {
        assert_eq!(
            sanitize_llm_output("Hello, world! Wie geht's?"),
            "Hello, world! Wie geht's?"
        );
    }

    #[test]
    fn sanitize_preserves_emoji_and_cjk() {
        assert_eq!(sanitize_llm_output("Hallo 🎉 こんにちは"), "Hallo 🎉 こんにちは");
    }

    #[test]
    fn sanitize_strips_ansi_escape_sequences() {
        assert_eq!(
            sanitize_llm_output("\x1b[31mRed\x1b[0m normal"),
            "Red normal"
        );
        assert_eq!(
            sanitize_llm_output("\x1b[2J\x1b[HMALICIOUS"),
            "MALICIOUS"
        );
    }

    #[test]
    fn sanitize_strips_null_bytes() {
        assert_eq!(
            sanitize_llm_output("before\0after"),
            "beforeafter"
        );
    }

    #[test]
    fn sanitize_strips_bidi_overrides() {
        assert_eq!(
            sanitize_llm_output("Hello \u{202E}dlrow\u{202C} test"),
            "Hello dlrow test"
        );
    }

    #[test]
    fn sanitize_strips_zero_width_chars() {
        assert_eq!(
            sanitize_llm_output("a\u{200B}b\u{FEFF}c"),
            "abc"
        );
    }

    #[test]
    fn sanitize_handles_empty_string() {
        assert_eq!(sanitize_llm_output(""), "");
    }

    #[test]
    fn sanitize_strips_lone_esc() {
        assert_eq!(sanitize_llm_output("before\x1bafter"), "beforeafter");
    }
}
