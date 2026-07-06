# GATE-4 Verdict — Story 9-14 HOLD-mode (B-Sprache REBUILD) — 2026-06-30

Build c431ba5 (dev ce20bb0 + code-review fixes A/B/C). **SUPERSEDES the 2026-06-26 evidence**,
which described the now-rejected slide-track implementation (HOLDDOCK 206x96dp, hint-arrows,
drag-LEFT>60dp, 24 tests). This is the grow-on-target B-Sprache surface (939x1076 window, 113 tests).

## Machine layer (conductor-verified) — PASS
- 113 JVM tests, 0 failures (13 new HoldTargetTouchZoneTest); KlarvoTheme drift gate PASS; APK built.
- Structure (emulator, dumpsys): HOLD state = panel 1080x525 (bottom) + HOLD-targets window 939x1076,
  both APPLICATION_OVERLAY, NOT_FOCUSABLE (no FLAG_NOT_TOUCHABLE), shown=true alpha=1.0.
- Fix A: no default-dock regression (window fits y=619..1695 on 2400). Fix B/C: no structural effect (verified by code review).
- Coarse visual: correct B-Sprache surface, correct colors, no slide-track residue.

## Code review
3 independent reviewers (Blind/Edge/Auditor) → A/B/C confirmed+fixed (commit c431ba5), 1 dismiss
(lockHoldToCluster false-positive), 7 defers to backlog (1 transient-frame + 6 GATE-4 fidelity items).
Convergence in one fix round, bounded re-review clean.

## Residual = Andi's real-device gate (unlocked, live mic) — the ACTUAL GATE-4
Emulator is a structural oracle only (contract visual_oracle.pixel=false). Andi's batched device gate must confirm:
  a) Motion/touch: hold=record · release-no-target=send · drag onto Abbrechen+release=cancel ·
     drag UP onto Sperren+release=lock→TAP-surface · pull-back-before-release=undo.
  b) grow-on-target: target grows + glows (red/teal) + label switches on finger-hit (AC3/AC4).
  c) Fix A engages: at a LOW dock position the Abbrechen target stays on-screen + tappable.
  d) Readability/fidelity at device scale; AC2 no chip↔target overlap.
  e) Deferred fidelity items (backlog 9-14): drag ghost-bubble + origin-fade (bHit), live-caption
     update on hit, .heldbub .finger indicator, inner amber ring, caption clip. Andi decides build-vs-accept.
