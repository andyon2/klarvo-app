use crate::traits::AudioBuffer;

/// Phase-1 inter-stage data carrier.
///
/// StageData is the Phase-1 inter-stage data carrier — `Text` and `Audio` variants cover
/// the three Phase-1 stage-types (Stt: Audio → Text, Cleanup: Text → Text, Passthrough:
/// identity over both variants). CleanupInput is not a distinct variant — it is constructed
/// dispatch-arm-internal via `CleanupInput::from_raw(String)` with `CleanupContext::default()`
/// (Phase-1-amendment to 1A.6; Pipeline-Config-passed-CleanupContext is Epic 4 / Phase-2+ scope).
/// Additional variants (offline-whisper-intermediate-representations, multi-language-token-streams)
/// are Phase-2+ extensions via additive enum-grow per `memory/feedback_manifest_compile_contract`
/// no-wildcard-match discipline.
#[derive(Debug, Clone)]
pub enum StageData {
    /// UTF-8 text produced by an Stt stage or consumed/produced by a Cleanup or Passthrough stage.
    Text(String),
    /// Full-utterance PCM audio buffer consumed by an Stt stage.
    Audio(AudioBuffer),
}

impl StageData {
    /// Returns the type discriminator string used for Type-Chaining-Compat-Check error reporting.
    ///
    /// Returns `"text"` for [`StageData::Text`] and `"audio"` for [`StageData::Audio`].
    pub fn type_name(&self) -> &'static str {
        match self {
            StageData::Text(_) => "text",
            StageData::Audio(_) => "audio",
        }
    }
}
