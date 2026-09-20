---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories", "step-04-final-validation"]
trackType: brownfield
mergedFrom:
  - epics.md  # 1-4
  - epics-live-preview.md  # 5
  - epics-bar-redesign.md  # 6
  - epics-cross-platform-parity.md  # 7
  - epics-visual-overhaul.md  # 8 + 9
  - epics-native-overlays.md  # 10
  - epics-cloud-resilience.md  # 12
  - epics-parity-line-audit-2.md  # 13
mergedOn: 2026-09-20
note: >
  One file for all increments (Andi, 2026-09-20). The former files are in git history.
  Each part keeps the frontmatter of its former file in a fenced block.
---

# klarvo - Breakdown of all increments

## How to read this file

- This file is the only planning file of its kind in klarvo. Each increment is one part.
- `_bmad-output/implementation-artifacts/sprint-status.yaml` carries the status. This file carries no status.
- **Rule for a story heading:** `### Story N.M: <title>`. The generator of `bmad-sprint-planning` derives the key as
  `N-M-<slug of the title>`. The keys of `sprint-status.yaml` stay. A title changes when its slug does not equal the key.
  The line `**Key ...**` below such a heading holds the former long title.
- **Proof:** `sprint_plan.py generate --epic-file <this file> --status-file <copy> --dry-run` reports `in_sync: true`.
- **New increment:** append a new part at the end, with `## Epic N: <title>` and `### Story N.M: <title>` headings.

| Part | Increment | Former file |
|---|---|---|
| 1 | 1-4 Robustness Remediation | `epics.md` |
| 2 | 5 Live-Cleanup-Preview | `epics-live-preview.md` |
| 3 | 6 Floating Bar Re-Architecture | `epics-bar-redesign.md` |
| 4 | 7 Cross-Platform Config-Contract Parity | `epics-cross-platform-parity.md` |
| 5 | 8 + 9 Visual Overhaul "Studio Dark" | `epics-visual-overhaul.md` |
| 6 | 10 Native Desktop Overlays | `epics-native-overlays.md` |
| 7 | 11 Cross-Platform Live-Preview | none (written 2026-09-20) |
| 8 | 12 Cloud-Resilienz | `epics-cloud-resilience.md` |
| 9 | 13 Parity-Linie über Audit #2 | `epics-parity-line-audit-2.md` |

# Part 1: Robustness Remediation (1-4)

*Former file: `epics.md`. Its frontmatter is kept below.*

```yaml
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories", "step-04-final-validation"]
inputDocuments:
  - docs/robustness-audit-2026-05-30.md
  - docs/adr/0015-state-file-write-convention.md
  - docs/adr/0016-android-path-parity-strategy.md
  - docs/remediation-session-kickoff.md
trackType: brownfield
```



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

### Story 1.1: Atomic state file writes via a shared save atomic helper

**Key `1-1` — Atomic state-file writes via a shared `save_atomic` helper**

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

### Story 1.2: Backup on corrupt recovery in load config

**Key `1-2` — Backup-on-corrupt recovery in `load_config`**

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

### Story 1.4: Hardened config migration backup and error propagation

**Key `1-4` — Hardened config migration — pre-migration backup + error propagation**

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

### Story 1.5: Migration ladder regression test history db open db

**Key `1-5` — Migration-ladder regression test — history-DB `open_db()`**

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

### Story 2.4: Banking app blocklist guards the paste path

**Key `2-4` — Banking-app blocklist guards the paste path, not just the bubble**

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

### Story 4.1: Isolate the load config core into migrate and normalize

**Key `4-1` — Isolate the `load_config` core into a tested `migrate_and_normalize`**

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

### Story 4.3: Single sanctioned config write path save config locked

**Key `4-3` — Single sanctioned config-write path (`save_config_locked` choke-point)**

**[Scope-fence EXCEPTION — decided 2026-05-31, Andi]** Pulled forward ahead of Stories 1.4/1.5 as a
one-off exception to the fence above (4.1/4.2 remain deferred). Source: code review of Story 1.3,
decision D1 Option 2. Rationale: tightly coupled to Story 1.3, which just rewired the 18 config-save
sites with an identical hand-written lock pattern — extracting the choke-point while the context is
hot and the sites are uniform is far cheaper than re-loading them later.

As a klarvo maintainer,
I want a single sanctioned `AppState::save_config_locked` that is the only runtime path to persist
`config.json`,
So that the ROB-04 disk-write serialization invariant is enforced by structure (one choke-point, the
lock impossible to get wrong at a call site) instead of by reviewer vigilance across 18 hand-written
copies, and the concurrency specs bind to the real production path.

**Acceptance Criteria:** see story file
`_bmad-output/implementation-artifacts/4-3-single-sanctioned-config-write-path-save-config-locked.md`
(helper added; all 18 sites routed; `save_config` demoted to `pub(crate)`; no behavior change; specs
rebound to the real helper; cargo test green + clippy clean on touched files).
**Findings covered:** code-review-1.3-D1 (deferred → pulled forward).
**Status:** done (2026-05-31).


# Part 2: Live-Cleanup-Preview (5)

*Former file: `epics-live-preview.md`. Its frontmatter is kept below.*

```yaml
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories", "step-04-final-validation"]
status: complete
inputDocuments:
  - docs/feature-ideas.md  # "Live-Cleanup-Preview" block — ✅ ENTSCHIEDEN 2026-06-03 + 7 resolved forks
  - _bmad-output/project-context.md
  - docs/adr/0016-android-path-parity-strategy.md  # NFR3 cross-platform config safety
trackType: brownfield-feature
featureEpic: 5
note: >
  Separate planning artifact by design — epics.md is the CLOSED robustness-remediation
  breakdown (Epics 1-4). This is the first FEATURE epic (Epic 5), built via the L3 feature
  route. There is no PRD/Architecture/UX doc; the requirements source is the resolved
  decision block in docs/feature-ideas.md (the WAS/WARUM + all 7 detail forks closed this
  session, grounded in a current-code audit). Shares the sprint-status.yaml ledger.
```



## Overview

The first **feature** epic on `v1-ship` (Epics 1-4 were robustness remediation). It adds an
**opt-in orientation preview**: while dictating in **Toggle/Hold**, speech-pause-triggered raw
Groq segments accumulate in an auto-expanding FloatingBar panel so the user can read along
during a long dictation. The preview is **purely display** — it never feeds the pasted output
(Variant B: at finish the existing whole-buffer path runs unchanged). German-accuracy risk is
therefore out of scope by construction.

There is no PRD/Architecture/UX document. Requirements below are extracted from the resolved
decision in `docs/feature-ideas.md` ("✅ ENTSCHIEDEN 2026-06-03" + the 7 closed forks **D1–D7**)
and grounded in a current-code audit performed this session. IDs are kept native (FR/NFR + the
D-fork they trace to) so traceability back to the decision stays intact.

**L3 guards (carried into every story):** (G-A) a characterization test pinning the *existing*
behavior BEFORE the additive code is written; (G-B) runtime integration lives **in the
acceptance criteria**, not just unit-green. Surface/UI stories require a **Windows release build
+ manual press-to-paste smoke** in the DoD (project-context.md testing rules).

## Requirements Inventory

### Functional Requirements

- **FR1 (D1, D2)** — While recording in **Toggle or Hold** with Preview enabled, on each
  detected speech pause ≥ the Preview-Pause threshold, the audio **delta since the last pause**
  is transcribed via Groq as **raw text** (no per-segment cleanup) and **appended** to an
  accumulating preview. Recording continues — the flush does **not** stop recording, does **not**
  paste, and does **not** loop.
- **FR2 (D5)** — The accumulated preview renders in the FloatingBar as **Variant 1**: the pill
  **auto-expands** downward into a **scrollable text panel** (fixed max-height) on the first
  chunk and **auto-scrolls to the newest** text.
- **FR3 (D2)** — The preview is **display-only and never feeds the output**. At **Finish** (key
  release / 2nd tap / shortcut), the **existing finish path runs unchanged**: whole WAV →
  `process_audio` → cleanup → **single paste**. Output for Toggle/Hold is **byte-identical to
  today**.
- **FR4 (D7)** — The preview is active **only in Toggle and Hold**. **Auto and AutoStop never
  show a preview feed** (they already transcribe+paste per segment; a feed would double).
- **FR5 (D4)** — The preview is **disabled in the offline/local-STT path** (`stt_provider ==
  "local"`): no Groq flush fires there. Waveform feedback remains.
- **FR6** — The preview is **opt-in** via a **Settings toggle** (default **off**).
- **FR7 (D6)** — At Finish the accumulated preview **clears with the done-pop** (does not
  persist for review).
- **FR8 (D3 — Regler A)** — A new general **"Preview-Pause"** slider in the Shortcut settings
  section sets the Preview-Pause threshold, stored in a **new** config key
  `preview_pause_silence_secs` (default **2.0**). Drives FR1's flush timing for Toggle+Hold.
- **FR9 (D3 — Regler B)** — A single general **"Send/Stop-Pause"** slider in the Shortcut
  settings section replaces the two per-mode controls and writes **both existing** keys
  `auto_mode_silence_secs` **and** `autostop_silence_secs` to the **same** value. **No key is
  renamed or removed.**
- **FR10 (D8 — Darstellungsform, added 2026-06-05)** — The preview panel's **display form is
  user-selectable via curated presets** — **Compact / Comfortable / Wide** — in the preview
  Settings section, stored in a **new** enum config key `preview_panel_form` (default
  **`comfortable`**, which reproduces the shipped 5-2 look exactly). Each preset maps to a coherent
  appearance set (panel width + screen-cap) in `FloatingBar.tsx`; **no raw-pixel sliders** (Andy's
  choice 2026-06-05). Desktop-only (the preview is Groq-only desktop). Full *layout* variants (other
  mockup forms) are explicitly **out of scope** here — parked as a later 5-6/backlog idea.

### NonFunctional Requirements

- **NFR1 (cost — the core constraint)** — The flush transcribes **only the delta** since the
  last pause, **never the growing whole buffer**. This is the exact failure of the old
  live-preview poller (3 s poll → `snapshot_wav()` of the whole buffer → re-transcribe →
  "10-20x Groq quota", which got it disabled). Per-segment STT during recording must total
  **~1× audio**, not N×. (Variant B's finish re-Groq adds the documented, accepted **~2× total**
  — segments + one whole-buffer pass — at finish.)
- **NFR2 (no-regression)** — The finish/paste path is **not touched**. The preview is a parallel,
  additive display path. Disabling Preview ⇒ behavior is exactly today's.
- **NFR3 (cross-platform config safety — ADR-0016)** — No existing `*_silence_secs` key is
  renamed/removed. Android (`KlarvoOverlayService.kt`) keeps reading `auto_mode_silence_secs` +
  `autostop_silence_secs` unchanged. `preview_pause_silence_secs` is **desktop-only**; Android
  ignores it. **Zero migration** of existing `config.json` keys.
- **NFR4 (event naming — G3)** — The new preview-chunk event uses **colon** form
  (`klarvo://live-preview-chunk`), never dots (Tauri reserves `.`).
- **NFR5 (threading)** — The pause-flush dispatches **async, off** the cpal OS audio-callback
  thread, non-blocking — like the existing pipeline.
- **NFR6 (BYOK / no telemetry)** — The preview adds **no** network calls beyond the user's
  configured Groq STT endpoint.

### Additional Requirements (from the current-code audit)

- **AR1 — Delta-snapshot primitive.** `audio/mod.rs:416 snapshot_wav()` returns the **whole**
  accumulated buffer (no marker/cursor). The feature needs an audio-since-last-pause slice:
  a sample-position marker captured at each pause + a slice→WAV encode. This is **net-new** and
  is the load-bearing backend primitive.
- **AR2 — Flush-without-stop silence callback for Toggle/Hold.** Today Toggle/Hold have **no**
  silence detection (`pipeline.rs:2008-2023` — stop only on user action). Auto/AutoStop's silence
  callbacks **stop** (and Auto loops). The feature installs a **new** callback in Toggle/Hold that
  fires the delta-flush and **keeps recording**.
- **AR3 — Push, not poll.** The old `FloatingBar.tsx` preview (commented at :389-405) **polled**
  the live `transcribe_live_preview` command (still live at `commands/recording.rs:346`) every
  3 s and whole-buffered. The feature replaces poll-whole-buffer with **event-push of deltas**
  (`klarvo://live-preview-chunk`). Re-enable the `livePreview` state (`FloatingBar.tsx:218`).
- **AR4 — Bar window resize for the panel.** The bar window `setSize`/`setBarShape("pill")` logic
  (`FloatingBar.tsx:280-308`) sizes a fixed 200×36 pill; Variant 1 needs a taller panel size +
  shape and a resize-back on collapse.
- **AR5 — Settings surface.** Add the opt-in toggle + Regler A/B sliders to the Shortcut section
  of the settings UI and persist via the single sanctioned `save_config` write path (ADR-0015 /
  Story 4-3); the React strings live in the existing settings UI.

### UX Design Requirements

- **UX-DR1 — Variant 1 (Auto-Expand-Panel).** Decided by Andy 2026-06-03 from rendered mockups
  (`/tmp/klarvo-mockups/bar.png`, faithful to `FloatingBar.tsx`): pill grows into a scrollable
  panel, fixed max-height, auto-scroll to newest, top-fade for scrolled-off text, thin
  scroll-indicator. Recording-accent border/teal logo/waveform unchanged. Collapses back to the
  pill on done-pop (FR7).

### FR Coverage Map

- **FR1** → Epic 5 (Story 5.1) — pause-triggered delta Groq flush, no stop/paste/loop
- **FR2** → Epic 5 (Story 5.2) — auto-expand scrollable panel (Variant 1)
- **FR3** → Epic 5 (Story 5.1, characterization) — finish path unchanged; preview never feeds output
- **FR4** → Epic 5 (Story 5.1) — Toggle/Hold only; Auto/AutoStop excluded
- **FR5** → Epic 5 (Story 5.1) — disabled in offline/local-STT path
- **FR6** → Epic 5 (Story 5.3) — opt-in Settings toggle (default off)
- **FR7** → Epic 5 (Story 5.2) — preview clears with done-pop
- **FR8** → Epic 5 (Story 5.3) — Regler A new key `preview_pause_silence_secs`
- **FR9** → Epic 5 (Story 5.4) — Regler B one slider → both existing keys (separable / deferral seam)
- **FR10** → Epic 5 (Story 5.5) — preview display-form presets (Compact/Comfortable/Wide), default comfortable = shipped look

All NFR1–NFR6 and AR1–AR5 are cross-cutting within Epic 5 (see per-story ACs).

## Epic List

### Epic 5: Live-Cleanup-Preview

When dictating a long passage in **Toggle or Hold**, the user can turn on an **opt-in live
preview**: raw Groq segments accumulate at speech pauses in an **auto-expanding FloatingBar
panel**, so they can **read along and spot errors before finishing** — while the **final pasted
text is produced exactly as today** (the preview never feeds the output). Standalone: builds only
on existing v1 recording/pipeline/FloatingBar surfaces; enables no future epic but closes the
long-parked "Live-Overlay" feature.

**FRs covered:** FR1, FR2, FR3, FR4, FR5, FR6, FR7, FR8, FR9
**NFRs:** NFR1–NFR6 · **AR:** AR1–AR5 · **UX:** UX-DR1

**Planned story decomposition** (detailed in Step 3 — shown here for shape review):

- **5.1 — Backend: delta-flush core** *(Wave 1, foundation)*. AR1 delta-snapshot primitive +
  AR2 flush-without-stop silence callback for Toggle/Hold + raw Groq segment transcribe +
  `klarvo://live-preview-chunk` event (NFR4). Scope guards FR4 (Toggle/Hold only) + FR5 (offline
  off). **G-A characterization test FIRST:** pin today's Toggle/Hold finish path (stop →
  `process_audio` → single paste) so FR3/NFR2 no-regression is provable. Covers FR1, FR3, FR4,
  FR5, NFR1, NFR2, NFR4, NFR5.
- **5.2 — Frontend: auto-expand preview panel** *(depends on 5.1's event)*. AR3 push-not-poll
  accumulation + AR4 bar window resize + Variant 1 panel (UX-DR1) + clear-on-done. Surface story
  → Windows release build + manual press-to-paste smoke in DoD. Covers FR2, FR7.
- **5.3 — Settings: opt-in toggle + Preview-Pause slider (Regler A)**. FR6 toggle (default off)
  gating the whole feature + FR8 new key `preview_pause_silence_secs` via the sanctioned
  `save_config` path (ADR-0015). Surface story → smoke. Covers FR6, FR8.
- **5.4 — Config: Send/Stop-Pause consolidation (Regler B)** *(separable — the deferral seam)*.
  FR9 one slider writes both existing `auto_mode_silence_secs` + `autostop_silence_secs`; **no
  key rename/removal**, Android unaffected (NFR3, ADR-0016). Independent of 5.1–5.3 — can ship,
  defer, or drop without touching the preview. Covers FR9, NFR3.
- **5.5 — Settings: Preview display-form presets (Compact/Comfortable/Wide)** *(added 2026-06-05;
  depends on 5.3's preview Settings section)*. FR10 enum key `preview_panel_form` (default
  `comfortable` = shipped look) via the sanctioned `save_config` path (ADR-0015); `FloatingBar.tsx`
  reads it and selects the form's width + screen-cap. No raw px-sliders, no layout variants. Surface
  story → Windows smoke. Covers FR10.
- **5.7 — Hardening: preview-flush stale-chunk guard + in-flight backpressure** *(added 2026-06-05;
  depends on 5.1's backend flush + 5.2's frontend listener — both done)*. Closes the carried-forward
  **5.1-C2** review defer (out-of-order / no-backpressure flushes) **and** its 5.2 face (late chunks
  bleeding across recordings). Frontend session-token / `isRecording` guard in the chunk listener +
  backend in-flight cap on `flush_preview_delta`. Surface story (touches FloatingBar) → Windows smoke.
  Not a new FR — a robustness hardening of the shipped feature (NFR1 cost + no-stale-bleed). *(5-6 stays
  reserved for the parked full-layout-variants backlog idea.)*

**Dependency flow:** 5.1 → 5.2; 5.3 parallel to 5.2 (independent surfaces); 5.4 fully
independent; **5.5 → after 5.3** (plugs into its Settings section); **5.7 → after 5.1 + 5.2** (hardens
their shipped flush/listener). No story depends on a *later* story.

## Epic 5: Live-Cleanup-Preview

When dictating a long passage in Toggle or Hold, the user can enable an opt-in live preview:
raw Groq segments accumulate at speech pauses in an auto-expanding FloatingBar panel so they can
read along and spot errors before finishing — while the final pasted text is produced exactly as
today (the preview never feeds the output).

### Story 5.1: Backend — pause-triggered delta-flush for Toggle/Hold

As a developer extending the recording pipeline,
I want a delta-snapshot + flush-without-stop path that transcribes only the new audio since the
last pause and emits it as a preview chunk in Toggle/Hold,
So that the live preview can accumulate raw text at ~1× STT cost without touching the finish/paste path.

**Acceptance Criteria:**

**Given** the current v1 Toggle and Hold finish behavior (stop → `process_audio` → single paste)
**When** a characterization test drives a fixed WAV fixture through the Toggle and Hold finish path with Preview disabled
**Then** it pins the produced `cleaned_text`/`raw_text` and single-paste outcome as a golden assertion
**And** this test is written and green BEFORE any preview code is added — it is the G-A no-regression baseline for FR3/NFR2 (the L3 "characterization-test-before-touching-existing-code" guard).

**Given** a recording in progress with an accumulating live buffer (`audio/mod.rs` live_buffer)
**When** the new audio API is asked for a delta snapshot at a pause boundary
**Then** it returns a WAV of only the samples captured since the previous delta marker (not the whole buffer, unlike today's `snapshot_wav()`)
**And** it advances the marker so the next delta starts where this one ended
**And** a unit test on a synthetic sample stream asserts two consecutive deltas are disjoint and together equal the full buffer (NFR1 — proves ~1× not N×).

**Given** Story 5.1 owns the two new `AppConfig` fields — `live_preview_enabled` (default `false`) and `preview_pause_silence_secs` (default `2.0`) — added with serde defaults so 5.1 is self-contained (no UI yet → flush never fires for a real user until Story 5.3 wires the toggle; tests set the fields directly)
**When** the schema is loaded
**Then** both fields read with their defaults and trigger NO migration write (additive defaults), so 5.1 has no forward dependency on 5.3.

**Given** Toggle or Hold mode is active, `live_preview_enabled == true`, and `stt_provider != "local"`
**When** a speech pause ≥ `preview_pause_silence_secs` is detected
**Then** the delta segment is transcribed via the configured Groq STT provider as raw text (no per-segment cleanup, FR1/D1)
**And** recording continues uninterrupted — no stop, no paste, no auto-loop
**And** the raw segment text is emitted on event `klarvo://live-preview-chunk` as an append payload (NFR4 — colon form, never dots).

**Given** the pause is detected on the cpal OS audio-callback thread
**When** the flush is triggered
**Then** the Groq transcription runs on an async task off the callback thread (non-blocking), mirroring the existing pipeline dispatch (NFR5).

**Given** Auto or AutoStop mode (not Toggle/Hold)
**When** a pause is detected
**Then** the existing per-segment stop/paste/loop behavior runs unchanged
**And** NO `klarvo://live-preview-chunk` event is emitted (FR4 scope guard — no double feed).

**Given** `stt_provider == "local"` (offline path, `is_offline()` true, `pipeline.rs:450`)
**When** recording in Toggle/Hold with Preview enabled
**Then** no delta flush fires and no chunk event is emitted (FR5 — preview disabled offline)
**And** waveform feedback is unaffected.

**Given** a delta-segment Groq transcription fails (network / 429 / 5xx) mid-recording
**When** the flush completes
**Then** the failing chunk is skipped (no append, or an explicit empty-skip payload), recording continues, and no error is surfaced to the user mid-stream (fail-soft — matches the existing `transcribe_live_preview` error-swallow at `commands/recording.rs`).

**Given** any number of preview chunks were emitted during a Toggle/Hold recording
**When** the user finishes (release / 2nd tap / shortcut)
**Then** the finish path runs the existing whole-WAV → `process_audio` → single paste, unchanged (FR3)
**And** the AC-1 characterization test still passes — output byte-identical to Preview-off (NFR2).

**DoD:** Backend story. Linux `cargo test` (characterization + delta-snapshot unit + guard logic) + `clippy` clean on touched files. End-to-end runtime is exercised by Story 5.2's smoke gate (the event has no user-visible effect until the frontend consumes it).

### Story 5.2: Frontend auto expand preview panel

**Key `5-2` — Frontend — auto-expand preview panel (Variant 1)**

As a user dictating a long passage in Toggle or Hold,
I want the FloatingBar to grow into a scrollable panel that accumulates the preview text and auto-scrolls to the newest line,
So that I can read along and spot errors before I finish.

**Acceptance Criteria:**

**Given** Preview is enabled and a recording is active in Toggle/Hold
**When** `klarvo://live-preview-chunk` events arrive
**Then** each chunk's raw text is appended to an accumulating preview string in the bar (push, not poll — AR3)
**And** the old 3 s-poll `transcribe_live_preview` caller stays removed (the commented block at `FloatingBar.tsx:389-405` is NOT re-enabled as a poller; the `livePreview` state at :218 is re-enabled as a push sink).

**Given** the first preview chunk arrives
**When** the bar renders
**Then** the pill auto-expands downward into a scrollable text panel — Variant 1: fixed max-height, top-fade for scrolled-off text, thin scroll-indicator, recording-accent border, teal logo + waveform retained (UX-DR1, FR2)
**And** the panel auto-scrolls to the newest text as chunks append.

**Given** the panel expands or collapses
**When** the bar window is resized
**Then** `setSize` + `setBarShape` are applied before `show`, preserving the white-line shape-guard ordering (`FloatingBar.tsx:280-308`)
**And** drag/position persistence (`saveBarPosition`/`getBarPosition`) is NOT regressed — manual smoke confirms drag still works while the panel is expanded (AR4 edge guard).

**Given** the user finishes (done state)
**When** the done-pop fires
**Then** the accumulated preview clears and the bar collapses back to the pill/done-pop with no lingering panel (FR7).

**Given** Preview is disabled (default)
**When** recording in any mode
**Then** no panel appears and the bar behaves exactly as today — pill + waveform (FR6 interaction, NFR2).

**Given** a chunk event carries an empty/skip payload (from a failed segment flush, 5.1 fail-soft)
**When** the bar processes it
**Then** nothing is appended and the panel does not flicker or error.

**DoD:** Surface story → **Windows release build + manual press-to-paste smoke**: dictate a multi-pause passage in Toggle with Preview on, watch the panel accumulate and auto-scroll, finish, confirm the correct single paste lands AND the panel clears. Linux `cargo test` + `tsc`/`npm run build` + `clippy` on touched.

### Story 5.3: Settings opt in preview toggle and preview pause slider

**Key `5-3` — Settings — opt-in Preview toggle + Preview-Pause slider (Regler A)**

As a user,
I want a Settings toggle to turn the live preview on/off and a Preview-Pause slider to set how long a pause triggers a flush,
So that the preview is opt-in and I can tune its responsiveness.

**Acceptance Criteria:**

**Given** the Shortcut section of the settings UI, and the `live_preview_enabled` field already exists in `AppConfig` (introduced by Story 5.1)
**When** the user toggles "Live Preview"
**Then** the existing `live_preview_enabled` field is written via the single sanctioned `save_config` write path (ADR-0015 / Story 4-3 — no second writer)
**And** the value gates the Story 5.1 flush (off → no flush, no event, FR6).

**Given** the Shortcut section
**When** the user adjusts the "Preview-Pause" slider (Regler A)
**Then** the `preview_pause_silence_secs` field (introduced in 5.1; range matching the existing silence sliders, e.g. 0.5–5.0) is written via `save_config`
**And** Story 5.1's flush uses this value as the pause threshold (FR8/D3).

**Given** a fresh or existing `config.json` with neither field set by the user
**When** it is loaded
**Then** `live_preview_enabled` reads `false` and `preview_pause_silence_secs` reads `2.0` via the serde defaults from 5.1
**And** NO migration write is triggered (additive defaults only — existing users see zero behavior change, NFR2).

**Given** the Preview-Pause slider
**When** it is shown
**Then** it carries the trade-off hint: short = more responsive + more Groq calls + shorter context per segment; long = less responsive + fewer calls + better context (decision doc).

**DoD:** Surface story → **Windows release build + manual smoke**: toggle on, set the slider, confirm the flush timing changes; toggle off, confirm the preview is gone. `tsc` build + `cargo test`.

### Story 5.4: Config send stop pause consolidation

**Key `5-4` — Config — Send/Stop-Pause consolidation (Regler B)**

As a user,
I want a single "Send/Stop-Pause" slider instead of two separate per-mode controls,
So that the Shortcut settings are simpler — without breaking any platform that reads the underlying keys.

**Acceptance Criteria:**

**Given** the Shortcut section
**When** the user adjusts the single "Send/Stop-Pause" slider (Regler B)
**Then** BOTH existing keys `auto_mode_silence_secs` AND `autostop_silence_secs` are written to the same value via `save_config` (FR9/D3)
**And** the two prior separate per-mode controls are removed from the UI.

**Given** the `AppConfig` schema
**When** Story 5.4 ships
**Then** neither `auto_mode_silence_secs` nor `autostop_silence_secs` is renamed or removed
**And** no config migration is added — the keys keep their identity and defaults (NFR3 — zero migration risk).

**Given** Android reads `auto_mode_silence_secs` + `autostop_silence_secs` mode-centrically (`KlarvoOverlayService.kt:807-808`)
**When** the desktop UI consolidation ships
**Then** a test or documented verification confirms both keys still exist with unchanged names/defaults so the Kotlin reads are unaffected
**And** NO Android code change is required by this story (ADR-0016 parity; [[android_silence_field_divergence]] guard).

**Given** the two keys currently hold different values (edge: hand-edited `config.json`)
**When** the consolidated slider opens
**Then** it displays one defined value (the larger of the two) and writing re-unifies both — documented behavior, not a silent pick.

**DoD:** Config-surface story → **Windows release build + manual smoke**: move the slider, confirm both keys change in `config.json`, confirm Auto + AutoStop still silence-stop at the new value. `cargo test` for the write-both behavior.

### Story 5.5: Settings preview display form presets

**Key `5-5` — Settings — Preview display-form presets (Compact/Comfortable/Wide)**

*Added 2026-06-05 after the 5-2 Windows smoke: a single hardcoded `PANEL_WIDTH` is the right default
but Andy wants the form selectable. Decision (Andy): curated presets, NOT raw-pixel sliders, NOT full
layout variants. Depends on Story 5.3's preview Settings section.*

As a user,
I want to pick the live-preview's display form from a few curated presets (Compact / Comfortable / Wide),
So that I can size the read-along panel to my taste without fiddling with raw pixels.

**Acceptance Criteria:**

**Given** `AppConfig`
**When** Story 5.5 ships
**Then** a new enum-style key `preview_panel_form` (values `"compact" | "comfortable" | "wide"`) exists with a serde default of `"comfortable"`
**And** the default is additive — a config without the key reads `"comfortable"`, triggers **no migration write**, and reproduces the **exact** shipped 5-2 look (width 320, screen-cap 320) — zero behavior change for existing users (NFR2)
**And** an unknown/garbage value falls back to `"comfortable"` (fail-soft, no panic).

**Given** the preview Settings section (built by Story 5.3)
**When** the user picks a display form (segmented/radio control labeled e.g. "Darstellung")
**Then** `preview_panel_form` is written via the single sanctioned `save_config` write path (ADR-0015 / Story 4-3 — no second writer)
**And** the control shows the three presets with the current one selected.

**Given** `FloatingBar.tsx` currently hardcodes `PANEL_WIDTH` (and `PANEL_ABS_MAX`)
**When** Story 5.5 ships
**Then** those constants are replaced by a form→appearance map (each preset = a coherent width + screen-cap pair; `comfortable` == today's 320/320)
**And** the bar reads `preview_panel_form` (via `getSettings` on mount + the settings-changed path) so a changed preset applies to the **next** preview open without an app restart.

**Given** the preview is disabled, or the active mode is Auto/AutoStop (no preview there)
**When** any form is selected
**Then** there is **no** visual or behavioral effect (the form only sizes the Toggle/Hold preview panel) — FR4/FR6 boundaries unchanged.

**Open for create-story / dev to finalize:** the exact width + screen-cap numbers per preset (illustrative: Compact ≈ 260/240, Comfortable = 320/320, Wide ≈ 400/400); whether a preset also varies font-size/line-height (density) or width-only. Default MUST equal the shipped look.

**DoD:** Surface story → **Windows release build + manual smoke**: cycle all three presets, confirm the panel renders at each width and `comfortable` is byte-identical to the shipped look; confirm the choice persists across a relaunch. `tsc` build + `cargo test` (config field default + fail-soft). Desktop-only — **no Android change** (preview is Groq-only desktop).

### Story 5.7: Preview flush hardening stale chunk guard and backpressure

**Key `5-7` — Hardening — preview-flush stale-chunk guard + in-flight backpressure**

*Added 2026-06-05 from the Epic-5 retro carry-forward. Closes the tracked **5.1-C2** review defer
and its 5.2 face — see `deferred-work.md` "From code review of story-5.1 (2026-06-04)" (Concurrent /
out-of-order preview flushes + no backpressure) and "From code review of story-5.2 (2026-06-04)"
(Concurrent / out-of-order / late preview-flush chunks bleed across recordings). The race is **real
but never reproduced** (default 2.0 s pause ≫ sub-1 s Groq latency makes overlap rare) and low-severity
(preview is orientation-only / throwaway, Variant B). This story applies **defensive guards on both
layers**; the DoD verifies the happy path is unregressed rather than gating on reproducing the rare
race. Depends on Stories 5.1 (backend `flush_preview_delta`) and 5.2 (frontend chunk listener), both done.*

As a developer hardening the shipped live preview,
I want stale/out-of-cycle preview chunks dropped at the listener and concurrent backend flushes capped,
So that a late chunk can never bleed into the wrong recording and a flurry of short pauses can never launch unbounded concurrent Groq calls — without changing the normal preview experience.

**Acceptance Criteria:**

**Given** today's normal live-preview behavior (Toggle/Hold, one recording, current-cycle chunks append and render per Story 5.2)
**When** the hardening is added
**Then** the in-cycle happy path is pinned as the no-regression baseline (backend: a test exercising the real flush-spawn path; frontend: the Story-5.2 Windows happy-path smoke) and stays green — the guards drop **only** stale/excess chunks, **never** a legitimate current-cycle chunk (FR2/NFR2, the L3 G-A "characterization-before-touching-code" guard).

**Given** Auto-Loop, or a finished Toggle/Hold cycle, where a `klarvo://live-preview-chunk` from cycle N is emitted **after** cycle N's `done` or **after** cycle N+1 has already started (the async `flush_preview_delta` emits only after the Groq round-trip — `pipeline.rs:1925-1927`)
**When** the chunk arrives at the frontend listener (`FloatingBar.tsx:288-294`, which today appends unconditionally)
**Then** the listener **drops** it via a session-token or `isRecording`-ref guard, so the stale chunk neither re-populates a just-cleared `livePreview` nor bleeds into the next recording's fresh buffer
**And** Story 5.2's recording-entry `setLivePreview("")` reset is preserved (the guard closes the in-flight-after-reset hole the reset alone could not).

**Given** a normal current-cycle chunk arriving during its own active recording
**When** the guarded listener processes it
**Then** it appends and renders exactly as today — the guard is a pass-through for in-cycle chunks (no regression to 5.2's accumulation / auto-grow / auto-scroll).

**Given** a flurry of short speech pauses in Toggle/Hold with Preview enabled, each Speaking→Silence edge spawning an independent `tauri::async_runtime::spawn(flush_preview_delta)` (`pipeline.rs:1977`) with no in-flight cap today
**When** multiple flushes would be in flight at once
**Then** concurrent in-flight flushes are **capped** (in-flight guard / serialization) so a pause-flood cannot launch unbounded concurrent Groq calls
**And** an excess flush is cleanly coalesced or skipped — acceptable because the preview is orientation-only/throwaway
**And** a unit test asserts the cap holds under N rapid pause triggers.

**Given** the in-flight cap coalesces or skips an excess flush
**When** the delta marker is managed
**Then** NFR1 is preserved — no double STT cost and no marker corruption (a skipped delta is either dropped or folded into the next flush, never double-transcribed)
**And** a unit test on the delta marker under capped/skipped flushes asserts deltas stay disjoint (no re-transcribe of already-marked audio).

**Given** Preview disabled (default), or `stt_provider == "local"` (offline), or Auto/AutoStop mode
**When** the hardening ships
**Then** those paths are unchanged — the guards are no-ops there (FR4/FR5/FR6 boundaries intact, NFR2).

**DoD:** Surface story (touches `FloatingBar.tsx` + the flush spawn path) → **Windows release build + manual smoke** per `docs/surface-smoke-checklist.md`: (1) happy path — a normal Toggle/Hold multi-pause dictation still accumulates, renders, auto-scrolls and clears on done; (2) the stale-bleed scenario — an Auto-Loop / rapid finish-then-restart sequence shows **no** leftover preview text bleeding into the next recording. Backend: Linux `cargo test` (in-flight cap + delta-marker integrity) + `clippy` clean on touched files. Frontend: `tsc` / `npm run build`. Empirical inversion check at writing time per the Epic-4-retro control (flip a guard → the relevant test goes RED), reviewer-verified at code review (not self-attested). Desktop-only — **no Android change** (preview is Groq-only desktop).

# Part 3: Floating Bar Re-Architecture (6)

*Former file: `epics-bar-redesign.md`. Its frontmatter is kept below.*

```yaml
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories"]
status: stories-defined
inputDocuments:
  - docs/bar-redesign-spec.md          # Soll-Spec + Foundation Design (the codeable contract)
  - docs/deep-dive-bar-subsystem.md    # Ist-Zustand + the 13-item race table this retires
  - _bmad-output/project-context.md
  - docs/surface-smoke-checklist.md     # surface-class DoD control
trackType: brownfield-feature
featureEpic: 6
note: >
  Re-architecture epic, not a new feature. Separate planning artifact: epics.md is the CLOSED
  remediation breakdown (Epics 1-4); epics-live-preview.md is Epic 5 (the live-preview feature).
  Epic 6 RE-ARCHITECTS the bar so the geometry race class cannot exist by construction. The
  Epic-5 retro's "no Epic 6" predates this need and is explicitly reversed (four failed
  single-window geometry fixes are the trigger). No PRD/Architecture/UX doc — the requirements
  source is docs/bar-redesign-spec.md (foundation design, grounded in the deep-dive audit).
  Shares the sprint-status.yaml ledger. Per-story full context via bmad-create-story per session.
  Supersedes the carried-forward Story 5-7 grow-upward clip blocker (parked → folded into 6-5).
```



## Overview

> **Resume (2026-06-07):** see `WAYPOINT.md` at repo root. Story 6-6 (preview appearance, redesigned) is functionally complete & confirmed on the real build; it is blocked only by a **cosmetic corner artifact** (bottom-left of the preview box renders rough). Next BMAD action = fix that corner **observe-first** (region + backdrop-filter already ruled out — do not retry; see `project-context.md` rule 29), routed through a fresh `bmad-dev-story`/`bmad-quick-dev` worker — **or** consciously defer it and start `6-3` (font-size = Increment B of the same panel).

Four geometry fix-attempts on the single-window FloatingBar failed (the last made it worse and was
reverted). Root cause, confirmed by the deep-dive (`docs/deep-dive-bar-subsystem.md`): the bar is
**one window** that re-measures → resizes → reshapes → repositions itself on **every preview chunk**
via independent async IPC. The races **R3/R4/R5/R6/R10/R11** are six faces of that single design
choice; point-guards cannot make per-chunk async geometry atomic.

This epic **re-designs** the surface (it does not refactor it):

1. The **pill becomes fully static** — fixed size, never resizes; the width-preset no longer affects
   it.
2. The **live preview moves into its own transparent, click-through `"preview"` window** above the
   pill, **centered**, growing **upward**.
3. The preview window is created **once at its full limit height**; the dark card grows **via pure
   CSS** inside it and scrolls past the cap — so there is **no per-chunk window IPC at all** (NFR1).
   This eliminates the entire geometry race class **by construction**.
4. Preview presentation is driven by a **single scale factor** `k = fontPx / 11`: width presets and
   the height limit are defined at the small font and scale with the chosen font size.

**FRs covered:** FR1–FR13 · **NFRs:** NFR1–NFR6 · **AR:** AR1–AR5 · **UX:** UX-DR1

## Requirements Inventory

Brownfield re-architecture: requirements are extracted from `docs/bar-redesign-spec.md`. IDs are
native to this epic.

### Functional Requirements

- **FR1** — The pill (`"bar"` window) is **static**: one fixed size, it and its elements never resize;
  the width-preset (Compact/Comfortable/Wide) no longer affects it.
- **FR2** — Live preview renders in a **separate `"preview"` window** above the pill: transparent,
  click-through, always-on-top, decorationless, skip-taskbar.
- **FR3** — The preview window is **horizontally centered over the pill** and stretches **upward**.
- **FR4** — The preview card **grows with the text up to a fixed height limit**, then **scrolls**
  inside (top-fade). The limit is independent of the width preset.
- **FR5** — The **width preset** (Compact/Comfortable/Wide) affects **only** the preview width.
- **FR6** — A **new font-size setting** (Small/Medium/Large, default Small) affects **only** the
  preview; persisted as `preview_font_size`.
- **FR7** — Width, height limit, and font **scale together** by `k = fontPx / 11` (the scale-factor
  model). Widths 260/320/400 and height limit 600 are defined at the small font.
- **FR8** — The preview window **appears only while recording** when preview text is present and
  **hides when recording ends**.
- **FR9** — The preview **follows the pill on drag** (via `klarvo://bar-moved`); only the **pill**
  position is persisted, the preview position is always derived.
- **FR10** — **Recovery for both windows** (`ensure_bar_window` + new `ensure_preview_window`).
- **FR11** — Preview **text appearance** settings (preview-only): text **color** + **brightness/opacity**,
  and **font-family** choice. Pure-CSS; no geometry/`k` impact. (Motivation: on a dark background the
  default `rgba(220,220,220,0.88)` text reads as dim/non-pure-white and is easy to miss.)
- **FR12** — Preview **card (box) appearance** settings (preview-only): **background color** +
  **background opacity**, **backdrop-blur** strength. Pure-CSS.
- **FR13** — Preview **border appearance** settings (preview-only): border **color** +
  **brightness/opacity**, **line thickness**, and **corner radius**. (Motivation: default
  `1px rgba(42,195,168,0.25)` is a barely-visible thin line on dark pages.) Color/opacity/thickness
  are pure-CSS; **corner radius is the exception** — it is coupled to the OS window region via
  `set_preview_shape` (R11: region radius MUST equal CSS `borderRadius` or a white-line artifact
  appears), so a configurable radius also touches Rust + must preserve the R11 invariant.

### NonFunctional Requirements

- **NFR1** *(the race-class killer)* — **No per-chunk window IPC.** Window size, position, and region
  are set **once per show / once per drag**, never per preview chunk. Inversion: any per-chunk
  `setSize`/`setPosition`/region call re-introduces R3/R4/R5/R10.
- **NFR2** — **Behavior-preserving** for the pill state machine, waveform, mode badge, recovery, and
  the **final pasted output** (Variant B carried over — preview never feeds output). Only geometry
  and preview presentation change.
- **NFR3** — **Retain the legitimate guards**, do not regress them: R1 stale-chunk (frontend), R2
  backpressure (backend), R7 done→idle, R8/R9 backend offline/leak guards.
- **NFR4** — Tauri event names use the **colon form**; the new event is `klarvo://bar-moved`.
- **NFR5** — Config key **camelCase** (`previewFontSize`); writes via the sanctioned single-writer
  atomic path (`save_config_locked`, ADR-0015); missing-field default = `small` (no migration write).
- **NFR6** — **Windows-only surface.** Linux is near-zero signal: a real Windows release build +
  manual press-to-paste smoke is the hard gate; walk `docs/surface-smoke-checklist.md`.

### Additional Requirements (from the deep-dive audit)

- **AR1** — A **third window label `"preview"`** + Rust `create_preview_window` + `main.tsx` routing
  (`main` → App, `bar` → FloatingBar, `preview` → new `PreviewPanel`).
- **AR2** — The preview window is **fixed at the clamped max height**; the card is **bottom-aligned
  and grows via CSS**, scrolls past the cap.
- **AR3** — Max-height clamp = `min(BASE_MAX_HEIGHT × k, (bar_y − GAP) − (screenTop + 12))`, computed
  at **show-time and on drag** (not per chunk).
- **AR4** — The pill **loses all geometry/measure logic** (`panelHeight`, `panelScrolls`, `geomTick`,
  `measureRef`, `previewPanelRef`, the measure `useLayoutEffect`, the per-chunk grow/resize effect,
  the panel render).
- **AR5** — Remove `set_bar_shape("panel")` and the per-preset `screenCap` remnants; the pill region
  is set **once** at creation.

### UX Design Requirements

- **UX-DR1** — Preview grows upward, centered above the pill; transparent empty area above the card;
  top-fade when scrolling; click-through (display-only, no controls).

### FR Coverage Map

| FR/NFR/AR | Story |
|---|---|
| FR2 (shell), FR10, AR1, NFR6 | 6.1 |
| FR2, FR3, FR4, FR5, FR8, FR7(width), NFR1, NFR2, NFR3(R1), AR2, AR3, UX-DR1 | 6.2 |
| FR6, FR7(full), NFR5 | 6.3 |
| FR9, NFR4 | 6.4 |
| FR1, NFR2, AR4, AR5 | 6.5 |
| FR11, FR12, FR13, NFR5, NFR6 | 6.6 |

## Epic List

### Epic 6: Floating Bar Re-Architecture

The user dictates with the bar exactly as today, but the **pill never moves or resizes**, and the
**live preview is a separate window above it** that grows upward and caps+scrolls. The win is
structural: the geometry is **static per recording**, so the race class that broke four fix-attempts
**cannot exist**. Standalone: builds only on existing v1 recording/pipeline/FloatingBar surfaces;
enables no future epic but retires the bar's geometry debt and unblocks the parked 5-7.

**FRs covered:** FR1–FR13 · **NFRs:** NFR1–NFR6 · **AR:** AR1–AR5 · **UX:** UX-DR1

**Planned story decomposition** (full ACs below):

- **6.1 — Scaffold the standalone `"preview"` window** *(Wave 1, foundation)*. Rust
  `create_preview_window` + handler registration + `ensure_preview_window` recovery; `main.tsx`
  routes `"preview"` → empty `PreviewPanel`; transparent/click-through/always-on-top/hidden at
  startup. Covers FR2(shell), FR10, AR1, NFR6.
- **6.2 — Move live preview into the window (CSS-grow, scale geometry)** *(depends on 6.1)*. The big
  one. `PreviewPanel` subscribes to `state-changed` + `live-preview-chunk` (R1 guard retained);
  card grows upward via CSS inside the fixed-max window, caps+scrolls; geometry via `previewGeometry`
  (Small font + width presets); centered above the pill + screen clamp; show-once / hide-on-end. The
  pill's in-window preview is **disabled here** → exactly one preview. Covers FR2, FR3, FR4, FR5, FR8,
  FR7(width), NFR1, NFR2, NFR3(R1), AR2, AR3, UX-DR1.
- **6.3 — Font-size axis** *(depends on 6.2)*. New `preview_font_size` config + 3-way Settings picker;
  width + height + font scale by `k`. Covers FR6, FR7(full), NFR5.
- **6.4 — Couple the preview to pill drag** *(depends on 6.2)*. Pill emits `klarvo://bar-moved`;
  preview re-centers; only pill position persisted. Covers FR9, NFR4.
- **6.5 — Pill fully static + cleanup** *(depends on 6.2 + 6.4)*. Delete the dead grow code; pill one
  fixed size, region once; clipboard-done re-laid-out to fit the fixed width; remove
  `set_bar_shape("panel")` + screenCap remnants; reconcile/park 5-7. Covers FR1, NFR2, AR4, AR5.
- **6.6 — Preview-box appearance customization** *(depends on 6.2; parallel with 6.3/6.4)*. User-facing
  Settings to style the preview box so it stays legible on any background: **text** color/brightness/
  font-family (FR11), **box** background color/opacity/blur (FR12), **border** color/brightness/
  thickness/radius (FR13). Mostly pure-CSS + new camelCase config keys (NFR5) read reactively in the
  separate preview window (Trap #3, separate-window reactivity); **corner radius is the one Rust-coupled
  value** (R11: `set_preview_shape` region radius must track CSS radius). Distinct risk class from 6.3
  (which is geometry/`k`-coupled); split out deliberately. Covers FR11, FR12, FR13, NFR5, NFR6.

**Dependency flow:** 6.1 → 6.2; **6.3, 6.4 and 6.6 parallel after 6.2**; **6.5 after 6.2 + 6.4**. No
story depends on a later story. After **6.2** the geometry race class (R3/R4/R5/R6/R10/R11) is already
gone. (6.6 was added 2026-06-06 — preview-box appearance, split from 6.3's font-size axis by risk class.)

**Working decisions (defaults from the spec, confirmable at story time):** pill stays at width 200
with the clipboard-done state re-laid-out to fit (not widening to 220); the preview window is created
hidden at startup; drag-follow is live via throttled `bar-moved` (fallback: snap on drag-end).

## Epic 6: Floating Bar Re-Architecture

The pill behaves as today but is fully static; the live preview is a separate transparent window
above the pill that grows upward and caps+scrolls — with the geometry set once per recording so the
race class cannot exist.

### Story 6.1: Scaffold the standalone "preview" window

As a developer re-architecting the bar,
I want a standalone transparent, click-through `"preview"` window created, routed, and recoverable,
So that live preview can render in its own window fully decoupled from the pill.

**Acceptance Criteria:**

**Given** the app starts on Windows
**When** the `setup` closure runs
**Then** a second WebView window labeled `"preview"` is created **hidden**, transparent, decorationless,
always-on-top, skip-taskbar, no-shadow, and **click-through** (ignores cursor events), mirroring the
bar's creation flags via a new `create_preview_window` helper.

**Given** both the `"bar"` and `"preview"` windows exist
**When** `main.tsx` evaluates the current window label
**Then** `"preview"` routes to a new `PreviewPanel` component (alongside `"main"` → App, `"bar"` →
FloatingBar); `PreviewPanel` renders only `RESET_CSS` for now (no content).

**Given** the preview window vanished or is unresponsive
**When** `ensure_preview_window` is invoked
**Then** it probes `get_webview_window("preview").is_visible()` and recreates via
`create_preview_window` when missing/unresponsive (mirrors `ensure_bar_window`), returning `true` when
recreated; desktop-only, registered in the `invoke_handler`.

**Given** the scaffolded window
**When** it is shown at a fixed test size/position and then hidden
**Then** it shows, positions, and hides with **no white-line / shape artifact** (verified in the
Windows smoke).

**DoD:** Real Windows release build + manual smoke (show/position/hide, no artifact).
`cargo check --target x86_64-pc-windows-gnu` green. Linux `cargo test` for compile + command
registration. Walk `docs/surface-smoke-checklist.md` (window-geometry/region, event push-wiring).

### Story 6.2: Move live preview into the window (CSS-grow, scale geometry)

As a user dictating with preview enabled,
I want the live text to appear in a window above the pill that grows upward and caps + scrolls,
So that I can read along while the pill never moves or resizes.

**Acceptance Criteria:**

**Given** preview is enabled and a recording is active in Toggle/Hold
**When** the **first** `klarvo://live-preview-chunk` of the cycle arrives
**Then** `PreviewPanel` computes `previewGeometry(widthPreset, "small")` with the screen clamp (AR3),
sets the window **size + rounded-rect region (radius 14) + position** (centered over the pill, bottom
edge `GAP` above the pill top) **once**, and shows the window.

**Given** preview chunks continue to arrive
**When** each chunk is appended
**Then** the dark card grows **upward via CSS** within the static window (bottom-aligned) and **no**
`setSize`/`setPosition`/region call is issued per chunk (NFR1).
**And** inversion: re-introducing a per-chunk window resize brings back the cold-expansion / pre-measure
clip (R3/R4) — proving the static-window invariant is load-bearing.

**Given** the accumulated text exceeds the window's max height
**When** further chunks arrive
**Then** the **inner** text area scrolls to the newest line with a **top-fade**; the window itself does
not grow further.

**Given** a chunk arrives after the recording ended (stale/out-of-cycle)
**When** the listener fires
**Then** the `isRecordingRef` guard (R1) drops it — no text bleeds into the next cycle.
**And** inversion: removing the guard lets a stale post-done chunk repopulate the panel.

**Given** the recording ends (`done`/`idle`/`error`)
**When** `klarvo://state-changed` fires
**Then** `PreviewPanel` clears the accumulated text and hides the preview window.

**Given** the width preset is Compact/Comfortable/Wide
**When** the preview opens
**Then** only the **preview width** changes (260/320/400 at the small font); the **pill is unaffected**
(FR5).

**Given** the pill is in any active state
**When** preview is active
**Then** the pill window is **not resized** — the old in-pill grow path is disabled in this story, so
exactly one preview surface exists (the new window). (Dead pill code is deleted in 6.5.)

**DoD:** Real Windows release build + manual smoke — text grows upward, caps + scrolls, centered above
a **non-resizing** pill, no top-clip on the first chunk. `tsc`/`vite` + `cargo check` win-target green.
Walk `docs/surface-smoke-checklist.md` (separate-window reactivity, geometry/region clip, event wiring).

### Story 6.3: Font size axis config settings picker k scaling

**Key `6-3` — Font-size axis (preview_font_size + Settings picker + k-scaling)**

As a user,
I want to choose among three preview font sizes in Settings,
So that the preview is readable at my preferred size with the whole box scaling proportionally.

**Acceptance Criteria:**

**Given** a fresh config (field absent)
**When** the schema is loaded
**Then** `preview_font_size` reads its serde default `"small"` (camelCase key `previewFontSize`) with
**no** migration write; round-trip + missing-field + camelCase tests assert this.

**Given** the Settings live-preview section
**When** the user picks Small / Medium / Large
**Then** the value persists via `save_config_locked` (ADR-0015) and the picker reflects the saved value.

**Given** a font size is chosen
**When** the preview next opens
**Then** font (11/13/15), width, and height limit all scale by `k = fontPx / 11` (1.0 / 1.18 / 1.36);
`PreviewPanel` reads the setting **reactively** (re-read on open / backend event — separate-window
rule), never frozen at app-start.
**And** inversion: hard-coding `k = 1` leaves Medium/Large visually identical to Small → RED.

**DoD:** Windows settings-smoke (pick a size → camelCase key in `config.json` → preview reflects it on
next open) + config tests. `tsc`/`vite` + `cargo check` win-target green.

### Story 6.4: Couple the preview to pill drag

As a user,
I want the preview to stay centered above the pill while I drag it,
So that the two always move together.

**Acceptance Criteria:**

**Given** a recording with the preview window open
**When** the user drags the pill
**Then** the pill emits `klarvo://bar-moved` `{x, y}` (throttled to an animation frame + once on
drag-end; colon form, NFR4).

**Given** the preview window is open
**When** `klarvo://bar-moved` fires
**Then** `PreviewPanel` re-centers via `setPosition` **only** (no resize), using the new pill anchor +
the screen clamp.

**Given** the drag ends
**When** the final position is computed
**Then** **only the pill** position is persisted (`save_bar_position`); the preview position is always
derived from the pill anchor.

**Given** preview is closed (not recording)
**When** the pill is dragged
**Then** no preview repositioning occurs (no preview window to move).

**DoD:** Real Windows release build + manual smoke — drag during recording keeps the preview centered
above the pill, no teleport, no resize. `tsc`/`vite` + `cargo check` win-target green.

### Story 6.5: Pill fully static and cleanup

**Key `6-5` — Pill fully static + cleanup**

As a developer,
I want the dead grow logic removed and the pill made truly static,
So that the codebase reflects the new foundation with no resize paths left and 5-7 is reconciled.

**Acceptance Criteria:**

**Given** `FloatingBar.tsx` after 6.2/6.4
**When** the dead grow code is removed
**Then** `livePreview`, `panelHeight`, `panelScrolls`, `geomTick`, `measureRef`, `previewPanelRef`,
`prevIsPanelOpenRef`, the measure `useLayoutEffect`, the per-chunk grow/resize effect, the panel render
block, and `setBarShape("panel")` are all deleted (AR4); the pill `show` effect is **show/hide +
position only** (no resize).

**Given** the pill window
**When** it is created
**Then** it is one fixed size (PILL_WIDTH × PILL_HEIGHT) with its pill region set **once** at creation;
it is **never resized** in any state (recording/processing/done/clipboard/error) (FR1).

**Given** the clipboard-done state ("In Clipboard")
**When** it is shown
**Then** it fits within the fixed pill width (200) — re-laid-out (icon + compact text) rather than
widening the window to 220 (FR1).

**Given** the cleanup is complete
**When** the build runs
**Then** the unused `set_bar_shape` "panel"/shape path and the per-preset `screenCap` remnants are
removed (AR5); lib tests green; `clippy` no-new; no dead code.

**Given** Story 5-7 (parked, `review`)
**When** Epic 6 lands
**Then** 5-7 is reconciled: its R1 stale-chunk guard and R2 backpressure live on in the new design;
5-7 is marked superseded/parked in `sprint-status.yaml` (it no longer needs the single-window smoke).

**DoD:** Real Windows release build + manual smoke — the pill never resizes in any state; preview still
works end-to-end. Lib tests green + `clippy` clean on touched files. `tsc`/`vite` + `cargo check`
win-target green.

---

_Epic 6 planning artifact. Codeable contract: `docs/bar-redesign-spec.md`. Ist-Zustand:
`docs/deep-dive-bar-subsystem.md`. Per-story full context via `bmad-create-story` per session._

### Story 6.6: Preview box appearance customization

**Key `6-6` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

Preview-box appearance: themes, visual pickers and a live in-panel preview (redesign). Track L3-feature, builds on 6.2.

**Source:** `_bmad-output/implementation-artifacts/6-6-preview-box-appearance-customization.md`


# Part 4: Cross-Platform Config-Contract Parity (7)

*Former file: `epics-cross-platform-parity.md`. Its frontmatter is kept below.*

```yaml
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics"]
status: in-progress
inputDocuments:
  - docs/cross-platform-drift-audit.md  # verified A/B drift audit — the requirements source
  - docs/dictation-quality-android-vs-desktop-2026-06-12.md  # STT evidence run — drove the 2026-06-12 re-scope
  - docs/adr/0016-android-path-parity-strategy.md  # Amendment 1 gates the per-row stories; Amendment 2 the STT consolidation
  - docs/adr/0017-shared-core-stt-path.md  # Hard Rule: shared STT/guard logic only in Rust, over JNI
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-10.md  # original correct-course routing
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-12.md  # STT re-scope routing (supersedes the STT rows)
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-09-10.md  # RE-CUT: 7.5 dissolved, 7.7 -> 7.8, 7.6 amended
  - _bmad-output/project-context.md
trackType: brownfield
featureEpic: 7
note: >
  Separate planning artifact by design (mirrors Epics 5/6). epics.md is the CLOSED
  robustness-remediation breakdown (Epics 1-4). This is the Cross-Platform Config-Contract
  Parity epic (Epic 7), routed via bmad-correct-course (sprint-change-proposal-2026-06-10.md)
  off the verified drift audit, gated by ADR-0016 Amendment 1. RE-SCOPED 2026-06-12
  (sprint-change-proposal-2026-06-12.md, ADR-0017): the STT-class rows (old 7.3 + 7.4 +
  silence-filter part of 7.2 + H14) collapse into one Rust-core consolidation story (new 7.3);
  old 7.4 removed; 7.6 shrinks to the M12 decision. No PRD/Architecture/UX — the requirements
  sources are docs/cross-platform-drift-audit.md (row IDs C/H/M/L + recall-sweep) and the
  2026-06-12 evidence run. Shares the sprint-status.yaml ledger. C1 (license) is already
  fixed (22553bc) and OUT of scope.
```

## Epic 7: Cross-Platform Config-Contract Parity



> **⏸ PARKED 2026-06-13** (`sprint-change-proposal-2026-06-13.md`): deferred zugunsten des Studio-Dark-Visual-Overhauls (Epics 8/9). 7-3 bleibt done. Beim Wiederaufnehmen: 7-1 (unabhängig) zuerst, **7-7 als Capstone ZULETZT** (lockt 7.1–7.6). Re-Eval: nach Epic 9.

> **RE-CUT 2026-09-10** (`sprint-change-proposal-2026-09-10.md`): resumed 2026-08-10 (7-1), 7-2 done
> 2026-09-10. A relevance audit of 7.5/7.6/7.7 against `v1-ship` (`3d7de0d`) found the June cut stale.
> **7.5 dissolved** (only M9 survives, in 7.8). **7.7 superseded by 7.8** (net close-out; fixture format +
> both harnesses already exist from 7-1/7-2/7-3; no CI exists — gates are `scripts/android-smoke.sh`
> + `cargo test --lib`). **7.6 amended** (still the M12 decision; fallback lock delivered by 7.8).
> Sequencing: **7.8 next**, independent of 7.6. Epic may close with 7.6 parked if M12 is undecided.

### Overview

**Brownfield drift remediation.** ADR-0016 Amendment 1 (2026-06-10) moves the parity line
*chirurgisch*: only **core-output-determinism drift** and **settable-but-silently-dead config keys**
cross from "accepted asymmetry" into stories. Pure feature-ports stay accepted → `docs/backlog.md`.

**RE-SCOPE 2026-06-12 (ADR-0017 + sprint-change-proposal-2026-06-12.md):** the STT evidence run
proved both platforms hit the identical Groq engine through **two divergent request/guard
implementations**. The STT-class rows are therefore no longer per-row Kotlin ports: Story 7.3 is now
the **Rust-core STT consolidation over JNI** (absorbs old 7.3 + 7.4 + the silence-filter part of 7.2
+ H14, deletes the Kotlin twins) plus new hallucination hardening. The Hard Rule (ADR-0017) is
STT-only; chunking (7.1), the VAD gate (7.2) and LLM routing (7.5) stay per-row Kotlin fixes.

Each remaining per-row story implements a cluster of audit rows (IDs from
`docs/cross-platform-drift-audit.md`). Unless noted, the fix is on the **Android** (Kotlin) side;
Story 7.3 changes **both** sides (shared Rust core).

**Epic DoD:** every HIGH/CRITICAL row fixed here carries a golden-vector regression that runs against
**both** Rust and Kotlin (built incrementally per story, consolidated by Story 7.7). Test-Architect
`*risk`/`*design` on the core-output stories (7.1, 7.2, 7.3).

**Sequencing:** 7.3 first (highest-value core change; 7.1 is independent and can run in parallel).
7.2/7.5/7.6 independent. 7.7 (the structural parity net) runs **last** so it locks the just-fixed
behavior + the dead-config landmines.

---

### Story 7.1: Android chunking parity (core output)

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

### Story 7.2: Android live auto stop vad gate parity

**Key `7-2` — Android live auto-stop VAD-gate parity *(narrowed 2026-06-12)***

**Rows:** H1, H17, M2, M3, M4, L1. *(Recall #1 + M1-read moved to 7.3 — the pre-STT silence
filter is consolidated there; no second source left to desync against.)*

As a klarvo user on Android,
I want live auto-stop driven by my configured thresholds, like on Desktop,
So that recording doesn't cut me off mid-sentence and Expert-mode tuning actually takes effect.

**Acceptance Criteria (outcomes):**
- **H1** — the live auto-stop energy gate derives `energy_floor` from config `silenceThreshold`
  (default 0.005), not a hardcoded `0.02f` (`KAR:58`). Defaults are no longer 4× stricter than Desktop.
- **H17** — silence→stop honors a 200ms floor (`hangover_ms.max(200)`, `audio/mod.rs:1074`); not a
  ~32ms single-frame floor (`KAR:77-78`).
- **M2** — `minRecordingMs` is config-driven (not hardcoded `500L`).
- **M3** — an 85 Hz highpass is applied before VAD (parity with `vad/mod.rs:73`).
- **M4** — RMS is computed at parity scale/precision so numeric thresholds are comparable (compounds H1).
- **L1** — VAD fps matches 31.25 (ceil), not integer 31.

> **Note:** This stays a **Kotlin** fix by design (ADR-0017 Scope): the VAD gate is a realtime frame
> stream — moving it over JNI is a large lift with speech-truncation risk, deliberately out of the
> consolidation. M1's *whisper-mode threshold swap* depends on whisper-mode (C2, **backlog**). **M5**
> (full 4-state VAD state machine) stays accepted → backlog (heavy algorithm rewrite, DIV-14).

**Golden-vectors:** energy-floor + stop-latency at default config and at one tuned config.
**Test-Architect:** `*risk` + `*design` (can truncate user speech).

---

### Story 7.3: Shared core stt request and guard path via jni

**Key `7-3` — Shared-core STT request + guard path via JNI *(re-scoped 2026-06-12 — consolidation centerpiece)***

**Rows:** H3, Recall #5, H9, H10, L3 (old 7.3) · H6, H7 (old 7.4) · Recall #1, M1-read (silence
filter, from old 7.2) · H14 (from old 7.6) · **+ new hallucination hardening** (not in the row audit;
source: `docs/dictation-quality-android-vs-desktop-2026-06-12.md`). *(Governed by ADR-0017; overrides
ADR-0016 DIV-08 + DIV-11 via consolidation instead of porting.)*

As a klarvo user,
I want both platforms to send the **same** STT request and apply the **same** hallucination/silence guards,
So that dictation quality is identical and stockphrase ghosts never reach my text — on either device.

**Acceptance Criteria (outcomes — full Given/When/Then in create-story):**
- **Single Rust STT path:** Android transcription routes through the Rust `GroqWhisper`/`WhisperStt`
  path over `stt/jni_bridge.rs`; `KlarvoApi.transcribe` + `buildMultipartBody` are **deleted**. This
  subsumes the old 7.3 rows by construction: prompt conditioning (language hint + dictionary +
  `customPrompt`, **H3**/**Recall #5**), `sttModel` read (**H9**), `localWhisperModel` read (**H10**),
  STT temperature parity (**L3**).
- **Shared guards:** `is_prompt_echo()` (**H6**) and `strip_prompt_fragments()` (**H7**) become the
  single Rust guards both platforms inherit; `HallucinationFilter.kt` is **deleted**.
- **H14 — explicit regression guard:** the Rust filter (`stt/hallucination.rs:160-164`) still
  substring-matches single-word entries, while the Kotlin twin already has the whole-word fix
  (ROB-03, `HallucinationFilter.kt:100-109`). The shared Rust filter MUST adopt whole-word matching
  **in this story** — otherwise deleting the Kotlin twin regresses Android on an already-fixed behavior.
- **Shared silence pre-filter:** one Rust pre-STT silence filter consumed by both platforms;
  `SilencePreFilter.kt` is **deleted**; the `0.02f`/`0.005f` self-desync (**Recall #1**) and the
  `silenceThreshold` config-read (**M1-read**) are resolved by construction.
- **Hallucination hardening (new):**
  - Blocklist the `Groß- und Kl(inge|ingel|einschreibung)[, Satzzeichen und Interpunktion]` stockphrase
    family, `Untertitelung des ZDF`, `amara.org`/credit/subtitle lines, `[Musik]`, subscribe/thank-you
    sign-offs — and run the **trailing-ghost** match **regardless of clip length** (kill the ≤8-word
    gate, `hallucination.rs:155-158`, that lets long-clip trailing ghosts through, ~1.1% of long clips).
  - Switch Groq to `response_format=verbose_json` and **drop segments** by `no_speech_prob` /
    `compression_ratio` / `avg_logprob` thresholds.
  - **Cleanup-no-invent:** LLM cleanup must not manufacture the full stockphrase from a recognizable
    ghost (`Klinge` → `Kleinschreibung`, observed desktop ids 2708/2891/2777); enforce by stripping the
    stockphrase family *after* cleanup and/or constraining the cleanup prompt.
- **Verifiability split (named decision, per Verifikations-Symmetrie):** stockphrase blocklist + paste
  path → **live on-device smoke** (Andi-reproducible: short/silent clips reliably trigger the ghosts).
  Confidence-drop (`verbose_json`) → **golden-vector fixtures** (segment metadata → expected drop);
  the human gate for this sub-part is deliberately downgraded to fixture-verified.

**Golden-vectors:** ZDF/`Kleinschreibung` ghosts on short clips → stripped; trailing ghost on a long
clip → stripped; low-confidence segment → dropped; "Standard"/"Milliarde" in a short utterance → NOT
discarded (H14, both platforms); prompt-body assembly given dictionary + language hint + customPrompt.
**Test-Architect:** `*risk` + `*design` (core output path, primary use case, crosses the JNI boundary).

---

### Withdrawn 7.5: Android LLM-routing contract hygiene — ⛔ SUPERSEDED 2026-09-10

**No key in sprint-status.yaml — superseded 2026-09-10 by 7.8.**

> Dissolved by `sprint-change-proposal-2026-09-10.md`. **M9 → Story 7.8.** M13 → backlog "Desktop
> Advanced settings + AutoSend: wire or remove" (premise false: no AutoSend toggle exists on either
> platform). M10/M11/L5 → dropped, unreachable via any UI (Desktop trims keys, writes the style as an
> enum, Rust core fills `deviceId`). M16 → dropped, no observed failure. Original text below for
> traceability only.

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

### Story 7.6: M12 open product decision dictionary in chat style

**Key `7-6` — M12 open product decision — dictionary in Chat style *(shrunk 2026-06-12)***

**Rows:** M12 (open decision). *(H14 moved to 7.3 — the shared Rust filter adopts the whole-word
match there, so both platforms get it from one fix.)*

As a klarvo user,
I want dictionary handling in Chat style to behave the same on both platforms,
So that my term-biasing is predictable regardless of device.

**Acceptance Criteria (outcomes):**
- **M12** — **resolve the open product decision first** (dictionary-in-Chat-style: should Chat include the
  dictionary like Android, or omit it like Desktop?), then make both platforms agree. Tracked as
  `OPEN-DECISION` in `docs/backlog.md`. If unresolved at story time, lock current behavior as a golden-vector
  and defer the code change.

**Golden-vectors:** Chat-style request assembly with a dictionary present → the decided canonical behavior.

> **Amendment 2026-09-10.** Still valid: Desktop's Chat arm omits `{dict_section}`
> (`llm/mod.rs:228-254`), Android appends the dictionary for every style. Two changes:
> (1) The fallback clause ("lock current behaviour as a golden-vector") is **delivered by Story 7.8**,
> so 7.6 is now purely: Andi decides M12 → both platforms agree → the 7.8 vector is flipped to the
> decided behaviour. (2) Andi's wider idea — one dictionary across devices instead of one
> `dictionaryTerms` per device — is homed in `docs/backlog.md` as a story candidate. It is
> **independent** of M12 (a synced list still needs the style decision) and is NOT part of 7.6.
> If M12 is still undecided when 7.8 closes, Epic 7 may close with 7.6 parked.
>
> **Decision 2026-09-10 (Andi):** M12 resolved — **Chat includes the dictionary**; Android is canon, Desktop
> adds `{dict_section}` to the Chat arm and the `M12-DICT-SCOPE-CHAT` vector flips to agree. Record and
> rationale: `docs/backlog.md` "DECIDED 2026-09-10 — M12". Implementation not started; quick-dev sized.

---

### Withdrawn 7.7: Golden-Vector parity net (C1-proper) + dead-config lock — ⛔ SUPERSEDED 2026-09-10 by 7.8

**No key in sprint-status.yaml — superseded 2026-09-10 by 7.8.**

> The June plan assumed a CI and an empty fixture directory. Neither holds: no CI exists, and 7-1/7-2/7-3
> delivered the shared fixture format and both harnesses. The dead-config-cluster lock is dropped:
> Desktop shows those keys in the Advanced panel and ignores them, with three disagreeing default
> sets — freezing that would cement a lying UI. → backlog decision "wire or remove".
> `scripts/dictation-quality-audit.py` → backlog tooling. Original text below for traceability only.

**Rows:** the structural net; locks every HIGH/CRITICAL Epic-7 fix + the dead-config-both-sides cluster.

As the klarvo maintainer,
I want shared golden-vector fixtures (config + input → expected behavior) run against BOTH Rust and Kotlin in CI,
So that any future re-divergence trips the net instead of shipping silently.

**Acceptance Criteria (outcomes):**
- A shared fixture format (config + input → expected behavior/output) consumed by both a Rust test target and
  a Kotlin test target.
- Every HIGH/CRITICAL row fixed in 7.1–7.6 has a corresponding golden-vector. For the consolidated STT path
  (7.3) the net pins the **shared contract** (ADR-0017 boundary): a vector asserts Android's transcription
  enters via the JNI bridge (no Kotlin STT request path exists to drift).
- **Quality-layer sibling (tooling, → backlog item):** commit the dictation-quality marker detectors from the
  2026-06-12 evidence run as `scripts/dictation-quality-audit.py`; run manually on a cadence (needs the phone
  over adb/Tailscale — no cloud cron). The parity net pins config-contract equality; this pins output-quality
  drift.
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

### Story 7.8: Parity net close out and twin hygiene

**Key `7-8` — Parity-net close-out + twin hygiene *(new 2026-09-10, supersedes 7.5 + 7.7)***

**Rows:** M9 · 7-3 AC9 guard · twin-constant lock · 7-2 residuals · `android-smoke.sh` traps.

As the klarvo maintainer,
I want the existing golden-vector net closed around the last unguarded twins and the accepted
7-2 residuals cleared,
So that a future re-divergence trips a test on the gates that actually exist, and the net is not
carrying known-false claims.

**Acceptance Criteria (outcomes — full Given/When/Then in create-story):**
- **M9** — both DeepSeek call sites in `KlarvoApi.kt` use `https://api.deepseek.com/v1/chat/completions`
  (parity with `llm/mod.rs:719`). A JVM test pins the URL constant.
- **ADR-0017 boundary guard** — a mechanical check fails if a Kotlin STT request or guard path
  re-appears (multipart transcription body, `HallucinationFilter`, `SilencePreFilter`, any
  `audio/transcriptions` string outside the JNI bridge). Runs inside an existing gate (JVM test or an
  `android-smoke.sh` step), not in a new pipeline.
- **Twin-constant lock** — one shared fixture pins the Rust↔Kotlin twin constants: LLM temperature
  0.3, `max_tokens` 2048, chunk threshold 400, chunk target 350, chunk join `\n`. Both a Rust test and
  a Kotlin test read the same file. Scope guard: this locks **twin parity**, NOT the dead-config
  cluster (that is a backlog decision).
- **M12 current-state vector** — a fixture records today's divergence (Desktop Chat omits the
  dictionary, Android includes it) as the documented state, so 7.6 flips one vector when Andi decides.
- **7-2 residuals** — the 8 round-3 findings in the 7-2 story file are fixed: test-claim accuracy,
  vacuous-pass `optDouble` defaults in `VadGateGoldenVectorsTest`, the false `HighpassFilterTest`
  claim, the stale `KlarvoAudioRecorder` KDoc (strike the clause; do NOT add a VAD short-circuit).
- **`android-smoke.sh` traps** — (a) the test/source copy prunes stale files (`rsync --delete` or
  clear-then-copy); (b) on an `emulator-*` serial the install uses `--abi arm64-v8a -r -g` so the
  Rust `.so` is present. Fix directions in `docs/backlog.md` "Story 7-2 residuals".
- **Gates replace CI** — the story documents the two commands that run the whole net
  (`scripts/android-smoke.sh` JVM gate; `cargo test --lib` in `src-tauri/`) in a
  `test-fixtures/README.md`, one paragraph.
- **Inversion check (mandatory, at writing time)** — for the boundary guard, the twin-constant lock
  and at least one repaired 7-2 assertion, deliberately re-introduce the drift and show RED.

**Out of scope:** the dead-config cluster (backlog decision), `dictation-quality-audit.py`
(backlog tooling), any change to STT, VAD, JNI, config schema or UI.

**DoD:** JVM gate + `cargo test --lib` green with the inversion evidence recorded; emulator smoke via
`scripts/android-smoke.sh` proves the install fix (fresh APK, one JNI call succeeds); **Andi's GATE-4:**
one dictation on the Xiaomi with DeepSeek cleanup returns cleaned text (proves M9 on the real path).

---

### Story 7.9: Desktop advanced settings dead keys and model ids

**Key `7-9` — Desktop Advanced settings + AutoSend — remove dead keys, wire 4 model IDs *(new 2026-09-11)***

**Source:** `docs/backlog.md` "DECIDED 2026-09-11 — Desktop Advanced settings + AutoSend" (Andi); audit in
`sprint-change-proposal-2026-09-10.md`. **Rows:** M13 · dead-config cluster.

As a BYOK power user,
I want the Advanced settings panel to show only keys that act at runtime, and the four cleanup model IDs
to be a real override on both platforms,
So that no setting silently does nothing, and a retired provider model ID does not need an app update.

**Acceptance Criteria (outcomes — full Given/When/Then in create-story):**
- **Remove the dead keys** (13 named below; the backlog record counts 14 — create-story pins the exact set from code) from the UI (`src/components/AdvancedSettingsPanel.tsx`), the Rust config struct
  and the Kotlin twin: `sttTemperature`, `llmTemperature`, `llmMaxTokens`, `chunkThreshold`,
  `chunkTargetSize`, `autoPaste`, `autoCapitalize`, `bubbleTapAutoSend`, `bubbleLongPressAutoSend`,
  `llmSystemPromptPolished`, `llmSystemPromptVerbatim`, `llmSystemPromptChat`, `llmCommandModePrompt`.
  Runtime behaviour stays: Whisper temperature fixed 0.0, cleanup 0.3 / 2048, chunking 400 / 350 (the
  Rust↔Kotlin twin constants locked in 7.8). "Custom Instructions" (`custom_prompt`) stays.
- **Untouched (they work):** `sttPromptDe/En/Auto`, `silenceThreshold`, `minRecordingMs`,
  `whisperModeThreshold`, `whisperModeGain`.
- **Old configs still load** — a `config.json` carrying the removed keys loads without error on both
  platforms (serde drops unknown fields; Kotlin parsing ignores them). A test pins this.
- **Wire 4 keys, both platforms:** `llmModelDeepseek`, `llmModelOpenai`, `llmModelAnthropic`,
  `llmModelGroq` replace the hard-coded model IDs in the Rust cleanup client (`llm/mod.rs`) and the
  Kotlin twin (`KlarvoApi.kt`); an empty value falls back to today's hard-coded default. The parity
  fixture from 7.8 pins the defaults for both sides.
- **Inversion check (mandatory, at writing time)** — re-introduce a hard-coded ID on one side and show
  the parity test RED.

**Out of scope:** the M12 dictionary decision (7.6), the style switches (epic candidate B), any change to
STT, VAD, JNI or the top-level `custom_prompt`.

**DoD:** `cargo test --lib` (`src-tauri/`) + JVM gate (`scripts/android-smoke.sh`) green with the inversion
evidence recorded; Desktop proxy smoke (Chromium preview) shows the Advanced panel with only the live
keys; **Andi's GATE-4:** Windows release build, Advanced panel shows only live keys, a changed DeepSeek
model ID appears in the request log (`[fe:…]` / Klarvo.log).

---

### Story 7.10: Cleanup failure raw text clipboard only

**Key `7-10` — Cleanup failure → raw text clipboard-only, no paste, no auto-send *(new 2026-09-14)***

**Source:** `docs/backlog.md` "DECIDED 2026-09-13 — Cleanup-Fehler: Rohtext NUR in die Zwischenablage" (Andi,
from the 7.9 GATE-4 finding 3a). **Rows:** degrade path (Epic 12 principle "never silent loss").

As a dictating user whose cleanup call failed (wrong model ID, provider down, key missing),
I want the raw transcript to land only in the clipboard — not pasted into the active window and never
auto-sent —
So that filler-laden raw text is never inserted or submitted behind my back, while Ctrl+V still gives me
the text in one keystroke.

**Acceptance Criteria (outcomes — full Given/When/Then in create-story):**
- **Desktop clipboard-only branch on `llm_error`:** in `src-tauri/src/pipeline.rs`, when
  `ProcessOutcome::Produced { llm_error: true }` reaches the paste step, the text is copied to the clipboard
  only (`PasteResult::ClipboardOnly`, today reached only on missing focus). No Ctrl+V. Because `send_enter`
  is bound to `PasteResult::Pasted`, auto-send (Insert+Send) is skipped without a second switch.
- **Android twin:** `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` Step 4 — on an LLM
  failure the service calls `copyToClipboard` only; `pasteIntoFocusedField` and the auto-send path are
  skipped. Toast/bubble wording mirrors the desktop pill.
- **The warning survives the done event (design constraint from 7-9 finding 3a):** after the degrade
  warning, no separate Done/DoneClipboard event may overwrite it. Either ONE event carries warning text +
  "in the clipboard", or DoneClipboard carries the warning text. Wording: "Cleanup failed — raw text in the
  clipboard, Ctrl+V to paste"; a model-not-found failure keeps naming the model ID (7-9 D2 stays).
- **No new setting, no new UI:** this is the new default. Overwriting the clipboard is accepted (Andi).
- **History unchanged:** the entry is written as today with `raw_text` (Epic 12: never silent loss; the
  failed-entries inbox is a separate backlog candidate).
- **Tests:** desktop — the degrade test `test_process_audio_nonretryable_degrades_to_raw` plus a
  paste-level test (`llm_error` → `ClipboardOnly`, no Enter); Kotlin twin test for Step 4.
  **Inversion check (mandatory, at writing time):** re-enable the paste on `llm_error` and show the test RED.

**Out of scope:** the failed-entries inbox (backlog candidate), pill buttons (assessed + parked in 7-9),
any change to STT, VAD, JNI, the fallback ladder or the config schema.

**DoD:** `cargo test --lib` (`src-tauri/`) + JVM gate (`scripts/android-smoke.sh`) green with the inversion
evidence recorded; **Andi's GATE-4:** Windows release build, wrong DeepSeek model ID + Insert+Send on →
nothing lands in the active window, no Enter is sent, the pill shows the warning with the model ID, Ctrl+V
pastes the raw text.

---

### Out of scope (→ `docs/backlog.md`)

Pure feature-ports / accepted asymmetries (ADR-0016 Amendment 1): C2, H4, H5, H8, H11, H12+M7, H15, H16, M5,
M6, M14, M15, L2, L6, L7, Recall #4, and the dead-config cluster's *wire-when-needed* implementations.
C1 (license) is already fixed (`22553bc`) and out of scope.
**Deliberate deferrals of the 2026-06-12 re-scope:** extending the ADR-0017 consolidation to the VAD gate
(realtime JNI lift) and to the chunking/LLM path — the Hard Rule is STT-only for now; 7.7 pins the rest
against silent re-drift.
**Re-cut 2026-09-10 (`sprint-change-proposal-2026-09-10.md`):** M10/M11/L5/M16 dropped (unreachable /
speculative); M13 + the dead-config cluster → backlog OPEN-DECISION "Desktop Advanced settings + AutoSend:
wire or remove"; `dictation-quality-audit.py` → backlog tooling; dictionary-across-devices → backlog
STORY-CANDIDATE.

# Part 5: Visual Overhaul "Studio Dark" (8 + 9)

*Former file: `epics-visual-overhaul.md`. Its frontmatter is kept below.*

```yaml
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories", "step-04-final-validation"]
status: complete
inputDocuments:
  - docs/design/overhaul/SPEC-studio-dark-overhaul.md   # binding tokens/surfaces/states/constraints (the codeable contract)
  - docs/design/overhaul/01-product-brief.md            # values / audience (the "seriousness signal")
  - docs/design/overhaul/02-surfaces.md                 # surface inventory A–E
  - docs/design/overhaul/03-design-tokens-current.md    # current tokens (consolidation baseline)
  - docs/design/overhaul/04-constraints.md              # hard rendering / platform constraints
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-13.md  # epic split 8/9, ADR precursor, DoD
  - _bmad-output/project-context.md                     # code rules (camelCase, ADR-0015/0016, surface DoD)
  - docs/surface-smoke-checklist.md                     # surface-class DoD control
trackType: brownfield-visual-overhaul
featureEpics: [8, 9]
note: >
  Two-epic visual overhaul ("Studio Dark"). Brownfield-v1, NO PRD by design (same as Epics 5/6/7,
  each routed via correct-course off a requirements source). The requirements source is the tracked
  design spec docs/design/overhaul/SPEC-studio-dark-overhaul.md (+ 01..04). The handoff IS the UX
  spec — no separate bmad-create-ux-design run. Epic 8 = Desktop (Tauri/React/Tailwind v4), pure
  visual re-skin. Epic 9 = Android (native Kotlin, View+Canvas today, no Compose): token source
  + bubble interaction redesign (feature work, not re-skin). Epic 9 opens with a load-bearing ADR
  (View+Canvas vs ComposeView). Shares the sprint-status.yaml ledger. Per-story full context via
  bmad-create-story per session. IDs are native to this overhaul track.
```



## Overview

A high-fidelity visual overhaul handoff ("Studio Dark") arrived from the cloud web-design agent and
is the binding direction: final color/type/spacing/radii/elevation/motion tokens, per-surface specs,
and an Android dictation-bubble **interaction** redesign. Two platforms share one design language:
**Windows-Desktop** (Tauri/WebView2 + React + Tailwind v4) and **Android** (native Kotlin; today
View + Canvas `onDraw`, no Compose).

The current-code gap that motivates the work:
- Desktop: `src/styles.css` has a ~69-line `@theme` block but **317 scattered inline hex** across
  components → the "not from one cast" feeling. Token consolidation has wide blast radius.
- Android: the overlay is **classic View + Canvas** (`FloatingBubbleView.kt` ~478 LOC,
  `KlarvoOverlayService.kt` ~1482 LOC) — **no Compose**, and **no Android color/theme file exists
  yet**. The bubble interaction (state sequence, listening panel, long-press popover) is genuinely
  new behavior, not a re-skin.

Two epics, separated by codebase / risk profile / human-test surface (deliberately not bundled):
- **Epic 8 — Desktop Visual Overhaul.** Foundation = token-ladder consolidation, then surface
  re-skins. Pure visual redesign; same function/IA/flows. Surface-class smoke DoD.
- **Epic 9 — Android Visual Overhaul + Bubble Interaction.** Opens with the rendering-tech ADR,
  then the Android token source, then bubble behavior (state sequence + listening panel + reactive
  waveform + long-press popover that remaps long-press from push-to-talk → menu). Hard IME
  constraints. On-device smoke DoD + a bubble state harness (verifiability symmetry).

## Requirements Inventory

Brownfield visual overhaul: requirements are extracted from `docs/design/overhaul/` (SPEC + 01..04).
IDs are native to this track. Categories: **DT** = design-token/system (the shared language),
**UX-DR** = per-surface visual requirements, **FR** = Android bubble interaction (the feature work),
**NFR** = non-functional/fidelity/DoD, **AR** = additional/architecture.

### Design Token & System Requirements (shared language — both platforms)

- **DT1** *(Desktop foundation)* — Consolidate the token ladder. Replace the ~69-line `@theme` block
  **and** the 317 scattered inline hex in `src/` with the Studio-Dark **named** tokens: graphite
  neutral ladder (`bg-deep #0A0B0C`, `bg #0F1112`, `surface #16181A`, `surface-2 #1B1E20`,
  `elevated #232729`, `border #282C2F`, `border-2 #353A3E`); text (`text #ECEEEF`, `muted #A4A9AC`,
  `dim #6F7479`, `faint #4B4F53`); teal (`teal #29C7AC`, `teal-hi #57DDC7`, `teal-lo #1B9C88`,
  `on-teal #05201B`); amber (`amber #E9A24C`, `amber-hi #F4BA72`); semantic (`danger #EE6F63`,
  `success #4FC58A`). Subtle/line variants via `color-mix`/rgba per spec. **No inline hex left for
  any covered role.**
- **DT2** *(Android foundation)* — Create the Android color/theme **source file** (none exists today)
  with the same named ladder as Kotlin `Color(0xFF…)` values per spec.
- **DT3** — Type system: **Geist** (UI, weights 400/500/600/700) + **Geist Mono** (dictation text,
  keys, IDs, timestamps). Scale (px): 11 label (uppercase, +8% tracking) · 12 · 13 · 14 · 16 · 20 ·
  28 · 40; LH 1.1–1.55. Desktop = Geist/Geist Mono **bundled locally** (no runtime CDN fetch —
  BYOK/no-phone-home, NFR6); **Android = bundled font resources**.
- **DT4** — Spacing (4-base: 2 4 6 8 12 16 20 24 32 40 48), radii (xs 6 · sm 8 · md 12 · lg 16 ·
  xl 20 · full), elevation (e1–e3 + pill, each + inset hairline; focus ring
  `0 0 0 3px rgba(41,199,172,.28)`), motion (micro 120 · state 180 · enter 240 spring
  `cubic-bezier(.34,1.56,.64,1)` · panel 320; standard ease `cubic-bezier(.2,0,0,1)`; respect
  `prefers-reduced-motion`). **Android: no native `backdrop-blur`** → solid `#16181A` +
  `Modifier.shadow`; the "glass ring" = a 4dp teal/amber ring, not real blur.
- **DT5** — Color **semantics** enforced everywhere: **teal** = brand / ready / processing / success
  / focus-ring; **amber** = live / listening (recording only — tally light); **danger/red** = stop /
  delete / error only; success (green) used sparingly.

### UX Design Requirements (Desktop surfaces — Epic 8)

- **UX-DR1** *(highest leverage)* — **FloatingBar** re-skin: transparent 200×36 overlay window;
  state sequence `idle` (invisible — pill materializes only on activity) → `recording` (glass pill,
  **amber** tally-light + **teal** waveform, spring-enter) → `transcribing` (teal spinner) → `done`
  (check). backdrop-blur 16px, 72% graphite fill, inset hairline. Stays **200×36** (no inflate),
  draggable, position persists. States designed as a sequence/transition, not static chrome.
- **UX-DR2** — **Settings Home + sub-pages**: color-coded icon badges, status dots, masked **mono**
  API keys, and a consistent high-quality form system — custom **Select / Segmented / Toggle /
  Slider** (native `<select>` removed; Linear/Raycast level). Categories: Recording & Audio · AI &
  Providers · Appearance · Language · Shortcuts · License · Dictionary.
- **UX-DR3** — **Live-Cleanup-Preview** re-skin (desktop-only): transparent panel, live **raw**
  transcript in mono, bottom-anchored; smooth expand/collapse; calm readable raw-vs-clean contrast.
  (Live LLM-*cleanup* stays off by design — quota; only the raw stream is live.)
- **UX-DR4** — **Main-Window / History**: better list density, **mono** timestamps, profile tags in
  **amber**, clear hierarchy, nice empty-states, better search/filter affordances.
- **UX-DR5** — **Onboarding** re-skin: trustworthy, elegant first impression (step indicators,
  illustration/empty); frame BYOK as a feature, not a hurdle. (Surface E.)

- **UX-DR6** — **Action feedback (desktop)**: every user-triggered action on a surface answers the
  click. Copy confirms ("Copied") and self-clears; a destructive action (Delete) is recoverable for a
  short window ("Deleted · Undo") rather than gated behind a prompt. Discovered 2026-08-19 at the 8-5
  smoke; covered by Story 8.8.

### Functional / Interaction Requirements (Android bubble — Epic 9; feature work)

- **FR1** — Bubble **idle**: one form across all states (**no** circle↔square morph) — a **teal-gradient
  squircle** (rounded square, not a circle) with a **dark "K"** centered, plus a subtle teal ring; **responsive
  size** `visual = clamp(36dp, 0.11 × min(screenW,screenH)dp, 44dp)`, touch target `max(visual, 48dp)` via
  transparent padding. *Visual values are anchored on the canon `docs/design/overhaul/source/` (`.ab-bubble.idle`
  in the HTML + `klarvo.css`), NOT transcribed here — read fill/shape/colors there.*
- **FR2** — **recording**: keyboard **collapses**, a **Klarvo-owned panel** rises (grab handle, K +
  **amber** live-dot + reactive waveform from RMS levels + timer + red stop). Live **raw** transcript
  runs multiline in the panel. Footer: "keyboard paused · returns on insert".
- **FR3** — **transcribing**: same panel, teal spinner + "Cleaning…", raw text dimmed.
- **FR4** — **done**: panel collapses, keyboard returns, **cleaned** text is in the field, bubble
  shows a brief check → idle.
- **FR5** — **Short-press = default gesture**, configurable: **Hold / Toggle / Auto-Stop / Auto** —
  the **same 4 modes as the desktop hotkey-mode** (mirror them).
- **FR6** — **Long-press = quick popover** (opens **inward**, never radial): block "default gesture"
  (4 modes) · "mode" (Polished / Verbatim / Chat) · row Target (field / clipboard) + Language
  (DE / EN / Auto) · footer "open settings". Two distinct axes: **gesture** (how triggered) vs
  **mode** (how cleaned). **This remaps long-press from today's push-to-talk → menu.**
- **FR7** — Bubble **anchoring** preserved: draggable, jumps up with the keyboard, edge-snap +
  remembered side.
- **FR8** — **In-app recording state** (`android-05`) re-skinned to the new design language.

### Non-Functional Requirements

- **NFR1** — **High-fidelity**: colors/type/spacing/radii/elevation/motion built pixel-accurate to
  spec; the two deliberate deviations (deeper/cooler graphite ladder; orange `#FFA344` → amber
  `#E9A24C`) are intentional and in-scope.
- **NFR2** — **Behavior-preserving**: Epic 8 is a **pure visual re-skin** (same function / IA /
  flows; no new features). Epic 9's bubble **interaction** change is in-scope, but the information
  architecture does not change.
- **NFR3** — **Desktop DoD** (surface-class): real **Windows release build** + **objective pixel
  metric** + observability loop + walk `docs/surface-smoke-checklist.md`. Human visual gate — **never
  make the user the rendering oracle** (observe-first; isolate the cause before changing app code).
- **NFR4** — **Android DoD**: on-device smoke (`scripts/android-smoke.sh`) **plus** a bubble **state
  harness** so each state (idle/recording/transcribing/done) is reachable for testing
  (verifiability symmetry — the user cannot otherwise produce the states).
- **NFR5** — Motion respects `prefers-reduced-motion` (desktop). Android renders glass effects as
  solid `#16181A` + `Modifier.shadow` (no native backdrop-blur).
- **NFR6** — **BYOK / privacy**: **no telemetry / tracking UI**. Real labels/providers only (Groq,
  DeepSeek, OpenAI, Anthropic, OpenRouter; verbatim/polished/chat) — no Lorem Ipsum.
- **NFR7** — Config keys are **camelCase** via the single-writer atomic path (ADR-0015). Any bubble
  change that reads/writes **shared** config keys (gesture/mode) must be **mirrored Rust ↔ Kotlin**
  (ADR-0016).

### Additional / Architecture Requirements

- **AR1** *(Epic 9 precursor — load-bearing)* — **ADR: Android bubble rendering tech** — extend the
  existing View + Canvas `onDraw` vs introduce a `ComposeView`. Must land **before** the bubble
  behavior stories; decides the implementation substrate for FR1–FR8.
- **AR2** — The **Android token/theme source** is a **new artifact** (none exists), plus bundled
  Geist / Geist Mono font resources (DT2/DT3).
- **AR3** — The **bubble state harness** is an explicit deliverable (NFR4): a dev-only path to drive
  the bubble through all four states on-device without needing live audio/network.
- **AR4** — **Token-consolidation blast radius**: DT1 touches every desktop surface (visual
  regression risk) → the desktop foundation story runs **first**, surfaces depend on it.
- **AR5** — **Hard IME constraints** (Android): (a) **no in-field preview text** from a
  `SYSTEM_ALERT_WINDOW` overlay — live raw text lives on Klarvo's own surface (the listening panel),
  the foreign field is written **only finally** (a11y `ACTION_SET_TEXT` or clipboard+paste);
  (b) **keyboard-collapse during recording** is via the a11y service — **optional, with fallback**
  (keep the keyboard open), not a default for all apps; (c) touch targets **≥ 48dp**; (d) fixed
  **56px nav-bar clearance** (`env(safe-area-inset-bottom)` is unreliable/0 in the Android WebView;
  native Kotlin handles its own insets).
- **AR6** — **FloatingBar transparent-window constraint** (Desktop): `html/body/#root` must stay
  `background: transparent` (else WebView2 paints its default behind the pill). Window size is set in
  **Rust at creation** (the frontend does **not** call `setSize`); if the redesign needs other
  dimensions, give an **explicit** value and keep it small (it is an overlay, not a panel).

### Requirements Coverage Map

| Req | Epic | Note |
|---|---|---|
| DT1 | 8 | Desktop token consolidation (foundation, blast radius) |
| DT3 (desktop), DT4 (desktop), DT5 | 8 | Type / spacing / radii / elevation / motion + semantics |
| UX-DR1 | 8 | FloatingBar re-skin |
| UX-DR2 | 8 | Settings form system + Home/sub-pages |
| UX-DR3 | 8 | Live-Cleanup-Preview re-skin |
| UX-DR4 | 8 | Main-Window / History re-skin |
| UX-DR5 | 8 | Onboarding re-skin (last/optional — see decision D1) |
| AR4, AR6 | 8 | Token blast-radius ordering; transparent-window constraint |
| NFR1, NFR2, NFR3, NFR5 (desktop), NFR6, NFR7 (desktop camelCase) | 8 | Fidelity / behavior-preserving / desktop DoD / reduced-motion / BYOK / config |
| AR1 | 9 | Precursor ADR: bubble rendering tech |
| DT2, DT3 (android), DT4 (android), AR2 | 9 | Android token/theme source + fonts (foundation) |
| DT5 | 9 | Color semantics (shared; applied on Android too) |
| FR1, FR7 | 9 | Bubble idle re-skin + responsive sizing + anchoring |
| FR2, FR3, FR4 | 9 | Bubble state sequence + listening panel + RMS waveform |
| AR3, NFR4 | 9 | Bubble state harness + on-device DoD |
| AR5 | 9 | Hard IME constraints (no in-field preview; optional keyboard-collapse; ≥48dp; nav-bar clearance) |
| FR5 | 9 | Short-press 4 gesture modes (mirror desktop) |
| FR6 | 9 | Long-press popover menu (remap from push-to-talk) |
| FR8 | 9 | In-app recording state re-skin (see decision D2) |
| NFR1, NFR2, NFR5 (android), NFR6, NFR7 (mirroring) | 9 | Fidelity / behavior / no-blur / BYOK / Rust↔Kotlin mirror |

**Scope decisions baked in (confirmable at the approval gate):**
- **D1 — Onboarding (UX-DR5):** *kept* in Epic 8 as the **last, lowest-priority** story (it is a real
  surface in `02-surfaces.md`, but was not among the four surfaces the proposal named). Droppable
  without affecting the other surfaces.
- **D2 — Android in-app recording state (FR8):** *kept* in Epic 9 as a **small re-skin story**,
  separate from the bubble interaction work (named in `02/04`; the bubble is the headline).
- **D3 — Light theme:** **out of scope.** The constraint says dark is the identity, light only ever
  optional — not part of this overhaul.

## Epic List

### Epic 8: Desktop Visual Overhaul ("Studio Dark")

The user sees a coherent, high-fidelity, instrument-grade dark UI across **every** desktop surface —
identical functions and flows, but now "from one cast": the scattered inline-hex look is gone, the
FloatingBar feels premium, and Settings has a real form system. Standalone: builds only on existing
v1 desktop surfaces; enables nothing downstream but retires the visual-inconsistency debt.

**Amendment 2026-08-19 (scope).** Epic 8 is a re-skin — "identical functions and flows" holds for
every story EXCEPT 8.8. Story 8.8 adds interaction behaviour (action feedback + an undo window),
admitted deliberately after Andi's 8-5 smoke found that the re-skinned surfaces answer no clicks.
The re-skin covenant continues to bind 8.1-8.7.

**Covers:** DT1, DT3/DT4 (desktop), DT5, UX-DR1–UX-DR5, AR4, AR6, NFR1, NFR2, NFR3, NFR5 (desktop),
NFR6, NFR7 (desktop).

**Planned story decomposition** (full ACs in Step 3):
- **8.1 — Token & type foundation** *(Wave 1, foundation; AR4)*. Define the Studio-Dark named `@theme`
  token block + type/spacing/radii/elevation/motion primitives, and **bundle Geist / Geist Mono
  locally** (no CDN fetch — NFR6). It does **not** blindly swap all 317 inline hex: a hex becomes a
  *named* token only once its semantic role is decided, and that decision lives with the surface — so
  the hex→token migration is done **per-surface** (8.2–8.6) where it is visually smoke-testable. 8.1
  may do a mechanical pass only for unambiguous global hex. (DT1 [definitions], DT3, DT4, DT5)
- **8.2 — Settings form system + Home/sub-pages** *(after 8.1)*. Custom Select/Segmented/Toggle/Slider
  (native `<select>` removed), color-coded icon badges, status dots, masked mono keys. (UX-DR2)
- **8.3 — FloatingBar re-skin** *(after 8.1)*. State sequence idle→recording→transcribing→done; glass
  pill, amber tally + teal waveform, spring motion; transparent-window constraint preserved; 200×36.
  Surface-class DoD (AR6, NFR3). (UX-DR1)
- **8.4 — Live-Cleanup-Preview re-skin** *(after 8.1)*. Transparent panel, mono raw transcript,
  bottom-anchored, smooth expand/collapse. (UX-DR3)
- **8.5 — Main-Window / History re-skin** *(after 8.1)*. List density, mono timestamps, amber profile
  tags, empty-states, search/filter affordances. (UX-DR4)
- **8.6 — Onboarding re-skin** *(after 8.1; last/optional — D1)*. Trustworthy first impression, step
  indicators, BYOK-as-feature framing. (UX-DR5)

**Dependency flow:** 8.1 first (foundation). 8.2–8.6 each depend only on 8.1 and are otherwise
parallel. Each surface story migrates its own inline hex → named tokens (DT1 [application]); a final
`grep` gate ("no inline hex left for covered roles") rides the **last** surface story to close DT1.
No story depends on a later story. **Risk note:** 8.3 (FloatingBar) is the same transparent-overlay
risk class that burned four build cycles in Epic 6 — it carries the hardest observe-first discipline
(isolate the cause before changing app code; never make the user the rendering oracle, NFR3).

### Epic 9: Android Visual Overhaul + Bubble Interaction

The Android app shares the Studio-Dark design language, **and** the dictation bubble gains the spec'd
behavior: a state sequence with a Klarvo-owned listening panel + reactive waveform, and a long-press
popover menu (the gesture/mode hub). Unlike Epic 8 this is partly real interaction work, not just a
re-skin. Standalone: builds on the existing v1 Android overlay; depends on no other epic.

**Covers:** AR1, AR2, AR3, AR5, DT2, DT3/DT4 (android), DT5, FR1–FR8, NFR1, NFR2, NFR4, NFR5
(android), NFR6, NFR7 (mirroring).

**Planned story decomposition** (full ACs in Step 3):
- **9.1 — ADR: bubble rendering tech** *(precursor, first; AR1)*. Decide extend View+Canvas vs
  introduce ComposeView. **A genuine gate, not a formality** — ComposeView is a far larger substrate
  change than extending Canvas, so the whole Epic-9 effort estimate swings on this decision.
  (Output: an ADR, not UI.)
- **9.2 — Android token/theme source + fonts** *(foundation, after 9.1; AR2)*. Create the Android
  color/theme file (none exists) + bundle Geist/Geist Mono. (DT2, DT3, DT4, DT5)
- **9.3 — Bubble idle re-skin + responsive sizing + anchoring** *(after 9.2)*. Teal K, glass ring
  (4dp), responsive size clamp, ≥48dp touch target, drag/edge-snap/remembered-side. (FR1, FR7, AR5c/d)
- **9.4 — Bubble state harness** *(after 9.3, BEFORE the states; AR3, NFR4)*. A dev-only path to drive
  the bubble through idle/recording/transcribing/done on-device without live audio/network.
  **Sequenced before 9.5 deliberately** — per the verifiability-symmetry rule the states must be
  reachable for test *before* they are built, else 9.5 ships states no one (not even Andi) can
  reproduce to verify.
- **9.5 — Bubble state sequence + listening panel + waveform** *(the big one; after 9.4)*.
  idle→recording→transcribing→done; Klarvo-owned panel (grab handle, amber live-dot, RMS waveform,
  timer, red stop); live **raw** transcript in-panel (IME constraint AR5a); final insert via
  a11y/clipboard. (FR2, FR3, FR4)
- **9.6 — Keyboard-collapse via a11y service** *(after 9.5; optional, own story — AR5b)*. Split out of
  9.5 because it is explicitly optional, per-app-fragile, and carries its own fallback (keep the
  keyboard open). Bundling it into 9.5 would couple a fragile optional behavior to the core state work.
- **9.7 — Short-press gesture modes (mirror desktop)** *(after 9.5)*. Hold/Toggle/Auto-Stop/Auto;
  shared config keys mirrored Rust↔Kotlin. (FR5, NFR7)
- **9.8 — Long-press popover menu** *(after 9.5)*. Inward popover: gesture (4) · mode (Polished/
  Verbatim/Chat) · target + language · settings link; **remaps long-press from push-to-talk → menu**.
  (FR6)
- **9.9 — In-app recording state re-skin** *(after 9.2; small — D2)*. Re-skin `android-05` to the new
  language. (FR8)
- **9.10 — Token codegen: `klarvo.css` → `KlarvoTheme.kt`** *(post-ADR-0019 insertion; sequenced
  BEFORE the 9.5 rebuild; ADR-0019 Decision #2)*. Replace the hand-typed `KlarvoTheme.kt` (the Token-
  Drift surface — proven by the 9.5-F6 AmberLine `.30→.32` copy-error) with a generator that projects
  the canon `--k-*` custom properties into Kotlin token constants, plus a build/CI drift gate so
  hand-edited values can no longer merge. Mechanical, highest leverage, cheap (ADR-0019 §Mitigations
  ordering #1). Foundation for the 9.5 rebuild (the recording state must render against real SSOT
  tokens, not a drifting copy).

**Dependency flow:** 9.1 (gate) → 9.2 (foundation) → 9.3 → **9.4 (harness) → 9.5 (states)** →
{9.6, 9.7, 9.8}; 9.9 after 9.2 (parallel with bubble work). The **harness-before-states** ordering is
load-bearing (verifiability symmetry). **Post-ADR-0019:** 9.10 (token codegen) sequences before the
9.5 *rebuild* (which is re-fashioned against the extended canon — `.ab-bubble.recording`, danger=cancel,
bubble-tap=send). No story depends on a later story.

---

## Epic 8: Desktop Visual Overhaul ("Studio Dark")

The user sees a coherent, high-fidelity dark UI across every desktop surface — identical functions and
flows, now "from one cast". Foundation first, then per-surface re-skins, each gated by a surface-class
smoke.

### Story 8.1: Token and type foundation

**Key `8-1` — Token & type foundation**

As a developer establishing the Studio-Dark design language,
I want the named token block, the type/spacing/radii/elevation/motion primitives, and locally bundled fonts in place,
So that every surface story re-skins against one source of truth instead of scattered hex.

**Acceptance Criteria:**

**Given** `src/styles.css`
**When** the `@theme` block is rewritten
**Then** it defines exactly the Studio-Dark named tokens (graphite ladder bg-deep/bg/surface/surface-2/
elevated/border/border-2; text/muted/dim/faint; teal/teal-hi/teal-lo/on-teal; amber/amber-hi;
danger/success) at the spec hex values
**And** the subtle/line variants (teal/amber/danger bg+line, glass hairline) exist as documented
`color-mix`/rgba utilities.

**Given** the type system
**When** primitives are added
**Then** Geist (400/500/600/700) and Geist Mono are **bundled as local assets** with no Google-Fonts
CDN / no runtime network fetch (NFR6), exposed as font-family tokens
**And** the px scale (11–40), line-heights, and the 11px-label uppercase+tracking rule are available as
utilities.

**Given** spacing / radii / elevation / motion
**When** primitives are added
**Then** the 4-base spacing, radii (xs6…xl20+full), elevation (e1–e3 + pill with inset hairline), the
teal focus ring, and the motion durations/eases are defined as tokens/utilities
**And** `prefers-reduced-motion` is honored by the motion utilities (NFR5).

**Given** the foundation is in place
**When** an existing surface renders unchanged
**Then** the app still builds and runs (tsc/vite green) — 8.1 introduces the **vocabulary**, not the
per-surface re-skin; no surface is migrated here beyond unambiguous global hex.

**And** inversion: a build that fetches Geist from a remote URL at runtime violates NFR6 and must fail
review.

**DoD:** `tsc`/`vite` green; `cargo check --target x86_64-pc-windows-gnu` green; fonts confirmed to
load offline in the Windows build (no font network request).

### Story 8.2: Settings form system home and sub pages

**Key `8-2` — Settings form system + Home & sub-pages**

As a user configuring Klarvo,
I want a consistent, high-quality settings form system across Home and every sub-page,
So that configuration feels instrument-grade instead of stock OS widgets.

**Acceptance Criteria:**

**Given** the Settings Home
**When** it renders
**Then** the categories (Recording & Audio, AI & Providers, Appearance, Language, Shortcuts, License,
Dictionary) appear as rows with color-coded icon badges + status dots, using only named tokens.

**Given** any sub-page with form controls
**When** it renders
**Then** every native `<select>` is replaced by the custom Select; toggles, sliders, and
segmented-controls use the new components; API keys render masked in Geist Mono.

**Given** a control bound to config
**When** the user changes and saves it
**Then** the existing value round-trips correctly through the new control (camelCase key, `save_config_locked`,
ADR-0015) with **no stuck-dirty** state — the new control is wired into the settings resync `useEffect`
(known trap).

**Given** the AI & Providers page
**When** providers/keys are shown
**Then** real provider labels are used (Groq, DeepSeek, OpenAI, Anthropic, OpenRouter) and **no
telemetry/tracking UI** exists (NFR6).

**And** the Settings surfaces carry **zero inline hex** for covered roles (DT1 application), and controls
are keyboard-operable with the teal focus ring.

**DoD:** Windows release build + settings smoke (each control type renders + an existing setting still
round-trips, camelCase key in `config.json`); walk `docs/surface-smoke-checklist.md`; `tsc`/`vite` +
`cargo check` win-target green.

### Story 8.3: FloatingBar re-skin

As a user dictating,
I want the FloatingBar to look premium and read its state clearly while staying tiny and transparent,
So that the most-seen surface signals quality without competing for attention.

**Acceptance Criteria:**

**Given** the bar window
**When** idle
**Then** it is invisible (the pill materializes only on activity) and the window background stays
transparent (`html/body/#root` transparent — AR6).

**Given** recording starts
**When** the pill appears
**Then** it renders the glass pill (backdrop-blur 16px, 72% graphite fill, inset hairline) with an
**amber** tally-light + **teal** waveform, entering via the spring motion; the visible pill stays
**200×36** (no inflate), draggable, position persisted.

**Given** the pipeline progresses
**When** state → transcribing → done
**Then** the pill shows a teal spinner then a check, following the state sequence; amber appears **only**
while recording (DT5).

**Given** the redesign needs no new dimensions
**When** the window is created
**Then** the size is still set in **Rust at creation** (the frontend does not call `setSize`); any dim
change is an explicit small value (AR6).

**And** the FloatingBar carries zero inline hex for covered roles (DT1).

**DoD (surface-class, hardest):** real Windows release build + an **objective pixel metric** (e.g.
measured tally color / fill opacity / blur presence) + observability loop; any rendering artifact is
**isolated and named before** any app-code change — never make the user the rendering oracle (NFR3);
walk `docs/surface-smoke-checklist.md` (transparent window, geometry/region).

### Story 8.4: Live-Cleanup-Preview re-skin

As a user reading along while dictating,
I want the live preview panel to present the raw transcript calmly and legibly,
So that I can orient on what's being captured without distraction.

**Acceptance Criteria:**

**Given** preview is enabled and recording
**When** raw chunks arrive
**Then** the live **RAW** transcript renders in Geist Mono on the transparent, bottom-anchored panel
using named tokens (live LLM cleanup stays off by design — only the raw stream is live).

**Given** the panel opens/closes
**When** expand/collapse fires
**Then** the motion uses the panel duration/ease and is smooth; `prefers-reduced-motion` honored.

**Given** the dark-background legibility issue
**When** text renders
**Then** the raw text is clearly legible (brightness/contrast resolves the known dim-text trap).

**Given** the preview is a **separate window**
**When** any appearance value is config-driven
**Then** it is re-read **reactively** (re-read on open / backend event), never frozen at app-start
(separate-window trap).

**And** zero inline hex for covered roles (DT1).

**DoD:** Windows release build + smoke (preview opens during recording, raw text legible, smooth
collapse); separate-window reactivity checked; `tsc`/`vite` + `cargo check` win-target green.

### Story 8.5: Main-Window / History re-skin

As a user reviewing past dictations,
I want the history list and main window to have clear hierarchy and pleasant density,
So that I can scan and find past dictations easily.

**Acceptance Criteria:**

**Given** the History list
**When** it renders
**Then** list density improves, timestamps render in Geist Mono, profile tags render in **amber**, and
hierarchy uses the type-scale + spacing tokens.

**Given** an empty history or no-match filter
**When** nothing matches
**Then** a designed empty-state renders (not a bare blank).

**Given** search/filter
**When** used
**Then** the affordances are clear and styled with named tokens.

**Given** real content
**When** shown
**Then** real dictation text + real labels are used (no Lorem Ipsum).

**And** zero inline hex for covered roles (DT1).

**DoD:** Windows release build + smoke (list density, empty-state, search/filter); `tsc`/`vite` +
`cargo check` win-target green.

### Story 8.6: Onboarding re skin

**Key `8-6` — Onboarding re-skin (last surface; optional — D1)**

As a first-time user,
I want an elegant, trustworthy onboarding,
So that the seriousness of the tool is clear and BYOK feels like a feature, not a hurdle.

**Acceptance Criteria:**

**Given** first launch
**When** onboarding renders
**Then** steps use the new type/spacing/tokens with clear step indicators, and API-key/provider setup is
framed as a feature (BYOK), using real provider labels.

**Given** the flow runs
**When** the user completes it
**Then** behavior and IA are unchanged (re-skin only) and the end state matches today's.

**Given** this is the **final** surface story
**When** 8.6 is done
**Then** a closing `grep` gate asserts **no inline hex remains for covered roles** across `src/` (DT1
closure).

**And** zero inline hex for covered roles in onboarding itself (DT1).

**DoD:** Windows release build + smoke (walk the onboarding flow); the DT1 closing grep-gate is green;
`tsc`/`vite` + `cargo check` win-target green.


### Story 8.7: Studio dark fidelity pass

**Key `8-7` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

Studio-Dark fidelity pass: the affordances that `epic-8-fidelity-audit.md` lists as not applied (Settings-Home status dots, compact German date format in History, pill-side items). Reactivated 2026-09-16 as a candidate (ADR-0016 Amendment 3). The `.focus-klarvo` always-on ring belongs here.

**Source:** `docs/backlog.md`, entry "8-7 Studio-Dark Fidelity-Pass"

### Story 8.8: Action feedback copy and delete

**Key `8-8` — Action feedback — Copy and Delete (interaction affordance)**

As a user acting on a history entry,
I want the app to answer my click,
So that I know the copy succeeded and a mis-click on Delete does not cost me a dictation.

**Acceptance Criteria:**

**Given** any Copy affordance on a desktop surface
**When** the user clicks it
**Then** the control confirms the action ("Copied") for a short, self-clearing moment, and returns to
its resting label afterwards.

**Given** the clipboard write fails
**When** the user clicks Copy
**Then** the control does NOT claim success, and the failure is visible to the user — not only in the
console.

**Given** a history entry
**When** the user clicks Delete
**Then** the row disappears immediately and an undo affordance ("Deleted · Undo") is offered for a few
seconds; the backend delete runs only after that window expires.

**Given** the undo affordance
**When** the user clicks Undo inside the window
**Then** the entry returns to the list unchanged and no backend delete is issued.

**Given** the covered surfaces
**When** the story is done
**Then** every desktop `clipboard.writeText` call site carries the feedback (7 sites at the time of
writing, in `App.tsx` / `VoiceNotesPanel.tsx` / `PreviewComments.tsx`) — no site is left silent.

**And** zero inline hex for covered roles (DT1); the feedback state uses named tokens. If the design
canon carries no token for a confirmed-action state, that is a GATE-1 question for Andi, not an
implementer's choice.

**Settled design decisions (Andi, 2026-08-19 — decided, NOT open):** Copy shows "Copied"; Delete gets
an undo window, deliberately NOT a confirmation prompt. Open and Andi's alone: how long "Copied"
lingers and how long the undo window holds — a matter of feel, not measurement.

**Scope guard:** desktop only. The Android clipboard twin is deferred to Epic 9 (see
`docs/backlog.md`). This story adds NO SQLite schema change and NO Rust change — the undo window is an
optimistic frontend delete, so a crash inside the window leaves the entry intact.

**DoD:** Windows release build + smoke (copy from a history card and confirm the label changes; delete
an entry and undo it; delete an entry and let the window lapse, then confirm it is gone after a
restart); `tsc`/`vite` green.

Source: `sprint-change-proposal-2026-08-19.md`.

---

## Epic 9: Android Visual Overhaul + Bubble Interaction

The Android app shares the Studio-Dark language, and the dictation bubble gains the spec'd behavior:
a state sequence with a Klarvo-owned listening panel + reactive waveform, and a long-press popover
menu. Opens with a load-bearing rendering ADR; the state harness lands before the states it verifies.

### Story 9.1: Adr android bubble rendering tech

**Key `9-1` — ADR — Android bubble rendering tech (precursor gate)**

As an architect,
I want a decision on whether to extend the existing View+Canvas overlay or introduce a ComposeView,
So that all bubble stories build on a settled substrate and the epic estimate is honest.

**Acceptance Criteria:**

**Given** the current overlay (`FloatingBubbleView.kt` View+Canvas, `KlarvoOverlayService.kt`)
**When** the ADR is written
**Then** it records the decision (extend View+Canvas vs introduce ComposeView inside the
`SYSTEM_ALERT_WINDOW` overlay) with rationale covering: motion needs (state sequence + spring), the
listening-panel composition, RMS-waveform rendering, the risk of mixing Compose into an overlay
service, and the effort delta.

**Given** the decision
**When** recorded
**Then** it lands as `docs/adr/00NN-*.md` per the ADR convention + index update and is referenced by the
Epic 9 stories.

**And** the ADR names the verifiability-symmetry implication — how the chosen substrate supports the
9.4 state harness.

**DoD:** ADR committed (own commit per ADR convention). No code.

### Story 9.2: Android token theme source and fonts

**Key `9-2` — Android token/theme source + fonts (foundation)**

As a developer,
I want the Android color/theme source and bundled fonts created,
So that the bubble and in-app surfaces re-skin against named tokens like the desktop.

**Acceptance Criteria:**

**Given** no Android theme file exists today
**When** 9.2 is done
**Then** a Kotlin token source defines the Studio-Dark ladder as `Color(0xFF…)` per spec
(Bg/Surface/Surface2/Elevated/Border/Border2/TextC/Muted/Dim/Teal/TealHi/TealLo/OnTeal/Amber/Danger).

**Given** fonts
**When** bundled
**Then** Geist + Geist Mono ship as font resources (no runtime fetch) and typography matches the scale.

**Given** Android has no native backdrop-blur
**When** glass surfaces are specified
**Then** the tokens encode solid `#16181A` + `Modifier.shadow` and the 4dp-ring approach (DT4 android).

**And** the teal/amber/danger color semantics are documented for Android use (DT5).

**DoD:** Android builds; on-device smoke that the app launches with the new theme on ≥1 reference
surface; APK freshness verified via `scripts/android-build.sh` timestamp gate (no in-UI version).

### Story 9.3: Bubble idle re-skin + responsive sizing + anchoring

As a user with a focused text field,
I want the idle bubble to look right and sit correctly at any screen size,
So that it's reachable and unobtrusive.

**Acceptance Criteria:**

**Given** a focused field + open keyboard
**When** the bubble shows idle
**Then** it renders a **teal-gradient squircle** (rounded square, 12px-equivalent corner radius — NOT a circle)
with a **dark "K"** (OnTeal) centered and a subtle teal ring, per the canon `.ab-bubble.idle`
(`docs/design/overhaul/source/`); the **same form** is used across states (no circle↔square morph).

**Given** varying screen sizes
**When** sized
**Then** `visual = clamp(36dp, 0.11 × min(screenW,screenH)dp, 44dp)` and the touch target =
`max(visual, 48dp)` via transparent padding (AR5c).

**Given** the bubble
**When** dragged
**Then** it is draggable, edge-snaps, remembers its side, and jumps up with the keyboard (FR7); nav-bar
clearance uses fixed px, not `env(safe-area-inset-bottom)` (AR5d).

**DoD:** on-device smoke (bubble appears on field focus, correct size on a real phone, drag/snap/side-
memory work); APK freshness verified.

### Story 9.4: Bubble state harness

**Key `9-4` — Bubble state harness (verifiability precursor — before 9.5)**

As a developer (and as Andi the human tester),
I want a dev-only way to drive the bubble through all four states on demand,
So that the upcoming state UI is verifiable without live audio/network — built BEFORE the states.

**Acceptance Criteria:**

**Given** a dev/debug entry point
**When** invoked
**Then** the bubble can be put into idle / recording / transcribing / done deterministically on-device,
with synthetic RMS levels + synthetic raw-transcript text feeding the panel.

**Given** the harness
**When** used by the human tester
**Then** **Andi can reproduce each state himself** (verifiability symmetry — the gate Andi must pass is
reachable by Andi, not only the agent).

**Given** release builds
**When** shipped
**Then** the harness is dev-only / gated out (no user-facing surface, no telemetry).

**And** the harness exists and is demonstrated **before** Story 9.5 begins (sequencing gate).

**DoD:** on-device demonstration that all four states + the waveform/transcript can be triggered via the
harness.

### Story 9.5: Bubble state sequence listening panel waveform

**Key `9-5` — Bubble state sequence + listening panel + waveform (the big one)**

As a user dictating from a text field,
I want the bubble to run idle→recording→transcribing→done with a Klarvo-owned listening panel,
So that I see live feedback and the cleaned text lands in my field.

**Acceptance Criteria:**

**Given** recording starts
**When** the panel rises
**Then** a Klarvo-owned panel shows a grab handle, K + amber live-dot, a reactive RMS waveform, a timer,
and a **red square = Abbrechen** (cancel/discard, parity with desktop); the footer reads "keyboard
paused · returns on insert".

**And** the bubble stays visible in its **recording state** (`.ab-bubble.recording`: teal squircle +
amber pulse-ring + send-glyph, NOT the idle K); **tapping the bubble = Senden** (stop → transcribe →
paste). Confirm (bubble-tap) and Cancel (red square) are distinct affordances; **red is never the
send/confirm action** (ADR-0019 colour-semantics rule).

**Given** recording
**When** raw text streams
**Then** the live **RAW** transcript runs multiline **in the panel** — NOT in the foreign field (AR5a:
a `SYSTEM_ALERT_WINDOW` overlay cannot set composing text; only a final write is possible).

**Given** transcribing
**When** cleanup runs
**Then** the same panel shows a teal spinner + "Cleaning…" with the raw text dimmed.

**Given** done
**When** complete
**Then** the panel collapses, the keyboard returns, the **cleaned** text is written to the field (a11y
`ACTION_SET_TEXT` or clipboard+paste), and the bubble shows a brief check → idle.

**And** the states are verified via the 9.4 harness; inversion: attempting in-field live preview text
from the overlay is impossible by AR5a and must not be claimed as done.

**DoD:** on-device smoke (real end-to-end dictation in a 3rd-party app: panel states, reactive waveform,
cleaned text lands in a real field) via `scripts/android-smoke.sh`.

### Withdrawn 9.6: Keyboard-collapse via a11y service (optional, own item — AR5b)

**No key in sprint-status.yaml — struck 2026-09-16, see docs/backlog.md "Story 9-6".**

As a user who wants the keyboard out of the way during dictation,
I want an optional setting to collapse the keyboard while recording,
So that the listening panel has room — with a safe fallback when it's unreliable.

**Acceptance Criteria:**

**Given** the option is OFF (default)
**When** recording
**Then** the keyboard stays as-is (fallback = keep keyboard open) — no behavior change for non-opt-in
users.

**Given** the option is ON
**When** recording starts
**Then** the a11y service dismisses the IME while keeping the target field focused, and the final insert
still works.

**Given** a per-app case where dismiss fails
**When** recording
**Then** it degrades gracefully to keyboard-open (no broken state).

**And** the toggle is a camelCase config key via `save_config_locked` (ADR-0015); if shared, mirrored
Rust↔Kotlin (ADR-0016 / NFR7).

**DoD:** on-device smoke on ≥2 apps (one where collapse works, one fallback path) via
`scripts/android-smoke.sh`.

### Story 9.7: Short-press gesture modes (mirror desktop)

As a user,
I want short-press to support the same four gesture modes as the desktop hotkey,
So that triggering dictation is consistent across platforms.

**Acceptance Criteria:**

**Given** settings
**When** the user picks a default gesture
**Then** Hold / Toggle / Auto-Stop / Auto are available — the **same four** modes as the desktop
hotkey-mode (FR5).

**Given** a short-press
**When** it fires
**Then** it behaves per the selected mode.

**Given** this is shared behavior
**When** stored
**Then** the config key is camelCase and **mirrored Rust↔Kotlin** (NFR7/ADR-0016); silence/auto-stop
thresholds reuse the existing **mode-centric** fields (avoid the Android silence-field divergence).

**DoD:** on-device smoke (each mode triggers correctly); config round-trip verified; `android-smoke.sh`.

### Story 9.7: Followup swap safe silence

**Key `9-7` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

Follow-up of the 9-7 code review: the four silence-duration values of `RecordingMode.selectSilenceSecs()` get distinct types, so that a call-site swap fails to compile.

**Source:** `_bmad-output/implementation-artifacts/9-7-followup-swap-safe-silence.md`

### Story 9.8: Long press popover menu

**Key `9-8` — Long-press popover menu (remap from push-to-talk)**

As a user,
I want long-press to open a quick popover instead of push-to-talk,
So that I can switch gesture/mode/target/language without opening full settings.

**Acceptance Criteria:**

**Given** the bubble
**When** long-pressed
**Then** a popover opens **inward** (never radial) with: a "default gesture" block (4 modes), a "mode"
block (Polished/Verbatim/Chat), a row for Target (field/clipboard) + Language (DE/EN/Auto), and a
footer "open settings" (FR6).

**Given** long-press previously triggered push-to-talk
**When** 9.8 lands
**Then** long-press is **remapped** to the menu; push-to-talk remains reachable via the short-press
gesture modes (no capability lost).

**Given** the two axes
**When** the user changes them
**Then** gesture (how triggered) and mode (how cleaned) are independent and persist (camelCase, mirrored
where shared).

**And** the popover uses tokens with ≥48dp touch targets.

**DoD:** on-device smoke (long-press opens the menu, selections persist + take effect, short-press still
dictates) via `scripts/android-smoke.sh`.

### Story 9.9: In app recording state re skin

**Key `9-9` — In-app recording state re-skin (small — D2)**

As a user recording inside the app,
I want the in-app recording surface to match the new design language,
So that the app is visually consistent end-to-end.

**Acceptance Criteria:**

**Given** the in-app recording state (`android-05`)
**When** it renders
**Then** it uses the new tokens/type/motion; behavior and IA are unchanged (re-skin only).

**And** no hardcoded colors for covered roles remain in this surface (DT closure for the Android in-app
surface).

**DoD:** on-device smoke (in-app recording visual) via the build/smoke scripts; APK freshness verified.

### Story 9.10: Token codegen klarvo css to klarvotheme

**Key `9-10` — Token codegen — `klarvo.css` → `KlarvoTheme.kt` (post-ADR-0019; before the 9.5 rebuild)**

As a developer maintaining two platform implementations of one design,
I want the Android token file generated from the canon CSS rather than hand-typed,
So that the token layer cannot structurally drift (closing the F6 class of copy-errors) and the 9.5
rebuild renders against the real single-source-of-truth.

**Acceptance Criteria:**

**Given** the canon `docs/design/overhaul/source/assets/klarvo.css` holds the `--k-*` custom properties
**When** the generator runs
**Then** it emits `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` with every canon color token as a
Kotlin constant, with hex `#RRGGBB` → `0xFFRRGGBB` and `rgba(r,g,b,a)` → `0xAARRGGBB` (alpha = round(a×255)),
and **no canon-derived hex is hand-typed** anywhere in platform code.

**Given** the current consumers (`FloatingBubbleView.kt`, `ListeningPanelView.kt`) reference identifiers
like `KlarvoTheme.TextC`, `Border2`, `AmberLine`, `TealBg`
**When** the file is regenerated
**Then** every currently-referenced identifier still resolves with a **byte-identical color value**
(zero visual regression) — an explicit alias map preserves non-mechanical names (e.g. `--k-text` → `TextC`).

**Given** the alpha conversion
**When** the file is generated
**Then** `AmberLine == 0x52E9A24C`, `TealBg == 0x1F29C7AC`, `DangerBg == 0x1FEE6F63` (the F6 class is
produced correctly by the rule, not by hand).

**Given** someone hand-edits a generated token value
**When** the build/smoke flow runs
**Then** a **drift gate** (regenerate to temp + diff against the committed file) fails the build with a
clear "KlarvoTheme.kt drifted from canon — re-run the generator" message.

**And** canon color tokens absent from today's hand-written file (`--k-bg-deep`, `--k-hairline`,
`--k-faint`, `--k-teal-line`, `--k-success`, `--k-info`) are added, so the file is a complete projection
of the canon color set.

**DoD:** generator + drift gate wired into `scripts/android-smoke.sh` (and `scripts/android-build.sh`)
**before** the `kotlin-src` sync; the 60 JVM unit tests still pass; the DEBUG APK builds. **No pixel
changes** (values are byte-identical to today) → the human visual gate is consciously downgraded to an
optional sanity glance; the binding gate is the byte-identity assertion + the drift check (machine-verifiable).

### Story 9.11: Android honors silence threshold mic sensitivity

**Key `9-11` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

Android honors the `silence_threshold` (mic sensitivity) setting and ships the desktop default 0.005 instead of the hard-coded 0.02.

**Source:** `_bmad-output/implementation-artifacts/9-11-android-honors-silence-threshold-mic-sensitivity.md`

### Story 9.12: Cluster waveform rms reactive

**Key `9-12` — Cluster-Waveform RMS-reaktiv (9-5 GATE Follow-up #1)**

As a user dictating on Android,
I want the amber recording-cluster waveform to move with my actual voice amplitude (RMS),
So that the live cue honestly reflects that I'm being heard — matching the desktop, not a generic idle animation.

**Scope (locked — fidelity fix, do NOT expand):** The recording-cluster waveform zone currently animates
with a generic/idle fallback and does not track live mic RMS. AC4 of Story 9.5 already specified "bars
driven by RMS amplitude (reuse `drawWaveformBarsInZone()`)"; the amplitude feed into the *cluster*
waveform zone is evidently unwired (or always falls back to the flat-idle `abwv`-style animation). Trace
the existing live RMS amplitude stream into the cluster waveform zone. **No** new tokens, **no** geometry
change, **no** new states, **no** gesture-mode changes (those are separate follow-ups #2/#4).

**Anchors:** `docs/backlog.md` §"Story 9-5 GATE-4 green" point (1); Story 9.5 AC4 + `drawWaveformBarsInZone()`;
canon `docs/design/overhaul/source/` (fingerprint `fc9ef745…`) `.hwave` comment = "RMS-getriebener Live-Cue,
NICHT idle-Animation". ADR-0019 §4′ + §4′-Amendment 2026-06-21.

**DoD (surface-class):** DEBUG APK builds; the existing JVM unit tests pass; emulator **structural** smoke
green (overlay-window structure intact via `scripts/android-smoke.sh` under `BMAD_CONDUCTOR=1`). **GATE-4
visual = real device (Andi's live mic):** RMS reactivity is only honestly verifiable against a live
microphone — the emulator is a structural oracle only, never a motion/pixel oracle. Andi's batched
real-device gate confirms the bars track his voice. Overlays must never use `FLAG_NOT_TOUCHABLE`.

---

### Story 9.13: Swap send cancel cluster order

**Key `9-13` — Recording-Cluster-Reihenfolge tauschen (9-5 GATE Follow-up #2)**

As a user dictating on Android,
I want the **➤ Send** control to sit at the dock/thumb position of the recording cluster (where the idle K-bubble sits) and **✗ Cancel** on the opposite (left) side,
So that the most-used action (send) is under my thumb and matches human habit, while the destructive action (cancel) is deliberately off the thumb path.

**Scope (locked — cluster-order/interaction change only, do NOT expand):** Swap the recording-state control-cluster order on Android from the current `[➤ Send (left) · waveform · ✗ Cancel (right/thumb)]` to `[✗ Cancel (left) · waveform (center) · ➤ Send (right/thumb)]`. ➤ Send (teal) moves to the dock/thumb anchor of the idle K-bubble; ✗ Cancel (red) moves to the left; the amber waveform stays centered. Color semantics are binding (ADR-0019): **red = Cancel, teal = Send** — both platforms. **No** RMS/waveform behavior change (that is #1 / Story 9.12, done — do not touch). **No** HOLD-mode surfaces (that is #4 — separate story; do not build `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-heldbub`/`.ab-lockchip` here). **No** new tokens, **no** new states, **no** gesture-mode logic change. Do **not** silently expand Story 9.7.

**Anchors:** `docs/backlog.md` §"Story 9-5 GATE-4 green" point (2); canon `docs/design/overhaul/source/Klarvo Design System.html` + `assets/klarvo.css` (fingerprint `fc9ef745…`, MANIFEST 2026-06-21) — cluster order `[✗ cancel (links) · hwave · ➤ send (RECHTS)]`; approval render `docs/design/overhaul/mockup-9-5-followups-2-4.html` (section #2); ADR-0019 §4′ + §4′-Amendment 2026-06-21. Design gate is **resolved** (Andi-approved, commit `864af40`) — no open design/UI/intent question.

**DoD (surface-class):** DEBUG APK builds; the existing JVM unit tests pass; emulator **structural** smoke green (overlay-window structure intact via `scripts/android-smoke.sh` under `BMAD_CONDUCTOR=1`; the structural assertion can confirm cluster element presence/order/anchor where machine-checkable). **GATE-4 visual = real device (Andi's batched gate):** final pixel/placement verdict is Andi's real-device sight, never an emulator screenshot — the emulator is a structural oracle only. Overlays must never use `FLAG_NOT_TOUCHABLE` (HyperOS dims them to alpha 0.8).

### Story 9.14: Hold mode push to talk cluster

**Key `9-14` — HOLD-Modus (Push-to-Talk) Bubble-Cluster-Variante (9-5 GATE Follow-up #4)**

As a user dictating on Android with the **Hold** gesture mode,
I want pressing-and-holding the bubble to record, releasing to send, and dragging away to cancel — with an upward drag to **lock** into a normal tappable cluster,
So that the recording cluster matches the familiar voice-message model that Hold actually implies, instead of the tap/toggle cluster whose ➤ Send is redundant (release already sends) and whose ✗ Cancel is unreachable while holding.

**Scope (locked — Hold-mode bubble interaction + its surfaces only, do NOT expand):** Add the HOLD-mode recording variant on Android, used **only** when the active gesture mode is **Hold**. While the finger holds: **hold = record · release = send · drag away = cancel** (no tappable ➤/✗ exist during the hold). **Drag up → 🔒 lock** converts the held state into the normal tap-cluster `[✗ Cancel (left) · waveform (center) · ➤ Send (right/thumb)]` (the order from #2 / Story 9.13) so the user can release without sending. Live cue stays the **amber** waveform; the hold ring is **amber**. New canon surfaces: `.ab-holddock` / `.ab-holdstrip` / `.ab-slidehint` / `.ab-heldbub` / `.ab-lockchip`. Color semantics binding (ADR-0019): **red = Cancel, teal = Send, amber = live** — never swapped. **No** change to Tap / Toggle / Auto-Stop / Auto modes (they keep the §4′ cluster unchanged). **No** RMS/waveform behavior change (that is #1 / Story 9.12, done). **No** token changes. Do **not** silently expand Story 9.7 (gesture modes) — this is its own story; the Hold-mode *detection* already exists, this story is its *recording-cluster surface + interaction*.

**Anchors:** `docs/backlog.md` §"Story 9-5 GATE-4 green" point (4); canon ADR-0019 §4′ + **§4′-Amendment 2026-06-21 (#4)** (`docs/adr/0019-cross-platform-design-ssot.md`); canon source `docs/design/overhaul/source/` (fingerprint `fc9ef7456700d19b8332dd2c34a43b8e`, MANIFEST 2026-06-21) — Artboard-Sektion „Aufnahme · HOLD-Modus" + surfaces `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-heldbub`/`.ab-lockchip`; approval render `docs/design/overhaul/mockup-9-5-followups-2-4.html` (HOLD section). Design gate is **resolved** (Andi-approved 2026-06-21) — no open design/UI/intent question.

**DoD (surface-class):** DEBUG APK builds; the existing JVM unit tests pass; emulator **structural** smoke green (overlay-window structure intact via `scripts/android-smoke.sh` under `BMAD_CONDUCTOR=1`; the structural assertion can confirm hold-dock/lock-chip surface presence + the lock→cluster window transition where machine-checkable). **GATE-4 motion/touch = real device (Andi's batched gate):** the press-hold-release / drag-to-cancel / drag-up-to-lock gesture and its live waveform are **only** verifiable on Andi's real device with a live mic — never an emulator (the emulator is a structural oracle only; it cannot drive a held touch + live amplitude). Overlays must never use `FLAG_NOT_TOUCHABLE` (HyperOS dims them to alpha 0.8).

> **⚠️ Story 9.14 NEU GEFASST 2026-06-26 (B-Sprache).** Die obige 9.14-Beschreibung (Slide-Spur-HOLD) ist
> **superseded**: Andis Real-Device-Test verwarf die mobile Aufnahme-Steuerung als zu klein/„Laptop-Feel".
> Redesign in „B-Sprache" — siehe ADR-0019 Amendment 2026-06-26 + Story-File `9-14-...md` (neu) + die Render
> `mockup-mobile-hold-B-refined.html` / `mockup-mobile-recording-states.html`. Build folgt in frischer Session.

### Story 9.15: Mobile tap recording surface reskin

**Key `9-15` — Mobile TAP-Aufnahme-Surface (B-Sprache Re-Skin, ersetzt den Klein-Cluster)**

As a user recording on Android in tap/toggle/auto modes (and after locking a HOLD recording),
I want large thumb-friendly **Senden / Abbrechen** targets instead of the small `[✗·Waveform·➤]` cluster,
So that I can hit the right control without my finger covering it (phone feature, not laptop feature).

**Scope (locked):** Replace the `.ab-cluster` small cluster (RECORDING, tap/toggle/auto modes) with two **large round tappable targets** — **Senden** (teal ➤) at the dock/thumb, **Abbrechen** (dark + red ring ✕) opposite, plus a calm amber waveform chip (no overlap). **Dock-adaptive** (mirror for left/up/down). This surface is **also the "gesperrt" state consumed by Story 9.14** (post-lock). Color semantics binding (teal=Senden, rot=Abbrechen). No pipeline change; surface + touch zones only. NOT in scope: idle/transcribing/done/preview (later pass).

**Anchors:** ADR-0019 **Amendment 2026-06-26** „B-Sprache"; binding render `docs/design/overhaul/mockup-mobile-recording-states.html` (frames `tapRight`/`tapLeft`); canon fingerprint `bac152993046699c5007612ac916d951` (MANIFEST 2026-06-26, supersedes `.ab-cluster`). Foundational for Story 9.14.

**DoD (surface-class):** DEBUG APK builds; JVM tests pass; emulator structural smoke green (TAP-surface window present, size ≠ old small cluster). **GATE-4 visual/touch = real device (Andi's batched gate):** placement/size/legibility + tap behaviour = Andi's real-device sight, never an emulator screenshot. No `FLAG_NOT_TOUCHABLE`.

---

_Visual-overhaul planning artifact (Epics 8 + 9). Codeable contract:
`docs/design/overhaul/SPEC-studio-dark-overhaul.md` (+ 01..04). Per-story full context via
`bmad-create-story` per session._

### Story 9.16: Revert non hold recording to compact cluster

**Key `9-16` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

Non-HOLD recording goes back to the compact cluster (small symbols without text, fixed 150x52 dp). HOLD stays as in 9-14. Decision: ADR-0019 Amendment 2026-07-01 #2.

**Source:** `_bmad-output/implementation-artifacts/9-16-revert-non-hold-recording-to-compact-cluster.md`


# Part 6: Native Desktop Overlays (10)

*Former file: `epics-native-overlays.md`. Its frontmatter is kept below.*

```yaml
status: ready-for-dev
trackType: brownfield-architecture-migration
featureEpics: [10]
inputDocuments:
  - docs/adr/0021-native-desktop-overlays.md          # binding architecture decision + proof + sub-decisions
  - src/FloatingBar.tsx                                # appearance SOLL for the native pill (current approved look)
  - src/PreviewPanel.tsx                               # appearance SOLL for the native preview
  - docs/design/overhaul/SPEC-studio-dark-overhaul.md  # directional token source (NOT a re-skin mandate here)
  - docs/bar-redesign-spec.md                          # pill geometry / positioning math
  - docs/deep-dive-bar-subsystem.md                    # Ist-Zustand of the bar subsystem
  - _bmad-output/project-context.md                    # code rules (camelCase, platform gates, surface DoD)
  - docs/surface-smoke-checklist.md                    # surface-class DoD control
note: >
  Mini-epic: replace the two transparent always-on-top desktop overlays (pill + preview) — which
  go blank when occluded because the WebView2 compositor halts (4 transient fixes, see ADR-0021) —
  with native Win32 layered windows drawn from Rust. Architecture migration, NOT a re-skin: the
  native overlays reproduce the CURRENT approved look 1:1; the parked Epic 8 Studio-Dark re-skin is
  out of scope here. Proof-first: pill (Story 10-1) ships first and validates the substrate; preview
  (Story 10-2) reuses it. Branch feat/native-desktop-overlays off v1-ship. Per-story full context via
  bmad-create-story. DoD split: occlusion = machine-verified (harness in ADR-0021); appearance =
  Andi smoke on real Windows.
```

## Epic 10: Native Desktop Overlays



### Overview

The pill (`bar`, 200×36) and live-preview (`preview`) overlays go blank whenever a foreground window
covers their screen region — the core "Pille unsichtbar in anderen Apps" blocker. Root cause
(measured, see [ADR-0021](../../docs/adr/0021-native-desktop-overlays.md)): the occlusion-present
halt lives inside the WebView2/Chromium compositor and is not fixable by any flag or runtime version
(four transient fixes confirmed it). A native layered topmost window stays fully composited when
occluded (proven 7600/7600 + dwell). Decision: render both overlays as native Win32 layered windows
from Rust.

Two stories, separated by risk and human-test surface:

- **Story 10-1 — Native pill** (the proof slice): all the hard primitives live here (layered-window
  substrate, per-pixel-alpha present, RMS waveform, state rendering, drag). Validates the whole
  approach before the preview reuses it.
- **Story 10-2 — Native preview** (reuses the substrate): scrollable text card, click-through,
  pill-anchored, grow-up.

This is an **architecture migration, not a re-skin** — each native overlay reproduces the *current*
`FloatingBar.tsx` / `PreviewPanel.tsx` appearance 1:1. The parked Epic 8 Studio-Dark re-skin is
explicitly out of scope.

### Requirements Inventory

Categories: **AR** = architecture/substrate, **VR** = visual fidelity (against the current render),
**IR** = interaction parity, **NFR** = non-functional / DoD.

- **AR1** — The pill and preview are native Win32 top-level windows
  (`WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE`; preview adds
  `WS_EX_TRANSPARENT`), content presented via `UpdateLayeredWindow(ULW_ALPHA)` from a top-down 32bpp
  premultiplied-BGRA DIB, CPU-rasterized. No GPU/swapchain/DirectComposition. Windows-only.
- **AR2** — The native overlays are driven by the Rust pipeline state + RMS **in-process** (no
  dependency on `klarvo://` events for these two windows). The existing emitters may remain for other
  consumers.
- **AR3** — As each native overlay lands, its WebView2 window, its `main.tsx` label route, and its
  React entry point (`FloatingBar.tsx` / `PreviewPanel.tsx`) are removed. The `main` (settings)
  window stays WebView2.
- **AR4** — ADR-0020's runtime-pin machinery (bundled runtime + `sync-and-build.ps1` self-heal) is
  retired once **both** overlays are native (tracked in 10-2's DoD; do not remove early — the main
  window unaffected, but the overlay driver is what justified it).
- **VR1** — Each pill state renders 1:1 with the current `FloatingBar.tsx` look: idle (hidden),
  recording (pill + 5-bar teal `#2AC3A8` waveform + stop affordance), transcribing/cleaning (amber
  `#FFA344` spinner + label), done (green `#4ADE80` check / clipboard amber), error (red `#FF7369`).
  Rounded-pill shape, ~96%-opaque dark fill. **No** Studio-Dark re-skin.
- **VR2** — The preview renders 1:1 with the current `PreviewPanel.tsx` look: dark card, scrollable
  cleaned text, bottom-aligned grow-up, top-fade when scrolled, teal hairline border.
- **VR3** — Backdrop blur is dropped (ADR-0021 sub-decision 3); the near-opaque fill makes this
  negligible. If Andi's smoke flags it, real blur is a separate follow-up.
- **IR1** — Pill drag-to-move + position persistence via the existing `config.bar_x/bar_y`
  (`save_bar_position` / `get_bar_position`), restored on next start.
- **IR2** — Preview is click-through (`WS_EX_TRANSPARENT`), anchored to the pill, repositions when
  the pill is dragged.
- **NFR1** — Occlusion-survival is **machine-verified** per story via the ADR-0021 harness (content
  pixels remain while a foreground app is maximized over the region, incl. 3 s dwell).
- **NFR2** — Visual fidelity is **Andi's smoke** on a real Windows release build. Never claimed from
  machine output.
- **NFR3** — No regression to the recording pipeline, hotkeys, paste, or the `main` window.

---

### Story 10.1: Native pill overlay

**Key `10-1` — Native pill (FloatingBar) overlay**

**As** a Klarvo user dictating into another app,
**I want** the recording pill to stay visible when that app covers its spot,
**so that** I can always tell whether recording is active — permanently, not until the next restart.

#### Acceptance Criteria

**AC-1 — Native layered window replaces the WebView2 `bar`:**
Given the app starts on Windows
When the pill is created (where `create_bar_window` is called today)
Then a native Win32 top-level window with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW |
WS_EX_NOACTIVATE` is created at the saved/default pill position, sized 200×36, its content presented
via `UpdateLayeredWindow(ULW_ALPHA)` from a premultiplied-BGRA DIB
And the WebView2 `"bar"` window, its `main.tsx` `"bar"` route, and `src/FloatingBar.tsx` are removed
And the change is gated `#[cfg(target_os = "windows")]`

**AC-2 — All pill states render natively, matching the current look 1:1:**
Given the recording pipeline drives the pill state
When the state is one of idle / recording / transcribing / cleaning / done / error
Then the native pill renders the **current** `FloatingBar.tsx` appearance for each state — idle
hidden; recording = rounded pill + 5-bar teal (`#2AC3A8`) waveform + stop affordance; transcribing &
cleaning = amber (`#FFA344`) spinner + label; done = green (`#4ADE80`) check (or amber clipboard when
paste failed); error = red (`#FF7369`) — with the rounded-pill shape and ~96%-opaque dark fill
And **no** Studio-Dark re-skin is introduced (this is a tech migration; colors/shape mirror today)

**AC-3 — Waveform is RMS-driven in-process:**
Given recording is active
When RMS amplitude updates arrive on the existing `set_level_callback` path (~15 Hz)
Then the native pill's waveform updates directly in-process (no `klarvo://audio-level` JS round-trip
required), using the same mapping as today (`pow(min(1, level*10), 0.4)`, noise floor `0.006`, 5 bars,
12% floor) so the visual response matches the current pill

**AC-4 — Drag-to-move + position persistence (parity):**
Given the native pill is visible
When the user drags it
Then it follows the cursor, and the new position persists via `config.bar_x/bar_y`
(`save_bar_position`) and is restored on next start — behavioural parity with the WebView2 pill

**AC-5 — Occlusion-survival, machine-verified (the whole point):**
Given the native pill is visible during recording
When a foreground app is maximized over its screen region, and again after a 3 s dwell
Then the pill stays fully painted (content pixels > 0, ≈100% of the region) — verified by the
ADR-0021 occlusion harness; this is the exact scenario where the WebView2 pill measured 0

**AC-6 — No pipeline / main-window regression:**
Given the native pill has replaced the WebView2 bar
When recording, transcription, cleanup, and paste run, and the settings window is opened
Then the pipeline, hotkeys, paste, and the `main` window behave exactly as before

#### DoD (surface-class)

- Real Windows release build via `scripts/sync-and-build.ps1`.
- **Occlusion harness PASS** (machine, agent-run, AC-5) — content pixels survive foreground occlusion
  + 3 s dwell; recorded as evidence before the human gate.
- **Andi smoke on real Windows** (NFR2): pill looks right across all states; drag works; survives
  occlusion in real use.
- `cargo check --target x86_64-pc-windows-gnu` green; Linux `cargo test` green; `tsc` / `npm run
  build` green after FloatingBar removal.
- Surface-smoke-checklist traps reviewed (esp. region/geometry; event-wiring N/A since in-process).
- Code-review inversion (reviewer-verified, not self-attested) per project rules.

---

### Story 10.2: Native preview overlay

**As** a Klarvo user,
**I want** the live-preview card to stay visible when an app covers it,
**so that** I can read the transcript-so-far while dictating into that app.

#### Acceptance Criteria

**AC-1 — Native layered window replaces the WebView2 `preview`:**
Given Story 10-1 established the native layered-window substrate
When the preview is created
Then a native Win32 window (`WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE |
WS_EX_TRANSPARENT` for click-through) renders the preview, and the WebView2 `"preview"` window, its
`main.tsx` route, and `src/PreviewPanel.tsx` are removed

**AC-2 — Renders the current preview look 1:1:**
Given live-preview chunks arrive
Then the native preview shows the current `PreviewPanel.tsx` appearance — dark card, scrollable
cleaned text bottom-aligned and growing up, top-fade when scrolled, teal hairline border — driven by
the existing live-preview-chunk flow (in-process or via the existing event)

**AC-3 — Click-through + pill-anchored positioning (parity):**
Given the preview is visible
Then cursor events pass through it, it is anchored above the pill, and it repositions when the pill
is dragged — parity with today

**AC-4 — Occlusion-survival, machine-verified:**
Given the preview is visible
When a foreground app is maximized over its region (+ 3 s dwell)
Then it stays fully painted — verified by the occlusion harness

**AC-5 — Retire ADR-0020 machinery:**
Given both overlays are now native
Then the bundled-runtime pin + `sync-and-build.ps1` self-heal (ADR-0020) are removed, and ADR-0020 is
marked Superseded in the index (already noted; confirm clean removal)

#### DoD (surface-class)

- Same shape as 10-1: Windows release build; occlusion harness PASS (machine); Andi smoke
  (appearance + click-through + anchoring); `cargo check`/`cargo test`/`tsc` green; review inversion.

### Story 10.3: Native pill standby resilience

**Key `10-3` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

The native pill survives power and session transitions (Modern Standby, sleep, lock). Root cause and diagnosis: ADR-0021 amendment.

**Source:** `_bmad-output/implementation-artifacts/10-3-native-pill-standby-resilience.md`

### Story 10.4: Native overlay dpi scaling and appearance wiring

**Key `10-4` — Native overlay DPI scaling + appearance-wiring audit**

**As** a Klarvo user on a high-DPI (125/150 %) display,
**I want** both native overlays (pill + preview) to render at the same size as the old WebView2
overlays, with the appearance/size settings visibly effective,
**so that** the overlays are legible and the font-size presets actually differ.

Source: Andi real-device smoke after Story 10-2. Since the native rebuild (Epic 10), **both** overlays
render too small vs. the old WebView2 overlays, and the font-size presets feel far too weak
(large ≈ tiny, small ≈ unusable). Full scope + read-only diagnosis: `docs/backlog.md`
("Epic 10 — Native-Overlay-Skalierung zu klein + Appearance-Wiring-Audit").

**Leading hypothesis (to confirm on device, NOT yet verified on Windows):** wrong DPI scale, shared by
both overlays. `native_pill.rs:1259` and `native_preview.rs:942` compute
`scale = GetDeviceCaps(screen_dc, LOGPIXELSX) / 96`. Under per-monitor-v2 DPI awareness (tao/Tauri's
embedded manifest), `GetDeviceCaps(screen_dc, LOGPIXELSX)` typically returns 96 → `scale = 1.0`
regardless of the monitor's real DPI → overlays render ~1.0× instead of ~1.5×. Correct API:
`GetDpiForWindow(hwnd)` / `GetDpiForMonitor`. The old WebView2 windows were correctly DPI-scaled by
Tauri. Settings→preview wiring is already correct (`previewFontSize` → `font_px` small=11/medium=13/
large=15, `native_preview.rs:94-97`) — verify end-to-end, but it is not the defect.

#### Acceptance Criteria (outline — create-story to enrich)

- **AC-1 — Root-cause confirmed on device:** the real scale is logged at runtime (`GetDeviceCaps` vs
  `GetDpiForWindow`/`GetDpiForMonitor`) so the absolute-scale defect is proven, not assumed.
- **AC-2 — Both overlays render 1:1 with the old WebView2 size:** pill (logo + red cancel button) and
  preview card match the pre-Epic-10 WebView2 scale on a high-DPI monitor (reference = git before
  Epic 10). One shared scale mechanism corrected for both.
- **AC-3 — Appearance settings verified end-to-end against the preview:** all `previewXxx` settings plus
  the size/width presets are visibly effective on the live preview.
- **AC-4 — Size presets calibrated:** small / medium / large are perceptibly distinct. *(Open design
  question — the exact target sizes are a human/taste call, surfaced at create-story elicitation, not
  pre-decided here.)*

#### DoD (surface-class)

- Windows release build; `cargo check`/`cargo test`/`tsc` green; review inversion.
- **GATE 4 is Andi's real Windows machine** at real monitor DPI (125/150 %): absolute scale + preset
  legibility are a genuine aesthetic/real-target judgment, not WSL-self-certifiable. Machine side
  (WSL) verifies cross-compile (`x86_64-pc-windows-gnu`) + any harness-observable structure only.

# Part 7: Cross-Platform Live-Preview (11)

*No former file. Epic 11 ran from `docs/backlog.md` and its story files (2026-07-02 to 2026-08-17). This part was written during the 2026-09-20 merge, so that every key of `sprint-status.yaml` has a heading.*

## Epic 11: Cross-Platform Live-Preview

The live-cleanup-preview box exists on Windows since Epics 5 and 6. Epic 11 brings the same feature to Android. Story 11.1 is the benchmark-first gate. There is no 11.5 (number gap).

**Evidence:** `_bmad-output/implementation-artifacts/epic-11-retro-2026-08-17.md` · **Backlog anchor:** `docs/backlog.md`, "Epic 11 — Cross-Platform Live-Preview (Android)"

### Story 11.1: Android live preview feasibility benchmark

**Key `11-1` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

Spike: measure on the device how fast raw transcript text is available after a speech pause. Green result releases 11.2.

**Source:** `_bmad-output/implementation-artifacts/11-1-android-live-preview-feasibility-benchmark.md`

### Story 11.2: Android live preview port

**Key `11-2` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

Port with Groq delta STT: text panel, HOLD and TOGGLE only, Settings mirror.

**Source:** `_bmad-output/implementation-artifacts/11-2-android-live-preview-port.md`

### Story 11.3: Android preview box device feedback pass

**Key `11-3` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

Device feedback pass: fixed-size rolling window, header and footer cleanup, font scale.

**Source:** `_bmad-output/implementation-artifacts/11-3-android-preview-box-device-feedback-pass.md`

### Story 11.4: Bubble structurally above preview z order

**Key `11-4` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

The bubble sits structurally above the preview panel (Z-order). Split out of 11.3.

**Source:** `_bmad-output/implementation-artifacts/11-4-bubble-structurally-above-preview-z-order.md`

### Story 11.6: Line spacing appearance setting

**Key `11-6` — added during the 2026-09-20 merge. This story had no heading in an epics file.**

Line spacing becomes an Appearance setting on both platforms.

**Source:** `_bmad-output/implementation-artifacts/11-6-line-spacing-appearance-setting.md`


# Part 8: Cloud-Resilienz (12)

*Former file: `epics-cloud-resilience.md`. Its frontmatter is kept below.*

```yaml
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories"]
status: in-progress
inputDocuments:
  - docs/backlog.md  # "Epic 12 — Cloud-Resilienz" section — ✅ ENTSCHIEDEN 2026-07-02 (verified-code Ist-Zustand + Andi decisions)
  - _bmad-output/project-context.md
trackType: brownfield-feature
featureEpic: 12
note: >
  Separate planning artifact by design. Epic 12 was triggered by a live production
  incident (2026-07-02 DeepSeek API outage) and scoped in a design pass with Andi.
  The requirements source is the decision-complete "Epic 12 — Cloud-Resilienz" section
  in docs/backlog.md, grounded in a current-code audit performed this session against
  conductor/epic-11 HEAD (the branch this epic builds on). Shares the sprint-status.yaml
  ledger. Built via the L3 feature route; no PRD/Architecture/UX document.
```

## Epic 12: Cloud-Resilienz



### Overview

A **reliability** epic, not a visual one. On 2026-07-02 the DeepSeek cleanup API went
down; Klarvo's existing provider-fallback did **not** fire (root cause below), cleanup
ran into ~30 s timeouts and silently degraded to raw text with **no user-visible
signal**. Epic 12 makes the failure behaviour robust and legible, and adds a brand-new
**audio-retry history** so a dictation is never lost when the cloud is unreachable.

There is no PRD/Architecture/UX document. Requirements below are extracted from the
decision-complete `docs/backlog.md` "Epic 12" section and a current-code audit.

### Verified current-state (audit 2026-07-02, conductor/epic-11 HEAD)

- **Fallback exists but was mis-gated.** `resolve_fallback_provider` (src-tauri/src/pipeline.rs:193)
  walks deepseek→groq→openai→openrouter. It only fires on `is_retryable_llm_error`
  (pipeline.rs:178) = `ApiError{status}` with 429 or ≥500. The outage produced **transport
  errors** ("error sending request for url" = timeout / connection-refused), which are NOT
  `ApiError{status}`, so they hit the non-retryable branch (pipeline.rs:1184) → straight to
  raw text, fallback never attempted. **← the incident's root cause.**
- **Warn message exists, UI discards it.** Backend emits `PipelineEvent::warn(degrade_warn_msg(..))`
  ("Cleanup failed — raw text inserted. <reason>", pipeline.rs:973/1163/1176/1188).
  `src/FloatingBar.tsx:335` deliberately drops `warning` events (`if (newState === "warning") return;`),
  so the user sees nothing.
- **Cleanup always degrades to raw text** (never a crash). **STT cannot** — no text means nothing
  to degrade to. Groq is today's STT provider AND the first cleanup fallback candidate → a cleanup
  fallback onto Groq eats the STT quota.
- **Audio is never persisted.** WAV bytes live only transiently (`last_recording`); the `history`
  table holds text only (`text, raw_text, style, language, is_note, app_name, uuid, device_id`).
  No audio column/blob/path, no re-processing. The audio-retry history is genuinely new.
- **Building blocks present:** local Whisper (`build_local_whisper_provider`, Windows+Android,
  pipeline.rs:84) today only on explicit offline mode; local llama.cpp cleanup also exists.

### Requirements Inventory

### Story 12.1: Robust fallback ladder and pillbar status

**Key `12-1` — Robust LLM/STT fallback ladder + pill-bar status signal**

- **FR1 — Transport errors trigger fallback.** Timeout / connection-refused / DNS / TLS errors
  from a cleanup or STT provider must be treated as fallback-eligible (same class as 429/5xx),
  not as non-retryable. This is the core fix of the incident.
- **FR2 — Cleanup fallback chain, never Groq.** Cleanup fallback order: primary (DeepSeek) →
  OpenAI / OpenRouter (only if a key is present) → **raw text**. **Groq is never a cleanup
  fallback candidate** — it must be excluded so the STT quota is protected. (Adjust
  `resolve_fallback_provider`'s candidate list for the *cleanup* path accordingly.) Terminal =
  raw text, never a crash.
- **FR3 — STT fallback to local Whisper.** When the cloud STT provider (Groq) fails
  (transport error or 429/5xx), automatically fall back to the local Whisper provider if a model
  is available (today this only runs in explicit offline mode). If no local model is available,
  the dictation's audio is preserved for retry (handoff to 12-2) and a clear error is shown.
  Terminal = never a silent loss.
- **FR4 — Pill-bar status signal.** The FloatingBar must surface the degradation/fallback as a
  brief, transient status instead of discarding the `warning` event (remove/replace the
  `FloatingBar.tsx:335` early-return). Messages are generic-but-informative, one line, no stack
  trace. Proposed taxonomy (final wording a copy detail, not a design gate):
  fallback ran `⚠ DeepSeek langsam → OpenAI` · degraded to raw `⚠ Cleanup nicht verfügbar → Rohtext eingefügt` ·
  STT safety net `⚠ Groq am Limit → lokale Transkription` · all failed `✗ Transkription fehlgeschlagen — Audio gesichert`.
- **FR5 — Both platforms.** The fallback ladder + status signal apply on **Windows and Android**
  (shared-core logic where it exists; the pill/bubble surface on each).
- **NFR1 — Output parity on the happy path.** When the primary provider succeeds, behaviour and
  output are byte-identical to today; the ladder only changes the failure path.
- **NFR2 — No new user-facing configuration required** for the default ladder (OpenAI/OpenRouter
  are used only if the user has already entered those keys).

### Story 12.2: Audio retry history

**Key `12-2` — Audio-retry history (primitive A + manual re-process)**

- On **terminal** pipeline failure (STT could not produce text after the ladder), persist the
  recording's raw **WAV to disk** (Windows + Android) and create a **second-history** entry with
  a status field (`pending`). Data model is **B-capable** (status pending/done/failed + audio-as-file),
  so 12-3 sits on it without a rebuild. Compression is noted as a later concern for B (raw WAV now).
- Provide a **manual "re-process" action** on a pending entry that re-runs STT+cleanup when the
  cloud is reachable; on success, delete the stored audio (A-retention = transient).
- Out of scope: automatic background retry, permanent audio retention.

### Story 12.3: Provider comparison

**Key `12-3` — Provider/settings comparison on the same recording (north star, later)**

- Re-run one stored recording through different providers/settings and compare results. Builds on
  the 12-2 primitive; requires durable audio retention + compression + a comparison UI. Deferred.

### L3 guards (carried into 12-1 and 12-2)

- **(G-A)** Rust unit/integration tests for the fallback ladder: transport-error classification,
  Groq-excluded-from-cleanup-fallback, terminal-degrades-to-raw, STT→local handoff. The fallback
  logic is machine-verifiable — cover it.
- **(G-B)** Surface residual: the pill-bar status is a UI change on Windows AND Android. Android is
  GATE-4-smokeable via the emulator's structural window oracle where applicable; the Windows visual
  verdict + the actual on-outage behaviour remain Andi's real-machine gate (surface-DoD,
  project-context.md testing rules).

# Part 9: Parity-Linie über Audit #2 (13)

*Former file: `epics-parity-line-audit-2.md`. Its frontmatter is kept below.*

```yaml
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories"]
status: in-progress
inputDocuments:
  - docs/adr/0016-android-path-parity-strategy.md  # Amendment 4 (2026-09-17) = the decision-complete source: 4 + 46 verdicts with form, direction, size, test symbol
  - docs/cross-platform-drift-audit-2026-09-16.md  # Audit #2: code evidence per D-* row (not re-measured here)
  - docs/backlog.md  # "DECIDED 2026-09-17 — Parity-Linie über Audit #2"
  - _bmad-output/project-context.md
trackType: brownfield-feature
featureEpic: 13
note: >
  Separate planning artifact by design. Epic 13 turns Andi's per-row verdicts over drift
  audit #2 (ADR-0016 Amendment 4, answered 2026-09-17 through the decision-sheet artifact)
  into nine stories. Nothing here re-measures: every row keeps its D-* id, and the code
  evidence lives in the audit. Cut released by Andi 2026-09-17 ("J zu allem"). The build of
  each story is its own go. Shares the sprint-status.yaml ledger. L3 brownfield route; no
  PRD/Architecture/UX document.
```



## Overview

A **parity** epic in the sense of ADR-0016 Amendment 4: v1-ship is the only product, Android
included, and every row of drift audit #2 now carries Andi's verdict. Epic 13 executes the
verdicts that are **close / gate / port / fix / strike**. Rows judged **asymmetry** or **open**
stay in the ADR list and are out of scope here.

Four principle decisions shape the stories:

- **G1a — license on both sides.** Desktop enforces the license in the dictation pipeline like
  Android. One free-tier definition for both platforms: **DeepSeek + Groq**.
- **G2a — "Offline" means no byte leaves the device.** Desktop-hotkey semantics are the norm;
  the Rust in-app button and Android follow.
- **G3a/G3b — Android hides both local paths for now.** Offline STT and local cleanup (plus the
  model manager) disappear behind platform gates. Android has no local path until a separate
  decision builds one (L, not in this epic).
- **Gate definition (new in Amendment 4):** a gate hides the control on Android **and**
  neutralizes a stored value at config load. The story spec names the normalization target;
  never a silent switch to cloud where the user had chosen local.

**Verification symmetry rule for this epic.** Andi ticked "Herstellbarkeit mitbauen" (**H+**)
on 15 agent-only rows. A story that carries an H+ row names, **before build**, how Andi
reproduces the state on his own device. Where no cheap path exists, the story records the
downgrade to "agent-verified only" (Weg 2) explicitly. Story 13-1 exists to make four of those
rows reproducible.

## Verified current-state (audit #2, 2026-09-16 — pointers, not re-measured)

- **License is reversed.** Android enforces; the Desktop hotkey pipeline has no gate; the
  Command gate reads `llm_priority` instead of `llm_provider`; the free-tier definition differs
  (Groq-only on Android, DeepSeek + Groq on Desktop). D-C1, D-H1, D-H2.
- **Guards moved after ADR-0017.** On Android the echo/fragment guard sees the dictionary
  (short sentences of dictionary words are dropped; a term ≥ 10 chars is deleted from every
  transcript), the order is inverted, ghost-strip runs before the guard only; Desktop strips
  after cleanup only. `customPrompt` goes to Whisper as a prompt on Android. D-H4–D-H7, D-M9.
- **No banking/password guard on Desktop.** Android has one but writes history/Turso before
  the guard blocks the paste. D-H3.
- **Android local cleanup cannot execute** (`libklarvo_mnn` never built, wrong model path) and
  reports raw text as success. D-H18. Local STT on Android runs `ggml-small` only, without
  dictionary or ghost-strip. D-H8, D-H13.
- **Silent loss / silent no-op on Android:** empty or truncated LLM answer is pasted, "nothing
  focused" shows the success check, clipboard failure crashes, cleanup failure shows success.
  D-H19, D-H20, D-M12, D-M16, D-M24.
- **Dead controls on Android:** `sttModel`, `localWhisperModel`, `outputLanguage`, profiles,
  silence slider, auto-send, Anthropic fields, Whisper fields, `pasteDelayMs`, `logLevel`,
  statistics panel, history app filter. D-H12–D-H17, D-M15, D-M17, D-L1–D-L3, D-L27.
- **Offline privacy:** Android "offline" + live preview uploads every pause to Groq; offline STT
  + cloud cleanup sends to DeepSeek on Android, not on Desktop; the in-app button has a third
  rule. D-H9, D-H10, D-M20, D-M21.
- **Twin infrastructure exists:** `test-fixtures/*.json` golden vectors read by Rust and JVM
  tests; `android/kotlin-test/.../Adr0017BoundaryGuardTest.kt` fails if a Kotlin STT/guard path
  reappears; `scripts/android-smoke.sh` (JVM gate) and `cargo test --lib` in `src-tauri/` run
  the net. Both twins hardcode provider URLs (`llm/mod.rs`, `KlarvoApi.kt`); the React settings
  gate with `isDesktop` / `isMobile` from `src/platform.ts`.

## Requirements Inventory

Row ids (A1 … G-Fix) and D-* ids are those of ADR-0016 Amendment 4 / audit #2. Test symbols:
📱 Andi on the Android device · 🖥️ Andi on the Windows machine · 🤖 agent only · **H+** Andi
ordered reproducibility. Sizes are Amendment-4 sizes.

## Epic 13: Parity line over audit #2

### Story 13.1: Debug test provider, both twins

**Key `13-1` — Debug test provider in both twins (H+ enabler) · S–M**

**Rows served:** H+ on D2, D3, D9, D10 (and D6 if the mechanism can inject a clipboard failure).

As the klarvo maintainer,
I want a debug provider for LLM and STT on **both** platforms that returns a configured canned
answer,
So that Andi can provoke "empty answer", "truncated answer", "malformed answer", "429/5xx" and
"transport error" on his own devices without a server.

**Acceptance Criteria (outcomes — full Given/When/Then in create-story):**
- A provider value (name chosen in the spec) exists in the Rust twin (`llm/mod.rs`, `stt/`) and
  the Kotlin twin (`KlarvoApi.kt`) and produces each of the five answer shapes from configuration.
- Selectable on both devices **without a computer attached** — an Advanced/Debug section in the
  React settings (renders on both platforms) or an equivalent on-device path; `config.json` alone
  is not enough for Android.
- Never part of the production fallback ladders (Epic 12 FR2 stays: Groq is never a cleanup
  fallback); never the default; invisible in the normal provider picker.
- Rust and JVM unit tests cover every shape; inversion check RED at writing time.

**Out of scope:** any change to real provider behaviour. **DoD:** tests green; 🖥️📱 Andi selects
"empty answer" on each device and sees the current (pre-13-2) behaviour, which proves the enabler.

### Story 13.2: Parity sweep: guards and silent loss

**Key `13-2` — Parity sweep 1: guards, core output, silent loss · S rows**

**Rows:** B2 (D-H5, D-H6, D-M9) · B3 (D-H7) · B4 (D-H4) · B6 (D-M10, D-L19, D-L21) · B1 Android
part (D-H3: history/Turso only after the guard) · D2 (D-H19) · D3 (D-M16) · D4 (D-H20) · D5
(D-M24) · D6 (D-M12) · D9 (D-M5, D-M6) · D10 (D-M2) · D11 (D-M14) · E1 (D-H9) · E2 (D-H10,
D-M20, D-M21). **Depends on 13-1** for the H+ rows D2, D3, D9, D10.

As a klarvo user on either platform,
I want the guards and the failure paths to behave the same on Android as on Desktop,
So that a dictation is never silently deleted, mangled, pasted half, or reported as success when
it failed.

**Acceptance Criteria (outcomes):**
- **B2** — in Rust `groq_jni.rs`, direction Desktop: echo/fragment guard without the dictionary,
  guard order as Desktop. 📱 Dictionary "Klarvo, Kubernetes", say "Klarvo und Kubernetes." → the
  sentence survives.
- **B3** — ghost-strip runs before the guard **and** after cleanup on both sides. 🤖 H+: a known
  ghost phrase dictated at the end is stripped on both devices.
- **B4** — Kotlin reads `advanced.sttPrompt*` and passes them through; `customPrompt` goes to the
  LLM only. 📱 Preset "Technical", same audio, transcript changes accordingly.
- **B6** — VAD hysteresis 0.5/0.35 and hangover frame in Kotlin as on Desktop (three measured
  deltas only; the state-machine parity L stays closed). 🤖 golden vector.
- **B1 Android part** — history and Turso writes happen after the banking guard's verdict. 🤖.
- **D2 / D3** — empty or truncated LLM answer ⇒ no paste, no `""` history entry (twin of Desktop).
  🤖 H+ via 13-1.
- **D4** — no focused field ⇒ "In Zwischenablage" hint instead of the success check. 📱.
- **D5** — pill = status light (as 7-10): no success check on cleanup failure. 📱.
- **D6** — Android try/catch around the clipboard; Desktop pill never says "In Clipboard" when
  nothing is in it. 🤖 H+ if 13-1 can inject it, else Weg 2 recorded.
- **D9** — empty STT result marked non-retryable; retry budget 1 like Desktop. 🤖 H+ via 13-1.
- **D10** — fallback also on a malformed provider answer to a short dictation. 🤖 H+ via 13-1.
- **D11** — no toast on "nothing recognized" on Android (direction Desktop). 📱.
- **E1** — live preview uploads nothing while a stored `local` STT value is active. 🤖 H+:
  stored `local`, airplane mode off, log shows no upload.
- **E2** — one offline rule: "offline" STT ⇒ cleanup local or none, on the Rust in-app button and
  on Android (after G3a only for stored values). 🖥️ in-app button: choose offline, dictate with
  filler words, the "ähm" stay.
- Golden vectors for B2, B3, D2, D3 in `test-fixtures/`; `Adr0017BoundaryGuardTest` stays green;
  inversion check RED at writing time for every new vector.

**Out of scope:** the Desktop banking blocklist (13-6), any UI gate (13-3). **DoD:** JVM gate +
`cargo test --lib` green with inversion evidence; 📱 rows above on Andi's Xiaomi; 🖥️ E2 on Windows.

### Story 13.3: Parity sweep: Android gates and config hygiene

**Key `13-3` — Parity sweep 2: Android gates + config hygiene · S rows**

**Rows:** A2 (D-H2) · C1 (D-H12, port) · C2 (D-H13, gate = G3a) · C3 (D-H14) · C5 (D-H16) · C6
(D-H17) · C7 (D-M17) · C8 (D-L1) · C9 (D-L2, D-L3, strike) · C11 (D-L6, strike) · C12 (D-L27,
port) · C13 gate (D-M15) · D1 (D-H18, gate = G3b + fix) · G-Fix list.

As a klarvo user on Android,
I want every control I can see to do something, and every hidden path to stay hidden,
So that a setting never silently dies and the two local paths cannot be reached on Android.

**Acceptance Criteria (outcomes):**
- **Gates (Amendment-4 definition: hide + neutralize stored value, never silently to cloud):**
  C2 `localWhisperModel` and the offline-STT option (G3a); D1 "Local (Offline)" cleanup + model
  manager (G3b); C6 auto-send; C7 Anthropic key + model fields; C8 the two Whisper Advanced fields
  (mobile); C13 statistics panel. The spec names the normalization target per key. 📱.
- **Ports / fixes:** C1 Kotlin reads `sttModel` and passes it to the JNI (🤖 H+: change the model,
  the log shows the request); C3 translate sentence in `KlarvoApi.appendPromptExtensions`, twin
  of `llm/mod.rs` (📱); C5 the silence slider takes effect on Android (📱); C12 Android writes the
  package name from the a11y service into history so the "App…" filter works (📱); D1 fix: a
  cleanup failure ⇒ clipboard + cause, never "success" (📱).
- **A2** — provider controls locked with `isPaid`; the lock names the two free-tier providers
  (DeepSeek, Groq). 📱.
- **Strikes:** C9 remove the `pasteDelayMs` and `logLevel` controls on both platforms (🖥️📱);
  C11 remove `voiceNotesHotkey`, `bubbleSize`, `bubbleOpacity`, `bubbleRecordingMode`,
  `webhookHeaders`, `webhookTimeoutSecs` from the config (both twins, load stays tolerant of
  old files). 🤖.
- **G-Fix hygiene list** (🤖 H+ where a state is reachable, else Weg 2 recorded): D-M1 config-load
  ladder (direction 12-1) · D-M11 unknown `cleanupStyle` ⇒ silently Polished, no config reset
  (direction Android) · D-M18 empty-vs-blank predicates · D-L5 deviceId · D-L10 trim · D-L11
  alphanumeric · D-L12 HTTP-2xx · D-L13 empty-term guard · D-L14 sanitize passthrough · D-L15
  blank tables · D-L16 clamp · D-L32 dead entry point · three wrong docstrings (audit §8).
  D-M22 (local cleanup without chunking) stays latent until a G3b build.
- Platform-reach statement per change (project-context rule); every gate uses `isDesktop` /
  `isMobile` from `src/platform.ts`, no ad-hoc user-agent checks.

**Out of scope:** the license gate itself (13-4), statistics port (candidate, not cut). **DoD:**
JVM gate + `cargo test --lib` + `npm` type-check green; 📱 Andi opens Settings on the Xiaomi and
finds none of the gated controls; 🖥️📱 C9 controls gone on both.

### Story 13.4: Desktop license gate and one free tier

**Key `13-4` — Desktop license gate + one free-tier definition (G1a) · M + S**

**Rows:** D-C1 (M) · D-H1 (S) · the Command-gate key bug (`llm_priority` vs `llm_provider`).

As the klarvo maintainer,
I want the Desktop dictation pipeline to enforce the license exactly like Android, with one
free-tier definition (DeepSeek + Groq) on both platforms,
So that the paywall means the same thing wherever Klarvo runs.

**Acceptance Criteria (outcomes):**
- Desktop hotkey pipeline applies the same license decision as Android's runtime: unlicensed ⇒
  providers outside the free tier are not used; the result is visible (pill status or settings
  lock), never silent.
- One free-tier definition **DeepSeek + Groq**, shared by both twins (fixture-pinned like the twin
  constants). Android widens from Groq-only accordingly; the A2 lock text (13-3) matches.
- The Command gate reads `llm_provider`. Local cleanup is not overridden by the license (A1 lands
  in 13-5, but nothing in 13-4 may widen the gap).
- Rust + JVM tests pin the free-tier set and the gate decision on both paths; inversion RED.
- 🖥️ Andi removes his license key on Windows and dictates with OpenAI configured ⇒ same behaviour
  as on Android today (cleanup runs on a free-tier provider, the UI says why).

**Out of scope:** the software license choice (BSL/PolyForm) and the publication question —
untouched, see `docs/backlog.md` "Lizenzwahl OFFEN". Trial and activation rows (13-5).

### Story 13.5: Parity sweep: license rows and sync hygiene

**Key `13-5` — Parity sweep 3: license side rows + sync hygiene · S rows**

**Rows:** A1 (D-C2) · A3 immediate (D-M23, D-L30, D-L7) · A4 (D-L31, D-L32) · A5 (D-M19) · A6
(D-L9) · F1 (D-M3) · F3 (D-L25) · F4 D-L24 · F5 D-L28. **Depends on 13-4** (A5, A6 semantics).

As a klarvo user,
I want trial, activation and sync to behave the same on both platforms,
So that deleting a file never restarts a trial, a new license works without a restart, and sync
never claims success it did not verify.

**Acceptance Criteria (outcomes):**
- **A1** — Android exempts `local` from the license override (after G3b: stored values). 🤖 H+.
- **A3 immediate** — the paywall advertises only features that exist on the platform showing it.
  🖥️📱. The feature fate of Snippets / Cross-Device-Sync / Command-Mode stays open (not here).
- **A4** — a library load error is reported as such, not as "No API keys configured"; the dead
  second license entry point is removed. 🤖 H+ (rename the library, see the message).
- **A5** — trial start no longer derived from `config.json` (direction Android: install time or
  equivalent). 🤖 H+: delete `config.json`, no new trial.
- **A6** — Desktop recognizes a newly activated license without restart. 🖥️.
- **F1** — Android reads the Turso response before marking rows synced. **F3** — Android rejects
  `http://` Turso URLs like Desktop. **F4** — D-L24 date format aligned. **F5** — D-L28 atomic
  writers (ADR-0015 class). 🤖 H+ needs a reachable Turso DB + log visibility; if Andi has none,
  the story records Weg 2 for the F rows.
- Docs: D-L23, D-L26, D-L29 documented as no-action / asymmetry in the story (Amendment 4 list).

**Out of scope:** F2 (open, coupled to A3). **DoD:** tests green; 🖥️ A6 on Windows; 🖥️📱 A3.

### Story 13.6: Desktop banking guard (process blocklist)

**Key `13-6` — Desktop banking/password guard by process name (B1) · M**

As a klarvo user on Windows,
I want dictation into a banking or password-manager window to be blocked like on Android,
So that a dictated secret never lands in history, sync or the wrong field.

**Acceptance Criteria (outcomes):** a blocklist by process name on Desktop mirrors Android's
`BankingGuard` semantics (block paste, no history/Turso write, pill status says why); default
list and its source named in the spec; Rust tests for the decision; 🖥️ Andi focuses a
blocklisted process and dictates ⇒ blocked with a visible status. **Out of scope:** the Android
order fix (13-2).

### Story 13.7: Android per-app profiles and profile language

**Key `13-7` — Android per-app profiles + per-profile language (C4) · M + S**

As a klarvo user on Android,
I want per-app profiles to fire by package name, and the language set on a profile to apply on
both platforms,
So that the profile UI keeps its promise.

**Acceptance Criteria (outcomes):** profiles match on the foreground package (the `BankingGuard`
already knows it), not on a window title; `profiles[].language` is consumed by both twins
(fixture-pinned); 📱 Andi sets a profile for one app with another language and dictates there.
**Out of scope:** new profile fields.

### Story 13.8: Native preview blur wiring

**Key `13-8` — `previewBgBlur` drives the native preview overlay (C10) · M**

As a klarvo user on Windows,
I want the blur slider to change the native preview overlay,
So that the control does what it says (lifts the Epic-10 defer "blur slider no-op").

**Acceptance Criteria (outcomes):** the slider value reaches `native_preview.rs` and changes the
painted background (tiny-skia); ADR-0021 gets an amendment note (VR3 dropped blur; reintroduced
natively); objective pixel metric before acceptance (`feedback_verify_surface_fix_with_objective_pixel_metric`);
🖥️ Andi moves the slider and sees the overlay change. **Out of scope:** blur on Android.

### Story 13.9: OpenAI STT provider via Rust

**Key `13-9` — OpenAI as STT provider via the Rust core (D-H11) · S–M**

As a klarvo user on either platform,
I want `sttProvider = openai` to transcribe with OpenAI,
So that the setting is not silently Groq.

**Acceptance Criteria (outcomes):** provider switch in `stt/groq_jni.rs` (ADR-0017: STT only in
Rust), Kotlin passes the OpenAI key; `Adr0017BoundaryGuardTest` stays green; Rust tests for the
request shape; 🖥️📱 Andi sets OpenAI + key, dictates, the log shows the OpenAI endpoint and the
text arrives. **Out of scope:** any Kotlin STT code.

## Not in this epic (candidates without release, recorded in ADR-0016 Amendment 4 / backlog)

- C13 statistics **port** (Kotlin writes usage rows), M.
- G3b **build** Android local cleanup (native library, build, model path), L.
- A3 / F2 feature fate: Snippets, Cross-Device-Sync, Command-Mode (build or strike from the paywall).
- Rows judged asymmetry (D12 D-M13, F5 D-L29, VAD state machine M5) and documentation-only rows
  (G-Dok: D-L8, D-L17, D-L18, D-L20, D-L22; F4 D-L23, D-L26).
- 12-3, 9-8, 8-6, 8-7 keep their own numbers.

## L3 guards (carried into every story)

- **(G-A) Twin rule.** Every shared-behaviour change lands in both the Rust and the Kotlin path or
  is platform-gated; each story states its platform reach (project-context "Android bypasses
  Tauri IPC").
- **(G-B) Golden-vector net.** Guard and core-output rows get a fixture in `test-fixtures/` read by
  both a Rust and a JVM test; **inversion check RED at writing time** for every new vector or
  guard. `Adr0017BoundaryGuardTest` stays green; STT logic stays in Rust.
- **(G-C) Verification symmetry.** A story with an H+ row names the reproduction path before
  build; a downgrade to agent-only is written into the story, never implied. Human gates only
  where the human sees what the machine cannot (Epic 7 retro).
- **(G-D) Surface DoD.** Rows that change what Andi sees (pill, toast, settings) end with a
  Windows release build and/or a fresh APK on his Xiaomi (`feedback_smoke_test_dod_gate`).
- **(G-E) Standing decisions untouched.** No auto-send on Android; Groq never a cleanup fallback
  (Epic 12 FR2); license choice BSL/PolyForm not decided here; nothing built before its own go.

## Proposed order (critical path, sprint-planning may reorder)

13-1 → 13-2 (needs 13-1 for H+) → 13-3 (independent, visible on the Xiaomi) → 13-4 → 13-5
(needs 13-4) → 13-6 · 13-7 · 13-8 · 13-9 (independent). Rationale: 13-2 carries the user-visible
guard bugs (a dictionary term ≥ 10 chars deleted from every Android transcript), so it goes
right after its enabler.
