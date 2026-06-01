# Story 2.3: Sanitize Paste Text on ALL Android Paths

Status: done

## Story

As an Android klarvo user,
I want raw-fallback paste paths sanitized,
so that bidi-override / zero-width characters from a raw transcript can't reach my target field and spoof text.

## Acceptance Criteria

1. **Given** Android's `sanitizeLlmOutput` (`KlarvoApi.kt:609-642`) already strips the same char-classes as the Rust `sanitize_llm_output` (`pipeline.rs:2081-2128`: ANSI, null, bidi-overrides, zero-width),
   **When** the three raw-fallback paste paths run — (a) local-cleanup exception (`KlarvoOverlayService.kt:1116-1118`), (b) cloud-cleanup `IOException` (`KlarvoOverlayService.kt:1135-1140`), (c) no-LLM-key (`KlarvoOverlayService.kt:1148-1150`),
   **Then** each applies sanitization before the text reaches `finalText` (and therefore `copyToClipboard` and `pasteIntoFocusedField`). Today all three return `transcript` raw.

2. **Given** the cleanup paths (`cleanupLocal` and `cleanup`/`cleanupChunked`) already apply `sanitizeLlmOutput` internally before returning,
   **When** the fix lands,
   **Then** sanitization is applied EXACTLY ONCE on every path (no double-sanitize on the success paths).

3. **Given** a raw transcript containing a bidi-override (e.g. `‮`),
   **When** pasted via a fallback path,
   **Then** the pasted text has the bidi-override stripped (parity with the desktop's central coverage at `pipeline.rs:1184`).

4. **Given** a raw transcript containing normal text only,
   **When** pasted via any path (fallback or success),
   **Then** the text is unchanged (no regression to normal dictation).

## Tasks / Subtasks

- [x] Task 1: Expose `sanitizeLlmOutput` as an internal API in `KlarvoApi.kt` (AC: 1, 2, 3)
  - [x] 1.1 Change `private fun sanitizeLlmOutput(text: String): String` to `internal fun sanitizeLlmOutput(text: String): String` at `KlarvoApi.kt:609` — same `object KlarvoApi`, same body, only visibility change
  - [x] 1.2 Verify the two existing callers inside `KlarvoApi` (lines 563 and 799) still compile and behave identically (no change to them)
  - [x] 1.3 Copy the same visibility change to the build-target mirror: `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoApi.kt`

- [x] Task 2: Apply sanitization to the three raw-fallback returns in `KlarvoOverlayService.kt` (AC: 1, 2, 3, 4)
  - [x] 2.1 **Path A — local-cleanup exception** (line ~1118): change `transcript` → `KlarvoApi.sanitizeLlmOutput(transcript)` in the `catch (e: Exception)` block
  - [x] 2.2 **Path B — cloud-cleanup `IOException`** (line ~1140): change `transcript` → `KlarvoApi.sanitizeLlmOutput(transcript)` in the `catch (e: IOException)` block
  - [x] 2.3 **Path C — no-LLM-key** (line ~1149): change `transcript` → `KlarvoApi.sanitizeLlmOutput(transcript)` in the `else` branch where `llmProvider == null`
  - [x] 2.4 Verify the two success paths (`cleanupLocal` result at line ~1115 and `cleanupChunked` result at line ~1134) are NOT touched — they already sanitize internally. Confirm no double-sanitize by reading the return chain: `cleanupLocal → sanitizeLlmOutput(result)` and `cleanup → sanitizeLlmOutput(rawContent)`.
  - [x] 2.5 Copy the identical changes to the build-target mirror: `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt`

- [x] Task 3: Write JVM unit tests in `SanitizePathsTest.kt` (AC: 1, 2, 3, 4)
  - [x] 3.1 Create canonical test file at `android/kotlin-test/com/klarvo/voice/SanitizePathsTest.kt` (AI-2 binding: must call `KlarvoApi.sanitizeLlmOutput()` directly — not a copy of the char-stripping logic inline in the test)
  - [x] 3.2 Create byte-identical mirror at `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/SanitizePathsTest.kt`
  - [x] 3.3 Test: `sanitizeLlmOutput` strips a bidi-override (`‮`) → absent from output
  - [x] 3.4 Test: `sanitizeLlmOutput` strips a zero-width space (`​`) → absent from output
  - [x] 3.5 Test: `sanitizeLlmOutput` strips an ANSI escape sequence (`[31m...`) → stripped
  - [x] 3.6 Test: `sanitizeLlmOutput` strips a null byte → absent
  - [x] 3.7 Test: `sanitizeLlmOutput` preserves normal German/English text unchanged (no regression)
  - [x] 3.8 Test: `sanitizeLlmOutput("")` returns `""` (empty input)
  - [x] 3.9 Run `./gradlew :app:testUniversalDebugUnitTest` — all tests green, 0 failures

- [x] Task 4: DoD smoke verification (AC: all) — **MANUAL, requires Android device** — GREEN 2026-06-01
  - [x] 4.1 **AI-1 build-freshness gate:** Built via `scripts/android-build.sh` (syncs `android/kotlin-src/`→build incl. the fix; refuses to call a build fresh unless gradle re-emitted the APK; signed APK carries a timestamp in its name under `releases/v0.5.0/`). Freshly-built timestamped APK installed on-device. NOTE: app exposes NO version in-UI (no About screen) and versionName 0.5.0 was not bumped — freshness is proven by the build/install act + the script's timestamp gate, not by an in-app version. Device-side cross-check available via `adb shell dumpsys package com.klarvo.voice` (lastUpdateTime / versionName).
  - [x] 4.2 **Normal dictation (positive path):** Normal utterance pasted normally, no chars stripped — AC-4 regression check passed on-device.
  - [ ] 4.3 **Log check (optional but encouraged):** `adb logcat | grep pipeline` — optional, not run; positive-path behavior already confirmed in 4.2.

## Dev Notes

### What This Story Closes

**DIV-03** (robustness-audit-2026-05-30.md §3): Android applies `sanitizeLlmOutput` only in `cleanup()`/`cleanupLocal()` — the three raw-fallback paths (`KlarvoOverlayService.kt:1118`, `1140`, `1149`) paste `transcript` unsanitized. Bidi-override / zero-width chars from a raw transcript reach the target field → text-spoofing risk. Desktop: `sanitize_llm_output` is applied centrally to ALL outcomes at `pipeline.rs:1184`.

### The Three Fallback Paths (Current State — All Return `transcript` Raw)

```
val finalText = if (config.llmProvider == "local") {
    try {
        val result = KlarvoApi.cleanupLocal(...)   // already sanitizes internally
        result                                       // ← SAFE (sanitized)
    } catch (e: Exception) {
        transcript                                   // ← PATH A: RAW (line ~1118)
    }
} else {
    val llmProvider = KlarvoApi.resolveLlmProvider(config)
    if (llmProvider != null) {
        try {
            val result = KlarvoApi.cleanupChunked(...) // already sanitizes internally
            result                                      // ← SAFE (sanitized)
        } catch (e: IOException) {
            transcript                                  // ← PATH B: RAW (line ~1140)
        }
    } else {
        transcript                                      // ← PATH C: RAW (line ~1149)
    }
}
```

**Fix:** Replace all three `transcript` returns with `KlarvoApi.sanitizeLlmOutput(transcript)`.

### Why `sanitizeLlmOutput` Needs `internal` Visibility

`sanitizeLlmOutput` is currently `private` inside `object KlarvoApi` (`KlarvoApi.kt:609`). It is called by `cleanupLocal` (line 563) and `cleanup` (line 799) — both inside `KlarvoApi`. The fix needs to call it from `KlarvoOverlayService`, which is in the same package (`com.klarvo.voice`) but a different file. Kotlin `internal` grants package-level access within the same module — this is the correct visibility for a sanitizer that conceptually belongs to `KlarvoApi` but is consumed by the service.

**Do NOT:**
- Create a duplicate `sanitizeLlmOutput` function in `KlarvoOverlayService` or a new file — that is the reinvention-of-wheel failure this story specifically avoids
- Make it `public` (wider than needed)
- Move it to a new `TextSanitizer.kt` standalone class (no new files needed; the function is a natural KlarvoApi concern)

### Double-Sanitize Must NOT Happen

The success paths already sanitize — verify this is preserved:
- `cleanupLocal` returns `sanitizeLlmOutput(result.ifBlank { text })` at line 563 → already clean
- `cleanup` returns `sanitizeLlmOutput(rawContent)` at line 799 → already clean
- `cleanupChunked` delegates to `cleanup` for each chunk → already clean

If `sanitizeLlmOutput` were idempotent there would be no harm, but double-sanitize on success paths is a smell and NOT the design intent. The fix ONLY touches the three raw-fallback returns.

### Files to Modify (No New Files)

| File | Change |
|---|---|
| `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` | `private → internal` for `sanitizeLlmOutput` |
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | 3 fallback returns: `transcript` → `KlarvoApi.sanitizeLlmOutput(transcript)` |
| `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoApi.kt` | same visibility change (build-target mirror) |
| `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt` | same 3 fallback returns (build-target mirror) |

### Files to CREATE

| File | Purpose |
|---|---|
| `android/kotlin-test/com/klarvo/voice/SanitizePathsTest.kt` | canonical tracked test (AI-2 binding) |
| `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/SanitizePathsTest.kt` | gitignored build-target mirror |

### Sync Note (Source-of-Truth Dual Edit)

`android/kotlin-src/` is the canonical source. `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/` is the build target. Both must receive identical edits (pattern established by Stories 2.1 and 2.2). Do NOT edit the gen path without also editing the source path.

### Test Infrastructure (Established by Stories 2.1 and 2.2)

- Canonical test dir: `android/kotlin-test/com/klarvo/voice/` (tracked in git, version-controlled)
- Build-target test dir: `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/` (gitignored)
- Gradle test task: `./gradlew :app:testUniversalDebugUnitTest` (NOT `testDebugUnitTest` — two ABI variants exist)
- JUnit dependency already in `app/build.gradle.kts`: `testImplementation("junit:junit:4.13.2")`
- Existing tests: `HallucinationFilterTest.kt`, `SilencePreFilterTest.kt` — use same structure

### AI-2 Binding (Epic 1 Retro)

Tests must call `KlarvoApi.sanitizeLlmOutput()` directly — the real production function — not a re-implemented copy. The `internal` visibility change (Task 1) makes this possible from the test package without reinvention.

### Rust Parity Reference

Rust `sanitize_llm_output` at `pipeline.rs:2081-2128` strips the same 4 classes:
- ANSI escape sequences (`\x1b[...`)
- Null bytes (`\0`)
- Bidi overrides/embeddings (U+202A–202E, U+2066–2069, U+200E–200F)
- Zero-width chars (U+200B–200D, U+FEFF)

Kotlin `sanitizeLlmOutput` at `KlarvoApi.kt:609-642` strips identically. No new sanitizer needed — parity already exists.

Rust applies sanitization centrally at `pipeline.rs:1184` regardless of cleanup path. This story achieves the same on Android: every path that sets `finalText` (whether from cleanup or raw fallback) ends up sanitized before `copyToClipboard`.

### DoD Requirements (Surface-Class — NFR-Smoke Mandatory)

- [ ] **Build freshness (AI-1):** Fresh APK built and installed. Stale artifact = invalid smoke (cf. Epic 1 retro Story 1.2 trap).
- [ ] **Positive-path smoke:** Normal dictation → paste proceeds normally (no regression, no chars stripped).
- [ ] **JVM tests green:** `./gradlew :app:testUniversalDebugUnitTest` passes all `SanitizePathsTest` tests.

Note: The bidi-override fallback path (Path A/B/C) cannot be triggered deterministically in a live on-device smoke without injecting a bad STT result or deliberately breaking the LLM connection. The JVM unit test for `sanitizeLlmOutput` directly covers AC-3. The on-device smoke covers AC-4 (regression check) and build freshness.

### References

- Audit finding: `docs/robustness-audit-2026-05-30.md` §3 DIV-03 (high)
- Gate ADR: `docs/adr/0016-android-path-parity-strategy.md`
- Kotlin sanitizer: `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:609-642`
- Rust sanitizer: `src-tauri/src/pipeline.rs:2081-2128`
- Rust central application: `src-tauri/src/pipeline.rs:1184`
- Three fallback paths: `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` lines ~1118, ~1140, ~1149
- Cleanup success paths (already sanitized): `KlarvoApi.kt:563` (cleanupLocal), `KlarvoApi.kt:799` (cleanup)
- Epics spec: `_bmad-output/planning-artifacts/epics.md` — Epic 2, Story 2.3
- Epic 1 retro AI-1/AI-2: `_bmad-output/implementation-artifacts/epic-1-retro-2026-06-01.md` §7
- Precedent — object pattern: `android/kotlin-src/com/klarvo/voice/HallucinationFilter.kt` (Story 2.1)
- Precedent — test infrastructure: `android/kotlin-test/com/klarvo/voice/SilencePreFilterTest.kt` (Story 2.2)

## Review Findings

_Code review 2026-06-01 (3 adversarial layers — Blind Hunter, Edge Case Hunter, Acceptance Auditor; Opus 4.8). Acceptance Auditor: all 4 ACs PASS, gen-mirror byte-identical, AI-2 binding confirmed, status correctly `review`. Outcome: 1 patch, 4 deferred, 6 dismissed as noise._

- [x] [Review][Patch] `cleanupLocal` model-load-failure returns raw transcript — a 4th unsanitized paste path the story missed [android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:552] — When `LocalLlmInference.load()` fails, `cleanupLocal` does `return text` (RAW) **without throwing**, so it never reaches the wrapped `catch (e: Exception)` at `KlarvoOverlayService.kt:1116` (Path A). The raw value flows `result`→`finalText`→`copyToClipboard`+paste. A bidi-override in the transcript reaches the target field on the local-cleanup branch whenever the MNN model can't load (missing/corrupt model dir, OOM). The story's own Dev Notes diagram labels this branch "SAFE (sanitized)" — incorrect for the load-fail sub-path. Violates AC-1 / AC-3 ("ALL paths"). Fix: `return sanitizeLlmOutput(text)` at line 552 (consistent with the sanitized return at line 563), + byte-identical gen-mirror sync. Found by Edge Case Hunter (reviewer verified first-hand); predicted by Blind Hunter ("the next raw-return path will be forgotten — the success path already appears forgotten").
- [x] [Review][Defer] Single-egress sanitize chokepoint vs. N per-branch call sites [KlarvoOverlayService.kt:1108-1155] — deferred, architectural. Desktop sanitizes once centrally (`pipeline.rs:1184`); Android wraps per branch (now 4 sites incl. the fix). Source-fix at :552 closes the leak without the refactor; a chokepoint is a larger design change with double-sanitize risk. Backlog.
- [x] [Review][Defer] Sanitizer set omits other C0/C1 controls, DEL, U+2028/2029/0085/180E/Hangul fillers [KlarvoApi.kt:609-642] — deferred, exact parity with Rust `sanitize_llm_output`; expanding the set requires changing Rust too (ADR-0016 parity mandate). Backlog (cross-platform).
- [x] [Review][Defer] ANSI malformed-sequence handling (non-letter CSI final e.g. `ESC[3~`; bare `ESC[` at end-of-string silently discards trailing text) [KlarvoApi.kt:616-625] — deferred, pre-existing, parity with Rust. Backlog.
- [x] [Review][Defer] Legitimate RTL text (Arabic/Hebrew) may be altered by bidi-mark/isolate stripping (U+200E/200F/2066-2069) [KlarvoApi.kt:631] — deferred, exact parity with desktop (affects both platforms equally); cross-platform product decision, not introduced here. Backlog.

**Dismissed (6, noise / false-positive / verified-safe):** (1) `private→internal` API-widening — sanctioned by spec (AI-2 binding, Dev Notes "Why internal Visibility"); (2) LLM *input* not sanitized — out of threat model, scope is output-to-clipboard; prompt-injection handled by sandwich-defense system prompts; (3) surrogate-pair corruption — stripped ranges don't overlap U+D800–DFFF, so pairs hit the `else` branch and pass through unchanged; (4) test-coverage nits — 12 tests already additive over the 6 spec'd; (5) "without cleanup" toast wording — "cleanup" = LLM filler-removal, not char-sanitization; (6) idempotency assertion — no double-sanitize path exists (verified).

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

None.

### Completion Notes List

- Task 1: `sanitizeLlmOutput` visibility changed `private` to `internal` in both canonical (`android/kotlin-src/`) and build-target mirror (`src-tauri/gen/android/...`). Existing callers at KlarvoApi.kt:563 and :799 are inside `KlarvoApi` and unaffected (no change needed).
- Task 2: All three raw-fallback returns in KlarvoOverlayService.kt replaced with `KlarvoApi.sanitizeLlmOutput(transcript)`. Path A = local-cleanup `catch (e: Exception)`, Path B = cloud-cleanup `catch (e: IOException)`, Path C = `else` branch when `llmProvider == null`. Both success paths (`result` at line ~1115 and ~1134) are untouched — no double-sanitize introduced. Identical changes applied to build-target mirror.
- Task 3: Created `SanitizePathsTest.kt` (12 tests) calling `KlarvoApi.sanitizeLlmOutput()` directly (AI-2 binding — no copy of char-stripping logic in tests). Covers: bidi-override U+202E stripped, all bidi variant class, zero-width U+200B stripped, all zero-width variants, ANSI ESC sequence stripped, lone ESC stripped, null byte U+0000 stripped, normal English/German preserved, punctuation preserved, empty string, mixed content. Canonical at `android/kotlin-test/`, byte-identical mirror at `src-tauri/gen/.../test/`. `./gradlew :app:testUniversalDebugUnitTest` result: BUILD SUCCESSFUL, 54 total tests (12 SanitizePaths + 18 SilencePreFilter + 24 HallucinationFilter), 0 failures, 0 errors.
- Task 4: Manual DoD smoke — requires Android device. Owed before marking done (surface-class hard-gate). Status set to review pending smoke.
- ✅ Resolved review finding [High]: `cleanupLocal` model-load-failure 4th unsanitized path — `KlarvoApi.kt:552` `return text` → `return sanitizeLlmOutput(text)` in canonical + gen-mirror. All paths (3 in KlarvoOverlayService + model-load-fail in cleanupLocal) now sanitized before text reaches clipboard. BUILD SUCCESSFUL, 0 failures.

### File List

- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` (modified — `private` to `internal` for `sanitizeLlmOutput`; model-load-failure path fixed: `return text` → `return sanitizeLlmOutput(text)`)
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` (modified — 3 fallback paths: `transcript` to `KlarvoApi.sanitizeLlmOutput(transcript)`)
- `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoApi.kt` (modified — build-target mirror, same visibility change + model-load-failure fix)
- `src-tauri/gen/android/app/src/main/java/com/klarvo/voice/KlarvoOverlayService.kt` (modified — build-target mirror, same 3 fallback path changes)
- `android/kotlin-test/com/klarvo/voice/SanitizePathsTest.kt` (created — canonical 12-test JVM unit test, AI-2 binding)
- `src-tauri/gen/android/app/src/test/java/com/klarvo/voice/SanitizePathsTest.kt` (created — byte-identical build-target mirror)

## Change Log

- 2026-06-01: Story 2.3 implemented (DIV-03 fix). `sanitizeLlmOutput` visibility `private` to `internal`; all 3 raw-fallback paste paths in KlarvoOverlayService.kt now apply sanitization before `finalText`; 12 JVM tests added in SanitizePathsTest.kt (AI-2 binding); 54 total JVM tests pass, 0 failures. Task 4 on-device smoke owed (surface-class gate).
- 2026-06-01: Addressed code review finding [High]: `cleanupLocal` model-load-failure 4th unsanitized path fixed — `return text` → `return sanitizeLlmOutput(text)` at KlarvoApi.kt:552 + byte-identical gen-mirror sync. BUILD SUCCESSFUL, all tests pass.
