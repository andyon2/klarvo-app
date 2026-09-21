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

    /**
     * And an `Error`, not just an `Exception`. The row promises "never an
     * uncaught main-thread exception", and an OEM clipboard service can
     * surface as a `LinkageError` behind a provider stub — which
     * `catch (e: Exception)` lets straight past onto the looper. Review
     * finding; the catch is `Throwable`.
     */
    @Test
    fun clipboardWriteAlsoCatchesAnError() {
        var failures = 0
        val ok = KlarvoOverlayService.guardedClipboardWrite(
            write = { throw UnsatisfiedLinkError("no clipboard shim") },
            onFailure = { failures++ },
        )
        assertFalse(ok)
        assertEquals(1, failures)
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
    // D10 / D-M2 — the ladder gate has no type pre-filter
    // -----------------------------------------------------------------------

    /**
     * Removing the `e is IOException &&` pre-gate **is** D10's behavioural
     * half: a malformed provider answer throws a bare `JSONException`, which
     * is not an `IOException`, so on a dictation under `CHUNK_THRESHOLD` the
     * provider ladder never ran while the chunked path fell back normally.
     *
     * `isRetryableCleanupFailure`'s own verdicts are unit-tested in
     * [TestProviderScenarioTest]; what nothing asserted is that the CALL SITE
     * still reaches it without a type filter in front. Restoring the pre-gate
     * kept every test and the fixture green.
     *
     * Asserted on the assignment's condition text, and paired with the
     * then-branch so a gate that is "clean" because it never calls the ladder
     * cannot pass.
     */
    @Test
    fun cleanupLadderGateHasNoTypePreFilter() {
        val assign = src.indexOf("val fallbackProvider = if (")
        assertTrue("the cleanup-fallback gate must exist", assign >= 0)
        val condEnd = src.indexOf(") {", assign)
        assertTrue(condEnd > assign)
        val condition = src.substring(assign + "val fallbackProvider = if (".length, condEnd).trim()

        assertEquals(
            "D10: the ladder gate must be isRetryableCleanupFailure(e) ALONE — " +
                "a type pre-filter is the defect, not a safeguard",
            "isRetryableCleanupFailure(e)",
            condition,
        )
        assertFalse(
            "D10: no `e is …` pre-filter in front of the gate",
            condition.contains(" is "),
        )

        // …and the gate must still lead somewhere, or "no pre-filter" would be
        // satisfied by a branch that never resolves a fallback at all.
        val thenEnd = src.indexOf("} else {", condEnd)
        assertTrue(thenEnd > condEnd)
        assertTrue(
            "the gate must still reach resolveFallbackLlmProvider",
            src.substring(condEnd, thenEnd).contains("KlarvoApi.resolveFallbackLlmProvider("),
        )
    }

    // -----------------------------------------------------------------------
    // B3 — the new native symbol must not be able to kill the worker thread
    // -----------------------------------------------------------------------

    /**
     * `nativeStripStockphraseGhosts` is a NEW native symbol, and a stale
     * `libklarvo_lib.so` raises `UnsatisfiedLinkError` — an `Error`, which
     * `processAudio`'s outer `catch (e: IOException)` does not catch.
     * Unguarded, the worker thread dies there, AFTER the paid STT and LLM
     * calls: nothing pasted, nothing stored, bubble stranded in TRANSCRIBING.
     * Losing a ghost is the pre-13-2 Android behaviour; losing the dictation
     * is not. Review finding.
     */
    @Test
    fun theGhostStripBridgeCallDegradesInsteadOfKillingTheRun() {
        val call = src.indexOf("GroqSttBridge.nativeStripStockphraseGhosts(")
        assertTrue("the post-cleanup ghost strip must exist", call >= 0)

        // The call must be the FIRST statement of its own `try`, not merely
        // somewhere downstream of an unrelated one. Measured 2026-09-21: the
        // `lastIndexOf("try {")` form of this assertion stayed GREEN when the
        // guard was removed, because `processAudio` has several earlier `try`
        // blocks.
        val tryStart = src.lastIndexOf("try {", call)
        assertTrue("the bridge call must sit inside a try", tryStart in 0 until call)
        assertTrue(
            "…as its FIRST statement, so nothing between the try and the call: " +
                src.substring(tryStart, call).trim(),
            src.substring(tryStart + "try {".length, call).isBlank(),
        )
        val catchStart = src.indexOf("catch (", call)
        assertTrue("…with a catch after it", catchStart > call)
        val catchClause = src.substring(catchStart, minOf(catchStart + 40, src.length))
        assertTrue(
            "…catching Throwable, not Exception — UnsatisfiedLinkError is an Error: $catchClause",
            catchClause.contains("Throwable"),
        )
        assertTrue(
            "…and degrading to the un-stripped text, not to nothing",
            src.substring(catchStart, minOf(catchStart + 400, src.length)).contains("finalText"),
        )
    }

    /**
     * The live accessibility reference is read ONCE per delivery.
     *
     * Read twice, the service can disconnect between the connected-check and
     * the paste: `shouldAttemptPaste` says yes, the paste is skipped by the
     * null-safe call, and `decideDelivery` sees `attempted = true` with a null
     * outcome — a delivery that shows no success and logs no cause, which is
     * the ending D4 exists to remove. Review finding.
     */
    @Test
    fun theLiveAccessibilityReferenceIsReadOnce() {
        val reads = Regex(Regex.escape("KlarvoAccessibilityService.instance")).findAll(src).count()
        assertEquals(
            "the delivery block must hoist the service reference into one local",
            1,
            reads,
        )
        assertTrue(
            "…and that local is what the paste is called on",
            src.contains("accessibility?.pasteIntoFocusedField()"),
        )
    }

    // -----------------------------------------------------------------------
    // One rule, one literal — the three G2a guards
    // -----------------------------------------------------------------------

    /**
     * `"local"` was spelled out three times — [KlarvoOverlayService.skipsCloudCleanup],
     * `RecordingMode.shouldInstallPreviewFlush` and `flushPreviewDelta`'s
     * flush-time re-check — in the story whose whole thesis is one rule. A
     * provider-id rename would have disabled two of the three G2a guards and
     * left the third reporting green. Review finding.
     *
     * The STT DISPATCH branch (`config.sttProvider == "local"` in
     * `processAudio`, which chooses local Whisper) is deliberately not part of
     * this: it is a provider selector, not an offline guard, and 13-1b's
     * tripwire pins it as a literal.
     */
    @Test
    fun theThreeOfflineGuardsShareOneProviderIdLiteral() {
        assertEquals("local", KlarvoOverlayService.LOCAL_PROVIDER_ID)

        // Scoped to the three guard BODIES. The file holds other, unrelated
        // `"local"` comparisons that are deliberately literals and are NOT
        // G2a guards: the two "a Groq key is required unless STT is local"
        // checks, and `processAudio`'s STT DISPATCH branch, which 13-1b's own
        // tripwire pins as a literal. A whole-file assertion would have
        // demanded those change too.
        for (name in listOf("skipsCloudCleanup", "shouldInstallPreviewFlush", "flushPreviewDelta")) {
            val body = bodyOf(name)
            assertTrue(
                "$name must read KlarvoOverlayService.LOCAL_PROVIDER_ID",
                body.contains("LOCAL_PROVIDER_ID"),
            )
            assertFalse(
                "$name must not carry its own copy of the provider id: $body",
                body.contains("\"local\""),
            )
        }
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

        val guard = body.indexOf("config.sttProvider == LOCAL_PROVIDER_ID")
        val snapshot = body.indexOf("deltaSnapshotWav()")
        val transcribe = body.indexOf("transcribeWithRetry(")

        assertTrue(
            "E1: flushPreviewDelta must re-check the stored STT provider against " +
                "KlarvoOverlayService.LOCAL_PROVIDER_ID (the one literal all three G2a guards read)",
            guard >= 0,
        )
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

    /**
     * The body of `fun <name>(...)`, by brace matching from its first `{`.
     *
     * The other order assertions in this file bound a body with "the next
     * top-level `private fun`", which does not work for a function nested in
     * the companion object. This does, and it scopes tightly enough that an
     * unrelated occurrence elsewhere in the file cannot satisfy an assertion.
     *
     * Handles an expression body (`fun f(...) = expr`) too: those have no
     * brace, so the statement up to the next blank line is the body.
     */
    private fun bodyOf(name: String): String {
        val decl = Regex("fun\\s+" + Regex.escape(name) + "\\s*\\(").find(src)
            ?: error(name + " must exist in KlarvoOverlayService.kt")
        val brace = src.indexOf('{', decl.range.last)
        val eq = src.indexOf('=', decl.range.last)
        val paramsEnd = src.indexOf("):", decl.range.last)
        val exprBody = eq in 0 until brace && paramsEnd in 0 until eq
        if (brace < 0 || exprBody) {
            val stop = src.indexOf("\n\n", decl.range.last).let { if (it < 0) src.length else it }
            return src.substring(decl.range.first, stop)
        }
        var depth = 0
        var i = brace
        while (i < src.length) {
            when (src[i]) {
                '{' -> depth++
                '}' -> {
                    depth--
                    if (depth == 0) return src.substring(brace, i + 1)
                }
            }
            i++
        }
        error("unbalanced braces reading the body of " + name)
    }

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
