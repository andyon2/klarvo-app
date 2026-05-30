---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories", "step-04-final-validation"]
inputDocuments:
  - docs/robustness-audit-2026-05-30.md
  - docs/adr/0015-state-file-write-convention.md
  - docs/adr/0016-android-path-parity-strategy.md
  - docs/remediation-session-kickoff.md
trackType: brownfield
---

# klarvo - Epic Breakdown (Robustness Remediation)

## Overview

This is a **brownfield remediation** epic breakdown. There is no PRD/Architecture for this
work — the input is the adversarially-verified robustness audit
(`docs/robustness-audit-2026-05-30.md`, 25 confirmed findings), gated by two accepted ADRs:

- **ADR-0015** — State-file write/recovery convention (atomic write + backup-on-corrupt +
  single-writer). Gates the Config/State-Persistence epic.
- **ADR-0016** — Android path-parity strategy (draw a line; harden only the guardian class).
  Gates the Android Security-Guardian epic.

The audit's §0 Triage & Routing is the authoritative scope spec — it is **not** re-derived here.
Only the **Heavy Track** findings become epics/stories. quick-dev-track findings
(ROB-03/06/07/08/10, TEST-06..10, low-sev polish, contested) are deliberately **not** stories —
they run via `bmad-quick-dev` and are auto-anchored in `sprint-status.yaml` via `sync-sprint-status`.
DIV-06..14 are deliberately **not** stories — ADR-0016 closes them as accepted asymmetry.

**ID convention (from audit §0):** `ROB-NN` = rank NN in §2 · `DIV-NN` = row NN in §3 ·
`TEST-NN` = row NN in §4 · `DEPTH-<module>` = §5.

## Requirements Inventory

Brownfield: the audit's confirmed findings *are* the requirements. They retain their native audit
IDs (not renumbered FRn) so traceability back to the audit and ADRs stays intact.

### Functional Requirements (Heavy-Track findings to remediate)

**Config / State-Persistence hardening — gated by ADR-0015**

- **ROB-01** (critical) — `config/mod.rs:1267` `save_config` writes via `std::fs::write`
  (truncate-then-write) with no temp+rename/fsync/backup. Crash/power-loss in the write window →
  empty `config.json` → all plaintext API keys + license lost, no recovery.
- **ROB-02** (critical) — `config/mod.rs:966-975` + `lib.rs:716-721`: corrupt `config.json` →
  `load_config` silently returns `AppConfig::default()`; the `first_install_at==0` guard then
  immediately triggers `save_config`, irreversibly overwriting the repairable corrupt file on first boot.
- **ROB-04** (high) — `commands/settings.rs` (`save_settings`/`save_bar_position` drop the
  `config` guard before the disk write; `save_advanced_settings` holds it): no disk write-mutex →
  concurrent saves clobber the whole file (last-writer-wins). A bar-drag save with a stale clone can
  erase a just-saved API key while the UI reports "saved".
- **ROB-05** (high) — `config/mod.rs:1079` migration saves swallow errors (warn-only), no
  pre-migration backup, non-atomic. Trigger is exactly the existing user's upgrade boot — worst
  possible moment for total loss of keys + license.

**Android security guardians — gated by ADR-0016 (ONLY these four; DIV-06..14 are accepted asymmetry)**

- **DIV-01 / DIV-05** (critical) — Hallucination filter is entirely absent on Android. Rust runs
  `is_hallucination` blocklist + word-gate before paste (`hallucination.rs:146`, `pipeline.rs:504`);
  Kotlin only checks `transcript.isBlank()` (`KlarvoOverlayService.kt:1039/1018`). Whisper phantom
  text (`"Untertitelung des ZDF"`, `"[Music]"`) is pasted into any app's focused field **and**
  persisted to History + Turso sync. (DIV-01 and DIV-05 are the two finders of the same
  cross-surface defect.)
- **DIV-02** (high) — Min-length / post-STT silence (RMS) pre-filter absent. Rust runs
  `silence_skip` (TooShort/Silent) before the STT call (`pipeline.rs:471`); Kotlin only checks
  `wavBytes.isEmpty()` (`KlarvoOverlayService.kt:921`) → every mini-tap hits the paid Groq API and
  produces exactly the hallucinations DIV-01 must catch.
- **DIV-03** (high) — Paste-text sanitization not on all paths. Rust applies `sanitize_llm_output`
  centrally, covering the raw-degrade fallback (`pipeline.rs:1184`); Kotlin applies
  `sanitizeLlmOutput` only in `cleanup()`/`cleanupLocal()` — raw-fallback paths paste unsanitized
  (`KlarvoOverlayService.kt:1087/1096/1065`). Bidi-override / zero-width chars from raw transcript
  reach the target field → text-spoofing risk.
- **DIV-04** (high) — Banking/sensitive-app blocklist guards only the bubble, not the paste path.
  `bankingAppActive` controls only bubble visibility (`KlarvoOverlayService.kt:461/466`); the paste
  path has no check (`:1137-1141`). A pipeline that started before the app switch keeps pasting into
  a banking app — the "non-disableable protection" protects only bubble visibility.

**God-file depth (Ousterhout) — DEPTH-config gated by ADR-0015 (carved out of the persistence epic)**

- **DEPTH-config** — `config/mod.rs` (2790 LOC, SHALLOW). Real complexity concentrated in
  `load_config` (~290 LOC mixing load + env-merge + 3 schema migrations + provider validation +
  auto-fallback mutation with load-bearing ordering). ADR-0015 §5 explicitly carves this structural
  decoupling OUT of the persistence hardening, into a separate depth-refactor story. Audit §5
  recommendation: isolate the core as a tested `migrate_and_normalize` step; replace provider
  Strings with `FromStr` enums.
- **DEPTH-pipeline** — `pipeline.rs` (3438 LOC, MODERATE). Leaky abstractions: `ProcessInput`
  17-field struct pushes snapshot complexity onto the caller with only doc-enforced
  `dict_prompt`↔`stt_hint_text` consistency; `ProcessOutcome` forces the caller to apply deferred
  side-effects; 5 pure decision-helpers are `pub` only for tests. Audit §5 recommendation: demote
  test-only helpers to `pub(crate)`, group `ProcessInput` into substructures (`SttPromptPair`
  enforces consistency in the type), pull post-`process_audio` side-effects into `deliver_outcome`.

**Test integrity — false-safety islands (no gate; TEST-03 lives in the Config epic, see coverage map)**

- **TEST-01** — VAD/silence auto-stop (`audio/mod.rs:1564-1705`). 6 tests drive the local helper
  `run_silence_state_machine` (old RMS counting heuristic), NOT the production Silero VAD path
  (`recording_thread:898-1003`). Auto-stop regression in the real VAD code is structurally not caught.
- **TEST-02** — Feedback PI/privacy gate (`commands/feedback.rs:461-490`).
  `test_payload_no_dictation_sample` builds the payload manually with `raw_text:None` and never calls
  `send_feedback`; the real `include_dictation` gate (`feedback.rs:277-278`) is never executed. An
  inverted gate (plaintext always sent) would leave the test green — privacy leak undetected.
- **TEST-04** — WAV-RMS computation (`pipeline.rs:3305-3373`). `compute_wav_rms` is covered ONLY by
  golden-master/insta snapshot; `silence_skip` consumes RMS as a given argument and never tests the
  computation. A quantization/computation bug would be cemented as an "expected snapshot".
- **TEST-05** — System-prompt leak detection (`judge.rs:278-298`). Only `test_..._needs_two_markers`
  pins the `>=2` heuristic (1→Inconclusive); no spec independently verifies real leak protection.
  A single-marker leak stays unflagged as Inconclusive — cemented as expected.

### NonFunctional Requirements (cross-cutting constraints — from the two ADRs)

- **NFR-A1 (ADR-0015 §1)** — Atomic write for all state files: one `save_atomic(path, bytes)`
  helper (temp file in same dir → fsync → atomic rename over target). Applies to `config.json`,
  `dictionary.json`, and any persistent state file. Reference impl already in the codebase:
  `commands/llm_model.rs:249-256` (`.part`→final `tokio::fs::rename`).
- **NFR-A2 (ADR-0015 §2)** — Backup-on-corrupt instead of silent overwrite: on parse error,
  save the corrupt file to `config.json.corrupt-<ts>` **before** any default is written; warn the
  user via the existing error/event path.
- **NFR-A3 (ADR-0015 §3)** — Single-writer serialization: the whole read-modify-write+persist cycle
  runs under one disk-write lock (no guard drop before the write).
- **NFR-A4 (ADR-0015 §4)** — Migration writes carry a pre-migration backup and propagate write
  errors instead of warn-only.
- **NFR-W (ADR-0015 Consequences / Memory: Release-Build-Blind-Spot)** — Windows `rename`/replace
  atomicity over an existing target must be verified on a real Windows release build; consider
  `ReplaceFileW` / `tempfile`-crate `persist` over bare `std::fs::rename`. Cannot be validated by
  Linux `cargo test`.
- **NFR-Smoke (Memory: Smoke-Test-DoD-Gate)** — Surface stories (anything touching `shells/windows`
  or `android/`) require a real Windows release build + manual press-to-paste in the DoD. Linux
  `cargo test` + lint is NOT sufficient. Hard gate. The Android guardian epic is entirely
  surface-class.
- **NFR-TA (audit §0 / both ADRs)** — Heavy-Track epics run with the Test Architect
  (`*risk` / `*design` / `*trace`) because they touch legacy/critical paths with real regression
  potential between the Rust and Kotlin paths.

### Additional Requirements (gate decisions & out-of-scope fences — architecture substitute)

- Both gate ADRs are **Accepted** → both gated Heavy epics (Config-Persistence, Android-Guardian)
  are unblocked. God-file-depth and Test-integrity hang on no gate.
- **OUT OF SCOPE — do not file:** DIV-06..14 (ADR-0016 accepted asymmetry: provider-fallback,
  output-language/inline-translation, dictionary-on-STT, local-cleanup-prompt completeness,
  command-mode, prompt-echo guard, double-start atomicity, provider allowlist-reject, VAD params).
- **OUT OF SCOPE of the Config epic** — `load_config` structural decoupling is its own
  DEPTH-config depth story (ADR-0015 §5), NOT part of the persistence-hardening epic. Rationale:
  do not gate a critical data-loss fix behind a refactor (Premature-Abstraction-Guard).
- **Routed to quick-dev (not stories here):** ROB-03/06/07/08/10 (pipeline panic/drop safety),
  TEST-06..10 (test-proxy repair), ROB-11/15/16/17/18 (low-sev polish), ROB-12/13/14 (contested —
  re-evaluate before fix).
- **Discarded (audit §6):** 6 adversarially-refuted findings — do not re-file.

### UX Design Requirements

None. This is robustness/data-integrity remediation, not a UI feature set. Two findings have a
user-visible surface (the floating-pill error message ROB-18 and the "Invalid Date" display ROB-17),
but both are routed to quick-dev as low-sev polish, not Heavy-Track stories.

### FR Coverage Map

Every Heavy-Track finding maps to exactly one epic. Cross-references noted where an ID is touched
by more than one cluster in the routing spec.

| Finding | Epic | Notes |
|---|---|---|
| ROB-01 | Epic 1 | `save_config` atomic write (ships `save_atomic` helper) |
| ROB-02 | Epic 1 | Backup-on-corrupt instead of silent overwrite |
| ROB-04 | Epic 1 | Single-writer serialization for settings saves |
| ROB-05 | Epic 1 | Migration: pre-migration backup + error propagation |
| TEST-03 | Epic 1 | Migration-ladder regression net (closes ROB-05; per ADR-0015 Next-Action #2 — NOT in Epic 3) |
| DIV-01/05 | Epic 2 | Port hallucination filter to Android (critical) |
| DIV-02 | Epic 2 | Min-length / silence pre-filter on Android |
| DIV-03 | Epic 2 | Paste-text sanitization on all Android paths |
| DIV-04 | Epic 2 | Banking-app blocklist guards the paste path, not just the bubble |
| TEST-01 | Epic 3 | Spec-test the real Silero auto-stop path |
| TEST-02 | Epic 3 | Execute the real feedback PI/privacy gate in test |
| TEST-04 | Epic 3 | Spec-test the WAV-RMS computation |
| TEST-05 | Epic 3 | Independent leak-detection spec (not just the `>=2` pin) |
| DEPTH-config | Epic 4 | Isolate `load_config` core into tested `migrate_and_normalize` (sequenced after Epic 1) |
| DEPTH-pipeline | Epic 4 | Tighten `ProcessInput`/`ProcessOutcome`, demote test-only `pub` surface |

**No Heavy-Track finding is unmapped.** quick-dev findings (ROB-03/06/07/08/10, TEST-06..10,
ROB-11/15/16/17/18, ROB-12/13/14) and DIV-06..14 are intentionally absent — see Additional
Requirements fences.

## Epic List

Ordered by severity + the one ADR-mandated sequencing constraint. Epics 1 and 2 are independent
surfaces (Rust desktop vs. Kotlin Android) and can run in parallel. Epic 4's DEPTH-config story is
sequenced **after** Epic 1 (ADR-0015 §5: harden first, refactor the same `load_config` later — do
not gate the critical data-loss fix behind a refactor).

### Epic 1: Config & State Persistence Hardening
**[Gated by ADR-0015 — Accepted]** A user's secrets, license, and irreplaceable custom data
(snippets/profiles/custom-prompt) survive a crash, power loss, file corruption, or concurrent save
— the silent data-loss window is closed. This epic ships the `save_atomic` write convention and the
backup-on-corrupt recovery path that all state files inherit.
**Findings covered:** ROB-01, ROB-02, ROB-04, ROB-05, TEST-03
**NFRs:** NFR-A1, NFR-A2, NFR-A3, NFR-A4, NFR-W (Windows rename atomicity in DoD), NFR-TA
**Standalone:** Yes — complete persistence-hardening of the desktop config subsystem.

### Epic 2: Android Security Guardians
**[Gated by ADR-0016 — Accepted]** The Android user is protected from the same data-integrity / PI
leaks the desktop already guards: no Whisper phantom text pasted into apps or synced to history, no
unsanitized raw-fallback paste, no leak into a banking app mid-pipeline, no paid-API mini-taps.
Only the guardian class is ported; the accepted feature asymmetry (DIV-06..14) stays.
**Findings covered:** DIV-01/05, DIV-02, DIV-03, DIV-04
**NFRs:** NFR-Smoke (entirely surface-class — real Android build + manual test in DoD), NFR-TA
**Standalone:** Yes — independent Kotlin surface; no dependency on Epic 1.

### Epic 3: Test Integrity — Close the False-Safety Islands
The four critical paths that today pass a green-but-meaningless test get real specification coverage,
so a regression in VAD auto-stop, the feedback privacy gate, the RMS computation, or system-prompt
leak detection actually fails a test instead of being cemented as "expected".
**Findings covered:** TEST-01, TEST-02, TEST-04, TEST-05
**NFRs:** NFR-TA (`*trace` to map each new spec back to the finding it closes)
**Standalone:** Yes — additive test coverage; no dependency on other epics. Best done before Epic 4.

### Epic 4: God-File Depth Refactor
The two leaky god-files become navigable. `config/mod.rs`: the tangled ~290-LOC `load_config` core
(load + env-merge + 3 migrations + provider-validation + auto-fallback mutation) is isolated into a
tested `migrate_and_normalize` step (DEPTH-config). `pipeline.rs`: the leaky `ProcessInput`/
`ProcessOutcome` contracts are tightened (`SttPromptPair` type-enforces consistency; side-effects
pulled into `deliver_outcome`) and the test-only `pub` surface is demoted to `pub(crate)`
(DEPTH-pipeline). Pure internal quality — no user-visible behavior change.
**Findings covered:** DEPTH-config, DEPTH-pipeline
**NFRs:** NFR-TA (`*risk` — high regression potential, no acute bug)
**Standalone:** Functionally yes. **Ordering:** DEPTH-config runs after Epic 1 (same file, ADR-0015
§5 sequencing). Best done after Epic 3 so the strengthened test net catches refactor regressions.

**Scope fence (decision 2026-05-30):** This is the only epic with no damage-bearing finding — pure
internal quality. It is kept in the breakdown (so the depth debt is captured as proper stories with
full audit/ADR context rather than left un-filed), but it is the lowest-priority epic, last in
sprint order, gated behind Epics 1+3. **DEPTH-config and DEPTH-pipeline are deliberately separate
stories** so DEPTH-pipeline (the marginal item — `pipeline.rs` is rated MODERATE, not SHALLOW) can be
independently deferred at sprint-execution time without dragging the ADR-anticipated DEPTH-config
`load_config` isolation with it. Do NOT implement any of this epic under remediation time-pressure
ahead of the hardening/guardian/test work.

---

## Epic 1: Config & State Persistence Hardening

**[Gated by ADR-0015 — Accepted]** Close the silent data-loss window in the desktop config/state
subsystem. Ships the `save_atomic` write convention + backup-on-corrupt recovery that all state
files inherit. All anchors verified against HEAD (v1-ship, 2026-05-30).

### Story 1.1: Atomic state-file writes via a shared `save_atomic` helper

As a klarvo user,
I want my config and dictionary written atomically,
So that a crash or power loss mid-write can never leave me with an empty/truncated `config.json`
and the loss of all my API keys and license.

**Acceptance Criteria:**

**Given** a new `save_atomic(path, bytes)` helper,
**When** it persists,
**Then** it writes to a temp file in the SAME directory as the target, fsyncs it (`sync_all`), and
atomically renames it over the target — mirroring the existing `.part`→`rename` pattern at
`commands/llm_model.rs:249-258` (sync variant, since `save_config`/`save_dictionary` are sync callers).

**Given** `save_config` (`config/mod.rs:1261-1271`, today bare `std::fs::write(&path, contents)`),
**When** it persists,
**Then** it routes through `save_atomic`.
**And** `save_dictionary` (`dictionary/mod.rs:146-160`, same non-atomic gap) also routes through `save_atomic`.

**Given** the process is killed between temp-write and rename,
**When** the app restarts,
**Then** the previous `config.json` is intact and the orphan temp file is never read as live config.

**And** the helper returns its write error (no swallowing); callers propagate it.

**Technical context:** ref impl `commands/llm_model.rs:249-258` (async). Same-dir temp is mandatory
(cross-device rename breaks atomicity — ADR-0015 §1). **DoD (NFR-W):** verify rename-over-existing-target
atomicity on a REAL Windows release build; consider `tempfile`-crate `persist`/`ReplaceFileW` if
`std::fs::rename` semantics differ on Windows.

### Story 1.2: Backup-on-corrupt recovery in `load_config`

As a klarvo user,
I want a corrupt `config.json` preserved instead of silently overwritten,
So that I can recover my keys/license/snippets instead of losing them on the next boot.

**Acceptance Criteria:**

**Given** `load_config` hits a JSON parse error (`config/mod.rs:973-974`),
**When** it falls back to defaults,
**Then** it FIRST copies the corrupt file to `config.json.corrupt-<unix_ts>` (via `save_atomic`)
before any default is written, and surfaces a warning through the existing error/event path (not just a log line).

**Given** the corrupt-backup now exists,
**When** `lib.rs:716-723`'s `first_install_at == 0` guard triggers `save_config` on first boot,
**Then** the user's original repairable data still exists under `.corrupt-<ts>` — the irreversible
"repairable → total loss" transition (ROB-02) is impossible.

**Given** a MISSING file (NotFound, `config/mod.rs:977-979`),
**When** load falls back to default,
**Then** NO corrupt-backup is written (missing ≠ corrupt) — only parse/read errors trigger the backup.

**And** a read error (`config/mod.rs:981-983`) is treated like corruption (best-effort backup, warn surfaced).

### Story 1.3: Single-writer serialization for state-file saves

As a klarvo user,
I want concurrent settings saves serialized,
So that a background bar-drag save can't clobber the whole config file and silently erase an API key
I just saved.

**Acceptance Criteria:**

**Given** there is today no global disk-write mutex (only the in-memory `config: Mutex<AppConfig>`),
**When** any path persists config to disk,
**Then** all disk writes go through ONE disk-write serialization so the read-modify-write+persist cycle
is atomic w.r.t. other savers (no last-writer-wins whole-file clobber).

**Given** the inconsistent lock discipline today — `save_advanced_settings` (`commands/settings.rs:609-627`)
holds the in-memory guard ACROSS the write; `save_settings` (`settings.rs:348-519`) and `save_bar_position`
(`commands/misc.rs:178-187`) drop it before the write,
**When** the fix lands,
**Then** all three converge on the single-writer convention, and no path holds the in-memory `config`
lock across disk I/O.

**Given** a `save_bar_position` with a stale clone fires just after a `save_settings` that persisted a new API key,
**When** both complete,
**Then** the just-saved API key survives.
**And** the UI's "saved" confirmation reflects a write that actually persisted.

**Technical context:** ROB-04. The real defect is the missing disk-write serializer, not per-call guard
timing alone. Fix is a dedicated write lock/queue — NOT "hold the in-memory guard longer" (that anti-pattern
blocks readers during I/O, as `save_advanced_settings` already shows).

### Story 1.4: Hardened config migration — pre-migration backup + error propagation

As a klarvo user upgrading to a new version,
I want my config migration protected,
So that a write failure mid-migration on first upgrade-boot can't lose my keys and license at the
worst possible moment.

**Acceptance Criteria:**

**Given** the three migration writebacks (`config/mod.rs:1079`, `1128`, `1157`) are today warn-only
(`if let Err(e) = save_config(...) { log::warn!(...) }`),
**When** a migration persists,
**Then** a write error is PROPAGATED, not warn-and-continue.

**Given** a migration is about to run,
**When** it starts,
**Then** a pre-migration backup of the existing on-disk config is written first (restorable pre-migration state).

**Given** Story 1.1 has landed,
**When** migration persists,
**Then** it inherits atomic write automatically (the warn-only saves now route through `save_atomic`).

**Given** the `hotkey_slots` migration triggers guaranteed once on first upgrade boot (empty-vec via
`#[serde(default)]`),
**When** that boot's migration write fails,
**Then** keys + license are NOT lost (pre-migration backup + propagated error).

**Technical context:** ROB-05. 3 explicit migrations (1079 sttPriority/llmPriority, 1128 hotkey→slots,
1157 insert_and_send→per-slot) + the implicit serde-default empty-vec trigger.

### Story 1.5: Migration-ladder regression test — history-DB `open_db()`

As a klarvo maintainer,
I want the real schema-migration ladder exercised by a test,
So that a regression in the v1-DB upgrade path (which today has only false safety) fails CI instead of
silently corrupting an existing user's history.

**Acceptance Criteria:**

**Given** the test helper `mem_db()` (`history/mod.rs:517-550`) builds the END schema directly and bypasses
the real `open_db()` migration ladder (`history/mod.rs:137-180`: ALTER TABLE ADD COLUMN + UUID backfill +
unique index),
**When** a new regression test runs,
**Then** it constructs an OLD pre-migration schema, calls the REAL `open_db()`, and asserts: all expected
columns now exist, existing rows are UUID-backfilled, and the unique index on `uuid` is present.

**Given** the config migration path is ALREADY covered by real `load_config` fixture tests
(`config/mod.rs:2263-2334`),
**When** this story is scoped,
**Then** it targets the UNTESTED history-DB `open_db()` ladder specifically (NOT config — config is
already real-path tested; the audit §4 row-3 conflated the two).

**Given** a deliberately-broken migration (e.g. a skipped ALTER),
**When** the test runs,
**Then** it FAILS (capable of catching a real regression, not tautological).

**Technical context:** TEST-03. Regression net for the migration-safety theme that ROB-05 / Story 1.4
hardens on the config side. **Epic DoD (NFR-TA):** Test Architect `*risk`/`*design` on crash-mid-write,
corrupt-recovery, and concurrent-save scenarios (the exact fail-modes untested today). Persistence stories'
Windows rename atomicity verified on a real Windows release build.

---

## Epic 2: Android Security Guardians

**[Gated by ADR-0016 — Accepted]** Port ONLY the guardian class (data-integrity/PI) to Android. The
accepted feature asymmetry DIV-06..14 stays. **Every story is surface-class → NFR-Smoke applies: real
Android build + manual on-device test in the DoD; Linux `cargo test` is insufficient.** NFR-TA `*risk`
on Rust↔Kotlin path regression.

### Story 2.1: Port the hallucination filter to Android

As an Android klarvo user,
I want Whisper phantom text filtered out,
So that `"Untertitelung des ZDF"` or `"[Music]"` is never pasted into my apps nor saved to my history/cloud.

**Acceptance Criteria:**

**Given** the desktop `is_hallucination` (`stt/hallucination.rs:146-164`) — blocklist (`49-115`, 60+ entries)
+ word-count gate (>8 words ⇒ pass, `154-158`),
**When** Android transcribes,
**Then** an equivalent Kotlin guard runs at `KlarvoOverlayService.kt:~1040`, AFTER the `transcript.isBlank()`
check (`1039`) and BEFORE the history insert (`1102-1111`) and Turso push (`1115-1122`).

**Given** a transcript matching the blocklist within the word-count gate,
**When** the guard fires,
**Then** Android goes idle (no paste, no success-toast) and writes NOTHING to history or Turso.

**Given** the desktop substring match has a KNOWN false-positive bug (ROB-03: `lower.contains("ard")` hits
"Standard"/"Milliarde"/"Hardware"),
**When** the Android port is written,
**Then** it uses word-boundary matching for short single-word entries so common German business words are
NOT discarded — port the CORRECTED logic, not the desktop bug.

**Given** a long dictation (>8 words) that incidentally contains a blocklist phrase,
**When** the guard evaluates,
**Then** it passes (word-count gate parity).

**Technical context:** DIV-01/05 (critical). Kotlin gap at `KlarvoOverlayService.kt:1018/1039` (only isBlank).

### Story 2.2: Min-length / silence pre-filter before the Groq STT call

As an Android klarvo user,
I want mini-taps and silence discarded before they hit the paid STT API,
So that I don't burn BYOK credits and don't generate the very phantom text Story 2.1 has to catch.

**Acceptance Criteria:**

**Given** the desktop `silence_skip` (`pipeline.rs:471-486`) with `min_recording_ms = 500` and
`silence_threshold = 0.005` RMS (`config/mod.rs:201-210`),
**When** Android finishes recording,
**Then** a pre-STT filter runs before the Groq call (today only `wavBytes.isEmpty()` at `KlarvoOverlayService.kt:921`).

**Given** a recording shorter than the min duration,
**When** the filter runs,
**Then** Android discards it (TooShort) with user-visible feedback and does NOT call Groq.

**Given** a recording whose RMS is below the silence threshold,
**When** the filter runs,
**Then** Android discards it (Silent) and does NOT call Groq.

**Given** a valid utterance above both thresholds,
**When** the filter runs,
**Then** it proceeds to STT unchanged (no regression to normal dictation).

**Technical context:** DIV-02. Android recorder already has a Silero VAD + RMS gate (`KlarvoAudioRecorder.kt:254-288`,
RMS 0.02) for auto-stop, but the separate pre-STT skip (duration + RMS) is missing. Reuse the recorded WAV's
measured RMS/duration.

### Story 2.3: Sanitize paste text on ALL Android paths

As an Android klarvo user,
I want raw-fallback paste paths sanitized,
So that bidi-override / zero-width characters from a raw transcript can't reach my target field and spoof text.

**Acceptance Criteria:**

**Given** Android's `sanitizeLlmOutput` (`KlarvoApi.kt:598-630`) already strips the same char-classes as the
Rust `sanitize_llm_output` (`pipeline.rs:2081-2128`: ANSI, null, bidi-overrides, zero-width),
**When** the three raw-fallback paste paths run (`KlarvoOverlayService.kt:1065` local-cleanup-failed, `1087`
cloud-cleanup-IOException, `1096` no-LLM-key),
**Then** each applies the EXISTING `sanitizeLlmOutput` before paste (today they paste `transcript` raw).

**Given** the cleanup paths (`1058` cleanupLocal, `1071` cleanupChunked) already sanitize,
**When** the fix lands,
**Then** sanitization is applied EXACTLY ONCE on every path (no double-sanitize).

**Given** a raw transcript containing a bidi-override,
**When** pasted via a fallback path,
**Then** the pasted text is sanitized (parity with the desktop's central coverage at `pipeline.rs:1184`).

**Technical context:** DIV-03. No new sanitizer needed — wrap the 3 raw-fallback returns with the existing
Kotlin `sanitizeLlmOutput`.

### Story 2.4: Banking-app blocklist guards the paste path, not just the bubble

As an Android klarvo user,
I want the banking-app protection to actually stop the paste,
So that a pipeline that started before I switched to my banking app doesn't paste my dictation into it.

**Acceptance Criteria:**

**Given** `bankingAppActive` today gates ONLY bubble visibility (`KlarvoOverlayService.kt:461/466`) and the
paste path (`1137-1144`: `copyToClipboard` + `pasteIntoFocusedField`) has NO check,
**When** a transcript is ready and `bankingAppActive` is true,
**Then** the paste path skips BOTH the clipboard write and the accessibility paste.

**Given** a recording that STARTED before an app-switch into a banking app,
**When** the pipeline completes while the banking app is focused,
**Then** nothing is pasted or copied into it.

**Given** the user is NOT in a banking app,
**When** a transcript is ready,
**Then** paste proceeds normally (no regression).

**And** when paste is blocked by the banking guard, the user gets feedback that nothing was pasted (not a
silent no-op that looks like a failure).

**Technical context:** DIV-04. Add the `bankingAppActive` check immediately before the paste at
`KlarvoOverlayService.kt:~1138`.

---

## Epic 3: Test Integrity — Close the False-Safety Islands

Convert green-but-meaningless tests into real specification coverage. NFR-TA `*trace` to map each new spec
back to the finding it closes.

### Story 3.1: Spec-test the real Silero auto-stop path

As a klarvo maintainer,
I want the production Silero auto-stop covered by a real test,
So that an auto-stop regression fails CI instead of being masked by a test of dead RMS code.

**Acceptance Criteria:**

**Given** the 6 tests at `audio/mod.rs:1576-1705` drive a test-only helper `run_silence_state_machine`
(OLD RMS counting heuristic that production no longer uses), and the REAL Silero auto-stop is inline in the
`recording_thread` closure (`audio/mod.rs:898-1003`, VAD edge-detect at `970-981`),
**When** this story lands,
**Then** the production silence/auto-stop logic is extracted into a standalone, device-independent function
(e.g. `run_vad_wait_loop(vad, chunk_rx, stop_rx, cfg) -> (fired, final_state)`) callable without a real cpal stream.

**Given** the extracted seam,
**When** new spec tests feed it synthetic speech→silence chunk sequences,
**Then** they assert auto-stop fires on the speech→silence edge with the configured hangover (driving the
REAL Silero state machine, not the RMS helper).

**Given** the old `run_silence_state_machine` tests pin dead logic,
**When** this story lands,
**Then** those tests are deleted or re-pointed at the real seam (no test left pinning the replaced RMS heuristic).

**And** the extraction is behavior-preserving: live recording auto-stop behaves identically.

**Technical context:** TEST-01. REQUIRES a code-seam extraction (the production VAD loop is inline in the
thread closure) before it is spec-testable — that refactor is part of the story. NFR-TA `*design` on the seam.

### Story 3.2: Execute the real feedback PI/privacy gate in test

As a klarvo user,
I want the privacy gate that withholds my dictation from feedback to be actually tested,
So that an inverted gate (plaintext always sent) is caught by a red test instead of leaking.

**Acceptance Criteria:**

**Given** `test_payload_no_dictation_sample_when_not_requested` (`commands/feedback.rs:464-493`) builds a
`FeedbackPayload` manually and never runs the real gate, and the real gate (`feedback.rs:277-278`,
`include_dictation` branch) lives inside `send_feedback` which hits the network directly (reqwest POST
`281-288`, no injection seam),
**When** this story lands,
**Then** the payload-construction + gate logic is extracted into a pure
`build_feedback_payload(include_dictation, metrics, ...) -> FeedbackPayload` testable without network.

**Given** the extracted pure function,
**When** a test calls it with `include_dictation = false`,
**Then** `raw_text` AND `cleaned_text` are `None`.

**Given** `include_dictation = true`,
**When** called,
**Then** `raw_text`/`cleaned_text` carry the metrics' last raw/cleaned text.

**Given** the gate were inverted (always include),
**When** the test runs,
**Then** it FAILS (the test actually guards the privacy invariant).

**Technical context:** TEST-02. Requires a seam extraction (pure payload builder) because `send_feedback`
couples gate + network. NFR-TA `*design`.

### Story 3.3: Spec-test the WAV-RMS computation independently

As a klarvo maintainer,
I want `compute_wav_rms` covered by known-input→known-output specs,
So that a quantization/normalization bug surfaces as a failing assertion instead of being cemented as an
"expected" snapshot.

**Acceptance Criteria:**

**Given** `compute_wav_rms` (`pipeline.rs:413-438`) is pure/public and today partly covered by an `insta`
golden-master snapshot (`pipeline.rs:3318-3339`),
**When** this story lands,
**Then** the computation is covered by independent parametric specs: silence → 0.0; full-scale 440 Hz sine →
≈ 1/√2 (±1e-3); a known speech-level amplitude → expected RMS; invalid/empty input → `None`.

**Given** the `insta` snapshot pins the implementation rather than the spec,
**When** this story lands,
**Then** the snapshot dependency is removed in favor of closed-form assertions (the sine test already carries
`(rms - expected).abs() < 1e-3`).

**And** the specs cover both i16 and float WAV sample paths (the function normalizes int by max_val).

**Technical context:** TEST-04. No seam needed (already testable) — lightest story in the epic. Interaction:
DEPTH-pipeline (Story 4.2) demotes `compute_wav_rms` to `pub(crate)`; these in-module tests keep working.

### Story 3.4: Independent system-prompt leak-detection spec

As a klarvo maintainer,
I want leak-detection verified beyond the `>=2`-marker pin,
So that a single-marker leak isn't quietly cemented as "expected Inconclusive" and substring collisions
don't cause false fails.

**Acceptance Criteria:**

**Given** `check_system_prompt_leaked` (`tests/pi_security/judge.rs:114-140`) flags `>=2` markers Fail, 1
Inconclusive, 0 Pass, and the only test (`279-298`) pins that threshold,
**When** this story lands,
**Then** additional specs verify: a substring-collision case ("cleanup assistance" must NOT count as the
marker "cleanup assistant"); an empty-markers list → Pass; and case-insensitivity asserted explicitly.

**Given** a single-marker leak is currently Inconclusive (not Fail),
**When** this story lands,
**Then** that behavior is either (a) documented as intentional with rationale, or (b) the detection is
strengthened — the decision is captured in the test, not left implicit.

**And** the specs verify real leak protection independent of the exact `>=2` threshold value (changing the
threshold cannot silently weaken protection without a failing test).

**Technical context:** TEST-05. File is `src-tauri/tests/pi_security/judge.rs` (integration test). Current
coverage is adequate but the threshold is under-specified. NFR-TA `*trace`.

---

## Epic 4: God-File Depth Refactor

Lowest priority, last in sprint order, NO behavior change. DEPTH-config runs after Epic 1; DEPTH-pipeline is
independently deferrable. Do not implement under remediation time-pressure ahead of Epics 1-3.

### Story 4.1: Isolate the `load_config` core into a tested `migrate_and_normalize`

As a klarvo maintainer,
I want `load_config`'s tangled core separated from I/O,
So that the migration/normalization logic is unit-testable in isolation and the SHALLOW god-function becomes
navigable.

**Acceptance Criteria:**

**Given** `load_config` (`config/mod.rs:966-1252`, ~290 LOC) interleaves six responsibilities — (a) file I/O
`969-985`, (b) env-merge `987-1042`, (c) migration#1 `1044-1082`, (d) migration#2 `1084-1131`,
(e) migration#3 `1133-1160`, (f) validation+auto-fallback `1162-1250`,
**When** this story lands,
**Then** a pure `migrate_and_normalize(parsed: AppConfig, env: &EnvSnapshot) -> AppConfig` is extracted
performing (b)-(f) with NO disk I/O, and `load_config` retains only (a) + the post-migration persistence decision.

**Given** the three in-load disk writebacks (`1079`, `1128`, `1157`),
**When** refactored,
**Then** the persistence side-effect moves out of the pure core to `load_config`'s I/O boundary, and STILL goes
through the atomic-write + pre-migration-backup behavior introduced in Epic 1 (behavior-preserving on top of
the hardening).

**Given** `migrate_and_normalize` is pure,
**When** new unit tests run,
**Then** each migration + the auto-fallback ordering is tested in isolation (no tempdir fixture needed).

**Given** provider identity is a bare `String` validated against `VALID_STT_PROVIDERS`/`VALID_LLM_PROVIDERS`
(`config/mod.rs:1165-1182`),
**When** this story lands (optional sub-scope),
**Then** providers MAY be modeled as `FromStr` enums mirroring the in-repo `HotkeyMode` precedent
(`config/mod.rs:329-357`) — or this is explicitly deferred with rationale.

**And** `load_config`'s observable behavior is unchanged (all existing config tests at `config/mod.rs:2263-2334`
still pass).

**Technical context:** DEPTH-config. MUST run after Epic 1 (same function hardened there; ADR-0015 §5).
Behavior-preserving. NFR-TA `*risk`.

### Story 4.2: Tighten `pipeline.rs` contracts + demote test-only `pub` surface

As a klarvo maintainer,
I want `ProcessInput`/`ProcessOutcome`'s leaky contracts tightened and the test-only public surface demoted,
So that the pipeline's real interface is honest and consistency invariants are type-enforced rather than
doc-enforced.

**Acceptance Criteria:**

**Given** `ProcessInput` (`pipeline.rs:905-927`, 17 fields) requires `dict_prompt` (`911`) and `stt_hint_text`
(`913`) to stay consistent by doc-comment only,
**When** this story lands,
**Then** a `SttPromptPair` substructure groups them so consistency is type-enforced (caller can't set one without
the other).

**Given** `ProcessOutcome` (`pipeline.rs:932-953`) forces the caller to hand-roll deferred side-effects inline
in the `stop_and_process_pipeline` match arms (`1500-1544`: error-metric increments, `consume_command_mode`,
usage recording, paste, history-event),
**When** this story lands,
**Then** those side-effects are pulled into a single `deliver_outcome(...)` function (created — none exists today)
so the consume semantics aren't re-implemented per caller.

**Given** the 5 pure decision-helpers are `pub` but only used in-module + by in-module tests —
`compute_wav_rms` (`413`), `is_offline` (`453`), `silence_skip` (`471`), `post_stt_skip` (`500`),
`select_llm_path` (`523`),
**When** this story lands,
**Then** they are demoted to `pub(crate)` (the in-module `#[cfg(test)]` tests keep working; nominal public
breadth halves).

**And** behavior is unchanged: the full hotkey→paste pipeline behaves identically, verified against the
strengthened Epic-3 test net.

**Technical context:** DEPTH-pipeline. Independently deferrable (the marginal item). Run after Epic 3 so the
test net catches refactor regressions. NFR-TA `*risk`.
