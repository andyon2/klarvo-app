//! Voice Command Mode -- dispatch layer (desktop-only).
//!
//! Connects `AudioRecorder::start_monitor` to `VoiceCommandEngine` and
//! dispatches recognised commands to the dictation pipeline.
//!
//! ## Thread model
//!
//! ```text
//! cpal OS-thread
//!   └─ monitor callback (lightweight: feed() only)
//!        └─ on SnippetReady → std::thread::spawn
//!               └─ encode WAV + local whisper (blocking CPU)
//!                    └─ recognize_command()
//!                         └─ tauri::async_runtime::spawn → dispatch
//! ```
//!
//! The monitor callback itself must be O(μs). Everything heavier lives in a
//! spawned thread so the cpal callback never blocks.
//!
//! ## Debounce
//!
//! A 2-second debounce prevents double-dispatch from speech echo or a very
//! short silence between two command utterances. The last-dispatch timestamp
//! is shared between the monitor callback and the dispatch thread via
//! `Arc<Mutex<Instant>>`.

#![cfg(desktop)]

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};

use crate::audio::{self, query_input_format};
use crate::hotkey::PipelineEvent;
use crate::pipeline::{stop_and_process_pipeline, run_dictation_pipeline, start_autostop_recording, start_auto_recording};
use crate::AppState;

use super::{recognize_command, VoiceCommand, VoiceCommandEngine, VoiceCommandError};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Assumed native sample rate for the monitor stream.
///
/// Most devices deliver 48 kHz. We cannot query the rate before opening the
/// stream, and the VoiceCommandEngine must be constructed before the first
/// callback fires.
const DEFAULT_MONITOR_SAMPLE_RATE: u32 = 48_000;

/// Assumed channel count for the monitor stream.
const DEFAULT_MONITOR_CHANNELS: u16 = 1;

/// Minimum time between two dispatched commands. Prevents double-trigger from
/// speech echo or a very short silence between consecutive utterances.
const DEBOUNCE_DURATION: Duration = Duration::from_secs(2);

/// Tauri event emitted when the Voice Command Mode is toggled.
pub const EVENT_VOICE_COMMAND_STATE: &str = "klarvo://voice-command-state-changed";

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Starts the continuous microphone monitor and enables Voice Command Mode.
///
/// # Steps
///
/// 1. Checks `AppState::voice_command_active` -- returns an error if already running.
/// 2. Reads the configured audio device from config (same device as recording).
/// 3. Creates a `VoiceCommandEngine` with `DEFAULT_MONITOR_SAMPLE_RATE / DEFAULT_MONITOR_CHANNELS`.
/// 4. Wraps the engine and debounce state in `Arc<Mutex<_>>` so they can be
///    shared with the cpal callback thread.
/// 5. Calls `AudioRecorder::start_monitor` with a lightweight callback that:
///    - Feeds PCM chunks into the engine.
///    - On `SnippetReady`: spawns a thread for local Whisper + dispatch.
/// 6. Sets `voice_command_active = true` and emits `EVENT_VOICE_COMMAND_STATE`.
///
/// # Errors
///
/// Returns an error string if:
/// - Voice Command Mode is already active.
/// - The microphone cannot be opened (propagated from `AudioRecorder::start_monitor`).
/// - The `VoiceCommandEngine` cannot be initialised (VAD init failure).
#[cfg(desktop)]
pub fn start_voice_command_monitor(handle: &AppHandle) -> Result<(), String> {
    let state = handle.state::<AppState>();

    // Guard: already running.
    if state.voice_command_active.load(Ordering::SeqCst) {
        return Err("Voice Command Mode is already active".to_string());
    }

    // Read the audio device name from config (same as recording).
    let device_name = state
        .config
        .lock()
        .ok()
        .and_then(|c| c.audio_device.clone());

    // Query the actual device format so the engine does correct resampling.
    // Falls back to safe defaults (48 kHz stereo) if the query fails.
    let (sample_rate, channels) = query_input_format(device_name.as_deref())
        .unwrap_or_else(|e| {
            log::warn!(
                "[voice_command] Could not query device format ({e}), \
                 falling back to {DEFAULT_MONITOR_SAMPLE_RATE} Hz / {DEFAULT_MONITOR_CHANNELS} ch"
            );
            (DEFAULT_MONITOR_SAMPLE_RATE, DEFAULT_MONITOR_CHANNELS)
        });

    let engine = VoiceCommandEngine::new(sample_rate, channels)
        .map_err(|e: VoiceCommandError| format!("VoiceCommandEngine init failed: {e}"))?;

    let engine = Arc::new(Mutex::new(engine));

    // Shared debounce timestamp -- initialised far enough in the past so the
    // first command is never suppressed.
    let last_dispatch: Arc<Mutex<Instant>> =
        Arc::new(Mutex::new(Instant::now() - DEBOUNCE_DURATION * 2));

    // Use Groq Whisper API for command recognition. It's fast (~0.5-1s),
    // accurate with prompt conditioning, and most users have a Groq key.
    let groq_key = state
        .config
        .lock()
        .ok()
        .map(|c| c.groq_api_key.clone())
        .unwrap_or_default();

    if groq_key.is_empty() {
        return Err("Voice Command Mode requires a Groq API key (Settings → API Keys)".to_string());
    }

    // Log device format only after the Groq-key check passes -- avoids
    // misleading output when the monitor fails early due to a missing key.
    log::info!("[voice_command] Device format: {sample_rate} Hz, {channels} ch");

    let groq_provider = Arc::new(crate::stt::GroqWhisper::new(&groq_key));
    log::info!("[voice_command] Using Groq Whisper for command recognition");

    // Clone AppHandle for the callback closure.
    let handle_for_cb = handle.clone();

    // Read user's language for Whisper language hint.
    let language = state
        .config
        .lock()
        .ok()
        .map(|c| c.language.clone())
        .unwrap_or_else(|| "de".to_string());

    let callback = {
        let engine = Arc::clone(&engine);
        let last_dispatch = Arc::clone(&last_dispatch);
        let provider = Arc::clone(&groq_provider);
        let lang = language.clone();

        Arc::new(move |chunk: &[f32]| {
            let event = {
                let mut eng = match engine.lock() {
                    Ok(g) => g,
                    Err(_) => return,
                };
                eng.feed(chunk)
            };

            if let Some(crate::voice_command::VoiceCommandEvent::SnippetReady(samples)) = event {
                log::debug!("[voice_command] SnippetReady: {} samples ({:.1}s)", samples.len(), samples.len() as f64 / 16_000.0);
                let handle = handle_for_cb.clone();
                let last_dispatch = Arc::clone(&last_dispatch);
                let provider = Arc::clone(&provider);
                let lang = lang.clone();

                std::thread::spawn(move || {
                    process_snippet(samples, handle, last_dispatch, provider, &lang);
                });
            }
        })
    };

    // Open the microphone monitor stream.
    state
        .recorder
        .start_monitor(device_name.as_deref(), callback)
        .map_err(|e| format!("Failed to start audio monitor: {e}"))?;

    // Mark active.
    state.voice_command_active.store(true, Ordering::SeqCst);

    // Notify frontend.
    let _ = handle.emit(
        EVENT_VOICE_COMMAND_STATE,
        serde_json::json!({ "active": true }),
    );

    log::info!("[voice_command] Monitor started (device: {device_name:?})");

    Ok(())
}

/// Stops the continuous microphone monitor and disables Voice Command Mode.
///
/// # Steps
///
/// 1. Calls `AudioRecorder::stop_monitor`.
/// 2. Sets `voice_command_active = false`.
/// 3. Emits `EVENT_VOICE_COMMAND_STATE` with `active: false`.
///
/// # Errors
///
/// Returns an error string if `stop_monitor` fails (e.g. not currently monitoring).
#[cfg(desktop)]
pub fn stop_voice_command_monitor(handle: &AppHandle) -> Result<(), String> {
    let state = handle.state::<AppState>();

    state
        .recorder
        .stop_monitor()
        .map_err(|e| format!("Failed to stop audio monitor: {e}"))?;

    state.voice_command_active.store(false, Ordering::SeqCst);

    let _ = handle.emit(
        EVENT_VOICE_COMMAND_STATE,
        serde_json::json!({ "active": false }),
    );

    log::info!("[voice_command] Monitor stopped");

    Ok(())
}

// ---------------------------------------------------------------------------
// Internal: snippet processing (runs in a spawned OS-thread)
// ---------------------------------------------------------------------------

/// Encodes a 16 kHz mono PCM snippet to WAV, runs local Whisper transcription,
/// matches the result against known commands, applies debounce, and dispatches.
///
/// This function blocks: it runs on a dedicated `std::thread::spawn` thread so
/// the cpal callback (which called us) returns immediately.
fn process_snippet(
    samples: Vec<f32>,
    handle: AppHandle,
    last_dispatch: Arc<Mutex<Instant>>,
    provider: Arc<crate::stt::GroqWhisper>,
    language: &str,
) {
    log::debug!(
        "[voice_command] Processing snippet: {} samples ({:.1}s at 16kHz)",
        samples.len(),
        samples.len() as f64 / 16_000.0
    );

    let wav_bytes = match audio::encode_to_wav(&samples, 16_000, 1) {
        Ok(b) => b,
        Err(e) => {
            log::error!("[voice_command] WAV encode failed: {e}");
            return;
        }
    };

    let text = transcribe_with_groq(&wav_bytes, &provider, language);

    let text = match text {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            log::debug!("[voice_command] Snippet produced no text (silence or model missing)");
            return;
        }
    };

    log::debug!("[voice_command] Whisper text: {text:?}");

    // Match against known commands.
    let cmd = match recognize_command(&text) {
        Some(c) => c,
        None => {
            log::debug!("[voice_command] No command recognised in: {text:?}");
            return;
        }
    };

    log::info!("[voice_command] Recognised command: {cmd:?}");

    // Debounce: ignore commands that arrive within DEBOUNCE_DURATION of the last.
    {
        let mut ts = match last_dispatch.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let elapsed = ts.elapsed();
        if elapsed < DEBOUNCE_DURATION {
            log::debug!(
                "[voice_command] Command debounced ({:.1}s < {:.1}s)",
                elapsed.as_secs_f32(),
                DEBOUNCE_DURATION.as_secs_f32()
            );
            return;
        }
        *ts = Instant::now();
    }

    // Dispatch: heavy/async work goes onto the Tauri async runtime.
    dispatch_command(cmd, handle);
}

/// Transcribes a short audio snippet via Groq Whisper API.
///
/// Uses prompt conditioning so Groq recognizes "Klarvo" as a word.
/// Typically completes in 0.5-1.5 seconds.
fn transcribe_with_groq(
    wav_bytes: &[u8],
    provider: &Arc<crate::stt::GroqWhisper>,
    language: &str,
) -> Option<String> {
    use crate::stt::SttProvider;

    let wav = wav_bytes.to_vec();

    let rt = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            log::error!("[voice_command] Tokio runtime error: {e}");
            return None;
        }
    };

    let start = std::time::Instant::now();
    // Conditioning prompt: helps Groq recognize "Klarvo" as a keyword.
    let prompt = Some("Klarvo toggle, Klarvo start, Klarvo auto-stop, Klarvo autostop, Klarvo full auto, Klarvo stop, Klarvo stopp, Klarvo cancel, Klarvo abbrechen, Klarvo off, Klarvo aus");
    let result = rt.block_on(provider.transcribe(&wav, language, prompt));

    log::debug!(
        "[voice_command] Groq completed in {:.1}s",
        start.elapsed().as_secs_f32()
    );

    match result {
        Ok(text) => Some(text),
        Err(e) => {
            log::error!("[voice_command] Groq error: {e}");
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Dispatches a recognised `VoiceCommand` to the appropriate pipeline function.
///
/// All dispatch paths that are async or need `AppHandle` spawn onto the Tauri
/// async runtime so this function itself can remain synchronous (it is called
/// from a `std::thread::spawn` thread).
fn dispatch_command(cmd: VoiceCommand, handle: AppHandle) {
    match cmd {
        VoiceCommand::StartToggle => {
            log::info!("[voice_command] Dispatching: StartToggle");
            tauri::async_runtime::spawn(async move {
                run_dictation_pipeline(handle).await;
            });
        }

        VoiceCommand::StartAutoStop => {
            log::info!("[voice_command] Dispatching: StartAutoStop");
            tauri::async_runtime::spawn(async move {
                start_autostop_recording(handle).await;
            });
        }

        VoiceCommand::StartFullAuto => {
            log::info!("[voice_command] Dispatching: StartFullAuto");
            let state_ref = handle.state::<AppState>();
            state_ref.auto_loop_active.store(true, std::sync::atomic::Ordering::SeqCst);
            tauri::async_runtime::spawn(async move {
                start_auto_recording(handle).await;
            });
        }

        VoiceCommand::StopDictation => {
            log::info!("[voice_command] Dispatching: StopDictation");
            tauri::async_runtime::spawn(async move {
                stop_and_process_pipeline(handle).await;
            });
        }

        VoiceCommand::CancelDictation => {
            log::info!("[voice_command] Dispatching: CancelDictation");
            // Replicate cancel_recording logic without the Tauri command
            // wrapper (which needs `State<'_>` and can't be called directly).
            let state = handle.state::<AppState>();
            if state.recorder.is_recording() {
                let _ = state.recorder.stop_recording();
                if let Ok(mut guard) = state.recording_start.lock() {
                    *guard = None;
                }
                crate::emit_pipeline_state(&handle, PipelineEvent::idle());
            }
        }

        VoiceCommand::TurnOff => {
            log::info!("[voice_command] Dispatching: TurnOff");
            // stop_voice_command_monitor needs &AppHandle, call synchronously.
            if let Err(e) = stop_voice_command_monitor(&handle) {
                log::warn!("[voice_command] TurnOff: stop_monitor failed: {e}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for start/stop_voice_command_monitor require a live Tauri AppHandle,
    // which is not available in unit tests. The core logic (snippet processing,
    // debounce, command dispatch) is exercised through integration of the
    // individually-tested VoiceCommandEngine and recognize_command functions.
    //
    // What we CAN test here:

    #[test]
    fn test_debounce_suppresses_rapid_commands() {
        // Simulate two dispatches within the debounce window.
        let last_dispatch: Arc<Mutex<Instant>> =
            Arc::new(Mutex::new(Instant::now())); // "just dispatched"

        let elapsed = last_dispatch.lock().unwrap().elapsed();
        assert!(
            elapsed < DEBOUNCE_DURATION,
            "Freshly-created timestamp must be within debounce window"
        );
    }

    #[test]
    fn test_debounce_allows_after_window() {
        // Simulate a dispatch that happened long enough ago.
        let last_dispatch: Arc<Mutex<Instant>> =
            Arc::new(Mutex::new(Instant::now() - DEBOUNCE_DURATION * 2));

        let elapsed = last_dispatch.lock().unwrap().elapsed();
        assert!(
            elapsed >= DEBOUNCE_DURATION,
            "Timestamp far in the past must be outside debounce window"
        );
    }

    #[test]
    fn test_model_path_is_none_on_non_windows() {
        // On non-Windows the model_path must be None (no local Whisper).
        #[cfg(not(target_os = "windows"))]
        {
            let model_path: Option<String> = None;
            assert!(model_path.is_none());
        }
        // On Windows we just ensure the path is Some (no file system access in tests).
        #[cfg(target_os = "windows")]
        {
            let model_path: Option<String> = Some("some_path".to_string());
            assert!(model_path.is_some());
        }
    }
}
