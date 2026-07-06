# GATE-4 Evidence — Story 9-5 (Bubble State Sequence + Listening Panel)

Run: story-conductor (Opus), 2026-06-16. Commit: `6d5791c`. Proxy: headless WSL emulator `emulator-5554` (arm64-v8a install via native-bridge). Drive: `DEBUG_SET_STATE` harness (9.4).

## Structural assertion (unattended GATE — `dumpsys window windows`, klarvo overlay windows only)

| State | klarvo overlay windows | Requested sizes | Verdict |
|---|---|---|---|
| idle | 1 | bubble 162×162 | ✓ |
| recording | **2** | panel **1080×525** + bubble **162×162** | ✓ panel + bubble, **no 3rd window** |
| transcribing | **2** | panel 1080×525 + bubble 162×162 | ✓ panel persists (AC3, no collapse) |
| done | 1 | bubble 162×162 | ✓ panel collapsed (AC4) |

**GREEN.** Window counts/sizes match the project-context expectation exactly (recording = panel ~1080×525 + bubble 162×162). **No double-window regression** — the old FloatingBubbleView recording-bar does NOT reappear as a 3rd overlay (this was the real 2026-06-15 defect). Raw dumps: `raw-<state>.txt`; per-state assertion: `structure-<state>.txt`.

## Proxy pixel sanity (NON-authoritative — emulator ≠ HyperOS; informational only)

- `ist-recording.png`: panel shows teal K-badge + amber live-dot + `0:03` timer + red Abbrechen square (right). Amber present.
- `ist-transcribing.png`: panel shows teal K + spinner + "Bereinigt…"; **amber dot gone, red square gone** (right side empty) — confirms the F1 fix (amber = RECORDING only; AC3) at panel level.
- (Both screenshots carry an emulator "System UI isn't responding" ANR dialog — emulator artifact, not the app. Overlays were NOT force-hidden in this run.)
- The bubble's own recording-state canvas (teal squircle + amber pulse-ring + send-glyph) is a 162×162 corner window — present structurally, not clearly resolvable at this proxy resolution.

## Residual for Andi — REAL-DEVICE visual/interaction gate (the binding visual verdict)

Emulator pixels are NOT trustworthy for HyperOS (skin/compositing/overlay force-hide; a past emulator "green" was broken on the real Xiaomi). On the real device, confirm:
1. **Bubble recording visual:** during recording the corner bubble shows the teal squircle + amber pulse-ring + send (paper-plane) glyph — NOT the idle "K".
2. **No amber on the bubble during transcribing** (teal squircle + send-glyph, pulse stopped).
3. **Interaction:** tapping the bubble = Senden (stop→clean→paste); the panel red square = Abbrechen (discard, no paste). Red is never wired to send.
4. **No double-window** on the real device (exactly one recording overlay form = the panel; the bubble stays a small corner squircle).
