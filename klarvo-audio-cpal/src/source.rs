use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::SampleFormat;
use klarvo_core::audio::{AudioError, AudioEvent, AudioSource, CaptureConfig, CaptureHandle};
use tokio::sync::broadcast;

use crate::resampler::Resampler;

const CHUNK_SIZE: usize = 1024;

/// Real `AudioSource` implementation backed by cpal (WASAPI / ALSA / CoreAudio).
///
/// Acquires the default host → default input device → default input config.
/// Resamples to 16 kHz mono f32 via rubato `FftFixedIn` before emitting
/// `AudioEvent::Samples`. Downmixes multi-channel input to mono (arithmetic
/// mean per frame). Emits `AudioEvent::Level` for each outgoing chunk.
///
/// `ts_ms` on `AudioEvent::Samples` is the chunk-START timestamp derived from
/// a single `Instant` captured at the top of `start()` (session-relative
/// monotone ms, ref ADR-0006-Amendment-1, `memory/project_event_ts_ms_convention`).
///
/// Platform note: cpal selects WASAPI on Windows, ALSA on Linux, CoreAudio on
/// macOS. Shell-consumer wiring (`cfg(target_os)` selection) is Epic-3-scope
/// (ADR-0006 Amendment 2); this crate builds unconditionally.
pub struct CpalAudioSource;

/// Owned guard that closes the broadcast channel and stops the cpal stream on Drop.
struct CpalGuard {
    _stream: cpal::Stream,
    tx_slot: Arc<Mutex<Option<broadcast::Sender<AudioEvent>>>>,
}

impl Drop for CpalGuard {
    fn drop(&mut self) {
        // Set the slot to None before the stream drops, so error-callback sees
        // a closed channel and downstream receivers get RecvError::Closed.
        *self.tx_slot.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }
}

// Safety: cpal::Stream is Send on all supported platforms in cpal 0.15 (WASAPI
// wraps COM calls on a dedicated thread; ALSA uses Arc<Mutex<...>>). CpalGuard
// has no &self methods so Sync is trivially sound.
unsafe impl Send for CpalGuard {}
unsafe impl Sync for CpalGuard {}

#[async_trait]
impl AudioSource for CpalAudioSource {
    async fn start(&mut self, config: CaptureConfig) -> Result<CaptureHandle, AudioError> {
        let host = cpal::default_host();

        // Resolve device: named lookup with OS-default fallback.
        let device = match config.device.as_deref() {
            None => host.default_input_device().ok_or(AudioError::DeviceUnavailable)?,
            Some(requested) => {
                let found = host
                    .input_devices()
                    .map_err(|e| AudioError::DeviceConfigError { msg: e.to_string() })?
                    .find(|d| d.name().ok().as_deref() == Some(requested));
                match found {
                    Some(d) => d,
                    None => {
                        tracing::warn!(
                            target: "klarvo.audio.device",
                            requested = %requested,
                            fallback = "os-default",
                            "configured audio device not found; falling back to OS default"
                        );
                        host.default_input_device().ok_or(AudioError::DeviceUnavailable)?
                    }
                }
            }
        };

        let default_cfg = device
            .default_input_config()
            .map_err(|e| AudioError::DeviceConfigError { msg: e.to_string() })?;

        let sample_format = default_cfg.sample_format();
        let hw_rate = default_cfg.sample_rate().0;
        let hw_channels = default_cfg.channels() as usize;

        let device_name = device.name().unwrap_or_else(|_| "<unknown>".to_string());
        let selection_mode = if config.device.is_some() { "configured" } else { "OS default" };
        tracing::info!(
            target: "klarvo.audio.device",
            device = %device_name,
            sample_rate = hw_rate,
            channels = hw_channels,
            sample_format = ?sample_format,
            selection = selection_mode,
            "audio input device selected"
        );

        let stream_config = cpal::StreamConfig {
            channels: default_cfg.channels(),
            sample_rate: default_cfg.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        // Shared slot: input-callback publishes; CpalGuard::drop closes it.
        let tx_slot: Arc<Mutex<Option<broadcast::Sender<AudioEvent>>>> =
            Arc::new(Mutex::new(Some(config.events)));

        let resampler =
            Resampler::new(hw_rate, klarvo_core::audio::AUDIO_SAMPLE_RATE, CHUNK_SIZE)
                .map_err(|e| match e {
                    AudioError::ResampleFailed { msg } => AudioError::ResampleFailed { msg },
                    other => other,
                })?;
        let resampler = Arc::new(Mutex::new(resampler));

        // Accumulation buffer for resampler (holds mono f32 samples between callbacks).
        // Tuple: (samples_buf, ts_ms_of_first_sample_in_buf).
        let pending: Arc<Mutex<(Vec<f32>, u64)>> = Arc::new(Mutex::new((Vec::new(), 0)));

        // Capture session-start for ts_ms derivation (ADR-0006-A1: FIRST in callback).
        let session_start = Instant::now();

        let stream = match sample_format {
            SampleFormat::F32 => {
                let slot = Arc::clone(&tx_slot);
                let slot_err = Arc::clone(&tx_slot);
                let res = Arc::clone(&resampler);
                let pend = Arc::clone(&pending);
                device.build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &cpal::InputCallbackInfo| {
                        let ts_ms = session_start.elapsed().as_millis() as u64;
                        let mono = downmix(data, hw_channels);
                        flush_chunks(&mono, ts_ms, &slot, &res, &pend);
                    },
                    move |err| {
                        tracing::warn!(target: "klarvo.audio.device", "capture error: {err}");
                        *slot_err.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    },
                    None,
                )
                .map_err(|e| AudioError::DeviceConfigError { msg: e.to_string() })?
            }
            SampleFormat::I16 => {
                let slot = Arc::clone(&tx_slot);
                let slot_err = Arc::clone(&tx_slot);
                let res = Arc::clone(&resampler);
                let pend = Arc::clone(&pending);
                device.build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &cpal::InputCallbackInfo| {
                        let ts_ms = session_start.elapsed().as_millis() as u64;
                        let f32s: Vec<f32> =
                            data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();
                        let mono = downmix(&f32s, hw_channels);
                        flush_chunks(&mono, ts_ms, &slot, &res, &pend);
                    },
                    move |err| {
                        tracing::warn!(target: "klarvo.audio.device", "capture error: {err}");
                        *slot_err.lock().unwrap_or_else(|e| e.into_inner()) = None;
                    },
                    None,
                )
                .map_err(|e| AudioError::DeviceConfigError { msg: e.to_string() })?
            }
            _ => return Err(AudioError::UnsupportedFormat),
        };

        stream
            .play()
            .map_err(|e| AudioError::CaptureInterrupted { msg: e.to_string() })?;

        let guard = CpalGuard { _stream: stream, tx_slot };
        Ok(CaptureHandle::new(guard))
    }
}

/// Downmix interleaved multi-channel samples to mono (arithmetic mean per frame).
fn downmix(data: &[f32], channels: usize) -> Vec<f32> {
    if channels == 1 {
        return data.to_vec();
    }
    data.chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Accumulate mono samples into the pending buffer; flush full CHUNK_SIZE chunks
/// through the resampler and emit AudioEvent::Samples on the broadcast channel.
/// `ts_ms_callback` is captured at the VERY START of the callback (ADR-0006-A1).
fn flush_chunks(
    mono: &[f32],
    ts_ms_callback: u64,
    slot: &Arc<Mutex<Option<broadcast::Sender<AudioEvent>>>>,
    resampler: &Arc<Mutex<Resampler>>,
    pending: &Arc<Mutex<(Vec<f32>, u64)>>,
) {
    let mut guard = pending.lock().unwrap_or_else(|e| e.into_inner());
    let (buf, buf_ts) = &mut *guard;

    if buf.is_empty() {
        *buf_ts = ts_ms_callback;
    }
    buf.extend_from_slice(mono);

    let chunk_size = resampler.lock().unwrap_or_else(|e| e.into_inner()).chunk_size();

    while buf.len() >= chunk_size {
        let chunk: Vec<f32> = buf.drain(..chunk_size).collect();
        let ts_ms = *buf_ts;
        // Next chunk's start ts is current elapsed when this one flushes.
        // (approximation; real chunk-start is within one callback interval)

        let resampled = match resampler.lock().unwrap_or_else(|e| e.into_inner()).process(&chunk) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(target: "klarvo.audio.device", "resample error: {e}");
                return;
            }
        };

        let slot_guard = slot.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(sender) = &*slot_guard {
            let rms = compute_rms(&resampled);
            let data: Arc<[f32]> = resampled.into();
            let _ = sender.send(AudioEvent::Samples { data: Arc::clone(&data), ts_ms });
            let _ = sender.send(AudioEvent::Level { rms, ts_ms });
        }
    }
}

fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
}
