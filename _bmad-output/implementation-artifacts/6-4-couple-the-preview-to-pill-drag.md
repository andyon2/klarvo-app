---
story: "6.4"
epic: "6"
title: "Couple the preview to pill drag"
status: review
track: L3-feature
gatedBy: ["6.2"]
buildsOn: ["6.2"]
enabledBy: ["6.5"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics-bar-redesign.md
  - docs/bar-redesign-spec.md
  - _bmad-output/project-context.md
  - docs/surface-smoke-checklist.md
---

# Story 6.4: Couple the preview to pill drag

Status: review

## Story

As a user dictating with preview enabled,
I want the preview to stay centered above the pill while I drag it,
so that the two always move together.

## Acceptance Criteria

**AC-1 — Pill emits `klarvo://bar-moved` during drag (throttled + on drag-end):**
Given the user is dragging the pill (in any state — recording or not)
When `onMouseMove` fires
Then the pill emits `klarvo://bar-moved` with payload `{ x, y }` (the pill anchor in logical px),
throttled to one emission per animation frame (requestAnimationFrame — one outstanding rAF at a
time; cancel the pending rAF before scheduling a new one so no stale position queues up).
And the event name is `klarvo://bar-moved` — **colon form, not dot form** (NFR4, Trap #5).

**AC-2 — Pill emits `klarvo://bar-moved` once more on drag-end (final position):**
Given the user releases the mouse after a drag
When `onMouseUp` fires
Then a final `klarvo://bar-moved` is emitted with the settled pill anchor `{ x, y }` (same values
as the `saveBarPosition` call that already happens on drag-end — the final position).
And the existing `saveBarPosition` call is NOT removed — only pill position is persisted (FR9).

**AC-3 — PreviewPanel re-centers via `setPosition` only on `klarvo://bar-moved` (no resize):**
Given the preview window is currently visible (recording is active with preview open)
When `klarvo://bar-moved` fires with payload `{ x, y }`
Then `PreviewPanel` reads the new pill anchor, recomputes the preview left/top using the same
centering and screen-clamp formula as `runShowSequence` (using `pillX + PILL_WIDTH/2 - W/2`,
clamped to `[screenLeft+12, screenRight-W-12]`, and `previewTop = pillY - GAP - H`),
and calls `win.setPosition(new LogicalPosition(previewLeft, previewTop))` — **no `setSize`, no
`setPreviewShape`, no `show()`** — just one `setPosition` call.
And `clampedMaxHeightRef.current` and the current `W` (width) are reused from the values set in
`runShowSequence` — no geometry re-computation, no IPC-heavy calls (NFR1 preserved).
And the screen clamp uses `currentMonitor()` (same as in `runShowSequence`) — but ONLY if the
monitor info is already cached; otherwise fall back to unclipped math to avoid adding IPC per drag
event. (A cached monitor ref updated at `runShowSequence`-time is sufficient — monitors don't
change during a drag.)

**AC-4 — No repositioning when preview is hidden:**
Given the preview is not currently visible (not recording, or recording has no chunks yet)
When `klarvo://bar-moved` fires
Then `PreviewPanel` ignores the event — no `setPosition` call, no errors.
The guard is `showOnceRef.current` (already `true` only while preview is shown within a cycle).

**AC-5 — Only pill position is persisted (FR9):**
Given drag-end fires
When the final position is saved
Then **only** `saveBarPosition(lx, pillY)` is called (already in the existing `onMouseUp` handler
in `FloatingBar.tsx`) — the preview position is NEVER written to config; it is always derived from
the pill anchor.
And inversion: writing preview position to config would cause preview to appear at a stale location
on the next recording (out-of-sync after user moves the pill while preview is closed) → RED.

**AC-6 — Smoke: drag during recording keeps the preview centered above the pill:**
Given a real Windows release build (via `sync-and-build.ps1`)
When the smoke is run
Then:
1. Start a recording with preview enabled; preview window appears above the pill.
2. Drag the pill while the preview is visible → the preview follows smoothly, staying centered above the pill.
3. Release the drag → the preview snaps to the final position (no teleport).
4. No window resize during drag (pill stays 200×36; preview window does NOT resize during drag — only setPosition fires).
5. After drag, trigger another recording cycle → the preview opens at the new (dragged) pill position.
6. Pill position persists across app restart (existing behavior; unchanged).
And inversion for AC-3: removing the `setPosition` call in the `bar-moved` listener → preview stays at the original show-time position while the pill moves → RED.

## Tasks / Subtasks

- [x] **Task 1 — `FloatingBar.tsx`: emit `klarvo://bar-moved` during drag (AC-1, AC-2)**

  The drag logic lives in two places in `FloatingBar.tsx`:
  - `onMouseMove` (inside the `useEffect` at ~line 614): handles per-frame drag moves.
  - `onMouseUp` (inside the same `useEffect`): handles drag-end, calls `saveBarPosition`.

  **1.1** Add an `emit` import. At the top of `FloatingBar.tsx`, `listen` is already imported from
  `@tauri-apps/api/event`. Add `emit` to the same import:
  ```ts
  import { listen, emit } from "@tauri-apps/api/event";
  ```

  **1.2** Add a `dragRafRef` ref near the other drag refs (after the existing `dragRef`):
  ```ts
  const dragRafRef = useRef<number | null>(null);
  ```
  This holds the pending `requestAnimationFrame` ID so we can cancel a stale one.

  **1.3** In `onMouseMove`, after the existing `win.setPosition(...)` call, add the throttled emit:
  ```ts
  // Throttled bar-moved emit: one rAF at a time, cancel any pending before scheduling.
  if (dragRafRef.current !== null) cancelAnimationFrame(dragRafRef.current);
  const pillX = d.winX + dx;
  // When the panel is open (legacy code — dead after 6.5) the window top-left sits
  // panelHeight above the pill. In 6.4's post-6.2 state, isPanelOpenRef.current is
  // always false (the pill never grows), so pillY = winY + dy directly.
  const pillY = isPanelOpenRef.current ? d.winY + dy + panelHeightRef.current : d.winY + dy;
  dragRafRef.current = requestAnimationFrame(() => {
    dragRafRef.current = null;
    emit("klarvo://bar-moved", { x: pillX, y: pillY }).catch(
      (e) => console.warn("[bar] bar-moved emit failed:", e)
    );
  });
  ```

  **1.4** In `onMouseUp`, after `barX.current = lx; barY.current = pillY;` and the
  `saveBarPosition(lx, pillY)` call, add the final drag-end emit:
  ```ts
  // Emit the settled position once more so the preview snaps to the exact final anchor.
  emit("klarvo://bar-moved", { x: lx, y: pillY }).catch(
    (e) => console.warn("[bar] bar-moved final emit failed:", e)
  );
  ```
  Note: `pillY` is already computed correctly in `onMouseUp` (it accounts for `isPanelOpenRef` and
  `panelHeightRef` — same adjustment as for `saveBarPosition`). No change needed to that logic.

  **1.5** In `onMouseUp`, also cancel any pending rAF on drag-end so no stale intermediate
  position fires after the final emit:
  ```ts
  if (dragRafRef.current !== null) {
    cancelAnimationFrame(dragRafRef.current);
    dragRafRef.current = null;
  }
  ```
  (Place this at the beginning of `onMouseUp`, before the `if (!d) return;` or right after it.)

- [x] **Task 2 — `PreviewPanel.tsx`: subscribe to `klarvo://bar-moved` and reposition (AC-3, AC-4)**

  **2.1** Add a cached monitor ref near the other refs (after `pillXRef`/`pillYRef`):
  ```ts
  // Cached monitor info from runShowSequence — reused for drag repositioning.
  // Updated once per show sequence; monitors don't change during a drag.
  const cachedMonitorRef = useRef<{
    screenLeft: number;
    screenRight: number;
  } | null>(null);
  ```

  **2.2** In `runShowSequence`, after the monitor clamp block, cache the monitor bounds:
  ```ts
  // Cache for drag repositioning (AC-3 — avoid per-drag monitor IPC).
  if (monitor) {
    const scale = monitor.scaleFactor || 1;
    const wa = monitor.workArea ?? { position: monitor.position, size: monitor.size };
    cachedMonitorRef.current = {
      screenLeft:  wa.position.x / scale,
      screenRight: (wa.position.x + wa.size.width) / scale,
    };
  }
  ```
  (This goes inside the existing `try { const monitor = await currentMonitor(); ... }` block, after
  the existing `previewLeft = Math.max(...)` clamp line.)

  **2.3** Add a `W` ref to hold the preview width for repositioning. The width is computed inside
  `runShowSequence` as `const W = geom.width;` — add a ref to persist it:
  ```ts
  const previewWidthRef = useRef<number>(BASE_WIDTH.comfortable); // updated each show
  ```
  In `runShowSequence`, after `const W = geom.width;`, set it:
  ```ts
  previewWidthRef.current = W;
  ```

  **2.4** Add the `klarvo://bar-moved` listener as a `useEffect` in `PreviewPanel`:
  ```ts
  // ---------------------------------------------------------------------------
  // Story 6.4: klarvo://bar-moved — reposition preview when pill is dragged (AC-3, AC-4)
  // ---------------------------------------------------------------------------
  useEffect(() => {
    const unlisten = listen<{ x: number; y: number }>("klarvo://bar-moved", (event) => {
      // AC-4: only reposition when the preview is currently showing.
      if (!showOnceRef.current) return;

      const { x: pillX, y: pillY } = event.payload;
      // Update stored pill anchor for subsequent runShowSequence calls (e.g. next cycle).
      pillXRef.current = pillX;
      pillYRef.current = pillY;

      const W = previewWidthRef.current;
      const H = clampedMaxHeightRef.current;
      const pillCenterX = pillX + PILL_WIDTH / 2;
      let previewLeft = pillCenterX - W / 2;

      // Apply horizontal screen clamp from cached monitor info (no new IPC).
      const m = cachedMonitorRef.current;
      if (m) {
        previewLeft = Math.max(m.screenLeft + 12, Math.min(previewLeft, m.screenRight - W - 12));
      }
      const previewTop = pillY - GAP - H;

      // setPosition ONLY — no setSize, no setPreviewShape, no show().
      // NFR1: no per-drag window IPC beyond the single setPosition call.
      const win = getCurrentWebviewWindow();
      win.setPosition(new LogicalPosition(previewLeft, previewTop)).catch(
        (e) => console.warn("[preview] bar-moved setPosition failed:", e)
      );
    });
    unlisten.then(() => console.log("[preview] bar-moved listener REGISTERED"));
    return () => { unlisten.then((fn) => fn()); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  ```

- [x] **Task 3 — Capabilities: confirm `klarvo://bar-moved` is receivable in the `"preview"` window (AC-1/AC-3)**

  The `"preview"` window is already in `capabilities/default.json`'s `windows` array (added in
  Story 6.2's fix for the capability-scope root cause). The `listen()` permission flows from
  `core:event:default` which is included via `core:default`. No capability change is needed.

  However, **verify**: the event is emitted by `FloatingBar.tsx` (the `"bar"` window) via the JS
  `emit()` API (`@tauri-apps/api/event`). The `"preview"` window listens via `listen()`. Both
  windows are in the `default` capability's `windows` array. This is the same topology as
  `klarvo://live-preview-chunk` and `klarvo://state-changed` — both already work in the preview
  window. No additional capability entry is required.

  ⚠️ **Durable lesson from 6.2**: if events stop working in the preview window, check
  `capabilities/default.json` first — the window label must be in the `windows` array.

- [x] **Task 4 — Verify and close (AC-6, DoD)**

  - [x] 4.1 `cargo test --lib` — 575 tests / 0 fail. No regression (baseline 575 from 6.3). No new Rust code.
  - [x] 4.2 `cargo check --target x86_64-pc-windows-gnu` — pre-existing whisper-rs-sys build failure
    in the GNU cross-compiler toolchain (PROCESS_POWER_THROTTLING_EXECUTION_SPEED / StateMask struct
    member — ggml-cpu.c:2424). Not introduced by 6.4 (JS-only change, no Rust files touched).
    Identical to baseline; story Dev Notes explicitly state "No Rust change is needed."
  - [x] 4.3 `tsc --noEmit` → exit 0 (no output); `npm run build` → Vite clean, 81 modules, 4.37s. ✅
  - [x] 4.4 Pre-smoke checklist (surface-smoke-checklist.md):
    - **Trap #5 (event wiring):** Producer `FloatingBar.tsx` calls `emit("klarvo://bar-moved", ...)` ✅ colon form; Consumer `PreviewPanel.tsx` calls `listen("klarvo://bar-moved", ...)` ✅ colon form.
    - **No new config key** → Trap #1/#2 not applicable. ✅
    - **No Settings field** → Trap #2 not applicable. ✅
    - **No geometry change at show-time** → Trap #4 not applicable. ✅
  - [ ] 4.5 Windows smoke (real release build, `sync-and-build.ps1`) — PENDING (requires Andi on Windows):
    - Start recording with preview enabled.
    - Drag pill while preview is showing → preview follows, centered.
    - Release → preview snaps to final position.
    - No resize of pill or preview window during drag.
    - New recording cycle → preview opens at dragged-to position.
    - Verify AC-6 inversion (dev-time only, not smoke gate): temporarily remove
      the `setPosition` call in the bar-moved listener → preview stays put while pill moves → RED.

## Dev Notes

### The two pieces of work

Story 6.4 is two independent changes wired together:

1. **`FloatingBar.tsx`** — pill side: emit `klarvo://bar-moved {x, y}` during drag (throttled rAF)
   and on drag-end (settled final position).
2. **`PreviewPanel.tsx`** — preview side: listen to `klarvo://bar-moved` and call `setPosition` only.

No Rust change is needed. No new Tauri command. The event is emitted and received entirely in JS
(`@tauri-apps/api/event` `emit` / `listen`). This is the same pattern already used for
`klarvo://active-mode` (emitted by Rust, listened in `FloatingBar.tsx`) and `klarvo://live-preview-chunk`.

### Why emit from JS, not from Rust

The drag logic is entirely in the FloatingBar JS (`onMouseMove` / `onMouseUp`). The pill position
during drag is only known to the frontend (computed as `d.winX + dx` from mouse deltas). There is
no backend drag event — the Rust `save_bar_position` command is only invoked on drag-end. Emitting
from JS means the event carries the live intermediate positions during drag, not just the final one.

### Throttling strategy (rAF)

Use `requestAnimationFrame` for the throttled emit, not `setInterval`. `rAF` naturally fires once
per frame (60 fps), which is the correct rate for drag tracking: smooth visually, no excess IPC.
Pattern: keep a `dragRafRef` ref; in `onMouseMove`, cancel any pending rAF before scheduling a new
one so exactly one emit fires per frame regardless of how fast mouse events arrive.

On drag-end (`onMouseUp`), cancel any pending rAF and emit the final settled position directly
(outside rAF) so the preview always snaps to the exact pill anchor that was saved to config.

### `setPosition`-only in the preview (NFR1 preserved)

The `bar-moved` handler in `PreviewPanel` calls **only** `win.setPosition()`. No `setSize`, no
`setPreviewShape`, no `show()`. Width and height are fixed for the lifetime of a recording cycle
(set once in `runShowSequence`). Calling `setSize` per drag would re-introduce the async IPC race
class (R3/R4/R5) that this entire epic was designed to eliminate.

### Cached monitor bounds (avoid per-drag IPC)

`currentMonitor()` is an async IPC call. Calling it per drag event would flood the IPC bus.
Cache the monitor bounds in `runShowSequence` (where `currentMonitor()` already runs) into a
`cachedMonitorRef`, and reuse it in the `bar-moved` handler. Monitors don't change during a drag.

### Width ref

The preview width (`W = geom.width`) is computed in `runShowSequence` from the user's font-size and
width-preset settings. It does not change during a drag. Store it in `previewWidthRef` in
`runShowSequence` so the `bar-moved` handler can use it without re-calling `previewGeometry`.

### `pillXRef`/`pillYRef` update in the bar-moved handler

`PreviewPanel` already stores `pillXRef` and `pillYRef` in `runShowSequence`. The `bar-moved`
handler should update these refs when it fires so that future `runShowSequence` calls (on the next
recording cycle) use the final dragged-to position, not the position the pill was at when the
current recording started. This ensures continuity across cycles.

### Pill `isPanelOpenRef` / `panelHeightRef` in drag-emit computation

`FloatingBar.tsx` computes the pill anchor in `onMouseUp` as:
```ts
const pillY = isPanelOpenRef.current ? ly + panelHeightRef.current : ly;
```
This adjustment exists because in the old single-window design the window top-left sits `panelHeight`
above the pill when the panel is open (the window grew upward). In the post-6.2 world the pill never
grows (`isPanelOpenRef.current` is always `false` because `isPanelOpen = isRecording && livePreview.length > 0`
and `livePreview` in the pill is never populated after 6.2). So `pillY = ly + dy` in practice.
The `onMouseMove` emit should mirror the same logic for correctness (use `isPanelOpenRef.current`
and `panelHeightRef.current` exactly as `onMouseUp` does) so the drag-time emits are consistent
with the drag-end emit and with what `saveBarPosition` writes.

### Event name: `klarvo://bar-moved` (colon, not dot)

Per `project-context.md` rule: Tauri reserves `.` in event strings. The event **must** be
`klarvo://bar-moved` — colon-slash-slash form. If the dot form `klarvo.bar-moved` is used,
it will silently fail to route correctly.

### No Android change

This is a Windows-desktop-only story. Do not touch any Android path (`android/`, `KlarvoApi.kt`,
`LocalWhisperInference.kt`). The floating-bar concept does not apply to Android.

### No Rust change

No new `#[tauri::command]` is needed. The `save_bar_position` / `get_bar_position` commands are
unchanged. The event flows purely in JS.

### Key files to touch

| File | Change |
|---|---|
| `src/FloatingBar.tsx` | Add `emit` import; `dragRafRef`; throttled `emit("klarvo://bar-moved", ...)` in `onMouseMove`; final emit + rAF cancel in `onMouseUp` |
| `src/PreviewPanel.tsx` | Add `cachedMonitorRef`, `previewWidthRef`; cache monitor bounds in `runShowSequence`; store `W` in `previewWidthRef`; add `klarvo://bar-moved` listener `useEffect` |

### Inversion checks

Per Epic-4 retro AI-1: every guard must prove it goes RED.

1. **AC-3 setPosition (smoke-time):** Remove the `setPosition` call in the `bar-moved` listener
   → drag the pill with preview open → preview stays at the show-time position → RED.
2. **AC-4 guard (dev-time):** Remove `if (!showOnceRef.current) return;` → drag pill with preview
   closed → `setPosition` fires on a hidden window (no visual effect, but log shows spurious
   `bar-moved setPosition` calls and `previewLeft`/`previewTop` may be NaN before first show).
3. **AC-1 event name (compile-time):** Use `"klarvo.bar-moved"` (dot form) instead of
   `"klarvo://bar-moved"` → Tauri rejects the dot-form event routing → listener never fires → RED.

### References

- FR9 (only pill persisted) + NFR4 (colon event name): `_bmad-output/planning-artifacts/epics-bar-redesign.md`
- Scale-factor geometry / `previewGeometry`: `src/PreviewPanel.tsx:31-43` (do not re-derive)
- `FONT_PX`, `PILL_WIDTH`, `GAP`, `BASE_MAX_HEIGHT` constants: `src/PreviewPanel.tsx:22-30`
- `runShowSequence` (show-once geometry, existing monitor clamp): `src/PreviewPanel.tsx:101-215`
- `pillXRef`, `pillYRef`, `clampedMaxHeightRef`, `showOnceRef`: `src/PreviewPanel.tsx:79-80`
- Existing drag logic (`onMouseMove`, `onMouseUp`, `dragRef`): `src/FloatingBar.tsx:596-648`
- `isPanelOpenRef`, `panelHeightRef`: `src/FloatingBar.tsx:245-248, 394-398`
- `saveBarPosition` command (called in `onMouseUp` — do NOT remove): `src/FloatingBar.tsx:638`
- `emit` / `listen` from `@tauri-apps/api/event`: already imported in `FloatingBar.tsx` (line 2)
- Event capability scope (preview window must be in `windows` array): `src-tauri/capabilities/default.json`
- Surface smoke checklist: `docs/surface-smoke-checklist.md`
- Project rules (event names, no Android, no Rust panics): `_bmad-output/project-context.md`
- Ist-Zustand: `docs/deep-dive-bar-subsystem.md`
- Foundation design: `docs/bar-redesign-spec.md §3.5` (drag coupling)

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

No debug issues — straightforward JS-only implementation.

### Completion Notes List

- Task 1 (FloatingBar.tsx): Added `emit` to the `@tauri-apps/api/event` import. Added `dragRafRef`
  ref after `dragRef`. In `onMouseMove`, added rAF-throttled `emit("klarvo://bar-moved", { x, y })`
  after the existing `setPosition` call — mirrors the same `isPanelOpenRef`/`panelHeightRef` pillY
  adjustment as `onMouseUp`. In `onMouseUp`, added rAF cancel at the start, then final
  `emit("klarvo://bar-moved", { x: lx, y: pillY })` after `saveBarPosition` (same position values,
  no divergence). `saveBarPosition` unchanged (AC-5 / FR9).
- Task 2 (PreviewPanel.tsx): Added `cachedMonitorRef` and `previewWidthRef` refs. In
  `runShowSequence`, after the monitor clamp block, cached `{ screenLeft, screenRight }` in
  `cachedMonitorRef`. After `const W = geom.width`, stored W in `previewWidthRef`. Added a
  `useEffect` subscribing to `klarvo://bar-moved`: guards on `showOnceRef.current` (AC-4), updates
  `pillXRef`/`pillYRef` for future cycles, recomputes `previewLeft` using the centering formula +
  cached monitor clamp (no new IPC), calls `win.setPosition(new LogicalPosition(...))` only — no
  `setSize`, no `setPreviewShape`, no `show()` (NFR1 preserved).
- Task 3 (Capabilities): Verified `"preview"` already in `src-tauri/capabilities/default.json`
  `windows` array — no change needed.
- Task 4: cargo test 575/575 pass (no Rust change); tsc exit 0; vite build clean. cargo check
  cross-compile has a pre-existing whisper-rs-sys/ggml-cpu.c failure unrelated to this story.
  Trap #5 verified: both `emit` and `listen` use `"klarvo://bar-moved"` (colon form).
- Windows smoke (4.5): PENDING — Andi to run sync-and-build.ps1 and test drag-follows-preview.

### File List

- src/FloatingBar.tsx
- src/PreviewPanel.tsx

### Change Log

- 2026-06-08: Story 6.4 implemented — FloatingBar.tsx emits klarvo://bar-moved (rAF-throttled
  during drag + final emit on drag-end); PreviewPanel.tsx listens and calls setPosition only
  (cachedMonitorRef + previewWidthRef avoid per-drag IPC, NFR1 preserved). JS-only, no Rust,
  no new config key, no capability change. tsc exit 0, vite clean, 575 Rust tests/0 fail.
- 2026-06-08: CODE-REVIEW PASS (Opus 4.8, 3 adversarial layers — Blind Hunter / Edge Case Hunter /
  Acceptance Auditor; fix-loop 0 rounds, 0 patch). Acceptance Auditor: AC-1…AC-5 all satisfied
  (centering/clamp formula identical to runShowSequence; colon-form event; no Rust/Android; no
  setSize/setPreviewShape/show — NFR1 held). Two High flags REFUTED against real code: (1) Blind
  Hunter "showOnceRef latches true forever" → FALSE, reset to false on every cycle end
  (PreviewPanel.tsx:247) so the AC-4 guard is correct; (2) "coordinate-basis divergence d.winX+dx
  vs lx" → same logical space (both pos.x/scale), the final emit is the intended sub-pixel
  snap-to-settled (AC-2/AC-6 "no teleport"). 1 finding DEFERRED → deferred-work.md: cachedMonitorRef
  not invalidated on cross-monitor drag mid-recording (spec-sanctioned no-per-drag-IPC tradeoff per
  AC-3). Rest dismissed (NaN guarded upstream; global emit = established pattern, throttled 1/frame;
  rAF-on-unmount near-impossible + caught no-op; dead isPanelOpenRef branch = intentional onMouseUp
  mirror, 6.5 cleans up). Code already committed 49bbbf5. BLOCKED on AC-6 Windows release smoke
  (surface-class hard gate, Andi) — Status flips to done after smoke GREEN.
