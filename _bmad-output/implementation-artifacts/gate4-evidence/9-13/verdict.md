# GATE-4 Evidence — Story 9-13 (Recording-Cluster Order Swap: ✗ Cancel LEFT · waveform · ➤ Send RIGHT)

Run: story-conductor (Opus), 2026-06-26. Commit range: `2f5c242..e6f8d0b` (impl `6f6903c`). Branch `conductor/epic-9`.
Proxy: headless WSL emulator `emulator-5554` (arm64-v8a install via native-bridge). Drive: `DEBUG_SET_STATE` harness (Story 9.4).
Conductor ran the smoke itself under `BMAD_CONDUCTOR=1` (real Xiaomi detached by `conductor-guard acquire` — workers/conductor physically cannot install on it).

## Build / test (reproduced independently — not trusting the dev worker's claim)

`BMAD_CONDUCTOR=1 scripts/android-smoke.sh` → **exit 0** (`smoke-run.log`):
- KlarvoTheme.kt drift-gate: **in sync** with canon `klarvo.css` (no hand-edit drift).
- 17 production Kotlin files synced; **24 JVM unit tests, 0 failures**.
- Universal debug APK built; installed on `emulator-5554`; versionName 0.5.0 (AI-1 freshness gate passed).

## Structural assertion (unattended GATE — `dumpsys window windows`, klarvo overlay windows only)

RECORDING state (`structure-recording.txt`, raw `raw-recording.txt`):

| Window | klarvo overlay | Requested size | Gravity / frame | Verdict |
|---|---|---|---|---|
| #7 `b9682b1` | panel | **1080×525** | BOTTOM CENTER_VERTICAL | ✓ passive panel |
| #8 `5ae0613` | cluster | **435×178** (= 166×68dp @ 420dpi) | TOP START, frame `[0,880,435,1058]` | ✓ cluster window |

**GREEN.** Exactly **2** klarvo overlay windows (panel + cluster), **no 3rd window** → no double-window regression. The cluster window is the expected 166×68dp ADR-0019 §4′ size — the swap did **not** change window geometry (it is a within-window canvas re-order), so the structural oracle confirms *no regression*, by design it cannot itself see the L/R order.

## Visual corroboration of the swap (`ist-recording.png`) — VALID here, exceptionally

Normally emulator pixels are non-authoritative (HyperOS skin/compositing). **But the button L/R position is pure app-side Canvas geometry (`cancelCx = clusterLeft+…` LEFT, `sendCx = clusterRight−…` RIGHT) — OS-skin-independent.** The screencap therefore *does* corroborate the order:

- Cluster shows, left→right: **✗ Cancel (red/Danger square) — amber waveform — ➤ Send (teal square, paper-plane glyph)**.
- Matches canon `[✗ cancel · waveform · ➤ send]` (Klarvo Design System.html l.744-751) and ADR-0019 §4′-Amendment #2 exactly.
- **Colors NOT swapped:** ✗ stays red, ➤ stays teal — only positions moved (binding DT5 rule held).

## Code-review (3 independent Opus reviewers — Blind / Edge / Auditor)

CLEAN. All ACs (AC1-AC5, AC7) CONFIRMED; predicate operators ↔ zone-field assignments mutually consistent (`isTouchInConfirmZone: touchX >= clusterSendZoneStart` RIGHT; `isTouchInCancelZone: touchX <= clusterCancelZoneEnd` LEFT); field rename complete (grep: 0 stale refs in code, generated tree in sync); **no scope violation** (waveform machinery / KlarvoOverlayService / tokens / Rust untouched). 2 Low pre-existing findings deferred (unbounded outer zones; coordinate-as-sentinel idiom — predate this story).

## Tap-injection probe (inconclusive — expected; NOT a defect)

`input tap` at the right Send zone (x=380,y=969) produced **no state transition** (`ist-after-right-tap.png` = unchanged cluster, timer advanced 0:03→0:04). This is the known unreliability of synthetic taps onto a harness-driven `APPLICATION_OVERLAY` (the overlay touch path is not exercised by `DEBUG_SET_STATE`), **not** evidence of broken routing. The dispatch consumer (`KlarvoOverlayService.kt:1137-1144`, Confirm→`stopAndProcessRecording` / Cancel→`cancelRecording`) is **unchanged** by this diff and was re-read as correct.

## Residual for Andi — REAL-DEVICE gate (the binding verdict)

The L/R geometry + colors are corroborated above (OS-independent). What remains Andi's real-device, live-mic confirmation:
1. **Touch routing with a real finger:** tapping the **right** button = Senden (stop → clean → paste); tapping the **left** button = Abbrechen (discard, no paste). Red is never wired to send.
2. **HyperOS fidelity:** overall cluster look on the real skin (blur/compositing/overlay dimming — never an emulator screenshot per `reference_hyperos_overlay_quirks`).
3. Thumb-reachability of ➤ Send at the dock spot (the intent of the swap).
