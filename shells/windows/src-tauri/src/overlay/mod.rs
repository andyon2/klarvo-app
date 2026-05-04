//! Pill-Bar overlay window (Story 9.6).
//!
//! Transparent always-on-top window declared in `tauri.conf.json` (label =
//! "pill-bar"). Driven by EventBus subscription on the main runtime: shows on
//! `Event::RecordingStarted`, fades out on `Event::RecordingCompleted`, and
//! emits `pill_bar.waveform_tick` to the WebView from `Event::AudioLevel`.

pub mod pill_bar;
