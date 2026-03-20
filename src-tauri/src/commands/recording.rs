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

/// Returns the ID of the active STT provider based on the priority list and available keys.
///
/// Walks `stt_priority` and returns the ID of the first provider with a non-empty key.
/// `"local"` is treated as always-available (no API key required).
/// Returns `"groq"` as fallback (matching `resolve_stt_provider` behaviour).
fn active_stt_provider_id(state: &AppState) -> String {
    let cfg = match state.config.lock() {
        Ok(g) => g,
        Err(_) => return "groq".to_string(),
    };
    for id in &cfg.stt_priority {
        match id.as_str() {
            "groq" if !cfg.groq_api_key.is_empty() => return "groq".to_string(),
            "openai" if !cfg.openai_api_key.is_empty() => return "openai".to_string(),
            // "local" requires no API key -- always considered available.
            "local" => return "local".to_string(),
            _ => continue,
        }
    }
    "groq".to_string()
}

/// Returns `true` if the user is in offline mode, i.e. the first entry in
/// `stt_priority` is `"local"`.
fn is_offline_mode(state: &AppState) -> bool {
    state
        .config
        .lock()
        .ok()
        .and_then(|c| c.stt_priority.first().cloned())
        .map(|id| id == "local")
        .unwrap_or(false)
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
        .transcribe(wav_bytes, &language, dict_prompt.as_deref())
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
        .transcribe(audio_data, &language, dict_prompt.as_deref())
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

    // Offline mode: if stt_priority[0] == "local", skip the LLM call entirely
    // and return the raw transcription unchanged.
    if is_offline_mode(inner) {
        log::info!("[cleanup] Offline mode: returning raw text without cleanup");
        return Ok(raw_text);
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
    use tauri::Emitter;
    let _ = handle.emit(
        crate::hotkey::EVENT_STATE_CHANGED,
        crate::hotkey::PipelineEvent::idle(),
    );

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

/// Takes a snapshot of the current audio buffer and transcribes it for live preview.
///
/// Returns the partial transcription text, or empty string if nothing recorded yet.
#[tauri::command]
pub async fn transcribe_live_preview(state: State<'_, AppState>) -> Result<String, String> {
    let inner = state.inner();

    // Only preview while actually recording
    if !inner.recorder.is_recording() {
        return Ok(String::new());
    }

    let wav_bytes = match inner.recorder.snapshot_wav() {
        Some(b) if b.len() > 44 => b, // 44 = WAV header only (no audio data)
        _ => return Ok(String::new()),
    };

    let (language, stt_provider, dict_prompt) = {
        let cfg = crate::lock!(inner.config)?;
        let lang = cfg.language.clone();
        let stt = inner
            .stt_provider
            .read()
            .map_err(|e| format!("Lock poisoned: {e}"))?
            .clone();
        let dict_terms = match inner.dictionary.lock() {
            Ok(g) => {
                let p = g.terms_as_prompt();
                if p.is_empty() { None } else { Some(p) }
            }
            Err(_) => None,
        };
        let prompt = build_stt_prompt(dict_terms.as_deref(), &lang);
        (lang, stt, prompt)
    };

    match stt_provider
        .transcribe(wav_bytes, &language, dict_prompt.as_deref())
        .await
    {
        Ok(text) => Ok(text),
        Err(e) => {
            log::warn!("[live-preview] transcription failed: {e}");
            Ok(String::new()) // Don't error out, just return empty
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::test_helpers::{make_state, temp_dir};

    /// `is_offline_mode` returns `true` when the first STT priority is "local".
    #[test]
    fn test_is_offline_mode_local_first() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_priority = vec!["local".to_string(), "groq".to_string()];
        }
        assert!(is_offline_mode(&state));
    }

    /// `is_offline_mode` returns `false` when "local" is not the first entry.
    #[test]
    fn test_is_offline_mode_cloud_first() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_priority = vec!["groq".to_string(), "local".to_string()];
            cfg.groq_api_key = "test-key".to_string();
        }
        assert!(!is_offline_mode(&state));
    }

    /// `is_offline_mode` returns `false` when `stt_priority` is empty.
    #[test]
    fn test_is_offline_mode_empty_priority() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_priority = vec![];
        }
        assert!(!is_offline_mode(&state));
    }

    /// `active_stt_provider_id` returns `"local"` when "local" appears in the
    /// priority list and no cloud key is configured before it.
    #[test]
    fn test_active_stt_provider_id_local() {
        let dir = temp_dir();
        let state = make_state(&dir);
        {
            let mut cfg = state.config.lock().unwrap();
            cfg.stt_priority = vec!["local".to_string()];
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
