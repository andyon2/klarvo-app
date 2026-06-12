# Golden-Vector Seeds — Chunking / LLM-cleanup parity

**Status:** Live shared fixture. Consumed by BOTH platforms today; to be folded into
the Story 7.7 cross-platform parity-net harness alongside the 7.3 STT seeds.

**Fixture (single source of truth):** `test-fixtures/chunking-cleanup-vectors.json`

**Consumers:**
- Rust — `src-tauri/src/llm/mod.rs`, tests `spec_chunking_vectors_split_invariants`
  + `spec_chunking_vectors_no_meta_refusal` (trait-mock refusal integration).
- Kotlin — `android/kotlin-test/com/klarvo/voice/ChunkingVectorsTest.kt`, test
  `splitProducesNoTrivialChunkAndKeepsTrailingPunctuation` (split-level invariant;
  no injectable LLM seam on Android to mock the cleanup network call).

**Origin:** history id=3041 meta-refusal leak (2026-06-12). `split_into_chunks` /
`splitIntoChunks` orphaned a trailing `.` into its own chunk; the LLM cleanup then
replied conversationally ("I don't see any text to clean up. You've only provided a
period…") and that prose was concatenated into the user's output on **both** platforms
(Rust desktop + the hand-maintained Kotlin twin). Fix: fold trivial fragments into an
adjacent chunk + verbatim passthrough guard in the cleanup orchestrator.

---

## Vectors

| id | Input shape | Triggers pre-fix on | Invariant |
|----|-------------|---------------------|-----------|
| CHUNK-001-orphan-ascii-trailing-period | first sentence + 350-char comma-only clause + `.` (ASCII) | rust-byte AND kotlin-char (identical) | no trivial chunk; last chunk ends `.` |
| CHUNK-002-leading-trivial-folds-forward | 360 `!` + real text (ASCII) | rust-byte AND kotlin-char | no trivial chunk (leading run folds forward) |
| CHUNK-003-id3041-real-transcript | the real 568-char / 573-byte German transcript | **rust-byte only** (char/byte divergence) | no trivial chunk; last chunk ends `Root-Claude-Ordner.` |

**Documented divergence (CHUNK-003):** the same input orphans `.` under Rust **byte**
counting (573 > 350 in the tail) but is a near-miss under Kotlin **UTF-16 char**
counting (568 chars; tail ≤ 350). The post-fix invariant holds on both; the vector
pins the real-world case and documents the counting asymmetry. The deterministic
cross-platform RED-on-revert guard is **CHUNK-001** (pure ASCII → byte == char).

**Inversion-check (proven at writing time, both platforms):** flipping
`is_trivial_chunk` / `isTrivialChunk` to `false` makes CHUNK-001 fail with
`chunk 2 is a lone trivial fragment: "."` on Rust AND Kotlin. The tests use an
**independent** triviality predicate (not the production guard) so the SUT cannot
mask its own regression.

---

## Open follow-up (for 7.7 / Epic-7 re-eval)

The chunking + LLM-cleanup path is still **two hand-maintained twins** (Rust
`llm/mod.rs` + Kotlin `KlarvoApi.kt`), NOT consolidated into the shared Rust core
(ADR-0017 covered STT only). This fixture is the parity-net seam for that path; the
strategic decision (consolidate cleanup into Rust vs. keep twins pinned by the net)
is deferred to the Epic-7 re-eval.
