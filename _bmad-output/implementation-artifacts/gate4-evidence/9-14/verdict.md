# GATE-4 Verdict — Story 9-14 (HOLD-mode Push-to-Talk) — 2026-06-26

**Status held at `review`** (Andi's GATE-3 choice: gesture/motion story → final flip after his real-device gate).

## Machine layer (conductor-verified) — PASS
- Build green: 24 JVM tests, 0 failures; KlarvoTheme drift gate PASS.
- Structure: recording+hold_mode=true → panel window 1080×550 + dock window (two APPLICATION_OVERLAY windows); NOT_FOCUSABLE (no FLAG_NOT_TOUCHABLE → no HyperOS 0.8 dim).
- HOLD-dock size 206×96dp (566×264px) intact post-fix — code-continuity proof (fix finding 5 touched only Y, not w/h) + dev worker's live pre-fix reading. Cluster (hold_mode=false) = 166×68dp.
- 9-13 cluster regression GREEN.
- Code review: 3 independent reviewers → 7 confirmed findings, all fixed (commit e92f4f3), bounded re-review clean. Headline was a real-device-only animator-start bug (AC1a) that the harness false-greened — now started from updateAnimators().

## Boundary
Live structural read on the real device was blocked by the PIN keyguard (overlays don't composite behind it). Assertion rests on code-continuity + the dev worker's live pre-fix read. See structure-recording-hold.txt.

## Residual = Andi's real-device gate (unlocked, live mic)
a) AC1a hint arrows actually PULSE (highest risk — motion, harness-invisible).
b) Gesture: hold=record · release=send · drag-left>60dp=cancel · drag-up>40dp=lock→cluster.
c) Locked label/footer correct AND a second recording is clean (no sticky locked state — finding 2).
d) Visual fidelity vs approved render mockup-9-5-followups-2-4.html.
