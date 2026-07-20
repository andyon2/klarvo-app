# GATE-4 verdict — Story 11-3 (Android Preview-Box: fixed rolling window + z-order split)

Date: 2026-07-07 · Conductor: bmad-story-conductor (interactive) · Range: `88bc48c..152ed10`
Scope: AC-1..AC-5 (AC-6 z-order split out to Story 11-4 per GATE-2 decision).

## Proxy surface
Headless WSL emulator `emulator-5554` (AVD `klarvo-emu`, stock Android — NO HyperOS skin, density 420),
`BMAD_CONDUCTOR=1`, APK v0.5.0 built + installed via `android-smoke.sh` (Kotlin-only path valid —
11-3 touches no React/frontend). Service woken via `DEBUG_SET_STATE` harness
(`--include-stopped-packages`), states driven via broadcasts.

## Structural assertion (the machine-checkable GATE) — PASS

`dumpsys window windows`, recording state, driven with a SHORT vs a LONG transcript to prove the
panel window no longer grows with content (the fix for the "fills the screen → device unusable" bug):

| | Panel window (#preview) | Cluster window (➤/✗ controls) |
|---|---|---|
| SHORT ("Hallo Welt") | `Requested w=1080 h=525`, gr=BOTTOM, frame `[0,1812,1080,2337]` | `435x178`, frame `[351,1200,786,1378]` |
| LONG (~250 chars) | `Requested w=1080 h=525` (**identical**), gr=BOTTOM, frame `[0,1812,1080,2337]` | `435x178`, frame `[0,1200,435,1378]` |

- **Panel height is CONSTANT 525px regardless of transcript length.** 525px = 200dp × 420/160 =
  `PANEL_FIXED_HEIGHT_DP` exactly. Under 11-2 (WRAP_CONTENT) this window grew with content toward
  full-screen; now it is pinned. **AC-3 (fixed-height rolling window) confirmed at the WindowManager
  level.** ✅
- **Two `APPLICATION_OVERLAY` windows, no doubling/missing, no structural regression vs. 11-2.** ✅

## Bonus finding for Story 11-4 (z-order) — no overlap at fixed height
Cluster bottom edge (y=1378) sits ~434px ABOVE the panel top edge (y=1812) → **panel and the ➤/✗
control cluster do NOT overlap** with the fixed-height panel. The original "controls unreachable"
symptom came from the WRAP_CONTENT panel growing UP past the cluster; a pinned 200dp panel cannot.
This suggests **11-4 may be resolvable by positioning / may even be moot** — to be confirmed on the
real device (keyboard insets + HyperOS geometry may differ from stock emulator). Do NOT build the
a11y-reparenting change before Andi's device measurement.

## NOT observable on this proxy — Andi's real-device gate (Xiaomi/HyperOS), using a FULL build
1. **Visual/pixel:** the rolling window READS correctly — newest line pinned at the bottom, older
   lines roll out the top with a soft (not abrupt) fade; header "Live-Preview", no footer caption,
   no grip line, font 13/15/18sp.
2. **Real-STT pacing:** the fixed 5-line height + fade feel right at real dictation speed (first-pass
   numbers, device-tunable per OPEN ITEMS).
3. **HyperOS overlap:** confirm the panel does not cover the bubble controls on the real device
   (this is the 11-4 go/no-go input).

## Verdict
Structural/mechanical layer **GREEN** (fixed height proven; no regression). Visual + real-STT +
real-device overlap = Andi's batched device gate. Story stays at `review` until Andi's confirmation;
then close-out flips both status fields to `done`. Evidence: `structure-recording-{short,long}.txt`,
`ist-recording-long.png`.
