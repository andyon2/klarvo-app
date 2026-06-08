---
title: 'B1 — Arm launch license gates (14-day trial + LS test-mode reject)'
type: 'chore'
created: '2026-06-08'
status: 'done'
context: []
baseline_commit: 'dbe3be6d45a734e08fad090dc8f7203ee82c81f4'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Three launch gates are intentionally disabled for Early Access and are hard launch-blockers ([[reference_license_keys_and_model]]). The auto-trial runs 60 days instead of 14, and LemonSqueezy `test_mode` keys are accepted in production builds (the test-mode reject is commented out at `ls_client.rs:171` and `:222`). Public launch must not ship with these open.

**Approach:** Reset `TRIAL_DURATION_SECS` to 14 days. Replace the two commented-out `#[cfg(not(debug_assertions))]` reject blocks with calls to one shared **pure helper** `rejects_test_mode(test_mode, is_release)`, so the launch-critical reject logic gets RED-able unit coverage instead of living only behind a cfg-gate that no `cargo test` exercises.

## Boundaries & Constraints

**Always:** Test-mode keys must still be accepted in debug builds (so devs/Andy can test without a live key). The reject helper is pure (no I/O, no cfg inside it) — production wires `is_release = !cfg!(debug_assertions)` at the call site. Both `activate()` and `validate()` route through the same helper. Both the production code AND the existing test mirrors (`parse_activate`/`parse_validate`) must call the real helper, not a parallel copy.

**Ask First:** Any change to the trial *key* path (byte `0x01` embedded-expiry keys) or to `GRACE_PERIOD_SECS` — out of scope, do not touch. Any change that would reject test-mode keys in debug builds.

**Never:** No network/wiremock integration test for the LS reject (consciously downgraded — see Design Notes). No new dependencies. No edit to `compute_trial_status` logic beyond the constant. No commit/push (human-owned).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Live key, release | `rejects_test_mode(false, true)` | `false` (accept) | N/A |
| Test-mode key, release | `rejects_test_mode(true, true)` | `true` (reject) → `LsApiError::Api` at call site | Err surfaced to caller |
| Test-mode key, debug | `rejects_test_mode(true, false)` | `false` (accept) | N/A |
| Live key, debug | `rejects_test_mode(false, false)` | `false` (accept) | N/A |
| Trial within window | `first_install_at = now − 5d` | `Trial { until }` | N/A |
| Trial past 14d | `first_install_at = now − 15d` | `Unlicensed` | N/A |

</frozen-after-approval>

## Code Map

- `src-tauri/src/license/mod.rs:99` -- `TRIAL_DURATION_SECS` constant (60d → 14d); `compute_trial_status` consumes it
- `src-tauri/src/license/ls_client.rs:170-177` -- commented test-mode reject in `activate()`
- `src-tauri/src/license/ls_client.rs:221-230` -- commented test-mode reject in `validate()`
- `src-tauri/src/license/ls_client.rs:273-429` -- `#[cfg(test)] mod tests` with `parse_activate`/`parse_validate` mirrors (must be rewired to the helper)
- `src-tauri/src/license/ls_client.rs:72` -- `LicenseKeyPayload.test_mode: bool`

## Tasks & Acceptance

**Execution:**
- [x] `src-tauri/src/license/mod.rs` -- set `TRIAL_DURATION_SECS = 14 * 24 * 60 * 60`, update the doc comment, remove the `TODO(launch)` line -- arm the trial gate
- [x] `src-tauri/src/license/ls_client.rs` -- add pure `fn rejects_test_mode(test_mode: bool, is_release: bool) -> bool`; call it in `activate()` (after `lk` is bound) and `validate()` (when `body.license_key` is `Some`), returning `LsApiError::Api("Test-mode license keys are not accepted in production builds")` when it returns true; pass `is_release = !cfg!(debug_assertions)` -- arm the LS gate with always-compiled, testable logic
- [x] `src-tauri/src/license/ls_client.rs` (tests) -- rewire `parse_activate`/`parse_validate` to call `rejects_test_mode(..., is_release)` so the mirrors exercise the real reject; add the 4 truth-table unit tests from the I/O matrix plus a release-mode reject test for both `parse_activate` and `parse_validate` -- bind tests to real logic + inversion-check
- [x] `src-tauri/src/license/mod.rs` (tests) -- add unit test asserting trial expires at 14d boundary (`first_install_at = now − 15d → Unlicensed`, `now − 5d → Trial`); update stale `lib.rs` `test_trial_expired` comment (61d/60d → 30d/14d) -- guards the constant against regression

**Acceptance Criteria:**
- Given a release build, when `rejects_test_mode` is called with `(true, true)`, then it returns `true` and the calling `activate`/`validate` path yields `LsApiError::Api`; flipping the boolean logic makes the unit test RED (inversion check).
- Given a debug build, when a test-mode key is activated/validated, then it is accepted (helper returns `false`) so local testing is unaffected.
- Given `TRIAL_DURATION_SECS`, when a unit test sets `first_install_at` to 15 days ago, then `compute_trial_status` returns `Unlicensed`; 5 days ago returns `Trial`.
- Given the change set, when `cargo test -p <license crate>` and `cargo clippy` run, then both pass with the new tests included.

## Design Notes

**Consciously downgraded human gate:** Andy currently has no LemonSqueezy live/test-mode keys, so the LS reject (gate 2) cannot be human-smoke-tested on the real device. Per the verification-symmetry rule, this is an explicit, recorded downgrade — gate 2 is **unit/machine-verified only** for this change. The pure-helper extraction is the compensating control: it gives the launch-critical reject a RED-able inversion check that the original cfg-gated comment never had. When LS keys become available, a release-build smoke (test-mode → rejected, live → accepted) should be run before public launch.

**Implementation finding — `validate()` gate is armed but dormant:** `ls_client::activate` is called in production (`mod.rs:429`), so its test-mode reject is **live** — this is the launch-critical entry point. `ls_client::validate` is **never called** in production today (confirmed via clippy `never used` + grep), so its reject is armed and correct-but-forward-looking with no runtime effect until periodic re-validation is wired. Pre-existing architectural gap, out of B1 scope; flagged so "both gates armed" is not mistaken for "both gates enforcing at runtime."

**Human-verifiable gate (gate 1, trial):** Andy sets `"firstInstallAt"` (camelCase, [[reference_config_json_camelcase_keys]]) in `%APPDATA%\com.klarvo.voice\config.json` to `now − 15 days`, restarts → app should report trial expired / Unlicensed; `now − 5 days` → still Trial.

Helper shape:
```rust
fn rejects_test_mode(test_mode: bool, is_release: bool) -> bool {
    test_mode && is_release
}
```

## Verification

**Commands:**
- `cargo test -p klarvo --lib license` (from `src-tauri/`) -- expected: all license tests pass incl. new truth-table + trial-boundary tests
- `cargo clippy --manifest-path src-tauri/Cargo.toml` -- expected: clean on touched files

**Manual checks (human, gate 1 only):**
- Edit `firstInstallAt` to ~15 days ago in real `config.json`, launch release build -- expected: trial expired / Unlicensed state in UI

## Suggested Review Order

**Gate 1 — trial duration**

- Entry point: the launch constant flipped 60d → 14d (the whole point of B1).
  [`mod.rs:98`](../../src-tauri/src/license/mod.rs#L98)

- Consumer that turns the constant into a status; logic untouched per spec `Never`.
  [`mod.rs:378`](../../src-tauri/src/license/mod.rs#L378)

**Gate 2 — LS test-mode reject**

- The pure, testable decision; `test_mode && is_release`, no cfg inside.
  [`ls_client.rs:128`](../../src-tauri/src/license/ls_client.rs#L128)

- Live gate: armed in `activate()` (the called production path), `is_release = !cfg!(debug_assertions)`.
  [`ls_client.rs:182`](../../src-tauri/src/license/ls_client.rs#L182)

- Armed-but-dormant gate in `validate()` (no production caller — see Design Notes).
  [`ls_client.rs:232`](../../src-tauri/src/license/ls_client.rs#L232)

**Tests (bind to real logic + inversion checks)**

- Test mirrors now thread `is_release` and call the REAL helper, not a copy.
  [`ls_client.rs:291`](../../src-tauri/src/license/ls_client.rs#L291)

- Truth-table guard: flipping `&&`→`||` turns this RED.
  [`ls_client.rs:457`](../../src-tauri/src/license/ls_client.rs#L457)

- Tightened 14-day boundary (±1h probes) — catches a 13-day off-by-one, not just 60d.
  [`mod.rs:969`](../../src-tauri/src/license/mod.rs#L969)

- Stale 60d-comment fixed in the AppState-level trial test.
  [`lib.rs:1562`](../../src-tauri/src/lib.rs#L1562)
