# WAYPOINT — Klarvo (v1-ship)

_Last session: 2026-06-07 (late). Written for a context-less BMAD session to continue with zero chat history._

## Resume — next BMAD action

**Story 6-6 is DONE (closed 2026-06-07).** The last open cosmetic bug — the teal border not closing on the RIGHT+BOTTOM edges — is **fixed and objectively verified on Windows**. `sprint-status.yaml`: `6-6 = done`. The fix is committed on `v1-ship`.

**What the fix was:** the preview card is stretch-aligned + bottom-anchored in a full-window flex wrapper, so its right/bottom border sat exactly on the window content boundary and got clipped at fractional DPI (dpr 1.5375). A **2px uniform `padding`** on the wrapper (`src/PreviewPanel.tsx` ~L332) gives all four borders room inside the window (and inside the `set_preview_shape` region). Confirms the earlier disproof of the GDI-region theory — that theory stays dead.

**How it was verified (objective, not eyeballed):** debug build via vite HMR, CDP-pinned the box open, DPI-aware screen-capture, counted teal-border px coverage per edge. INVERSION proved diagnosis + fix: `padding:0` → TOP/LEFT 100%, BOTTOM/RIGHT **0%**; `padding:2` → all four edges **100%**.

**Next BMAD action:** Epic 6 continues — 6-3 (font-SIZE axis, Increment B, same panel) and 6-4 (couple preview to pill drag), both depend 6-2 (done) and are parallelizable. Or run the Epic-6 retro. Use `bmad-sprint-status` / `bmad-create-story` to pick up the next story.

## What was PROVEN this session (do not re-litigate)

1. **The GDI window region is NOT the cause.** Removed the `set_preview_shape` call (so the preview window had `region=none`, verified non-visually via Win32 `GetWindowRgn` → ERROR/none) — and the right/bottom border was **still missing**. So the region the prior waypoint blamed is a red herring for this defect. (The region IS redundant on this transparent click-through window, so removing it is valid *cleanup*, but it is **not this fix**. It was reverted to keep the tree clean.)
2. **Real cause (HIGH confidence, NOT yet visually verified):** the card (`#preview-card`) is `width:100%` + `box-sizing:border-box`, bottom-aligned in a full-window flex wrapper (`justifyContent:flex-end`). So the card is **flush** to the window's right and bottom edges. At fractional DPI scale (dpr 1.5375 observed), the 1px border on the right/bottom falls at/just beyond the window's integer content boundary → **clipped**. Left/top borders sit at origin (0,0) → survive. This asymmetry (left/top OK, right/bottom gone) matches exactly and is **region-independent**.
3. **Proposed fix (UNVERIFIED):** give the card a small inset from the window edges so the border has room — e.g. small `padding` on the outer wrapper (`PreviewPanel.tsx` ~line 320 wrapper div) **or** a few px `margin` on `#preview-card`. Live CDP test confirmed adding `margin:6px` moved the card inward (rect x 0→6), but the box auto-closed before a clean capture could confirm the border closed. **Verify before claiming done.**

## How to VERIFY without making Andi the test machine

This session built a working **Windows-screen-capture-from-WSL** capability — see memory [[reference-windows-screen-capture-from-wsl]]. Key rules learned the hard way:

- **Use an OBJECTIVE metric, not your eyes.** I made the confirmation-bias error 2–3× reading a busy screenshot as "border complete" when pixel-sampling proved it wasn't. **Sample teal border pixels per edge** (border color ≈ rgba(42,195,168), detect G>125 & B>105 & R<120 & G−R>35) and compare counts; don't eyeball. See [[verify-surface-fix-with-objective-pixel-metric]].
- **DPI-aware capture** (`SetProcessDPIAware()` before `CopyFromScreen`) — the defect is sub-pixel; a DPI-unaware (downscaled) capture blurs it away.
- **The preview box is a TRANSIENT overlay that auto-closes fast** (a `klarvo://state-changed` = done/idle hides it). Remote single-snapshot capture kept missing it. Mitigations: parallel **burst** capture (start ~14 frames @300ms in a held bg task, fire the re-show concurrently), or keep it visible, or just verify on a normal stable build with Andi glancing once.
- **Showing the box without real recording (CDP inject):** launch the **debug** build (`D:\Apps\klarvo\src-tauri\target\debug\klarvo.exe`) with env `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222`, **held in a background task** (WSL-interop job-object kills detached `Start-Process` children otherwise). Debug build needs the **Vite dev server** running (`npm run dev` in `D:\Apps\klarvo`, also held bg) — else the windows show "localhost connection refused". CDP client must run **Windows-side** (node v24 has global `fetch`+`WebSocket`; the debug port binds to 127.0.0.1, unreachable from WSL). Find the preview target by `window.__TAURI_INTERNALS__.metadata.currentWindow.label === 'preview'`. To (re)show the box, emit via `window.__TAURI_INTERNALS__.invoke('plugin:event|emit', {event, payload})`: first `klarvo://state-changed {state:'done'}` (resets `showOnceRef`), then `{state:'recording'}`, then `klarvo://live-preview-chunk '<text>'`.

## Dev-workflow facts (still valid)

- Andi = Windows, Claude = WSL, **same machine**. Build at `D:\Apps\klarvo` (robocopy target from the WSL repo). Logs: `/mnt/c/Users/Andi/AppData/Local/com.klarvo.voice/logs/Klarvo.log` (webview `console.*` bridged as `[fe:preview]`). Config: `/mnt/c/Users/Andi/AppData/Roaming/com.klarvo.voice/config.json`.
- Frontend-only iteration: "Klarvo Win Dev" (`dev.ps1 -SkipNpm` = `tauri dev` + HMR) + "Klarvo Win Sync" (push WSL edit → `D:\Apps\klarvo`). Full release: "Klarvo Win Rebuild" (`sync-and-build.ps1`).
- Anti-pattern guard (project-context.md rule 29 / "never make the user the rendering oracle") is still in force; this session it nearly slipped again — the durable answer is the capture capability above, used with an objective metric.

## Consistency

Tree clean at HEAD; `sprint-status.yaml` `6-6 = review` (correct). No BMAD parsed contract was changed this session. The earlier WAYPOINT claim "GDI region ruled out / backdrop-filter ruled out" is now superseded by the stronger objective finding above (right/bottom-border-missing = flush-edge clipping, region-independent).
