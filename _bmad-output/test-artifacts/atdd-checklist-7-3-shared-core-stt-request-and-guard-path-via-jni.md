---
stepsCompleted: ['step-01-preflight-and-context', 'step-02-generation-mode', 'step-03-test-strategy', 'step-04-generate-tests']
lastStep: 'step-04-generate-tests'
lastSaved: '2026-06-12'
storyId: '7.3'
storyKey: '7-3-shared-core-stt-request-and-guard-path-via-jni'
storyFile: '_bmad-output/implementation-artifacts/7-3-shared-core-stt-request-and-guard-path-via-jni.md'
atddChecklistPath: '_bmad-output/test-artifacts/atdd-checklist-7-3-shared-core-stt-request-and-guard-path-via-jni.md'
generatedTestFiles: ['_bmad-output/test-artifacts/atdd-redphase-7-3-scaffolds.rs']
detectedStack: 'backend'
generationMode: 'ai-generation'
inputDocuments:
  - _bmad-output/implementation-artifacts/7-3-shared-core-stt-request-and-guard-path-via-jni.md
  - _bmad-output/test-artifacts/test-design-epic-7-story-7-3.md
  - _bmad-output/project-context.md
  - _bmad/tea/config.yaml
  - src-tauri/src/stt/hallucination.rs
  - src-tauri/src/stt/mod.rs
  - src-tauri/src/stt/jni_bridge.rs
  - src-tauri/src/license/jni.rs
  - src-tauri/src/pipeline.rs
---

# ATDD Red-Phase Checklist — Story 7.3 (Shared-core STT request + guard path via JNI)

## Step 1 — Preflight & Context

**Stack detection:** `backend` (auto). Manifests found: `src-tauri/Cargo.toml` (Rust),
`package.json` (React frontend, NOT exercised by this story). No `playwright.config.*` /
`cypress.config.*` — and none is required: this story's test surface is **Rust unit (inline
`#[cfg(test)]`) + JNI integration (device) + golden-vector fixtures + wiremock for request shape**.

**Prerequisite ruling (recorded decision):** the ATDD generic hard-requirement "frontend/fullstack
needs `playwright.config.ts`" does **not apply** — the configured test framework for this story is
`cargo test` with inline `#[cfg(test)]` modules, which exists and is in active use
(`stt/hallucination.rs`, `pipeline.rs`, `stt/mod.rs` all carry test modules). Prerequisites **met**.

**Story:** approved, `ready-for-dev`, 10 ACs, clear Given/When/Then. Test-Architect gate DONE
(`*risk`/`*design` in `test-design-epic-7-story-7-3.md`); R-001 (async Groq over JNI) **resolved →
Weg A** (per-call throwaway current-thread Tokio runtime + `block_on`); residual = PROVE in first P0.

**Knowledge base:** backend tier — `test-levels-framework`, `test-priorities-matrix`, `test-quality`,
`data-factories` patterns applied. No Playwright Utils / Pact loaded (not applicable to Rust+JNI).

## Step 2 — Generation Mode

**Mode: AI generation** (backend → always AI generation; recording section skipped per step-02 §2).
Tests authored from source-code analysis + the test-design coverage plan, bound to the real Rust
code paths (project-context rule: bind tests to real paths, not parallel mocks).

## Step 3 — Test Strategy (AC → level → priority → red-phase)

E2E layer is **N/A** (native desktop + Android, no browser — test-design §"Test levels"). Levels used:
Rust unit (inline `#[cfg(test)]`), JNI integration (device), golden-vector fixtures (seed for 7.7),
wiremock (request shape, device-free), manual on-device + desktop smoke.

| AC | Risk | Level | Prio | Red-phase status |
| --- | --- | --- | --- | --- |
| AC3 H14 whole-word | R-002 | Rust unit | **P0** | **PROVEN-RED** (run on current impl, fails) |
| AC5 stockphrase family + trailing-ghost gate | R-005 | Rust unit + golden | P1 | **PROVEN-RED** (both fail today; FP-guard test included) |
| AC4 silence/duration boundary parity | R-002 | Rust unit + golden | P1 | NEEDS-SURFACE (bind to `pipeline.rs` fns) |
| AC1 prompt assembly (H3/Recall#5) | — | Rust unit + golden | P1 | NEEDS-SURFACE (`build_stt_prompt_with_hint`) |
| AC6 verbose_json + both-shape parse | R-004 | Rust unit (fixtures) | **P0** | NEEDS-SURFACE (segment parser not present) |
| AC10 JNI panic-safety | R-003 | Rust unit + integration | **P0** | NEEDS-SURFACE (`groq_jni.rs` not present) |
| AC1 Weg A async-over-JNI proof | R-001 | Integration (device) + wiremock | **P0** | DEVICE (residual PROVE gate; escalate on fail) |
| AC1 retry/4xx around JNI | R-007 | Kotlin/integration | P1 | NEEDS-SURFACE (Kotlin reroute) |
| AC6 confidence-drop thresholds | R-009 | Golden-vector | P1 | NEEDS-SURFACE; thresholds UNKNOWN — do not invent |
| AC9 no-Kotlin-twin boundary | — | CI grep gate | P2 | Seeded here, enforced by 7.7 |

**Red-phase requirement confirmed:** every scaffold asserts target behavior and fails before impl.
The PROVEN-RED set was run on the current code to satisfy the project's writing-time inversion-check
(test-design Quality Gate: "H14 RED-first proven … at writing time").

## Step 4 — Generated Red-Phase Scaffolds

**Output:** `_bmad-output/test-artifacts/atdd-redphase-7-3-scaffolds.rs` (staging artifact, **not**
compiled into the build — the repo stays green for dev-story handoff).

**Adaptation (recorded):** step-04's two-worker Playwright/`test.skip()` orchestration is frontend-shaped
and does **not** fit Rust. Run sequentially as the unit/integration worker; the E2E worker is N/A.
Rust RED idiom = inversion-check (`assert!` that fails on current impl), not `test.skip()`.

**Empirical RED proof (2026-06-12, on current `hallucination.rs`):**
- `red_h14_whole_word_real_speech_not_blocked` → **FAILED** ("Standard"/"Milliarde"/"Hardware" wrongly
  blocked by `contains("ard")`). ✅ RED
- `red_stockphrase_family_blocked` → **FAILED** ("Groß- und Kleinschreibung" passes today). ✅ RED
- `red_trailing_ghost_on_long_clip_blocked` → **FAILED** (>8-word gate lets trailing ghost through). ✅ RED
- `red_control_ghosts_still_blocked` ("ZDF"/"amara.org") → **PASSED** — control proves the tests
  discriminate, not blanket-flip. ✅
Temp block was reverted after capture; `git status` clean except these test-artifacts.

**NEEDS-SURFACE / DEVICE scaffolds** are documented in the `.rs` artifact with exact target paths,
activation conditions, and the R-001 escalation contract (fail → escalate, do not iterate blind).

## Handoff to dev-story (Task 3/4/5/1/2 mapping)

- **Task 3** ← H14 PROVEN-RED block → implement whole-word for single-word entries → GREEN.
- **Task 4** ← stockphrase family + trailing-ghost block (keep FP-guard GREEN) → GREEN.
- **Task 5** ← verbose_json + both-shape parse scaffolds (R-004 desktop-regression guard).
- **Task 1/2** ← JNI panic-safety scaffolds + the R-001 device PROOF (first P0; escalate on fail).
- Golden vectors authored here are **seeds**; Story 7.7 consolidates the cross-platform net.
