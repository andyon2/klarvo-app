package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Unit tests for [KlarvoOverlayService.decideDelivery] — the Step-4 delivery
 * decision introduced by Story 7-10 (AC2).
 *
 * AI-2 binding (Epic 1 retro): these call the real production decision function
 * that `processAudio` Step 4 calls, not a re-implemented copy of the branch.
 *
 * No Android context is needed: `decideDelivery` is a pure function in the
 * companion object — the repo's established seam pattern
 * ([BankingGuard.shouldBlockPaste], `sanitizePreviewChunk`) — so it runs in a
 * plain JVM test. No mocking library exists on this classpath.
 *
 * **What this does NOT cover** (honesty boundary, mirroring
 * [BankingGuardTest]'s KDoc): the real `copyToClipboard` write, the real
 * `pasteIntoFocusedField()` accessibility call, the toast rendering and its
 * ordering against HyperOS's own "pasted from your clipboard" system toast, the
 * banking guard's position ahead of the whole block, and that Step 2 actually
 * sets `llmCleanupFailed` on the right branches. All of those are on-device
 * smoke territory; this test decides the branch logic only.
 *
 * Coverage:
 * - AC2: cleanup failed → no paste, "Copied: …" toast suppressed (Q5)
 * - AC2 inverse: cleanup OK → paste exactly as before (both a11y states)
 * - AC2: a successful fallback is NOT a cleanup failure
 */
class CleanupFailureDeliveryTest {

    // -----------------------------------------------------------------------
    // AC2: cleanup failed → clipboard only
    // -----------------------------------------------------------------------

    /**
     * The story's core claim: a failed cleanup never reaches the focused field.
     *
     * Asserted with the accessibility service CONNECTED — i.e. against a target
     * that could really have been pasted into. That is the discriminating case:
     * asserting `paste == false` while no service is connected would pass
     * against code that never had the option in the first place.
     */
    @Test
    fun cleanupFailure_withAccessibilityConnected_doesNotPaste() {
        val decision = KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = true,
            accessibilityConnected = true
        )

        assertFalse(
            "a failed cleanup must never insert filler-laden raw text",
            decision.paste
        )
    }

    /** Q5: exactly one toast on this path — the combined degrade message. */
    @Test
    fun cleanupFailure_suppressesCopiedToast() {
        val connected = KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = true,
            accessibilityConnected = true
        )
        val notConnected = KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = true,
            accessibilityConnected = false
        )

        assertFalse("Q5: the degrade toast is the only one", connected.showCopiedToast)
        assertFalse("Q5: also when no a11y service is connected", notConnected.showCopiedToast)
    }

    /** The clipboard-only branch does not depend on the a11y service at all. */
    @Test
    fun cleanupFailure_decisionIsIndependentOfAccessibilityState() {
        assertEquals(
            KlarvoOverlayService.decideDelivery(true, accessibilityConnected = true),
            KlarvoOverlayService.decideDelivery(true, accessibilityConnected = false)
        )
    }

    // -----------------------------------------------------------------------
    // AC2 inverse: cleanup OK → unchanged behaviour
    // -----------------------------------------------------------------------

    /**
     * Regression half: with cleanup working, Step 4 behaves exactly as it did
     * before Story 7-10 — paste when the service is connected, otherwise fall
     * back to the "Copied: …" toast.
     */
    @Test
    fun cleanupOk_pastesWhenAccessibilityConnected() {
        val decision = KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = false,
            accessibilityConnected = true
        )

        assertTrue("a successful cleanup must still paste", decision.paste)
        assertFalse("a successful paste stays silent (story 12-1)", decision.showCopiedToast)
    }

    @Test
    fun cleanupOk_withoutAccessibility_showsCopiedToastInstead() {
        val decision = KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = false,
            accessibilityConnected = false
        )

        assertFalse(decision.paste)
        assertTrue(
            "without an a11y service the user must learn the text is on the clipboard",
            decision.showCopiedToast
        )
    }

    /**
     * A successful FALLBACK provider is not a cleanup failure — the trap the
     * story's Dev Notes call out. `degradeStatusMsg` is set on that path too
     * ("⚠ Cleanup-Anbieter gewechselt"), which is exactly why the explicit
     * `llmCleanupFailed` boolean exists. Cleanup produced real text, so it
     * still pastes.
     */
    @Test
    fun successfulFallbackProvider_isNotACleanupFailure_andStillPastes() {
        val decision = KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = false,
            accessibilityConnected = true
        )

        assertTrue(
            "a switched-but-working provider produced cleaned text — it must paste",
            decision.paste
        )
    }

    // -----------------------------------------------------------------------
    // Q2/Q5: the message mirrors the desktop pill
    // -----------------------------------------------------------------------

    /**
     * Twin-wording lock: the toast literal must match the desktop pill's generic
     * degrade text (`pipeline::degrade_warn_msg`). This is a written record, not
     * a cross-platform lock — no fixture is shared, so a desktop-only edit
     * cannot fail this test. Change both sides together.
     */
    @Test
    fun degradeMessage_mirrorsDesktopPillWording() {
        assertEquals(
            "Cleanup failed — raw text in clipboard (Ctrl+V)",
            KlarvoOverlayService.CLEANUP_FAILED_CLIPBOARD_MSG
        )
    }

    /** The old wording claimed an insertion that no longer happens. */
    @Test
    fun degradeMessage_noLongerClaimsTheTextWasInserted() {
        val msg = KlarvoOverlayService.CLEANUP_FAILED_CLIPBOARD_MSG

        assertFalse(msg.contains("eingefügt"))
        assertFalse(msg.contains("inserted"))
        assertTrue(msg.contains("clipboard"))
    }
}
