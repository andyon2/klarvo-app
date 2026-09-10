# Sprint Change Proposal — Epic 7 re-cut after the September relevance audit

Date: 2026-09-10 · Author: Correct Course workflow · Requested by: Andi
Mode: Batch (the trigger and the evidence were settled before this run; one approval covers all edits)

---

## 1. Issue Summary

**Trigger story:** 7-2 (Android live auto-stop VAD-gate parity), closed `done` on 2026-09-10 and merged
to `v1-ship` (`3d7de0d`). Right after the close-out Andi asked whether the rest of Epic 7 still holds.
Two audit agents checked every remaining acceptance criterion of 7-5, 7-6 and 7-7 against the code on
`v1-ship` (`3d7de0d`). This run re-verified the load-bearing claims against today's tree.

**Issue type:** Plan staleness. Epic 7 was cut on 2026-06-10/12 and parked on 2026-06-13. Between the
cut and today, Epics 8–12 and Stories 7-1/7-2/7-3 changed the code the June stories aim at. The
remaining stories still describe June's code. No story was started against the stale cut.

**Problem statement.** Three of the six June rows in Story 7.5 are unreachable through any UI, one rests
on a false premise, one is speculative; Story 7.7 assumes a CI that does not exist and plans work that
is already ~40 % delivered by 7-1/7-2/7-3; Story 7.6 is still valid but waits on a product decision
that Andi has widened rather than answered. Running a conductor against these stories would burn runs on
targets that no longer exist.

**Evidence (verified against `v1-ship` on 2026-09-10).**

| Claim | Finding | Location |
|---|---|---|
| M9: DeepSeek URL lacks `/v1` on Android | Confirmed, two sites. Desktop uses `/v1/chat/completions`. DeepSeek accepts both hosts today, so this is drift, not an outage. | `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:164,198` vs `src-tauri/src/llm/mod.rs:719` |
| M13: AutoSend toggles read-then-hardcoded on Android | Premise is false on **both** platforms. Desktop declares the props but renders no toggle; Android reads the key and uses `false`. Dead-config class, not a routing bug. | `src/components/settings/ShortcutsContent.tsx:199-207` (props only), `KlarvoApi.kt:66-69,334-337` |
| M10 / M11 / L5 (blank-key trim, unknown `cleanupStyle`, `deviceId` default) | Unreachable via UI: Desktop trims keys, writes the style as an enum, and the Rust core fills `deviceId`. M11's stated Desktop behaviour is wrong: Desktop discards the whole config, it does not "reject" the key. | audit 2026-09-10 |
| M16: 50 ms pre-paste settle | No observed failure on Android. Speculative. | audit 2026-09-10 |
| 7.7 "CI runs the net on both platforms" | **No CI exists.** No `.github/`, no GitLab file. Kotlin tests run only through `scripts/android-smoke.sh`; Rust tests only by hand. | repo root |
| 7.7 shared fixture format + both harnesses | **Already delivered** by 7-1/7-2/7-3: `chunking-cleanup-vectors.json` (Rust↔Kotlin twin with inversion proof), `wav-rms-vectors.json`, `vad-gate-golden-vectors-7-2.json`. | `test-fixtures/` |
| 7.7 "vector asserts no Kotlin STT request path" | Still missing. 7-3 AC9 deleted the twins but left no guard against re-growth. | `7-3-…md` AC9 |
| 7.7 twin hardcodes | Still unlocked: Rust `DEFAULT_TEMPERATURE 0.3`, `DEFAULT_MAX_TOKENS 2048`; Kotlin `0.3`, `2048`, `CHUNK_THRESHOLD 400`, `CHUNK_TARGET_SIZE 350`. | `llm/mod.rs:473-474`, `KlarvoApi.kt:971-972,1000-1001` |
| 7.7 dead-config-cluster "lock current both-hardcode value" | Wrong remedy. Desktop **shows** these keys in the Advanced panel and ignores every input. Three default sets disagree: `llmMaxTokens` is 1024 (frontend), 2048 (runtime), 4096 (config). No Rust code outside `config/` reads `llm_temperature`, `chunk_threshold`, `stt_temperature`. Freezing this would cement a lying UI. | `src/components/AdvancedSettingsPanel.tsx:14-33,330-382`, `src-tauri/src/config/*.rs:76-141,197` |
| M12 (dictionary in Chat style) | Still open and still real: Desktop's Chat arm omits `{dict_section}`, Android appends the dictionary for every style. | `src-tauri/src/llm/mod.rs:126-131,228-254` |
| 7-2 residuals | 8 accepted review findings (test-claim accuracy, vacuous-pass guards, stale KDoc) plus 3 tooling traps in `android-smoke.sh` (no test-dir prune, plain `adb install -r` picks the x86_64 split without the Rust `.so`). | `docs/backlog.md` "Story 7-2 residuals", story file round 3 |

**Decisions Andi has already made (2026-09-10).** Re-cut Epic 7 through this workflow. Start no Epic-7
story before the re-cut. **Decisions Andi has NOT made:** M12; and the wider idea that the dictionary
should be one list across devices instead of one `dictionaryTerms` per device. Neither is decided here.

---

## 2. Impact Analysis

### Epic Impact

**Epic 7 can still be completed, but not as cut in June.** 7-1, 7-2, 7-3 stay `done` and untouched.
The epic goal (core-output determinism + no silently dead config key, ADR-0016 Amendment 1) still holds.
What changes is the remaining slice:

- **Story 7.5 is dissolved.** Only M9 survives, as one acceptance criterion in the new close-out story.
  M10/M11/L5/M16 leave the plan with a recorded reason. M13 moves to the dead-config decision (below).
- **Story 7.7 is superseded by a smaller close-out story (7.8).** Its shape changes from "build the net"
  to "close the net": the fixture format and both harnesses exist; what remains is the STT-boundary
  guard, the twin-constant lock, the 7-2 residuals, and the tooling traps that bite the story's own gate.
  The CI acceptance criterion is replaced by the two gates that actually exist. The dead-config-cluster
  lock is dropped from the story and becomes a product decision.
- **Story 7.6 stays, amended.** Still the M12 decision story. Its fallback (lock current behaviour,
  defer the code change) is now satisfied by 7.8, so 7.6 can wait for Andi's decision without blocking
  the fixture work. Andi's wider dictionary-sync idea is homed in the backlog as a story candidate, not
  bolted onto 7.6 (Reduktion vor Konstruktion: the M12 style question stays open whether or not the
  list is synced; the two are independent).

**Sequencing changes.** June said "7.7 runs last as the capstone". After the re-cut, 7.8 is
**independent** of 7.6 and runs **next**. 7.6 runs whenever Andi decides M12.

**Epic-close rule (recommendation).** After 7.8 is `done`, Epic 7 may close with 7.6 parked, if M12 is
still undecided at that point. Precedent: Epic 8 (8-6/8-7 parked) and Epic 12 (12-3 parked). A parked
decision story must not hold an epic open indefinitely.

**Other epics.** Epic 9 (in-progress, 9-6/9-8 parked) and the parked Epic-8 stories are untouched. No
epic becomes obsolete. No new epic is needed. The dictionary-sync idea, if Andi pursues it, is bigger
than one story (Turso schema on both sides, Rust push/pull, Kotlin pull) and would be cut separately.

### Artifact Conflicts

| Artifact | Impact |
|---|---|
| PRD | No separate PRD exists. The requirements authority for Epic 7 is `docs/cross-platform-drift-audit.md` (row IDs) + ADR-0016 Amendment 1 + ADR-0017. Both still hold. Nothing in them is contradicted; only the June story cut against them is stale. |
| `epics-cross-platform-parity.md` | Header re-scope note; 7.5 and 7.7 marked superseded (text kept for traceability); new Story 7.8; 7.6 amendment; sequencing paragraph; out-of-scope list extended. |
| `sprint-status.yaml` | Remove `7-5-…` and `7-7-…` (never had story files, same handling as old 7-4 in June); add `7-8-parity-net-close-out-and-twin-hygiene: backlog`; epic-7 header comment updated. |
| Architecture / ADRs | **No impact.** ADR-0016/0017 boundaries are reaffirmed, not moved. 7.8's STT-boundary guard is the mechanical enforcement of ADR-0017 that 7-3 AC9 promised. |
| UI/UX | **No impact from this proposal.** The dead-config decision (wire or remove the Advanced-panel fields) will touch UI later, but that is the decision's story, not this one. |
| `docs/backlog.md` | Update the dead-config note (line ~148, "locked by 7.7" is no longer true); add "Desktop Advanced settings: wire or remove" decision item incl. M13; add dropped rows M10/M11/L5/M16 with reasons; add dictionary cross-device sync as a story candidate; add `dictation-quality-audit.py` as tooling; update the M12 OPEN-DECISION pointer (stays 7.6). |
| `_bmad/custom/bmad-sprint-status.toml` | Routing hook: NEXT = Story 7-8 via `bmad-story-conductor`; drop the "no story before correct-course" lock. |
| Memory | `project_epic_7_rest_relevance_audit` gets the outcome pointer to this proposal. |

### Technical Impact

7.8 is test-heavy and code-light. The only runtime change is two URL strings in `KlarvoApi.kt` (M9).
Everything else is fixtures, JVM/Rust tests, KDoc, and two `android-smoke.sh` corrections. No Rust
core, no JNI, no config schema, no UI. Kotlin tests run device-free (project-context rule); the
emulator gate is only needed to prove the `android-smoke.sh` install fix itself.

---

## 3. Recommended Approach

**Option 1 — Direct Adjustment. Selected.**
Modify stories inside the existing epic: dissolve 7.5, supersede 7.7 with 7.8, amend 7.6. Effort
**Low** (planning) + **Low–Medium** (7.8 implementation). Risk **Low**: the runtime delta is two URL
strings; the rest is test net and tooling.

**Option 2 — Rollback: not viable.** Nothing to roll back. 7-1/7-2/7-3 met their criteria and carry
evidence; the drift is in the plan, not in the code.

**Option 3 — MVP review: not applicable.** No product goal is threatened. The epic's requirements
sources still hold; the June stories simply over-described the remaining work.

**Justification.** The audit is unambiguous where it matters (unreachable rows, no CI, ~40 % already
delivered) and explicitly undecided where a human owns the call (M12, dictionary sync, dead settings).
The re-cut puts the mechanical remainder into one small story a conductor can run tonight, and homes
every human decision in the backlog with its evidence, so no conductor run stalls on a taste call.

---

## 4. Detailed Change Proposals

### 4.1 — `epics-cross-platform-parity.md`: header note (after the PARKED banner)

```
> **RE-CUT 2026-09-10** (`sprint-change-proposal-2026-09-10.md`): resumed 2026-08-10 (7-1), 7-2 done
> 2026-09-10. A relevance audit of 7.5/7.6/7.7 against `v1-ship` (`3d7de0d`) found the June cut stale.
> **7.5 dissolved** (only M9 survives, in 7.8). **7.7 superseded by 7.8** (net close-out; fixture format +
> both harnesses already exist from 7-1/7-2/7-3; no CI exists — gates are `scripts/android-smoke.sh`
> + `cargo test --lib`). **7.6 amended** (still the M12 decision; fallback lock delivered by 7.8).
> Sequencing: **7.8 next**, independent of 7.6. Epic may close with 7.6 parked if M12 is undecided.
```

### 4.2 — Story 7.5: superseded banner (text kept)

```
## Story 7.5: Android LLM-routing contract hygiene — ⛔ SUPERSEDED 2026-09-10

> Dissolved by `sprint-change-proposal-2026-09-10.md`. **M9 → Story 7.8.** M13 → backlog "Desktop
> Advanced settings + AutoSend: wire or remove" (premise false: no AutoSend toggle exists on either
> platform). M10/M11/L5 → dropped, unreachable via any UI (Desktop trims keys, writes the style as an
> enum, Rust core fills `deviceId`). M16 → dropped, no observed failure. Original text below for
> traceability only.
```

### 4.3 — Story 7.6: amendment (appended to the story)

```
> **Amendment 2026-09-10.** Still valid: Desktop's Chat arm omits `{dict_section}`
> (`llm/mod.rs:228-254`), Android appends the dictionary for every style. Two changes:
> (1) The fallback clause ("lock current behaviour as a golden-vector") is **delivered by Story 7.8**,
> so 7.6 is now purely: Andi decides M12 → both platforms agree → the 7.8 vector is flipped to the
> decided behaviour. (2) Andi's wider idea — one dictionary across devices instead of one
> `dictionaryTerms` per device — is homed in `docs/backlog.md` as a story candidate. It is
> **independent** of M12 (a synced list still needs the style decision) and is NOT part of 7.6.
> If M12 is still undecided when 7.8 closes, Epic 7 may close with 7.6 parked.
```

### 4.4 — Story 7.7: superseded banner (text kept)

```
## Story 7.7: Golden-Vector parity net (C1-proper) + dead-config lock — ⛔ SUPERSEDED 2026-09-10 by 7.8

> The June plan assumed a CI and an empty fixture directory. Neither holds: no CI exists, and 7-1/7-2/7-3
> delivered the shared fixture format and both harnesses. The dead-config-cluster lock is dropped:
> Desktop shows those keys in the Advanced panel and ignores them, with three disagreeing default
> sets — freezing that would cement a lying UI. → backlog decision "wire or remove".
> `scripts/dictation-quality-audit.py` → backlog tooling. Original text below for traceability only.
```

### 4.5 — New Story 7.8 (inserted after 7.7)

```
## Story 7.8: Parity-net close-out + twin hygiene *(new 2026-09-10, supersedes 7.5 + 7.7)*

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
```

### 4.6 — `sprint-status.yaml`

```
OLD:
  # Epic 7 — Cross-Platform Parity — ⏸ PARKED 2026-06-13 (…); resume mit 7-2 (7-1 done 2026-08-10), 7-7 ZULETZT
  epic-7: in-progress
  7-1-android-chunking-parity-core-output: done
  7-2-android-live-auto-stop-vad-gate-parity: done
  7-3-shared-core-stt-request-and-guard-path-via-jni: done
  7-5-android-llm-routing-contract-hygiene: backlog
  7-6-m12-open-product-decision-dictionary-in-chat-style: backlog
  7-7-golden-vector-parity-net-and-dead-config-lock: backlog
  epic-7-retrospective: optional

NEW:
  # Epic 7 — Cross-Platform Parity — RE-CUT 2026-09-10 (sprint-change-proposal-2026-09-10.md): 7-5/7-7 superseded by 7-8; 7-6 waits on M12
  epic-7: in-progress
  7-1-android-chunking-parity-core-output: done
  7-2-android-live-auto-stop-vad-gate-parity: done
  7-3-shared-core-stt-request-and-guard-path-via-jni: done
  7-6-m12-open-product-decision-dictionary-in-chat-style: backlog
  7-8-parity-net-close-out-and-twin-hygiene: backlog
  epic-7-retrospective: optional
```

### 4.7 — `docs/backlog.md`

**(a) Replace the dead-config note (~line 148):**

```
OLD:
> The dead-config cluster's *current* state (both sides hardcode) is **locked by Epic 7 Story 7.7** …

NEW:
> The dead-config cluster is NOT locked (7.7 superseded 2026-09-10). It is a product decision — see
> "OPEN-DECISION — Desktop Advanced settings + AutoSend: wire or remove" below.
```

**(b) New decision item (next to the M12 OPEN-DECISION):**

```
### OPEN-DECISION — Desktop Advanced settings + AutoSend: wire or remove (2026-09-10)

**Source:** Epic-7 relevance audit, `sprint-change-proposal-2026-09-10.md`. Desktop's
`AdvancedSettingsPanel.tsx` renders `sttTemperature`, `llmTemperature`, `llmMaxTokens`,
`chunkThreshold`/`chunkTargetSize`, model/prompt overrides, `autoPaste`, `autoCapitalize` — and no
runtime code reads them (`grep` outside `src-tauri/src/config/` is empty). Three default sets disagree
(`llmMaxTokens`: 1024 frontend / 2048 runtime / 4096 config). Same class: `bubbleTapAutoSend` /
`bubbleLongPressAutoSend` (drift row M13) — Desktop declares the props and renders no toggle, Android
reads the key and uses `false`. **Decision for Andi:** per key, wire it (both platforms, twin) or remove
it from the UI and the config surface. Do not freeze it with a vector. Becomes one story after the
decision.
```

**(c) Dropped rows:**

```
- **Dropped 2026-09-10 (7.5 dissolved):** M10 (blank-key trim), M11 (unknown `cleanupStyle`), L5
  (`deviceId` default) — unreachable via any UI; M16 (pre-paste settle) — no observed failure.
  Re-open only on a real report. Source: `sprint-change-proposal-2026-09-10.md`.
```

**(d) Story candidate:**

```
### STORY-CANDIDATE — Dictionary shared across devices (Andi, 2026-09-10)

Today each device holds its own `dictionaryTerms`; two lists are kept in sync by hand. Existing
primitives: Turso history sync on both sides (`src-tauri/src/sync/mod.rs` push/pull; Kotlin
`KlarvoApi.pushToTurso` / `ensureRemoteTable`), Android settings pass through the Rust core
(`TauriActivity`). Reduction sketch (NOT approved to build): dictionary as a second Turso table, push
on save in the Rust core, pull on settings-open (Rust) plus a small Kotlin pull before recording.
Independent of M12. Needs its own cut (both platforms, schema) — not a 7.6 add-on.
```

**(e) Tooling:**

```
- **`scripts/dictation-quality-audit.py`** — commit the marker detectors from the 2026-06-12 evidence
  run; manual cadence over adb/Tailscale. Was a 7.7 sibling; 7.7 superseded 2026-09-10.
```

### 4.8 — `_bmad/custom/bmad-sprint-status.toml` (routing hook, lean)

```
ROUTING (2026-09-10 abends): Epic 7 RE-CUT per sprint-change-proposal-2026-09-10.md. LIVE = Story 7-8
(parity-net close-out + twin hygiene) via bmad-story-conductor; independent of 7-6. 7-6 waits on
Andis M12-Entscheidung (docs/backlog.md OPEN-DECISION M12); Epic 7 darf mit geparkter 7-6 schliessen.
7-5/7-7 sind superseded (nicht mehr im Ledger). PARKED: 8-3/8-4 superseded, 8-6, 8-7, 9-6, 9-8, 12-3.
Epics 1-6, 8, 10, 11, 12 done. FALLE 8-6/8-7: epic-8 vorher auf in-progress. STORY-KANDIDATEN ohne
Sprint-Eintrag: Desktop Token-Enforcement-Gate (backlog 'Epic-8-Retro AI-4'), Woerterbuch
geraeteuebergreifend (backlog STORY-CANDIDATE). OPEN-DECISIONS fuer Andi, nicht auto-entscheiden:
M12, Desktop-Advanced-Settings wire-or-remove, Lizenz. GATES in project-context.md;
Android-GATE-4-Proxy = Laptop-Emulator.
```

---

## 5. Implementation Handoff

**Scope classification: Moderate.** Backlog reorganisation (one story dissolved, one superseded, one
added, one amended) plus decision homing. No architecture change, no replan.

| Recipient | Responsibility |
|---|---|
| Correct Course (this workflow) | On approval apply 4.1–4.8: epic doc, sprint-status, backlog, routing hook; update the audit memory with the outcome pointer; commit. |
| `bmad-story-conductor` | Run Story 7-8 end-to-end (create-story → dev → review → smoke). GATE-1 has no open design question; the story is decision-complete. |
| Andi | (1) Approve this proposal. (2) GATE-4 for 7-8: one Android dictation with DeepSeek cleanup on the fresh APK. (3) Later, at his own pace: decide M12 and the Advanced-settings wire-or-remove question. Neither blocks 7-8. |

**Success criteria.** Story 7-8 exists in the ledger and is the only Epic-7 story a fresh session
routes to. Every dropped row carries a reason in the backlog. Every open human decision is homed
with its evidence. 7-1/7-2/7-3 remain untouched.

**Sequencing.** 7-8 next, independent of 7-6 and of Epic 9. 7-6 when M12 is decided. Epic-7 retro
after 7-8 (optional).
