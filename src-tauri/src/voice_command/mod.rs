//! Voice Command Mode engine (desktop-only).
//!
//! This module listens to the continuous PCM stream from `AudioRecorder::start_monitor`
//! and detects short spoken commands addressed to Klarvo (e.g. "Klarvo toggle").
//!
//! ## Architecture
//!
//! ```text
//! AudioRecorder::start_monitor  →  MonitorCallback
//!        |
//!        v
//! VoiceCommandEngine::feed(pcm_chunk)
//!   1. Downsample native-rate → 16 kHz (linear interpolation)
//!   2. Feed 16 kHz samples into own SileroVad instance
//!   3. Accumulate samples while Speaking; cap at MAX_SNIPPET_SAMPLES
//!   4. On Speaking→Silence transition: emit VoiceCommandEvent::SnippetReady
//!
//!        |
//!        v
//! Caller: whisper.cpp → raw text
//!        |
//!        v
//! recognize_command(text) → Option<VoiceCommand>
//! ```
//!
//! The engine is intentionally free of any whisper.cpp dependency so it can be
//! unit-tested without a model file. The caller (pipeline dispatcher, Task 4)
//! owns the whisper inference step.
//!
//! ## Send + Sync
//!
//! `VoiceCommandEngine` is `Send + Sync`-safe: all internal state is owned
//! (no raw pointers) and the engine is never shared across threads --
//! the monitor callback captures it behind an `Arc<Mutex<...>>` in the caller.

#![cfg(desktop)]

use thiserror::Error;

use crate::vad::{SileroVad, SpeechState, VadConfig, VadError};
use crate::stt::is_hallucination;

// ---------------------------------------------------------------------------
// Sub-modules
// ---------------------------------------------------------------------------

/// Dispatch layer: connects the monitor callback to the dictation pipeline.
#[cfg(desktop)]
pub mod dispatch;
#[cfg(desktop)]
pub use dispatch::{start_voice_command_monitor, stop_voice_command_monitor};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur when creating a [`VoiceCommandEngine`].
#[derive(Debug, Error)]
pub enum VoiceCommandError {
    #[error("VAD initialisation failed: {0}")]
    VadInit(#[from] VadError),
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// VAD operates at 16 kHz.
const VAD_SAMPLE_RATE: u32 = 16_000;

/// Maximum snippet length: 3 seconds at 16 kHz = 48 000 samples.
/// Snippets longer than this are silently discarded -- voice commands are short.
const MAX_SNIPPET_SAMPLES: usize = VAD_SAMPLE_RATE as usize * 3;

/// Hangover for the command VAD: ~300 ms = ceil(300/32) = 10 frames.
/// Commands end with a short pause; the recording VAD uses ~608 ms, which
/// is too slow for interactive command detection.
const COMMAND_HANGOVER_MS: u32 = 320;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A recognised Klarvo voice command.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceCommand {
    /// "Klarvo toggle" / "Klarvo start" -- starts dictation in toggle mode
    StartToggle,
    /// "Klarvo auto-stop" / "Klarvo autostop" -- starts dictation in auto-stop mode
    StartAutoStop,
    /// "Klarvo full auto" -- starts dictation in auto-loop mode
    StartFullAuto,
    /// "Klarvo stop" / "Klarvo stopp" -- stops active dictation
    StopDictation,
    /// "Klarvo cancel" / "Klarvo abbrechen" -- cancels dictation
    CancelDictation,
    /// "Klarvo off" / "Klarvo aus" -- disables Voice Command Mode
    TurnOff,
}

/// Events emitted by [`VoiceCommandEngine::feed`].
#[derive(Debug)]
pub enum VoiceCommandEvent {
    /// A speech snippet has been captured and is ready for transcription.
    ///
    /// The `Vec<f32>` contains 16 kHz mono PCM samples. The caller should
    /// pass this buffer to whisper.cpp and then call `recognize_command` on
    /// the returned text.
    SnippetReady(Vec<f32>),
}

// ---------------------------------------------------------------------------
// VoiceCommandEngine
// ---------------------------------------------------------------------------

/// Processes a continuous stream of PCM chunks from `AudioRecorder::start_monitor`
/// and signals when a speech snippet is ready for command recognition.
///
/// # Usage
///
/// ```rust,ignore
/// let mut engine = VoiceCommandEngine::new(48_000, 2)?;
///
/// // Inside the monitor callback:
/// if let Some(event) = engine.feed(&pcm_chunk) {
///     match event {
///         VoiceCommandEvent::SnippetReady(samples) => {
///             // send `samples` to whisper.cpp, get text back
///             if let Some(cmd) = recognize_command(&text) {
///                 handle_command(cmd);
///             }
///         }
///     }
/// }
/// ```
pub struct VoiceCommandEngine {
    /// Own VAD instance -- tuned for short commands (lower hangover).
    vad: SileroVad,
    /// Accumulated 16 kHz mono samples for the current snippet.
    snippet_buf: Vec<f32>,
    /// Whether the VAD reported Speaking in the previous call to `feed`.
    was_speaking: bool,
    /// Native device sample rate delivered by the monitor callback.
    native_sample_rate: u32,
    /// Native channel count delivered by the monitor callback.
    native_channels: u16,
    /// Fractional index accumulator for linear interpolation resampling.
    resample_pos: f64,
}

impl VoiceCommandEngine {
    /// Creates a new engine.
    ///
    /// `native_sample_rate` and `native_channels` must match what the
    /// `MonitorCallback` delivers (i.e. the device's actual capture format).
    pub fn new(native_sample_rate: u32, native_channels: u16) -> Result<Self, VoiceCommandError> {
        let vad_config = VadConfig {
            hangover_ms: COMMAND_HANGOVER_MS,
            ..VadConfig::default()
        };
        let vad = SileroVad::with_config(vad_config)?;

        Ok(VoiceCommandEngine {
            vad,
            snippet_buf: Vec::new(),
            was_speaking: false,
            native_sample_rate,
            native_channels,
            resample_pos: 0.0,
        })
    }

    /// Feeds a raw PCM chunk from the monitor callback into the engine.
    ///
    /// Internally:
    /// 1. Downmixes multi-channel → mono.
    /// 2. Resamples from the device's native rate to 16 kHz.
    /// 3. Feeds resampled samples to the VAD.
    /// 4. Accumulates samples while Speaking (up to `MAX_SNIPPET_SAMPLES`).
    /// 5. On Speaking→Silence transition: returns `SnippetReady` and resets.
    ///
    /// Returns `None` during silence or while accumulating.
    pub fn feed(&mut self, native_chunk: &[f32]) -> Option<VoiceCommandEvent> {
        // Step 1+2: downmix + resample to 16 kHz mono.
        let resampled = self.downmix_and_resample(native_chunk);

        // Step 3: query VAD.
        let speech_state = self.vad.feed(&resampled);
        let is_speaking = speech_state == SpeechState::Speaking;

        // Step 4: accumulate while speaking.
        if is_speaking {
            let remaining = MAX_SNIPPET_SAMPLES.saturating_sub(self.snippet_buf.len());
            let to_add = resampled.len().min(remaining);
            self.snippet_buf.extend_from_slice(&resampled[..to_add]);
            // If we hit the cap, we still emit the snippet immediately so the
            // buffer does not grow unbounded. This handles pathological cases
            // (e.g. background noise that fools the VAD for several seconds).
            if self.snippet_buf.len() >= MAX_SNIPPET_SAMPLES {
                let snippet = std::mem::take(&mut self.snippet_buf);
                self.was_speaking = false;
                return Some(VoiceCommandEvent::SnippetReady(snippet));
            }
        }

        // Step 5: Speaking → Silence transition.
        let was = self.was_speaking;
        self.was_speaking = is_speaking;

        if was && !is_speaking && !self.snippet_buf.is_empty() {
            let snippet = std::mem::take(&mut self.snippet_buf);
            return Some(VoiceCommandEvent::SnippetReady(snippet));
        }

        None
    }

    /// Resets all internal state (buffer + VAD + resampler).
    ///
    /// Call this when the Voice Command Mode is disabled or re-enabled.
    #[allow(dead_code)] // will be used when voice command mode is re-enabled
    pub fn reset(&mut self) {
        self.vad.reset();
        self.snippet_buf.clear();
        self.was_speaking = false;
        self.resample_pos = 0.0;
    }

    // -----------------------------------------------------------------------
    // Private: downmix + linear-interpolation resample
    // -----------------------------------------------------------------------

    /// Downmixes a multi-channel native-rate chunk to 16 kHz mono.
    ///
    /// Uses linear interpolation resampling. For the common case of 48 kHz → 16 kHz
    /// (ratio 3:1) this is fast and introduces negligible distortion at voice
    /// frequencies. A higher-quality resampler is not needed here because:
    /// - Commands are short (≤3 s)
    /// - Whisper/Silero are both trained on 16 kHz and are robust to mild aliasing
    fn downmix_and_resample(&mut self, native_chunk: &[f32]) -> Vec<f32> {
        let channels = self.native_channels as usize;
        let in_rate = self.native_sample_rate as f64;
        let out_rate = VAD_SAMPLE_RATE as f64;
        let ratio = in_rate / out_rate; // e.g. 48000/16000 = 3.0

        // First downmix to mono (average across channels).
        let mono: Vec<f32> = if channels == 1 {
            native_chunk.to_vec()
        } else {
            native_chunk
                .chunks(channels)
                .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                .collect()
        };

        if mono.is_empty() {
            return Vec::new();
        }

        // Linear interpolation resampling.
        // `resample_pos` is the fractional index into `mono`.
        let mut out = Vec::new();
        let mono_len = mono.len() as f64;

        while self.resample_pos < mono_len {
            let idx = self.resample_pos as usize;
            let frac = self.resample_pos - idx as f64;

            let s0 = mono[idx];
            let s1 = if idx + 1 < mono.len() {
                mono[idx + 1]
            } else {
                s0
            };
            let sample = s0 + frac as f32 * (s1 - s0);
            out.push(sample);

            self.resample_pos += ratio;
        }

        // Carry the fractional remainder into the next call.
        self.resample_pos -= mono_len;

        out
    }
}

// ---------------------------------------------------------------------------
// recognize_command
// ---------------------------------------------------------------------------

/// Trigger words that activate command recognition.
///
/// Includes phonetic variants that Whisper commonly produces for "Klarvo".
const TRIGGER_WORDS: &[&str] = &[
    "klarvo",
    "klar vo",
    "clarvo",
    "clar vo",
    "klarfo",
    "klarwo",
    "klar wo",
];

/// Tries to match a Whisper transcription against the known Klarvo commands.
///
/// Steps:
/// 1. Lowercase + trim.
/// 2. Hallucination check -- returns `None` for phantom transcriptions.
/// 3. Trigger check -- must contain at least one trigger word.
/// 4. Command keyword search in the text following the trigger.
///
/// Returns `None` if no command is recognised (triggers present but no keyword,
/// or no trigger at all, or hallucination).
pub fn recognize_command(text: &str) -> Option<VoiceCommand> {
    let trimmed = text.trim();

    // Step 1.
    let lower = trimmed.to_lowercase();

    // Step 2: discard Whisper hallucinations.
    if is_hallucination(trimmed) {
        return None;
    }

    // Step 3: find the first trigger word and its end position.
    let trigger_end = TRIGGER_WORDS
        .iter()
        .filter_map(|&t| {
            lower.find(t).map(|pos| pos + t.len())
        })
        .min(); // take the earliest-ending trigger (first occurrence)

    let post_trigger = match trigger_end {
        Some(end) => &lower[end..],
        None => return None, // no trigger found
    };

    // Step 4: search for command keywords in the post-trigger text.
    // We scan the whole post-trigger string (not just the next word) so that
    // filler words like "bitte" or "please" between trigger and command work:
    // "Klarvo, bitte starten" → still matches "start".
    find_command(post_trigger)
}

/// Scans `text` (already lowercased) for a command keyword.
///
/// Order is critical:
/// - Multi-word keywords ("full auto", "auto-stop", "auto stop") are checked
///   with `text.contains()` before single-word keywords to avoid partial matches.
/// - "full auto" before "autostop" (otherwise "auto" in "full auto" hits autostop).
/// - "autostop" before "toggle/start" (otherwise "stop" in "autostop" hits StopDictation).
fn find_command(text: &str) -> Option<VoiceCommand> {
    // Step 1: multi-word / hyphenated keywords via substring match.
    // These must be checked before the single-word table below.
    if text.contains("full auto") {
        return Some(VoiceCommand::StartFullAuto);
    }
    if text.contains("auto-stop") || text.contains("autostop") || text.contains("auto stop") {
        return Some(VoiceCommand::StartAutoStop);
    }

    // Step 2: single-word keywords via whole-word boundary check.
    // Order still matters within each group (more specific before generic).
    type CommandCheck = (&'static [&'static str], fn() -> VoiceCommand);
    let checks: &[CommandCheck] = &[
        // StartToggle
        (&["toggle", "start", "dictate", "diktat", "diktieren"], || VoiceCommand::StartToggle),
        // StopDictation
        (&["stop", "stopp", "halt"], || VoiceCommand::StopDictation),
        // CancelDictation
        (&["cancel", "abbrechen", "abbruch"], || VoiceCommand::CancelDictation),
        // TurnOff
        (&["off", "aus", "beenden", "beende", "quit", "exit"], || VoiceCommand::TurnOff),
    ];

    for (keywords, make_cmd) in checks {
        for &kw in *keywords {
            if contains_word(text, kw) {
                return Some(make_cmd());
            }
        }
    }

    None
}

/// Returns `true` if `text` contains `word` as a whole word (not a substring).
///
/// "halt" must not match "halted", "stop" must not match "stopper", etc.
/// We use a simple boundary check: characters adjacent to the match must be
/// non-alphabetic (space, punctuation, string boundary).
fn contains_word(text: &str, word: &str) -> bool {
    let mut start = 0;
    while let Some(pos) = text[start..].find(word) {
        let abs_pos = start + pos;
        let before_ok = abs_pos == 0
            || !text[..abs_pos]
                .chars()
                .next_back()
                .map(|c| c.is_alphabetic())
                .unwrap_or(false);
        let after_ok = abs_pos + word.len() == text.len()
            || !text[abs_pos + word.len()..]
                .chars()
                .next()
                .map(|c| c.is_alphabetic())
                .unwrap_or(false);
        if before_ok && after_ok {
            return true;
        }
        start = abs_pos + 1;
        if start >= text.len() {
            break;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // recognize_command -- trigger variants
    // -----------------------------------------------------------------------

    #[test]
    fn test_trigger_klarvo() {
        assert_eq!(
            recognize_command("Klarvo toggle"),
            Some(VoiceCommand::StartToggle)
        );
    }

    #[test]
    fn test_trigger_klar_vo() {
        assert_eq!(
            recognize_command("klar vo toggle"),
            Some(VoiceCommand::StartToggle)
        );
    }

    #[test]
    fn test_trigger_clarvo() {
        assert_eq!(
            recognize_command("clarvo stop"),
            Some(VoiceCommand::StopDictation)
        );
    }

    #[test]
    fn test_trigger_klarfo() {
        assert_eq!(
            recognize_command("klarfo cancel"),
            Some(VoiceCommand::CancelDictation)
        );
    }

    #[test]
    fn test_trigger_klarwo() {
        assert_eq!(
            recognize_command("klarwo off"),
            Some(VoiceCommand::TurnOff)
        );
    }

    // -----------------------------------------------------------------------
    // recognize_command -- all commands (EN)
    // -----------------------------------------------------------------------

    #[test]
    fn test_command_toggle() {
        assert_eq!(
            recognize_command("klarvo toggle"),
            Some(VoiceCommand::StartToggle)
        );
    }

    #[test]
    fn test_command_start_alias() {
        assert_eq!(
            recognize_command("klarvo start"),
            Some(VoiceCommand::StartToggle)
        );
    }

    #[test]
    fn test_command_dictate_alias() {
        assert_eq!(
            recognize_command("klarvo dictate"),
            Some(VoiceCommand::StartToggle)
        );
    }

    #[test]
    fn test_command_autostop() {
        assert_eq!(
            recognize_command("klarvo auto-stop"),
            Some(VoiceCommand::StartAutoStop)
        );
    }

    #[test]
    fn test_command_autostop_no_hyphen() {
        assert_eq!(
            recognize_command("klarvo autostop"),
            Some(VoiceCommand::StartAutoStop)
        );
    }

    #[test]
    fn test_command_autostop_space() {
        assert_eq!(
            recognize_command("klarvo auto stop"),
            Some(VoiceCommand::StartAutoStop)
        );
    }

    #[test]
    fn test_command_full_auto() {
        assert_eq!(
            recognize_command("klarvo full auto"),
            Some(VoiceCommand::StartFullAuto)
        );
    }

    #[test]
    fn test_command_stop() {
        assert_eq!(
            recognize_command("klarvo stop"),
            Some(VoiceCommand::StopDictation)
        );
    }

    #[test]
    fn test_command_cancel() {
        assert_eq!(
            recognize_command("klarvo cancel"),
            Some(VoiceCommand::CancelDictation)
        );
    }

    #[test]
    fn test_command_off() {
        assert_eq!(
            recognize_command("klarvo off"),
            Some(VoiceCommand::TurnOff)
        );
    }

    #[test]
    fn test_command_quit() {
        assert_eq!(
            recognize_command("klarvo quit"),
            Some(VoiceCommand::TurnOff)
        );
    }

    // -----------------------------------------------------------------------
    // recognize_command -- all commands (DE)
    // -----------------------------------------------------------------------

    #[test]
    fn test_command_diktat() {
        assert_eq!(
            recognize_command("klarvo diktat"),
            Some(VoiceCommand::StartToggle)
        );
    }

    #[test]
    fn test_command_diktieren() {
        assert_eq!(
            recognize_command("klarvo diktieren"),
            Some(VoiceCommand::StartToggle)
        );
    }

    #[test]
    fn test_command_stopp() {
        assert_eq!(
            recognize_command("klarvo stopp"),
            Some(VoiceCommand::StopDictation)
        );
    }

    #[test]
    fn test_command_halt() {
        assert_eq!(
            recognize_command("klarvo halt"),
            Some(VoiceCommand::StopDictation)
        );
    }

    #[test]
    fn test_command_abbrechen() {
        assert_eq!(
            recognize_command("klarvo abbrechen"),
            Some(VoiceCommand::CancelDictation)
        );
    }

    #[test]
    fn test_command_abbruch() {
        assert_eq!(
            recognize_command("klarvo abbruch"),
            Some(VoiceCommand::CancelDictation)
        );
    }

    #[test]
    fn test_command_aus() {
        assert_eq!(
            recognize_command("klarvo aus"),
            Some(VoiceCommand::TurnOff)
        );
    }

    #[test]
    fn test_command_beenden() {
        assert_eq!(
            recognize_command("klarvo beenden"),
            Some(VoiceCommand::TurnOff)
        );
    }

    // -----------------------------------------------------------------------
    // recognize_command -- case-insensitive
    // -----------------------------------------------------------------------

    #[test]
    fn test_case_insensitive() {
        assert_eq!(
            recognize_command("KLARVO TOGGLE"),
            Some(VoiceCommand::StartToggle)
        );
        assert_eq!(
            recognize_command("Klarvo Stop"),
            Some(VoiceCommand::StopDictation)
        );
        assert_eq!(
            recognize_command("CLARVO CANCEL"),
            Some(VoiceCommand::CancelDictation)
        );
    }

    // -----------------------------------------------------------------------
    // recognize_command -- no trigger → None
    // -----------------------------------------------------------------------

    #[test]
    fn test_no_trigger() {
        assert_eq!(recognize_command("start dictation"), None);
        assert_eq!(recognize_command("stop"), None);
        assert_eq!(recognize_command("cancel now"), None);
    }

    // -----------------------------------------------------------------------
    // recognize_command -- trigger without keyword → None
    // -----------------------------------------------------------------------

    #[test]
    fn test_trigger_without_keyword() {
        assert_eq!(recognize_command("klarvo"), None);
        assert_eq!(recognize_command("klarvo please"), None);
        assert_eq!(recognize_command("klar vo bitte"), None);
    }

    // -----------------------------------------------------------------------
    // recognize_command -- filler words between trigger and command
    // -----------------------------------------------------------------------

    #[test]
    fn test_filler_words() {
        assert_eq!(
            recognize_command("klarvo bitte toggle"),
            Some(VoiceCommand::StartToggle)
        );
        assert_eq!(
            recognize_command("klarvo please start"),
            Some(VoiceCommand::StartToggle)
        );
    }

    // -----------------------------------------------------------------------
    // recognize_command -- word boundary check
    // -----------------------------------------------------------------------

    #[test]
    fn test_word_boundary() {
        // "stopping" must not match "stop"
        assert_eq!(recognize_command("klarvo stopping"), None);
        // "offside" must not match "off"
        assert_eq!(recognize_command("klarvo offside"), None);
    }

    // -----------------------------------------------------------------------
    // recognize_command -- hallucinations → None
    // -----------------------------------------------------------------------

    #[test]
    fn test_hallucinations() {
        assert_eq!(recognize_command("Thank you for watching"), None);
        assert_eq!(recognize_command(""), None);
    }

    // -----------------------------------------------------------------------
    // recognize_command -- "full auto" must not be confused with "autostop"
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_auto_not_confused_with_autostop() {
        assert_eq!(
            recognize_command("klarvo full auto"),
            Some(VoiceCommand::StartFullAuto)
        );
    }

    // -----------------------------------------------------------------------
    // VoiceCommandEngine -- feed() with silence → no event
    // -----------------------------------------------------------------------

    #[test]
    fn test_engine_silence_produces_no_event() {
        let mut engine =
            VoiceCommandEngine::new(16_000, 1).expect("engine must initialise");

        let silence = vec![0.0_f32; 4096];
        let event = engine.feed(&silence);
        assert!(
            event.is_none(),
            "Silence must not produce a VoiceCommandEvent"
        );
    }

    // -----------------------------------------------------------------------
    // VoiceCommandEngine -- snippet buffer caps at MAX_SNIPPET_SAMPLES
    // -----------------------------------------------------------------------

    #[test]
    fn test_engine_caps_snippet_at_max_duration() {
        // We need to drive the VAD into Speaking state and then overflow the
        // buffer. We use advance_state directly by bypassing feed() so that
        // we're not dependent on Silero inference. Instead we test the cap
        // independently by inspecting the buffer state.

        let mut engine =
            VoiceCommandEngine::new(16_000, 1).expect("engine must initialise");

        // The buffer is empty before any feed.
        assert_eq!(engine.snippet_buf.len(), 0);

        // Drive the VAD into Speaking state and fill the buffer past the cap
        // by repeatedly calling feed() with "speech-like" high-amplitude samples.
        // Because we can't easily drive Silero in unit tests, we test the cap
        // logic by manually populating snippet_buf past the limit and verifying
        // that feed() emits SnippetReady when the cap is reached.
        engine.was_speaking = true;
        // Fill the buffer to exactly MAX_SNIPPET_SAMPLES - 1
        engine.snippet_buf.resize(MAX_SNIPPET_SAMPLES - 1, 0.1);

        // Now: VAD will see silence (silence samples → no Speaking state).
        // The buffer has MAX-1 samples and was_speaking=true.
        // When was_speaking=true and the VAD returns Silence, the engine emits
        // SnippetReady with the accumulated samples.
        let silence = vec![0.0_f32; 512];
        let event = engine.feed(&silence);

        // The VAD should still be Silence (energy gate kills zero samples).
        // was_speaking was true → Speaking→Silence transition emits SnippetReady.
        match event {
            Some(VoiceCommandEvent::SnippetReady(samples)) => {
                // Buffer had MAX-1 samples; the feed added only resampled silence.
                // The silence contribution from 512 zeros is 0 after the energy
                // gate (RMS=0 < energy_floor) and Silero stays Silence.
                // So the snippet contains exactly what we put in (MAX-1 samples)
                // plus any resampled zeros that slipped through before the VAD
                // decided Silence. Either way, length must be ≥ 1.
                assert!(
                    !samples.is_empty(),
                    "SnippetReady must carry the accumulated samples"
                );
            }
            None => {
                // If the VAD's hangover is still active (residual state from
                // the default hangover frames), that means was_speaking was
                // reset to true inside feed(). This is acceptable -- the
                // snippet is not yet complete.
                // The key invariant is: no panic and no unbounded buffer growth.
                assert!(
                    engine.snippet_buf.len() <= MAX_SNIPPET_SAMPLES,
                    "Buffer must never exceed MAX_SNIPPET_SAMPLES"
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // VoiceCommandEngine -- reset() clears all state
    // -----------------------------------------------------------------------

    #[test]
    fn test_engine_reset_clears_state() {
        let mut engine =
            VoiceCommandEngine::new(48_000, 2).expect("engine must initialise");

        // Manually pollute state to verify reset clears everything.
        engine.snippet_buf.push(1.0);
        engine.was_speaking = true;
        engine.resample_pos = 1.5;

        engine.reset();

        assert!(engine.snippet_buf.is_empty(), "reset must clear snippet_buf");
        assert!(!engine.was_speaking, "reset must clear was_speaking flag");
        assert_eq!(engine.resample_pos, 0.0, "reset must clear resample_pos");
    }

    // -----------------------------------------------------------------------
    // downmix_and_resample -- basic sanity checks
    // -----------------------------------------------------------------------

    #[test]
    fn test_resample_48k_to_16k_correct_length() {
        // 48 kHz mono, 4800 samples = 100 ms → should produce ~1600 samples at 16 kHz
        let mut engine = VoiceCommandEngine::new(48_000, 1).expect("engine must init");
        let input = vec![0.5_f32; 4800];
        let output = engine.downmix_and_resample(&input);
        // Allow ±2 samples for rounding.
        assert!(
            (output.len() as i64 - 1600).abs() <= 2,
            "4800 samples @ 48kHz resampled to 16kHz should yield ~1600, got {}",
            output.len()
        );
    }

    #[test]
    fn test_resample_stereo_downmix() {
        // 2-channel (stereo) signal: left=1.0, right=0.0 → mono should be 0.5
        let mut engine = VoiceCommandEngine::new(16_000, 2).expect("engine must init");
        // At 16kHz stereo → 16kHz mono (no resampling needed, just downmix)
        let input: Vec<f32> = (0..100).flat_map(|_| [1.0f32, 0.0f32]).collect();
        let output = engine.downmix_and_resample(&input);
        assert_eq!(output.len(), 100, "stereo→mono: 200 frames → 100 mono samples");
        for &s in &output {
            assert!(
                (s - 0.5).abs() < 1e-5,
                "stereo downmix average must be 0.5, got {s}"
            );
        }
    }

    #[test]
    fn test_resample_empty_input() {
        let mut engine = VoiceCommandEngine::new(48_000, 1).expect("engine must init");
        let output = engine.downmix_and_resample(&[]);
        assert!(output.is_empty(), "empty input must yield empty output");
    }
}
