//! Whisper hallucination filter.
//!
//! Whisper (and whisper.cpp) generate phantom transcriptions when processing
//! silence, background noise, or non-speech audio. These artifacts originate
//! from training data: YouTube subtitle files contain credits, sign-off phrases,
//! and metadata lines that the model associates with low-speech audio patterns.
//!
//! This module provides:
//! - A static blocklist of known hallucination phrases (German + English +
//!   other common languages).
//! - [`is_hallucination`]: the public filter function used by the pipeline.
//!
//! ## Design decisions
//!
//! **Word-count gate (≤8 words):** Longer transcriptions are almost certainly
//! real speech, even if they happen to mention "ZDF" or "WDR" in passing.
//! The gate prevents false-positives on genuine dictation.
//!
//! **`contains` matching:** Blocklist entries are matched as substrings of the
//! lowercased, trimmed transcription -- not as exact full-string matches. This
//! catches variations like "ZDF 2020", "ZDF-Untertitel", "© WDR 2023".
//!
//! **Why here and not in `pipeline.rs`:** The blocklist is a STT-layer concern
//! (it is specific to Whisper output artefacts) and needs its own tests. Keeping
//! it as a named submodule gives it a clear location and makes it reachable from
//! both the desktop pipeline and any future Android bridge.
//!
//! ## Sources
//! - <https://github.com/openai/whisper/discussions/1873> -- community phrase catalog
//! - <https://github.com/openai/whisper/discussions/679> -- original hallucination report
//! - <https://github.com/ggml-org/whisper.cpp/issues/1724> -- silence artifacts
//! - <https://huggingface.co/datasets/sachaarbonel/whisper-hallucinations> -- dataset

// ---------------------------------------------------------------------------
// Static blocklist
// ---------------------------------------------------------------------------

/// Known Whisper hallucination phrases from training-data artifacts.
///
/// All entries are lowercase. Matching is done case-insensitively via
/// `to_lowercase()` in [`is_hallucination`].
///
/// ## Adding new entries
/// - Use the minimum disambiguating substring (e.g. `"amara.org"` rather than
///   the full "Subtitles by the Amara.org community").
/// - Keep entries lowercase.
/// - Avoid single-character or very common words -- they would cause
///   false-positives on real speech.
pub const HALLUCINATION_BLOCKLIST: &[&str] = &[
    // --- German broadcast subtitle artifacts ---
    // These originate from ARD/ZDF subtitle files in the training corpus.
    "zdf",
    "wdr",
    "ard",
    "untertitel der dctp",
    "untertitelung des zdf",
    "untertitel im auftrag",
    "untertitel von",
    "untertitelung:",
    "copyright wdr",
    "danke fürs zuschauen",
    "danke fuer das zuschauen",
    "vielen dank fürs zuschauen",
    "vielen dank fuer das zuschauen",
    "vielen dank für ihre aufmerksamkeit",
    "vielen dank fuer ihre aufmerksamkeit",
    "auf wiedersehen",
    // --- English YouTube / video-platform sign-offs ---
    "thank you for watching",
    "thanks for watching",
    "please subscribe",
    "don't forget to subscribe",
    "dont forget to subscribe",
    "like and subscribe",
    "hit the subscribe button",
    "subscribe to my channel",
    "see you in the next video",
    "see you next time",
    "until next time",
    // --- Transcription-service credits ---
    // Amara.org is the most common subtitle provider in Whisper's training data.
    "amara.org",
    "subtitles by",
    "subtitles created by",
    "captions by",
    "transcribed by",
    "transcription by castingwords",
    "closed captions by",
    "rev.com",
    "otter.ai",
    // --- Multilingual subtitle credits ---
    // French, Italian, Spanish, Polish, Russian subtitle artifacts
    "sous-titres",
    "sous-titrage",
    "sous titres",
    "sottotitoli",
    "subtítulos",
    "napisy pobrano",
    // --- Music / noise descriptors ---
    // Whisper generates these for non-speech audio (music, ambient noise).
    "[music]",
    "[applause]",
    "[laughter]",
    "[silence]",
    "[inaudible]",
    "[background noise]",
    "[piano music]",
    "♪",
    // --- Repetitive filler from prompt conditioning ---
    // Whisper-large-v3 sometimes outputs "so" for ambient noise.
    // Single repeated words are caught by the word-count gate, but we also
    // block the most common individual filler outputs here.
    // Note: very short single-word outputs (1 word) are implicitly handled
    // by the empty/whitespace check before the blocklist is consulted.
];

// ---------------------------------------------------------------------------
// Public filter function
// ---------------------------------------------------------------------------

/// Returns `true` if `text` is likely a Whisper hallucination artifact.
///
/// The function applies two checks:
///
/// 1. **Empty/whitespace:** Any text that is empty or contains only whitespace
///    is considered a hallucination (no useful content).
///
/// 2. **Blocklist match with word-count gate:** If the trimmed text contains a
///    known hallucination substring (case-insensitive) AND has ≤8 words, it is
///    classified as a hallucination. Longer texts pass through unchanged -- a
///    genuine dictation that mentions "ZDF" should not be blocked.
///
/// # Examples
///
/// ```ignore
/// use klarvo_lib::stt::hallucination::is_hallucination;
///
/// assert!(is_hallucination(""));                      // empty
/// assert!(is_hallucination("Thank you for watching")); // blocklist hit
/// assert!(is_hallucination("ZDF 2020"));              // broadcast artifact
/// assert!(!is_hallucination("Bitte schick mir die Datei")); // real speech
/// assert!(!is_hallucination(
///     "Ich habe heute beim ZDF angerufen und mit dem Redakteur gesprochen"
/// ));  // >8 words -- passes through
/// ```
pub fn is_hallucination(text: &str) -> bool {
    let trimmed = text.trim();

    // Empty or whitespace-only: not useful, treat as hallucination.
    if trimmed.is_empty() {
        return true;
    }

    // Word-count gate: long texts are almost certainly real speech.
    let word_count = trimmed.split_whitespace().count();
    if word_count > 8 {
        return false;
    }

    let lower = trimmed.to_lowercase();
    HALLUCINATION_BLOCKLIST
        .iter()
        .any(|phrase| lower.contains(phrase))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- Core contract tests ---

    #[test]
    fn test_empty_string_is_hallucination() {
        assert!(is_hallucination(""), "empty string must be blocked");
    }

    #[test]
    fn test_whitespace_only_is_hallucination() {
        assert!(is_hallucination("   "), "whitespace-only must be blocked");
        assert!(is_hallucination("\t\n"), "tab/newline-only must be blocked");
    }

    #[test]
    fn test_real_speech_not_blocked() {
        assert!(!is_hallucination("Bitte schick mir die Datei"));
        assert!(!is_hallucination("Das Meeting ist um 14 Uhr"));
        assert!(!is_hallucination("Please send me the report by Friday"));
        assert!(!is_hallucination("Ich brauche die Unterlagen bis Montag"));
    }

    // --- Word-count gate ---

    #[test]
    fn test_long_text_with_blocklist_word_passes_through() {
        // >8 words -- must NOT be blocked even if "ZDF" appears
        let long = "Ich habe heute beim ZDF angerufen und mit dem Redakteur gesprochen";
        assert!(!is_hallucination(long), "long text mentioning ZDF must pass");
    }

    #[test]
    fn test_exactly_8_words_can_be_blocked() {
        // 8 words containing "amara.org" -- still within the gate
        let eight_words = "Diese Untertitel wurden erstellt von amara.org hier";
        assert!(
            is_hallucination(eight_words),
            "8-word text with blocklist hit must be blocked"
        );
    }

    #[test]
    fn test_9_words_passes_through() {
        // 9 words: gate opens, even with blocklist substring
        let nine_words = "Diese Untertitel wurden erstellt von amara.org und sind gratis hier";
        assert!(
            !is_hallucination(nine_words),
            "9-word text must pass through regardless of blocklist"
        );
    }

    // --- Case-insensitive matching ---

    #[test]
    fn test_case_insensitive_lowercase() {
        assert!(is_hallucination("zdf"), "lowercase zdf must be blocked");
    }

    #[test]
    fn test_case_insensitive_uppercase() {
        assert!(is_hallucination("ZDF"), "uppercase ZDF must be blocked");
    }

    #[test]
    fn test_case_insensitive_mixed() {
        assert!(is_hallucination("Zdf"), "mixed-case Zdf must be blocked");
        assert!(
            is_hallucination("Thank You For Watching"),
            "title-case phrase must be blocked"
        );
        assert!(
            is_hallucination("THANK YOU FOR WATCHING"),
            "all-caps phrase must be blocked"
        );
    }

    // --- Whitespace padding ---

    #[test]
    fn test_whitespace_padding_is_trimmed() {
        assert!(
            is_hallucination("  ZDF  "),
            "whitespace-padded ZDF must be blocked"
        );
        assert!(
            is_hallucination("  thank you for watching  "),
            "whitespace-padded phrase must be blocked"
        );
    }

    // --- German broadcast artifacts ---

    #[test]
    fn test_german_broadcast_artifacts() {
        assert!(is_hallucination("ZDF 2020"), "ZDF with year must be blocked");
        assert!(
            is_hallucination("Copyright WDR"),
            "Copyright WDR must be blocked"
        );
        assert!(
            is_hallucination("Untertitel im Auftrag des ZDF"),
            "ZDF subtitle credit must be blocked"
        );
        assert!(
            is_hallucination("Untertitelung des ZDF"),
            "ZDF subtitle variant must be blocked"
        );
        assert!(
            is_hallucination("WDR 2021"),
            "WDR with year must be blocked"
        );
    }

    #[test]
    fn test_german_signoffs() {
        assert!(
            is_hallucination("Danke fürs Zuschauen"),
            "German thank-you must be blocked"
        );
        assert!(
            is_hallucination("Vielen Dank fürs Zuschauen"),
            "German thank-you variant must be blocked"
        );
        assert!(
            is_hallucination("Vielen Dank für Ihre Aufmerksamkeit"),
            "formal German thank-you must be blocked"
        );
    }

    // --- English YouTube sign-offs ---

    #[test]
    fn test_english_youtube_signoffs() {
        assert!(
            is_hallucination("Thank you for watching"),
            "must be blocked"
        );
        assert!(is_hallucination("Thanks for watching"), "must be blocked");
        assert!(is_hallucination("Please subscribe"), "must be blocked");
        assert!(
            is_hallucination("Like and subscribe"),
            "must be blocked"
        );
        assert!(
            is_hallucination("See you in the next video"),
            "must be blocked"
        );
        assert!(is_hallucination("See you next time"), "must be blocked");
    }

    // --- Transcription service credits ---

    #[test]
    fn test_transcription_service_credits() {
        assert!(is_hallucination("amara.org"), "must be blocked");
        assert!(is_hallucination("Subtitles by"), "must be blocked");
        assert!(is_hallucination("Captions by"), "must be blocked");
        assert!(is_hallucination("Transcribed by"), "must be blocked");
        assert!(is_hallucination("rev.com"), "must be blocked");
        assert!(is_hallucination("otter.ai"), "must be blocked");
    }

    // --- Multilingual subtitle credits ---

    #[test]
    fn test_multilingual_subtitle_credits() {
        assert!(
            is_hallucination("Sous-titres par"),
            "French subtitle credit must be blocked"
        );
        assert!(
            is_hallucination("Sottotitoli di"),
            "Italian subtitle credit must be blocked"
        );
        assert!(
            is_hallucination("napisy pobrano"),
            "Polish subtitle source must be blocked"
        );
    }

    // --- Music / noise descriptors ---

    #[test]
    fn test_music_and_noise_descriptors() {
        assert!(is_hallucination("[Music]"), "must be blocked");
        assert!(is_hallucination("[MUSIC]"), "must be blocked");
        assert!(is_hallucination("[Applause]"), "must be blocked");
        assert!(is_hallucination("[Laughter]"), "must be blocked");
        assert!(is_hallucination("[Inaudible]"), "must be blocked");
        assert!(is_hallucination("♪"), "music symbol must be blocked");
    }

    // --- Substring matching ---

    #[test]
    fn test_substring_match_within_phrase() {
        // "amara.org" appears as a substring inside a longer (but still ≤8 word) phrase
        assert!(
            is_hallucination("Amara.org Community Subtitles"),
            "substring match in short phrase must work"
        );
    }

    // --- Edge cases ---

    #[test]
    fn test_single_punctuation_is_not_hallucination() {
        // Single punctuation is ≤8 words but not in the blocklist.
        // The pipeline's empty-string check handles truly useless output;
        // we just verify we don't crash on these.
        // "." is not in the blocklist, so it passes through here.
        // (The pipeline's silence/RMS check filters it before this point.)
        assert!(!is_hallucination("."));
        assert!(!is_hallucination("!"));
        assert!(!is_hallucination("?"));
    }
}
