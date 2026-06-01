package com.klarvo.voice

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Unit tests for BankingGuard.shouldBlockPaste — the banking-app paste guard
 * introduced by Story 2.4 (DIV-04 fix).
 *
 * AI-2 binding (Epic 1 retro): tests call BankingGuard.shouldBlockPaste()
 * directly — the real production decision function — not a re-implemented copy
 * of the `if (bankingAppActive)` logic inline in the test body.
 *
 * No Android context is needed: BankingGuard is a pure Kotlin object with no
 * Android dependencies, so it runs in a plain JVM test.
 *
 * The full integration path (pipeline completes while banking app is focused →
 * nothing reaches clipboard or accessibility paste, UI returns to IDLE, toast shown)
 * is covered by the on-device smoke (Task 3).
 *
 * Coverage:
 * - AC-3: bankingActive = false → paste proceeds (positive-regression path)
 * - AC-1: bankingActive = true  → paste blocked
 * - AC-4 side-effect (IDLE transition, toast) verified in on-device smoke only
 */
class BankingGuardTest {

    // ---------------------------------------------------------------------------
    // AC-3: bankingActive == false → paste proceeds (positive / regression path)
    // ---------------------------------------------------------------------------

    @Test
    fun shouldBlockPaste_bankingNotActive_returnsFalse() {
        assertFalse(
            "When banking app is NOT active, paste must NOT be blocked (AC-3 regression)",
            BankingGuard.shouldBlockPaste(bankingActive = false)
        )
    }

    // ---------------------------------------------------------------------------
    // AC-1: bankingActive == true → paste blocked
    // ---------------------------------------------------------------------------

    @Test
    fun shouldBlockPaste_bankingActive_returnsTrue() {
        assertTrue(
            "When banking app IS active, paste must be blocked (AC-1)",
            BankingGuard.shouldBlockPaste(bankingActive = true)
        )
    }

    // ---------------------------------------------------------------------------
    // AC-1 edge: decision is idempotent for true (no mutable state inside guard)
    // ---------------------------------------------------------------------------

    @Test
    fun shouldBlockPaste_bankingActive_idempotent() {
        assertTrue("First call with true must block", BankingGuard.shouldBlockPaste(bankingActive = true))
        assertTrue("Second call with true must still block", BankingGuard.shouldBlockPaste(bankingActive = true))
    }

    // ---------------------------------------------------------------------------
    // AC-3 edge: decision is idempotent for false as well
    // ---------------------------------------------------------------------------

    @Test
    fun shouldBlockPaste_bankingNotActive_idempotent() {
        assertFalse("First call with false must not block", BankingGuard.shouldBlockPaste(bankingActive = false))
        assertFalse("Second call with false must still not block", BankingGuard.shouldBlockPaste(bankingActive = false))
    }
}
