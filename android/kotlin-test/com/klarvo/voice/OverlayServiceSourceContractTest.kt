package com.klarvo.voice

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * Story 13-2 — the four matrix rows whose behaviour lives in `processAudio`'s
 * statement order or in a method that needs a live `Service`.
 *
 * ## Why this file exists
 * A Matrix Test Audit of `spec-13-2-parity-sweep-guards-and-silent-loss-2.md`
 * found four I/O & Edge-Case Matrix rows — **D11**, **B1-Android**,
 * **D6-Android** and the flush-time half of **E1** — whose only evidence was
 * "read the diff". A behaviour a human has to read the diff for is not covered,
 * however well argued, so each one gets an assertion that RUNS.
 *
 * ## The instrument, and its honest weight
 * Three of the four are **source-text tripwires** over the production Kotlin,
 * with comments stripped. That is a weak instrument and it is the sanctioned
 * one in this repo for exactly this situation — `Adr0017BoundaryGuardTest` is
 * the precedent, and [GuardChainBridgeTest] carries the same rationale: no
 * executing JVM test can reach a `handler.post` body, a `ClipboardManager`, or
 * a statement's position inside a 400-line method. A tripwire proves the CODE
 * SAYS the right thing, never that the DEVICE does it; the device half is on
 * Andi's list in `gate4-evidence/13-2/verdict.md`.
 *
 * Where a real seam was cheap, it is used instead: D6's catch semantics run
 * through [KlarvoOverlayService.guardedClipboardWrite] with a throwing fake.
 *
 * ## Vacuity discipline
 * Two of this story's first tripwires came back GREEN under inversion, both
 * because they asserted a `contains` that some *other* call site satisfied. So
 * every assertion here is either an ORDER assertion (which needs both needles
 * to exist and cannot be satisfied by a second occurrence elsewhere) or a
 * two-sided one (the thing that must be gone AND the thing that must remain).
 * Each is inverted against the specific line it claims to pin — see
 * `gate4-evidence/13-2/code-inversion-report.md`.
 */
class OverlayServiceSourceContractTest {

    private val src: String by lazy { stripComments(readOverlayServiceSource()) }

    // -----------------------------------------------------------------------
    // D11 / D-M14 — "nothing recognized" is silent on Android
    // -----------------------------------------------------------------------

    /**
     * Four of the five toasts are recognition RESULTS and have a mute desktop
     * twin (`PipelineEvent::idle()`); they are gone. The fifth,
     * `"No audio recorded"`, fires on an empty capture buffer — a
     * recorder/plumbing fault, not a recognition result — and stays, as do the
     * two other non-recognition messages.
     *
     * **Both halves are asserted.** An assertion that only checked the removals
     * would pass just as happily against a file that had lost all five toasts,
     * which is the silent loss this story exists to remove.
     */
    @Test
    fun nothingRecognizedIsSilent_butCaptureAndConfigFaultsStillSpeak() {
        for (gone in listOf("Recording too short", "No speech detected", "Speech not recognized")) {
            assertFalse(
                "D11: \"$gone\" must no longer be toasted — Desktop is mute on this path",
                src.contains("showToast(\"$gone\")"),
            )
        }
        for (kept in listOf(
            "showToast(\"No audio recorded\")",
            "showToast(\"No API keys configured",
            "showToast(\"Paste blocked",
        )) {
            assertTrue(
                "D11 keeps the non-recognition messages; missing: $kept",
                src.contains(kept),
            )
        }
    }

    // -----------------------------------------------------------------------
    // B1-Android / D-H3 — history and Turso only after a non-blocking verdict
    // -----------------------------------------------------------------------

    /**
     * The whole row is a statement ORDER: `saveToHistory` (Step 3) and
     * `pushToTurso` (Step 3b) used to run before Step 4's guard, so a dictation
     * into a banking app was blocked from the paste but had already been
     * written to `history.db` and pushed to the cloud.
     *
     * Asserted as offsets, with every needle proved to exist first — an
     * `indexOf` that returns -1 for a renamed symbol would otherwise "prove"
     * the guard comes first.
     */
    @Test
    fun bankingVerdictPrecedesTheHistoryAndTursoWrites() {
        val guard = src.indexOf("BankingGuard.shouldBlockPaste(")
        val blockedReturn = src.indexOf("return@post", startIndex = maxOf(guard, 0))
        val history = src.indexOf("KlarvoApi.saveToHistory(")
        val turso = src.indexOf("KlarvoApi.pushToTurso(")

        assertTrue("the banking guard must exist on the delivery path", guard >= 0)
        assertTrue("the blocked branch must still bail out", blockedReturn >= 0)
        assertTrue("the history write must exist", history >= 0)
        assertTrue("the Turso push must exist", turso >= 0)
        assertEquals(
            "the guard is read exactly once, on the delivery path",
            1,
            Regex(Regex.escape("BankingGuard.shouldBlockPaste(")).findAll(src).count(),
        )

        assertTrue(
            "B1-Android: the history write must come AFTER the banking verdict " +
                "(guard@$guard, saveToHistory@$history)",
            guard < history,
        )
        assertTrue(
            "B1-Android: the Turso push must come AFTER the banking verdict " +
                "(guard@$guard, pushToTurso@$turso)",
            guard < turso,
        )
        assertTrue(
            "B1-Android: the blocked branch must return BEFORE the first write " +
                "(return@post@$blockedReturn, saveToHistory@$history)",
            blockedReturn < history,
        )
    }

    /**
     * The writes were moved into a worker thread because the verdict is read on
     * the main looper (`BankingGuard`'s KDoc: `bankingAppActive` is
     * main-looper-owned). Without this the reorder would have put SQLite and a
     * network push on the UI thread.
     */
    @Test
    fun theWritesAreStillOffTheMainLooper() {
        val history = src.indexOf("KlarvoApi.saveToHistory(")
        assertTrue(history >= 0)
        val threadStart = src.lastIndexOf("Thread {", history)
        assertTrue(
            "the history write must sit inside a worker Thread, not on the looper",
            threadStart >= 0 && threadStart > src.indexOf("BankingGuard.shouldBlockPaste("),
        )
    }

    // -----------------------------------------------------------------------
    // D6-Android / D-M12 — caught, counted, never an uncaught main-thread throw
    // -----------------------------------------------------------------------

    /**
     * The catch semantics, EXECUTED: a throwing writer returns `false`, calls
     * `onFailure` exactly once with the throwable, and nothing escapes.
     *
     * `clipboardWriteFailed_showsNothingAndClaimsNothing` in
     * [CleanupFailureDeliveryTest] covers what the user then sees; this covers
     * the other two clauses of the matrix row.
     */
    @Test
    fun clipboardWriteIsCaughtCountedAndNeverPropagates() {
        val seen = mutableListOf<Throwable>()
        val boom = IllegalStateException("clipboard service died")

        val ok = KlarvoOverlayService.guardedClipboardWrite(
            write = { throw boom },
            onFailure = { seen.add(it) },
        )

        assertFalse("a failed clipboard write must report failure", ok)
        assertEquals("onFailure fires exactly once", 1, seen.size)
        assertEquals("…with the real throwable, so the log can name it", boom, seen[0])
    }

    /** The happy path must not report a failure or call the handler. */
    @Test
    fun clipboardWriteReportsSuccessWithoutTouchingTheFailurePath() {
        var wrote = 0
        var failures = 0
        val ok = KlarvoOverlayService.guardedClipboardWrite(
            write = { wrote++ },
            onFailure = { failures++ },
        )
        assertTrue(ok)
        assertEquals(1, wrote)
        assertEquals(0, failures)
    }

    /**
     * And the production wiring: `copyToClipboard` goes through that seam, and
     * its failure handler is what increments `pasteErrorCount` — the counter
     * `KlarvoApi.FeedbackMetrics` has always declared and serialised and that
     * nothing had ever incremented.
     *
     * A tripwire, because the handler body needs a `Context`, a `Thread` and
     * `android.util.Log`. Order-anchored: the increment must sit inside
     * `copyToClipboard`, not merely somewhere in the file.
     */
    @Test
    fun copyToClipboardRoutesThroughTheSeamAndCountsTheFailure() {
        val fn = src.indexOf("private fun copyToClipboard(")
        assertTrue("copyToClipboard must exist", fn >= 0)
        // The next top-level `private fun` bounds the body.
        val end = src.indexOf("\n    private fun ", fn + 1).let { if (it < 0) src.length else it }
        val body = src.substring(fn, end)

        assertTrue(
            "copyToClipboard must return Boolean so the delivery decision can read it",
            body.contains("private fun copyToClipboard(text: String): Boolean"),
        )
        assertTrue(
            "…through the guarded seam, so the throw cannot reach the main looper",
            body.contains("guardedClipboardWrite("),
        )
        assertTrue(
            "…and its failure handler increments pasteErrorCount (D-M12)",
            body.contains("pasteErrorCount = m.pasteErrorCount + 1"),
        )
    }

    // -----------------------------------------------------------------------
    // E1 / D-H9 — the flush-time re-check, not only the install decision
    // -----------------------------------------------------------------------

    /**
     * `ShouldInstallPreviewFlushTest` covers the install decision. This covers
     * the second guard the matrix row asks for: `flushPreviewDelta` re-reads
     * `sttProvider` **at flush time**, as `pipeline::flush_preview_delta` does,
     * because the config can change between starting a recording and the first
     * pause.
     *
     * Order is the claim, not existence: the guard must precede
     * `deltaSnapshotWav()` and the JNI call, or the audio is already on its way
     * out when it fires.
     */
    @Test
    fun previewFlushRechecksTheStoredSttProviderBeforeItTouchesAudio() {
        val fn = src.indexOf("private fun flushPreviewDelta(")
        assertTrue("flushPreviewDelta must exist", fn >= 0)
        val end = src.indexOf("\n    private fun ", fn + 1).let { if (it < 0) src.length else it }
        val body = src.substring(fn, end)

        val guard = body.indexOf("config.sttProvider == \"local\"")
        val snapshot = body.indexOf("deltaSnapshotWav()")
        val transcribe = body.indexOf("transcribeWithRetry(")

        assertTrue("E1: flushPreviewDelta must re-check the stored STT provider", guard >= 0)
        assertTrue("the delta snapshot must still be taken on the normal path", snapshot >= 0)
        assertTrue("the flush must still reach the JNI on the normal path", transcribe >= 0)
        assertTrue(
            "E1: the re-check must precede the delta snapshot (guard@$guard, snapshot@$snapshot)",
            guard < snapshot,
        )
        assertTrue(
            "E1: the re-check must precede the upload (guard@$guard, transcribe@$transcribe)",
            guard < transcribe,
        )
        assertTrue(
            "E1: the re-check must actually bail out, not just log",
            body.substring(guard, snapshot).contains("return"),
        )
    }

    // -----------------------------------------------------------------------
    // helpers
    // -----------------------------------------------------------------------

    private fun readOverlayServiceSource(): String {
        val rel = "android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt"
        val base = File(System.getProperty("user.dir") ?: ".")
        val candidates = listOf(
            "../../../../$rel", // gen/android/app/ -> repo root
            "../../../$rel",
            "../../$rel",
            "../$rel",
            rel,
        )
        return candidates.map { File(base, it) }.firstOrNull { it.isFile }?.readText()
            ?: error("KlarvoOverlayService.kt not found from ${base.absolutePath}")
    }

    /** Minimal, string-aware comment stripper (the `Adr0017BoundaryGuardTest` approach). */
    private fun stripComments(source: String): String {
        val out = StringBuilder()
        var i = 0
        var inString = false
        var inLine = false
        var inBlock = false
        while (i < source.length) {
            val c = source[i]
            val next = if (i + 1 < source.length) source[i + 1] else '\u0000'
            when {
                inLine -> if (c == '\n') { inLine = false; out.append(c) }
                inBlock -> if (c == '*' && next == '/') { inBlock = false; i++ }
                inString -> {
                    out.append(c)
                    if (c == '\\') { if (i + 1 < source.length) out.append(next); i++ }
                    else if (c == '"') inString = false
                }
                c == '/' && next == '/' -> { inLine = true; i++ }
                c == '/' && next == '*' -> { inBlock = true; i++ }
                c == '"' -> { inString = true; out.append(c) }
                else -> out.append(c)
            }
            i++
        }
        return out.toString()
    }
}
