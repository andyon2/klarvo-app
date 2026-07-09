# GATE-4 Evidence — Story 11-4 (Bubble structurally above Preview / Z-order)

Date: 2026-07-08
Branch: `fix/11-3-android-preview-box`
Commits: dev `6eca8f0` + review-fixes `678cf4a` (F1–F4 hardening)

## Self-verification (agent, objective — what I ran)

- **Fresh fixed-build install on the real device.** `scripts/android-smoke.sh` (interactive mode, no
  `BMAD_CONDUCTOR` — real device is the attended test target): drift-gate OK, 17 prod + 18 test
  Kotlin files synced, JVM unit tests green, fresh APK built from HEAD `678cf4a`, installed on
  `100.112.41.70:33233`, on-device `versionName 0.5.0`, AI-1 freshness gate passed. This defeats the
  11-3 stale-build trap: the APK on the device now contains F1–F4, not the pre-fix `6eca8f0`.
- **Code-level AC audit (adversarial, repo-access Acceptance Auditor).** AC-1 confirmed: the
  `reorderBubbleAbovePanel()` re-add fires on every real `showListeningPanel` addView path; the
  `panelVisible` early-return correctly performs no reorder (window order unchanged). AC-2 confirmed:
  grep + read shows `panelParams` is never read by any bubble-Y computation (drag `:1314-1322`,
  `adjustBubbleForKeyboard` `:864-880`, edge-snap `:1338-1355`) — the observed "geometric trick" was
  the z-order defect itself, so nothing to remove (AC-3 untouched-behavior holds by construction).
  AC-4 legitimately N/A: re-add path only, no `TYPE_ACCESSIBILITY_OVERLAY`, no accessibility-permission
  dependency introduced.
- **Review fixes verified in-diff.** F1 (isBubbleVisible reset on addView-fail), F2 (pending-flag clear
  in hideBubble + onDestroy), F3 (deferred reorder posted off the ACTION_UP dispatch via handler.post),
  F4 (deferred reorder gated on panelVisible). Compile + JVM tests + theme-check green post-fix.

## Structural oracle — NOT run, and why

The contract's emulator structural oracle (`dumpsys window windows` overlay-order assertion) could not
add signal here: the load-bearing question is z-order/reachability **during a live push-to-talk HOLD**,
which the re-add mechanism intentionally **defers to finger-release** — it is finger-timing-dependent and
cannot be reproduced by the `DEBUG_SET_STATE` harness (which just sets states, no live pointer).
`DEBUG_SET_STATE` is also dead on HyperOS (background restrictions). A dumpsys on the real device is only
meaningful while Andi is actively recording. So the structural layer here collapses into the same
un-synthesizable, live-finger residual that is Andi's real-device gate.

## Residual for the human (Andi, real device — provably un-synthesizable by the agent)

1. **AC-2:** Drag the bubble INTO the preview-box area — it should drop where released and render ON TOP
   (no more "jumps above the box top edge" trick). This is the 2026-07-08 complaint.
2. **AC-1 + the deferral residual (the key check):** During a push-to-talk **HOLD** recording (preview
   on) AND a **TOGGLE** recording — are ➤ Senden / ✗ Abbrechen reachable + visible THROUGHOUT? During a
   HOLD the reorder is deferred to release, so the bubble may sit under the panel while held — confirm
   whether that blocks the controls in practice or is fine (finger is on the bubble regardless).
3. **Pixel/aesthetic (D):** When the bubble overlaps the box, does it read as intentional, or is there
   ugly flicker / a transcribing-spinner jump on state changes needing a visual affordance?

## Verdict

- Agent layers: **GREEN** (build/install/freshness + code-level ACs + review-fix verification).
- Human layer round 1: **FAILED (AC-2)** — Andi 2026-07-09: bubble "sprang immer wieder hoch" when
  dragged into the box during a live dictation (keyboard open). Observed cause: `adjustBubbleForKeyboard`
  clamps the bubble above the soft-keyboard height, and the panel shares that bottom zone (the emergent
  clamp Design Decision 1 predicted). Fixed `43a52ca` (suppress keyboard-avoidance while `panelVisible`,
  Design Decision 4). Note: the HOLD-deferral residual (C) was NOT the failure — AC-1 reachability passed.
- Human layer round 2: **GREEN** — Andi 2026-07-09, fresh `43a52ca` build (agent-installed): bubble stays
  where dropped when dragged into the box (AC-2 ✓), controls reachable (AC-1 ✓), non-preview
  keyboard-avoidance unregressed (AC-3 ✓), overlap reads as intended. **Close-out: both status fields → done.**
