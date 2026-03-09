//! Dictation pipeline: audio stop → STT → LLM cleanup → paste.
//!
//! These functions are called by the global hotkey handler and are not
//! directly exposed as Tauri commands. They operate on [`AppState`] via
//! an [`AppHandle`] so they can emit state-change events to the frontend.

use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager};

use crate::audio;
use crate::config::{self, AppConfig, HotkeyMode};
use crate::history;
use crate::hotkey::{PipelineEvent, EVENT_STATE_CHANGED};
use crate::llm::{self, chunked_cleanup, CleanupProvider, CleanupStyle};
use crate::paste::{capture_foreground_window, capture_foreground_window_title, create_paste_handler};
use crate::stt::{self, SttProvider};
use crate::sync;
use crate::{AppState, friendly_error};

#[cfg(desktop)]
use crate::setup_audio_level_emitter;

// ---------------------------------------------------------------------------
// Provider resolution from priority lists
// ---------------------------------------------------------------------------

/// Selects the STT provider to use based on the priority list and available keys.
///
/// Walks `cfg.stt_priority` left-to-right and returns the first provider for
/// which a non-empty API key is configured.  Falls back to a no-key Groq
/// instance if nothing matches (will fail at call-time with an auth error).
pub fn resolve_stt_provider(cfg: &AppConfig) -> Arc<dyn SttProvider> {
    for id in &cfg.stt_priority {
        match id.as_str() {
            "groq" if !cfg.groq_api_key.is_empty() => {
                return Arc::new(
                    stt::GroqWhisper::new(&cfg.groq_api_key).with_model(cfg.stt_model.clone()),
                );
            }
            "openai" if !cfg.openai_api_key.is_empty() => {
                return Arc::new(stt::OpenAiWhisper::new(&cfg.openai_api_key));
            }
            _ => continue,
        }
    }
    Arc::new(stt::GroqWhisper::new(&cfg.groq_api_key).with_model(cfg.stt_model.clone()))
}

/// Selects the LLM cleanup provider based on the priority list and available keys.
///
/// Same walk-and-pick logic as [`resolve_stt_provider`].
pub fn resolve_cleanup_provider(cfg: &AppConfig) -> Arc<dyn CleanupProvider> {
    for id in &cfg.llm_priority {
        match id.as_str() {
            "deepseek" if !cfg.deepseek_api_key.is_empty() => {
                return Arc::new(llm::DeepSeekCleanup::new(&cfg.deepseek_api_key));
            }
            "openai" if !cfg.openai_api_key.is_empty() => {
                return Arc::new(llm::OpenAiCleanup::new(&cfg.openai_api_key));
            }
            "anthropic" if !cfg.anthropic_api_key.is_empty() => {
                return Arc::new(llm::AnthropicCleanup::new(&cfg.anthropic_api_key));
            }
            "groq" if !cfg.groq_api_key.is_empty() => {
                return Arc::new(llm::GroqCleanup::new(&cfg.groq_api_key));
            }
            _ => continue,
        }
    }
    Arc::new(llm::DeepSeekCleanup::new(&cfg.deepseek_api_key))
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
        let _ = handle.emit(
            EVENT_STATE_CHANGED,
            PipelineEvent::error(format!("Failed to start recording: {e}")),
        );
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
    let (language, stt_provider, cleanup_provider, dict_prompt) = {
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

        (cfg.language.clone(), stt_prov, cleanup_prov, prompt)
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
    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::cleaning());

    let cleanup_result = if let Some(ref sel_text) = selected_text {
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
    let prev_hwnd = state.prev_foreground_hwnd.lock().ok().and_then(|g| *g);
    let paste_handler = create_paste_handler(prev_hwnd);
    if let Err(e) = paste_handler.paste(&cleaned_text) {
        log::warn!("[pipeline] paste failed: {e}. Text is still available.");
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

    let _ = handle.emit(EVENT_STATE_CHANGED, PipelineEvent::done(cleaned_text, raw_text));
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

/// Registers the global shortcut with mode-aware handlers.
///
/// Unregisters all existing shortcuts first so this can be called to
/// re-register after a settings change.
///
/// - `Toggle`: Pressed fires [`run_dictation_pipeline`] (start or stop+process).
/// - `Hold`: Pressed fires [`start_recording_only`]; Released fires [`stop_and_process_pipeline`].
#[cfg(desktop)]
pub fn register_hotkey(
    handle: &AppHandle,
    shortcut: tauri_plugin_global_shortcut::Shortcut,
    mode: HotkeyMode,
) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

    println!("[hotkey] Registering shortcut: {shortcut:?} mode={mode:?}");

    handle
        .global_shortcut()
        .unregister_all()
        .map_err(|e| format!("Failed to unregister shortcuts: {e}"))?;

    // --- Dictation hotkey ---
    let handle_clone = handle.clone();
    handle
        .global_shortcut()
        .on_shortcut(shortcut, move |_app, _shortcut, event| {
            println!("[hotkey] Event: {event:?}");

            let h = handle_clone.clone();
            println!("[hotkey] mode={mode:?} state={:?}", event.state);
            match (mode, event.state) {
                (HotkeyMode::Toggle, ShortcutState::Pressed) => {
                    tauri::async_runtime::spawn(async move {
                        run_dictation_pipeline(h).await;
                    });
                }
                (HotkeyMode::Hold, ShortcutState::Pressed) => {
                    tauri::async_runtime::spawn(async move {
                        start_recording_only(h).await;
                    });
                }
                (HotkeyMode::Hold, ShortcutState::Released) => {
                    tauri::async_runtime::spawn(async move {
                        stop_and_process_pipeline(h).await;
                    });
                }
                _ => {}
            }
        })
        .map_err(|e| format!("Failed to register shortcut: {e}"))?;

    // --- Command Mode hotkey ---
    let cmd_shortcut_str = handle
        .state::<AppState>()
        .config
        .lock()
        .ok()
        .map(|c| c.command_hotkey.clone())
        .unwrap_or_else(|| "ctrl+shift+e".to_string());

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
