# Story 7.6: M12 — dictionary in Chat style (Desktop Chat arm includes the dictionary)

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a klarvo user,
I want dictionary handling in Chat style to behave the same on both platforms,
so that my term-biasing is predictable regardless of device.

## Context & Governing Decision

**Row:** M12 (drift audit, `docs/cross-platform-drift-audit.md`). **Opposite-direction drift:** Android does
more than Desktop.

- **Android** (`KlarvoApi.appendPromptExtensions`) appends the dictionary sentence for **every** cleanup style,
  Chat included — it takes no style parameter.
- **Desktop** (`CleanupStyle::system_prompt_with_translation` in `src-tauri/src/llm/mod.rs`) interpolates
  `{dict_section}` in the Polished and Verbatim arms but **omits it in the Chat arm**.

**Decision (Andi, 2026-09-10) — load-bearing, do not re-litigate:** Chat **includes** the dictionary. Android
is canon; **Desktop changes**. Rationale (`docs/backlog.md` "DECIDED 2026-09-10 — M12"): the omission was
deliberate at birth (`3a9f5d0`, "keeps it short"), but the brevity reason is gone — the Chat arm now carries
punctuation commands, language rules, `{custom_section}` and the sandwich defence, and the dictionary sentence
is one line. The dictionary's purpose (preserve terms exactly) is style-independent.

**Epic amendment 2026-09-10:** the fallback ("lock current behaviour as a golden vector") was delivered by
Story 7.8 (`test-fixtures/m12-dictionary-scope-vectors.json`). 7.6 is now purely: Desktop agrees with Android →
the 7.8 vector is flipped to the decided behaviour. Backlog sizing: "Quick-dev sized."

## Acceptance Criteria

### AC1 — Desktop Chat arm includes the dictionary

**Given** a non-empty dictionary term list (e.g. `"Klarvo, Tauri, powerhouse"`) and `CleanupStyle::Chat`,
**When** `CleanupStyle::system_prompt(Some(terms), custom)` or `system_prompt_with_translation(Some(terms), custom, lang)`
builds the system prompt,
**Then** the prompt contains `{dict_section}` — the existing sentence
`"\n\nThe user's custom dictionary terms (preserve these exactly): {terms}"` — exactly as the Polished and
Verbatim arms do.

**And** when `dictionary_terms` is `None` or `Some("")`, the Chat prompt is byte-identical to today's Chat
prompt (`dict_section` is the empty string).

### AC2 — The `M12-DICT-SCOPE-CHAT` vector is flipped

**Given** `test-fixtures/m12-dictionary-scope-vectors.json`,
**When** the Chat arm change from AC1 is in place,
**Then** the `M12-DICT-SCOPE-CHAT` entry reads `expected_dictionary_in_prompt: true` and
`platforms_agree: true` (`expected_dictionary_in_prompt_kotlin` stays `true`),
**And** `spec_m12_dictionary_scope_current_state_still_holds` passes **without editing the test** (it asserts
the desktop column against the real prompt and derives `platforms_agree` from the two columns — designed in
7.8 so that 7.6 flips one vector and the test follows).

### AC3 — A Chat dictionary test replaces the "Chat ignores dictionary" test

**Given** the existing test `test_cleanup_style_chat_ignores_dictionary` asserts the opposite of the decision
(Chat prompt with terms == Chat prompt without terms),
**When** AC1 lands, that test no longer describes intended behaviour,
**Then** it is replaced by a Chat dictionary test placed next to `test_system_prompt_chat_with_custom_prompt`,
asserting that the Chat prompt built with dictionary terms contains the dictionary sentence with those terms.

### AC4 — No regression on the other arms, the other callers, or Android

- Polished and Verbatim prompts are unchanged (byte-identical).
- The Chat prompt without a dictionary is unchanged (AC1 "And").
- No Kotlin file changes (Android is canon).
- All existing `cargo test --lib` tests in `src-tauri/` stay green.

## Tasks / Subtasks

- [ ] **Task 1 — Rust: add `{dict_section}` to the Chat arm** (AC1, AC4)
  - [ ] In `src-tauri/src/llm/mod.rs`, `CleanupStyle::system_prompt_with_translation`, Chat arm: change the
        trailing interpolation from `{custom_section}{translation_section}{sandwich}` to
        `{dict_section}{custom_section}{translation_section}{sandwich}` — the same order the Polished and
        Verbatim arms use, and the order the sandwich comment names ("dictionary, custom prompt, translation").
  - [ ] Change nothing else in the prompt text. Do not reword `dict_section`.
- [ ] **Task 2 — Replace the contradicting test** (AC3)
  - [ ] Remove `test_cleanup_style_chat_ignores_dictionary` (its comment "Chat style intentionally omits
        dictionary context to keep prompts short" is the rationale Andi's decision retired).
  - [ ] Add a Chat dictionary test next to `test_system_prompt_chat_with_custom_prompt`: Chat +
        `Some("Kubernetes")` → prompt contains `"The user's custom dictionary terms (preserve these exactly): Kubernetes"`.
- [ ] **Task 3 — Flip the M12 vector** (AC2)
  - [ ] In `test-fixtures/m12-dictionary-scope-vectors.json`, entry `M12-DICT-SCOPE-CHAT`: set
        `expected_dictionary_in_prompt` → `true`, `platforms_agree` → `true`.
  - [ ] Do **not** edit `spec_m12_dictionary_scope_current_state_still_holds`.
  - [ ] Also bring the fixture prose to the decided state (GATE-1 decision, Andi, 2026-09-16 — see Dev Notes):
        rewrite the `M12-DICT-SCOPE-CHAT` description so it no longer claims the platforms disagree or that
        the decision is open; rewrite the `M12-DICT-SCOPE-README` description the same way and drop its
        `open_decision: "M12"` field. Keep `record_type: "current-state-record"` on every entry — the label
        stays, so `spec_m12_dictionary_scope_current_state_still_holds` needs no edit (7.8 decision D1).
- [ ] **Task 4 — Gates + RED proof** (AC2, AC4)
  - [ ] Baseline before any edit: `cargo test --manifest-path src-tauri/Cargo.toml --lib spec_m12` green.
  - [ ] RED 1 — vector flipped, code NOT changed: `spec_m12_dictionary_scope_current_state_still_holds` fails on
        the Chat arm ("desktop chat arm: recorded dictionary-in-prompt state no longer matches the tree").
  - [ ] RED 2 — code changed, vector NOT flipped: the same spec fails on the Chat arm, and
        `test_cleanup_style_chat_ignores_dictionary` (if not yet replaced) fails.
  - [ ] Record both as a table `| # | Reverted change (symbol) | Went RED | Verbatim failure |` (7-8/7-10 format);
        `git status` shows only the intended edits afterwards.
  - [ ] Final: `cargo test --manifest-path src-tauri/Cargo.toml --lib` green; record the pass count with its
        coverage statement (see Testing requirements).
- [ ] **Task 5 — Record** — fill the Dev Agent Record (File List, Completion Notes) from `git diff`, anchored at
      symbols, not line numbers.

## Dev Notes

### Current state of the code this story touches (read before changing)

**`src-tauri/src/llm/mod.rs` → `CleanupStyle::system_prompt_with_translation`**
- Builds four sections up front: `dict_section` (non-empty terms only), `custom_section` (non-blank, trimmed),
  `translation_section` (non-blank language code → `language_name`), `sandwich` (fixed reminder string).
- `match self`: Polished and Verbatim arms end with `{dict_section}{custom_section}{translation_section}{sandwich}`;
  **Chat arm ends with `{custom_section}{translation_section}{sandwich}`** — the only arm without `{dict_section}`.
  `dict_section` is computed for Chat today and simply not interpolated.
- `system_prompt(dict, custom)` is a thin wrapper → `system_prompt_with_translation(dict, custom, None)`.
- **What must be preserved:** every other character of all three prompts; section order; sandwich defence last.

**Callers that inherit the change automatically (no edit needed):**
- Every `CleanupProvider` in `llm/mod.rs` builds its system text via `style.system_prompt*`:
  `OpenAiCompatibleCleanup` (wrapped by `DeepSeekCleanup`, `OpenAiCleanup`, `GroqCleanup`) and
  `AnthropicCleanup` (`system:` field).
- `src-tauri/src/llm/local.rs` (Windows local llama cleanup) calls `style.system_prompt*`.
- `src-tauri/src/pipeline.rs` passes `dictionary.terms_as_prompt()` as `dict_terms` (None when empty).
- `src-tauri/src/commands/recording.rs::cleanup_text` passes caller terms or `terms_as_list()`.
Neither the pipeline nor the command filters terms by style — so after AC1, a Chat dictation on Desktop sends the
dictionary to the cleanup model with no further change.

**Tests in `llm/mod.rs` `#[cfg(test)]` that touch this:**
- `test_cleanup_style_chat_ignores_dictionary` — **will fail after AC1 by design** (Task 2 replaces it).
- `test_system_prompt_chat_with_custom_prompt` — Chat + custom prompt; stays green; the new test goes next to it.
- `test_build_request_with_dictionary_terms` (Polished), `test_anthropic_cleanup_build_request_with_dictionary`
  (Polished), `test_system_prompt_dict_and_custom_prompt` (Verbatim) — unaffected.
- `spec_m12_dictionary_scope_current_state_still_holds` + `load_m12_vectors` — reads the fixture; for each styled
  entry asserts `system_prompt(Some("Klarvo, Tauri, powerhouse"), None).contains(dict) == expected_dictionary_in_prompt`,
  asserts `checked == 3`, asserts every entry has `record_type == "current-state-record"`, and asserts
  `platforms_agree == (desktop column == kotlin column)`. It deliberately has **no** assertion that some style still
  disagrees (7-8 review decision D1) — so the flip needs no test edit.

**`android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — canon, NOT modified**
- `appendPromptExtensions(base, dictionaryTerms, customInstructions)`: appends
  `"\n\nThe user's custom dictionary terms (preserve these exactly): $dictionaryTerms"` when non-blank, then custom
  instructions, then the sandwich reminder. Called for every style from the cloud cleanup path.
- The sentence is character-identical to Rust's `dict_section`. Order dict → custom → sandwich matches Rust
  (Android has no translation section).
- Kotlin's column in the M12 fixture stays a **written record**, not machine-asserted: `buildSystemPrompt` and
  `appendPromptExtensions` are `private` and reachable only from inside the network-calling cleanup function
  (7-8 AC4 reasoning). Do not open a seam in this story.

### GATE-1 decision — fixture record text after the flip (Andi, 2026-09-16: DECIDED)

**Decision: flip the fields AND correct the prose.** The fixture must not keep claiming M12 is open after
this story closes. Concretely: the two booleans on `M12-DICT-SCOPE-CHAT`, plus the descriptions of
`M12-DICT-SCOPE-CHAT` and `M12-DICT-SCOPE-README`, plus removal of `open_decision: "M12"`. **`record_type`
stays `"current-state-record"`** — renaming it was offered and rejected, because it would force an edit to
the test that 7.8 designed the flip to avoid. Rationale for the record below.



After Task 3 the fixture's prose still says M12 is open: the `M12-DICT-SCOPE-README` description ("THIS FILE
RECORDS A DIVERGENCE. IT DOES NOT DECIDE IT. M12 is an open product decision…"), its `open_decision: "M12"`
field, and the `M12-DICT-SCOPE-CHAT` description ("THE M12 DIVERGENCE … PLATFORMS DISAGREE"). The epic and the
backlog record prescribe only the two-field flip. Whether (and how) the prose / `open_decision` field /
`record_type` label change is **not decided** — note that changing `record_type` away from
`"current-state-record"` would require editing the test (it asserts that label on every entry), which 7.8
designed the flip to avoid. Nothing asserts `open_decision`. The row in `test-fixtures/README.md` is still
accurate after the flip (Rust reader only). Raised at GATE 1 and decided as recorded above.

### GATE-1 decision — human gate (Andi, 2026-09-16: DECIDED)

**Decision: machine proof PLUS Andi's Windows check.** The machine gates (`cargo test --lib` + the RED proof
in both directions) prove the desktop prompt carries the dictionary sentence. They prove nothing about a real
dictation. So 7.6 gets a GATE-4: the **conductor** triggers the Windows release build (`scripts/windows-build.sh`
— not a worker step, per the contract's `[desktop_build]`), and **Andi** runs one real check on his machine:
enter a dictionary term, select Chat style, dictate the term, confirm it arrives unchanged. Andi produces that
state himself, so verification symmetry holds. Rationale for the record below.



The epic gives 7.6 no DoD. Machine proof (`cargo test --lib` + RED proof) covers the prompt string. It does not
cover a real Windows build or a real Chat-style dictation with a dictionary term. Epic-7 retro agreement: "A human
gate is placed only where the human sees what the machine cannot (perception, real network path)." Whether 7.6
gets an Andi GATE-4 was Andi's call — raised at GATE 1 and decided as recorded above.

### Misleading prior wording (do not follow)

Epic-7 retro §6 says a 7-6 flip is "a fixture-only edit (… no change to `llm/mod.rs`)". That is wrong for the
code: 7.8 promised no edit to the Rust **test**; the Chat-arm prompt code in `llm/mod.rs` must still change (AC1).

### Scope guards

- **In scope:** the Chat arm interpolation, the contradicting test, the one vector.
- **Out of scope:** Kotlin changes; prompt wording; the Android local (MNN) cleanup path (`KlarvoApi.cleanupLocal`
  uses `buildSystemPrompt(style)` without `appendPromptExtensions`, so it carries no dictionary for any style —
  pre-existing, drift row H11/DIV-09, accepted → backlog); dictionary shared across devices (EPIC-CANDIDATE A /
  T4); dictionary enforcement second pass, spoken forms, learning loop (EPIC-CANDIDATE A, `docs/backlog.md`
  "DECIDED 2026-09-11"); style switches (EPIC-CANDIDATE B). Backlog: "Story 7-6 (M12 one-liner) stays the minimal
  coherent state and does not pre-empt A."
- No config key, no schema change, no UI change, no STT/VAD/JNI change.
- 7-8's deferred review items on this test (e.g. "assert that a style-less entry carries no `platforms_agree`
  field", "nothing pins that `open_decision` is present") stay deferred; not part of 7.6.

### Technical requirements / architecture compliance

- **Cleanup/LLM prompt assembly is a Rust↔Kotlin twin, not shared core** (ADR-0017 is STT-only). This story fixes
  the twin by moving Desktop to Android's behaviour — one side changes.
- ADR-0016 Amendment 1 names M12 as the open decision "in Story 7.6 zu lösen"; the decision is now recorded in
  `docs/backlog.md`.
- Sandwich defence stays last in the prompt (anti-prompt-injection hardening). The dictionary is user-controlled
  text; placing `{dict_section}` before the sandwich — as in the other two arms — keeps that defence intact.
- Code and comments English; match the surrounding `format!` string-continuation style (`\` line continuations,
  no reformatting of the prompt block).
- No new dependency, no library change.

### File structure requirements

| File | Change |
|---|---|
| `src-tauri/src/llm/mod.rs` | Chat arm interpolation (Task 1); test replace (Task 2) |
| `test-fixtures/m12-dictionary-scope-vectors.json` | `M12-DICT-SCOPE-CHAT` flip (Task 3) |
| `_bmad-output/implementation-artifacts/7-6-m12-open-product-decision-dictionary-in-chat-style.md` | Dev Agent Record |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | status transitions |

No other file is expected to change. If `git diff --stat` shows another source file, justify it or revert it.

### Testing requirements

- **Rust half only.** The M12 fixture has one reader (`llm/mod.rs` `spec_m12_*`); `test-fixtures/README.md`
  already records "Android column is a written record". No Kotlin test exists or is added, so the JVM gate
  (`scripts/android-smoke.sh` / `:app:testUniversalDebugUnitTest`) is not exercised by this change.
- Command: `cargo test --manifest-path src-tauri/Cargo.toml --lib` (targeted first: `… --lib spec_m12`,
  `… --lib chat`). Linux is sufficient for this assertion: the Chat arm is not platform-gated.
- **RED proof at writing time** (Task 4), both directions. A green run that never saw the failure does not prove
  the fixture reads the Chat arm. 7-8 already recorded direction 1 as its inversion row 7
  ("flipped M12 chat vector to expected_dictionary_in_prompt: true → 🔴 spec_m12_…").
- **JVM gate (optional, regression only).** If run: device-free sync of `android/kotlin-src/` + `android/kotlin-test/`
  into `src-tauri/gen/android`, then `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks`
  (`--rerun-tasks` is mandatory after a fixture edit — retro AI-3 has not declared fixtures as task inputs);
  count from `app/build/test-results/testUniversalDebugUnitTest/*.xml`. It exercises no M12 assertion — say so.
- **DoD gates for 7.6 — settled at GATE 1 (Andi, 2026-09-16).** The epic pins none; precedent 7-8 (same fixture,
  no desktop surface) needed no Windows build. Andi chose the stronger gate anyway: machine gates **plus** a
  Windows release build **plus** his own real Chat-style dictation with a dictionary term. See
  "GATE-1 decision — human gate".
- Every result states its coverage (project-context "A number states what it covers"): the Rust tests prove the
  desktop prompt string contains the dictionary sentence. They do **not** prove the Kotlin column (written record),
  model behaviour (no model is called), or anything on the Windows release build.

### Previous story intelligence

- **7-8 (created the M12 fixture):** the fixture and `spec_m12_dictionary_scope_current_state_still_holds` were
  built so that 7.6 flips ONE vector without editing the test — `platforms_agree` is derived, and the review
  removed a "some style still disagrees" assertion (D1) because it would fire on a correct resolution. Respect that
  design: change code, then the vector; never make the file agree by editing the file alone.
- **7-8 / Epic-7 retro D2:** story records anchor at symbols, never at line numbers; resolution rows are written from
  `git diff`, not memory. Three of four 7-8 review rounds were mostly record-text fixes.
- **7-3:** Kotlin twin work is verified by reading when no seam exists; say so explicitly instead of implying a test.
- **7-9 / 7-10:** every Epic-7 story since 7.8 carried a mandatory inversion check at writing time; the RED step in
  Task 4 is the equivalent here.
- **Project-context "Grep before declaring done":** re-verify every count in the record (tests passed, files
  touched) against today's tree.

### Git intelligence

- Branch `conductor/story-7-6` (cut from `v1-ship` at `d9f83ef`, plus `456db25` "epic-7 auf in-progress fuer den
  7-6-Cut"). Recent commits are 7-10 close-out and backlog/waypoint chores — no recent change to `llm/mod.rs` prompt
  assembly; last touches to `llm/mod.rs` are 7-9 review rounds (model-ID wiring, `9015a0a`).
- Commit style: `feat(7-6): …` / `test(7-6): …` / `chore(story): …`; small, scoped, never `git add .`.

### Project Structure Notes

- Tests stay inline in `llm/mod.rs` `#[cfg(test)]` (project rule: no second `tests/` file for unit checks).
- The fixture lives in repo-root `test-fixtures/`; the Rust reader resolves it via `CARGO_MANIFEST_DIR/..`.
- No conflicts with the unified structure detected.

### References

- [Source: _bmad-output/planning-artifacts/epics-cross-platform-parity.md#Story 7.6] — story, AC, amendment 2026-09-10, decision note.
- [Source: docs/backlog.md#DECIDED 2026-09-10 — M12 (dictionary-in-Chat-style): Chat INCLUDES the dictionary] — decision, rationale, implementation sketch.
- [Source: docs/backlog.md#DECIDED 2026-09-11 — Dictionary rework + style add-on] — 7-6 does not pre-empt EPIC-CANDIDATE A.
- [Source: docs/cross-platform-drift-audit.md] — row M12.
- [Source: docs/adr/0016-android-path-parity-strategy.md#Amendment 1] — M12 as open decision for 7.6.
- [Source: docs/adr/0017-shared-core-stt-path.md] — Hard Rule is STT-only; cleanup is a twin.
- [Source: test-fixtures/m12-dictionary-scope-vectors.json] — the vector to flip.
- [Source: test-fixtures/README.md] — fixture ledger (M12 row: Rust reader only).
- [Source: src-tauri/src/llm/mod.rs::CleanupStyle::system_prompt_with_translation] — Chat arm.
- [Source: src-tauri/src/llm/mod.rs::tests::spec_m12_dictionary_scope_current_state_still_holds] — the reader.
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoApi.kt::appendPromptExtensions] — canon behaviour.
- [Source: _bmad-output/implementation-artifacts/7-8-parity-net-close-out-and-twin-hygiene.md] — AC4 (M12 current-state vector).
- [Source: _bmad-output/project-context.md] — testing rules, record anchoring, coverage statements.

## Dev Agent Record

### Agent Model Used

{{agent_model_name_version}}

### Debug Log References

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created

### File List
