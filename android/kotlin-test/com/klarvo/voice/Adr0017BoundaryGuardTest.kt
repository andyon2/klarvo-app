package com.klarvo.voice

import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.File

/**
 * ADR-0017 boundary guard (story 7-8, AC2 — the promise 7-3 AC9 made but did not build).
 *
 * ## The hard rule this enforces
 * Shared STT request + guard logic lives **only** in the Rust core, consumed over the
 * JNI bridge (`src-tauri/src/stt/groq_jni.rs`: `nativeTranscribe`, `nativeIsHallucination`,
 * `nativeIsPromptEcho`, `nativeStripPromptFragments`, `nativeSilenceCheck`). Story 7-3
 * deleted the Kotlin twins — `KlarvoApi.transcribe`, `buildMultipartBody`,
 * `HallucinationFilter.kt`, `SilencePreFilter.kt` — and left no guard against re-growth.
 * This test is that guard.
 *
 * > Naming note: ADR-0017's Decision text names `src-tauri/src/stt/jni_bridge.rs` as the
 * > consumption point. On today's tree that file holds only the `LocalWhisperInference`
 * > entry points; the actual Groq STT + guard JNI surface is `groq_jni.rs`. This comment
 * > points at the real file. The ADR itself is deliberately not edited here.
 *
 * ## Why this is not a grep
 * A plain `grep -r 'HallucinationFilter\|SilencePreFilter\|buildMultipartBody'` over
 * `android/` is **RED on a clean tree**: those names legitimately survive in KDoc and
 * comments that document the deletion (`GroqSttBridge.kt:7-8,63`,
 * `KlarvoOverlayService.kt:2073,2632`). A guard that is red on a clean tree is worthless.
 * So this guard strips comments and string-aware-scans the remaining **code**, then asserts
 * on **declarations**, not on the mere appearance of a name.
 *
 * ## Deliberately allowed (must NOT trip)
 * **No rule allowlists any file.** The files below are safe not because they are exempted
 * but because no rule keys on their vocabulary — which is strictly stronger, and is why the
 * `audio/transcriptions` rule really does mean "anywhere in production `voice/`" — the scan
 * scope stated under "What this covers" below:
 * - `GroqSttBridge.kt` — this **is** the sanctioned JNI bridge, the thing being protected.
 *   It declares `external fun nativeTranscribe`; no rule matches that.
 * - `LocalWhisperInference.kt` — the dormant local-whisper path; 7-3 ruled it a different
 *   surface, not to be deleted. Its `transcribeAudio`/`nativeTranscribe` are not Groq STT,
 *   and it is **not** the JNI bridge — so exempting it from the endpoint rule would have
 *   opened exactly the hole AC2 forbids (7-8 review round 1).
 * - `KlarvoOverlayService.transcribeWithRetry` — ADR-0017 deliberately keeps the retry/4xx
 *   loop in Kotlin. It is a retry wrapper, not an STT request implementation, so no rule
 *   below keys on it.
 * - `MainActivity`'s mic-permission copy and `KlarvoTheme`'s provenance line contain the
 *   word "transcribe". Pure grep false positives; no rule keys on that word.
 *
 * ## What this covers — and what it does NOT
 * Covers: the production Kotlin files directly under `android/kotlin-src/com/klarvo/voice/`
 * only, for four shapes — a `class`/`object`/`interface` named
 * `HallucinationFilter`/`SilencePreFilter`, a `fun buildMultipartBody`, a
 * `multipart/form-data` literal, and an `audio/transcriptions` literal — the latter two in
 * **any** of those files, no exemptions — plus re-appearance of the two deleted files by name.
 * Does NOT cover: `android/kotlin-test/` (test sources may name the deleted twins in
 * prose), the Rust side, `gen/android/` (generated, gitignored), semantic re-implementations
 * that avoid all four shapes (e.g. a hand-rolled body builder under a different name), or
 * whether the bridge itself is correct. It is a re-growth tripwire, not a proof of
 * architectural purity.
 */
class Adr0017BoundaryGuardTest {

    private fun kotlinSrcDir(): File {
        val cwd = File(System.getProperty("user.dir") ?: ".")
        val name = "android/kotlin-src/com/klarvo/voice"
        val candidates = listOf(
            cwd.resolve("../../../../$name"), // gen/android/app/ → repo root
            cwd.resolve(name),
            cwd.resolve("../$name"),
            cwd.resolve("../../../$name"),
        )
        return candidates.firstOrNull { it.canonicalFile.isDirectory }?.canonicalFile
            ?: error(
                "Cannot find $name. Tried:\n" +
                    candidates.joinToString("\n") { "  ${it.canonicalPath}" } +
                    "\nCWD=${cwd.canonicalPath}"
            )
    }

    /**
     * Strips `//` line comments and block/KDoc comments, preserving string literals.
     *
     * String-aware on purpose: a naive `substringBefore("//")` would truncate the line at
     * the `//` inside `"https://api.deepseek.com/..."` and silently hide whatever followed
     * — the exact class of false-negative this guard must not have. Newlines are preserved
     * so reported line numbers stay meaningful.
     */
    private fun stripComments(src: String): String {
        val out = StringBuilder(src.length)
        var i = 0
        while (i < src.length) {
            val c = src[i]
            val next = if (i + 1 < src.length) src[i + 1] else ' '
            when {
                // Raw string """ ... """
                c == '"' && src.startsWith("\"\"\"", i) -> {
                    val end = src.indexOf("\"\"\"", i + 3)
                    val stop = if (end < 0) src.length else end + 3
                    out.append(src, i, stop)
                    i = stop
                }
                // Regular string " ... " (escape-aware, never crosses a newline)
                c == '"' -> {
                    out.append(c); i++
                    while (i < src.length && src[i] != '"' && src[i] != '\n') {
                        if (src[i] == '\\' && i + 1 < src.length) { out.append(src[i]); i++ }
                        out.append(src[i]); i++
                    }
                    if (i < src.length) { out.append(src[i]); i++ }
                }
                // Char literal ' ... '
                c == '\'' -> {
                    out.append(c); i++
                    while (i < src.length && src[i] != '\'' && src[i] != '\n') {
                        if (src[i] == '\\' && i + 1 < src.length) { out.append(src[i]); i++ }
                        out.append(src[i]); i++
                    }
                    if (i < src.length) { out.append(src[i]); i++ }
                }
                c == '/' && next == '/' -> {
                    while (i < src.length && src[i] != '\n') i++
                }
                c == '/' && next == '*' -> {
                    // Kotlin block comments nest.
                    var depth = 1; i += 2
                    while (i < src.length && depth > 0) {
                        if (src.startsWith("/*", i)) { depth++; i += 2 }
                        else if (src.startsWith("*/", i)) { depth--; i += 2 }
                        else { if (src[i] == '\n') out.append('\n'); i++ }
                    }
                }
                else -> { out.append(c); i++ }
            }
        }
        return out.toString()
    }

    private data class Hit(val file: String, val line: Int, val text: String, val rule: String)

    private fun scan(): List<Hit> {
        val dir = kotlinSrcDir()
        val files = dir.listFiles { f: File -> f.isFile && f.name.endsWith(".kt") }
            ?: error("no .kt files under ${dir.path}")
        check(files.isNotEmpty()) { "no .kt files under ${dir.path} — resolver pointed at the wrong tree" }

        // No rule carries a file allowlist (7-8 review round 1). An earlier draft exempted
        // `GroqSttBridge.kt` + `LocalWhisperInference.kt` from the two literal rules, but
        // `LocalWhisperInference.kt` is not the JNI bridge, so that exemption weakened AC2's
        // "outside the JNI bridge" requirement into "anywhere except two files" — and bought
        // nothing: neither literal appears anywhere under kotlin-src. Every rule now applies
        // to every file, and the guard is still green.
        val rules: List<Pair<String, Regex>> = listOf(
            Pair(
                "Kotlin re-implementation of a deleted STT guard (ADR-0017: Rust-only)",
                Regex("""\b(class|object|interface)\s+(HallucinationFilter|SilencePreFilter)\b""")
            ),
            Pair(
                "Kotlin multipart STT request body (ADR-0017: the Rust core owns the request)",
                Regex("""\bfun\s+buildMultipartBody\b""")
            ),
            Pair(
                "multipart/form-data literal — an STT request body being built in Kotlin",
                Regex("""multipart/form-data""")
            ),
            Pair(
                "audio/transcriptions endpoint reached from Kotlin instead of the JNI bridge",
                Regex("""audio/transcriptions""")
            ),
        )

        val hits = mutableListOf<Hit>()
        for (f in files) {
            val code = stripComments(f.readText())
            code.lineSequence().forEachIndexed { idx, rawLine ->
                for ((ruleName, pattern) in rules) {
                    if (pattern.containsMatchIn(rawLine)) {
                        hits += Hit(f.name, idx + 1, rawLine.trim(), ruleName)
                    }
                }
            }
        }
        return hits
    }

    @Test
    fun noKotlinSttRequestOrGuardTwinHasRegrown() {
        val hits = scan()
        assertTrue(
            buildString {
                append("ADR-0017 boundary violated — Kotlin STT request/guard logic has re-grown.\n")
                append("The Rust core owns this (src-tauri/src/stt/groq_jni.rs); call it over GroqSttBridge.\n\n")
                hits.forEach { append("  ${it.file}:${it.line}  [${it.rule}]\n    ${it.text}\n") }
            },
            hits.isEmpty()
        )
    }

    @Test
    fun deletedTwinFilesHaveNotReappeared() {
        val dir = kotlinSrcDir()
        val forbidden = listOf("HallucinationFilter.kt", "SilencePreFilter.kt")
        val present = forbidden.filter { dir.resolve(it).exists() }
        assertTrue(
            "Story 7-3 deleted these Kotlin twins; the shared logic lives in the Rust core " +
                "and is reached via GroqSttBridge. Re-appeared: $present",
            present.isEmpty()
        )
    }

    /**
     * Meta-test: proves the comment stripper actually removes the KDoc that documents the
     * deletion, and — critically — that it does NOT truncate a line at the `//` inside a URL
     * string. Without this, the guard could be silently blind and still report green.
     */
    @Test
    fun stripperRemovesCommentsButPreservesStringsWithDoubleSlash() {
        val sample = """
            val url = "https://api.deepseek.com/v1/chat/completions" // trailing comment
            /** KDoc: HallucinationFilter and SilencePreFilter are deleted. */
            // line comment: buildMultipartBody
            class Real
        """.trimIndent()
        val stripped = stripComments(sample)
        assertTrue("URL string must survive intact", stripped.contains("https://api.deepseek.com/v1/chat/completions"))
        assertTrue("code after a URL-bearing line must survive", stripped.contains("class Real"))
        assertTrue("KDoc mention must be stripped", !stripped.contains("HallucinationFilter"))
        assertTrue("line-comment mention must be stripped", !stripped.contains("buildMultipartBody"))
        assertTrue("trailing comment must be stripped", !stripped.contains("trailing comment"))
    }
}
