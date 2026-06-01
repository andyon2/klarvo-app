package com.klarvo.voice

/**
 * Whisper hallucination filter — Android port of stt/hallucination.rs.
 *
 * Whisper generates phantom transcriptions for silence, background noise, and
 * non-speech audio by associating them with subtitle/credit patterns from its
 * training data (YouTube subtitle files, ARD/ZDF credits, etc.).
 *
 * Matching strategy (corrects Rust ROB-03 false-positive):
 * - Single-word blocklist entries (no spaces): whole-word match only.
 *   Prevents "ard" from hitting "Standard", "Milliarde", "Hardware".
 * - Multi-word entries (contain spaces): substring match.
 *   Phrases are specific enough that substring collision is not a concern.
 *
 * Word-count gate (>8 words → pass): long dictation that incidentally mentions
 * a blocklist term is almost certainly real speech and must not be discarded.
 */
object HallucinationFilter {

    // Mirrors HALLUCINATION_BLOCKLIST in stt/hallucination.rs (all entries lowercase).
    private val HALLUCINATION_BLOCKLIST = listOf(
        // --- German broadcast subtitle artifacts ---
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
        "sous-titres",
        "sous-titrage",
        "sous titres",
        "sottotitoli",
        "subtítulos",
        "napisy pobrano",
        // --- Music / noise descriptors ---
        "[music]",
        "[applause]",
        "[laughter]",
        "[silence]",
        "[inaudible]",
        "[background noise]",
        "[piano music]",
        "♪",
    )

    /**
     * Returns true if [text] is likely a Whisper hallucination artifact.
     *
     * Applies two checks in order:
     * 1. Empty/whitespace → true (no useful content).
     * 2. Blocklist match with word-count gate: if the text contains a known
     *    hallucination phrase AND has ≤8 words, it is a hallucination.
     *    Texts longer than 8 words pass through regardless of blocklist content.
     */
    fun isHallucination(text: String): Boolean {
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return true

        val words = trimmed.lowercase().split(Regex("\\s+"))
        // Word-count gate: long texts are almost certainly real speech.
        if (words.size > 8) return false

        // Reconstruct normalised lower for multi-word substring matching.
        val lower = words.joinToString(" ")

        return HALLUCINATION_BLOCKLIST.any { entry ->
            if (' ' in entry) {
                // Multi-word entry: substring match is safe (phrase is specific).
                lower.contains(entry)
            } else {
                // Single-word entry: whole-word match to prevent false positives
                // ("ard" must not discard "Standard", "Milliarde", "Hardware").
                words.any { it == entry }
            }
        }
    }
}
