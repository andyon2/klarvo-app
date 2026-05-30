# BMAD-Autopilot — Escalation Contract (DRAFT, harvested)

**Status:** Draft, harvested from one real story (Epic 1 / Story 1.1).
**Purpose:** The decision policy a (hypothetical) autonomous BMAD-story driver would follow —
*what it resolves on its own* vs. *what it stops and asks the human about*. This is the single
load-bearing artifact: without it the driver either over-asks (useless) or under-asks (dangerous).

This contract is **grounded, not speculative.** Every AUTO / VERIFY / ESCALATE rule below cites the
concrete fork in Story 1.1 that produced it. Rules carried from up-front analysis but not yet
observed in a real run are marked **(not yet observed)** — they are hypotheses to confirm or kill on
the next harvest, not established policy.

> Why harvest instead of design up-front: the escalation boundary can only be drawn well after
> watching a real story trip over real forks. Story 1.1 (`save_atomic`) was chosen as the first
> harvest precisely because it hides a cross-platform correctness trap (Windows `rename`) and a
> manual DoD gate — the two failure modes most likely to separate a safe driver from a reckless one.

---

## The three bins

A driver classifies every fork it hits into exactly one of:

1. **AUTO-RESOLVE** — decide, log the rationale visibly, proceed. No stop.
2. **VERIFY-THEN-RESOLVE** — the choice depends on a fact the driver does not know for certain.
   It must *verify* the fact (docs, source, a probe), not guess — then resolve. No stop.
3. **ESCALATE** — stop and ask the human. The answer changes what gets built, or the step is
   physically impossible for the driver.

The hard part is not bins 1 and 3 — it is keeping bin 2 honest (don't let a guess masquerade as an
AUTO) and keeping bin 3 small (don't escalate what an ADR/spec already settles).

---

## AUTO-RESOLVE rules (driver decides, logs, proceeds)

| # | Rule | Evidence from Story 1.1 |
|---|------|--------------------------|
| **A1** | Implementation choice among options a **governing ADR explicitly sanctions**. Pick the simplest correct one; log which ADR clause covers it. | ADR-0015 §1 named *both* `tempfile`-`persist` *and* `ReplaceFileW`. Choosing `tempfile::persist` (no unsafe FFI, fewer LOC) is executing the ADR, not a fresh decision. |
| **A2** | Module / file **placement** derivable from the existing layout convention. | No shared util module existed; `src-tauri/src/fs.rs` (declared in `lib.rs`) is the obvious home for a helper two modules share — and ADR-0015 says all state files inherit it. |
| **A3** | Type / signature choices that **match the existing caller contract**. | `save_atomic` returns `anyhow::Result<()>` because `save_config`/`save_dictionary` already do, and every caller either `?`-propagates into anyhow or `.map_err(format!)`s. |
| **A4** | **AC-vs-anchor conflict**: when an AC's explicit requirement contradicts an illustrative "ref impl" anchor, the **AC wins** — and the driver must *verify the anchor actually does what the spec claims*, never mirror it blindly. | The AC's "ref impl" `llm_model.rs:249-258` does **not** `sync_all` (only `flush()`), yet the AC *requires* fsync. A driver that "mirrors the ref impl" silently drops the durability step. Anchors are illustrative, not authoritative. |
| **A5** | **Scope-line**: do exactly the AC, defer gold-plating whose **residual risk is strictly smaller** than the risk being fixed — with logged rationale. | AC requires temp-file fsync; full durability also wants parent-dir fsync after rename. Deferred: the residual risk ("durable old-but-complete file after power loss") ⊂ the risk being fixed ("truncated/empty file"). The AC's guarantee is fully met; the extra is a documented limitation. |
| **A6** | **Report-not-stop**: a pre-existing broken gate / unrelated defect discovered while working is *surfaced for triage*, not fixed under this story's clock and not a stop. | The subagent found `cargo clippy --lib -- -D warnings` already RED on `v1-ship` (19 errors in untouched files). Touched files are clean → story DoD met. Logged as a quick-dev candidate; not fixed here (would be scope creep into another story's time). |
| **A7** | **Review-nit handling**: review findings ranked *nit* (test-quality, no correctness impact) are the conductor's call — fix only if trivial, else document and defer. Do not spawn fresh work to chase a nit. | Two test-quality nits (test (d) error-source not pinned; `.parent()`-None guard untested) — documented in the story, not churned. Neither touches correctness. |

## VERIFY-THEN-RESOLVE rules (must verify a fact, must not guess; no stop)

| # | Rule | Evidence from Story 1.1 |
|---|------|--------------------------|
| **V1** | A choice that hinges on an **external-crate / platform internal semantic** must be *verified at the source* (docs/source/probe) before it is committed — especially when it gates a dependency or a correctness-critical path. | "Does `tempfile::persist` *atomically replace an existing* file on Windows, and does it sync?" was load-bearing for adding the dependency. Verified via docs ("atomically replace it"; "neither contents nor directory are synchronized") *before* committing — not assumed. |

## ESCALATE rules (driver stops, asks the human)

| # | Rule | Status in Story 1.1 |
|---|------|----------------------|
| **E1** | **Manual-gate DoD the driver physically cannot execute** — a real Windows/Android release build + on-device manual test. Unconditional stop; package the build + the *exact* test to run, then hand off. | **FIRED.** NFR-W: Windows `MoveFileExW`+`MOVEFILE_REPLACE_EXISTING` atomicity must be confirmed on a real Windows release build. Linux can't validate it. **The review step paid for itself here**: it converted the vague gate into a precise 3-check handoff (normal kill-cycle / locked target / read-only target) — a good escalation package is *specific*, not "go test Windows". |
| **E2** | A **new normal dependency / unsafe FFI / supply-chain expansion NOT pre-sanctioned by an ADR**. (BYOK/no-telemetry product → the user cares about supply chain.) | **NEAR-MISS — did not fire.** Promoting `tempfile` from dev- to normal-dependency *would* trigger this, but ADR-0015 §1 pre-sanctioned it (→ demoted to A1). The rule stands for the un-sanctioned case. |
| **E3** | A decision that **changes scope, a public contract, or architecture beyond the story boundary** — e.g. would require touching another epic or a fenced item. | (not yet observed) |
| **E4** | The **AC is genuinely ambiguous or self-contradictory** with no ADR/spec tiebreaker. | (not yet observed — 1.1's AC-vs-anchor conflict was resolvable by A4, so it did *not* escalate.) |
| **E5** | A **behavior change is required to make a test pass**. The test encodes an invariant; going green by changing behavior needs human sign-off. | (not yet observed — anticipated for Epic 3's seam-extraction stories.) |
| **E6** | **Repeated automated failure** (build/test) past a small attempt budget with no clear fix — escalate rather than thrash. | (not yet observed) |
| **E7** | **Touching a do-not-do fence** in the planning artifacts (DIV-06..14, `load_config` decoupling inside the Config epic, re-deriving §0 triage). | (not yet observed — the fences held; nothing in 1.1 approached them.) |

---

## What this run proved about the *mechanism* (not just the contract)

- **The conductor must be the main loop.** Only it can stop-and-ask the human mid-run. The Workflow
  tool can't ask mid-run; subagents report to the conductor, not the user. So: conductor = main loop
  (fixed on the session model), subagents = the hands.
- **Subagents must surface forks, not bury them.** The whole contract depends on the conductor
  *seeing* each fork. A subagent that silently picks defeats the harvest — and would defeat the
  autopilot. Implementation-subagent prompts must demand an explicit "decisions & assumptions I made"
  report so the conductor can classify each into the bins above.
- **Model choice per step is real and cheap:** recon = Sonnet (read-only), verify = small/fast,
  implementation-to-spec = Sonnet, review = fresh-context Opus (BMAD-prescribed). The conductor
  can only set the model on *delegated* steps — another reason judgment stays with subagents.
- **The expensive-looking primitives fight the requirement.** A background Workflow can't honor
  "only contact me when needed"; a custom subagent type can't reach the human. The thin
  conductor-plus-subagents shape is not a compromise — it is the only shape that satisfies all three
  of {context protection, model-per-step, human-escalation-mid-run}.

## Open question for the next harvest

Whether a subagent can *run a BMAD step-file skill* (`create-story`/`dev-story`) without losing its
interactive checkpoints (a subagent can't reach the human, so it would auto-answer every menu). This
run side-stepped it: the conductor treated the code-grounded epics.md story as dev-ready and
delegated only the mechanical implementation. If skills can't be safely sub-agented, the autopilot's
granularity is fixed: conductor runs skill orchestration, subagents do context-heavy sub-work.

---

*Harvested 2026-05-30 during Epic 1 / Story 1.1 (`save_atomic`). Update on each subsequent story:
promote (not yet observed) rules that fire, kill rules that never do, add new bins if a fork fits
none.*
