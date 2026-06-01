package com.klarvo.voice

/**
 * DIV-04: Banking-app paste guard — Android port of the security gate that
 * prevents clipboard writes and accessibility paste when a banking or security
 * app holds the foreground.
 *
 * Extracted as a standalone object so that unit tests can bind directly to the
 * real decision site without requiring an Android context (Epic-1-Retro AI-2).
 *
 * Thread-safety: the caller (KlarvoOverlayService paste handler) ensures this is
 * only evaluated on the main looper — the same thread that writes bankingAppActive.
 * No synchronization is needed here.
 */
internal object BankingGuard {

    /**
     * Returns true when the paste path should be suppressed.
     *
     * @param bankingActive Current value of KlarvoOverlayService.bankingAppActive,
     *   read on the main looper immediately before the paste lambda body executes.
     */
    fun shouldBlockPaste(bankingActive: Boolean): Boolean = bankingActive
}
