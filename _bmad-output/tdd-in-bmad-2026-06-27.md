# Running the BMAD TEA chain end-to-end — empirical findings

**Date:** 2026-06-27 · **BMAD installed:** 6.6.1-next.2 (npm `latest` = 6.9.0) · **Repo:** klarvo
**Purpose:** Research for adopting **TDD in BMAD as a generic capability** — run the real TEA
chain (`atdd → dev → trace`) on a live slice and document where stock BMAD is smooth vs. breaks.
**Companion:** verified-defect note (Step 5/6 contradiction + missing red-observer), GitHub issues
#1784 / #2275 / #843.

---

## 1. What was run

The actual installed skills `bmad-testarch-atdd` and `bmad-testarch-trace` were invoked (Create
mode), driven against a real Rust-core logic slice:

- **Slice:** `is_prompt_echo()` in `src-tauri/src/pipeline.rs` — Whisper prompt-echo detector.
- **Real bug found (German-relevant):** the word-significance filter used `w.len() >= 3`
  (**byte** length) while the doc says "significant words (≥3 chars)". The German 2-char filler
  `"äh"` is 3 UTF-8 bytes → wrongly counted as significant, skewing the echo-overlap ratio.
- **Fix:** `w.chars().count() >= 3` (one line), driven test-first.

## 2. The cycle actually happened — observed, not self-attested

| Phase | Action | Observed evidence |
|-------|--------|-------------------|
| RED | ported scaffold, ran `cargo test … red_word_significance` | **FAILED** at `'äh' … must not count` (byte logic); control test passed → discriminating, not blanket |
| GREEN | applied `chars().count()` fix, re-ran | both tests **ok** |
| REGRESSION | full `cargo test --lib pipeline::tests` | **93 passed, 0 failed** (was 91 + 2 new) |
| GATE | `trace` deterministic logic | **PASS** (P0 100% / P1 100% / overall 100%) — advisory (no CI) |

The red was observed by re-running the suite at the pre-fix state — i.e. an **independent
red-observer step performed by hand**, which is precisely what stock BMAD does not enforce.

## 3. Where stock BMAD was SMOOTH

- Cargo satisfied atdd's framework prerequisite — pure Rust-core logic is genuinely test-first-able.
- `cargo test` is fast (~0.03s for the module) and Linux-runnable — no device/emulator needed.
- The trace gate logic is deterministic and sound (P0 100% / overall 80% / P1 90-80 thresholds).
- trace IS skip-aware: it counts `skipped/fixme/pending` tests and downgrades confidence when
  zero active tests exist — so the tooling *knows about* the skip problem.

## 4. Where stock BMAD BROKE (the findings)

**A — Stack auto-detect HALTs a backend run on a frontend manifest.**
`test_stack_type: auto` sees root `package.json` (`react ^19.1.0`) **and** `Cargo.toml` →
classifies **fullstack** → the hard prereq becomes `playwright.config.ts`/`cypress.config.ts`,
which doesn't exist → **a stock run HALTs at preflight**, even though the work is pure Rust. Had to
override `detected_stack=backend` manually. (This is documented blocker #3 — *worse* than expected,
since the frontend manifest triggers it for backend work.)

**B — Rust isn't in the backend-prereq recognized list.**
The backend prereq examples are `conftest.py / src/test/ / *_test.go / .rspec`. Rust's inline
`#[cfg(test)]` (no separate config file) isn't listed, despite `cargo test` running green.

**C — The generator has no native-Rust path; it is hard-wired to Playwright/JS.**
`atdd` step-04 dispatches exactly two workers — "Red-Phase **API**" and "Red-Phase **E2E**" —
both emitting `test.skip()` (JS/Playwright). There is no backend-unit worker for Rust/Go/etc. The
Rust red scaffold had to be authored by the operating agent from the AC intent. (A prior run's
`atdd-redphase-7-3-scaffolds.rs` is likewise hand-written Rust — same deviation.)

**D — "RED phase" means `test.skip()`, not observed-failing — the root of issue #1784.**
The skill states *"Scaffolds stay skipped until a developer activates the current task."* The
generated state is **skipped** (neither red nor green). The red only materializes if someone
un-skips AND runs against unimplemented code — and **nothing enforces that**. This is exactly how
issue #1784 reported 29 tests left permanently `test.fixme()`, story marked "Ready for Review with
zero test evidence."

**E — Composition is manual (confirmed with prior-run data).**
atdd writes to `_bmad-output/test-artifacts/` (an output dir), never the build. Checked the prior
7-3 run: its scaffold test names (`h14_single_word_real_speech_not_blocked`, …) are **NOT present**
in `src-tauri/src` — instead a human re-authored equivalent tests under different names
(`test_h14_standard_not_blocked_single_word_ard`, …) and implemented the fix. So composition
worked there only because a person bridged it by hand — matching user complaint #843 ("I have to
tell the dev each time").

**F — No independent red-observer; the gate is advisory.**
The dev worker that writes the code is the same actor that would attest the red (Epic-4
anti-pattern). And `trace` runs *after* green, writing `gate-decision.json` "for any CI/CD pipeline"
— but with no CI consuming it, the gate **blocks nothing**. trace confirms coverage, never TDD
ordering.

**G — Source-level self-contradiction (verified in 6.9.0 / main).**
`bmad-dev-story` Step 5 ("Write FAILING tests first / confirm tests fail") vs. Step 6 ("Author
comprehensive tests") — a worker can satisfy Step 6 (tests-after) and truthfully report compliance
while skipping Step 5's red-first ordering. Present byte-identical in 6.6.1-next.2, 6.9.0, and
`main`; issues #1784/#2275 closed 2026-04-26 **without the fix shipping**.

## 5. Conclusion for a generic "TDD in BMAD" capability

1. **The valuable core is real**: red→green on a logic slice with a unit harness works and is
   honestly observable. It is NOT limited to one language in principle, but the *generator* today
   only emits JS — a generic capability needs native-test emitters (or must accept operator-authored
   scaffolds).
2. **The missing lever is enforcement, not instruction.** The red-first rule already exists (Step 5)
   and is ignored. The fix is an **independent red-observer**: a non-implementing actor (conductor /
   reviewer / CI) that mechanically asserts `baseRef..HEAD` contains an impl-free commit whose test
   run was RED before the GREEN commit.
3. **CI is the biggest design fork.** With CI the observer is free and the gate is a real block;
   without CI it must live in the conductor/reviewer. A generic capability must not *assume* CI —
   recommend: observer lives in the orchestration layer, uses CI as a verifier where present.
4. **Resolve the Step 5/6 contradiction** so "red-first" and "comprehensive tests" are one ordered
   instruction, not two contradictory ones.
5. **Honest scope:** TDD floors the *logic core*; it does not catch integration/runtime/device/
   cross-platform bugs (on klarvo's own bug history it would have cleanly caught 0 of 3). Sell it as
   a logic floor, not a bug net.

## 6. Side effect of this run (decision needed)
This run left a **real, correct fix** in `src-tauri/src/pipeline.rs` (the byte→char bug) plus an
extracted `is_significant_word` helper and 2 green tests. Options: (a) keep it as a genuine small
improvement, or (b) revert to leave the tree untouched. Awaiting your call.

## Artifacts produced
- `_bmad-output/test-artifacts/atdd-checklist-prompt-echo-char-significance.md`
- `_bmad-output/test-artifacts/atdd-redphase-prompt-echo-char-significance-scaffolds.rs`
- `_bmad-output/test-artifacts/traceability/traceability-matrix.md`
- code: `src-tauri/src/pipeline.rs` (helper + fix + 2 tests)
