package com.klarvo.voice

import org.junit.Test

/**
 * Task 2.2 — Story 11-2 (AC-3/AC-4): guard governing whether the repeatable preview-flush
 * callback gets installed at all. HOLD/TOGGLE + enabled -> true; every other combination -> false
 * (Auto/AutoStop never get a preview flush, mirrors desktop FR4 parity; disabled means byte-
 * identical existing behavior, AC-4).
 *
 * **Story 13-2 (E1 / D-H9) added the `sttProvider` dimension.** With a stored
 * `sttProvider = "local"` the live preview uploaded every pause's delta WAV to
 * Groq — with or without a key, because `readConfig` admits `local` with a
 * blank Groq key and the Rust `WhisperStt::transcribe` sends the whole
 * multipart body before the 401 comes back. Audio left the device under a
 * setting called "Offline". The desktop twin,
 * `pipeline::preview_flush_should_install`, has always taken `stt_provider`.
 *
 * What this does NOT cover: the flush-TIME re-check in
 * `KlarvoOverlayService.flushPreviewDelta`, which needs a live Service. The
 * two guards are deliberately separate (the config can change between the
 * install and the pause, exactly as `pipeline.rs` re-checks at flush time),
 * and only the install half is a pure function.
 */
class ShouldInstallPreviewFlushTest {

    private fun check(
        mode: KlarvoOverlayService.RecordingMode,
        enabled: Boolean,
        sttProvider: String = "groq",
    ) = KlarvoOverlayService.RecordingMode.shouldInstallPreviewFlush(mode, enabled, sttProvider)

    @Test
    fun hold_enabled_true() {
        assert(check(KlarvoOverlayService.RecordingMode.HOLD, true))
    }

    @Test
    fun toggle_enabled_true() {
        assert(check(KlarvoOverlayService.RecordingMode.TOGGLE, true))
    }

    @Test
    fun hold_disabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.HOLD, false))
    }

    @Test
    fun toggle_disabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.TOGGLE, false))
    }

    @Test
    fun autostop_enabled_false() {
        // AC-3 scope guard: Auto/AutoStop never install a preview flush, even when enabled.
        assert(!check(KlarvoOverlayService.RecordingMode.AUTOSTOP, true))
    }

    @Test
    fun auto_enabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.AUTO, true))
    }

    @Test
    fun autostop_disabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.AUTOSTOP, false))
    }

    @Test
    fun auto_disabled_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.AUTO, false))
    }

    // -----------------------------------------------------------------------
    // Story 13-2 (E1 / D-H9): a stored "local" STT provider refuses the install
    // -----------------------------------------------------------------------

    /**
     * The row itself, asserted on the combination that WOULD otherwise upload:
     * HOLD, preview explicitly enabled — i.e. against a configuration that had
     * every other reason to install. Asserting it for AUTO would pass against
     * code that ignored `sttProvider` entirely.
     */
    @Test
    fun hold_enabled_butOfflineStt_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.HOLD, true, sttProvider = "local"))
    }

    @Test
    fun toggle_enabled_butOfflineStt_false() {
        assert(!check(KlarvoOverlayService.RecordingMode.TOGGLE, true, sttProvider = "local"))
    }

    /**
     * The gate is "local", not "anything unusual": an unknown or future cloud
     * provider must still install, or a provider rename would silently disable
     * the live preview.
     */
    @Test
    fun otherCloudProviders_stillInstall() {
        assert(check(KlarvoOverlayService.RecordingMode.HOLD, true, sttProvider = "openai"))
        assert(check(KlarvoOverlayService.RecordingMode.HOLD, true, sttProvider = ""))
    }

    /**
     * Inversion (per DoD): flip one branch (e.g. treat AUTOSTOP like HOLD) and this test would
     * go RED -- proving the guard's mode-check is load-bearing, documented empirically here
     * rather than merely asserted.
     */
    @Test
    fun inversion_autostopMustNotEqualHoldBehavior() {
        val autostopResult = check(KlarvoOverlayService.RecordingMode.AUTOSTOP, true)
        val holdResult = check(KlarvoOverlayService.RecordingMode.HOLD, true)
        assert(autostopResult != holdResult) {
            "AUTOSTOP and HOLD must resolve differently for the same enabled=true input -- " +
                "if they ever match, the mode guard has regressed to ignore RecordingMode entirely."
        }
    }

    /**
     * The same shape for the new dimension: drop the `sttProvider != "local"`
     * clause and these two stop differing.
     */
    @Test
    fun inversion_offlineSttMustNotEqualCloudBehavior() {
        val offline = check(KlarvoOverlayService.RecordingMode.HOLD, true, sttProvider = "local")
        val cloud = check(KlarvoOverlayService.RecordingMode.HOLD, true, sttProvider = "groq")
        assert(offline != cloud) {
            "a stored offline STT provider must not install the preview flush -- if these match, " +
                "the E1 guard has been removed and the live preview uploads audio in Offline mode."
        }
    }
}
