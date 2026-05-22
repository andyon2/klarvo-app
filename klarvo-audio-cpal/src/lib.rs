pub mod resampler;
pub mod source;

pub use source::CpalAudioSource;

use cpal::traits::{DeviceTrait, HostTrait};

/// Returns all enumerable input-device names from the default host.
///
/// Returns an empty `Vec` on enumeration failure (fail-soft; caller should treat as "no devices").
pub fn list_input_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| devices.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

/// Returns `true` if a device with the given name exists in the default host's input-device list.
///
/// Best-effort: returns `false` on enumeration failure. Used by the orchestrator to pre-flight
/// check before starting a session so the user gets a toast instead of silent OS-default fallback.
pub fn device_exists(name: &str) -> bool {
    let host = cpal::default_host();
    host.input_devices()
        .map(|mut devices| devices.any(|d| d.name().ok().as_deref() == Some(name)))
        .unwrap_or(false)
}
