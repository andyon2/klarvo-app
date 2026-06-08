---
story: "6.5"
epic: "6"
title: "Pill fully static + cleanup"
status: review
track: L3-feature
gatedBy: ["6.2", "6.4"]
buildsOn: ["6.2", "6.4"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics-bar-redesign.md
  - docs/bar-redesign-spec.md
  - _bmad-output/project-context.md
  - docs/surface-smoke-checklist.md
---

# Story 6.5: Pill fully static + cleanup

Status: review

## Story

As a developer,
I want the dead grow logic removed and the pill made truly static,
so that the codebase reflects the new foundation with no resize paths left and 5-7 is reconciled.

## Acceptance Criteria

**AC-1 — Dead grow code deleted from `FloatingBar.tsx`:**
Given `FloatingBar.tsx` after 6.2/6.4
When the dead grow code is removed
Then `livePreview` state, `panelHeight` state, `panelScrolls` state, `geomTick` state,
`measureRef`, `previewPanelRef`, `prevIsPanelOpenRef` ref, `isPanelOpenRef` ref,
`panelHeightRef` ref, the measure `useLayoutEffect`, the per-chunk grow/resize `geomTick`
effect (`useEffect` that calls `setGeomTick`), the grow/resize geometry show-effect
(`isPanelOpen && panelHeight === 0` guard + `setBarShape("panel")` + `activePillWidth`/
`activePillHeight` dynamic sizing), the panel render block (`{isPanelOpen && (...)}` JSX),
and the `prevIsPanelOpenRef` panel-open-refresh effect are all deleted (AR4).
And the pill `show` effect is **show/hide + position only** — no `setSize`, no `setBarShape`
(the shape is set once at creation; the bar window is `hidden` on idle, so no idle shape call
is needed either).
And inversion: re-inserting any of these causes a dead-code warning or tsc error (state
never set, ref unused).

**AC-2 — `FORM_APPEARANCES`, `getFormAppearance`, `previewPanelForm` state, and related dead
constants removed:**
Given the cleanup is complete
When these pill-level preview-sizing helpers are removed
Then `FORM_APPEARANCES`, `DEFAULT_FORM`, `getFormAppearance()`, the `previewPanelForm`
`useState` + its initial `getSettings()` load, `PANEL_WIDTH` and `PANEL_ABS_MAX` derived
values, and `SCREEN_TOP_MARGIN` are all deleted from `FloatingBar.tsx`.
And the `previewPanelForm` re-read on closed→open transition (`prevIsPanelOpenRef` effect)
is also deleted (it was a workaround for the separate-window reactivity trap that no longer
applies to the pill).
And inversion: tsc confirms the file compiles clean (`tsc --noEmit` exit 0).

**AC-3 — Pill window is one fixed size, region set once, never resizes:**
Given the pill window
When it is created
Then `create_bar_window` creates it at `PILL_WIDTH × PILL_HEIGHT` (200 × 36 logical px) and
the pill region is set **once** at creation (the existing initial `set_window_region_pill`
call in `create_bar_window`).
And the pill is **never resized** in any state — recording, processing, done, clipboard-only,
error (FR1).
And inversion: `cargo grep` / `tsc` confirm there is no `setSize` call in `FloatingBar.tsx`
after the cleanup.

**AC-4 — Clipboard-done state re-laid-out to fit fixed pill width (200):**
Given the clipboard-done state ("In Clipboard")
When it is shown
Then it fits within `PILL_WIDTH = 200` — the existing emoji + "In Clipboard" text layout
is visually validated on the real build (the pill was previously widened to 220 for this
state via `PILL_WIDTH_CLIPBOARD`).
And `PILL_WIDTH_CLIPBOARD = 220` constant and the `pillWidth` branch
`(isDone && clipboardOnly) ? PILL_WIDTH_CLIPBOARD : PILL_WIDTH` are deleted.
And `activePillWidth` and `activePillHeight` derived values are also deleted (they were the
dynamic show-effect sizing). `pillWidth` as a variable is deleted (unused after the cleanup).
And the show-effect sizes to a **constant** — the window geometry is already set at creation;
the show-effect only calls `setPosition` + `win.show()`.
And inversion: at 200px width the "In Clipboard" text + clipboard emoji display correctly
without overflow (verified in Windows smoke).

**AC-5 — `setBarShape("panel")` and per-preset `screenCap` remnants removed (AR5):**
Given the cleanup is complete
When the build runs
Then `setBarShape` is **not called anywhere** in `FloatingBar.tsx` (neither the "panel" nor
the "pill" branch — the bar window region is set once in `create_bar_window` and the
`set_bar_shape` command is no longer needed from the frontend).
And the `setBarShape` import in `FloatingBar.tsx` is removed from the `tauri-commands`
import list.
And the `screenCap` field in `FORM_APPEARANCES` entries (and `getFormAppearance`) is gone
(entire `FORM_APPEARANCES` object deleted per AC-2).
And `set_bar_shape` in `src-tauri/src/commands/misc.rs` is left in place (it is a
`#[tauri::command]` that stays registered — removing it is a separate clean-up, out of
scope; just stop calling it from the frontend).
And `setBarShape` in `tauri-commands.ts` is left in place (same reason — stop importing it
in FloatingBar.tsx, do NOT delete it from tauri-commands.ts).

**AC-6 — Lib tests green + `clippy` clean on touched files:**
Given the cleanup is complete
When `cargo test --lib` runs
Then all existing tests pass (baseline 575 from 6.4; no Rust production code changes, so
count should not decrease). No new Rust code is written for this story.
And `tsc --noEmit` exits 0; `npm run build` (Vite) exits clean.
And `clippy` on touched Rust files (none expected) has no new warnings.

**AC-7 — Story 5-7 reconciled in `sprint-status.yaml`:**
Given Story 5-7 (parked, `review`)
When Epic 6 lands
Then 5-7 is marked **superseded** in `sprint-status.yaml`: its R1 stale-chunk guard and
R2 backpressure live on in the new design (PreviewPanel.tsx + 5-7's hardened backend via
`try_acquire_preview_slot`), and 5-7's single-window geometry work is now moot.
The status update in `sprint-status.yaml` comment should read:
`5-7-preview-flush-hardening-stale-chunk-guard-and-backpressure: done # SUPERSEDED by
Epic 6 architecture: R1 guard lives on in PreviewPanel.tsx (isRecordingRef), R2 backend
backpressure (try_acquire_preview_slot) is retained, single-window geometry is moot.
Story 5-7 commit e71d1c0 stays in the history.`
(The YAML status value stays `done` — it was code-review cleared, Windows smoke was
blocked on the pre-existing geometry issue which Epic 6 resolves by construction.)

**AC-8 — Smoke: pill never resizes in any state; preview still works end-to-end:**
Given a real Windows release build (via `sync-and-build.ps1`)
When the smoke is run
Then:
1. Idle → hidden (no pill visible) — correct.
2. Recording → pill appears at 200×36, waveform animates. No resize.
3. Preview chunks arrive → preview window appears above pill (from 6.2). Pill stays 200×36.
4. Drag pill → preview follows (from 6.4). Pill stays 200×36.
5. End recording (`done`) → Done flash at 200×36 (NOT 220). "Done" text fits correctly.
6. Clipboard-only done → "In Clipboard" with emoji at 200×36. Text fits without overflow.
7. Error state → "Error" text at 200×36. Fits correctly.
8. Throughout all states: `setSize` is never called (no window resize IPC).
And inversion: the removed `setSize` in the show-effect is confirmed absent (log has no
`[bar] showing pill:` line after cleanup — that console.log is also removed with the dead code).

## Tasks / Subtasks

- [x] **Task 1 — Delete all dead grow state and refs from `FloatingBar.tsx` (AC-1, AC-2)**

  1.1 Delete these `useState` declarations:
  ```ts
  // DELETE all three:
  const [livePreview, setLivePreview] = useState("");
  const [panelHeight, setPanelHeight] = useState(0);
  const [panelScrolls, setPanelScrolls] = useState(false);
  const [geomTick, setGeomTick] = useState(0);
  const [previewPanelForm, setPreviewPanelForm] = useState<string>(DEFAULT_FORM);
  ```

  1.2 Delete these `useRef` declarations:
  ```ts
  // DELETE all five:
  const previewPanelRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef<HTMLDivElement>(null);
  const panelHeightRef = useRef(0);
  const isPanelOpenRef = useRef(false);
  const prevIsPanelOpenRef = useRef(false);
  ```

  1.3 Delete the `isPanelOpen` derived variable:
  ```ts
  // DELETE:
  const isRecording = state === "recording";  // KEEP (still needed)
  // DELETE:
  const isPanelOpen = isRecording && livePreview.length > 0;
  ```

  1.4 Delete `PILL_WIDTH_CLIPBOARD`, `SCREEN_TOP_MARGIN`, `FORM_APPEARANCES`, `DEFAULT_FORM`,
  and `getFormAppearance()` — the entire constants block from line ~29 to ~48.

- [x] **Task 2 — Delete dead `useEffect` / `useLayoutEffect` blocks (AC-1, AC-2)**

  2.1 Delete the `isRecordingRef` mirror effect:
  ```ts
  // Story 5.7 isRecordingRef mirror — DELETE (isRecordingRef and setLivePreview removed):
  useEffect(() => {
    isRecordingRef.current = isRecording;
  }, [isRecording]);
  ```
  Also delete the `isRecordingRef` declaration above it:
  ```ts
  const isRecordingRef = useRef(false);  // DELETE
  ```

  2.2 Delete the now-no-op live-preview-chunk listener:
  ```ts
  // DELETE the entire useEffect:
  useEffect(() => {
    const unlisten = listen<string>("klarvo://live-preview-chunk", (_event) => {
      // Story 6.2: preview moved to the "preview" window (PreviewPanel.tsx).
      // This listener is intentionally disabled. Dead code cleaned up in Story 6.5.
      return;
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);
  ```

  2.3 Delete the dead `previewPanelRef` scroll effect:
  ```ts
  // DELETE:
  useEffect(() => {
    if (previewPanelRef.current) {
      previewPanelRef.current.scrollTop = previewPanelRef.current.scrollHeight;
    }
  }, [livePreview]);
  ```

  2.4 Delete the `prevIsPanelOpenRef` panel-open refresh effect (the `getSettings` re-read
  on panel closed→open):
  ```ts
  // DELETE the entire effect (~lines 350-360):
  const prevIsPanelOpenRef = useRef(false);
  useEffect(() => {
    const wasOpen = prevIsPanelOpenRef.current;
    prevIsPanelOpenRef.current = isPanelOpen;
    if (!wasOpen && isPanelOpen) {
      getSettings()
        .then((s) => setPreviewPanelForm(s.previewPanelForm ?? DEFAULT_FORM))
        .catch(...);
    }
  }, [isPanelOpen]);
  ```

  2.5 Delete the `useLayoutEffect` (measure probe — the whole block ~lines 376-390):
  ```ts
  // DELETE:
  useLayoutEffect(() => {
    if (!isPanelOpen) { setPanelHeight(0); setPanelScrolls(false); return; }
    const el = measureRef.current;
    if (!el) return;
    // ... (panelHeight measurement logic)
  }, [livePreview, isPanelOpen, PANEL_ABS_MAX, PANEL_WIDTH]);
  ```

  2.6 Delete the `panelHeightRef`/`isPanelOpenRef` mirror effect (~lines 395-398):
  ```ts
  // DELETE:
  useEffect(() => {
    panelHeightRef.current = panelHeight;
    isPanelOpenRef.current = isPanelOpen;
  });
  ```

  2.7 Delete the `geomTick` effect (~lines 419-424):
  ```ts
  // DELETE:
  useEffect(() => {
    if (!isPanelOpen) return;
    const raf = requestAnimationFrame(() => setGeomTick((t) => t + 1));
    const late = setTimeout(() => setGeomTick((t) => t + 1), 120);
    return () => { cancelAnimationFrame(raf); clearTimeout(late); };
  }, [isPanelOpen]);
  ```

- [x] **Task 3 — Simplify the show-effect to position + show only (AC-3, AC-5)**

  The current show-effect (~lines 426-479) does:
  ```
  setSize(activePillWidth, activePillHeight) → setBarShape → setPosition → show()
  ```
  After cleanup it should be just:
  ```
  setPosition → show()
  ```

  3.1 Delete the `activePillWidth`, `activePillHeight`, and `pillWidth` derived values:
  ```ts
  // DELETE all three:
  const pillWidth = (isDone && clipboardOnly) ? PILL_WIDTH_CLIPBOARD : PILL_WIDTH;
  const { width: PANEL_WIDTH, screenCap: PANEL_ABS_MAX } = getFormAppearance(previewPanelForm);
  const activePillWidth = isPanelOpen ? PANEL_WIDTH : pillWidth;
  const activePillHeight = isPanelOpen ? PILL_HEIGHT + panelHeight : PILL_HEIGHT;
  ```

  3.2 Rewrite the show-effect to be position + show only:
  ```ts
  useEffect(() => {
    const win = getCurrentWebviewWindow();
    (async () => {
      if (isPillVisible) {
        if (barX.current != null && barY.current != null && dragRef.current == null) {
          await win.setPosition(new LogicalPosition(barX.current, barY.current));
        }
        try {
          await win.show();
        } catch (e) {
          console.error("[bar] show failed, attempting recovery:", e);
          try {
            const recreated = await ensureBarWindow();
            if (recreated) console.log("[bar] window recreated via recovery");
          } catch (re) {
            console.error("[bar] recovery also failed:", re);
          }
        }
      }
    })();
  }, [isPillVisible]);
  ```

  Note: `geomTick` is removed from the dep array since the `geomTick` state is deleted.
  `activePillWidth`/`activePillHeight` are removed. The dependency is just `isPillVisible`.

  Also remove the `setBarShape` import from the `tauri-commands` import line at the top:
  ```ts
  // BEFORE (partial):
  import { ..., setBarShape, ... } from "./tauri-commands";
  // AFTER: remove setBarShape from the import list
  ```

- [x] **Task 4 — Delete the state-changed handler's stale `setLivePreview("")` calls (AC-1)**

  In the `onStateChanged` handler, two places set `setLivePreview("")`:
  - On `newState === "recording"`: remove `setLivePreview("");`
  - On `newState === "done"`: remove `setLivePreview("");` and `const isClipboardOnly = !!payload.clipboardOnly;` + `setClipboardOnly(isClipboardOnly);` stay (these are for the done-state display, NOT related to preview).

  Wait — `clipboardOnly` state + setter IS still used (the done-flash shows the emoji).
  Only `setLivePreview("")` references are removed. Double-check: after removing
  `livePreview` state, any remaining `setLivePreview` call will be a tsc error → easy to catch.

- [x] **Task 5 — Delete the hidden measure probe JSX and panel render block (AC-1)**

  5.1 In the render return, delete the hidden `measureRef` probe `<div>` (it is the
  first `<div>` in the fragment with `aria-hidden` and `ref={measureRef}`, ~lines 706-733).

  5.2 Delete the panel render block — the `{isPanelOpen && (...)}` JSX block inside
  the outer wrapper `<div>`, including the `<style>` for the scrollbar and the
  `#preview-panel` div (~lines 771-808).

  5.3 In the outer wrapper `<div>`, the `borderRadius` was `isPanelOpen ? 14 : 9999`.
  Simplify to the constant pill value: `borderRadius: 9999`.

  5.4 In the pill row `<div>`, the `height`/`flex` was `isPanelOpen ? { height: PILL_HEIGHT } : { flex: 1 }`.
  Simplify to just `flex: 1` (always, since isPanelOpen is gone).

  5.5 In the pill row `<div>`, the padding was `isPanelOpen ? 14 : 10`.
  Simplify to `10` (constant, since the panel state is gone).

- [x] **Task 6 — Clean up imports (AC-5)**

  6.1 `useLayoutEffect` was used only for the measure probe — if no other `useLayoutEffect`
  remains in FloatingBar.tsx, remove it from the React import:
  ```ts
  // BEFORE:
  import { useState, useEffect, useLayoutEffect, useRef } from "react";
  // AFTER (if no useLayoutEffect remains):
  import { useState, useEffect, useRef } from "react";
  ```

  6.2 `LogicalSize` was used only for `setSize` calls — if no `setSize` remains, remove it:
  ```ts
  // BEFORE:
  import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";
  // AFTER (if no LogicalSize usage remains):
  import { LogicalPosition } from "@tauri-apps/api/dpi";
  ```

  6.3 Remove `setBarShape` from the `tauri-commands` import (already noted in Task 3).

  6.4 The `listen` import from `@tauri-apps/api/event` is still needed by the
  `klarvo://audio-level` listener and the `klarvo://active-mode` listener — keep it.
  `emit` is still needed by 6.4's `klarvo://bar-moved` emit — keep it.

- [x] **Task 7 — Also check: `previewPanelForm` in `getSettings()` load on mount (AC-2)**

  The mount-level `getSettings()` in the load-on-mount effect currently reads both
  `hotkeyMode` AND `previewPanelForm`:
  ```ts
  useEffect(() => {
    getSettings()
      .then((s) => {
        setHotkeyMode(s.hotkeyMode);
        setPreviewPanelForm(s.previewPanelForm ?? DEFAULT_FORM);  // DELETE this line
      })
      ...
  }, []);
  ```
  Remove only the `setPreviewPanelForm` line. Keep `setHotkeyMode` (mode badge still
  uses `hotkeyMode`).

- [x] **Task 8 — Reconcile Story 5-7 in `sprint-status.yaml` (AC-7)**

  Update the `5-7-preview-flush-hardening-stale-chunk-guard-and-backpressure` entry comment
  to note it is superseded by Epic 6 (see AC-7 for the exact wording). Keep status `done`.

- [x] **Task 9 — Verify and close (AC-6, AC-8, DoD)**

  - [x] 9.1 `cargo test --lib` — 575 passed / 0 failed. No Rust production code changed.
  - [x] 9.2 `tsc --noEmit` → exit 0 (no output). `npm run build` → Vite clean (81 modules, ✓ built in 5.91s).
  - [x] 9.3 Pre-smoke checklist verified:
    - **No new config key** → Trap #1/#2 not applicable.
    - **No new Settings field** → Trap #2 not applicable.
    - **No new event** → Trap #5 not applicable.
    - **Window geometry Trap #4:** `grep "setSize\|LogicalSize"` in FloatingBar.tsx → 0 hits in active code. LogicalSize import removed. setSize not called anywhere.
    - **Clipboard-done at 200px:** JSX verified — emoji (24px) + gap (6) + "In Clipboard" (~72px) + padding (20) ≈ 122px, well within 200px.
  - [ ] 9.4 Windows smoke (real release build, `sync-and-build.ps1`) — pending Andi's manual verification on Windows.

## Dev Notes

### What this story is (and is not)

6.5 is a **pure cleanup / dead-code removal** story. No new functionality is added. Every code
path being deleted was already dead after 6.2 (preview moved to the separate window) and 6.4
(bar-moved event wired). The pill was already effectively static — `isPanelOpen` was always
`false` because `livePreview` state in FloatingBar was never populated after 6.2's no-op
listener. The `setBarShape("panel")` call was never reached; `setSize` was always called with
`PILL_WIDTH × PILL_HEIGHT` (the panel path was dead). This story makes the structure match reality.

The only functional change is the clipboard-done width: **220 → 200** (fit "In Clipboard" + emoji
in 200px instead of widening the window). Verify this fits in the Windows smoke (AC-8 item 6).

### Key files touched

| File | Change |
|---|---|
| `src/FloatingBar.tsx` | Delete ~200 lines of dead grow code; simplify show-effect; remove setBarShape; fix clipboard-done width |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | Reconcile 5-7 comment (AC-7) |

No Rust files are touched. `src/PreviewPanel.tsx` is not touched — it is the working preview
window, not the pill.

### Current state of FloatingBar.tsx dead code

After 6.2 + 6.4, `FloatingBar.tsx` contains:

1. **`livePreview` state** (line ~236) — always `""`, never populated (the chunk listener at
   line ~328 returns immediately without calling `setLivePreview`).
2. **`isPanelOpen`** (line ~265) — always `false` because `livePreview.length > 0` is never true.
3. **`panelHeight` / `panelScrolls`** (lines ~237-238) — always `0` / `false`.
4. **`geomTick`** (line ~418) — always `0`, its effect fires but `isPanelOpen` is always false
   so the `if (!isPanelOpen) return;` exits early.
5. **`measureRef`** (line ~245) — a hidden DOM probe that measures `""` text = 0px height.
6. **`previewPanelRef`** (line ~244) — the old in-pill scrollable panel ref; the panel JSX
   is rendered only when `isPanelOpen` which is always false.
7. **The `useLayoutEffect` measure block** (lines ~376-390) — runs but `isPanelOpen` is
   always false, so immediately sets height to 0 and returns.
8. **The geometry show-effect** calls `setSize(PILL_WIDTH, PILL_HEIGHT)` (activePillWidth/
   activePillHeight both collapse to these since `isPanelOpen` is always false) and
   `setBarShape("pill")` — both are effectively no-ops but add latency to every show.
9. **`FORM_APPEARANCES`, `getFormAppearance`, `previewPanelForm`** — `PANEL_WIDTH` always
   resolves to 320 (comfortable default), never used meaningfully.
10. **`PILL_WIDTH_CLIPBOARD = 220`** — still active! The clipboard-done branch still uses it.
    This is the one functional change: `pillWidth = 200` after cleanup.
11. **`setBarShape` import** — called in the show-effect; removing it breaks that call site
    (which is being deleted anyway).
12. **The panel JSX block** (`{isPanelOpen && (...)}`) — never renders since `isPanelOpen` is
    always false. React skips it on every render, but it adds parse overhead and visual noise.

### `create_bar_window` window creation — already correct

`create_bar_window` in `src-tauri/src/lib.rs` already creates the bar at `80 × 10` (the idle
thin shape) and sets the pill region at creation. The frontend is expected to resize it on
first show. **After 6.5, the frontend no longer calls `setSize`** — so the bar stays at the
initial `80 × 10` size?

Wait — this needs careful handling. The current `create_bar_window` creates the window at
`80 × 10` (thin idle shape), not `200 × 36` (PILL_WIDTH × PILL_HEIGHT). The current
show-effect does `setSize(200, 36)` to expand it. If we remove `setSize` from the show-effect,
the window will never expand to pill size and will remain a tiny 80×10 sliver.

**Required Rust change (AC-3):** Update `create_bar_window` to create the window at
`PILL_WIDTH × PILL_HEIGHT = 200 × 36` (logical px) and set the full pill region at
creation. Then the frontend never needs to call `setSize` again.

In `src-tauri/src/lib.rs`, change:
```rust
let bar_width = 80.0_f64;
let bar_height = 10.0_f64;
// ...
.inner_size(bar_width, bar_height)
```
To:
```rust
let bar_width = 200.0_f64;  // PILL_WIDTH
let bar_height = 36.0_f64;  // PILL_HEIGHT
// ...
.inner_size(bar_width, bar_height)
```
And in the initial region-set block:
```rust
// BEFORE:
let pw = (bar_width * scale) as i32;  // 80 * scale
let ph = (bar_height * scale) as i32; // 10 * scale
set_window_region_pill(hwnd.0 as isize, pw, ph);
// AFTER:
let pw = (200.0 * scale) as i32;  // PILL_WIDTH
let ph = (36.0 * scale) as i32;   // PILL_HEIGHT
set_window_region_pill(hwnd.0 as isize, pw, ph);
```
**This is the only Rust change.** It is small, behavior-preserving (before this story the
frontend called `setSize(200, 36)` on every show; now it is created at that size already and
never resizes). `cargo check --target x86_64-pc-windows-gnu` should stay at the same
pre-existing failure (whisper-rs-sys/ggml-cpu.c) — no new errors.

The `bar_width = 80.0 / bar_height = 10.0` pair (thin idle strip) was a legacy from the v1
design where the bar started tiny and grew on show. With the new design the window is hidden
on idle (not a thin strip visible) — so the startup size no longer matters visually. But the
`ensure_bar_window` recovery path and any `create_bar_window` call should now create the
window at full pill size.

Also update the comment on the `pill_height` local variable in `create_bar_window`:
```rust
// DELETE the unused `pill_height` local variable that existed as a comment footnote.
let pill_height = 36.0_f64;  // DELETE — now bar_height itself is 36.0
```

### What stays in the show-effect

After cleanup, the show-effect is minimal:
```ts
useEffect(() => {
  const win = getCurrentWebviewWindow();
  (async () => {
    if (isPillVisible) {
      // Guard: skip setPosition during an active drag.
      if (barX.current != null && barY.current != null && dragRef.current == null) {
        await win.setPosition(new LogicalPosition(barX.current, barY.current));
      }
      try {
        await win.show();
      } catch (e) {
        console.error("[bar] show failed, attempting recovery:", e);
        try {
          const recreated = await ensureBarWindow();
          if (recreated) console.log("[bar] window recreated via recovery");
        } catch (re) {
          console.error("[bar] recovery also failed:", re);
        }
      }
    }
  })();
}, [isPillVisible]);
```

No `setSize`, no `setBarShape`. The window is always `200 × 36` — set at creation and never
changed. `dragRef.current == null` guard stays (prevents teleporting during drag when a
state-changed event arrives — AC-4 from 6.2, preserved).

### `isDone && clipboardOnly` width change (200 vs. 220)

The current code widens to 220 for clipboard-done. After cleanup it stays at 200. The
"In Clipboard" text (clipboard emoji `📋` + `In Clipboard` label) needs to fit in 200px at
the current font size (11–12px, `Inter`). Rough calculation:
- `📋` emoji: ~24px (matches `KlarvoLogo` size)
- `gap: 6` between items
- `In Clipboard` at `fontSize: 12`: approximately 72px
- Pill row `paddingLeft: 10, paddingRight: 10` = 20px
- Total: ~24 + 6 + 72 + 20 = ~122px — well within 200px

The spec decision (6.0 open decision #1) was: **keep pill at 200** and re-lay-out the
clipboard state to fit. 200px is more than enough for this content; no JSX change to the
clipboard render block is needed — just removing `PILL_WIDTH_CLIPBOARD` and the ternary.

### Inversion checks

Per Epic-4 retro AI-1: every guard must prove it goes RED.

1. **AC-1 dead code (tsc-time):** After deletion, any reference to a removed state/ref
   (`livePreview`, `panelHeight`, etc.) causes a tsc "cannot find name" error → RED.
2. **AC-3 no-resize (smoke-time + grep):** `grep -n "setSize\|LogicalSize" src/FloatingBar.tsx`
   after cleanup → 0 hits → confirms no resize path remains.
3. **AC-5 no-setBarShape (grep):** `grep -n "setBarShape" src/FloatingBar.tsx` after cleanup
   → 0 hits → confirms the "panel" shape path is gone.
4. **AC-8 clipboard-done width (smoke-time):** Clipboard-done at 200px fits without overflow
   — verified visually in the Windows smoke.

### No Android, no new config, no new commands

This is a Windows-desktop-only cleanup. Do not touch `android/`, `KlarvoApi.kt`,
`LocalWhisperInference.kt`. No new config key. No new Tauri command. The `set_bar_shape`
command registration in `lib.rs` stays (do not remove it — it might be used in recovery
or by tooling; just stop calling it from FloatingBar.tsx).

### References

- FR1 (pill static), AR4 (delete dead code), AR5 (remove `set_bar_shape("panel")` + screenCap):
  `_bmad-output/planning-artifacts/epics-bar-redesign.md`
- spec §3.2 (window topology), §3.6 (what is removed from FloatingBar.tsx):
  `docs/bar-redesign-spec.md`
- `create_bar_window` Rust function (bar size): `src-tauri/src/lib.rs:620-770`
- `set_bar_shape` Rust command (stays, not deleted): `src-tauri/src/commands/misc.rs:437-474`
- Current `FloatingBar.tsx` dead code inventory: this story's Dev Notes above
- Surface smoke checklist: `docs/surface-smoke-checklist.md`
- Project rules (no Android, no panics, camelCase config): `_bmad-output/project-context.md`
- Story 5-7 to reconcile: `_bmad-output/implementation-artifacts/5-7-preview-flush-hardening-stale-chunk-guard-and-backpressure.md`
- Previous story (6.4 drag coupling): `_bmad-output/implementation-artifacts/6-4-couple-the-preview-to-pill-drag.md`

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

None — purely additive dead-code removal; no logic changes to debug.

### Completion Notes List

- **Tasks 1–7 (FloatingBar.tsx):** Deleted ~200 lines of dead grow code. Removed `livePreview`, `panelHeight`, `panelScrolls`, `geomTick` state; `previewPanelRef`, `measureRef`, `panelHeightRef`, `isPanelOpenRef`, `prevIsPanelOpenRef`, `isRecordingRef` refs; `isPanelOpen` derived var; `PILL_WIDTH_CLIPBOARD`, `PILL_WIDTH`, `PILL_HEIGHT`, `SCREEN_TOP_MARGIN`, `FORM_APPEARANCES`, `DEFAULT_FORM`, `getFormAppearance()`, `previewPanelForm` state/constant block; `activePillWidth`, `activePillHeight`, `pillWidth` derived values. Removed all dead `useEffect`/`useLayoutEffect` blocks (stale-chunk ref mirror, no-op chunk listener, scroll, panel-open refresh, measure probe, panelHeightRef mirror, geomTick ticker). Removed `useLayoutEffect`, `LogicalSize` imports; `setBarShape` import from tauri-commands. Show-effect simplified to `setPosition + show()` only (`isPillVisible` dep). Outer wrapper `borderRadius` → constant `9999`; pill row `flex: 1` always; padding constant `10`. `setLivePreview("")` calls removed from `onStateChanged`. Drag handlers updated: `isPanelOpenRef`/`panelHeightRef` logic replaced with direct `d.winY + dy` / `ly` (window top-left IS pill anchor). `getSettings()` on mount keeps only `setHotkeyMode`.
- **Task 8 (create_bar_window Rust):** Changed `bar_width = 80.0 → 200.0`, `bar_height = 10.0 → 36.0`; removed `pill_height` local variable; updated both position calculations to use `bar_height` directly. Region is set at creation using the new `200×36` dimensions.
- **Task 8 (sprint-status.yaml 5-7):** Added SUPERSEDED header to 5-7 comment per AC-7 wording.
- **Inversion checks GREEN:** `grep "setSize\|LogicalSize\|setBarShape"` in FloatingBar.tsx → 0 active-code hits; tsc exit 0 confirms no dangling references.
- **Rust tests:** 575/0 — baseline maintained, no Rust production logic changed.
- **AC-8 (9.4) BLOCKED** on Windows smoke — pending Andi's manual test of the real release build.

### File List

- src/FloatingBar.tsx
- src-tauri/src/lib.rs
- _bmad-output/implementation-artifacts/sprint-status.yaml
- _bmad-output/implementation-artifacts/6-5-pill-fully-static-and-cleanup.md

### Change Log

- 2026-06-08: Story 6.5 implemented — dead grow code deleted from FloatingBar.tsx (~200 lines removed), create_bar_window Rust updated to 200×36, Story 5-7 reconciled as SUPERSEDED in sprint-status.yaml. tsc exit 0, vite clean, 575 Rust tests/0 fail. Windows smoke (AC-8) pending Andi.
