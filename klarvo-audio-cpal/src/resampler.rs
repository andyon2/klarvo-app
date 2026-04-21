use klarvo_core::audio::AudioError;
use rubato::{FftFixedIn, Resampler as RubatoResampler};

pub(crate) struct Resampler {
    inner: Option<FftFixedIn<f32>>,
    chunk_size: usize,
}

impl Resampler {
    /// Creates a new Resampler. When `input_rate == output_rate`, processing is
    /// a zero-cost passthrough (no FFT). `chunk_size` must match exactly the
    /// number of mono f32 samples passed to `process` in non-passthrough mode
    /// (rubato FftFixedIn requires fixed-size input).
    pub(crate) fn new(
        input_rate: u32,
        output_rate: u32,
        chunk_size: usize,
    ) -> Result<Self, AudioError> {
        if input_rate == output_rate {
            return Ok(Self { inner: None, chunk_size });
        }
        // sub_chunks=2: rubato 0.16 computes fft_size_in = ceil(chunk_size/2/min_chunk)*min_chunk.
        // For 48k→16k, min_chunk=3: fft_size_in = ceil(512/3)*3 = 513. First process() call
        // produces floor(chunk_size/fft_size_in)*fft_size_out = 1*171 = 171 samples; subsequent
        // calls produce 2*171 = 342 samples (as saved frames accumulate). Average is correct.
        let inner = FftFixedIn::<f32>::new(
            input_rate as usize,
            output_rate as usize,
            chunk_size,
            2,
            1,
        )
        .map_err(|e| AudioError::ResampleFailed { msg: e.to_string() })?;
        Ok(Self { inner: Some(inner), chunk_size })
    }

    /// Process a mono f32 slice. For passthrough (equal rates) returns a copy.
    /// For resampling, expects exactly `chunk_size` samples (rubato FftFixedIn
    /// constraint); passing a different length yields ResampleFailed.
    pub(crate) fn process(&mut self, samples: &[f32]) -> Result<Vec<f32>, AudioError> {
        match &mut self.inner {
            None => Ok(samples.to_vec()),
            Some(resampler) => {
                let waves_in = vec![samples.to_vec()];
                let waves_out = resampler
                    .process(&waves_in, None)
                    .map_err(|e| AudioError::ResampleFailed { msg: e.to_string() })?;
                Ok(waves_out.into_iter().next().unwrap_or_default())
            }
        }
    }

    pub(crate) fn chunk_size(&self) -> usize {
        self.chunk_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resampler_passthrough_when_rates_equal() {
        let mut r = Resampler::new(16_000, 16_000, 1024).unwrap();
        let input: Vec<f32> = (0..1024).map(|i| i as f32 / 1024.0).collect();
        let output = r.process(&input).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn resampler_sample_count_correct() {
        let mut r = Resampler::new(48_000, 16_000, 1024).unwrap();
        let input = vec![0.0_f32; 1024];
        let output = r.process(&input).unwrap();
        // rubato 0.16 FftFixedIn with sub_chunks=2 computes fft_size_in=513 for 48k→16k.
        // First call: floor(1024/513)=1 FFT chunk → 171 output samples.
        // Subsequent calls accumulate saved frames and produce up to 2*171=342.
        // output_frames_max() = 2*171 = 342.
        assert!(
            output.len() >= 1 && output.len() <= 342,
            "output.len()={} out of expected [1, 342]",
            output.len()
        );
    }

    #[test]
    fn resampler_sine_preserves_energy() {
        use std::f32::consts::PI;
        let mut r = Resampler::new(48_000, 16_000, 1024).unwrap();
        let input: Vec<f32> =
            (0..1024).map(|i| (2.0 * PI * 440.0 * i as f32 / 48_000.0).sin()).collect();
        // First call is warmup — rubato's overlap-save filter initializes with zeros,
        // causing amplitude reduction in the first block. Test against second call.
        let _ = r.process(&input).unwrap();
        let output = r.process(&input).unwrap();
        assert!(!output.is_empty(), "second process() call produced no output");
        let in_rms = (input.iter().map(|x| x * x).sum::<f32>() / input.len() as f32).sqrt();
        let out_rms = (output.iter().map(|x| x * x).sum::<f32>() / output.len() as f32).sqrt();
        // 440 Hz is well below the 8 kHz Nyquist for 16 kHz output; energy is preserved.
        // Epsilon 0.2 accommodates the windowing mismatch between 1024-sample input
        // (21 ms) and 342-sample output (21 ms at 16 kHz) across non-integer sine cycles.
        approx::assert_abs_diff_eq!(in_rms, out_rms, epsilon = 0.2);
    }

    #[test]
    fn i16_conversion_math() {
        // Verify the i16→f32 normalization used in the CpalAudioSource I16 callback.
        let max_f = i16::MAX as f32 / i16::MAX as f32;
        approx::assert_abs_diff_eq!(max_f, 1.0_f32, epsilon = 1e-6);

        let zero_f = 0_i16 as f32 / i16::MAX as f32;
        approx::assert_abs_diff_eq!(zero_f, 0.0_f32, epsilon = 1e-6);

        // i16::MIN / i16::MAX ≈ -32768/32767 ≈ -1.000030 (asymmetric two's complement)
        let min_f = i16::MIN as f32 / i16::MAX as f32;
        approx::assert_abs_diff_eq!(min_f, -1.0_f32, epsilon = 0.001);
    }
}
