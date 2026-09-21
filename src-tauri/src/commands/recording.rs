//! Tauri commands for recording and transcription.
//!
//! These commands expose the audio capture and STT pipeline to the frontend.
//! They are the building blocks of the manual dictation flow (as opposed to
//! the fully automatic hotkey pipeline in `pipeline.rs`).

use tauri::{AppHandle, State};

use crate::audio;
use crate::license::LicensedFeature;
use crate::llm::{chunked_cleanup, CleanupStyle};
use crate::paste::{capture_foreground_window, capture_foreground_window_title};
use crate::stt::{self, build_stt_prompt};
use crate::{require_license, AppState, RecordingInfo};

#[cfg(desktop)]
use crate::setup_audio_level_emitter;

/// Opens the default microphone and starts capturing audio.
///
/// Returns an error string if recording is already in progress or no
/// microphone is available.
#[tauri::command]
pub async fn start_recording(
    handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let inner = state.inner();

    // Capture the foreground window BEFORE we start recording.
    // This is the window the user was typing in -- we'll restore focus to it
    // before pasting the result.
    if let Ok(mut guard) = inner.prev_foreground_hwnd.lock() {
        *guard = capture_foreground_window();
    }
    if let Ok(mut guard) = inner.prev_window_title.lock() {
        *guard = capture_foreground_window_title();
        log::debug!("[start_recording] foreground window title: {:?}", *guard);
    }

    // Re-install the audio level callback before recording.
    #[cfg(desktop)]
    setup_audio_level_emitter(&handle);

    let device_name = crate::lock!(inner.config)?.audio_device.clone();
    inner
        .recorder
        .start_recording(device_name.as_deref())
        .map_err(|e: audio::AudioError| e.to_string())?;

    *crate::lock!(inner.recording_start)? = Some(std::time::Instant::now());

    Ok(())
}

/// Stops the active recording and stores the WAV bytes in `AppState`.
///
/// Returns `RecordingInfo` with the recording duration. This command does NOT
/// run STT or cleanup -- call `transcribe_audio` and `cleanup_text` for that.
///
/// Returns an error if no recording is active.
#[tauri::command]
pub async fn stop_recording(state: State<'_, AppState>) -> Result<RecordingInfo, String> {
    let inner = state.inner();

    // Measure duration before stopping (start timestamp is cleared below).
    let duration_ms = {
        let start_guard = crate::lock!(inner.recording_start)?;
        start_guard
            .map(|t: std::time::Instant| t.elapsed().as_millis() as u64)
            .unwrap_or(0)
    };

    // Stop the cpal stream and get WAV bytes.
    let wav_bytes = inner
        .recorder
        .stop_recording()
        .map_err(|e: audio::AudioError| e.to_string())?;

    // Persist WAV for the subsequent `transcribe_audio` call.
    *crate::lock!(inner.last_recording)? = Some(wav_bytes);

    // Clear the start timestamp.
    *crate::lock!(inner.recording_start)? = None;

    Ok(RecordingInfo { duration_ms })
}

/// Returns the ID of the active STT provider from `stt_provider` config field.
///
/// Returns `"groq"` as fallback if the config lock fails.
fn active_stt_provider_id(state: &AppState) -> String {
    state
        .config
        .lock()
        .ok()
        .map(|c| c.stt_provider.clone())
        .unwrap_or_else(|| "groq".to_string())
}

/// The whole offline decision of [`cleanup_text`]: `Some(raw_text)` when this
/// dictation makes no cleanup call, `None` to continue to the provider.
///
/// Story 13-2 (E2 / D-M20). `is_offline_mode` is gone — it was Rust's
/// **second** definition of "offline" (`stt_provider == "local"` alone), so the
/// React in-app record button skipped a cleanup the hotkey performed for the
/// very same config. There is one rule now, `pipeline::config_skips_cleanup`.
///
/// Extracted at review because `cleanup_text` is a `#[tauri::command]` taking
/// `State<'_, AppState>`, whose constructor is `pub(crate)` **to Tauri** — no
/// test in this crate can build one, so nothing exercised the command at all
/// and a second rule reintroduced inside it would have stayed green. That is
/// D-M20 verbatim, one level up. This is `&AppState`-shaped, so
/// `spec_in_app_button_offline_returns_the_raw_text_unchanged` drives the real
/// branch.
///
/// A poisoned config lock answers "not offline", which is the shipped
/// fail-direction: the alternative is silently swallowing a cleanup the user
/// asked for.
pub(crate) fn offline_passthrough(state: &AppState, raw_text: &str) -> Option<String> {
    let skips = state
        .config
        .lock()
        .ok()
        .map(|c| crate::pipeline::config_skips_cleanup(&c))
        .unwrap_or(false);
    if skips {
        log::info!("[cleanup] Offline rule: returning raw text without cleanup");
        Some(raw_text.to_string())
    } else {
        None
    }
}

/// Returns the ID of the active LLM cleanup provider based on the priority list and available keys.
///
/// Walks `llm_priority` and returns the ID of the first provider with a non-empty key.
/// Returns `"deepseek"` as fallback (matching `resolve_cleanup_provider` behaviour).
fn active_llm_provider_id(state: &AppState) -> String {
    let cfg = match state.config.lock() {
        Ok(g) => g,
        Err(_) => return "deepseek".to_string(),
    };
    for id in &cfg.llm_priority {
        match id.as_str() {
            "deepseek" if !cfg.deepseek_api_key.is_empty() => return "deepseek".to_string(),
            "openai" if !cfg.openai_api_key.is_empty() => return "openai".to_string(),
            "anthropic" if !cfg.anthropic_api_key.is_empty() => return "anthropic".to_string(),
            "groq" if !cfg.groq_api_key.is_empty() => return "groq".to_string(),
            _ => continue,
        }
    }
    "deepseek".to_string()
}

/// Transcribes the most recently recorded audio using the configured STT provider.
///
/// Reads WAV bytes stored by the last `stop_recording` call.
/// Dictionary terms are injected as a Groq `prompt` hint to improve accuracy
/// for technical vocabulary.
///
/// `language`: ISO-639-1 code (e.g. `"de"`, `"en"`). Empty string = auto-detect.
///
/// Returns an error if no recording is available or the STT call fails.
#[tauri::command]
pub async fn transcribe_audio(
    state: State<'_, AppState>,
    language: String,
) -> Result<String, String> {
    let inner = state.inner();

    // License gate: non-Groq STT providers require a paid license.
    if active_stt_provider_id(inner) != "groq" {
        require_license!(state, LicensedFeature::AlternativeProviders);
    }

    // Clone the WAV out of the mutex so we don't hold the lock across the await.
    let wav_bytes = {
        let guard = crate::lock!(inner.last_recording)?;
        guard
            .clone()
            .ok_or_else(|| "No recording available. Call stop_recording first.".to_string())?
    };

    // Read dictionary terms for the STT prompt hint.
    let dict_prompt = {
        let guard = crate::lock!(inner.dictionary)?;
        let terms = guard.terms_as_prompt();
        let terms_opt = if terms.is_empty() { None } else { Some(terms) };
        build_stt_prompt(terms_opt.as_deref(), &language)
    };

    // Read the current provider (shared read lock -- no contention with other readers).
    let provider = crate::read_lock!(inner.stt_provider)?.clone();

    provider
        .transcribe(&wav_bytes, &language, dict_prompt.as_deref())
        .await
        .map_err(|e: stt::SttError| e.to_string())
}

/// Transcribes raw audio bytes passed directly from the frontend.
///
/// Intended for Android, where `cpal` is not available and audio capture is
/// handled on the JavaScript/Kotlin side. The bytes are stored as
/// `last_recording` so the rest of the pipeline (history, stats) can reference
/// them, then the same STT provider pipeline as `transcribe_audio` is used.
///
/// `audio_data`: raw WAV or PCM bytes recorded by the caller.
/// `language`: ISO-639-1 code (e.g. `"de"`, `"en"`). Empty string = auto-detect.
#[tauri::command]
pub async fn transcribe_audio_bytes(
    state: State<'_, AppState>,
    audio_data: Vec<u8>,
    language: String,
) -> Result<String, String> {
    let inner = state.inner();

    // License gate: non-Groq STT providers require a paid license.
    if active_stt_provider_id(inner) != "groq" {
        require_license!(state, LicensedFeature::AlternativeProviders);
    }

    // Store the audio data as last_recording so history/stats can reference it.
    {
        let mut guard = crate::lock!(inner.last_recording)?;
        *guard = Some(audio_data.clone());
    }

    // Read dictionary terms for the STT prompt hint.
    let dict_prompt = {
        let guard = crate::lock!(inner.dictionary)?;
        let terms = guard.terms_as_prompt();
        let terms_opt = if terms.is_empty() { None } else { Some(terms) };
        build_stt_prompt(terms_opt.as_deref(), &language)
    };

    // Read the current provider (shared read lock -- no contention with other readers).
    let provider = crate::read_lock!(inner.stt_provider)?.clone();

    provider
        .transcribe(&audio_data, &language, dict_prompt.as_deref())
        .await
        .map_err(|e: stt::SttError| e.to_string())
}

/// Cleans up raw transcription text using the configured LLM provider.
///
/// Can be called independently of the recording pipeline (e.g. to re-clean
/// text with a different style).
///
/// `raw_text`: text to clean up.
/// `style`: cleanup aggressiveness.
/// `dictionary_terms`: optional comma-separated list of terms to preserve
///   verbatim. If `None`, the current app dictionary is used automatically.
#[tauri::command]
pub async fn cleanup_text(
    state: State<'_, AppState>,
    raw_text: String,
    style: CleanupStyle,
    dictionary_terms: Option<String>,
) -> Result<String, String> {
    let inner = state.inner();

    // Story 13-2 (E2 / G2a, rows D-H10 / D-M20 / D-M21): THE offline rule, the
    // same one the hotkey pipeline reads.
    if let Some(raw) = offline_passthrough(inner, &raw_text) {
        return Ok(raw);
    }

    // License gate: DeepSeek and Groq are free; all other LLM providers require a paid license.
    if !["deepseek", "groq"].contains(&active_llm_provider_id(inner).as_str()) {
        require_license!(state, LicensedFeature::AlternativeProviders);
    }

    let provider = crate::read_lock!(inner.cleanup_provider)?.clone();

    // Use caller-supplied terms if provided; otherwise fall back to app dictionary.
    let terms = match dictionary_terms {
        Some(t) => {
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        }
        None => {
            let guard = crate::lock!(inner.dictionary)?;
            let l = guard.terms_as_list();
            if l.is_empty() { None } else { Some(l) }
        }
    };

    let custom_prompt = match state.inner().config.lock() {
        Ok(g) => {
            let p = g.custom_prompt.clone();
            if p.is_empty() { None } else { Some(p) }
        }
        Err(_) => None,
    };

    let output_lang = match state.inner().config.lock() {
        Ok(c) => {
            let l = c.output_language.clone();
            if l.is_empty() { None } else { Some(l) }
        }
        Err(_) => None,
    };

    chunked_cleanup(
        provider.as_ref(),
        &raw_text,
        style,
        terms.as_deref(),
        custom_prompt.as_deref(),
        output_lang.as_deref(),
    )
    .await
    .map(|r| r.text)
    .map_err(|e: crate::llm::LlmError| e.to_string())
}

/// Cancels the active recording, discarding any captured audio.
///
/// Stops the cpal stream and emits `state=idle` so the floating bar
/// returns to its dormant state. Unlike `stop_recording`, no WAV bytes
/// are retained -- the audio is thrown away entirely.
#[tauri::command]
pub async fn cancel_recording(
    handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let inner = state.inner();

    if !inner.recorder.is_recording() {
        return Ok(()); // nothing to cancel
    }

    // Stop the recorder and discard the WAV bytes.
    let _ = inner.recorder.stop_recording();

    // Clear the start timestamp.
    *crate::lock!(inner.recording_start)? = None;

    // Emit idle state so all windows (main + floating bar) update.
    // Route through emit_pipeline_state so the native pill also transitions.
    crate::emit_pipeline_state(&handle, crate::hotkey::PipelineEvent::idle());

    Ok(())
}

/// Returns whether the recorder is currently active.
///
/// Useful for frontend state sync (e.g. showing a recording indicator).
#[tauri::command]
pub fn is_recording(state: State<'_, AppState>) -> bool {
    state.inner().recorder.is_recording()
}

/// Returns the names of all available audio input devices.
#[tauri::command]
pub fn list_audio_devices() -> Vec<String> {
    audio::list_input_devices()
}

// transcribe_live_preview removed (Story 10-2: NativePreview driven by flush_preview_delta in pipeline.rs).

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{make_state, temp_dir};

    /// Story 13-2 (D-M20): `cleanup_text` reads the SHARED rule now. These
    /// three keep their original inputs but call the one predicate the hotkey
    /// pipeline calls, which is the whole point of the row — before this story
    /// the two answers could differ for the identical config.
    #[test]
    fn test_offline_rule_local_stt_skips_cleanup() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            // Production code reads stt_provider (not the deprecated stt_priority list).
            cfg.stt_provider = "local".to_string();
        }
        let cfg = state.config.lock().unwrap();
        assert!(crate::pipeline::config_skips_cleanup(&cfg));
    }

    /// Cloud STT with the default cloud cleanup: the cleanup call still runs.
    #[test]
    fn test_offline_rule_cloud_stt_still_cleans() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_priority = vec!["groq".to_string(), "local".to_string()];
            cfg.groq_api_key = "test-key".to_string();
        }
        let cfg = state.config.lock().unwrap();
        assert!(!crate::pipeline::config_skips_cleanup(&cfg));
    }

    /// The deprecated `stt_priority` list is NOT an input of the offline rule:
    /// emptying it changes nothing, because the rule reads `stt_provider`. Named
    /// for what it asserts -- it used to be called `test_is_offline_mode_empty_priority`,
    /// after a function story 13-2 deleted, and after an input the rule never
    /// reads (review finding, 2026-09-21).
    #[test]
    fn test_offline_rule_ignores_the_deprecated_stt_priority_list() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_priority = vec![];
        }
        let cfg = state.config.lock().unwrap();
        assert!(!crate::pipeline::config_skips_cleanup(&cfg));
    }

    /// The in-app record button's own path, EXECUTED — not the predicate it
    /// happens to call.
    ///
    /// Added at review: every other test here drives
    /// `pipeline::config_skips_cleanup` directly, so nothing exercised
    /// `cleanup_text`'s branch and a second rule reintroduced inside it would
    /// have stayed green — which is drift row D-M20 verbatim, one level up.
    /// `cleanup_text` itself takes a `State<'_, AppState>` no test can build,
    /// so the branch lives in [`offline_passthrough`] and this drives that.
    ///
    /// The `local` branch returns before any provider is resolved, so this
    /// makes no network call and needs no key.
    #[test]
    fn spec_in_app_button_offline_returns_the_raw_text_unchanged() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_provider = "local".to_string();
            cfg.llm_provider = "deepseek".to_string();
            cfg.deepseek_api_key = "sk-would-have-been-called".to_string();
        }

        let raw = "also ähm ich glaube das passt so";
        assert_eq!(
            offline_passthrough(&state, raw),
            Some(raw.to_string()),
            "offline STT must yield the raw transcript, filler words and all \
             (Andi's own E2 check: the \"ähm\" stay)"
        );
    }

    /// The other side of the same branch: an ordinary cloud config must NOT
    /// short-circuit, or the fix would have disabled cleanup for everyone.
    #[test]
    fn spec_in_app_button_cloud_config_still_reaches_the_provider() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_provider = "groq".to_string();
            cfg.llm_provider = "deepseek".to_string();
        }
        assert_eq!(offline_passthrough(&state, "text"), None);
    }

    /// D-M20's discriminating case, and the reason this row exists: the
    /// in-app button and the hotkey must agree for the SAME config. Asserted
    /// against the one predicate both now read — if a second definition is
    /// ever reintroduced here, this stops being a tautology only because the
    /// matrix below covers a config where the two used to disagree
    /// (`stt=groq`, `llm=local` on a build without a local provider: the old
    /// `is_offline_mode` said "cleanup runs" and sent it to DeepSeek).
    #[test]
    fn spec_in_app_button_and_hotkey_share_one_offline_rule() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_provider = "groq".to_string();
            cfg.llm_provider = "local".to_string();
        }
        let skips = {
            let cfg = state.config.lock().unwrap();
            crate::pipeline::config_skips_cleanup(&cfg)
        };
        assert_eq!(
            skips,
            !crate::pipeline::local_cleanup_available(),
            "a selected-but-unavailable local cleanup must skip cleanup, not call DeepSeek"
        );
        // …and the in-app button's OWN branch must reach the same answer. Until
        // this was added the test named for that button never touched it: it
        // asserted the shared predicate twice over (review finding, 2026-09-21).
        assert_eq!(
            offline_passthrough(&state, "raw").is_some(),
            skips,
            "the in-app button must not re-decide what the shared rule already decided"
        );
    }

    /// `active_stt_provider_id` returns `"local"` when `stt_provider` is "local".
    #[test]
    fn test_active_stt_provider_id_local() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            // Production code reads stt_provider directly (not the deprecated stt_priority list).
            cfg.stt_provider = "local".to_string();
            cfg.groq_api_key = String::new();
        }
        assert_eq!(active_stt_provider_id(&state), "local");
    }

    /// `active_stt_provider_id` returns `"groq"` when a Groq key is present and
    /// "groq" comes before "local" in the priority list.
    #[test]
    fn test_active_stt_provider_id_groq_beats_local() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_priority = vec!["groq".to_string(), "local".to_string()];
            cfg.groq_api_key = "gsk-test".to_string();
        }
        assert_eq!(active_stt_provider_id(&state), "groq");
    }
}
