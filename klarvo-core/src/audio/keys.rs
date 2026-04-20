/// Used by Epic 3 cpal-impl (`CpalAudioSource::start`) when the OS-audio-device
/// is unavailable. Phase-1 `MockAudioSource` never emits this key — defined
/// here to co-locate the error-contract with the Trait-definition (precedent:
/// `memory/project_keystore_trait_surface` 1C.1-pattern).
pub const DEVICE_UNAVAILABLE: &str = "error.audio.device_unavailable";

/// Emitted when the impl cannot resample/downmix to the advisory 16 kHz mono
/// f32 format. Explicitly named in ADR-0006-Rustdoc as the concrete error-path
/// from `CaptureConfig.sample_rate` advisory-miss.
pub const UNSUPPORTED_FORMAT: &str = "error.audio.unsupported_format";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_key_format() {
        assert!(DEVICE_UNAVAILABLE.starts_with("error.audio."));
        assert!(UNSUPPORTED_FORMAT.starts_with("error.audio."));
    }
}
