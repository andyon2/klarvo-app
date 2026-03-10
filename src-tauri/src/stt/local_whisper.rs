//! Local offline STT provider using whisper.cpp via the `whisper-rs` crate.
//!
//! ## Design
//!
//! `LocalWhisperProvider` implements the `SttProvider` trait and is available
//! on Windows only. On Android we use cloud STT exclusively because the GGML
//! build for aarch64 requires extra cmake configuration and is not needed for
//! the MVP. On macOS/Linux the feature could be enabled later; for now only
//! the Windows production build path matters.
//!
//! The `WhisperContext` (model weights) is expensive to load (~100-200ms +
//! RAM allocation). We therefore keep a single cached instance wrapped in
//! `Arc<Mutex<Option<WhisperContext>>>` and reload it only when the model
//! path changes. The `WhisperState` (inference state) is created fresh per
//! request as required by the whisper-rs API.
//!
//! ## Audio format
//!
//! whisper.cpp requires 32-bit float, 16 kHz, mono audio. The provider
//! accepts raw WAV bytes (as produced by the `audio` module) and converts
//! them internally using `hound` for WAV decoding.

#![cfg(target_os = "windows")]

use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use super::{SttError, SttProvider};

// ---------------------------------------------------------------------------
// LocalWhisperError -- whisper.cpp-specific errors
// ---------------------------------------------------------------------------

/// Errors produced by the local whisper.cpp backend.
///
/// These are wrapped into `SttError::LocalWhisper` before being returned
/// through the `SttProvider` trait.
#[derive(Debug, thiserror::Error)]
pub enum LocalWhisperError {
    #[error("Model not found at path: {0}")]
    ModelNotFound(String),

    #[error("Failed to load whisper model: {0}")]
    ModelLoad(String),

    #[error("Failed to create whisper inference state: {0}")]
    StateCreate(String),

    #[error("WAV decoding failed: {0}")]
    WavDecode(String),

    #[error("Audio resampling required: input is {0} Hz but whisper needs 16000 Hz")]
    WrongSampleRate(u32),

    #[error("Failed to run whisper transcription: {0}")]
    Transcription(String),

    #[error("Failed to read transcription segment: {0}")]
    SegmentRead(String),

    #[error("Context lock poisoned")]
    LockPoisoned,
}

// ---------------------------------------------------------------------------
// LocalWhisperProvider
// ---------------------------------------------------------------------------

/// Offline STT provider backed by whisper.cpp.
///
/// Create once per application; the loaded `WhisperContext` is reused across
/// all `transcribe` calls. Thread-safe via `Arc<Mutex<_>>`.
pub struct LocalWhisperProvider {
    /// Full filesystem path to the GGML model file (e.g. `.../models/ggml-base.bin`).
    model_path: String,
    /// Cached whisper context. `None` = not yet loaded.
    ctx: Arc<Mutex<Option<whisper_rs::WhisperContext>>>,
}

impl LocalWhisperProvider {
    /// Creates a new `LocalWhisperProvider`.
    ///
    /// The model is loaded lazily on the first `transcribe` call. No I/O is
    /// performed here.
    ///
    /// `model_path` must point to a GGML-format model file, e.g.
    /// `{app_data_dir}/models/ggml-base.bin`.
    pub fn new(model_path: impl Into<String>) -> Self {
        LocalWhisperProvider {
            model_path: model_path.into(),
            ctx: Arc::new(Mutex::new(None)),
        }
    }

    /// Returns the configured model path (used in tests).
    #[cfg(test)]
    pub fn model_path(&self) -> &str {
        &self.model_path
    }

    /// Ensures the `WhisperContext` is loaded and returns a clone of the `Arc`.
    ///
    /// If the context is already loaded this is a cheap mutex lock + clone.
    /// If not, the model is loaded from disk (~100-200 ms first time).
    fn ensure_context(&self) -> Result<Arc<Mutex<Option<whisper_rs::WhisperContext>>>, SttError> {
        let mut guard = self.ctx.lock().map_err(|_| {
            SttError::LocalWhisper(LocalWhisperError::LockPoisoned.to_string())
        })?;

        if guard.is_none() {
            // Verify the model file exists before attempting to load.
            if !std::path::Path::new(&self.model_path).exists() {
                return Err(SttError::LocalWhisper(
                    LocalWhisperError::ModelNotFound(self.model_path.clone()).to_string(),
                ));
            }

            log::info!("[local_whisper] Loading model from: {}", self.model_path);

            let ctx = whisper_rs::WhisperContext::new_with_params(
                &self.model_path,
                whisper_rs::WhisperContextParameters::default(),
            )
            .map_err(|e| {
                SttError::LocalWhisper(
                    LocalWhisperError::ModelLoad(e.to_string()).to_string(),
                )
            })?;

            *guard = Some(ctx);
            log::info!("[local_whisper] Model loaded successfully");
        }

        Ok(Arc::clone(&self.ctx))
    }
}

// ---------------------------------------------------------------------------
// WAV -> PCM f32 conversion
// ---------------------------------------------------------------------------

/// Decodes a WAV byte buffer and returns 16 kHz mono f32 PCM samples.
///
/// Accepts WAV files with either i16 or f32 samples and any channel count.
/// Multi-channel audio is downmixed to mono by averaging channels.
///
/// # Errors
/// - Returns `LocalWhisperError::WavDecode` if the bytes cannot be decoded.
/// - Returns `LocalWhisperError::WrongSampleRate` if the sample rate is not 16 000 Hz.
fn wav_bytes_to_pcm_f32(wav_bytes: &[u8]) -> Result<Vec<f32>, LocalWhisperError> {
    let cursor = std::io::Cursor::new(wav_bytes);
    let mut reader = hound::WavReader::new(cursor)
        .map_err(|e| LocalWhisperError::WavDecode(e.to_string()))?;

    let spec = reader.spec();

    if spec.sample_rate != 16_000 {
        return Err(LocalWhisperError::WrongSampleRate(spec.sample_rate));
    }

    let channels = spec.channels as usize;

    // Collect all samples as f32 (interleaved channels).
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .filter_map(|s| s.ok())
            .collect(),
        hound::SampleFormat::Int => {
            let max_val = (1_i64 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
    };

    if channels == 1 {
        return Ok(interleaved);
    }

    // Downmix: average across channels per frame.
    let mono: Vec<f32> = interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();

    Ok(mono)
}

// ---------------------------------------------------------------------------
// SttProvider implementation
// ---------------------------------------------------------------------------

#[async_trait]
impl SttProvider for LocalWhisperProvider {
    /// Transcribes audio using the local whisper.cpp model.
    ///
    /// The method is `async` to satisfy the trait, but the heavy CPU work runs
    /// synchronously inside `tauri::async_runtime::spawn_blocking` so the
    /// async executor is not blocked.
    ///
    /// # Parameters
    /// - `audio`: raw WAV bytes at 16 kHz mono (as produced by the audio module).
    /// - `language`: ISO-639-1 code (`"de"`, `"en"`) or empty for auto-detect.
    /// - `prompt`: optional dictionary terms injected as `initial_prompt`.
    ///
    /// # Errors
    /// - `SttError::EmptyAudio` -- `audio` is empty.
    /// - `SttError::LocalWhisper(...)` -- model load / inference failure.
    async fn transcribe(
        &self,
        audio: Vec<u8>,
        language: &str,
        prompt: Option<&str>,
    ) -> Result<String, SttError> {
        if audio.is_empty() {
            return Err(SttError::EmptyAudio);
        }

        // Ensure model is loaded (cheap after first call).
        let ctx_arc = self.ensure_context()?;

        // Clone values we need to move into the blocking closure.
        let model_path = self.model_path.clone();
        let language = language.to_owned();
        let prompt = prompt.map(|p| p.to_owned());

        // CPU-bound work runs in a blocking thread to avoid stalling the async runtime.
        let result = tauri::async_runtime::spawn_blocking(move || {
            transcribe_blocking(&ctx_arc, &audio, &language, prompt.as_deref(), &model_path)
        })
        .await
        .map_err(|e| SttError::LocalWhisper(format!("Task join error: {e}")))?;

        result
    }
}

/// Runs the whisper.cpp inference on the current thread (blocking).
///
/// Called from within `spawn_blocking`; must not hold any async-friendly
/// locks (all locking here is sync `std::sync::Mutex`).
fn transcribe_blocking(
    ctx_arc: &Arc<Mutex<Option<whisper_rs::WhisperContext>>>,
    wav_bytes: &[u8],
    language: &str,
    prompt: Option<&str>,
    _model_path: &str,
) -> Result<String, SttError> {
    // Decode WAV -> mono f32 at 16 kHz.
    let audio_f32 = wav_bytes_to_pcm_f32(wav_bytes)
        .map_err(|e| SttError::LocalWhisper(e.to_string()))?;

    if audio_f32.is_empty() {
        return Err(SttError::EmptyAudio);
    }

    // Acquire context lock. Keep the lock for the duration of inference so
    // that two concurrent requests don't race on the same context.
    let guard = ctx_arc.lock().map_err(|_| {
        SttError::LocalWhisper(LocalWhisperError::LockPoisoned.to_string())
    })?;

    let ctx = guard.as_ref().ok_or_else(|| {
        SttError::LocalWhisper("WhisperContext not loaded".to_string())
    })?;

    // Create a fresh inference state for this request.
    let mut state = ctx.create_state().map_err(|e| {
        SttError::LocalWhisper(LocalWhisperError::StateCreate(e.to_string()).to_string())
    })?;

    // Build transcription parameters.
    let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy { best_of: 1 });

    // Language: empty string = whisper auto-detect.
    if !language.is_empty() {
        params.set_language(Some(language));
    }

    // Use all available logical CPUs, capped at 8 to avoid thrashing.
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get().min(8) as i32)
        .unwrap_or(4);
    params.set_n_threads(n_threads);

    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);
    params.set_no_timestamps(true);
    params.set_suppress_blank(true);

    // Inject dictionary terms as initial_prompt to improve recognition of
    // rare technical words and names (same effect as the Groq API `prompt`).
    if let Some(p) = prompt {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            params.set_initial_prompt(trimmed);
        }
    }

    log::debug!(
        "[local_whisper] Transcribing {} samples ({:.1}s) with lang={:?}, threads={}",
        audio_f32.len(),
        audio_f32.len() as f32 / 16_000.0,
        language,
        n_threads,
    );

    // Run inference.
    state
        .full(params, &audio_f32)
        .map_err(|e| SttError::LocalWhisper(
            LocalWhisperError::Transcription(e.to_string()).to_string()
        ))?;

    // Collect segment text.
    // In whisper-rs 0.15+, full_n_segments() returns i32 directly (no Result),
    // and full_get_segment_text() is replaced by get_segment(i) -> Option<WhisperSegment>
    // with WhisperSegment::to_str_lossy() for the text.
    let n_segments = state.full_n_segments();

    let mut transcript = String::new();
    for i in 0..n_segments {
        let seg = state.get_segment(i).ok_or_else(|| {
            SttError::LocalWhisper(
                LocalWhisperError::SegmentRead(format!("segment {i}: index out of bounds")).to_string()
            )
        })?;
        let text = seg.to_str_lossy().map_err(|e| {
            SttError::LocalWhisper(
                LocalWhisperError::SegmentRead(format!("segment {i}: {e}")).to_string()
            )
        })?;
        transcript.push_str(&text);
    }

    let result = transcript.trim().to_string();
    log::debug!("[local_whisper] Transcription result: {:?}", result);

    Ok(result)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// `LocalWhisperProvider::new` stores the model path without performing I/O.
    #[test]
    fn test_new_stores_model_path() {
        let path = "/tmp/models/ggml-base.bin";
        let provider = LocalWhisperProvider::new(path);
        assert_eq!(provider.model_path(), path);
    }

    /// Context is `None` before the first `transcribe` call (lazy load).
    #[test]
    fn test_context_starts_as_none() {
        let provider = LocalWhisperProvider::new("/tmp/ggml-tiny.bin");
        let guard = provider.ctx.lock().expect("lock should succeed");
        assert!(guard.is_none(), "context should be None before first use");
    }

    /// Empty audio is rejected immediately without touching the model.
    #[tokio::test]
    async fn test_transcribe_empty_audio_returns_error() {
        let provider = LocalWhisperProvider::new("/tmp/nonexistent-model.bin");
        let result = provider.transcribe(vec![], "de", None).await;
        assert!(
            matches!(result, Err(SttError::EmptyAudio)),
            "expected EmptyAudio, got: {result:?}"
        );
    }

    /// `ensure_context` returns `ModelNotFound` when the file does not exist.
    #[tokio::test]
    async fn test_transcribe_missing_model_returns_error() {
        let provider = LocalWhisperProvider::new("/tmp/definitely-does-not-exist-12345.bin");
        // Non-empty dummy WAV header (enough to pass the empty check).
        let dummy = vec![0u8; 64];
        let result = provider.transcribe(dummy, "de", None).await;
        assert!(
            matches!(result, Err(SttError::LocalWhisper(_))),
            "expected LocalWhisper error for missing model, got: {result:?}"
        );
        // Verify the error message mentions the path.
        if let Err(SttError::LocalWhisper(msg)) = result {
            assert!(
                msg.contains("definitely-does-not-exist"),
                "error should mention the missing path, got: {msg}"
            );
        }
    }

    /// `wav_bytes_to_pcm_f32` rejects audio that is not 16 kHz.
    #[test]
    fn test_wav_decode_wrong_sample_rate() {
        // Build a minimal 44100 Hz WAV header.
        let wav = build_minimal_wav(44_100, 1, &[0i16; 16]);
        let result = wav_bytes_to_pcm_f32(&wav);
        assert!(
            matches!(result, Err(LocalWhisperError::WrongSampleRate(44_100))),
            "expected WrongSampleRate(44100), got: {result:?}"
        );
    }

    /// `wav_bytes_to_pcm_f32` decodes a 16 kHz mono i16 WAV correctly.
    #[test]
    fn test_wav_decode_16khz_mono_i16() {
        // 16 samples at full positive scale.
        let samples: Vec<i16> = vec![i16::MAX; 16];
        let wav = build_minimal_wav(16_000, 1, &samples);
        let pcm = wav_bytes_to_pcm_f32(&wav).expect("decode should succeed");
        assert_eq!(pcm.len(), 16, "should have 16 mono frames");
        // i16::MAX / i16::MAX = approximately 1.0 (exact depends on hound).
        for s in &pcm {
            assert!(*s > 0.99 && *s <= 1.0, "sample should be ~1.0, got {s}");
        }
    }

    /// `wav_bytes_to_pcm_f32` downmixes stereo to mono.
    #[test]
    fn test_wav_decode_stereo_downmix() {
        // Interleaved stereo: left=i16::MAX, right=0 -> mono average = 0.5
        let mut samples = Vec::new();
        for _ in 0..8 {
            samples.push(i16::MAX);
            samples.push(0i16);
        }
        let wav = build_minimal_wav(16_000, 2, &samples);
        let pcm = wav_bytes_to_pcm_f32(&wav).expect("decode should succeed");
        assert_eq!(pcm.len(), 8, "8 stereo frames -> 8 mono samples");
        for s in &pcm {
            assert!(
                (*s - 0.5).abs() < 0.01,
                "mono average should be ~0.5, got {s}"
            );
        }
    }

    /// `wav_bytes_to_pcm_f32` returns an error for garbage input.
    #[test]
    fn test_wav_decode_garbage_input() {
        let result = wav_bytes_to_pcm_f32(b"not a wav file at all!!");
        assert!(
            matches!(result, Err(LocalWhisperError::WavDecode(_))),
            "expected WavDecode error for invalid input"
        );
    }

    // -----------------------------------------------------------------------
    // Test helper: build a minimal WAV file in memory
    // -----------------------------------------------------------------------

    /// Builds a minimal WAV byte buffer from raw i16 PCM samples.
    ///
    /// Only used in tests. The WAV spec is minimal: PCM, 16-bit, mono or stereo,
    /// at the given sample rate.
    fn build_minimal_wav(sample_rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut buf = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buf);
            let mut writer = hound::WavWriter::new(cursor, spec)
                .expect("should create WAV writer");
            for &s in samples {
                writer.write_sample(s).expect("should write sample");
            }
            writer.finalize().expect("should finalize WAV");
        }
        buf
    }
}
