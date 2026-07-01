package com.klarvo.voice

import org.junit.Test

/**
 * Code-review fix F4 (2026-07-01) -- Story 11-2: [ListeningPanelView.updateTranscriptColor] used
 * to hard-reset the transcript color to Muted/Dim on every panelState change, discarding the
 * `previewTextColor` [ListeningPanelView.applyAppearance] configured -- so a configured preview
 * color reverted on the first RECORDING->TRANSCRIBING transition. Guards
 * [ListeningPanelView.resolveTranscriptColor], the pure decision extracted from that fix.
 */
class ResolveTranscriptColorTest {

    private val recordingColor = 0xFFA4A9AC.toInt()   // stand-in for KlarvoTheme.Muted
    private val transcribingColor = 0xFF6F7479.toInt() // stand-in for KlarvoTheme.Dim
    private val configuredColor = 0xFF2AC3A8.toInt()   // stand-in for an applied previewTextColor

    @Test
    fun noAppearanceApplied_recording_usesRecordingColor() {
        val result = ListeningPanelView.resolveTranscriptColor(
            null, ListeningPanelView.State.RECORDING, recordingColor, transcribingColor
        )
        assert(result == recordingColor)
    }

    @Test
    fun noAppearanceApplied_transcribing_usesTranscribingColor() {
        val result = ListeningPanelView.resolveTranscriptColor(
            null, ListeningPanelView.State.TRANSCRIBING, recordingColor, transcribingColor
        )
        assert(result == transcribingColor)
    }

    @Test
    fun appearanceApplied_recording_honorsConfiguredColor() {
        val result = ListeningPanelView.resolveTranscriptColor(
            configuredColor, ListeningPanelView.State.RECORDING, recordingColor, transcribingColor
        )
        assert(result == configuredColor)
    }

    @Test
    fun appearanceApplied_transcribing_honorsConfiguredColor() {
        val result = ListeningPanelView.resolveTranscriptColor(
            configuredColor, ListeningPanelView.State.TRANSCRIBING, recordingColor, transcribingColor
        )
        assert(result == configuredColor)
    }

    /**
     * Inversion (per DoD): the pre-fix behavior (always fall back to recording/transcribing
     * color, ignoring any applied config) would make this test go RED for the TRANSCRIBING case
     * -- proving the applied-color override is load-bearing, not accidental.
     */
    @Test
    fun inversion_ignoringAppliedColorWouldFailThisTest() {
        val preFixBehavior = if (false) configuredColor
            else transcribingColor // pre-fix: always Dim in TRANSCRIBING, appliedColor ignored
        val fixedResult = ListeningPanelView.resolveTranscriptColor(
            configuredColor, ListeningPanelView.State.TRANSCRIBING, recordingColor, transcribingColor
        )
        assert(fixedResult != preFixBehavior) {
            "resolveTranscriptColor(configuredColor, TRANSCRIBING, ...) must return the " +
                "configured color, not the hardcoded Dim fallback -- if it ever matches the " +
                "pre-fix value, the F4 regression is back."
        }
    }
}
