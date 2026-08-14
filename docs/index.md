# Klarvo Documentation Index

**Last Updated:** 2026-06-05
**Deep-Dives:** 1

> This index was created by the `document-project` workflow in **deep-dive mode** (no full project
> scan has been run). It currently lists only the deep-dive(s) plus existing hand-written docs. Run
> `document-project` → "Re-scan entire project" to generate a complete project map.

## Start Here

- [**Architecture**](./ARCHITECTURE.md) — the front door: current system map plus an index tying
  together the decisions and rules that live elsewhere (ADRs, the rules digest, the audit).
  Deliberately thin — it points to those authorities rather than copying them.

## Deep-Dive Documentation

Detailed exhaustive analysis of specific areas:

- [Floating Bar Subsystem Deep-Dive](./deep-dive-bar-subsystem.md) — Comprehensive analysis of the
  `"bar"` overlay window: all responsibilities, the 15 effect hooks, the full Tauri coupling
  (6 commands / 4 events / 0 emitted), and the 13-item race class. Fact-basis for bar remediation.
  (9 files: 2 owned + 7 coupling surfaces) — Generated 2026-06-05

## Existing Project Docs

- [Robustness Audit (2026-05-30)](./robustness-audit-2026-05-30.md) — audit/remediation context.
- [Surface Smoke Checklist](./surface-smoke-checklist.md) — running ledger of Linux-green traps for
  surface/UI stories.
- [Feature Ideas](./feature-ideas.md)
- [Remediation Session Kickoff](./remediation-session-kickoff.md)
- [Windows Rebuild Kickoff](./windows-rebuild-kickoff.md)
- [BMAD Autopilot Escalation Contract](./bmad-autopilot-escalation-contract.md)
- ADRs — [`docs/adr/`](./adr/) (0015 state-file writes, 0016 Android path parity).

## Project Context for AI Agents

- `_bmad-output/project-context.md` — the lean rules digest agents must read first.
