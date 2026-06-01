package com.klarvo.voice

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Unit tests for HallucinationFilter.
 *
 * These run as JVM unit tests (no device required). They bind to the real
 * HallucinationFilter.isHallucination() production function — not a re-declared
 * copy — so a logic regression turns a test red (AI-2 from Epic 1 retro).
 *
 * Coverage:
 * - Empty / whitespace input
 * - Real speech must not be blocked
 * - ROB-03 false-positive prevention: single-word entries use whole-word matching
 * - Canonical blocklist phrases (German, English, service credits, music descriptors)
 * - Word-count gate (>8 words → pass regardless of blocklist)
 * - Case-insensitivity
 */
class HallucinationFilterTest {

    // --- Empty / whitespace ---

    @Test
    fun emptyString_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination(""))
    }

    @Test
    fun whitespaceOnly_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("   "))
        assertTrue(HallucinationFilter.isHallucination("\t\n"))
    }

    // --- Real speech must pass ---

    @Test
    fun realSpeech_notBlocked() {
        assertFalse(HallucinationFilter.isHallucination("Bitte schick mir die Datei"))
        assertFalse(HallucinationFilter.isHallucination("Das Meeting ist um 14 Uhr"))
        assertFalse(HallucinationFilter.isHallucination("Please send me the report by Friday"))
        assertFalse(HallucinationFilter.isHallucination("Ich brauche die Unterlagen bis Montag"))
    }

    // --- ROB-03 false-positive prevention (whole-word matching for single-word entries) ---

    @Test
    fun standard_notBlocked() {
        // "ard" is a blocklist entry; "Standard" must NOT be caught as a false positive
        assertFalse(HallucinationFilter.isHallucination("Standard"))
    }

    @Test
    fun milliarde_notBlocked() {
        assertFalse(HallucinationFilter.isHallucination("Milliarde"))
    }

    @Test
    fun hardware_notBlocked() {
        assertFalse(HallucinationFilter.isHallucination("Hardware"))
    }

    @Test
    fun standardInPhrase_notBlocked() {
        assertFalse(HallucinationFilter.isHallucination("Das ist der Standard hier"))
    }

    // --- Single-word blocklist entries: whole-word match ---

    @Test
    fun zdfAlone_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("ZDF"))
    }

    @Test
    fun zdfWithYear_isHallucination() {
        // "ZDF 2020" → 2 words, both within gate; word "zdf" matches
        assertTrue(HallucinationFilter.isHallucination("ZDF 2020"))
    }

    @Test
    fun ardAlone_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("ARD"))
    }

    @Test
    fun wdrAlone_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("WDR"))
    }

    @Test
    fun musicSymbol_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("♪"))
    }

    @Test
    fun amaraDotOrg_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("amara.org"))
    }

    // --- Multi-word blocklist phrases ---

    @Test
    fun untertitelungDesZdf_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("Untertitelung des ZDF"))
    }

    @Test
    fun musicDescriptor_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("[Music]"))
        assertTrue(HallucinationFilter.isHallucination("[MUSIC]"))
    }

    @Test
    fun englishSignOff_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("Thank you for watching"))
        assertTrue(HallucinationFilter.isHallucination("Thanks for watching"))
        assertTrue(HallucinationFilter.isHallucination("Please subscribe"))
        assertTrue(HallucinationFilter.isHallucination("Like and subscribe"))
    }

    @Test
    fun germanSignOff_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("Danke fürs Zuschauen"))
        assertTrue(HallucinationFilter.isHallucination("Vielen Dank fürs Zuschauen"))
    }

    @Test
    fun transcriptionCredit_isHallucination() {
        assertTrue(HallucinationFilter.isHallucination("Subtitles by"))
        assertTrue(HallucinationFilter.isHallucination("Captions by"))
        assertTrue(HallucinationFilter.isHallucination("rev.com"))
        assertTrue(HallucinationFilter.isHallucination("otter.ai"))
    }

    // --- Word-count gate: >8 words always pass ---

    @Test
    fun longTextWithZdf_passesThrough() {
        // 11 words — word-count gate opens regardless of "ZDF"
        val long = "Ich habe heute beim ZDF angerufen und mit dem Redakteur gesprochen"
        assertFalse(HallucinationFilter.isHallucination(long))
    }

    @Test
    fun nineWordsWithBlocklistPhrase_passesThrough() {
        // 9 words containing "amara.org"
        val nine = "Diese Untertitel wurden erstellt von amara.org und sind gratis hier"
        assertFalse(HallucinationFilter.isHallucination(nine))
    }

    @Test
    fun exactlyEightWords_canBeBlocked() {
        // 8 words — still within the gate; "amara.org" is a blocklist entry
        val eight = "Diese Untertitel wurden erstellt von amara.org sind hier"
        assertTrue(HallucinationFilter.isHallucination(eight))
    }

    // --- Case-insensitivity ---

    @Test
    fun caseInsensitive_uppercase() {
        assertTrue(HallucinationFilter.isHallucination("THANK YOU FOR WATCHING"))
    }

    @Test
    fun caseInsensitive_mixed() {
        assertTrue(HallucinationFilter.isHallucination("Thank You For Watching"))
    }

    // --- Whitespace padding ---

    @Test
    fun whitespacePadded_blocklisted() {
        assertTrue(HallucinationFilter.isHallucination("  ZDF  "))
        assertTrue(HallucinationFilter.isHallucination("  thank you for watching  "))
    }
}
