# VAD and end-of-speech detection for real-time dictation

**Silero VAD v5 is the clear winner for your dictation app.** It runs in under 1ms per 32ms frame via ONNX, rejects music bleed-through that defeats energy-based detection, outputs calibrated speech probabilities (not just binary), and the same model file works on both Windows (Rust `ort` crate) and Android (ONNX Runtime AAR). Combined with an 85Hz high-pass filter to strip bass-heavy headphone bleed and a dual-threshold state machine with ~600ms hangover, this pipeline processes each 66ms chunk in roughly 2ms — well under your 10ms budget. Pre-filtering audio through VAD before Whisper is also the single most effective technique for eliminating phantom transcriptions like "Thank you for watching" and "Copyright WDR."

---

## Why Silero VAD dominates the alternatives

Five VAD approaches were evaluated against your specific constraints: 16kHz mono PCM, 66ms chunks, <10ms latency, offline operation, and the critical requirement to distinguish speech from headphone music bleed-through.

**WebRTC VAD (libfvad)** uses a 20-year-old Gaussian Mixture Model that splits audio into six frequency sub-bands and computes log-energy features. It runs in microseconds and ships at **158KB**, but it was designed for telephony, not music discrimination. At the Picovoice benchmark's 5% false-positive rate, WebRTC achieves only **50% true-positive rate** versus Silero's 87.7%. Critically, it returns binary speech/no-speech with no probability score, and it classifies most music — even instrumental — as speech. Mode 3 (Very Aggressive) reduces music false positives but misses legitimate quiet speech. For a dictation app where headphones play music, WebRTC VAD alone is inadequate.

**Silero VAD v5** (released June 2024) uses a convolutional + LSTM architecture trained on **13,000+ hours** of speech across 6,000+ languages. The ONNX model is ~2MB and processes each 512-sample frame (32ms at 16kHz) in under 1ms on a single CPU thread. A key finding from GitHub discussions: Silero VAD rejects singing in music (cumbia, children's songs) and returns empty timestamps, meaning headphone music bleed-through — even with vocals — generally does **not** trigger it. This is exactly the behavior a dictation app needs. The model outputs a probability between 0.0 and 1.0, enabling smooth hysteresis logic rather than the chattery binary decisions WebRTC produces.

**RNNoise (Mozilla/Xiph)** is primarily a noise suppressor with a VAD probability as a byproduct. Its GRU-based architecture operates on 480 samples at **48kHz only**, requiring upsampling from your 16kHz stream. The Rust port (`nnnoiseless` v0.5.2) does not expose the VAD probability in its public API — you would need to fork the crate. Music was not in rnnoise's primary training set, so its speech/music discrimination is unvalidated. The 48kHz requirement and missing VAD API make it impractical as a primary VAD, though it could serve as an optional noise-suppression preprocessor.

**Spectral feature approaches** (zero-crossing rate, spectral centroid, energy) require **1–2 seconds** of audio for their most discriminative features (variance of spectral centroid, 4Hz modulation energy). Your 66ms chunks are far too short. Frame-level spectral features alone yield ~20–25% error rate for speech/music classification. Building an adequate classifier from these features approaches the complexity of just using Silero, with worse results.

**Cobra VAD (Picovoice)** claims the highest accuracy (98.9% TPR at 5% FPR) with pure C implementation and no runtime dependencies. However, it requires a **proprietary API key**, is closed-source, and introduces vendor lock-in. Its benchmarks come from the vendor itself. For an offline, open-source-friendly dictation app, this is a non-starter unless you accept the licensing constraint.

| Approach | Speech/Music | Latency/frame | Frame size (16kHz) | Output | License |
|---|---|---|---|---|---|
| **Silero VAD v5** | Good (rejects music) | <1ms | 512 samples (32ms) | Probability 0–1 | MIT |
| WebRTC VAD | Poor (triggers on music) | <0.01ms | 160/320/480 samples | Binary | BSD |
| RNNoise | Unvalidated | ~0.13ms | 480 @ 48kHz only | Probability (hidden) | BSD |
| Spectral features | Poor at 66ms | <0.5ms | Flexible | Custom | N/A |
| Cobra VAD | Good (claimed) | <0.1ms | ~512 samples | Probability 0–1 | Proprietary |

---

## Cross-platform library ecosystem

The most important finding for cross-platform development: **the identical `silero_vad.onnx` model file runs on both Rust (via the `ort` crate) and Android (via `onnxruntime-android` AAR)**. Same input tensor shapes, same parameters, same thresholds. You share one model file and one set of VAD configuration constants across platforms.

**On Rust/Windows**, the recommended crate is **`voice_activity_detector` v0.2.1**, which wraps Silero VAD v5 with a clean builder API. It bundles the ONNX model internally and depends on the `ort` crate for inference. It is actively maintained (updated ~2 months ago) with ~29,000 downloads. The API is straightforward:

```rust
// Rust: voice_activity_detector crate
let mut vad = VoiceActivityDetector::builder()
    .sample_rate(16000)
    .chunk_size(512usize)  // Silero v5 fixed at 512 for 16kHz
    .build()?;

let probability: f32 = vad.predict(&audio_frame_512_samples);
```

The underlying `ort` crate (v2.0.0-rc.12, wrapping ONNX Runtime 1.88.0) is production-grade — used by Google Magika, SurrealDB, and Hugging Face. It supports Windows, Linux, macOS, and Android with CPU, CUDA, DirectML, and XNNPACK execution providers.

The older `webrtc-vad` crate (v0.4.0) wraps libfvad but has not been updated since 2019. The `nnnoiseless` crate (v0.5.2) ports RNNoise to pure Rust but is stalled for 3 years and does not expose the VAD probability output.

**On Android/Kotlin**, the best option is **`android-vad` v2.0.10** by gkonovalov (471 GitHub stars, MIT license, actively maintained). It provides both WebRTC and Silero VAD modules as separate Gradle dependencies:

```kotlin
// Android: android-vad Silero module
implementation("com.github.gkonovalov.android-vad:silero:2.0.10")

val vad = VadSilero(context,
    sampleRate = SampleRate.SAMPLE_RATE_16K,
    frameSize = FrameSize.FRAME_SIZE_512,
    mode = Mode.NORMAL,
    silenceDurationMs = 600,
    speechDurationMs = 50
)
val isSpeech: Boolean = vad.isSpeech(shortArrayPcm)
```

This library bundles the ONNX model in assets and uses ONNX Runtime Mobile internally. The Silero module requires minSdkVersion 24. Alternatively, you can use `onnxruntime-android:1.24.3` directly from Maven Central and load the same ONNX model yourself for more control over inference parameters and threshold logic.

---

## Integration architecture and the complete pipeline

Your 66ms chunks (1056 samples at 16kHz) do not align with Silero's fixed 512-sample frame size. The solution is a **ring buffer** that accumulates samples and drains aligned frames. Each 1056-sample chunk yields **two 512-sample Silero inferences** with 32 samples left over for the next cycle.

The recommended pipeline flows: **audio chunk → high-pass filter → ring buffer → Silero VAD frames → energy gate → hysteresis state machine → speech buffer → Whisper**.

**High-pass filtering at 85Hz** removes bass-heavy headphone bleed that leaks around ear cushions. A second-order Butterworth biquad provides 12dB/octave roll-off without touching speech fundamentals (male F0 ≈ 85Hz, female F0 ≈ 165Hz). In Rust, the `biquad` crate handles this in a single per-sample multiply-accumulate — negligible cost. On Android, implement the same biquad coefficient math or use `AudioEffect` noise suppression.

**The energy gate** is an optional but valuable supplement. Before running Silero inference, check if the chunk's RMS exceeds a noise floor (e.g., **-50 dBFS**). Pure silence can be rejected without invoking the neural network, saving CPU. When Silero does run, combine its probability with the energy check: `is_speech = silero_prob >= threshold AND rms_db > noise_floor`.

**Dual-threshold hysteresis** prevents chattering at speech boundaries. Use an **onset threshold of 0.5** and an **offset threshold of 0.35** — the 0.15 gap means that once speech starts, it takes a definitive drop below 0.35 to trigger the silence counter, not just a momentary dip below 0.5. This pattern is battle-tested in faster-whisper, LiveKit, and Pipecat.

The **state machine** has two states: SILENCE and SPEAKING. Require **3 consecutive speech frames (~96ms)** before transitioning from SILENCE to SPEAKING — this filters transient pops and door slams. Require **19 consecutive silence frames (~608ms)** before transitioning from SPEAKING to SILENCE — this preserves natural between-word pauses in dictation. During SILENCE, maintain a rolling **300ms prefix buffer** so that when speech starts, you capture the onset phonemes that preceded the detection.

**Recommended default parameters for dictation:**

| Parameter | Value | Rationale |
|---|---|---|
| Onset threshold | 0.5 | Silero's calibrated default |
| Offset threshold | 0.35 | Hysteresis gap of 0.15 |
| Min onset frames | 3 (~96ms) | Filters transient noise |
| Min silence frames | 19 (~608ms) | Preserves natural pauses |
| High-pass cutoff | 85Hz | Below male F0, removes bass bleed |
| Energy floor | -50 dBFS | Skip Silero on dead silence |
| Prefix buffer | 300ms (4800 samples) | Capture speech onset context |
| Silero frame size | 512 samples (32ms) | V5 fixed requirement |

**Total processing time per 66ms chunk**: ~1ms for two Silero inferences + ~0.01ms for biquad filter + negligible state machine logic = **~1–2ms total**, leaving 8ms of headroom within your 10ms budget.

```rust
// Simplified pipeline pseudocode (Rust)
fn on_audio_chunk(&mut self, chunk: &[f32; 1056]) -> Option<Vec<f32>> {
    // 1. High-pass filter at 85Hz
    let filtered: Vec<f32> = chunk.iter()
        .map(|&s| self.highpass.process(s)).collect();

    // 2. Energy gate
    let rms = (filtered.iter().map(|s| s * s).sum::<f32>() / 1056.0).sqrt();
    let energy_ok = rms > self.energy_floor;

    // 3. Buffer and drain Silero frames
    self.ring_buffer.extend(&filtered);
    let mut max_prob: f32 = 0.0;
    while self.ring_buffer.len() >= 512 {
        let frame: Vec<f32> = self.ring_buffer.drain(..512).collect();
        max_prob = max_prob.max(self.silero.predict(&frame));
    }

    // 4. State machine with hysteresis
    match self.state {
        Silence => {
            self.prefix_buf.extend(chunk);  // rolling 300ms window
            if max_prob >= 0.5 && energy_ok {
                self.onset_count += 1;
                if self.onset_count >= 3 {
                    self.state = Speaking;
                    self.speech_buf.extend(&self.prefix_buf);
                }
            } else { self.onset_count = 0; }
            None
        }
        Speaking => {
            self.speech_buf.extend(chunk);
            if max_prob < 0.35 || !energy_ok {
                self.silence_count += 1;
                if self.silence_count >= 19 {  // ~608ms hangover
                    self.state = Silence;
                    return Some(std::mem::take(&mut self.speech_buf));
                    // → Send returned audio to Whisper
                }
            } else { self.silence_count = 0; }
            None
        }
    }
}
```

---

## Whisper hallucination mitigation is a three-layer problem

VAD pre-filtering is Layer 1 and the most impactful. Academic research from AGH University of Kraków (January 2025) confirms that Silero VAD pre-processing yields **"a significant reduction in WER results, as well as the incidence of hallucinations."** The approach is simple: if Silero says no speech, never invoke Whisper. This completely eliminates phantom transcriptions from silence and most non-speech audio.

Both faster-whisper and whisper.cpp now integrate Silero VAD natively. faster-whisper enables it with `vad_filter=True` and applies padded speech extraction before Whisper inference. whisper.cpp added native Silero VAD support in v1.7+ with a GGML-converted model. For your dictation app, you run your own VAD pipeline upstream and only send confirmed speech segments to Whisper, achieving the same effect.

**Layer 2 is Whisper parameter tuning.** The most impactful settings for dictation:

- **`condition_on_previous_text = False`** — prevents cascading hallucinations where one phantom output poisons the next window's prompt. WhisperX uses this default. For dictation (short utterances), cross-window context matters less than hallucination prevention.
- **`beam_size = 1`** — the AGH paper found that beam_size=1 produces the **lowest hallucination rate**. Higher beam sizes increase hallucinations.
- **`no_speech_threshold = 0.3`** — lower than the default 0.6. This tells Whisper to treat a segment as silence if there is even a 30% no-speech probability combined with low average log probability. An OpenAI developer acknowledged the default 0.6 "worked okay for a few datasets" but is not optimal for all audio.
- **`temperature = 0.0`** with no fallback — eliminates randomness. Higher temperatures increase hallucination risk.

**Layer 3 is post-processing filtering.** Whisper's hallucinations are training-data artifacts from YouTube subtitles. The most common phantoms are highly predictable: "Thank you for watching," "Please subscribe," "Subtitles by the Amara.org community," "Copyright WDR 2021," "Untertitel im Auftrag des ZDF," and "Sous-titres réalisés par la communauté d'Amara.org." The HuggingFace dataset `sachaarbonel/whisper-hallucinations` contains **7,890 catalogued hallucination phrases** organized by language.

For each transcription segment, check three signals: if `no_speech_prob > 0.5`, discard; if `avg_logprob < -1.0`, discard; if `compression_ratio > 2.4`, discard (catches repetitive loops). Then run the text against a blocklist of known hallucination strings using simple regex or Aho-Corasick matching. This multi-layer approach — VAD gate, tuned parameters, post-processing filter — effectively eliminates phantom transcriptions in practice.

---

## How production dictation apps solve this problem

**Wispr Flow** sidesteps continuous VAD entirely by using a **hold-to-talk interface** — press a hotkey, speak, release. The system processes the entire audio blob on key release. End-of-speech is user-signaled, not algorithmically detected. This is the simplest reliable approach but limits hands-free use.

**Google Speech-to-Text V2** uses proprietary internal endpointer models deeply integrated into the recognition pipeline. Their streaming API exposes `SPEECH_ACTIVITY_BEGIN` and `SPEECH_ACTIVITY_END` events, configurable `speech_start_timeout` and `speech_end_timeout` (both 500ms–60s range), and endpointing sensitivity modes including `SUPERSHORT` for commands like "Yes" or "Stop." The endpointer is not a separate pre-processing step — it is fused with the recognition model.

**Deepgram** combines two complementary systems: audio-level **VAD endpointing** (configurable silence duration, default 10ms) and transcript-level **utterance_end** detection (analyzes word-timing gaps, recommended ≥1000ms). The dual approach handles noisy environments where audio-level VAD fails — if background noise prevents `speech_final` from firing, the word-timing-based `UtteranceEnd` acts as fallback.

**faster-whisper's VadOptions** provides the most relevant reference implementation. Its defaults are conservative for batch processing: `min_silence_duration_ms=2000`, `speech_pad_ms=400`, `window_size_samples=1024`. For real-time dictation, reduce `min_silence_duration_ms` to **600–1000ms** and `speech_pad_ms` to **200–300ms**. The dual-threshold system (`threshold=0.5`, `neg_threshold=0.35`) with hysteresis is the standard pattern across faster-whisper, LiveKit, and Pipecat.

Open-source dictation tools largely converge on the same architecture. **LinuxWhispr** uses faster-whisper with Silero VAD and configurable `silence_duration=2.0s`. **Handy** uses Silero VAD for silence filtering. **Nerd Dictation** uses VOSK with manual hotkeys and optional timeout. The consistent pattern: Silero VAD for speech detection, configurable silence timeout, padding around speech segments, and Whisper/VOSK for transcription of confirmed speech only.

---

## Ranked recommendations with integration estimates

### Recommendation 1: Silero VAD v5 via ONNX (strongly recommended)

This is the right choice for almost all dictation applications. It provides the best balance of accuracy, latency, cross-platform support, and open-source licensing.

- **Rust**: `voice_activity_detector` v0.2.1 (bundles Silero v5, depends on `ort`)
- **Android**: `android-vad:silero:2.0.10` via JitPack (bundles ONNX Runtime Mobile)
- **Model**: Same `silero_vad.onnx` (~2MB), MIT license
- **Pros**: Best speech/music discrimination among open options; probability output enables smooth hysteresis; <1ms per frame; proven in faster-whisper, whisper.cpp, LiveKit, Pipecat; same model on both platforms
- **Cons**: ONNX Runtime adds ~5–15MB to binary size; requires frame buffering for 512-sample alignment; `voice_activity_detector` crate is relatively young
- **Integration effort**: ~12–16 hours for Rust (ring buffer, high-pass filter, state machine, Silero integration, testing), ~8–12 hours for Kotlin (android-vad handles most complexity), ~4–6 hours for Whisper hallucination filtering. **Total: ~24–34 hours.**

### Recommendation 2: Silero VAD + WebRTC VAD hybrid

Use WebRTC as a fast pre-filter before Silero. If WebRTC mode 3 says no speech, skip Silero inference entirely. This saves CPU on mobile devices where ONNX overhead matters.

- **Rust**: `webrtc-vad` v0.4.0 + `voice_activity_detector` v0.2.1
- **Android**: `android-vad:webrtc:2.0.10` + `android-vad:silero:2.0.10`
- **Pros**: Reduces Silero inference calls by ~40–60% in quiet environments; WebRTC adds negligible latency; defense-in-depth
- **Cons**: WebRTC crate unmaintained since 2019; added complexity of two VAD engines; marginal benefit if CPU is not constrained
- **Integration effort**: Add ~4–6 hours to Recommendation 1 for WebRTC integration and fallback logic. **Total: ~28–40 hours.**

### Recommendation 3: WebRTC VAD only (lightweight fallback)

If ONNX Runtime's binary size (~15MB) is unacceptable or you need a quick initial implementation to iterate on, WebRTC VAD mode 3 with aggressive hysteresis and high-pass filtering provides a functional baseline.

- **Rust**: `webrtc-vad` v0.4.0
- **Android**: `android-vad:webrtc:2.0.10`
- **Pros**: Tiny footprint (158KB); microsecond latency; no ML runtime; simple API
- **Cons**: Poor speech/music discrimination — will trigger on headphone music bleed; binary output makes smooth hysteresis harder; unmaintained Rust crate; will require aggressive post-processing and Whisper-level hallucination filtering to compensate
- **Integration effort**: ~8–12 hours for Rust, ~6–8 hours for Android. **Total: ~14–20 hours.** But expect to spend additional time debugging false positives from music.

---

## Conclusion

The dictation VAD problem reduces to one core decision: **Silero VAD v5 via ONNX Runtime is the production-grade solution**, and every serious open-source and commercial implementation has converged on it. The `voice_activity_detector` Rust crate and `android-vad` Android library provide clean, maintained wrappers. The 85Hz high-pass filter addresses your specific headphone-bleed scenario at near-zero cost. The dual-threshold state machine (onset 0.5, offset 0.35, ~600ms hangover) is the industry-standard pattern validated across faster-whisper, LiveKit, and whisper.cpp.

The finding that Silero VAD rejects music with vocals — confirmed by multiple user reports — directly solves your headphone music bleed problem without custom model training. Combined with the three-layer Whisper hallucination strategy (VAD gate → tuned parameters → blocklist post-filter), this pipeline should eliminate the "ZDF 2020" phantom transcriptions that plague energy-threshold approaches. Start with Recommendation 1, tune the silence hangover duration to match your users' dictation cadence (600ms for rapid dictation, 1000ms+ for thoughtful composition), and iterate from there.