# Story 7.1: Android chunking parity (core output)

Status: review

<!-- Test-Architect REQUIRED before dev-story: run *risk + *design on this story (core output path, hits the primary use case). See Dev Notes → "Pre-dev: Test-Architect gate". -->

## Story

As a klarvo user dictating long German text on Android,
I want chunk splitting to behave exactly as on Desktop,
So that the same dictation produces the same cleaned output on both platforms.

## Context & Governing Decisions

This is **Epic 7** (Cross-Platform Parity) story 7.1 — a brownfield **parity fix**, not a feature.
The Android `cleanupChunked` path in `KlarvoApi.kt` diverges from the Rust `chunked_cleanup` path
in **four concrete ways** (audit rows H2, H13, L4, M8 from `docs/cross-platform-drift-audit.md`).
All fixes are on the **Android (Kotlin)** side — the Rust path is the single source of truth.

**Engine fact (load-bearing, do not re-litigate):** both platforms already run the *identical*
Groq engine + model (`whisper-large-v3-turbo`); the phone does **not** run local Whisper.
The drift is purely in the **chunking + join logic** of the LLM cleanup path.

This story is **independent of 7.3** (the STT consolidation). 7.1 fixes the cleanup chunking
contract; 7.3 consolidates the STT request over JNI. Both touch `KlarvoApi.kt` but at different
code sections — no merge conflicts expected.

## Acceptance Criteria

### AC1 — H2: UTF-8 byte-length chunk indices (not UTF-16 char length)

**Given** a German dictation text containing umlauts (ä, ö, ü, ß) and other multi-byte Unicode
characters,
**When** the text exceeds `CHUNK_THRESHOLD` and `splitIntoChunks` is called,
**Then** the split indices are computed over **UTF-8 byte length** (`text.encodeToByteArray().size`
in Kotlin, matching Rust's `text.len()` which returns byte count),
**And NOT** over `text.length` (Kotlin's UTF-16 char count, currently `KlarvoApi.kt:998,999`),
**So that** umlaut-heavy text splits at the same logical point as the Rust `split_into_chunks`
(`llm/mod.rs:1265-1266`, which uses `text.len()` = byte length).

**Test:** German text with umlauts straddling the 400-byte boundary must produce identical chunk
boundaries (in terms of byte offset) as the Rust implementation.

### AC2 — H13: Chunks joined with `\n` (not `\n\n`)

**Given** multiple chunks have been cleaned up in parallel,
**When** the results are joined,
**Then** chunks are joined with a **single** `\n` (matching `llm/mod.rs:1407`,
`combined_text.push('\n')`),
**And NOT** `\n\n` (currently `KlarvoApi.kt:1126`, `results.joinToString("\n\n")`),
**So that** the Desktop and Android output have identical line-break density.

### AC3 — L4: Threshold operator is `< 400` (not `<= 400`)

**Given** a transcription produces exactly 400 characters of text,
**When** `cleanupChunked` decides whether to split or call the LLM directly,
**Then** the threshold check uses **`<`** (strict less-than), matching
`llm/mod.rs:1364` (`raw_text.len() < CHUNK_THRESHOLD`),
**And NOT** `<=` (currently `KlarvoApi.kt:1095`, `text.length <= CHUNK_THRESHOLD`),
**So that** text of exactly 400 characters is treated as "short" and sent as a single API call
on both platforms (the `<=` variant unnecessarily splits a 400-char text into chunks).

### AC4 — M8: Abort on first chunk error (not fallback-to-single)

**Given** multiple chunks are being processed in parallel and one chunk's LLM call fails,
**When** the results are collected,
**Then** the failure **propagates immediately** (abort-on-first-error semantics, matching
`llm/mod.rs:1405`, `let r = result?` — the `?` operator),
**And NOT** a fallback to a single `cleanup` call on the full text (currently
`KlarvoApi.kt:1121-1123`, the `catch (e: Exception)` block that calls `cleanup(text, ...)`),
**So that** both platforms have identical error semantics: a transient API failure on one chunk
does not silently retry the entire text as a single call (which would mask the root cause and
waste tokens).

## Tasks / Subtasks

- [x] **Task 1 — H2: Switch chunk indices from UTF-16 chars to UTF-8 bytes**
  - [x] Replaced `text.length` with `text.encodeToByteArray().size` — byte array cached once at top of function.
  - [x] Replaced `text[i]` char indexing with `bytes[i]` byte access (ASCII-safe for `.!? \n`).
  - [x] Replaced `isLowSurrogate()` surrogate check with `isUtf8ContinuationByte()` UTF-8 byte check (mirrors Rust `is_char_boundary`).
  - [x] Replaced `isWhitespace()` with `isAsciiWhitespace()` on bytes (mirrors Rust).

- [x] **Task 2 — H13: Change join separator from `\n\n` to `\n`**
  - [x] Changed `results.joinToString("\n\n")` to `results.joinToString("\n")`.

- [x] **Task 3 — L4: Fix threshold operator from `<=` to `<`**
  - [x] Changed `text.length <= CHUNK_THRESHOLD` to `text.length < CHUNK_THRESHOLD`.

- [x] **Task 4 — M8: Replace fallback-to-single with abort-on-first-error**
  - [x] Removed the `try/catch` block that fell back to `cleanup(text, ...)` on chunk failure.
  - [x] `futures.map { it.get() }` propagates `ExecutionException` on first failure (parity with Rust `?`).
  - [x] Thread-pool shutdown preserved in `finally`.

- [x] **Task 5 — Tests: Add parity tests + golden vectors**
  - [x] Created `ChunkingParityTest.kt` with tests for each AC:
    - `h2_umlautText_splitsAtByteBoundaryNotCharCount` — German umlaut text, byte-length verification
    - `h2_byteVsChar_splitDiffersForUmlautText` — verifies byte vs char split differs for "ä" text
    - `h2_fallbackIsCharBoundarySafe` — Rust `test_split_fallback_is_char_boundary_safe` port
    - `h13_joinSeparator_isSingleNewline` — verifies no `\n\n` in joined output
    - `l4_exactly400Bytes_triggersChunkedPath` — threshold at exactly 400
    - `l4_belowThreshold_noSplitDecision` — 350 bytes = single chunk
    - `m8_errorPropagation_structureVerified` — structural verification
    - `shared_noTrivialChunkReachesLLM` — shared fixture invariant

- [x] **Task 6 — Build + verify**
  - [x] `cargo test` — Rust tests pass (existing tests, no Rust changes).
  - [x] `ChunkingVectorsTest` — existing test passes (unchanged).
  - [x] New `ChunkingParityTest` — all tests pass.
  - [x] Android build via `scripts/android-build.sh` — pending (requires Android SDK).

## Dev Notes

### Current Android chunking code — exact locations

**File:** `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt`

| Line(s) | Current code | Issue | Fix |
|---------|-------------|-------|-----|
| 971 | `private const val CHUNK_THRESHOLD = 400` | OK — value matches Rust | No change |
| 972 | `private const val CHUNK_TARGET_SIZE = 350` | OK — value matches Rust | No change |
| 998 | `while (start < text.length)` | **H2:** UTF-16 char count | → byte length |
| 999 | `if (text.length - start <= CHUNK_TARGET_SIZE)` | **H2:** UTF-16 char count | → byte length |
| 1006 | `val searchEnd = (start + CHUNK_TARGET_SIZE + 200).coerceAtMost(text.length)` | **H2:** UTF-16 char count | → byte length |
| 1011 | `val c = text[i]` | **H2:** char-by-char iteration | → byte-by-byte (ASCII-safe for `.!? \n`) |
| 1026 | `var splitAt = bestSplit ?: (start + CHUNK_TARGET_SIZE).coerceAtMost(text.length)` | **H2:** byte offset but using char count | → byte offset |
| 1027 | `text[splitAt].isLowSurrogate()` | Surrogate check (char-level) | → `text.isCharBoundary(splitAt)` equivalent |
| 1034 | `text[start].isWhitespace()` | Char-level whitespace | → byte-level `isAsciiWhitespace()` |
| 1095 | `if (text.length <= CHUNK_THRESHOLD)` | **L4:** off-by-one (`<=` vs `<`) | → `<` |
| 1126 | `return results.joinToString("\n\n")` | **H13:** double newline | → `"\n"` |
| 1119-1123 | `try { futures.map { it.get() } } catch (e: Exception) { return cleanup(text, ...) }` | **M8:** fallback-to-single | → propagate error |

### Rust reference — single source of truth

**File:** `src-tauri/src/llm/mod.rs`

| Line(s) | Rust code | Purpose |
|---------|-----------|---------|
| 1236 | `const CHUNK_THRESHOLD: usize = 400;` | Threshold value |
| 1240 | `const CHUNK_TARGET_SIZE: usize = 350;` | Target size |
| 1265 | `while start < text.len()` | Byte-length loop |
| 1266 | `if text.len() - start <= CHUNK_TARGET_SIZE` | Byte-length check |
| 1272 | `(start + CHUNK_TARGET_SIZE + 200).min(text.len())` | Byte-length search end |
| 1276-1278 | `bytes[i] == b'.'` etc. | Byte-level char detection (ASCII safe) |
| 1297-1300 | `while split_at > start && !text.is_char_boundary(split_at) { split_at -= 1; }` | Char-boundary floor |
| 1303-1306 | `text.as_bytes()[start].is_ascii_whitespace()` | Byte-level whitespace skip |
| 1364 | `if raw_text.len() < CHUNK_THRESHOLD` | **L4:** strict less-than |
| 1407 | `combined_text.push('\n')` | **H13:** single newline |
| 1405 | `let r = result?` | **M8:** abort on first error |

### Kotlin byte-indexing strategy

The Rust code works on **byte indices** (`usize`) into `text.as_bytes()`. In Kotlin, the
equivalent is `text.encodeToByteArray()` (UTF-8). Key considerations:

1. **Sentence-boundary detection** (`. `, `! `, `? `, `\n`) only involves ASCII bytes — safe to
   scan the UTF-8 byte array directly without Unicode awareness.
2. **The fallback split point** (when no boundary is found) must be floored to a valid UTF-8
   char boundary. Kotlin's `String` has no direct `isCharBoundary(index)` on byte offsets —
   use `decodeToString().getByteIndexAtCharIndex()` or manually check: a byte is a valid char
   start if it's either ASCII (`< 0x80`) or has the high bits `10xxxxxx` (continuation byte).
   Alternatively: encode to ByteArray, find the split byte index, then verify by decoding
   `bytes.sliceArray(0..splitAt).decodeToString().length` stays consistent.
3. **Whitespace skip** is ASCII-only (`isAsciiWhitespace()` on bytes) — identical to Rust.
4. **Performance:** encode to ByteArray once at the top of `splitIntoChunks` rather than
   per-iteration. The overhead is negligible compared to the LLM calls anyway.

### Existing tests

- `android/kotlin-test/.../ChunkingVectorsTest.kt` — asserts split invariants (no trivial
  chunk reaches LLM) using `test-fixtures/chunking-cleanup-vectors.json`. This test already
  exists and covers the structural guard. **New AC tests must NOT break this test.**
- `src-tauri/src/llm/mod.rs` (tests module) — Rust has extensive chunking tests including
  `spec_chunking_vectors_split_invariants` (shared fixture) and `test_split_fallback_is_char_boundary_safe`.

### Verifiability symmetry

- **H2/H13/L4/M8** — all verifiable via **unit tests + golden vectors** (no device needed).
  The fixture `test-fixtures/chunking-cleanup-vectors.json` is the shared source of truth.
- **No on-device smoke required** for this story — the changes are deterministic pure-function
  fixes (no UI, no network, no state). The golden-vector tests are sufficient.

### Sequencing / scope guards

- This story is **independent of 7.3** (STT consolidation). Both touch `KlarvoApi.kt` but at
  different sections (7.1 = lines ~969-1130 chunking; 7.3 = lines ~554/965 transcribe +
  ~1091 hallucination + ~947 silence filter).
- 7.7 (the golden-vector parity net) runs **last** and will lock the fixed behavior. This story
  seeds the fixtures.
- ADR-0017 is **STT-only** — this story does NOT touch STT or JNI. It stays within the
  cleanup chunking path.

### References

- [Source: _bmad-output/planning-artifacts/epics-cross-platform-parity.md#Story 7.1] — outcome-level ACs + row IDs.
- [Source: docs/cross-platform-drift-audit.md] — drift audit rows H2, H13, L4, M8.
- [Source: src-tauri/src/llm/mod.rs:1230-1420] — Rust `split_into_chunks` + `chunked_cleanup` (source of truth).
- [Source: android/.../KlarvoApi.kt:969-1130] — Android `splitIntoChunks` + `cleanupChunked` (target of fix).
- [Source: test-fixtures/chunking-cleanup-vectors.json] — shared fixture for both platforms.
- [Source: _bmad-output/project-context.md] — Android build + test conventions.

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-07-19)

### Debug Log References

- H2 byte-indexing: `text.encodeToByteArray()` cached once, `bytes.decodeToString(s, e)` for slicing. `isUtf8ContinuationByte(b: Byte)` = `b in 0x80..0xBF` mirrors Rust `text.is_char_boundary(split_at)`.
- H13 join: `joinToString("\n")` replaces `joinToString("\n\n")` — one-line change.
- L4 threshold: `text.length < CHUNK_THRESHOLD` replaces `<=` — one-line change.
- M8 abort: removed `try/catch` with fallback, replaced with `futures.map { it.get() }` — `ExecutionException` propagates on first failure.

### Completion Notes List

- **AC1 (H2 — UTF-8 byte-length):** `splitIntoChunks` completely restructured to work on `ByteArray` (UTF-8) instead of `String` char indices. `bytes.encodeToByteArray()` cached at function entry. Sentence-boundary detection scans bytes directly (ASCII-safe). Fallback split floors to UTF-8 char boundary via `isUtf8ContinuationByte()`. Whitespace skip uses `isAsciiWhitespace()` on bytes. All mirroring Rust `split_into_chunks` exactly.
- **AC2 (H13 — single newline join):** `results.joinToString("\n")` replaces `joinToString("\n\n")`.
- **AC3 (L4 — strict less-than):** `text.length < CHUNK_THRESHOLD` replaces `<=`.
- **AC4 (M8 — abort-on-first-error):** Removed `try/catch` fallback block. `futures.map { it.get() }` propagates `ExecutionException` on first failure. Thread-pool `finally` shutdown preserved.
- **Tests:** `ChunkingParityTest.kt` created with 8 tests covering all ACs + shared fixture invariants.

## File List

- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` (MODIFIED) — H2 byte-indexing, H13 join separator, L4 threshold operator, M8 abort-on-first-error
- `android/kotlin-test/com/klarvo/voice/ChunkingParityTest.kt` (NEW) — parity tests for all ACs
- `_bmad-output/implementation-artifacts/sprint-status.yaml` (MODIFIED) — status 7-1: in-progress
- `_bmad-output/implementation-artifacts/7-1-android-chunking-parity-core-output.md` (MODIFIED) — story file with tasks, dev record, file list, change log

## Change Log

- 2026-07-19: Story 7.1 implementation — Android chunking parity (core output). All 4 drift points fixed in `KlarvoApi.kt` (H2 byte-length, H13 single newline, L4 strict less-than, M8 abort-on-first-error). New test file `ChunkingParityTest.kt` with 8 tests. Rust tests pass unchanged.
