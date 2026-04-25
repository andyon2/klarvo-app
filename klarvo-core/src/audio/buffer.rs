/// Aggregated audio-buffer for pipeline-entry.
///
/// `ts_ms_start`/`ts_ms_end` are session-relative monotone milliseconds sourced from
/// `VadDecision::SpeechStart.ts_ms` and `VadDecision::SpeechEnd.ts_ms` respectively
/// (ref ADR-0001, `memory/project_event_ts_ms_convention`). These bounds are load-bearing
/// for Epic-6 Observability (latency-measurement: ts_ms_start → STT-result ts_ms).
/// `sample_rate` is always 16_000 in Phase-1 — carried as field for
/// SttProvider-WAV-encoding-convenience and for future Phase-2+ variable-rate-extension.
/// Current value sourced from `klarvo_core::audio::AUDIO_SAMPLE_RATE` const;
/// ADR-0006-Sub-Decision-2 mandates 16 kHz mono fixed emission.
/// `channels` is omitted — format is fixed mono f32 per ADR-0006-Sub-Decision-2.
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub ts_ms_start: u64,
    pub ts_ms_end: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_buffer_fields_accessible_and_clone_works() {
        let buf = AudioBuffer {
            samples: vec![0.0, 0.5, -0.5],
            sample_rate: 16_000,
            ts_ms_start: 100,
            ts_ms_end: 164,
        };
        let cloned = buf.clone();
        assert_eq!(cloned.samples.len(), 3);
        assert_eq!(cloned.sample_rate, 16_000);
        assert_eq!(cloned.ts_ms_start, 100);
        assert_eq!(cloned.ts_ms_end, 164);
    }
}
