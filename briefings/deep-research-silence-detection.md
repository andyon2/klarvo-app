# Deep Research: Silence Detection for Voice Dictation

## System Prompt

```
You are a senior audio engineering researcher specializing in Voice Activity Detection (VAD) and real-time audio processing. You have deep expertise in:

- Speech vs. non-speech classification algorithms
- Real-time audio analysis on resource-constrained devices
- Cross-platform audio APIs (Windows WASAPI/cpal, Android AudioRecord)
- Whisper and other STT systems' behavior with silence/noise

Your task is to research and recommend a robust silence detection strategy for a voice dictation application. The application needs to reliably detect when the user has stopped speaking, even in challenging audio environments (background music, ambient noise, typing sounds).

Be specific and technical. Include concrete algorithm recommendations with parameters, library/crate suggestions with version numbers, and code-level integration guidance. Cite sources where possible. Prioritize solutions that are:
1. Proven in production (not experimental)
2. Lightweight enough for real-time use (<10ms per frame)
3. Available as libraries for both Rust and Kotlin/Java
4. Well-documented with active maintenance
```

## Analysis Prompt

```
# Research Question

How should a voice dictation app implement robust silence/end-of-speech detection that works reliably across these scenarios:

1. **Normal dictation** — User speaks into laptop mic, room is quiet
2. **Music through headphones** — User wears in-ear headphones playing music; the mic picks up bleed-through. Current system detects music as "not silence" and never triggers auto-stop
3. **Background noise** — Office environment, fan noise, keyboard typing
4. **Whisper hallucination** — When actual silence occurs, Whisper STT sometimes generates phantom text ("ZDF 2020", "Copyright WDR") instead of returning empty

## Current Implementation (broken)

We use a simple **RMS energy threshold**:

```rust
// Every ~66ms audio chunk:
let rms = compute_rms(&samples); // sqrt(mean(samples^2))
if rms < threshold {
    silent_chunk_count += 1;
} else {
    silent_chunk_count = 0;
}
if silent_chunk_count >= required_silent_chunks {
    trigger_stop(); // fires silence callback
}
```

**Parameters:**
- Sample rate: 16kHz mono (PCM i16)
- Chunk size: ~66ms (~1066 samples)
- Default threshold: 0.005 RMS
- Default silence duration: 2.0 seconds (configurable)

**Problems:**
- Music bleed-through from headphones has RMS well above 0.005 → silence never detected
- Raising the threshold causes normal speech to be rejected as "silence"
- A single RMS number cannot distinguish speech from music or ambient noise
- No hysteresis: the counter resets to 0 on ANY above-threshold chunk, even a brief noise spike

## Technical Constraints

| Constraint | Detail |
|-----------|--------|
| **Platform 1** | Windows — Rust, audio via `cpal` crate, 16kHz mono PCM |
| **Platform 2** | Android — Kotlin, `AudioRecord` API, 16kHz mono PCM |
| **Latency budget** | <10ms processing per 66ms chunk (real-time, on audio thread) |
| **CPU budget** | Must run alongside Whisper STT (which may use GPU) |
| **No training** | Cannot train custom models; must use pre-trained or algorithmic approaches |
| **Offline option** | Solution should work without internet (some users run fully offline) |
| **Binary size** | Prefer lightweight; ONNX runtime is acceptable if needed |

## What I Need You to Research

### 1. Voice Activity Detection (VAD) Approaches
Compare these approaches for our use case:
- **WebRTC VAD** (libwebrtc) — the classic. Quality, latency, availability in Rust/Kotlin?
- **Silero VAD** — ONNX-based neural VAD. Quality vs. resource usage? Rust integration path?
- **Spectral feature-based** (zero-crossing rate + spectral centroid + energy) — can this distinguish speech from music?
- **rnnoise** — Mozilla's RNN-based noise suppression. Does it help with VAD?
- **Any other proven approach** I'm missing

### 2. Integration Architecture
For the recommended approach:
- How does it integrate with a 16kHz mono PCM stream in real-time?
- What's the frame size / hop size?
- How do we handle the transition from "speaking" to "silence" (hysteresis, hangover time)?
- Should we pre-filter (high-pass to remove low-freq music bleed)?

### 3. Cross-Platform Implementation
- **Rust crates** available? (webrtc-vad, silero-vad, voice-activity-detector, etc.)
- **Kotlin/Android libraries** available? (WebRTC AAR, Silero ONNX, etc.)
- Can we share the same model/algorithm on both platforms?

### 4. Whisper Hallucination Mitigation
Separately from VAD: Whisper generates phantom transcriptions on silence. Our current mitigation is a prompt-echo filter. Research:
- Does running VAD *before* sending audio to Whisper help? (Skip STT if VAD says "no speech")
- Are there known Whisper prompt strategies to reduce hallucinations on silence?
- Does `no_speech_threshold` / `logprob_threshold` in the Whisper API help?

### 5. Production Examples
- How does **Whispr Flow** handle end-of-speech detection?
- How does **Google Voice Typing** / **Apple Dictation** handle it?
- How does **Otter.ai** or **Deepgram** handle streaming end-of-utterance?
- Any open-source dictation tools with good silence detection?

## Deliverable

A ranked recommendation of 2-3 approaches with:
- Pros/cons for each
- Estimated integration effort (hours)
- Specific library names + versions
- Sample integration pseudocode
- Recommended default parameters for our use case
```
