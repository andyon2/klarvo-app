# GATE-4 Evidence — Story 9-15 (Mobile TAP-Aufnahme-Surface, B-Sprache Re-Skin)

Run: bmad-story-conductor (Opus), 2026-06-30. Commit range: `ed219f10..218ee5d` (impl `a3d0233`, review-fix `218ee5d`). Branch `conductor/epic-9`.
Proxy: headless WSL emulator `emulator-5554` (stock Android, density 420dpi, arm64-v8a install via native-bridge). Drive: `DEBUG_SET_STATE` harness (Story 9.4) via `android-emulator-smoke.sh`.
Conductor ran the smoke itself under `BMAD_CONDUCTOR=1` (real Xiaomi `100.112.41.70:5555` detached by `conductor-guard acquire` — workers/conductor physically cannot install on it). Emulator stopped + device reconnected on release.

## Build / install (reproduced independently — not trusting the dev worker's claim)

`BMAD_CONDUCTOR=1 scripts/android-emulator-smoke.sh` → **exit 0** (`smoke-run.log`):
- 17 production Kotlin files + the new test synced; universal debug APK built fresh (**120 MB**) with the review-fix code (`218ee5d`).
- Installed `--abi arm64-v8a` on `emulator-5554`, no crash; grants + DebugHarnessReceiver + service wake all succeeded; state machine drove idle↔recording cleanly.
- (JVM unit gate already green in dev + fix: 97 tests, 0 failures; KlarvoTheme drift-gate in sync.)

## Structural assertion (unattended GATE — `dumpsys window windows`, klarvo overlay windows only)

RECORDING state (`structure-recording.txt`, raw `raw-recording.txt`):

| Window | klarvo overlay | Requested px | = dp @420dpi (÷2.625) | Verdict |
|---|---|---|---|---|
| #7 `621cbe8` | panel | **1080×525** | fillx × 200dp | ✓ passive listening panel, BOTTOM CENTER |
| #8 `e7d22f` | **TAP surface** | **892×582** | **340×222dp** | ✓✓ exact TAP-surface size (TAP_VISUAL_W_DP 320 + 2×10 pad) |

IDLE contrast (`structure-idle.txt`): **1** klarvo window `162×162` = **62dp** idle bubble.

**GREEN.** RECORDING shows exactly **2** klarvo overlay windows (panel + TAP surface), **no 3rd window** → no double-window regression.
The recording-state bubble window is **340×222dp** — unambiguously the new TAP surface, NOT the old small cluster (≈150×68dp; 9-13 measured the prior cluster at 166×68dp = 435×178px). The window grew from ~178px tall to **582px** → the new surface is **wired**, not merely "no regression". The structural oracle here confirms *the new surface is live*, which 9-13's same-size swap could not.

## Visual corroboration (`ist-recording.png`) — OS-skin-independent canvas geometry, VALID here

The circle positions + colors are pure app-side Canvas (`tapCircleCenters` + the TAP_* paints) — independent of the OS skin, so the stock-Android screencap *does* corroborate them (same exception class as 9-13):

- TAP surface, left→right: **Senden** (teal-gradient circle, ➤ paper-plane glyph, "Senden"/"tippen") — **amber waveform chip + 0:00 timer** (opaque dark, above, overlapping neither circle) — **Abbrechen** (dark fill + red ring + ✕ in danger-hi coral, "Abbrechen"/"tippen").
- The idle bubble was **left-docked** (idle window x=44) → Send rendered on the **left** = **dock mirroring works** (AC3 Send-at-dock + AC6 mirroring corroborated, consistent with the now-real `tapCircleCenters` JVM tests).
- **Colors NOT swapped:** teal=Senden, amber=live, red=Abbrechen — inversion gates hold (DT5 binding rule).
- Old `.ab-cluster` **not present** (AC1 inversion gate clears).

## Code-review (3 independent Opus reviewers — Blind / Edge / Auditor; conductor-triaged + self-verified)

CLEAN after one fix round. ACs confirmed; no inversion gate tripped. 2 confirmed patches applied (`218ee5d`):
1. AC6 dock-mirroring tests were **tautological** (point-in-own-center; `SEND_CX_LEFT==LEFT_CX` definitional) → extracted pure `tapCircleCenters` from `drawTapSurface` (behaviour-identical) + rewrote the 3 tests to exercise the real binding (left/right swap + tap-at-send-center inside-send-not-cancel).
2. Cancel glyph+label used `KlarvoTheme.Danger` (#EE6F63) instead of render `--k-danger-hi` (#F4897E) → local const `TAP_CANCEL_DANGER_HI` (KlarvoTheme untouched).
5 findings deferred to `docs/backlog.md` (per-frame allocations; cancel-label 13sp-vs-15px; window<340dp clamp; drag edge-snap; AC6 top/bottom mirroring). 6 dismissed (full triage in the story's "Review Findings").

## Residual for Andi — REAL-DEVICE gate (the binding verdict)

The emulator is stock Android (no HyperOS), software GPU — pixels/aesthetics are NOT authoritative here (`reference_hyperos_overlay_quirks`). What remains Andi's real-device, batched visual/touch confirmation:
1. **The original 9-14 rejection criterion:** does the surface now read as a real phone feature (not "laptop-small")? Target size / thumb-reach of the two large circles at true device scale.
2. **Real-finger touch routing (AC4):** tapping **Senden** = stop→clean→paste; tapping **Abbrechen** = discard, no paste. (Synthetic `input tap` is unreliable on a harness-driven `APPLICATION_OVERLAY` — not exercised here; the dispatch consumers `stopAndProcessRecording`/`cancelRecording` were re-read as correctly wired.)
3. **HyperOS fidelity:** circle/chip look on the real skin (blur/compositing/overlay dimming — never an emulator screenshot).
4. **Deferred fidelity nits on the real screen:** does "Abbrechen" fit at 13sp (vs SOLL 15px)? Is the danger-hi shade right? (Both in backlog — decide against the real device.)
