//! Audio capture module.
//!
//! On desktop: uses cpal for cross-platform microphone access.
//! On mobile (Android): stub implementation -- audio capture happens in
//! Kotlin via AudioRecord API in the IME service.
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

#[cfg(desktop)]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(desktop)]
use cpal::{Device, SampleFormat, StreamConfig};
use thiserror::Error;

/// Errors that can occur during audio capture or encoding.
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("No input device available")]
    NoInputDevice,

    #[cfg(desktop)]
    #[error("Failed to query device config: {0}")]
    DeviceConfig(#[from] cpal::DefaultStreamConfigError),

    #[cfg(desktop)]
    #[error("Failed to build input stream: {0}")]
    BuildStream(#[from] cpal::BuildStreamError),

    #[cfg(desktop)]
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

    #[error("Not supported on this platform")]
    NotSupported,
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

/// Callback type fired once when continuous silence is detected.
/// No arguments -- the consumer just needs to know "silence happened".
pub type SilenceCallback = Box<dyn Fn() + Send + 'static>;

/// Configuration for silence detection. Stored per-recorder so it can be
/// cleared or updated between recording sessions.
#[cfg(desktop)]
struct SilenceConfig {
    /// Minimum number of consecutive silent chunks before firing the callback.
    silent_chunks_required: usize,
    /// RMS threshold below which audio is considered silence.
    threshold: f32,
    /// The closure to call (exactly once) when silence is detected.
    callback: SilenceCallback,
}

#[cfg(desktop)]
/// Everything the recording thread needs to know so it can stop cleanly.
struct RecordingSession {
    /// Sender: the main thread sends `()` to signal "stop recording".
    stop_tx: std::sync::mpsc::SyncSender<()>,
    /// Receiver: the main thread waits for the collected samples.
    result_rx: std::sync::mpsc::Receiver<RecordingResult>,
}

#[cfg(desktop)]
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
/// On desktop: uses cpal for audio capture on a dedicated background thread.
/// On mobile: stub -- audio capture happens in Kotlin (IME service).
pub struct AudioRecorder {
    #[cfg(desktop)]
    session: Mutex<Option<RecordingSession>>,
    #[cfg(desktop)]
    level_callback: Mutex<Option<AudioLevelCallback>>,
    #[cfg(desktop)]
    live_buffer: Arc<Mutex<LiveBuffer>>,
    /// Optional silence detection config. Installed before `start_recording`,
    /// consumed by the recording thread, cleared by `clear_silence_callback`.
    #[cfg(desktop)]
    silence_config: Mutex<Option<SilenceConfig>>,
}

#[cfg(desktop)]
/// Shared buffer for live audio preview during recording.
struct LiveBuffer {
    samples: Vec<f32>,
    native_sample_rate: u32,
    native_channels: u16,
}

impl AudioRecorder {
    /// Creates a new `AudioRecorder`. Does not open any device yet.
    pub fn new() -> Self {
        AudioRecorder {
            #[cfg(desktop)]
            session: Mutex::new(None),
            #[cfg(desktop)]
            level_callback: Mutex::new(None),
            #[cfg(desktop)]
            live_buffer: Arc::new(Mutex::new(LiveBuffer {
                samples: Vec::new(),
                native_sample_rate: 16000,
                native_channels: 1,
            })),
            #[cfg(desktop)]
            silence_config: Mutex::new(None),
        }
    }

    /// Installs a silence-detection callback.
    ///
    /// When the RMS of the incoming audio stays below `threshold` for at least
    /// `duration_secs` seconds, `callback` is called exactly once and then
    /// removed.  Call this *before* `start_recording`.
    ///
    /// The chunk size used for RMS evaluation is ~66 ms (same as the level
    /// callback interval), so `duration_secs` is rounded to the nearest
    /// chunk boundary.
    pub fn set_silence_callback(
        &self,
        _duration_secs: f32,
        _threshold: f32,
        _callback: SilenceCallback,
    ) {
        #[cfg(desktop)]
        {
            // We resolve the chunk count here (at install time) and store it
            // so the recording thread just needs an atomic counter to track
            // progress. The actual sample rate is not known yet (it comes from
            // the device at stream-open time), so we use the target rate as an
            // approximation. For speech, 66 ms resolution is more than enough.
            let chunks_per_sec = 15.0_f32; // ~66 ms per chunk (see recording_thread)
            let silent_chunks_required = ((_duration_secs * chunks_per_sec).round() as usize).max(1);
            let config = SilenceConfig {
                silent_chunks_required,
                threshold: _threshold,
                callback: _callback,
            };
            if let Ok(mut guard) = self.silence_config.lock() {
                *guard = Some(config);
            }
        }
    }

    /// Removes any installed silence callback (e.g. when stopping early).
    pub fn clear_silence_callback(&self) {
        #[cfg(desktop)]
        if let Ok(mut guard) = self.silence_config.lock() {
            *guard = None;
        }
    }

    /// Returns `true` if a silence callback is currently installed.
    ///
    /// Used in tests to verify that `set_silence_callback` took effect.
    pub fn has_silence_callback(&self) -> bool {
        #[cfg(desktop)]
        {
            self.silence_config
                .lock()
                .ok()
                .map(|g| g.is_some())
                .unwrap_or(false)
        }
        #[cfg(mobile)]
        {
            false
        }
    }

    /// Sets a callback that receives RMS audio levels during recording.
    pub fn set_level_callback(&self, _cb: AudioLevelCallback) {
        #[cfg(desktop)]
        { *self.level_callback.lock().unwrap() = Some(_cb); }
    }

    /// Opens an input device and begins capturing audio on a background thread.
    #[cfg(desktop)]
    pub fn start_recording(&self, device_name: Option<&str>) -> Result<(), AudioError> {
        let mut guard = self.session.lock().unwrap();
        if guard.is_some() {
            return Err(AudioError::AlreadyRecording);
        }

        let (stop_tx, stop_rx) = std::sync::mpsc::sync_channel::<()>(1);
        let (result_tx, result_rx) = std::sync::mpsc::channel::<RecordingResult>();

        let level_cb = self.level_callback.lock().unwrap().take();

        // Take the silence config so the recording thread owns it.
        let silence_cfg = self.silence_config.lock().ok().and_then(|mut g| g.take());

        if let Ok(mut lb) = self.live_buffer.lock() {
            lb.samples.clear();
        }
        let live_buf = Arc::clone(&self.live_buffer);

        let device_name_owned = device_name.map(|s| s.to_string());

        std::thread::spawn(move || {
            if let Err(e) = recording_thread(stop_rx, result_tx, level_cb, silence_cfg, device_name_owned.as_deref(), live_buf) {
                eprintln!("[audio] recording thread error: {e}");
            }
        });

        *guard = Some(RecordingSession {
            stop_tx,
            result_rx,
        });

        Ok(())
    }

    /// Stub: audio capture not available on mobile.
    #[cfg(mobile)]
    pub fn start_recording(&self, _device_name: Option<&str>) -> Result<(), AudioError> {
        Err(AudioError::NotSupported)
    }

    /// Stops the active recording and returns the captured audio encoded as WAV bytes.
    pub fn stop_recording(&self) -> Result<Vec<u8>, AudioError> {
        self.stop_recording_with_gain(1.0)
    }

    /// Stops recording and applies a gain multiplier to the audio.
    #[cfg(desktop)]
    pub fn stop_recording_with_gain(&self, gain: f32) -> Result<Vec<u8>, AudioError> {
        let mut guard = self.session.lock().unwrap();
        let session = guard.take().ok_or(AudioError::NotRecording)?;

        let _ = session.stop_tx.send(());

        let result = session.result_rx.recv().map_err(|_| AudioError::ThreadError)?;

        encode_to_wav_with_gain(&result.samples, result.native_sample_rate, result.native_channels, gain)
    }

    #[cfg(mobile)]
    pub fn stop_recording_with_gain(&self, _gain: f32) -> Result<Vec<u8>, AudioError> {
        Err(AudioError::NotSupported)
    }

    /// Returns a WAV snapshot of the audio captured so far, without stopping.
    #[cfg(desktop)]
    pub fn snapshot_wav(&self) -> Option<Vec<u8>> {
        let lb = self.live_buffer.lock().ok()?;
        if lb.samples.is_empty() {
            return None;
        }
        encode_to_wav(&lb.samples, lb.native_sample_rate, lb.native_channels).ok()
    }

    #[cfg(mobile)]
    pub fn snapshot_wav(&self) -> Option<Vec<u8>> {
        None
    }

    /// Returns `true` if a recording is currently active.
    pub fn is_recording(&self) -> bool {
        #[cfg(desktop)]
        { self.session.lock().unwrap().is_some() }
        #[cfg(mobile)]
        { false }
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
#[cfg(desktop)]
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

#[cfg(mobile)]
pub fn list_input_devices() -> Vec<String> {
    Vec::new()
}

/// Finds an input device by name, falling back to the default if not found.
#[cfg(desktop)]
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

#[cfg(desktop)]
/// Entry point for the background recording thread.
///
/// Opens the specified (or default) input device, starts the stream,
/// accumulates samples until the stop signal arrives, then sends samples
/// back and exits.
///
/// If `silence_cfg` is provided, the thread monitors RMS on each ~66ms chunk
/// and fires the callback (once) when silence has lasted the required number
/// of chunks.  The stop signal always takes priority -- if the main thread
/// sends a stop while waiting for silence, the thread exits normally.
fn recording_thread(
    stop_rx: std::sync::mpsc::Receiver<()>,
    result_tx: std::sync::mpsc::Sender<RecordingResult>,
    level_cb: Option<AudioLevelCallback>,
    silence_cfg: Option<SilenceConfig>,
    device_name: Option<&str>,
    live_buffer: Arc<Mutex<LiveBuffer>>,
) -> Result<(), AudioError> {
    let device = find_input_device(device_name)?;

    let config = device.default_input_config()?;
    let native_sample_rate = config.sample_rate().0;
    let native_channels = config.channels();
    let sample_format = config.sample_format();
    let stream_config: StreamConfig = config.into();

    // Initialize the live buffer with the correct format info.
    if let Ok(mut lb) = live_buffer.lock() {
        lb.native_sample_rate = native_sample_rate;
        lb.native_channels = native_channels;
    }

    let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let samples_writer = Arc::clone(&samples);

    // Shared level callback wrapped in Arc for use in the stream callback.
    let level_cb = level_cb.map(|cb| Arc::new(cb));
    let level_cb_clone = level_cb.clone();

    // Track samples for periodic RMS calculation (~15 Hz).
    let level_chunk: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
    let level_chunk_writer = Arc::clone(&level_chunk);
    let samples_per_tick = (native_sample_rate / 15) as usize; // ~66ms chunks

    // Silence detection: channel from stream callback to this thread.
    // The stream callback sends the RMS of each completed chunk.
    let (rms_tx, rms_rx) = std::sync::mpsc::channel::<f32>();

    let stream = build_stream_with_level(
        &device, &stream_config, sample_format, samples_writer,
        level_cb_clone, level_chunk_writer, samples_per_tick, live_buffer,
        Some(rms_tx),
    )?;

    stream.play()?;

    if let Some(cfg) = silence_cfg {
        // Silence-aware wait loop.
        // We use try_recv on the stop channel so we can interleave RMS checks
        // and stop-signal checks without blocking on either alone.
        //
        // IMPORTANT: We only start counting silence AFTER speech has been
        // detected (at least one chunk above the threshold). This prevents
        // the callback from firing immediately when the user hasn't started
        // speaking yet (e.g. ambient noise in a quiet room).
        let mut consecutive_silent_chunks = 0usize;
        let mut has_seen_speech = false;
        let mut fired = false;

        'outer: loop {
            // Check stop signal (non-blocking).
            match stop_rx.try_recv() {
                Ok(_) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break 'outer,
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }

            // Drain all pending RMS values (may be multiple per loop iteration).
            loop {
                match rms_rx.try_recv() {
                    Ok(rms) => {
                        if rms >= cfg.threshold {
                            // Speech detected -- from now on we track silence.
                            has_seen_speech = true;
                            consecutive_silent_chunks = 0;
                        } else if has_seen_speech {
                            // Silence AFTER speech -- count towards auto-stop.
                            consecutive_silent_chunks += 1;
                        }
                        // else: silence before any speech -- ignore.

                        if has_seen_speech
                            && consecutive_silent_chunks >= cfg.silent_chunks_required
                            && !fired
                        {
                            fired = true;
                            (cfg.callback)();
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break 'outer,
                }
            }

            // Small sleep to avoid busy-waiting (5 ms -- well within 66 ms chunk).
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    } else {
        // No silence detection -- just block until the stop signal arrives.
        let _ = stop_rx.recv();
    }

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
// Stream builders -- one per sample format (desktop only)
// ---------------------------------------------------------------------------

#[cfg(desktop)]
type SampleBuffer = Arc<Mutex<Vec<f32>>>;

/// Computes the RMS (root mean square) amplitude of a sample buffer.
pub fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

#[cfg(desktop)]
/// Helper: appends f32 data to the sample buffer and periodically fires the level callback.
///
/// When `rms_tx` is provided, sends the computed RMS to the recording thread
/// for silence detection after each completed chunk. This avoids any shared-state
/// synchronization inside the cpal callback: the callback stays cheap and lock-free.
fn process_f32_data(
    data: &[f32],
    buffer: &SampleBuffer,
    level_cb: &Option<Arc<AudioLevelCallback>>,
    level_chunk: &Arc<Mutex<Vec<f32>>>,
    samples_per_tick: usize,
    live_buf: &Arc<Mutex<LiveBuffer>>,
    rms_tx: &Option<std::sync::mpsc::Sender<f32>>,
) {
    buffer.lock().unwrap().extend_from_slice(data);
    if let Ok(mut lb) = live_buf.lock() {
        lb.samples.extend_from_slice(data);
    }

    let mut chunk = level_chunk.lock().unwrap();
    chunk.extend_from_slice(data);
    if chunk.len() >= samples_per_tick {
        let rms = compute_rms(&chunk);
        chunk.clear();

        // Fire the UI level callback (for the recording bar animation).
        if let Some(ref cb) = level_cb {
            cb(rms);
        }

        // Send RMS to the recording thread for silence detection.
        if let Some(ref tx) = rms_tx {
            // Ignore send errors -- the thread may have exited already.
            let _ = tx.send(rms);
        }
    }
}

#[cfg(desktop)]
/// Builds a cpal input stream for the given sample format, with audio-level callback support.
///
/// `rms_tx`: if provided, the completed RMS value of each chunk is sent to this
/// channel so the recording thread can do silence detection without touching the
/// stream callback closure (which must be `Send`).
fn build_stream_with_level(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    buffer: SampleBuffer,
    level_cb: Option<Arc<AudioLevelCallback>>,
    level_chunk: Arc<Mutex<Vec<f32>>>,
    samples_per_tick: usize,
    live_buf: Arc<Mutex<LiveBuffer>>,
    rms_tx: Option<std::sync::mpsc::Sender<f32>>,
) -> Result<cpal::Stream, AudioError> {
    match sample_format {
        SampleFormat::F32 => {
            let stream = device.build_input_stream(
                config,
                move |data: &[f32], _| {
                    process_f32_data(data, &buffer, &level_cb, &level_chunk, samples_per_tick, &live_buf, &rms_tx);
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
                    process_f32_data(&converted, &buffer, &level_cb, &level_chunk, samples_per_tick, &live_buf, &rms_tx);
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
                    process_f32_data(&converted, &buffer, &level_cb, &level_chunk, samples_per_tick, &live_buf, &rms_tx);
                },
                |err| eprintln!("[audio] stream error: {err}"),
                None,
            )?;
            Ok(stream)
        }
        _ => {
            let stream = device.build_input_stream(
                config,
                move |data: &[f32], _| {
                    process_f32_data(data, &buffer, &level_cb, &level_chunk, samples_per_tick, &live_buf, &rms_tx);
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
    encode_to_wav_with_gain(samples, native_sample_rate, native_channels, 1.0)
}

/// Like `encode_to_wav` but applies a gain multiplier to the audio.
/// `gain` of 1.0 = no change, 3.0 = 3x louder (for whisper mode).
pub fn encode_to_wav_with_gain(
    samples: &[f32],
    native_sample_rate: u32,
    native_channels: u16,
    gain: f32,
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
            let amplified = (sample * gain).clamp(-1.0, 1.0);
            let int_sample = (amplified * i16::MAX as f32) as i16;
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

    /// compute_rms returns 0.0 for an empty slice.
    #[test]
    fn test_compute_rms_empty() {
        assert_eq!(compute_rms(&[]), 0.0);
    }

    /// compute_rms for a constant amplitude signal equals the amplitude.
    #[test]
    fn test_compute_rms_constant_signal() {
        let samples = vec![0.5f32; 100];
        let rms = compute_rms(&samples);
        assert!((rms - 0.5).abs() < 1e-6, "rms of constant 0.5 signal should be 0.5, got {rms}");
    }

    /// compute_rms for silence (all zeros) returns 0.0.
    #[test]
    fn test_compute_rms_silence() {
        let samples = vec![0.0f32; 1000];
        let rms = compute_rms(&samples);
        assert_eq!(rms, 0.0);
    }

    /// compute_rms for a full-scale sine-like signal stays below 1.0.
    #[test]
    fn test_compute_rms_mixed_signal() {
        // Alternating +0.8 / -0.8 -- RMS should be 0.8.
        let samples: Vec<f32> = (0..100).map(|i| if i % 2 == 0 { 0.8 } else { -0.8 }).collect();
        let rms = compute_rms(&samples);
        assert!((rms - 0.8).abs() < 1e-5, "expected rms ≈ 0.8, got {rms}");
    }

    /// AudioRecorder: set_silence_callback and clear_silence_callback do not panic.
    #[test]
    fn test_set_and_clear_silence_callback() {
        let recorder = AudioRecorder::new();
        recorder.set_silence_callback(2.0, 0.01, Box::new(|| {}));
        recorder.clear_silence_callback();
        // No panic = pass
    }
}
