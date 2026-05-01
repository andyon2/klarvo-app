use async_trait::async_trait;

use crate::error::PluginError;

use super::provider::{VadDecision, VadProvider};

const RMS_THRESHOLD: f32 = 0.01;

#[derive(Debug, Default)]
pub struct RmsVad {
    is_speaking: bool,
    speech_start_ms: Option<u64>,
}

impl RmsVad {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl VadProvider for RmsVad {
    async fn process(
        &mut self,
        samples: &[f32],
        ts_ms: u64,
    ) -> Result<VadDecision, PluginError> {
        if samples.is_empty() {
            return Ok(if self.is_speaking {
                VadDecision::Speech
            } else {
                VadDecision::Silence
            });
        }

        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        let rms = (sum_sq / samples.len() as f32).sqrt();
        let above = rms > RMS_THRESHOLD;

        Ok(match (self.is_speaking, above) {
            (false, true) => {
                self.is_speaking = true;
                self.speech_start_ms = Some(ts_ms);
                VadDecision::SpeechStart { ts_ms }
            }
            (true, false) => {
                self.is_speaking = false;
                let start = self.speech_start_ms.take().unwrap_or(ts_ms);
                let duration_ms = ts_ms.saturating_sub(start);
                VadDecision::SpeechEnd { ts_ms, duration_ms }
            }
            (true, true) => VadDecision::Speech,
            (false, false) => VadDecision::Silence,
        })
    }

    fn reset(&mut self) {
        self.is_speaking = false;
        self.speech_start_ms = None;
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn silence(n: usize) -> Vec<f32> {
        vec![0.0_f32; n]
    }

    fn loud(n: usize) -> Vec<f32> {
        vec![0.5_f32; n]
    }

    #[tokio::test]
    async fn silence_stays_silent() {
        let mut vad = RmsVad::new();
        let d = vad.process(&silence(160), 0).await.unwrap();
        assert_eq!(d, VadDecision::Silence);
    }

    #[tokio::test]
    async fn loud_triggers_speech_start_with_caller_ts() {
        let mut vad = RmsVad::new();
        let d = vad.process(&loud(160), 100).await.unwrap();
        assert_eq!(d, VadDecision::SpeechStart { ts_ms: 100 });
    }

    #[tokio::test]
    async fn continued_loud_emits_speech_not_start() {
        let mut vad = RmsVad::new();
        vad.process(&loud(160), 100).await.unwrap();
        let d = vad.process(&loud(160), 110).await.unwrap();
        assert_eq!(d, VadDecision::Speech);
    }

    #[tokio::test]
    async fn silence_after_speech_emits_end_with_duration() {
        let mut vad = RmsVad::new();
        vad.process(&loud(160), 100).await.unwrap();
        vad.process(&loud(160), 110).await.unwrap();
        let d = vad.process(&silence(160), 250).await.unwrap();
        assert_eq!(
            d,
            VadDecision::SpeechEnd {
                ts_ms: 250,
                duration_ms: 150,
            }
        );
    }

    #[tokio::test]
    async fn reset_clears_state() {
        let mut vad = RmsVad::new();
        vad.process(&loud(160), 100).await.unwrap();
        vad.reset();
        let d = vad.process(&loud(160), 200).await.unwrap();
        assert_eq!(d, VadDecision::SpeechStart { ts_ms: 200 });
    }

    #[tokio::test]
    async fn empty_samples_preserve_state() {
        let mut vad = RmsVad::new();
        vad.process(&loud(160), 100).await.unwrap();
        let d = vad.process(&[], 150).await.unwrap();
        assert_eq!(d, VadDecision::Speech);
    }

    #[tokio::test]
    async fn threshold_gate_below_stays_silent() {
        let mut vad = RmsVad::new();
        let quiet = vec![0.005_f32; 160];
        let d = vad.process(&quiet, 100).await.unwrap();
        assert_eq!(d, VadDecision::Silence);
    }
}
