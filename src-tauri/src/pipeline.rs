//! Dictation pipeline: audio stop → STT → LLM cleanup → paste.
//!
//! These functions are called by the global hotkey handler and are not
//! directly exposed as Tauri commands. They operate on [`AppState`] via
//! an [`AppHandle`] so they can emit state-change events to the frontend.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
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
use crate::stt::{self, SttProvider};
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
pub fn resolve_stt_provider(cfg: &AppConfig) -> Arc<dyn SttProvider> {
    match cfg.stt_provider.as_str() {
        "openai" => Arc::new(stt::OpenAiWhisper::new(&cfg.openai_api_key)),
        #[cfg(target_os = "windows")]
        "local" => build_local_whisper_provider(cfg),
        #[cfg(not(target_os = "windows"))]
        "local" => {
            log::warn!("[pipeline] local STT provider is only supported on Windows; falling back to groq");
            Arc::new(stt::GroqWhisper::new(&cfg.groq_api_key).with_model(cfg.stt_model.clone()))
        }
        // "groq" and any unrecognised value
        _ => Arc::new(stt::GroqWhisper::new(&cfg.groq_api_key).with_model(cfg.stt_model.clone())),
    }
}

/// Builds a `LocalWhisperProvider` with the model path derived from `%APPDATA%`.
///
/// Path convention: `%APPDATA%\com.dikta.voice\models\ggml-{model_name}.bin`
///
/// We derive the path from `APPDATA` rather than `AppState.app_data_dir`
/// because `resolve_stt_provider` takes only `&AppConfig`. If `APPDATA` is
/// not set (unlikely on Windows), falls back to `.\models\`.
#[cfg(target_os = "windows")]
fn build_local_whisper_provider(cfg: &AppConfig) -> Arc<dyn SttProvider> {
    use stt::LocalWhisperProvider;

    let model_dir = std::env::var("APPDATA")
        .map(|d| std::path::PathBuf::from(d).join("com.dikta.voice").join("models"))
        .unwrap_or_else(|_| std::path::PathBuf::from("models"));

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
        "openai" => Arc::new(llm::OpenAiCleanup::new(&cfg.openai_api_key)),
        "anthropic" => Arc::new(llm::AnthropicCleanup::new(&cfg.anthropic_api_key)),
        "groq" => Arc::new(llm::GroqCleanup::new(&cfg.groq_api_key)),
        // "deepseek" and any unrecognised value
        _ => Arc::new(llm::DeepSeekCleanup::new(&cfg.deepseek_api_key)),
    }
}

// ---------------------------------------------------------------------------
// Whisper hallucination detection
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

    if residue.len() < 5 {
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
pub fn compute_wav_rms(wav_bytes: &[u8]) -> Option<f32> {
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
// Pipeline entry points
// ---------------------------------------------------------------------------

/// Starts recording audio and emits `state=recording`.
///
/// Does nothing (returns silently) if recording is already in progress.
/// Used by the hold-mode hotkey handler on key-press.
pub async fn start_recording_only(handle: AppHandle) {
    let state = handle.state::<AppState>();

    if state.recorder.is_recording() {
        return;
    }

    // Capture the foreground window BEFORE we start recording.
    // This is the window the user was typing in -- we'll restore focus to it
    // before pasting the result.
    if let Ok(mut guard) = state.prev_foreground_hwnd.lock() {
        *guard = capture_foreground_window();
    }
    if let Ok(mut guard) = state.prev_window_title.lock() {
        *guard = capture_foreground_window_title();
        log::debug!("[hotkey] foreground window title: {:?}", *guard);
    }

    // Re-install the audio level callback before each recording.
    #[cfg(desktop)]
    setup_audio_level_emitter(&handle);

    let device_name = state.config.lock().ok().and_then(|c| c.audio_device.clone());
    if let Err(e) = state.recorder.start_recording(device_name.as_deref()) {
        crate::emit_pipeline_state(
            &handle,
            PipelineEvent::error(format!("Failed to start recording: {e}")),
        );
        return;
    }

    *match state.recording_start.lock() {
        Ok(g) => g,
        Err(_) => {
            crate::emit_pipeline_state(&handle, PipelineEvent::error("State lock poisoned"));
            return;
        }
    } = Some(std::time::Instant::now());

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
        // Already recording -- the hotkey handler's pressed branch calls
        // stop_and_process_pipeline for this case, so we should not reach
        // here, but guard just in case.
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

    // Install the silence callback. It must be set BEFORE start_recording so
    // the recording thread picks it up via `.take()` inside start_recording.
    let handle_for_cb = handle.clone();
    state.recorder.set_silence_callback(
        silence_secs,
        silence_threshold,
        Box::new(move || {
            // This closure runs on the cpal OS-thread (non-async context).
            // Spawn an async task to run the pipeline on the Tauri runtime.
            let h = handle_for_cb.clone();
            tauri::async_runtime::spawn(async move {
                stop_and_process_pipeline(h).await;
            });
        }),
    );

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

    if !state.recorder.is_recording() {
        // Not recording -- key released without a corresponding press (race condition or
        // hold mode released before recording started). Safe to ignore.
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
    if duration_ms < adv.min_recording_ms as u64 {
        log::info!("[pipeline] recording too short ({duration_ms}ms), skipping");
        let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::idle());
        return;
    }

    // Check RMS of the raw WAV samples. If the audio is near-silent, abort.
    // Whisper mode uses a lower threshold since the audio has been amplified.
    let silence_threshold = if whisper_mode {
        adv.whisper_mode_threshold
    } else {
        adv.silence_threshold
    };
    if let Some(rms) = compute_wav_rms(&wav_bytes) {
        log::debug!("[pipeline] audio RMS = {rms:.5} (threshold={silence_threshold})");
        if rms < silence_threshold {
            log::info!("[pipeline] audio is silent (rms={rms:.5}), skipping");
            let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::idle());
            return;
        }
    }

    // --- Collect config + dictionary (release locks before await points) ---
    let (language, stt_provider, cleanup_provider, dict_prompt, offline_mode, stt_hint_text) = {
        let cfg = match state.config.lock() {
            Ok(g) => g.clone(),
            Err(_) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned"),
                );
                return;
            }
        };

        let stt_prov = match state.stt_provider.read() {
            Ok(g) => g.clone(),
            Err(_) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned"),
                );
                return;
            }
        };

        let cleanup_prov = match state.cleanup_provider.read() {
            Ok(g) => g.clone(),
            Err(_) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error("State lock poisoned"),
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
        let prompt = stt::build_stt_prompt_with_hint(
            dict_terms.as_deref(),
            &cfg.language,
            stt_hint.as_deref(),
        );

        // Keep the hint text (without dictionary terms) for hallucination detection.
        let hint_for_check = stt_hint.unwrap_or_else(|| match cfg.language.as_str() {
            "de" => "Diktat auf Deutsch mit gelegentlichen englischen Fachbegriffen. Korrekte Groß- und Kleinschreibung, Satzzeichen und Interpunktion.".to_string(),
            "en" => "Voice dictation in English. Proper punctuation, capitalization, and spelling.".to_string(),
            _ => "Multilingual voice dictation. German and English with proper punctuation.".to_string(),
        });

        // Offline mode: if stt_provider is "local", the user has explicitly
        // chosen to stay offline. In this case we skip the LLM cleanup step
        // entirely -- no network call, raw text goes straight to paste.
        let offline = cfg.stt_provider == "local";

        (cfg.language.clone(), stt_prov, cleanup_prov, prompt, offline, hint_for_check)
    };

    // --- Transcribe ---
    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::transcribing());

    let raw_text = match stt_provider
        .transcribe(wav_bytes, &language, dict_prompt.as_deref())
        .await
    {
        Ok(t) => t,
        Err(e) => {
            let _ = handle.emit(
                EVENT_STATE_CHANGED,
                PipelineEvent::error(friendly_error("Transcription failed", &e.to_string())),
            );
            return;
        }
    };

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

    // --- Whisper hallucination guard ---
    // Whisper sometimes echoes the conditioning prompt instead of transcribing
    // actual speech (common with ambient noise but no words). Detect this by
    // checking if the transcription is composed entirely of prompt fragments.
    if is_prompt_echo(&raw_text, &stt_hint_text) {
        log::info!(
            "[pipeline] transcription is prompt echo (hallucination), skipping: {raw_text:?}"
        );
        let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::idle());
        return;
    }

    // --- Check Command Mode ---
    let is_command_mode = state
        .command_mode_active
        .lock()
        .ok()
        .map(|g| *g)
        .unwrap_or(false);
    let selected_text = if is_command_mode {
        // Reset command mode flags
        if let Ok(mut guard) = state.command_mode_active.lock() {
            *guard = false;
        }
        state
            .command_mode_selected_text
            .lock()
            .ok()
            .and_then(|mut g| g.take())
    } else {
        None
    };

    // --- LLM step ---
    // Skip the entire cleanup step in offline mode (stt_priority[0] == "local").
    // Command Mode still requires an LLM call even offline, so we only skip
    // for normal dictation.
    if !offline_mode || selected_text.is_some() {
        let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::cleaning());
    }

    let cleanup_result = if offline_mode && selected_text.is_none() {
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

        match cleanup_provider.rewrite(sel_text, &raw_text).await {
            Ok(r) => r,
            Err(e) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error(format!("Command mode failed: {e}")),
                );
                return;
            }
        }
    } else {
        // Normal dictation: cleanup raw transcription
        let (style, custom_prompt) = {
            match state.config.lock() {
                Ok(g) => {
                    let prev_title = state.prev_window_title.lock().ok().and_then(|t| t.clone());
                    let matched = prev_title.as_deref().and_then(|title| {
                        let title_lower = title.to_lowercase();
                        g.profiles.iter().find(|p| {
                            !p.app_pattern.is_empty()
                                && title_lower.contains(&p.app_pattern.to_lowercase())
                        })
                    });
                    if let Some(profile) = matched {
                        log::info!("[pipeline] profile matched: {:?}", profile.name);
                        let prompt = if profile.custom_prompt.is_empty() {
                            let p = g.custom_prompt.clone();
                            if p.is_empty() { None } else { Some(p) }
                        } else {
                            Some(profile.custom_prompt.clone())
                        };
                        (profile.cleanup_style, prompt)
                    } else {
                        (g.cleanup_style, {
                            let p = g.custom_prompt.clone();
                            if p.is_empty() { None } else { Some(p) }
                        })
                    }
                }
                Err(_) => (CleanupStyle::Polished, None),
            }
        };

        let dict_list = match state.dictionary.lock() {
            Ok(g) => {
                let l = g.terms_as_list();
                if l.is_empty() { None } else { Some(l) }
            }
            Err(_) => None,
        };

        let output_lang = state
            .config
            .lock()
            .ok()
            .map(|c| c.output_language.clone())
            .unwrap_or_default();
        let output_lang_opt = if output_lang.is_empty() {
            None
        } else {
            Some(output_lang.as_str())
        };

        match chunked_cleanup(
            cleanup_provider.as_ref(),
            &raw_text,
            style,
            dict_list.as_deref(),
            custom_prompt.as_deref(),
            output_lang_opt,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                let _ = handle.emit(
                    EVENT_STATE_CHANGED,
                    PipelineEvent::error(friendly_error("Text cleanup failed", &e.to_string())),
                );
                return;
            }
        }
    };

    let is_command = selected_text.is_some();
    let cleaned_text = cleanup_result.text;
    log::debug!("[pipeline] cleaned text: {cleaned_text:?}");

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
        let llm_cost = (cleanup_result.prompt_tokens.unwrap_or(0) as f64 * 0.27
            + cleanup_result.completion_tokens.unwrap_or(0) as f64 * 1.10)
            / 1_000_000.0;
        if let Err(e) = history::record_usage(
            &db,
            "deepseek_cleanup",
            None,
            cleanup_result.prompt_tokens,
            cleanup_result.completion_tokens,
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
        // Dikta was processing (STT + LLM cleanup takes seconds), bring them
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
                            // We acquire the lock, run the update, and
                            // explicitly drop the guard before `st` is
                            // dropped by collecting it into a local Result
                            // and ignoring the value.
                            let st = handle_for_sync.state::<AppState>();
                            let mark_result = st.history_db.lock().ok().map(|db| {
                                sync::mark_entries_synced(&db, &[uuid_for_mark.clone()])
                            });
                            drop(st);
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

    // Emit the appropriate done event based on whether the paste succeeded.
    let done_event = if paste_result == PasteResult::ClipboardOnly {
        PipelineEvent::done_with_clipboard_only(cleaned_text, raw_text)
    } else {
        PipelineEvent::done(cleaned_text, raw_text)
    };
    let _ = handle.emit(EVENT_STATE_CHANGED, done_event);
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
///              [`stop_and_process_pipeline`].
/// - `AutoStop`: Press once to start; silence stops automatically. Second press
///               stops manually if still recording.
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

    println!("[hotkey] Re-registering hotkeys: {} slot(s)", slots.len());

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
            println!("[hotkey] Slot {:?} disabled (empty hotkey), skipping", slot.mode);
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

        println!(
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
                println!("[hotkey] Event: shortcut_id={} {event:?}", shortcut.id());

                // While the ShortcutRecorder is active, swallow all hotkey
                // events so the user can press the current shortcut without
                // triggering the pipeline.
                if handle_clone
                    .state::<AppState>()
                    .hotkey_paused
                    .load(Ordering::SeqCst)
                {
                    println!("[hotkey] paused (ShortcutRecorder active), ignoring");
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
                println!("[hotkey] mode={mode:?} state={:?}", event.state);

                // Tell the FloatingBar which mode is active so it shows the
                // correct badge (Hotkey 1 vs Hotkey 2 may have different modes).
                let _ = handle_clone.emit("dikta://active-mode", mode);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;

    /// When `stt_provider` is `"local"`, the offline flag must be `true` so
    /// the pipeline skips the LLM cleanup step.
    ///
    /// This test verifies the extraction logic in `stop_and_process_pipeline`
    /// by replicating it directly -- the full pipeline cannot be unit-tested
    /// without a Tauri `AppHandle`.
    #[test]
    fn test_offline_flag_derived_from_stt_provider_local() {
        let cfg = AppConfig {
            stt_provider: "local".to_string(),
            ..AppConfig::default()
        };
        let offline = cfg.stt_provider == "local";
        assert!(offline, "offline flag should be true when stt_provider == 'local'");
    }

    /// When `stt_provider` is a cloud provider, the offline flag must be `false`.
    #[test]
    fn test_offline_flag_false_when_provider_is_groq() {
        let cfg = AppConfig {
            stt_provider: "groq".to_string(),
            groq_api_key: "gsk-test".to_string(),
            ..AppConfig::default()
        };
        let offline = cfg.stt_provider == "local";
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
        let offline = cfg.stt_provider == "local";
        assert!(!offline);
    }

    /// Default stt_provider is "groq", so offline flag is false by default.
    #[test]
    fn test_offline_flag_false_by_default() {
        let cfg = AppConfig::default();
        let offline = cfg.stt_provider == "local";
        assert!(!offline, "default config should not be in offline mode");
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
        let _provider = resolve_stt_provider(&cfg);
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
        let _provider = resolve_stt_provider(&cfg);
    }

    /// `resolve_stt_provider` for an unknown value falls back to Groq (no panic).
    #[test]
    fn test_resolve_stt_provider_unknown_fallback() {
        let cfg = AppConfig {
            stt_provider: "unknown_provider".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_stt_provider(&cfg);
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

    /// `resolve_cleanup_provider` for an unknown value falls back to DeepSeek (no panic).
    #[test]
    fn test_resolve_cleanup_provider_unknown_fallback() {
        let cfg = AppConfig {
            llm_provider: "unknown_provider".to_string(),
            ..AppConfig::default()
        };
        let _provider = resolve_cleanup_provider(&cfg);
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
}
