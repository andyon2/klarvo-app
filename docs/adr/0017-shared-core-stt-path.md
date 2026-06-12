# ADR-0017: Shared-Core STT Path — Single Rust STT Request + Guards over JNI

**Status:** Accepted
**Date:** 2026-06-12
**Relates to:** ADR-0016 (Android path parity strategy) — see its Amendment 2.

## Context

Both platforms use the identical Groq engine (`whisper-large-v3-turbo`); the phone does
NOT run local Whisper (verified 2026-06-12: phone config `sttProvider=groq`, code dispatch,
46/46 runtime STT runs `provider=groq`, 0 local — see
`docs/dictation-quality-android-vs-desktop-2026-06-12.md`). Yet two separate STT request
implementations hit the same endpoint — Rust `GroqWhisper` (`src-tauri/src/stt/mod.rs`) and
Kotlin `KlarvoApi.transcribe` — and demonstrably send different parameters (silence handling,
`response_format`, prompt conditioning, model selection). The guard logic is likewise
duplicated (`HallucinationFilter.kt` / `SilencePreFilter.kt` vs the Rust pipeline guards),
with real bugs on both sides:

- The Rust blocklist filter has a `word_count > 8` gate → never runs on long clips →
  trailing ghosts (`… Groß- und Klinge.`) pass through (~1.1 % of long desktop clips).
- The Rust filter substring-matches single-word entries (H14); the Kotlin twin already has
  the whole-word fix (ROB-03) — the twins have *diverged in opposite directions*.
- LLM cleanup rationalizes recognizable junk (`Klinge`) into the convincing full stockphrase
  (`Kleinschreibung`) — detectable noise becomes fluent, undetectable noise.

Per-row porting (ADR-0016 Amendment 1's approach for these rows) fixes symptoms, not the
divergence class: even a multi-model audit sweep missed 5 real divergences. The Android
license consolidation (`22553bc`, `src-tauri/src/license/jni.rs`) already proved the
shared-Rust-over-JNI pattern on this codebase.

## Decision

The STT request, the STT-output guards (hallucination filter, prompt-echo, fragment-strip),
and the pre-STT silence filter are **single-sourced in the Rust core** and consumed by
Android **via JNI** (`src-tauri/src/stt/jni_bridge.rs`). The Kotlin twins
(`KlarvoApi.transcribe`/`buildMultipartBody`, `HallucinationFilter.kt`,
`SilencePreFilter.kt`) are deleted.

**Hard rule:** shared STT/guard logic MUST live only in the Rust core. A parallel Kotlin
re-implementation of any STT-request or STT-guard behavior is **forbidden**; Android calls
the Rust path. New STT/guard behavior is added in Rust and exposed over JNI, never re-coded
in Kotlin.

## Scope

STT path only. The live auto-stop VAD gate (realtime frame stream — large JNI lift, speech-
truncation risk), text chunking, and LLM-provider routing remain platform-local per-row
parity (ADR-0016 Amendment 1) until a future decision extends this rule. The golden-vector
parity net (Epic 7.7) enforces the boundary.

## Consequences

- **+** Divergence in the STT path becomes impossible by construction; both platforms
  inherit every guard and parameter change for free.
- **+** The ~2000-LOC Android duplicate SHRINKS for the STT path (vs. ADR-0016's "grows
  minimally and deliberately").
- **−** Core STT code now crosses the JNI boundary → regression risk on both platforms;
  mitigated by golden-vectors (7.7) and Test-Architect `*risk`/`*design` on Story 7.3.
- **⚠ Consolidation regression trap (named):** the surviving Rust filter must adopt the
  Kotlin twin's whole-word fix (H14) *in the same story* that deletes the twin — otherwise
  Android regresses on an already-fixed behavior.

## Sources

- `docs/dictation-quality-android-vs-desktop-2026-06-12.md` (evidence run + correction)
- `_bmad-output/planning-artifacts/sprint-change-proposal-2026-06-12.md` (routing)
- `docs/cross-platform-drift-audit.md` (row IDs)
