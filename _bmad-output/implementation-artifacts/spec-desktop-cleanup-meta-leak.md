---
title: 'Chunked cleanup leaks LLM meta-refusal from orphaned trivial chunks'
type: 'bugfix'
created: '2026-06-12'
status: 'done'
context: []
baseline_commit: '8e0a55faea9c00178f6d8a0cbff300b3aa668163'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** When a dictation exceeds the 400-char chunking threshold and its tail has no `". "` sentence boundary, `split_into_chunks` falls back to a hard byte cut at `start + CHUNK_TARGET_SIZE`, which can orphan the final punctuation into a lone `"."` chunk. That `"."` is sent to the LLM cleanup, which replies conversationally ("*I apologize, I don't see any text to clean up. You've only provided a period…*"); `chunked_cleanup` concatenates that refusal into the output, so it lands in both the pasted text and the saved history entry (verified in history.db id=3041). The same byte-offset fallback is also not UTF-8 char-boundary-safe (latent panic on multibyte tails).

**Approach:** Fix it structurally at the chunk layer so no trivial fragment ever reaches the LLM: (1) make the fallback split char-boundary-safe and fold any trivial trailing fragment back into the preceding chunk; (2) defense-in-depth — `chunked_cleanup` passes chunks with no alphanumeric content through verbatim instead of calling the provider. No refusal-text pattern matching.

## Boundaries & Constraints

**Always:** A "trivial chunk" = a chunk with no alphanumeric character (`!chars().any(char::is_alphanumeric)`) — covers `"."`, `"..."`, `"!?"`, whitespace-only. Trivial fragments must stay attached to adjacent real text in the output (verbatim fidelity: `…Ordner.` stays intact, the period is not dropped or moved to its own line). Existing multi-sentence chunking behavior for normal text must not regress.

**Ask First:** If fixing this cleanly requires changing `CHUNK_THRESHOLD`/`CHUNK_TARGET_SIZE` or the chunk-combine join semantics (`\n`) — these affect every long dictation; HALT and confirm before touching them.

**Never:** Do NOT pattern-match the LLM's refusal prose ("I apologize", "I don't see") to strip it — brittle, language-dependent, false-positives on legitimate dictation about apologizing. Do NOT touch the history-list display truncation that hid the corruption from view (real but separate observability concern → note only). Out of scope: Android (separate Kotlin path), Epic 7, the live-preview flush path (already ruled out — raw STT, no clipboard, no LLM).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Orphan-period (the bug) | 568-char text, long comma-only tail, trailing `.` (history id=3041) | Chunks contain NO lone `"."`; trailing `.` stays attached to the last real chunk; combined output ends `…Root-Claude-Ordner.` with no refusal line | N/A |
| Multibyte fallback cut | >350-byte run with no boundary, multibyte char straddling byte `start+350` | Fallback split floored to a char boundary; no panic | floor index, never slice mid-char |
| Trivial chunk reaches combine | A chunk with no alphanumeric content (any path) | Not sent to provider; passed through verbatim; never a refusal in output | bypass LLM |
| Normal long text | Multi-sentence text with `". "` boundaries | Same chunk count/content as today (regression guard) | N/A |

</frozen-after-approval>

## Code Map

- `src-tauri/src/llm/mod.rs:1244-1290` -- `split_into_chunks`: the `best_split.unwrap_or(start + CHUNK_TARGET_SIZE)` fallback (line 1277) orphans the trailing `.` and is not char-boundary-safe.
- `src-tauri/src/llm/mod.rs:1297-1352` -- `chunked_cleanup`: fires every chunk at the provider (line 1320) and concatenates results incl. meta-refusals (line 1336).
- `src-tauri/src/llm/mod.rs:1233-1240` -- `CHUNK_THRESHOLD=400` / `CHUNK_TARGET_SIZE=350` (do not change without Ask-First).
- `src-tauri/src/llm/mod.rs:1354+` -- existing `#[cfg(test)]` module; mock `CleanupProvider` patterns live here.
- `src-tauri/src/pipeline.rs:1197-1198` -- `sanitize_llm_output` + `strip_stockphrase_ghosts` post-cleanup; context only (does not and should not catch refusal prose).

## Tasks & Acceptance

**Execution:**
- [x] `src-tauri/src/llm/mod.rs` -- `split_into_chunks` rewritten range-based: fallback `split_at` floored to a char boundary; trivial fragment (new `is_trivial_chunk` helper) folded into its predecessor by widening the previous range's end -- removes the orphan `"."` and the latent multibyte panic.
- [x] `src-tauri/src/llm/mod.rs` -- `chunked_cleanup`: trivial whole-input entry guard (covers the single-call path too) + per-chunk short-circuit to verbatim passthrough (defense-in-depth) -- a trivial chunk never reaches the LLM, so a refusal can never be concatenated.
- [x] `src-tauri/src/llm/mod.rs` (tests) -- id=3041 `raw_text` golden fixture; `test_split_id3041_does_not_orphan_trailing_period`, `test_split_fallback_is_char_boundary_safe`, `RefusingOnTrivialProvider` + `test_chunked_cleanup_id3041_no_meta_refusal`, `test_chunked_cleanup_whole_input_trivial_passthrough`. Inversion-check proven RED at writing time (flipping `is_trivial_chunk` → both id3041 tests fail, integration reproduces the exact refusal leak).

**Acceptance Criteria:**
- Given the history id=3041 raw_text, when it is chunked, then no emitted chunk is trivial and the trailing `.` is attached to the last real chunk.
- Given a mock provider that returns the refusal string for input `"."`, when `chunked_cleanup` runs on the id=3041 text, then the combined output contains the cleaned dictation and NOT the refusal line.
- Given any input, when `split_into_chunks` takes the byte-offset fallback, then `split_at` is always a UTF-8 char boundary (no panic on multibyte text).

## Spec Change Log

- **2026-06-12 — Review (3 reviewers, no loopback).** Blind-hunter / edge-case-hunter / acceptance-auditor: no Critical/High; acceptance-auditor 3/3 AC + all constraints PASS. Blind-hunter's `async move`-capture and token-`None`-poisoning concerns confirmed moot (compiles → captures are `Copy`; combine loop uses `if let Some`). **One patch** (converged blind#2 + edge CASE-1, LOW): a *leading* trivial fragment had no predecessor to fold backward into and was emitted standalone (caught by the per-chunk guard — no leak — but violated "stays attached" asymmetrically). Fixed by folding a leading trivial fragment FORWARD into the next chunk; added `test_split_leading_trivial_folds_forward`. 50/50 llm tests green.

- **2026-06-12 — Scope extended to Android (human renegotiation of the frozen "out of scope: Android" line).** The frozen intent excluded Android as a separate Kotlin path; the human flagged this perpetuates the exact cross-platform drift Klarvo forbids. The Kotlin twin (`android/kotlin-src/.../KlarvoApi.kt` `splitIntoChunks`/`cleanupChunked`) had the identical structural bug. Mirrored the fix to Kotlin (range-fold + surrogate-safe fallback + cleanup guards) and added a **shared** golden-vector fixture `test-fixtures/chunking-cleanup-vectors.json` consumed by BOTH a Rust test and a new Kotlin `ChunkingVectorsTest` (the Story 3.3 `test-fixtures/*.json` parity convention; seed doc `_bmad-output/test-artifacts/golden-vectors-chunking-cleanup-seeds.md`). Both platforms green; both inversion-proven RED. Verification: Rust `cargo test --lib llm::` (49 green); Kotlin `gradlew :app:testUniversalDebugUnitTest` (60 green incl. ChunkingVectorsTest).
- **2026-06-12 — Test-quality catch (vacuous green).** The first parity tests asserted no-trivial-chunk via the production `is_trivial_chunk`/`isTrivialChunk` itself, so the inversion check passed vacuously (flipping the guard blinded both the SUT and the detector). Fixed: both tests now use an **independent** local triviality predicate. KEEP: a parity/guard test must never judge the SUT with the SUT's own primitive — re-derive with an independent predicate.

## Design Notes

Root cause is deterministic and reproduced byte-for-byte: chunk2 = `"."` (1 byte) because the second half is one long comma-only clause with no `". "`, so the fallback cuts at byte 572 — exactly before the final period.

Fold-trivial sketch (boundaries, not prescription):
```rust
// fallback, char-safe:
let mut split_at = (start + CHUNK_TARGET_SIZE).min(text.len());
while split_at > start && !text.is_char_boundary(split_at) { split_at -= 1; }
// after building `chunks`: keep punctuation attached
if let Some(last) = chunks.last() {
    if !last.chars().any(|c| c.is_alphanumeric()) && chunks.len() > 1 { /* re-slice last into prev */ }
}
```

## Verification

**Commands:**
- `cd src-tauri && cargo test --lib llm::` -- expected: new chunk/cleanup tests pass, existing llm tests green.
- `cd src-tauri && cargo check` -- expected: clean.

**Manual checks (if no CLI):**
- Verifikations-Symmetrie: human live-repro is deliberately downgraded to fixture-verified — the trigger is deterministic and pinned by the golden fixture, so Andi is not handed a non-producible test.

## Suggested Review Order

**The guard primitive**

- The structural definition of "nothing to clean" — no alphanumeric content; the whole fix keys off this.
  [`mod.rs:1247`](../../src-tauri/src/llm/mod.rs#L1247)

**Chunking: never orphan a trivial fragment**

- Fallback split floored to a UTF-8 char boundary — kills the latent multibyte panic.
  [`mod.rs:1298`](../../src-tauri/src/llm/mod.rs#L1298)
- Backward fold: a trivial fragment widens its predecessor instead of standing alone (the "." stays attached).
  [`mod.rs:1319`](../../src-tauri/src/llm/mod.rs#L1319)
- Forward fold (review patch): a *leading* trivial fragment with no predecessor rides into the next chunk.
  [`mod.rs:1329`](../../src-tauri/src/llm/mod.rs#L1329)

**Cleanup orchestration: defense-in-depth**

- Whole-input guard — a fully-trivial capture passes through verbatim, never erroring the dictation.
  [`mod.rs:1355`](../../src-tauri/src/llm/mod.rs#L1355)
- Per-chunk short-circuit — a trivial chunk never reaches the LLM, so a refusal can't be concatenated.
  [`mod.rs:1382`](../../src-tauri/src/llm/mod.rs#L1382)

**Regression tests (golden fixture + inversion-proven)**

- The exact id=3041 transcript no longer orphans its trailing period.
  [`mod.rs:1951`](../../src-tauri/src/llm/mod.rs#L1951)
- A refusing mock proves the refusal never lands in cleaned output.
  [`mod.rs:2023`](../../src-tauri/src/llm/mod.rs#L2023)
- Leading-trivial forward-fold + char-boundary safety.
  [`mod.rs:1980`](../../src-tauri/src/llm/mod.rs#L1980)
