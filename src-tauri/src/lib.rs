//! Dikta -- Tauri backend entry point.
//!
//! Wires together the audio, STT and LLM modules and exposes them to the
//! React frontend via Tauri commands.

mod audio;
mod llm;
mod stt;

use std::sync::Arc;

use audio::AudioRecorder;
use llm::{CleanupProvider, CleanupStyle, DeepSeekCleanup};
use serde::{Deserialize, Serialize};
use stt::{GroqWhisper, SttProvider};
use tauri::State;

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// Result returned to the frontend after a full dictation cycle.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranscriptionResult {
    /// Raw text from the STT engine.
    pub raw_text: String,
    /// Cleaned-up text (same as `raw_text` if cleanup was skipped).
    pub cleaned_text: String,
    /// Duration of the recording in milliseconds.
    pub duration_ms: u64,
}

/// Shared application state managed by Tauri.
///
/// Tauri requires `State<T>: Send + Sync + 'static`.
/// All dynamic providers are boxed behind `Arc<dyn Trait>` so they are
/// heap-allocated and their concrete types do not bleed into this struct.
pub struct AppState {
    recorder: Arc<AudioRecorder>,
    stt_provider: Arc<dyn SttProvider>,
    cleanup_provider: Arc<dyn CleanupProvider>,
    /// Timestamp set by `start_recording`, cleared by `stop_recording`.
    recording_start: std::sync::Mutex<Option<std::time::Instant>>,
}

// AudioRecorder carries an `unsafe impl Send + Sync` (see audio/mod.rs).
// The trait objects are `Arc<dyn Trait + Send + Sync>`, so AppState is
// transitively Send + Sync.
unsafe impl Send for AppState {}
unsafe impl Sync for AppState {}

impl AppState {
    fn new(groq_api_key: String, deepseek_api_key: String) -> Self {
        AppState {
            recorder: Arc::new(AudioRecorder::new()),
            stt_provider: Arc::new(GroqWhisper::new(groq_api_key)),
            cleanup_provider: Arc::new(DeepSeekCleanup::new(deepseek_api_key)),
            recording_start: std::sync::Mutex::new(None),
        }
    }
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

/// Opens the default microphone and starts capturing audio.
///
/// Returns an error string if recording is already in progress or no
/// microphone is available.
#[tauri::command]
async fn start_recording(state: State<'_, AppState>) -> Result<(), String> {
    state
        .inner()
        .recorder
        .start_recording()
        .map_err(|e: audio::AudioError| e.to_string())?;

    let mut start = state.inner().recording_start.lock().unwrap();
    *start = Some(std::time::Instant::now());

    Ok(())
}

/// Stops the active recording and runs the full pipeline:
/// 1. Encode captured audio as 16kHz mono WAV.
/// 2. Send WAV to the STT provider (Groq Whisper).
/// 3. Send raw transcript to the LLM cleanup provider (DeepSeek).
///
/// `language`: ISO-639-1 code (e.g. `"de"`, `"en"`). Empty string = auto-detect.
/// `style`: cleanup aggressiveness.
/// `skip_cleanup`: if `true`, skip the LLM step and return raw STT text.
#[tauri::command]
async fn stop_recording(
    state: State<'_, AppState>,
    language: String,
    style: CleanupStyle,
    skip_cleanup: bool,
) -> Result<TranscriptionResult, String> {
    let inner = state.inner();

    // Measure recording duration before stopping.
    let duration_ms = {
        let start_guard = inner.recording_start.lock().unwrap();
        start_guard
            .map(|t: std::time::Instant| t.elapsed().as_millis() as u64)
            .unwrap_or(0)
    };

    // Stop recording and get WAV bytes.
    let wav_bytes = inner
        .recorder
        .stop_recording()
        .map_err(|e: audio::AudioError| e.to_string())?;

    // Clear the start timestamp.
    *inner.recording_start.lock().unwrap() = None;

    // STT -- send WAV to Groq Whisper.
    let raw_text = inner
        .stt_provider
        .transcribe(wav_bytes, &language)
        .await
        .map_err(|e: stt::SttError| e.to_string())?;

    // Optional LLM cleanup.
    let cleaned_text = if skip_cleanup {
        raw_text.clone()
    } else {
        inner
            .cleanup_provider
            .cleanup(&raw_text, style, None)
            .await
            .map_err(|e: llm::LlmError| e.to_string())?
    };

    Ok(TranscriptionResult {
        raw_text,
        cleaned_text,
        duration_ms,
    })
}

/// Transcribes pre-recorded audio bytes without going through the recorder.
///
/// Useful for testing the STT pipeline directly or for a future
/// file-import feature.
#[tauri::command]
async fn transcribe_audio(
    state: State<'_, AppState>,
    audio_bytes: Vec<u8>,
    language: String,
) -> Result<String, String> {
    state
        .inner()
        .stt_provider
        .transcribe(audio_bytes, &language)
        .await
        .map_err(|e: stt::SttError| e.to_string())
}

/// Cleans up raw text using the configured LLM provider.
///
/// Can be called independently of the recording pipeline (e.g. to re-clean
/// text with a different style after the fact).
#[tauri::command]
async fn cleanup_text(
    state: State<'_, AppState>,
    raw_text: String,
    style: CleanupStyle,
    dictionary_terms: Option<String>,
) -> Result<String, String> {
    state
        .inner()
        .cleanup_provider
        .cleanup(&raw_text, style, dictionary_terms.as_deref())
        .await
        .map_err(|e: llm::LlmError| e.to_string())
}

/// Returns whether the recorder is currently active.
///
/// Useful for frontend state sync (e.g. showing a recording indicator).
#[tauri::command]
fn is_recording(state: State<'_, AppState>) -> bool {
    state.inner().recorder.is_recording()
}

// ---------------------------------------------------------------------------
// Tauri entry point
// ---------------------------------------------------------------------------

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // API keys come from environment variables.
    // In production the settings UI will store them in the system keystore;
    // for now we fall back to the process environment / .env file.
    let groq_api_key = std::env::var("GROQ_API_KEY").unwrap_or_default();
    let deepseek_api_key = std::env::var("DEEPSEEK_API_KEY").unwrap_or_default();

    let app_state = AppState::new(groq_api_key, deepseek_api_key);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            start_recording,
            stop_recording,
            transcribe_audio,
            cleanup_text,
            is_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
