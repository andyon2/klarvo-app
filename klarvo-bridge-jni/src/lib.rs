//! Klarvo JNI bridge — dual-surface (uniffi control-plane + raw-jni data-plane).
//!
//! Phase-0 spike scope: Audio-Level-Meter end-to-end on Linux without NDK.
//! See `docs/adr/0003-jni-spike-outcome.md`.

uniffi::setup_scaffolding!();

pub mod audio_level;
pub mod commands;
pub mod streams;

pub use audio_level::AudioLevel;
pub use commands::Session;
