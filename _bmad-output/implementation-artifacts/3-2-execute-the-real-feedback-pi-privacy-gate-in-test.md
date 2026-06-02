# Story 3.2: Execute the Real Feedback PI/Privacy Gate in Test

Status: done

## Story

As a klarvo user,
I want the privacy gate that withholds my dictation from feedback to be actually tested,
so that an inverted gate (plaintext always sent) is caught by a red test instead of leaking.

## Acceptance Criteria

1. **Given** `test_payload_no_dictation_sample_when_not_requested` (`commands/feedback.rs:464-493`)
   builds a `FeedbackPayload` manually and never runs the real gate, and the real gate
   (`feedback.rs:277-278`, `include_dictation` branch) lives inside `send_feedback` which hits the
   network directly (reqwest POST `281-288`, no injection seam),
   **When** this story lands,
   **Then** the payload-construction + gate logic is extracted into a pure
   `build_feedback_payload(include_dictation, metrics, ...) -> FeedbackPayload` testable without network.

2. **Given** the extracted pure function,
   **When** a test calls it with `include_dictation = false`,
   **Then** `raw_text` AND `cleaned_text` are `None`.

3. **Given** `include_dictation = true`,
   **When** called,
   **Then** `raw_text`/`cleaned_text` carry the metrics' last raw/cleaned text.

4. **Given** the gate were inverted (always include),
   **When** the test runs,
   **Then** it FAILS (the test actually guards the privacy invariant).

## Tasks / Subtasks

- [x] Task 1: Extract `build_feedback_payload` pure function (AC: 1, 2, 3, 4)
  - [x] 1.1 In `src-tauri/src/commands/feedback.rs`, locate the payload construction block at
        lines `257-279` inside `send_feedback`
  - [x] 1.2 Extract a pure function with the signature:
    ```rust
    fn build_feedback_payload(
        include_dictation: bool,
        metrics: &FeedbackMetrics,
        category: String,
        message: String,
        email: Option<String>,
        context_area: String,
        area: Option<String>,
        version: String,
        os: String,
        license_status: String,
        platform: String,
    ) -> FeedbackPayload
    ```
    The function contains no network I/O, no lock acquisition, no async — pure data transformation
    from `FeedbackMetrics` + scalar args into `FeedbackPayload`.
  - [x] 1.3 In `send_feedback`, replace the inline payload construction with a call to
        `build_feedback_payload(include_dictation, &metrics, ...)` — behaviour is unchanged
  - [x] 1.4 Mark visibility as `pub(crate)` only if the test module requires it; `fn` (private) is
        sufficient since tests live in the same module via `#[cfg(test)]`
  - [x] 1.5 Confirm `cargo test --lib` still passes (540+ lib tests, 0 failures)

- [x] Task 2: Delete the dead test and replace it with real specs (AC: 1, 2, 3, 4)
  - [x] 2.1 Delete `test_payload_no_dictation_sample_when_not_requested` (lines 460-493) — this test
        builds the payload manually and never calls the real gate, making it tautological
  - [x] 2.2 Add `spec_privacy_gate_excludes_text_when_not_requested`:
        Call `build_feedback_payload(false, &metrics_with_text, ...)` where `metrics_with_text` has
        non-`None` `last_raw_text` and `last_cleaned_text`. Assert both `raw_text` and `cleaned_text`
        in the returned payload are `None`.
  - [x] 2.3 Add `spec_privacy_gate_includes_text_when_requested`:
        Call `build_feedback_payload(true, &metrics_with_text, ...)`. Assert `raw_text` ==
        `Some("hello world")` and `cleaned_text` == `Some("Hello, world.")` (the values from
        `metrics_with_text`).
  - [x] 2.4 Add `spec_privacy_gate_excludes_text_when_metrics_has_none`:
        Call `build_feedback_payload(true, &FeedbackMetrics::default(), ...)` (metrics where
        `last_raw_text` is `None`). Assert `raw_text` is `None` — tests that `include_dictation=true`
        with absent metrics produces `None`, not a panic.
  - [x] 2.5 Verify the gate-inversion property: if you swap the `include_dictation` condition (i.e.,
        artificially pass `true` in task 2.2's call), the `None` assertions fail — documenting in
        the task description/comment that the spec actively guards the inversion scenario.

- [x] Task 3: Verify test count and lint (AC: 1, 2, 3, 4)
  - [x] 3.1 Run `cargo test --lib -p klarvo -- feedback` and confirm new specs pass
  - [x] 3.2 Run full `cargo test --lib` and confirm total count is stable (540 before; 1 old test
        removed + 3 new tests added → net +2, expect ~542)
  - [x] 3.3 Run `cargo clippy` on `commands/feedback.rs` (or `cargo clippy -- -D warnings` on the
        crate) — no new warnings

- [x] Task 4 (Review fix — closes Review-Defer #1, AC: 2, 3, 4): Cover the real `send_feedback` network path
  - [x] 4.1 Extract a testable async seam out of `send_feedback` so the gate→wire path can be exercised
        without a live network. `send_feedback` already reads `feedback_webhook_url` from config (injectable)
        and POSTs `build_feedback_payload(...)` via `reqwest::Client::new()`. Keep the `#[tauri::command]`
        a thin wrapper; the production change must be minimal — extraction only, behaviour unchanged.
  - [x] 4.2 Add an integration test using the project's existing mock HTTP stack (wiremock — ADR-0005)
        that drives the seam end-to-end against a mock endpoint and captures the POSTed JSON body.
  - [x] 4.3 Assert: with `include_dictation = false`, the captured wire body has `rawText`/`cleanedText`
        absent or null; with `include_dictation = true`, they carry the metrics' raw/cleaned text. This
        guards the call-site binding (correct arg order + the real flag) that the pure-function specs cannot reach.
  - [x] 4.4 `cargo test --lib` green; clippy reports no new warnings on touched files.

- [x] Task 5 (Re-review fix — Task 4 was insufficient, AC: 2, 3, 4): actually exercise the call-site binding
  - [x] 5.1 Task 4 extracted `post_feedback_to_url` (the POST only) and the 2 wire tests call
        `build_feedback_payload(false/true, …)` **directly** before posting — so `send_feedback`'s own
        `build_feedback_payload(include_dictation, …)` binding is still untested. The test comment claiming
        a `send_feedback` hardcode/arg-swap "would FAIL" is false (the test hardcodes the flag itself).
  - [x] 5.2 Extract `send_feedback_inner(client: &reqwest::Client, webhook_url: &str, include_dictation: bool,
        metrics: &FeedbackMetrics, category, message, email, context_area, area, license_status, platform)
        -> Result<(), String>` that calls `build_feedback_payload(include_dictation, metrics, …)` then
        `post_feedback_to_url(…)`. The `#[tauri::command] send_feedback` reads State (webhook_url, metrics,
        license_status), builds the client, and delegates to it. Minimal extraction, behaviour unchanged.
  - [x] 5.3 Rewrite the 2 wire specs to call `send_feedback_inner(&client, &mock_url, false /* and true */,
        &metrics_with_text(), …)` — passing the flag to the SEAM, not pre-building the payload. Now a
        hardcode/arg-swap of `include_dictation` inside `send_feedback_inner` fails the test. Fix the comment.
  - [x] 5.4 `cargo test --lib` green; clippy no new warnings on touched files.

## Review Findings

Code review 2026-06-02 (3 adversarial layers — Blind Hunter / Edge Case Hunter / Acceptance Auditor, Opus). Edge Case Hunter and Acceptance Auditor both clean: field-by-field extraction verified faithful, all 4 ACs satisfied, AC-4 inversion property **empirically confirmed** (Auditor flipped the gate to `if !include_dictation` → both gate specs went RED, then restored). Initial triage: 2 deferred, 10 dismissed as noise/refuted. At GATE 3, Andi un-deferred finding #1 to close it now → became a patch, closed via fix-loop round 2 (the round-1 attempt was insufficient; re-review caught it). Final: 1 patch (closed), 1 deferred, 10 dismissed.

- [x] [Review][Patch] `send_feedback` call-site binding is not covered by an automated test [feedback.rs send_feedback call site] — the gate logic is guarded in the pure seam, but a future arg-reorder or hardcoded flag at the call site would not fail any unit test. **GATE-3 decision (Andi, 2026-06-02): un-defer and close now.** First attempt (Task 4) was INSUFFICIENT — it extracted `post_feedback_to_url` and added wire tests that called `build_feedback_payload` directly with a hardcoded flag, leaving the `send_feedback` binding untested (re-review caught this). **Closed by Task 5**: extracted `send_feedback_inner` (threads the flag → `build_feedback_payload` → POST); the 2 wire specs now pass `include_dictation` INTO the seam. **Empirically verified**: flipping the flag inside `send_feedback_inner` turns BOTH wire specs RED, restore → green.
- [x] [Review][Defer] 11-positional-arg `build_feedback_payload` has no compile-time slot safety [feedback.rs:158-169] — 6 of 11 args are `String`/`Option<String>`; a caller swap (e.g. `category`/`message`, `version`/`os`/`platform`) would compile. `#[allow(clippy::too_many_arguments)]` is sanctioned per DoD. Single verified caller today. A param-struct/builder refactor is out of TEST-02 (test-integrity, no behaviour change) scope. Deferred to deferred-work.md.

## Dev Notes

### What This Story Closes

**TEST-02** (`docs/robustness-audit-2026-05-30.md §4`): `test_payload_no_dictation_sample_when_not_requested`
builds the payload manually with `raw_text:None` and never calls `send_feedback`; the real
`include_dictation` gate (`feedback.rs:277-278`) is never executed. An inverted gate (plaintext always
sent) would leave the test green — privacy leak undetected. This story extracts a testable pure seam
and replaces the dead test with real gate-driving specs.

**NFR-TA**: Heavy-Track epic — `*design` on the seam. Story must close TEST-02 cleanly and be traceable
to the audit finding.

### The Bug: Why the Existing Test Is False Safety

The dead test (`test_payload_no_dictation_sample_when_not_requested`, lines 464-493) does this:

```rust
let payload = FeedbackPayload {
    // ...
    raw_text: None,      // Simulates include_dictation == false
    cleaned_text: None,  // <- manually set to None, never via the gate
};
let json = serde_json::to_string(&payload).unwrap();
assert!(json.contains("\"rawText\":null"));
```

This only tests JSON serialization of a manually-constructed `None`. The REAL gate at lines 277-278:

```rust
raw_text: if include_dictation { metrics.last_raw_text.clone() } else { None },
cleaned_text: if include_dictation { metrics.last_cleaned_text.clone() } else { None },
```

…is never called. If this conditional were inverted to `if !include_dictation { ... }` (a plausible
typo), the test stays green while raw dictation text leaks into every feedback submission.

### Current Production Code — The Seam to Extract

`src-tauri/src/commands/feedback.rs:257-279` inside `send_feedback`:

```rust
let payload = FeedbackPayload {
    category,
    message,
    email,
    context_area,
    area,
    version: env!("CARGO_PKG_VERSION").to_string(),
    os: std::env::consts::OS.to_string(),
    license_status,
    platform: platform.to_string(),
    // Metrics
    stt_latency_ms: metrics.last_stt_latency_ms,
    llm_latency_ms: metrics.last_llm_latency_ms,
    total_latency_ms: metrics.last_total_latency_ms,
    last_target_app: metrics.last_target_app.clone(),
    last_dictation_at: metrics.last_dictation_at.clone(),
    stt_error_count: metrics.stt_error_count,
    llm_error_count: metrics.llm_error_count,
    paste_error_count: metrics.paste_error_count,
    // Opt-in dictation sample — THE GATE
    raw_text: if include_dictation { metrics.last_raw_text.clone() } else { None },
    cleaned_text: if include_dictation { metrics.last_cleaned_text.clone() } else { None },
};
```

The extracted `build_feedback_payload` function is a pure lift of this block — all inputs become
parameters, the `env!("CARGO_PKG_VERSION")` and `std::env::consts::OS` calls stay INSIDE the
function (they are compile-time / static, not I/O), and the `include_dictation` gate moves with them.

`send_feedback` then becomes:

```rust
let payload = build_feedback_payload(
    include_dictation, &metrics, category, message, email,
    context_area, area,
    env!("CARGO_PKG_VERSION").to_string(),
    std::env::consts::OS.to_string(),
    license_status,
    platform.to_string(),
);
```

### Files to Modify

| File | Change |
|---|---|
| `src-tauri/src/commands/feedback.rs` | Extract `build_feedback_payload` from `send_feedback`; delete 1 dead test; add 3 real gate specs |

### Files NOT to touch

- `src-tauri/src/pipeline.rs` — not in scope
- `src-tauri/src/commands/settings.rs` — not in scope
- Any Android Kotlin files — not in scope
- `src-tauri/src/lib.rs` — not in scope

### Key Constraints

- **`FeedbackPayload` is private (`struct FeedbackPayload`)**: `build_feedback_payload` returns
  `FeedbackPayload`; since both live in the same module this is fine — no visibility change needed
  on the struct itself.
- **`FeedbackMetrics` is already `pub`**: usable in tests without any change.
- **No async, no Tauri `State`**: `build_feedback_payload` must be a plain synchronous `fn`. All
  async/lock work happens in `send_feedback` before the call. The function is a pure data transformer.
- **`env!("CARGO_PKG_VERSION")`** is a macro that expands at compile time — safe inside the pure
  function, not I/O.
- **`std::env::consts::OS`** is a compile-time constant — same, safe inside the function.
- **Do not change `FeedbackPayload` fields or `FeedbackMetrics` fields**: the extraction is
  behaviour-preserving. The existing serialization tests (`test_feedback_payload_serialization`,
  `test_feedback_payload_email_none`) must still pass unchanged.
- **No `#[cfg(desktop)]` or platform gate** required: feedback is cross-platform. The
  `#[cfg(desktop)]` / `#[cfg(mobile)]` guards are on the metrics-lock path and the metrics-file path
  inside `send_feedback`, not on the payload construction — they stay in `send_feedback`.

### Seam Design: Minimal Surface, Maximum Coverage

The `build_feedback_payload` extraction is the smallest possible seam that exercises the gate. Do NOT
widen the scope to extract more of `send_feedback` (webhook call, lock acquisition, metrics reset) —
that would require async test infrastructure and is explicitly out of scope (Premature-Abstraction-Guard).
The gate is the only untested logic, so the seam boundary is: everything BEFORE the `client.post(...)` call.

### Test Pattern from Epic-1 AI-2 / Epic-2 Retro Lesson

From `epic-2-retro-2026-06-02.md`:

> Bind tests to the REAL production call site, not to a parallel mock or indirect proxy.
> AI-2 caught a 4th unsanitized paste path in 2.3 precisely because the test was bound to the
> real `KlarvoApi.sanitizeLlmOutput`, not a manually-set `None`.

This story applies the same pattern: the new tests call `build_feedback_payload(false, ...)` — the
REAL function that `send_feedback` delegates to — with a `metrics` that CONTAINS text in
`last_raw_text`/`last_cleaned_text`. If the gate were inverted, `raw_text` would come back `Some(...)`,
and the `assert!(payload.raw_text.is_none())` would FAIL. That is the inversion-detection property
the audit requires.

### Helper: Constructing a `metrics_with_text` in Tests

The test needs a `FeedbackMetrics` that has real `last_raw_text`/`last_cleaned_text` to prove the gate
suppresses them. Use field-update syntax on `Default`:

```rust
let metrics_with_text = FeedbackMetrics {
    last_raw_text: Some("hello world".to_string()),
    last_cleaned_text: Some("Hello, world.".to_string()),
    last_stt_latency_ms: Some(400),
    ..FeedbackMetrics::default()
};
```

Then pass to `build_feedback_payload(false, &metrics_with_text, ...)`. Assert `payload.raw_text.is_none()`.
Assert `payload.cleaned_text.is_none()`. If someone inverts the gate, these assertions blow up.

### DoD Gate

Epic 3 is Test Integrity — no surface/UI changes. There is **NO Windows release build or Android smoke
requirement** for this story. `cargo test --lib` green on Linux + `clippy` clean on touched files is
sufficient. No manual press-to-paste needed.

### Epic 3 Cross-Story Context

- **Story 3.1** (done, `ec2fd78`): Extracted `process_vad_step` seam from `recording_thread`. Pattern:
  extract pure per-chunk function from inline async/thread closure; bind tests to the extracted seam.
  The same extract-and-test-the-seam pattern applies here.
- **Story 3.3** (done, `4109e3d`): WAV-RMS computation spec + shared cross-platform fixture. Pattern:
  in-module `#[cfg(test)]` tests in the same file as the production code.
- **Story 3.4** (backlog): System-prompt leak detection — `tests/pi_security/judge.rs`. Independent.
- Story 3.2 is independent of 3.4 and has no code dependency on 3.1 or 3.3.

### References

- `src-tauri/src/commands/feedback.rs:257-279` — Payload construction block to extract [Source: feedback.rs]
- `src-tauri/src/commands/feedback.rs:277-278` — The `include_dictation` gate (critical lines) [Source: feedback.rs]
- `src-tauri/src/commands/feedback.rs:281-288` — reqwest POST — stays inside `send_feedback`, NOT extracted [Source: feedback.rs]
- `src-tauri/src/commands/feedback.rs:464-493` — Dead test to delete [Source: feedback.rs]
- `docs/robustness-audit-2026-05-30.md §4` — TEST-02 finding
- `_bmad-output/planning-artifacts/epics.md` — Epic 3 Story 3.2 (p. 568-597)
- `_bmad-output/implementation-artifacts/epic-2-retro-2026-06-02.md` — AI-2: bind-tests-to-real-paths lesson

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

None — clean single-pass implementation.

### Completion Notes List

- Extracted `build_feedback_payload` as a private pure fn (no visibility change needed; test module is `#[cfg(test)]` in the same file via `use super::build_feedback_payload`).
- `send_feedback` now delegates to the extracted seam; behaviour is identical.
- Deleted dead test `test_payload_no_dictation_sample_when_not_requested` (was building `FeedbackPayload` manually with `raw_text: None`, never calling the gate).
- Added 3 gate specs: `spec_privacy_gate_excludes_text_when_not_requested` (gate OFF + inversion comment), `spec_privacy_gate_includes_text_when_requested` (gate ON), `spec_privacy_gate_excludes_text_when_metrics_has_none` (gate ON with absent metrics = no panic).
- Inversion-guard property: `spec_privacy_gate_excludes_text_when_not_requested` passes `false` to the real gate and asserts `is_none()` — inverting the condition to `!include_dictation` would produce `Some("hello world")` and the assert would FAIL.
- Test count: 540 before → 542 after (1 removed + 3 added). Full suite: 542 passed / 0 failed.
- Clippy on feedback.rs: only pre-existing `METRICS_FILENAME` dead_code warning (present before this story); no new warnings introduced.
- TEST-02 closed.
- Task 4 (Review-Defer #1, 2026-06-02): Extracted `post_feedback_to_url(client, url, payload)` async seam from `send_feedback`; `#[tauri::command]` is now a thin wrapper delegating to the seam. Added wiremock 0.6 to dev-dependencies. Added 2 wire-path integration specs (`spec_wire_gate_off_body_has_no_dictation_text`, `spec_wire_gate_on_body_carries_dictation_text`) — both drive `post_feedback_to_url` against a real `MockServer`, capture the POSTed JSON body, and assert rawText/cleanedText null-vs-present. Guards the call-site arg-binding that the pure-function specs cannot reach. 544 Rust lib tests / 0 fail; no new clippy warnings on feedback.rs.
- Task 5 (re-review fix, 2026-06-02): Task 4 was insufficient — the wire tests called `build_feedback_payload` directly with a hardcoded flag, so `send_feedback`'s binding was still untested. Fix: extracted `send_feedback_inner(client, webhook_url, include_dictation, metrics, …)` that internally calls `build_feedback_payload(include_dictation, …)` then `post_feedback_to_url`. The `#[tauri::command] send_feedback` now delegates to `send_feedback_inner`. Rewrote both wire specs to pass `include_dictation` into `send_feedback_inner` — not into `build_feedback_payload` directly. A hardcode or arg-swap of `include_dictation` inside `send_feedback_inner` (or its call to `build_feedback_payload`) now causes the wire assertions to fail. Defer #1 genuinely closed. 544 Rust lib tests / 0 fail; no new clippy warnings.

### File List

- `src-tauri/src/commands/feedback.rs` — extracted `build_feedback_payload`; replaced inline block in `send_feedback`; deleted 1 dead test; added 3 gate specs + 2 test helpers; extracted `post_feedback_to_url` seam; extracted `send_feedback_inner` seam; rewrote 2 wire specs to drive `send_feedback_inner`; `send_feedback` delegates to `send_feedback_inner`
- `src-tauri/Cargo.toml` — added wiremock 0.6 to dev-dependencies

### Change Log

- 2026-06-02: Story 3.2 implemented — extracted `build_feedback_payload` pure seam, deleted tautological test, added 3 privacy-gate specs. TEST-02 closed. 542 Rust lib tests / 0 fail.
- 2026-06-02: Review-Defer #1 closed — extracted `post_feedback_to_url` wire seam; added wiremock 0.6; added 2 wire-path integration specs guarding call-site binding. 544 Rust lib tests / 0 fail.
- 2026-06-02: Task 5 (re-review fix) — extracted `send_feedback_inner` seam; rewrote 2 wire specs to pass `include_dictation` flag into `send_feedback_inner` (not directly to `build_feedback_payload`). A hardcode or arg-swap inside `send_feedback_inner` now breaks the wire tests. `send_feedback` delegates to the seam. 544 Rust lib tests / 0 fail; no new clippy warnings.
