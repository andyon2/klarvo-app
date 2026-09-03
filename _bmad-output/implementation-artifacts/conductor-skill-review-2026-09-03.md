# Conductor-Skill Review — Epic 8 lessons (AI-6)

- **Date:** 2026-09-03
- **Trigger:** Epic 8 retrospective, action item AI-6 (`epic-8-retro-2026-09-03.md`). Andi's proposal:
  re-examine the conductor skills against the retro's lessons; he is willing to change them globally.
- **Scope:** the two global skills the project uses for autonomous runs, plus the skill the
  epic-conductor calls at close, plus the per-project contract. Reviewed as they are on disk today:

| File | Last changed | md5 (first 8) | Lines |
|---|---|---|---|
| `~/.claude/skills/bmad-epic-conductor/SKILL.md` | 2026-06-17 | `47286bc2` | 143 |
| `~/.claude/skills/bmad-story-conductor/SKILL.md` | 2026-07-09 | `f4b466cf` | 218 |
| `~/.claude/skills/bmad-waypoint/SKILL.md` | 2026-06-12 | `25f76eff` | 256 |
| `_bmad/custom/bmad-epic-conductor.toml` (project contract) | 2026-08-19 | — | 93 |

  Correction to the retro text: the project does **not** symlink the conductor skills. The project's
  `.claude/skills/` holds no conductor entry. The global `~/.claude/skills/` copy is the only one, and it
  is **not version-controlled** (no git repo in `~/.claude`). Every earlier change to these skills
  survives only in memory notes.
- **Method:** each lesson (a)–(g) from AI-6 was checked against the skill text and against the run
  evidence of Epic 8 (`RUN-2026-08-21.md`, the 8-5 and 8-8 story records, the commit range
  `90d035e..cf93fd4`). Verdict per lesson: **honoured / violated / absent**. "Violated" means the skill
  text says the right thing and the run broke it. "Absent" means the skill text does not carry the rule.
- **This is a review. Nothing has been changed.** The proposed edits below are the change set for
  Andi's approval. Build follows approval.

## Verdict at a glance

| Lesson | Verdict | Where the gap is | Proposed change | Size |
|---|---|---|---|---|
| (a) Retro not optional at close | **absent** | epic-conductor "End-of-epic close" and story-conductor GATE 3 both say "leave the retro to the human"; `bmad-waypoint` never names the retro as a next action | C1 — the epic close routes to the retro | 3 small edits |
| (b) No host mutation, no dead gates | **absent** (both halves) | workers dispatch with `--dangerously-skip-permissions`; no rule against installing packages; a gate that is red at `baseRef` is still a GATE-2 trip condition | C2 — worker sandbox rule + pre-existing-red rule | 2 edits + contract line |
| (c) Measurement names its claim, at worker level | **partially honoured** | conductor level: yes (E11, GATE-4 self-verification line). Worker level: the dev worker's return contract asks for "pass/fail" only; 8-8 wrote "18/18" without the scope qualifier | C3 — return contract + GATE-2 trip condition | 2 edits |
| (d) No unbooked working-tree work | **absent** | neither skill checks `git status` before `baseRef`; 8-8's dev worker found the feature already in the tree with no record | C4 — clean-tree assertion at the A→B boundary and at story receipt | 2 edits |
| (e) Re-review after every fix round | **violated at the cap** | the loop counts "fix→re-review iterations", but at `maxFixRounds` it ends on a fix: 8-8's round-2 fix `d8e7abb` (5 changes, incl. the StrictMode fix) was never reviewed — only smoked | C5 — the loop always ends on a review, never on a fix | 1 edit |
| (f) Phase A asks which response states the canon lacks | **absent** | Phase A validates coverage canon→story only; a canon that knows no click state passes trivially. That is exactly how Copy/Delete stayed silent through three review layers | C6 — response-state completeness check in Phase A step 5 | 1 edit |
| (g) Proxy-green is never a visual pass, as a worker rule | **partially honoured** | conductor and report: yes. Worker: no rule; the dev worker's story record called the proxy run "GATE-4 green" | folded into C3 | — |

Three further findings outside the seven lessons (F1–F3) and a list of what to keep unchanged follow
the per-lesson detail.

## Per-lesson detail

### (a) The retro runs in the same stroke as the epic close — absent

**Skill text.** epic-conductor, "End-of-epic close": *"Leave the `epic → done` transition and the
retrospective to the human. Note epic-complete and stop."* story-conductor GATE 3: *"If this was the
last open story in the epic, note it but leave the `epic → done` / retro to the human."*
`bmad-waypoint` treats the latest retro doc as a signal it *reads*; it never proposes the retro as the
next action when an epic has just closed.

**Evidence.** `epic-8-retrospective` sat on `optional` (the value sprint-planning assigns) from the
close on 2026-08-23 (`4ccb78f`) until 2026-09-03. The routing hook written at the close said "kein
Strang live" and did not name the retro. Andi's memory of the acceptance had faded by the time the
retro ran (retro lesson 1). `epic-7-retrospective` sits on `optional` today.

**Why the skills are the right place.** The skills are where the close is executed. The status value
`optional` is a BMAD default no skill may hand-edit (waypoint R1). The lever that exists is the routing
hook the waypoint already writes.

**Proposed change C1.**
1. epic-conductor, "End-of-epic close" — replace the sentence *"Leave the `epic → done` transition and
   the retrospective to the human"* with: *"Leave the `epic → done` flip to the human. The
   retrospective is not optional: when the last story of the epic is `done`, the waypoint's routing
   directive names `bmad-retrospective <epic>` as the NEXT action, before any new strand. Its human
   input decays daily."*
2. story-conductor, GATE 3 — after *"leave the `epic → done` / retro to the human"* add: *"and name
   the retro as the next action in your close-out note — it is the next step, not an option."*
3. `bmad-waypoint`, Step 2.1 (b) and Step 3 — add one audit line: *"If an epic has every story `done`
   and its `epic-N-retrospective` entry is not `done`, the aggregate must point at
   `bmad-retrospective` for that epic. Anything else is a hard gap."*

This is the same change AI-5 names for the contract. C1 puts it in the skills; AI-5 records the
project-side posture in the contract (one line: retro-at-close). Together they cover both the
conductor-driven close and a hand-driven close like `4ccb78f`.

### (b) No host mutation, no dead gates — absent

**Skill text (host).** story-conductor, "Worker dispatch": every worker is launched with
`claude -p --model <seat> --dangerously-skip-permissions …`. No rule in either skill forbids a worker
to change the host outside the working tree.

**Evidence (host).** 8-5, Debug Log: the dev worker ran `apt-get install gcc-mingw-w64-x86-64`, a
later fix worker `apt-get install g++-mingw-w64-x86-64` — two package installs on powerhouse by
unattended workers, to reach a gate that then failed anyway. 8-8 (RUN-2026-08-21, ledger 5): the
story-conductor chose an explicit `--allowedTools` grant plus `--permission-mode acceptEdits` instead
of the skip-permissions flag. That was an ad-hoc decision inside one run; the skill still says
skip-permissions.

**Skill text (dead gates).** story-conductor GATE 2 trips when *"tests or build are RED"*. Nothing
distinguishes a failure the story introduced from one that was already red at `baseRef`. The only
"stop chasing" rule is `maxSmokeStrikes`, which covers smokes, not build gates.

**Evidence (dead gates).** `cargo check --target x86_64-pc-windows-gnu` has failed at the same
`ort-sys` spot since 8-1. Three stories re-ran it, two workers installed packages for it, 8-8's Dev
Notes finally said "do not chase it". The retirement of that specific gate is AI-3 (project). The
generic rule belongs in the skill.

**Proposed change C2.**
1. story-conductor, "Worker dispatch" — replace `--dangerously-skip-permissions` in both launch lines
   with `--permission-mode acceptEdits --allowedTools "<the project's worker grant>"` and add the
   rule: *"**R6 — Workers stay inside the working tree.** A worker never installs system packages,
   never edits files outside the repo, never touches a device or host service the contract does not
   name. A gate that needs a tool the host lacks is reported as `blocked` (autonomous) or as a
   residual — never reached by mutating the machine."* The concrete grant list is a contract value
   (`[workers].allowed_tools`) so each project sets it; the 8-8 run is the template. Reconstruct the
   exact list from that run at build time.
2. story-conductor, GATE 2 trip conditions — add: *"A gate that was already red at `baseRef` (run it
   there, or take the project's documented baseline) is **not** a trip condition. Report it as
   pre-existing in one line and move on. Never dispatch a fix worker to chase it, never let a worker
   install anything to reach it. A gate red across two consecutive stories is retired through the
   project's DoD templates — a report item, not a re-run."*
3. Contract (`_bmad/custom/bmad-epic-conductor.toml`) — one new block `[workers]` with
   `allowed_tools = [...]` and `host_mutation = false`, plus the line AI-3 already calls for: the
   cross-compile check is retired. (Project side; listed here so the skill's reference resolves.)

### (c) A measurement names its claim — at worker level — partially honoured

**Skill text.** Conductor level is covered: epic-conductor E11 and the final-report section say
"proxy-green is build/install/logic/structural only, never a visual pass"; story-conductor GATE 4
demands the line `Self-verification: <what you ran + objective results>; residual for you: <the
unobservable part>`. Worker level is not: the dev worker's `EXPECTED_RETURN` is *"the test + build
result (pass/fail, verbatim key lines), a short summary of the diff, whether you committed …, and
any AC left unmet"*. No field asks what the run did **not** exercise, and no GATE-2 trip condition
fires on a bare count.

**Evidence.** 8-8, Change Log: the dev worker wrote "18/18 GATE-4 smoke". The round-1 review had to
add the qualifier that four of the five Copy sites were never clicked (story line 166: `[Review][Patch]
… claims "18/18" without the qualifier`). Epic 11's AI-3 asked for exactly this and was graded
"partial — held at conductor level, not at worker level" in the Epic 8 retro. `project-context.md`
rule 103 states the rule and the workers load it; it did not prevent the claim once.

**Why R1 does not block this.** R1 forbids injecting *method* into a worker prompt. The return
contract is the skill's own sanctioned channel — it already asks for SHAs and AC gaps. Asking for the
scope of a claim is the same kind of data.

**Proposed change C3** (covers (c) and (g)).
1. story-conductor, `EXPECTED_RETURN` for dev-story and fix — extend to: *"… and for every gate
   result that carries a number or the word green: what the run exercised and what it did **not**
   (sites, states, platforms never driven), and whether the result proves wiring, logic, structure, or
   design. A proxy render is never reported as a visual or design pass."*
2. story-conductor, GATE 2 trip conditions — add: *"the worker's return or its story record carries a
   count or a 'green' without stating what was not exercised, or calls a proxy result a visual pass.
   Do not proceed on it: rewrite the claim's scope in the story record (Dev Agent Record) before
   review, so the reviewer inherits the qualifier instead of discovering it."*

### (d) No unbooked working-tree work between workers — absent

**Skill text.** epic-conductor Phase B step 1 derives `baseRef = git rev-parse HEAD` and never looks
at `git status`. story-conductor, autonomous activation: *"do not stage anything — just record
`baseRef`"*. Neither asserts a clean tree.

**Evidence.** 8-8, Completion Notes: *"Found the implementation already substantially in place in
the working tree at story start (hook, 5 wired Copy sites, PreviewComments alignment, optimistic
delete + undo strip, refetch-safety flush) but with every task checkbox still unchecked and no Dev
Agent Record."* The git history has nothing between the story commit `90d035e` (08-19 18:43) and the
dev commit `48061be` (08-21 11:54). Who wrote that code, and from which decision, is recorded
nowhere — that absence is the finding. The review then diffed `baseRef..HEAD`, so the code *was*
reviewed, but its provenance is lost and the dev worker had to treat a finished feature as
"unverified in-progress work".

**Proposed change C4.**
1. epic-conductor, "PHASE A→B boundary" and Phase B step 1 — add before `baseRef`: *"`git status
   --porcelain` is empty. A dirty tree is a halt, not a warning: either commit it as its own booked
   step with a one-line origin (which decision, which session), or stash it and report it. Code
   written in Phase A for a mockup or a probe is committed or reverted before the boundary. Never
   launch a worker over unbooked changes."*
2. story-conductor, autonomous activation step 3 — add: *"Assert the tree is clean at receipt. Dirty
   → return `blocked` with the file list; do not adopt the changes as the story's own."*

### (e) A re-review after every fix round — violated at the cap

**Skill text.** story-conductor, "Autonomous budget": *"`maxFixRounds` (default 2) — fix→re-review
iterations. At the cap: gates green → `review-cleared` with the remaining findings as
`review.residual`."* "Gates green" means build and tests. The pipeline diagram ends the fix loop with
*"re-dispatch review, scoped to those findings"*, but the cap semantics let the sequence end on a
fix.

**Evidence.** 8-8's actual sequence: review 0 → fix 1 (`197ce13`) → review 1 (found the StrictMode
latch that fix 1 had introduced) → fix 2 (`d8e7abb`, five changes) → GATE 4 + close-out (`cf93fd4`).
The story record shows no review of `d8e7abb`. The 26/26 smoke ran on it, and a smoke checks
behaviour, not the diff. Retro lesson 4 in one line: *"a fix round without a re-review is a new
unreviewed change"*. The loop's own logic produced one.

**Proposed change C5.** story-conductor, "Autonomous budget" — replace the `maxFixRounds` sentence
with: *"`maxFixRounds` (default 2) counts **fix rounds**. Every fix round is followed by a scoped
re-review (landing + regression on the touched lines only — cheap). **The loop ends on a review,
never on a fix.** At the cap, the final re-review's findings become `review.residual`; no further
fix worker is dispatched. Gates red at the cap → `blocked`."* The same sentence goes into the
pipeline diagram's fix-loop line.

### (f) Phase A asks a re-skin which response states the canon lacks — absent

**Skill text.** epic-conductor Phase A step 5 validates *(a) coverage — every surface the canon
defines is covered by a story; (b) constraint-mirror; (c) anchored by reference*. Coverage runs from
the canon outward. A canon that defines no click response is complete by that test.

**Evidence.** The 8-5 canon knew no confirmed state for Copy and no in-place state for Delete. Three
review layers and the Chromium harness compared the re-skin against that canon and passed. Andi found
the silent controls in minutes (retro lessons 3 and key takeaway 1). Only Phase A of the 8-8 run then
added the states — after a correct-course, not before the build.

**Proposed change C6.** epic-conductor, Phase A step 5 — add a fourth validation: *"**(d)
response-state completeness** — for every interactive control on the surface (click, hover, focus,
submit, copy, delete, error, empty, loading, offline), does the canon define what the user sees in
response? List the controls; a control with no defined response is a design question for the human
**now**, with a mockup, and its answer is baked into the canon. A 'no behaviour change' rule does not
waive this check — a re-skin makes silent controls visible."* Same check as one line in the
story-conductor's interactive GATE 1 (a story-level run without the epic-conductor still gets it).

### (g) "Proxy-green is never a visual pass" as a worker rule — partially honoured

Covered by C3. Conductor-level text already carries the rule three times (E11, final report, GATE 4);
the worker's return contract and the GATE-2 trip condition are the two missing enforcement points.

## Findings outside the seven lessons

**F1 — The billing paragraph in the epic-conductor is stale and self-contradictory.** It says the
subagents are *"NOT `claude -p`"* and draw from the subscription *"not the separate Agent-SDK/headless
credits"*. The story-conductor it spawns dispatches every worker as a `claude -p` process. And the
separate pool does not exist: since 2026-08-18 the record says everything draws subscription usage
(memory `reference_claude_code_billing_subagent_boundary`). Proposed: replace the paragraph with two
sentences — all seats draw the subscription; if the login might be an API key, say so before a large
run. No behaviour change.

**F2 — The contract knows one proxy surface; Epic 8 was desktop.** `[smoke].command` is
`scripts/android-smoke.sh`; `[visual_oracle]` describes `dumpsys window windows`. Every desktop gate
in Epic 8 ran the puppeteer/Chromium harness against `npm run preview` — a precedent since 8-2, in no
contract block. The `[desktop_build]` block names `windows-build.sh` but not the machine gate. The
skill reads the contract as singular ("the smoke command + unattended proxy surface"). Proposed: skill
wording *"one proxy surface per platform the contract declares; pick by the story's platform"*, and a
`[smoke.desktop]` block in the contract (harness, port, evidence dir, what it decides / does not
decide). Project side; pairs with AI-2 (project-context) which has the same gap.

**F3 — The global skills are not version-controlled.** All three live only in `~/.claude/skills/`.
The 2026-06-17 redesign, the 2026-07-09 change, and this change set have no diff anyone can read
back. Cheapest remedy, if Andi wants one: `git init` in `~/.claude/skills` (private, no remote
needed) so every skill edit has a commit. No new construct, no copy elsewhere.

## What held — keep unchanged

- **Phase A as a human gate with the canon as the only SOLL.** RUN-2026-08-21: "No design question
  surfaced during Phase B". Keep E1/E2/E6 and step 4 as they are.
- **Structural-vs-pixel split (E11, GATE 4).** The run report named its residuals correctly. Keep.
- **Run-guard mechanics (E10).** Lock, HEAD-watch and detach worked (`expect cf93fd4` → exit 0).
  The env-flag alias (`BMAD_CONDUCTOR` with `KLARVO_CONDUCTOR` accepted) is fine in
  `android-smoke.sh` line 84. Keep.
- **Per-step commits + independent review of `baseRef..HEAD` (E4, R3).** The round-1 review caught a
  plausible-but-wrong fix. Keep.
- **Convergence discipline (scoped re-review, residual instead of grind).** Keep; C5 only fixes the
  endpoint.
- **Evidence dir + RUN-<date>.md.** The retro could reconstruct everything from it. Keep.

## Change set summary (for approval)

| # | File | Section | Lesson | Kind |
|---|---|---|---|---|
| C1.1 | epic-conductor | End-of-epic close | (a) | replace 1 sentence |
| C1.2 | story-conductor | GATE 3 | (a) | add 1 clause |
| C1.3 | bmad-waypoint | Step 2.1 / Step 3 | (a) | add 1 audit line |
| C2.1 | story-conductor | Worker dispatch + new R6 | (b) | replace flag, add rule |
| C2.2 | story-conductor | GATE 2 | (b) | add trip exclusion |
| C2.3 | contract (project) | new `[workers]` | (b) | add block |
| C3.1 | story-conductor | `EXPECTED_RETURN` dev/fix | (c)(g) | extend return contract |
| C3.2 | story-conductor | GATE 2 | (c)(g) | add trip condition |
| C4.1 | epic-conductor | A→B boundary + Phase B step 1 | (d) | add clean-tree halt |
| C4.2 | story-conductor | autonomous activation 3 | (d) | add clean-tree assert |
| C5 | story-conductor | Autonomous budget + pipeline | (e) | replace cap semantics |
| C6 | epic-conductor + story-conductor | Phase A step 5 / GATE 1 | (f) | add validation (d) |
| F1 | epic-conductor | billing paragraph | — | replace 1 paragraph |
| F2 | epic-conductor + story-conductor + contract | contract reading | — | wording + `[smoke.desktop]` |
| F3 | `~/.claude/skills` | — | — | `git init` (optional) |

Estimated delta: about 40 changed lines across the two conductor skills, 3 in the waypoint, one new
block plus one line in the contract. No rule is removed. No mechanic (run-guard, nesting, verdict
schema) changes shape; C3 adds fields to the return *text*, not to the verdict object.

## Decisions Andi has to take

1. **Approve the change set** — as a whole, or per item (C1–C6, F1–F3).
2. **C2.1 changes every future worker's permissions in every project** (skip-permissions → explicit
   grant). The 8-8 run proved it works for a desktop story. An Android story has not run under it yet.
   Approve as the default, or approve for klarvo only via the contract and leave the skill's default?
   Recommendation: skill default, because the host mutation happened under the old default.
3. **F3** — version the global skills or not.

## Next

After approval: edit the three global skills and the contract in one session, verify each edit
against this document, and record the new md5s here as a Change Log line. Then AI-1 → AI-2 → AI-3/4/5,
then Epic 7 with 7-2 — the first run under the changed skills. The first conductor run afterwards
is the acceptance test for this change set; its RUN report must show C4 (clean tree) and C5 (ends on
a review) in the ledger.

## Change Log

- 2026-09-03 — review written. No skill or contract edited.
