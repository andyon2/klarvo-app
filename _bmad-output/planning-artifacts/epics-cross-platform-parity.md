---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics"]
status: in-progress
inputDocuments:
  - docs/cross-platform-drift-audit.md  # verified A/B drift audit — the requirements source
  - docs/adr/0016-android-path-parity-strategy.md  # Amendment 1 (2026-06-10) gates this epic
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-10.md  # correct-course routing
  - _bmad-output/project-context.md
trackType: brownfield
featureEpic: 7
note: >
  Separate planning artifact by design (mirrors Epics 5/6). epics.md is the CLOSED
  robustness-remediation breakdown (Epics 1-4). This is the Cross-Platform Config-Contract
  Parity epic (Epic 7), routed via bmad-correct-course (sprint-change-proposal-2026-06-10.md)
  off the verified drift audit, gated by ADR-0016 Amendment 1. No PRD/Architecture/UX — the
  requirements source is docs/cross-platform-drift-audit.md (row IDs C/H/M/L + recall-sweep).
  Shares the sprint-status.yaml ledger. C1 (license) is already fixed (22553bc) and OUT of scope.
---

# klarvo - Epic Breakdown (Cross-Platform Config-Contract Parity · Epic 7)

## Overview

**Brownfield drift remediation.** ADR-0016 Amendment 1 (2026-06-10) moves the parity line
*chirurgisch*: only **core-output-determinism drift** and **settable-but-silently-dead config keys**
cross from "accepted asymmetry" into stories. Pure feature-ports stay accepted → `docs/backlog.md`.

Each story implements a cluster of audit rows (IDs from `docs/cross-platform-drift-audit.md`).
Unless noted, the fix is on the **Android** (Kotlin) side; Story 7.6 is the one **Desktop** (Rust) fix.

**Epic DoD:** every HIGH/CRITICAL row fixed here carries a golden-vector regression that runs against
**both** Rust and Kotlin (built incrementally per story, consolidated by Story 7.7). Test-Architect
`*risk`/`*design` on the two core-output stories (7.1, 7.2).

**Sequencing:** 7.1 first (highest-value core output). 7.2–7.6 are largely independent. 7.7 (the
structural parity net) runs **last** so it locks the just-fixed behavior + the dead-config landmines.

---

## Story 7.1: Android chunking parity (core output)

**Rows:** H2, H13, L4, M8.

As a klarvo user dictating long German text on Android,
I want chunk splitting to behave exactly as on Desktop,
So that the same dictation produces the same cleaned output on both platforms.

**Acceptance Criteria (outcomes — full Given/When/Then in create-story):**
- **H2** — Android chunk-split indices are computed over **UTF-8 byte length** (matching `raw_text.len()`,
  `llm/mod.rs:1244-1306`), not UTF-16 `text.length` (`KApi:816-884`). Umlaut-heavy text splits at the same point.
- **H13** — chunks are joined with `\n`, not `\n\n` (`KApi:909` → match `llm/mod.rs:1334`).
- **L4** — the threshold operator is `< 400`, not `<= 400` (off-by-one at exactly 400, `KApi:884`).
- **M8** — chunk-failure handling matches Desktop's abort-on-first-error semantics (or a deliberately
  documented, golden-vector-locked divergence), not "retry whole text as one call then raw" (`KApi:902-907`).

**Golden-vectors:** German-umlaut strings straddling the 400-char boundary; input that splits into N chunks.
**Test-Architect:** `*risk` + `*design` (core output path, hits the primary use case).

---

## Story 7.2: Android live auto-stop gate parity

**Rows:** H1, H17, Recall #1, M1 (read-part), M2, M3, M4, L1.

As a klarvo user on Android,
I want auto-stop and pre-STT silence handling driven by my configured thresholds, like on Desktop,
So that recording doesn't cut me off mid-sentence and Expert-mode tuning actually takes effect.

**Acceptance Criteria (outcomes):**
- **H1** — the live auto-stop energy gate derives `energy_floor` from config `silenceThreshold`
  (default 0.005), not a hardcoded `0.02f` (`KAR:58`). Defaults are no longer 4× stricter than Desktop.
- **Recall #1** — Android's live-autostop gate and its **own** pre-STT gate (`SilencePreFilter.kt:27`,
  `0.005f`) read the same source; the `0.02f`/`0.005f` self-desync is resolved.
- **H17** — silence→stop honors a 200ms floor (`hangover_ms.max(200)`, `audio/mod.rs:1074`); not a
  ~32ms single-frame floor (`KAR:77-78`).
- **M2** — `minRecordingMs` is config-driven (not hardcoded `500L`).
- **M3** — an 85 Hz highpass is applied before VAD (parity with `vad/mod.rs:73`).
- **M4** — RMS is computed at parity scale/precision so numeric thresholds are comparable (compounds H1).
- **L1** — VAD fps matches 31.25 (ceil), not integer 31.

> **Note:** M1's *whisper-mode threshold swap* depends on whisper-mode (C2, **backlog**) — only the
> `silenceThreshold`-read part is in scope here. **M5** (full 4-state VAD state machine) stays accepted →
> backlog (heavy algorithm rewrite, DIV-14); this story narrows the gap without porting the state machine.

**Golden-vectors:** energy-floor + stop-latency at default config and at one tuned config.
**Test-Architect:** `*risk` + `*design` (can truncate user speech).

---

## Story 7.3: Android STT conditioning + model selection

**Rows:** H3, Recall #5, H9, H10, L3. *(Overrides ADR-0016 DIV-08.)*

As a klarvo user on Android,
I want my dictionary, language hint, custom prompt and model choice to reach the STT call,
So that transcription quality and term-biasing match Desktop.

**Acceptance Criteria (outcomes):**
- **H3** — `buildMultipartBody` (`KApi:917-953`) sends a `prompt` field: language hint (`sttPromptDe/En/Auto`),
  dictionary terms, and `customPrompt` — mirroring `stt/mod.rs:226-256`.
- **Recall #5** — Android separates `customPrompt` (LLM cleanup) from `sttPrompt*` (STT conditioning),
  ending the single-field conflation.
- **H9** — the STT model is read from `cfg.sttModel`, not hardcoded `whisper-large-v3-turbo` (`KApi:924`).
- **H10** — the local Whisper model is read from `localWhisperModel`, not hardcoded `ggml-small.bin`
  (`KOS:1017` TODO).
- **L3** — STT temperature wire parity (Desktop sends `temperature=0.0`).

**Golden-vectors:** prompt-body assembly given dictionary + language hint + customPrompt.

---

## Story 7.4: Android output guards (prompt-echo + fragment strip)

**Rows:** H6, H7. *(Overrides ADR-0016 DIV-11 — near the guardian class.)*

As a klarvo user on Android,
I want Whisper's leaked prompt text filtered out before it's pasted,
So that prompt echoes and leaked hint sentences never reach my focused field.

**Acceptance Criteria (outcomes):**
- **H6** — port `is_prompt_echo()` (exact-fragment + 70% overlap heuristic, `pipeline.rs:234-314`).
- **H7** — port `strip_prompt_fragments()` (removes leaked hint sentences, `pipeline.rs:325-393`).

**Golden-vectors:** transcripts containing an echoed prompt / leaked fragment → expected stripped output.

---

## Story 7.5: Android LLM-routing contract hygiene

**Rows:** M9, M10, M11, M13, M16, L5.

As a klarvo user on Android,
I want provider routing, config-key handling and paste timing to follow the same contract as Desktop,
So that no setting silently no-ops and no future provider change silently breaks Android.

**Acceptance Criteria (outcomes):**
- **M9** — DeepSeek endpoint uses `…/v1/chat/completions` (parity with `llm/mod.rs:719`).
- **M10** — blank-key check uses whitespace-trimming semantics matching Desktop's `is_empty()` decision.
- **M11** — an unknown `cleanupStyle` is rejected (error), not silently mapped to Polished (`KApi:570/717`).
- **M13** — `bubbleTapAutoSend` / `bubbleLongPressAutoSend` are honored at runtime, not read-then-hardcoded
  `false` (`KOS:353-355`). *(Surface-operable trap: the setting round-trips but has zero effect today.)*
- **M16** — a pre-paste settle delay is applied (clipboard-ready) — parity with Desktop's 50ms.
- **L5** — `deviceId` default parity (auto-generated UUID v4 when the key is absent).

**Golden-vectors:** config round-trip asserting each key changes runtime behavior; unknown-`cleanupStyle` rejection.

---

## Story 7.6: Desktop back-port — whole-word hallucination match (H14) + M12 decision

**Rows:** H14 (**Desktop/Rust fix**), M12 (open decision).

As a klarvo user on Desktop,
I want single-word hallucination entries matched as whole words,
So that real words like "Standard" or "Milliarde" aren't false-discarded in short utterances.

**Acceptance Criteria (outcomes):**
- **H14** — `stt/hallucination.rs:160-164` matches single-word entries (incl. `ard/zdf/wdr`) as **whole words**,
  back-porting Android's ROB-03 fix (`HallucinationFilter.kt:100-109`). Substring matching no longer discards
  real words in ≤8-word utterances.
- **M12** — **resolve the open product decision first** (dictionary-in-Chat-style: should Chat include the
  dictionary like Android, or omit it like Desktop?), then make both platforms agree. Tracked as
  `OPEN-DECISION` in `docs/backlog.md`. If unresolved at story time, lock current behavior as a golden-vector
  and defer the code change.

**Golden-vectors:** "Standard"/"Milliarde" in a short utterance → not discarded; hallucination phrase → discarded.

---

## Story 7.7: Golden-Vector parity net (C1-proper) + dead-config lock

**Rows:** the structural net; locks every HIGH/CRITICAL Epic-7 fix + the dead-config-both-sides cluster.

As the klarvo maintainer,
I want shared golden-vector fixtures (config + input → expected behavior) run against BOTH Rust and Kotlin in CI,
So that any future re-divergence trips the net instead of shipping silently.

**Acceptance Criteria (outcomes):**
- A shared fixture format (config + input → expected behavior/output) consumed by both a Rust test target and
  a Kotlin test target.
- Every HIGH/CRITICAL row fixed in 7.1–7.6 has a corresponding golden-vector.
- The **dead-config-both-sides cluster** (`advanced.llmTemperature/llmMaxTokens`, `chunkThreshold/TargetSize`,
  `sttTemperature`, `llmModel*`/`llmSystemPrompt*` overrides, `autoCapitalize`/`autoPaste`) is locked: a vector
  asserts the current both-hardcode value, so the moment one side wires the key the net goes RED.
- CI runs the net on both platforms.
- **Inversion-check (mandatory):** deliberately re-introduce one fixed drift (e.g. revert H13 to `\n\n`) and
  prove the net goes RED — at writing time, not just review.

**Sequencing:** run after 7.1–7.6 so it locks the corrected behavior. This is the durable structural
drift-detection net (the [[cross-platform-parity-net]] deliverable, C1-proper) — it supersedes
"run the smartest model once" (the A/B audit's own finding: union + verifier + recall sweep still missed 5
real divergences).

---

## Out of scope (→ `docs/backlog.md`)

Pure feature-ports / accepted asymmetries (ADR-0016 Amendment 1): C2, H4, H5, H8, H11, H12+M7, H15, H16, M5,
M6, M14, M15, L2, L6, L7, Recall #4, and the dead-config cluster's *wire-when-needed* implementations.
C1 (license) is already fixed (`22553bc`) and out of scope.
