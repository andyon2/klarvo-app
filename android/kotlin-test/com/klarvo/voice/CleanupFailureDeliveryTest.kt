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
 * sets `llmCleanupFailed` on the right branches — including the one the story
 * calls out, "the fallback provider succeeded". All of those are on-device smoke
 * territory; this test decides the branch logic only.
 *
 * Coverage:
 * - AC2: cleanup failed → no paste, "Copied: …" toast suppressed (Q5)
 * - AC2 inverse: cleanup OK → paste exactly as before (both a11y states); this
 *   is also the successful-fallback case, at the flag's consequence only
 * - Q2/Q5: the toast literal mirrors the pill, minus the key hint (GATE 2)
 */
class CleanupFailureDeliveryTest {

    /**
     * Drives the real decision for the ordinary case: the clipboard write
     * worked, and the paste landed **iff** it was attempted. Story 13-2 gave
     * `decideDelivery` the paste OUTCOME, so every call now has to say what
     * the paste did; this keeps the pre-existing tests reading as before while
     * the new rows below vary the two new inputs explicitly.
     */
    private fun pasted(
        llmCleanupFailed: Boolean,
        accessibilityConnected: Boolean,
    ): KlarvoOverlayService.DeliveryDecision {
        val attempted = KlarvoOverlayService.shouldAttemptPaste(
            llmCleanupFailed,
            accessibilityConnected,
            clipboardOk = true,
        )
        return KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = llmCleanupFailed,
            accessibilityConnected = accessibilityConnected,
            clipboardOk = true,
            pasteOutcome = if (attempted) KlarvoAccessibilityService.PasteOutcome.PASTED else null,
        )
    }

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
        val decision = pasted(llmCleanupFailed = true, accessibilityConnected = true)

        assertFalse(
            "a failed cleanup must never insert filler-laden raw text",
            decision.paste
        )
    }

    /** Q5: exactly one toast on this path — the combined degrade message. */
    @Test
    fun cleanupFailure_suppressesCopiedToast() {
        val connected = pasted(llmCleanupFailed = true, accessibilityConnected = true)
        val notConnected = pasted(llmCleanupFailed = true, accessibilityConnected = false)

        assertFalse("Q5: the degrade toast is the only one", connected.showCopiedToast)
        assertFalse("Q5: also when no a11y service is connected", notConnected.showCopiedToast)
    }

    /** The clipboard-only branch does not depend on the a11y service at all. */
    @Test
    fun cleanupFailure_decisionIsIndependentOfAccessibilityState() {
        assertEquals(
            pasted(llmCleanupFailed = true, accessibilityConnected = true),
            pasted(llmCleanupFailed = true, accessibilityConnected = false)
        )
    }

    // -----------------------------------------------------------------------
    // AC2 inverse: cleanup OK → unchanged behaviour
    // -----------------------------------------------------------------------

    /**
     * Regression half: with cleanup working, Step 4 behaves exactly as it did
     * before Story 7-10 — paste when the service is connected, otherwise fall
     * back to the "Copied: …" toast.
     *
     * **This is also the successful-FALLBACK case**, and the honest scope of
     * that claim: a switched-but-working provider (`"⚠ Cleanup-Anbieter
     * gewechselt"`) reaches Step 4 with `llmCleanupFailed = false`, i.e. exactly
     * this call, and must still paste. What is asserted is the *consequence* of
     * that flag being false. Whether Step 2 really leaves it false on the
     * fallback-succeeded branch is **not** asserted here and cannot be — that is
     * a `processAudio` branch, see the "does NOT cover" list above. A separate
     * test passing `false` by hand would have read like a second check while
     * being this one verbatim (review round 1).
     */
    @Test
    fun cleanupOk_pastesWhenAccessibilityConnected() {
        val decision = pasted(llmCleanupFailed = false, accessibilityConnected = true)

        assertTrue("a successful cleanup must still paste", decision.paste)
        assertFalse("a successful paste stays silent (story 12-1)", decision.showCopiedToast)
        assertTrue("a landed paste is the success path", decision.success)
    }

    @Test
    fun cleanupOk_withoutAccessibility_showsCopiedToastInstead() {
        val decision = pasted(llmCleanupFailed = false, accessibilityConnected = false)

        assertFalse(decision.paste)
        assertTrue(
            "without an a11y service the user must learn the text is on the clipboard",
            decision.showCopiedToast
        )
        // Unchanged by story 13-2 and deliberately so: clipboard delivery with
        // no accessibility service is the intended outcome, not a failure. No
        // audit row re-opens it, and Desktop's twin (`DoneClipboard`) is a
        // terminal state rather than an error either.
        assertTrue("nothing was attempted, so nothing failed", decision.success)
    }

    // -----------------------------------------------------------------------
    // Q2/Q5: the message mirrors the desktop pill
    // -----------------------------------------------------------------------

    /**
     * Twin-wording lock: the toast literal mirrors the desktop pill's generic
     * degrade text (`pipeline::degrade_warn_msg`) MINUS its "(Ctrl+V)" key hint —
     * GATE 2 (Andi, 2026-09-14): there is no Ctrl+V on a phone. The
     * model-not-found form (`Model '<id>' not found — in clipboard`) carries no
     * key hint on either platform and stays identical to the pill.
     *
     * This is a written record, not a cross-platform lock — no fixture is
     * shared, so a desktop-only edit cannot fail this test. Change both sides
     * together.
     */
    @Test
    fun degradeMessage_mirrorsDesktopPillWording() {
        assertEquals(
            "Cleanup failed — raw text in clipboard",
            KlarvoOverlayService.CLEANUP_FAILED_CLIPBOARD_MSG
        )
    }

    /** GATE 2: the phone toast must not advertise a keyboard shortcut. */
    @Test
    fun degradeMessage_hasNoKeyHintOnAndroid() {
        assertFalse(KlarvoOverlayService.CLEANUP_FAILED_CLIPBOARD_MSG.contains("Ctrl+V"))
    }

    /** The old wording claimed an insertion that no longer happens. */
    @Test
    fun degradeMessage_noLongerClaimsTheTextWasInserted() {
        val msg = KlarvoOverlayService.CLEANUP_FAILED_CLIPBOARD_MSG

        assertFalse(msg.contains("eingefügt"))
        assertFalse(msg.contains("inserted"))
        assertTrue(msg.contains("clipboard"))
    }

    // -----------------------------------------------------------------------
    // Story 13-2 — D4 / D-H20: an attempted paste that did not land
    // -----------------------------------------------------------------------

    /**
     * The row's own case: the accessibility service IS connected (so a paste
     * was genuinely attempted) but no editable field was focused. That is the
     * discriminating setup — asserting against a disconnected service would
     * pass against code that never had the option.
     *
     * Before story 13-2 `pasteIntoFocusedField()` returned `Unit`, Step 4
     * decided on `instance != null`, and this path showed the DONE checkmark
     * with no toast: the pill claimed success for text that appeared nowhere.
     */
    @Test
    fun pasteAttemptedButNoFocusedField_showsClipboardToastAndNoCheck() {
        for (miss in listOf(
            KlarvoAccessibilityService.PasteOutcome.NO_ACTIVE_WINDOW,
            KlarvoAccessibilityService.PasteOutcome.NO_FOCUSED_FIELD,
            KlarvoAccessibilityService.PasteOutcome.ACTION_REFUSED,
        )) {
            val decision = KlarvoOverlayService.decideDelivery(
                llmCleanupFailed = false,
                accessibilityConnected = true,
                clipboardOk = true,
                pasteOutcome = miss,
            )
            assertTrue("$miss: the paste was attempted", decision.paste)
            assertTrue("$miss: the user must learn the text is on the clipboard", decision.showCopiedToast)
            assertFalse("$miss: a miss is not a success", decision.success)
            assertEquals(
                "$miss: a run that delivered nothing ends in the shipped IDLE",
                KlarvoOverlayService.RecordingState.IDLE,
                KlarvoOverlayService.terminalStateFor(decision),
            )
        }
    }

    /**
     * Inversion guard for the row above: the ONLY difference between the two
     * calls is the paste outcome. If `decideDelivery` ever stops reading it,
     * these two agree and the test goes red.
     */
    @Test
    fun inversion_pasteOutcomeIsLoadBearing() {
        val landed = KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = false,
            accessibilityConnected = true,
            clipboardOk = true,
            pasteOutcome = KlarvoAccessibilityService.PasteOutcome.PASTED,
        )
        val missed = KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = false,
            accessibilityConnected = true,
            clipboardOk = true,
            pasteOutcome = KlarvoAccessibilityService.PasteOutcome.NO_FOCUSED_FIELD,
        )
        assertTrue(
            "a landed paste and a missed paste must not produce the same delivery",
            landed != missed,
        )
    }

    // -----------------------------------------------------------------------
    // Story 13-2 — D5 / D-M24: no DONE flash after a cleanup failure
    // -----------------------------------------------------------------------

    /**
     * What the DONE-flash block's own comment has always claimed ("Only the
     * success path gets the DONE state; error paths go straight to IDLE") and
     * what the code did not do: `setState(DONE)` ran on every non-blocked
     * Step-4 exit, so Android flashed the success checkmark next to the
     * cleanup-failure degrade toast.
     */
    @Test
    fun cleanupFailure_getsNoDoneFlash() {
        val decision = pasted(llmCleanupFailed = true, accessibilityConnected = true)
        assertFalse(decision.success)
        assertEquals(
            KlarvoOverlayService.RecordingState.IDLE,
            KlarvoOverlayService.terminalStateFor(decision),
        )
    }

    /**
     * No new bubble state (Andi, 2026-09-21): the terminal choice is only ever
     * between the two shipped endings.
     */
    @Test
    fun terminalStateIsOnlyEverDoneOrIdle() {
        for (failed in listOf(true, false)) {
            for (connected in listOf(true, false)) {
                val state = KlarvoOverlayService.terminalStateFor(pasted(failed, connected))
                assertTrue(
                    "terminalStateFor must not invent a state: $state",
                    state == KlarvoOverlayService.RecordingState.DONE ||
                        state == KlarvoOverlayService.RecordingState.IDLE,
                )
            }
        }
    }

    // -----------------------------------------------------------------------
    // Story 13-2 — D6 / D-M12: the clipboard write itself failed
    // -----------------------------------------------------------------------

    /**
     * `copyToClipboard` is now guarded (a throwing `setPrimaryClip` used to be
     * an uncaught main-thread exception). When it fails there is nothing on
     * the clipboard, so nothing may be pasted from it, the "Copied: …" toast
     * would be a lie, and no success may be shown.
     *
     * The real `setPrimaryClip` failure is NOT reproducible on Andi's device
     * and the test provider cannot inject it — ADR-0016 Amendment 4 already
     * records D6 as agent-verified only (Weg 2). This decides the branch
     * logic; the try/catch itself is covered by inspection.
     */
    @Test
    fun clipboardWriteFailed_showsNothingAndClaimsNothing() {
        val decision = KlarvoOverlayService.decideDelivery(
            llmCleanupFailed = false,
            accessibilityConnected = true,
            clipboardOk = false,
            pasteOutcome = null,
        )
        assertFalse("nothing is on the clipboard to paste from", decision.paste)
        assertFalse("\"Copied: …\" would be a lie", decision.showCopiedToast)
        assertFalse("a lost text is not a success", decision.success)
        assertEquals(
            KlarvoOverlayService.RecordingState.IDLE,
            KlarvoOverlayService.terminalStateFor(decision),
        )
    }

    /** The pre-paste gate reads the clipboard outcome too. */
    @Test
    fun shouldAttemptPaste_requiresAClipboardThatWasWritten() {
        assertTrue(KlarvoOverlayService.shouldAttemptPaste(false, true, clipboardOk = true))
        assertFalse(KlarvoOverlayService.shouldAttemptPaste(false, true, clipboardOk = false))
        assertFalse(KlarvoOverlayService.shouldAttemptPaste(true, true, clipboardOk = true))
        assertFalse(KlarvoOverlayService.shouldAttemptPaste(false, false, clipboardOk = true))
    }
}
