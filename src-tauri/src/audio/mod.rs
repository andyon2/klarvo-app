//! Audio capture module using cpal for cross-platform microphone access.
//!
//! Captures microphone input and encodes it as 16kHz mono 16-bit PCM WAV,
//! which is the format required by Groq Whisper and whisper.cpp.
//!
//! ## Thread safety
//!
//! `cpal::Stream` is deliberately NOT `Send` on some platforms (e.g. Linux/ALSA
//! needs to stay on the thread that created it). To allow `AudioRecorder` to
//! live inside Tauri's `State` (which requires `Send + Sync`), we spawn a
//! dedicated OS thread that owns the stream for its lifetime. Communication
//! happens through a channel: the main code sends a "stop" signal and receives
//! the collected samples back.

use std::io::Cursor;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, StreamConfig};
use thiserror::Error;

/// Errors that can occur during audio capture or encoding.
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("No input device available")]
    NoInputDevice,

    #[error("Failed to query device config: {0}")]
    DeviceConfig(#[from] cpal::DefaultStreamConfigError),

    #[error("Failed to build input stream: {0}")]
    BuildStream(#[from] cpal::BuildStreamError),

    #[error("Failed to start stream: {0}")]
    PlayStream(#[from] cpal::PlayStreamError),

    #[error("Recording is already in progress")]
    AlreadyRecording,

    #[error("No recording in progress")]
    NotRecording,

    #[error("WAV encoding failed: {0}")]
    WavEncoding(#[from] hound::Error),

    #[error("Recording thread panicked or channel closed")]
    ThreadError,
}

/// Target output format for WAV encoding -- what Groq and whisper.cpp expect.
pub const TARGET_SAMPLE_RATE: u32 = 16_000;
pub const TARGET_CHANNELS: u16 = 1;
pub const TARGET_BIT_DEPTH: u16 = 16;

// ---------------------------------------------------------------------------
// Internal session state -- lives entirely on the cpal thread
// ---------------------------------------------------------------------------

/// Callback type for real-time audio level updates during recording.
/// The f32 value is the RMS amplitude (0.0..1.0) of the most recent chunk.
/// Must be `Send + Sync` because cpal's stream callback requires `Send`.
pub type AudioLevelCallback = Box<dyn Fn(f32) + Send + Sync + 'static>;

/// Everything the recording thread needs to know so it can stop cleanly.
struct RecordingSession {
    /// Sender: the main thread sends `()` to signal "stop recording".
    stop_tx: std::sync::mpsc::SyncSender<()>,
    /// Receiver: the main thread waits for the collected samples.
    result_rx: std::sync::mpsc::Receiver<RecordingResult>,
}

struct RecordingResult {
    samples: Vec<f32>,
    native_sample_rate: u32,
    native_channels: u16,
}

// ---------------------------------------------------------------------------
// Public recorder
// ---------------------------------------------------------------------------

/// Manages microphone recording state.
///
/// `Send + Sync` safe: the non-Send `cpal::Stream` lives on a dedicated
/// background thread; only `Send` types cross the boundary.
pub struct AudioRecorder {
    session: Mutex<Option<RecordingSession>>,
    level_callback: Mutex<Option<AudioLevelCallback>>,
}

impl AudioRecorder {
    /// Creates a new `AudioRecorder`. Does not open any device yet.
    pub fn new() -> Self {
        AudioRecorder {
            session: Mutex::new(None),
            level_callback: Mutex::new(None),
        }
    }

    /// Sets a callback that receives RMS audio levels during recording.
    /// Called approximately 15 times per second from the audio thread.
    pub fn set_level_callback(&self, cb: AudioLevelCallback) {
        *self.level_callback.lock().unwrap() = Some(cb);
    }

    /// Opens an input device and begins capturing audio on a background thread.
    ///
    /// If `device_name` is `None`, the system default input device is used.
    /// If a name is given but not found, falls back to the default device.
    ///
    /// Returns `AudioError::AlreadyRecording` if a session is already active.
    pub fn start_recording(&self, device_name: Option<&str>) -> Result<(), AudioError> {
        let mut guard = self.session.lock().unwrap();
        if guard.is_some() {
            return Err(AudioError::AlreadyRecording);
        }

        // These channels cross the thread boundary, so they must be Send.
        let (stop_tx, stop_rx) = std::sync::mpsc::sync_channel::<()>(1);
        let (result_tx, result_rx) = std::sync::mpsc::channel::<RecordingResult>();

        // Take the level callback (if any) to pass into the recording thread.
        let level_cb = self.level_callback.lock().unwrap().take();

        let device_name_owned = device_name.map(|s| s.to_string());

        // Spawn a dedicated thread that owns the cpal stream.
        std::thread::spawn(move || {
            if let Err(e) = recording_thread(stop_rx, result_tx, level_cb, device_name_owned.as_deref()) {
                eprintln!("[audio] recording thread error: {e}");
            }
        });

        *guard = Some(RecordingSession {
            stop_tx,
            result_rx,
        });

        Ok(())
    }

    /// Stops the active recording and returns the captured audio encoded as WAV bytes.
    ///
    /// The returned `Vec<u8>` is a complete WAV file (16kHz, mono, 16-bit PCM)
    /// ready to be sent to the Groq API or whisper.cpp.
    ///
    /// Returns `AudioError::NotRecording` if no session is active.
    pub fn stop_recording(&self) -> Result<Vec<u8>, AudioError> {
        let mut guard = self.session.lock().unwrap();
        let session = guard.take().ok_or(AudioError::NotRecording)?;

        // Signal the recording thread to stop.
        // Ignore send errors -- the thread may have already exited.
        let _ = session.stop_tx.send(());

        // Wait for the samples (blocks until the thread flushes).
        let result = session.result_rx.recv().map_err(|_| AudioError::ThreadError)?;

        encode_to_wav(&result.samples, result.native_sample_rate, result.native_channels)
    }

    /// Returns `true` if a recording is currently active.
    pub fn is_recording(&self) -> bool {
        self.session.lock().unwrap().is_some()
    }
}

impl Default for AudioRecorder {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: AudioRecorder only exposes Send-safe types across thread boundaries.
// The cpal::Stream (non-Send) is confined to the background thread.
unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}

// ---------------------------------------------------------------------------
// Device enumeration
// ---------------------------------------------------------------------------

/// Returns the names of all available audio input devices.
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| {
            devices
                .filter_map(|d| d.name().ok())
                .collect()
        })
        .unwrap_or_default()
}

/// Finds an input device by name, falling back to the default if not found.
fn find_input_device(name: Option<&str>) -> Result<Device, AudioError> {
    let host = cpal::default_host();

    if let Some(name) = name {
        if let Ok(devices) = host.input_devices() {
            for device in devices {
                if device.name().ok().as_deref() == Some(name) {
                    return Ok(device);
                }
            }
        }
        eprintln!("[audio] Device {name:?} not found, falling back to default");
    }

    host.default_input_device().ok_or(AudioError::NoInputDevice)
}

// ---------------------------------------------------------------------------
// Recording thread -- owns the cpal stream
// ---------------------------------------------------------------------------

/// Entry point for the background recording thread.
///
/// Opens the specified (or default) input device, starts the stream,
/// accumulates samples until the stop signal arrives, then sends samples
/// back and exits.
fn recording_thread(
    stop_rx: std::sync::mpsc::Receiver<()>,
    result_tx: std::sync::mpsc::Sender<RecordingResult>,
    level_cb: Option<AudioLevelCallback>,
    device_name: Option<&str>,
) -> Result<(), AudioError> {
    let device = find_input_device(device_name)?;

    let config = device.default_input_config()?;
    let native_sample_rate = config.sample_rate().0;
    let native_channels = config.channels();
    let sample_format = config.sample_format();
    let stream_config: StreamConfig = config.into();

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_writer = Arc::clone(&samples);

    // Shared level callback wrapped in Arc for use in the stream callback.
    let level_cb = level_cb.map(|cb| Arc::new(cb));
    let level_cb_clone = level_cb.clone();

    // Track samples for periodic RMS calculation (~15 Hz).
    let level_chunk: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let level_chunk_writer = Arc::clone(&level_chunk);
    let samples_per_tick = (native_sample_rate / 15) as usize; // ~66ms chunks

    let stream = build_stream_with_level(
        &device, &stream_config, sample_format, samples_writer,
        level_cb_clone, level_chunk_writer, samples_per_tick,
    )?;

    stream.play()?;

    // Block until the main thread sends a stop signal (or the channel closes).
    let _ = stop_rx.recv();

    // Drop stream to stop capture before reading samples.
    drop(stream);

    let captured = samples.lock().unwrap().clone();

    let _ = result_tx.send(RecordingResult {
        samples: captured,
        native_sample_rate,
        native_channels,
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Stream builders -- one per sample format
// ---------------------------------------------------------------------------

type SampleBuffer = Arc<Mutex<Vec<f32>>>;

/// Computes the RMS (root mean square) amplitude of a sample buffer.
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Helper: appends f32 data to the sample buffer and periodically fires the level callback.
fn process_f32_data(
    data: &[f32],
    buffer: &SampleBuffer,
    level_cb: &Option<Arc<AudioLevelCallback>>,
    level_chunk: &Arc<Mutex<Vec<f32>>>,
    samples_per_tick: usize,
) {
    buffer.lock().unwrap().extend_from_slice(data);

    if let Some(ref cb) = level_cb {
        let mut chunk = level_chunk.lock().unwrap();
        chunk.extend_from_slice(data);
        if chunk.len() >= samples_per_tick {
            let rms = compute_rms(&chunk);
            chunk.clear();
            cb(rms);
        }
    }
}

/// Builds a cpal input stream for the given sample format, with audio-level callback support.
fn build_stream_with_level(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    buffer: SampleBuffer,
    level_cb: Option<Arc<AudioLevelCallback>>,
    level_chunk: Arc<Mutex<Vec<f32>>>,
    samples_per_tick: usize,
) -> Result<cpal::Stream, AudioError> {
    match sample_format {
        SampleFormat::F32 => {
            let stream = device.build_input_stream(
                config,
                move |data: &[f32], _| {
                    process_f32_data(data, &buffer, &level_cb, &level_chunk, samples_per_tick);
                },
                |err| eprintln!("[audio] stream error: {err}"),
                None,
            )?;
            Ok(stream)
        }
        SampleFormat::I16 => {
            let stream = device.build_input_stream(
                config,
                move |data: &[i16], _| {
                    let converted: Vec<f32> = data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                    process_f32_data(&converted, &buffer, &level_cb, &level_chunk, samples_per_tick);
                },
                |err| eprintln!("[audio] stream error: {err}"),
                None,
            )?;
            Ok(stream)
        }
        SampleFormat::U16 => {
            let stream = device.build_input_stream(
                config,
                move |data: &[u16], _| {
                    let converted: Vec<f32> = data.iter().map(|&s| (s as f32 / u16::MAX as f32) * 2.0 - 1.0).collect();
                    process_f32_data(&converted, &buffer, &level_cb, &level_chunk, samples_per_tick);
                },
                |err| eprintln!("[audio] stream error: {err}"),
                None,
            )?;
            Ok(stream)
        }
        _ => {
            // Fallback: try f32
            let stream = device.build_input_stream(
                config,
                move |data: &[f32], _| {
                    process_f32_data(data, &buffer, &level_cb, &level_chunk, samples_per_tick);
                },
                |err| eprintln!("[audio] stream error: {err}"),
                None,
            )?;
            Ok(stream)
        }
    }
}

// ---------------------------------------------------------------------------
// WAV encoding (public -- also used in tests)
// ---------------------------------------------------------------------------

/// Converts raw f32 samples (possibly multi-channel, any sample rate) into a
/// 16kHz mono 16-bit PCM WAV buffer.
///
/// Steps:
/// 1. Downmix to mono by averaging channels.
/// 2. Resample from `native_sample_rate` to `TARGET_SAMPLE_RATE` using linear
///    interpolation (adequate for speech; avoids a heavy DSP dependency).
/// 3. Clamp and convert f32 -> i16.
/// 4. Encode as WAV using `hound`.
pub fn encode_to_wav(
    samples: &[f32],
    native_sample_rate: u32,
    native_channels: u16,
) -> Result<Vec<u8>, AudioError> {
    let mono = downmix_to_mono(samples, native_channels);

    let resampled = if native_sample_rate == TARGET_SAMPLE_RATE {
        mono
    } else {
        resample_linear(&mono, native_sample_rate, TARGET_SAMPLE_RATE)
    };

    let spec = hound::WavSpec {
        channels: TARGET_CHANNELS,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: TARGET_BIT_DEPTH,
        sample_format: hound::SampleFormat::Int,
    };

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
        for sample in resampled {
            let clamped = sample.clamp(-1.0, 1.0);
            // -1.0 * i16::MAX gives -32767; i16::MIN is -32768.
            // This avoids overflow on the most negative representable value.
            let int_sample = (clamped * i16::MAX as f32) as i16;
            writer.write_sample(int_sample)?;
        }
        writer.finalize()?;
    }

    Ok(cursor.into_inner())
}

/// Averages interleaved multi-channel samples into a single mono channel.
pub fn downmix_to_mono(samples: &[f32], channels: u16) -> Vec<f32> {
    if channels == 1 {
        return samples.to_vec();
    }
    let ch = channels as usize;
    samples
        .chunks_exact(ch)
        .map(|frame| frame.iter().sum::<f32>() / ch as f32)
        .collect()
}

/// Resamples `samples` from `src_rate` to `dst_rate` using linear interpolation.
///
/// Suitable for speech audio at dictation quality. For music or high-fidelity
/// audio a windowed-sinc resampler would be preferred.
pub fn resample_linear(samples: &[f32], src_rate: u32, dst_rate: u32) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let ratio = src_rate as f64 / dst_rate as f64;
    let out_len = (samples.len() as f64 / ratio).ceil() as usize;
    let mut output = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;

        let s0 = samples[idx.min(samples.len() - 1)];
        let s1 = samples[(idx + 1).min(samples.len() - 1)];
        output.push(s0 + frac * (s1 - s0));
    }

    output
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that a silence buffer produces a valid WAV file with the
    /// correct header parameters (16kHz, mono, 16-bit PCM).
    #[test]
    fn test_encode_to_wav_silence_produces_valid_wav() {
        // 1 second of silence at 44100 Hz stereo (common device default)
        let samples = vec![0.0f32; 44100 * 2];
        let wav_bytes = encode_to_wav(&samples, 44100, 2).unwrap();

        let cursor = Cursor::new(wav_bytes);
        let reader = hound::WavReader::new(cursor).expect("should be valid WAV");
        let spec = reader.spec();

        assert_eq!(spec.sample_rate, TARGET_SAMPLE_RATE);
        assert_eq!(spec.channels, TARGET_CHANNELS);
        assert_eq!(spec.bits_per_sample, TARGET_BIT_DEPTH);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);
    }

    /// Verifies that a tone at native rate is correctly resampled to the
    /// target rate -- the output length should match what we expect.
    #[test]
    fn test_encode_to_wav_resamples_correctly() {
        // 0.5 seconds at 48kHz mono -> should produce ~0.5s at 16kHz
        let input_duration_secs = 0.5f64;
        let native_rate = 48_000u32;
        let samples = vec![0.1f32; (native_rate as f64 * input_duration_secs) as usize];

        let wav_bytes = encode_to_wav(&samples, native_rate, 1).unwrap();
        let cursor = Cursor::new(wav_bytes);
        let reader = hound::WavReader::new(cursor).unwrap();

        let expected_samples = (TARGET_SAMPLE_RATE as f64 * input_duration_secs).ceil() as u32;
        let actual = reader.len();
        // Allow +-1 sample tolerance from rounding in the linear resampler
        assert!(
            actual.abs_diff(expected_samples) <= 1,
            "expected ~{expected_samples} samples, got {actual}"
        );
    }

    /// Verifies that f32 samples already at 16kHz mono pass through unchanged.
    #[test]
    fn test_encode_to_wav_passthrough_at_native_rate() {
        let samples: Vec<f32> = (0..16000)
            .map(|i| (i as f32 / 16000.0 * 2.0 - 1.0) * 0.5)
            .collect();

        let wav_bytes = encode_to_wav(&samples, 16_000, 1).unwrap();
        let cursor = Cursor::new(wav_bytes);
        let reader = hound::WavReader::new(cursor).unwrap();

        assert_eq!(reader.len(), 16000);
    }

    /// Verifies that downmix_to_mono averages stereo pairs correctly.
    #[test]
    fn test_downmix_to_mono_stereo() {
        let stereo = vec![0.5f32, 0.0, 0.5, 0.0, -1.0, 1.0];
        let mono = downmix_to_mono(&stereo, 2);
        assert_eq!(mono.len(), 3);
        assert!((mono[0] - 0.25).abs() < 1e-6, "frame 0 should average to 0.25");
        assert!((mono[1] - 0.25).abs() < 1e-6, "frame 1 should average to 0.25");
        assert!((mono[2] - 0.0).abs() < 1e-6, "frame 2 should average to 0.0");
    }

    /// Verifies that overly-loud samples are clamped and do not overflow.
    #[test]
    fn test_encode_to_wav_clips_correctly() {
        let samples = vec![2.0f32, -3.0, 0.0];
        let wav_bytes = encode_to_wav(&samples, 16_000, 1).unwrap();

        let cursor = Cursor::new(wav_bytes);
        let mut reader = hound::WavReader::new(cursor).unwrap();
        let pcm: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();

        assert_eq!(pcm[0], i16::MAX);
        assert_eq!(pcm[1], -i16::MAX); // clamp(-3.0, -1.0, 1.0) * i16::MAX
        assert_eq!(pcm[2], 0);
    }

    /// Verifies that AudioRecorder reports not-recording initially.
    #[test]
    fn test_audio_recorder_initially_not_recording() {
        let recorder = AudioRecorder::new();
        assert!(!recorder.is_recording());
    }

    /// Verifies that stopping without starting returns NotRecording.
    #[test]
    fn test_audio_recorder_stop_without_start_returns_error() {
        let recorder = AudioRecorder::new();
        let result = recorder.stop_recording();
        assert!(
            matches!(result, Err(AudioError::NotRecording)),
            "expected NotRecording, got: {result:?}"
        );
    }

    #[test]
    fn test_resample_linear_empty_input() {
        let result = resample_linear(&[], 44100, 16000);
        assert!(result.is_empty());
    }

    #[test]
    fn test_resample_linear_same_rate_is_noop() {
        let input = vec![0.1f32, 0.2, 0.3, 0.4];
        let output = resample_linear(&input, 16000, 16000);
        assert_eq!(output.len(), input.len());
        for (a, b) in input.iter().zip(output.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }
}
