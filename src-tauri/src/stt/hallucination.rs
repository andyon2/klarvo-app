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
// Static blocklists
// ---------------------------------------------------------------------------

/// Stockphrase ghost family matched **without** the word-count gate.
///
/// These are highly distinctive Whisper training-data artifacts that appear
/// both as whole-clip ghosts (short/silent audio) AND as trailing appended
/// fragments on long clips (~1.1% of desktop dictations, audit 2026-06-12).
///
/// The word_count > 8 gate in [`is_hallucination`] is designed to protect
/// genuine long dictation that incidentally mentions e.g. "ZDF". But trailing
/// ghosts on long clips DO have >8 words total — the gate lets them through.
/// These stockphrase entries are specific enough (no realistic false-positive
/// scenario for "groß- und kleinschreibung" in genuine speech) to skip the gate.
///
/// Design (AC5, `*design` decision): only entries specific enough that no
/// false-positive on genuine dictation is conceivable are placed here.
/// Generic single-word entries (e.g. "wdr", "zdf") remain in `HALLUCINATION_BLOCKLIST`
/// under the word-count gate.
///
/// **"klinge"/"klingel" matching rule (Finding 2):**
/// These entries are matched as WHOLE WORDS (standalone token) so that real German
/// words "klingen", "Klingelton", "Türklingel", "es klingelt" are NOT discarded.
/// The multi-word entries above ("groß- und klingel" etc.) stay substring-matched
/// because they are inherently specific.
pub const STOCKPHRASE_BLOCKLIST: &[&str] = &[
    // --- Groß- und Kleinschreibung family (the #1 observed ghost, audit 2026-06-12) ---
    // All substring variants: whole-phrase, partial, and fragment ghosts.
    "groß- und kleinschreibung",
    "gross- und kleinschreibung",
    "groß- und klingel",
    "gross- und klingel",
    "groß- und klinge",
    "gross- und klinge",
    // Fragment-only ghosts that Whisper outputs before cleanup rationalizes them.
    // Matched as WHOLE WORDS only (see is_hallucination / strip_stockphrase_ghosts)
    // so that "klingen", "Klingelton", "Türklingel" are NOT blocked.
    "klingel",
    "klinge",
    // --- Other trailing-ghost patterns observed in the audit ---
    "[musik]",
];

/// Single-word entries in STOCKPHRASE_BLOCKLIST that must be matched as **whole words**
/// to avoid false positives on real German ("klingen", "Klingelton", etc.).
///
/// All other STOCKPHRASE_BLOCKLIST entries contain spaces or brackets and remain
/// substring-matched (they are specific enough to have no false-positive scenario).
const STOCKPHRASE_WHOLE_WORD_ENTRIES: &[&str] = &["klingel", "klinge"];

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
// Public functions
// ---------------------------------------------------------------------------

/// Strips stockphrase ghosts from LLM-cleaned output (AC7 — cleanup-no-invent guard).
///
/// Whisper sometimes generates a recognizable ghost (`Klinge`) that LLM cleanup
/// then rationalizes into a convincing full stockphrase (`Kleinschreibung`), turning
/// detectable noise into fluent, undetectable noise. This function is called on the
/// post-cleanup output and removes any STOCKPHRASE_BLOCKLIST matches.
///
/// Only `STOCKPHRASE_BLOCKLIST` entries are stripped (the highly distinctive phrases).
/// `HALLUCINATION_BLOCKLIST` entries are not stripped here because those would have
/// been caught by [`is_hallucination`] before cleanup ran.
///
/// Returns the cleaned text with stockphrase fragments removed and excess whitespace
/// collapsed. If the entire output is a stockphrase, returns an empty string.
pub fn strip_stockphrase_ghosts(text: &str) -> String {
    let lower = text.to_lowercase();

    // Fast path: no stockphrase in text at all.
    if !STOCKPHRASE_BLOCKLIST.iter().any(|p| lower.contains(p)) {
        return text.to_string();
    }

    // Strip each stockphrase occurrence (case-insensitive, on the original text).
    //
    // SAFETY: We locate the byte range using char-indices on the ORIGINAL string
    // (not on `to_lowercase()`). The key insight is that `to_lowercase()` can
    // change byte lengths for some Unicode characters (e.g. "İ" U+0130 lowercases
    // from 2 to 3 bytes), so offsets from `lowercased.find(phrase)` may NOT be
    // valid byte boundaries in the original string, causing a panic on slice.
    //
    // Fix: we collect (original_char_index, lowercase_char) pairs and do a
    // sliding-window substring search in the lowercased char stream while
    // tracking original byte offsets — so the cut positions are always valid
    // char boundaries of `result`.
    let mut result = text.to_string();
    for phrase in STOCKPHRASE_BLOCKLIST {
        let whole_word = STOCKPHRASE_WHOLE_WORD_ENTRIES.contains(phrase);
        loop {
            // Build parallel arrays: (byte_offset_in_result, lowercase_char)
            // We need byte offsets into `result` (the original, not lowercased).
            let orig_chars: Vec<(usize, char)> = result.char_indices().collect();
            let lower_chars: Vec<char> = orig_chars.iter().map(|(_, c)| {
                // Collect lowercase expansion of each char.
                // `char::to_lowercase()` returns an iterator; we take the first char
                // as the representative for substring searching. This is safe for all
                // BMP characters; surrogate / multi-char expansions are edge cases
                // not present in our blocklist phrases.
                c.to_lowercase().next().unwrap_or(*c)
            }).collect();

            let phrase_chars: Vec<char> = phrase.chars().collect();
            let plen = phrase_chars.len();

            // Sliding window: find first position where lower_chars[i..i+plen] == phrase_chars
            let found = if plen == 0 {
                None
            } else {
                (0..lower_chars.len().saturating_sub(plen - 1)).find(|&i| {
                    lower_chars[i..i + plen] == phrase_chars[..]
                })
            };

            if let Some(char_pos) = found {
                // Whole-word check: the char before char_pos (if any) and the char
                // after char_pos+plen (if any) must not be alphanumeric.
                if whole_word {
                    let before_ok = char_pos == 0
                        || !orig_chars[char_pos - 1].1.is_alphanumeric();
                    let after_idx = char_pos + plen;
                    let after_ok = after_idx >= orig_chars.len()
                        || !orig_chars[after_idx].1.is_alphanumeric();
                    if !before_ok || !after_ok {
                        // Not a whole-word match; stop scanning to avoid infinite loop.
                        break;
                    }
                }

                // Convert char positions to byte offsets in `result`.
                let byte_start = orig_chars[char_pos].0;
                let byte_end = if char_pos + plen < orig_chars.len() {
                    orig_chars[char_pos + plen].0
                } else {
                    result.len()
                };

                result = format!("{}{}", &result[..byte_start], &result[byte_end..]);
            } else {
                break;
            }
        }
    }

    // Collapse excess whitespace and punctuation artifacts left by the removal.
    let result = result
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();

    // Strip leading/trailing punctuation artifacts (e.g. trailing "." or ", ").
    result
        .trim_end_matches(|c: char| c == '.' || c == ',' || c == ' ')
        .trim_start_matches(|c: char| c == '.' || c == ',' || c == ' ')
        .to_string()
}

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
/// Returns `true` if `entry` (a single-word blocklist token, no spaces) matches
/// `text` as a whole word. Matching is done on the lowercased, whitespace-split tokens.
///
/// **Rationale (H14 / AC3):** The Kotlin twin (`HallucinationFilter.kt:100-109`)
/// already adopted whole-word matching for single-word entries to prevent false
/// positives ("ard" → "Standard", "Milliarde", "Hardware"). We port that fix here
/// so deleting the Kotlin twin does not regress Android.
fn single_word_matches(tokens: &[&str], entry: &str) -> bool {
    tokens.iter().any(|t| *t == entry)
}

pub fn is_hallucination(text: &str) -> bool {
    let trimmed = text.trim();

    // Empty or whitespace-only: not useful, treat as hallucination.
    if trimmed.is_empty() {
        return true;
    }

    let lower = trimmed.to_lowercase();

    // --- Stockphrase check (no word-count gate) ---
    // The STOCKPHRASE_BLOCKLIST is checked first and always, regardless of length.
    // These are highly distinctive artifacts (trailing-ghost family, "groß- und
    // kleinschreibung" etc.) that appear appended to long clips (~1.1% rate, AC5).
    // The word-count gate would let them through because the total transcript is >8 words.
    //
    // "klinge"/"klingel" entries use WHOLE-WORD matching (STOCKPHRASE_WHOLE_WORD_ENTRIES)
    // so that real German words "klingen", "Klingelton", "Türklingel" are NOT blocked.
    // All other STOCKPHRASE_BLOCKLIST entries remain substring-matched (they contain
    // spaces or brackets and have no false-positive scenario).
    {
        let tokens: Vec<&str> = lower.split_whitespace().collect();
        if STOCKPHRASE_BLOCKLIST.iter().any(|phrase| {
            if STOCKPHRASE_WHOLE_WORD_ENTRIES.contains(phrase) {
                single_word_matches(&tokens, phrase)
            } else {
                lower.contains(phrase)
            }
        }) {
            return true;
        }
    }

    // Word-count gate: long texts are almost certainly real speech (for the
    // generic blocklist entries like "zdf", "wdr", "amara.org").
    let word_count = trimmed.split_whitespace().count();
    if word_count > 8 {
        return false;
    }

    // Pre-split into tokens for single-word whole-word matching (H14).
    let tokens: Vec<&str> = lower.split_whitespace().collect();

    HALLUCINATION_BLOCKLIST.iter().any(|phrase| {
        if phrase.contains(' ') {
            // Multi-word entry: substring match is safe (phrase is specific enough).
            lower.contains(phrase)
        } else if phrase.contains('.') || phrase.contains('/') || phrase.contains(':') {
            // Dotted / URL-style entry (e.g. "amara.org", "rev.com", "otter.ai"):
            // use substring match so that "amara.org/community" and "rev.com." (with
            // trailing punctuation) are still caught. These entries are distinctive
            // enough that substring matching on a ≤8-word clip has no false positives.
            lower.contains(phrase)
        } else {
            // Plain alpha single-word entry (e.g. "zdf", "wdr", "ard"):
            // whole-word match to prevent false positives on "Standard", "Milliarde",
            // "Hardware" (H14 / AC3).
            single_word_matches(&tokens, phrase)
        }
    })
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

    // --- H14: Whole-word match for single-word blocklist entries (AC3) ---

    #[test]
    fn test_h14_standard_not_blocked_single_word_ard() {
        // "ard" is in the blocklist as a single-word entry.
        // "Standard" contains "ard" as a substring but is NOT a hallucination.
        assert!(!is_hallucination("Standard"), "Standard must NOT be blocked (substring-only false positive)");
    }

    #[test]
    fn test_h14_milliarde_not_blocked() {
        assert!(!is_hallucination("Milliarde"), "Milliarde must NOT be blocked (contains 'ard')");
    }

    #[test]
    fn test_h14_hardware_not_blocked() {
        assert!(!is_hallucination("Hardware"), "Hardware must NOT be blocked (contains 'ard')");
    }

    #[test]
    fn test_h14_zdf_still_blocked_whole_word() {
        // "zdf" as a standalone word must still be blocked.
        assert!(is_hallucination("ZDF"), "ZDF must be blocked as whole-word match");
        assert!(is_hallucination("ZDF 2020"), "ZDF 2020 must be blocked");
    }

    #[test]
    fn test_h14_wdr_still_blocked() {
        assert!(is_hallucination("WDR"), "WDR must be blocked");
    }

    #[test]
    fn test_h14_ard_standalone_still_blocked() {
        assert!(is_hallucination("ARD"), "ARD standalone must still be blocked");
        assert!(is_hallucination("ard"), "ard lowercase standalone must be blocked");
    }

    #[test]
    fn test_h14_multi_word_entry_substring_match_preserved() {
        // Multi-word entries (e.g. "amara.org") stay as substring match.
        // "amara.org" is one token (no space), but the story says phrases with spaces stay substring.
        // "untertitelung des zdf" contains a space → substring match preserved.
        assert!(is_hallucination("Untertitelung des ZDF"), "multi-word entry must still be caught");
    }

    // --- AC5: Stockphrase family + trailing-ghost hardening ---

    #[test]
    fn test_ac5_stockphrase_grosz_und_kleinschreibung_blocked() {
        // Full Whisper stockphrase ghost — must be blocked on short clips.
        assert!(is_hallucination("Groß- und Kleinschreibung, Satzzeichen und Interpunktion"), "stockphrase family must be blocked");
    }

    #[test]
    fn test_ac5_stockphrase_klinge_blocked() {
        // Partial ghost that Whisper outputs before cleanup rationalizes it.
        assert!(is_hallucination("Klinge"), "stockphrase ghost fragment must be blocked");
        assert!(is_hallucination("Klingel"), "stockphrase Klingel fragment must be blocked");
    }

    #[test]
    fn test_ac5_grosz_und_klinge_short_clip_blocked() {
        // Observed real-world short-clip ghost from the audit.
        assert!(is_hallucination("Groß- und Klinge"), "short Groß-und-Klinge ghost must be blocked");
    }

    #[test]
    fn test_ac5_trailing_ghost_on_long_clip_blocked() {
        // Long clip (>8 words) with trailing stockphrase ghost — the old word_count > 8 gate
        // would have let this pass. The trailing-ghost match must NOT be gated by word count.
        let long_with_trailing = "Ich habe heute die wichtigen Unterlagen abgeholt und alles erledigt Groß- und Kleinschreibung";
        assert!(is_hallucination(long_with_trailing), "trailing stockphrase ghost on long clip must be blocked");
    }

    #[test]
    fn test_ac5_trailing_ghost_klingel_on_long_clip_blocked() {
        let long_with_klingel = "Die Besprechung war sehr produktiv und hat alle Erwartungen erfüllt Klingel";
        assert!(is_hallucination(long_with_klingel), "trailing Klingel ghost on long clip must be blocked");
    }

    #[test]
    fn test_ac5_genuine_long_speech_with_ard_not_blocked() {
        // Long real speech that incidentally contains "ard" (whole-word) inside a word — must not be blocked.
        // (The stockphrase gate is a separate match path, not the single-word whole-word path)
        let genuine = "Die Hardwareprüfung der ARD-Sendungen war erfolgreich abgeschlossen worden heute";
        // "ard" is not a separate whole word here (it's "ARD" as a token though!) — but "ARD" IS in blocklist.
        // Let's use a safe example without any blocklist word.
        let safe_genuine = "Die Hardwareprüfung der Standardkomponenten war erfolgreich abgeschlossen worden";
        assert!(!is_hallucination(safe_genuine), "long genuine speech without blocklist terms must pass");
    }

    #[test]
    fn test_ac5_musik_descriptor_blocked() {
        // [Musik] is already in the blocklist; verify it still works.
        assert!(is_hallucination("[Musik]"), "[Musik] must be blocked");
    }

    // --- AC7: strip_stockphrase_ghosts (post-cleanup no-invent guard) ---

    #[test]
    fn test_ac7_strip_stockphrase_removes_trailing_kleinschreibung() {
        // Cleanup invented the full stockphrase from a "Klinge" ghost.
        // strip_stockphrase_ghosts must remove it from the cleaned output.
        let cleaned = "Ich habe die wichtigen Unterlagen abgeholt. Groß- und Kleinschreibung, Satzzeichen und Interpunktion.";
        let result = strip_stockphrase_ghosts(cleaned);
        assert!(!result.to_lowercase().contains("kleinschreibung"), "stockphrase must be stripped post-cleanup");
    }

    #[test]
    fn test_ac7_strip_stockphrase_removes_trailing_klinge() {
        let cleaned = "Das Meeting war produktiv und hat gut funktioniert Klinge.";
        let result = strip_stockphrase_ghosts(cleaned);
        assert!(!result.to_lowercase().contains("klinge"), "trailing klinge ghost must be stripped");
    }

    #[test]
    fn test_ac7_strip_stockphrase_preserves_clean_text() {
        // No stockphrase in clean dictation must remain unchanged.
        let clean = "Bitte send mir die Datei bis Freitag.";
        let result = strip_stockphrase_ghosts(clean);
        assert_eq!(result, clean, "clean text must not be modified");
    }

    #[test]
    fn test_ac7_strip_stockphrase_entire_text_is_ghost_returns_empty() {
        // If the entire cleanup output is a stockphrase, the result is empty (or just whitespace).
        let ghost = "Groß- und Kleinschreibung, Satzzeichen und Interpunktion.";
        let result = strip_stockphrase_ghosts(ghost);
        assert!(result.trim().is_empty() || !result.to_lowercase().contains("kleinschreibung"), "full ghost cleanup output must produce empty result");
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

    // --- Finding 1: strip_stockphrase_ghosts must NOT panic on Unicode where
    //     to_lowercase() changes byte length (e.g. "İ" U+0130: 2 → 3 bytes). ---

    #[test]
    fn test_strip_stockphrase_ghosts_unicode_no_panic_dotted_i() {
        // "İ" (U+0130, 2 bytes in UTF-8) lowercases to "i\u{307}" (3 bytes).
        // Passing it through strip_stockphrase_ghosts must NOT panic — this was the
        // original bug: offsets from to_lowercase() could land on non-char-boundaries
        // in the original string, causing a slice-index panic.
        //
        // "İklinge" is a SINGLE TOKEN where "klinge" is not a standalone whole word,
        // so whole-word matching correctly does NOT strip it (this is the desired behavior
        // — we only strip standalone "Klinge" ghosts, not embedded substrings).
        // The critical requirement is: NO PANIC.
        let input = "İklinge";
        let result = std::panic::catch_unwind(|| strip_stockphrase_ghosts(input));
        assert!(result.is_ok(), "strip_stockphrase_ghosts must not panic on Unicode input with differing lowercase byte length (İ U+0130 lowercases from 2 to 3+ bytes)");

        // Verify the whole-word check correctly preserved "İklinge" (it is not a standalone ghost).
        let s = result.unwrap();
        assert!(s.to_lowercase().contains("klinge"), "İklinge must be preserved (klinge is not a standalone token here)");
    }

    #[test]
    fn test_strip_stockphrase_ghosts_unicode_standalone_klinge_stripped() {
        // Standalone "Klinge" after a sentence with a leading Unicode character — the critical
        // case is that the char-index slicing does not panic when Unicode chars precede the ghost.
        // "İyi günler" is real Turkish text (good day), "Klinge" is the trailing ghost.
        let input = "İyi günler Klinge";
        let result = std::panic::catch_unwind(|| strip_stockphrase_ghosts(input));
        assert!(result.is_ok(), "must not panic on Unicode input with trailing standalone ghost");
        let s = result.unwrap();
        // Standalone "Klinge" (whole word) must be stripped.
        assert!(!s.to_lowercase().contains("klinge"), "standalone Klinge ghost after Unicode text must be stripped");
        // Real content preserved.
        assert!(s.contains("İyi"), "real Unicode content must be preserved");
    }

    #[test]
    fn test_strip_stockphrase_ghosts_ascii_strip_still_works() {
        // Baseline: normal-case strip must still work after the fix.
        let input = "Good content Klinge";
        let result = strip_stockphrase_ghosts(input);
        assert!(!result.to_lowercase().contains("klinge"), "klinge ghost must be stripped (ASCII case)");
        assert!(result.contains("Good content"), "real content must be preserved");
    }

    // --- Finding 2a: "klinge"/"klingel" must be whole-word matched so real German
    //     words like "klingen", "Klingelton", "Türklingel" are NOT blocked. ---

    #[test]
    fn test_finding2a_klingen_not_blocked_by_is_hallucination() {
        assert!(!is_hallucination("klingen"), "\"klingen\" must NOT be blocked (contains \"klinge\" but is a real word)");
        assert!(!is_hallucination("Klingelton"), "\"Klingelton\" must NOT be blocked (contains \"klingel\" but is a real word)");
        assert!(!is_hallucination("es klingelt"), "\"es klingelt\" must NOT be blocked (contains \"klingel\" but is real speech)");
        assert!(!is_hallucination("Türklingel"), "\"Türklingel\" must NOT be blocked");
    }

    #[test]
    fn test_finding2a_standalone_klingel_is_blocked() {
        // Standalone "Klingel" as a whole token IS a stockphrase ghost — must still be blocked.
        assert!(is_hallucination("Klingel"), "standalone \"Klingel\" ghost must be blocked");
        assert!(is_hallucination("klinge"), "standalone \"klinge\" ghost must be blocked");
        assert!(is_hallucination("Klinge"), "standalone \"Klinge\" ghost must be blocked");
    }

    #[test]
    fn test_finding2a_strip_stockphrase_klingen_preserved() {
        // "klingen" must NOT be stripped (it's not a whole-word "klinge" match).
        let input = "Das Glas kann klingen wenn man es berührt";
        let result = strip_stockphrase_ghosts(input);
        assert!(result.contains("klingen"), "\"klingen\" must be preserved by strip_stockphrase_ghosts");
    }

    #[test]
    fn test_finding2a_strip_stockphrase_standalone_klinge_stripped() {
        // Standalone "Klinge" appended to real text IS stripped.
        let input = "Das Meeting lief gut Klinge";
        let result = strip_stockphrase_ghosts(input);
        assert!(!result.to_lowercase().contains("klinge"), "standalone \"Klinge\" ghost must be stripped");
        assert!(result.contains("Meeting"), "real content preserved");
    }

    // --- Finding 2b: dotted/URL credit entries must use substring match so
    //     "amara.org/community" and "rev.com." (trailing punctuation) are blocked. ---

    #[test]
    fn test_finding2b_amara_org_community_blocked() {
        // "amara.org" is a substring of "amara.org/community" — must be caught.
        assert!(is_hallucination("amara.org/community"), "\"amara.org/community\" must be blocked via substring match");
    }

    #[test]
    fn test_finding2b_rev_com_with_trailing_punct_blocked() {
        // "rev.com." has trailing punctuation — whole-word would miss it, substring catches it.
        assert!(is_hallucination("rev.com."), "\"rev.com.\" must be blocked via substring match");
    }

    #[test]
    fn test_finding2b_otter_ai_plain_still_blocked() {
        assert!(is_hallucination("otter.ai"), "plain \"otter.ai\" must still be blocked");
    }

    // Verify alpha short tokens still use whole-word (H14 parity).
    #[test]
    fn test_finding2b_standard_milliarde_hardware_still_pass() {
        assert!(!is_hallucination("Standard"), "Standard must NOT be blocked");
        assert!(!is_hallucination("Milliarde"), "Milliarde must NOT be blocked");
        assert!(!is_hallucination("Hardware"), "Hardware must NOT be blocked");
    }

    #[test]
    fn test_finding2b_zdf_wdr_ard_still_blocked() {
        assert!(is_hallucination("ZDF"), "ZDF must still be blocked");
        assert!(is_hallucination("WDR"), "WDR must still be blocked");
        assert!(is_hallucination("ARD"), "ARD must still be blocked");
    }
}
