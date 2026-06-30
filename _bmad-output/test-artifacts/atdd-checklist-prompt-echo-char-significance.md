---
stepsCompleted: ['step-01-preflight-and-context', 'step-02-generation-mode', 'step-03-test-strategy', 'step-04-generate-tests', 'step-04c-aggregate']
lastStep: 'step-04c-aggregate'
lastSaved: '2026-06-27'
storyId: 'prompt-echo-char-significance'
storyKey: 'prompt-echo-char-significance'
storyFile: '(none — slice-level run, no formal BMM story)'
atddChecklistPath: '_bmad-output/test-artifacts/atdd-checklist-prompt-echo-char-significance.md'
generatedTestFiles: ['_bmad-output/test-artifacts/atdd-redphase-prompt-echo-char-significance-scaffolds.rs']
inputDocuments: ['_bmad-output/project-context.md', '_bmad/tea/config.yaml', 'src-tauri/src/pipeline.rs']
detectedStack: 'backend (OVERRIDDEN; auto-detect=fullstack would HALT — no playwright.config)'
---

# ATDD Checklist — is_prompt_echo word-significance (char vs byte)

## Acceptance Criterion
Word significance inside `is_prompt_echo` must be measured by **character count** (≥3 chars),
as the doc comment states, not byte length. The German 2-char filler `"äh"` (3 UTF-8 bytes)
must NOT be counted as a significant word, because it skews the word-overlap ratio used to
detect Whisper prompt echoes.

## Red-phase tests (assert TARGET behavior; fail on current impl)
- [ ] `red_word_significance_counts_chars_not_bytes` — `"äh"`, `"öl"` (2 chars / 3 bytes) → NOT significant
- [ ] `red_word_significance_control_ascii_unchanged` — `"abc"`, `"über"` significant; `"an"` not (anti-blanket-flip control)

## Handoff to dev (MANUAL — not auto-consumed)
1. Behavior-preserving extraction: pull the inline `w.len() >= 3` filter in `extract_words`
   into `fn is_significant_word(w: &str) -> bool` (keep byte logic for now).
2. Port both scaffold tests into `pipeline.rs` `#[cfg(test)]`. Run → MUST be RED
   (`is_significant_word("äh")` returns true under byte logic).
3. GREEN: change body to `w.chars().count() >= 3`. Run → all pass.
4. Regression: full `cargo test --lib pipeline::tests` stays green.

## Quality-gate notes (for trace)
- Independent red observation REQUIRED before green (do not self-attest).
- No CI in this repo ⇒ the red observation is performed by the operating agent, not a pipeline.
