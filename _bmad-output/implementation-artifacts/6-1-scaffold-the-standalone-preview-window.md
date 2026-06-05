---
story: "6.1"
epic: "6"
title: "Scaffold the standalone "preview" window"
status: done
track: L3-feature
gatedBy: []
buildsOn: []
enabledBy: ["6.2"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics-bar-redesign.md
  - docs/bar-redesign-spec.md
  - docs/deep-dive-bar-subsystem.md
  - _bmad-output/project-context.md
  - docs/surface-smoke-checklist.md
---

# Story 6.1: Scaffold the standalone "preview" window

Status: done

## Story

As a developer re-architecting the bar,
I want a standalone transparent, click-through `"preview"` window created, routed, and recoverable,
so that live preview can render in its own window fully decoupled from the pill.

## Acceptance Criteria

**AC-1 — Window created hidden at startup:**
Given the app starts on Windows
When the `setup` closure runs (after `create_bar_window`)
Then a second WebView window labeled `"preview"` is created **hidden** via a new `create_preview_window`
helper, with: `inner_size(1.0, 1.0)` (tiny hidden initial size), `decorations(false)`,
`transparent(true)`, `always_on_top(true)`, `resizable(false)`, `skip_taskbar(true)`,
`focused(false)`, `shadow(false)` (Windows), and `set_ignore_cursor_events(true)` called on the
built handle (Tauri 2 click-through API — confirmed in tauri-2.10.3)
And the window creation is gated `#[cfg(target_os = "windows")]` — mirrors the bar's boot gate

**AC-2 — `main.tsx` routes `"preview"` to `PreviewPanel`:**
Given both the `"bar"` and `"preview"` windows exist
When `main.tsx` evaluates the current window label
Then `"preview"` routes to a new `PreviewPanel` component (alongside `"main"` → App, `"bar"` →
FloatingBar); `PreviewPanel` renders **only** `RESET_CSS` for now (no content, no subscriptions)
And the routing uses a dynamic import pattern identical to FloatingBar's (keeps Tauri API off the
module-eval path in a plain browser)
And `PreviewPanel` is a new file `src/PreviewPanel.tsx`

**AC-3 — `ensure_preview_window` recovery command:**
Given the preview window vanished or is unresponsive
When `ensure_preview_window` is invoked by the frontend (or future recovery path)
Then it probes `get_webview_window("preview").is_visible()` and recreates via
`create_preview_window` when missing/unresponsive (mirrors the `ensure_bar_window` pattern
in `commands/misc.rs:208`), returning `true` when recreated
And the command is `#[cfg(desktop)]`, `async`, and registered in the `invoke_handler`
And a TS wrapper `ensurePreviewWindow(): Promise<boolean>` is added to `tauri-commands.ts`
(returns `false` in preview/Android mode — mirrors `ensureBarWindow`)

**AC-4 — `CloseRequested` is prevented for the preview window:**
Given the preview window receives a close event
When `CloseRequested` fires
Then the window is hidden and the close is prevented — same as the `"bar"` window handling in
`lib.rs:951`
And the `on_window_event` branch covers both `"bar"` and `"preview"` labels

**AC-5 — Click-through is enforced:**
Given the scaffolded `"preview"` window is shown at any position
When the user moves the cursor over it
Then cursor events pass through to the underlying window (the window is display-only, no controls)
And the implementation calls `preview.set_ignore_cursor_events(true)` on the built window handle
immediately after `builder.build()?` — this is the correct Tauri 2 API (confirmed in tauri-2.10.3)

**AC-6 — Smoke: show/position/hide no artifact:**
Given the scaffolded window
When it is shown at a fixed test size (e.g. 320×400) and position (e.g. centered on screen) and
then hidden
Then it shows, positions, and hides with **no white-line / shape artifact**
And the Windows smoke verifies this (see DoD)

**DoD:**
- Real Windows release build via `scripts/sync-and-build.ps1` + manual smoke:
  - Show the preview window at a fixed size/position (temporary test call in dev), verify it is
    visible, transparent, click-through, and has no shape artifact; then hide it
  - Confirm the `"bar"` window still works as before (no regression)
- Pre-smoke trap checks from `docs/surface-smoke-checklist.md`:
  - Trap #5 (new event/push wiring): N/A — no new events in this story
  - Trap #3 (separate-window reactivity): N/A — no settings reading in the stub
  - Trap #4 (geometry/region): if a test region is set, verify radius matches CSS; the stub has
    no CSS card yet so this is low-risk; any explicit `set_window_region_*` call MUST be verified
- `cargo check --target x86_64-pc-windows-gnu` green (touches `lib.rs`, `commands/misc.rs`)
- Linux `cargo test` green (no logic change; Rust test coverage limited here per testing rules)
- `tsc` / `npm run build` green

## Tasks / Subtasks

- [x] Task 1: Add `create_preview_window` Rust helper (AC-1)
  - [x] 1.1 In `src-tauri/src/lib.rs`, add a `#[cfg(desktop)]` function
    `pub fn create_preview_window<M: tauri::Manager<tauri::Wry>>(app: &M) -> Result<(), Box<dyn std::error::Error>>`
    that builds the `"preview"` WebviewWindow with all required flags:
    `inner_size(1.0, 1.0)`, `decorations(false)`, `transparent(true)`, `always_on_top(true)`,
    `resizable(false)`, `skip_taskbar(true)`, `focused(false)`, `shadow(false)` (Windows-gated);
    then call `preview.set_ignore_cursor_events(true)` on the built handle — see Dev Notes
  - [x] 1.2 **Do NOT** call `set_position` or `set_window_region_*` in `create_preview_window` —
    the preview has no meaningful position at startup (Story 6.2 sets position on first chunk)
  - [x] 1.3 The preview window's routing `index.html` is the same bundle as the bar (same
    `WebviewUrl::App("index.html".into())`) — `main.tsx` does the routing by label

- [x] Task 2: Boot call in the `setup` closure (AC-1)
  - [x] 2.1 In the `setup` closure in `lib.rs`, immediately after the
    `#[cfg(target_os = "windows")] create_bar_window(...)` call (line ~870), add:
    ```rust
    #[cfg(target_os = "windows")]
    if let Err(e) = create_preview_window(app) {
        log::warn!("[setup] Could not create preview window: {e}");
    }
    ```
  - [x] 2.2 Also gate `CloseRequested` handling for `"preview"`: in the `on_window_event` closure
    (line ~951), add `|| label == "preview"` to the `"bar"` arm so both windows prevent close +
    hide (AC-4)

- [x] Task 3: `ensure_preview_window` command (AC-3)
  - [x] 3.1 In `src-tauri/src/commands/misc.rs`, add a `#[cfg(desktop)]` `#[tauri::command]`
    `pub async fn ensure_preview_window(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<bool, String>`
    that mirrors `ensure_bar_window` exactly: probe `get_webview_window("preview").is_visible()`,
    recreate via `create_preview_window(&app)` if missing/unresponsive, return true/false
    — no `saved_x/saved_y` needed (position set at show-time, not at creation)
  - [x] 3.2 Register in `lib.rs` `invoke_handler`:
    ```rust
    #[cfg(desktop)]
    commands::misc::ensure_preview_window,
    ```
    (next to the existing `ensure_bar_window` registration)

- [x] Task 4: `main.tsx` routing update (AC-2)
  - [x] 4.1 Add a `label === "preview"` branch using the same dynamic-import pattern as `"bar"`:
    ```tsx
    if (label === "bar" && !isPreviewMode) {
      const { default: FloatingBar } = await import("./FloatingBar");
      Root = FloatingBar;
    } else if (label === "preview" && !isPreviewMode) {
      const { default: PreviewPanel } = await import("./PreviewPanel");
      Root = PreviewPanel;
    } else {
      Root = App;
    }
    ```
  - [x] 4.2 Create `src/PreviewPanel.tsx` — a minimal stub with `RESET_CSS` only, no subscriptions
  - [x] 4.3 Add `import React from "react";` at the top of `PreviewPanel.tsx`

- [x] Task 5: `tauri-commands.ts` TS wrapper (AC-3)
  - [x] 5.1 At the end of `src/tauri-commands.ts`, after the `ensureBarWindow` export, add:
    ```ts
    // --- Preview window recovery ---
    export async function ensurePreviewWindow(): Promise<boolean> {
      if (isPreviewMode) return false;
      return invoke<boolean>("ensure_preview_window");
    }
    ```

- [x] Task 6: Verify and close (AC-6, DoD)
  - [x] 6.1 `cargo check --target x86_64-pc-windows-gnu` — pre-existing `ort-sys` failure (no
    downloaded binaries for x86_64-pc-windows-gnu); IDENTICAL to baseline without these changes;
    no NEW errors on touched files
  - [x] 6.2 `cargo test` (Linux) — **572 passed, 0 failed** (green, no regressions)
  - [x] 6.3 `tsc` / `npm run build` — **green** (`tsc` + `vite build` clean; PreviewPanel
    built as separate 0.21 kB chunk, dynamic import confirmed working; fixed `JSX.Element` →
    `React.ReactElement` return type annotation)
  - [x] 6.4 Windows smoke via `scripts/sync-and-build.ps1` — **no-regression confirmed (Andi,
    2026-06-05): "wie immer", bar unaffected, nothing new.** The active transparency/click-through
    portion was NOT exercised here: Story 6.1 creates the preview window **hidden with no position**
    (`set_position` is deferred to 6.2), so there is no trigger to show it. Rather than add a
    throwaway test-show call, the transparency / click-through / `CloseRequested`-prevention
    smoke-time inversions are **carried forward into the 6.2 surface smoke** (where the preview
    window is shown for real with content). See 6.2 DoD.

## Dev Notes

### Key files to touch

| File | Change |
|---|---|
| `src-tauri/src/lib.rs` | Add `create_preview_window` helper; boot call in `setup`; `CloseRequested` extension |
| `src-tauri/src/commands/misc.rs` | Add `ensure_preview_window` command (mirrors `ensure_bar_window`) |
| `src/main.tsx` | Add `label === "preview"` branch (dynamic import) |
| `src/PreviewPanel.tsx` | **New file** — empty transparent stub, imports React, exports default |
| `src/tauri-commands.ts` | Add `ensurePreviewWindow()` wrapper at the bottom |

### `create_bar_window` as the exact blueprint

Copy the structure of `create_bar_window` at `lib.rs:603`. Key differences for the preview:
- Label is `"preview"` (not `"bar"`)
- Initial size is `inner_size(1.0, 1.0)` (invisible; 6.2 resizes to `maxHeight × width` on first chunk)
- **Add** `preview.set_ignore_cursor_events(true)` after `build()` — this is what makes it click-through
- **Omit** `set_window_region_*` — no region at creation (region set once per show in 6.2, after size is known)
- **Omit** position logic — no saved position for the preview (derived from pill anchor in 6.2)
- The `shadow(false)` Windows gate and `decorations(false)`/`transparent(true)` are identical

### Click-through: `set_ignore_cursor_events`

```rust
let preview = builder.build()?;
let _ = preview.set_ignore_cursor_events(true);
```
This is the correct Tauri 2 API (confirmed in `tauri-2.10.3`; the method is on the
`WebviewWindow` handle, NOT on the builder). There is no builder-level `.cursor_events(...)` in
Tauri 2 — the method must be called after `.build()`. Fail-soft: use `let _ =` and log a warning
on error; the window is still useful (just not click-through) if the call fails.

### `ensure_bar_window` as the exact blueprint for `ensure_preview_window`

The existing `ensure_bar_window` at `commands/misc.rs:207` is the model. Mirror it exactly:
```rust
#[cfg(desktop)]
#[tauri::command]
pub async fn ensure_preview_window(
    app: tauri::AppHandle,
    _state: State<'_, AppState>,  // not needed (no saved pos), but keep signature consistent
) -> Result<bool, String> {
    if let Some(preview) = app.get_webview_window("preview") {
        match preview.is_visible() {
            Ok(_) => return Ok(false),
            Err(e) => log::warn!("[preview] ensure_preview_window: not responding: {e}"),
        }
    } else {
        log::warn!("[preview] ensure_preview_window: window not found, recreating");
    }
    match crate::create_preview_window(&app) {
        Ok(_) => { log::info!("[preview] ensure_preview_window: recreated"); Ok(true) }
        Err(e) => { log::error!("[preview] ensure_preview_window: failed: {e}"); Err(format!("Failed: {e}")) }
    }
}
```

### `main.tsx` routing — preserve the existing structure

Current `main.tsx` (33 lines total):
```tsx
if (label === "bar" && !isPreviewMode) {
  const { default: FloatingBar } = await import("./FloatingBar");
  Root = FloatingBar;
} else {
  Root = App;
}
```
Replace the `else` block with:
```tsx
} else if (label === "preview" && !isPreviewMode) {
  const { default: PreviewPanel } = await import("./PreviewPanel");
  Root = PreviewPanel;
} else {
  Root = App;
}
```
This keeps the module-eval-path guard (`!isPreviewMode`) intact.

### `CloseRequested` extension

The existing handler at `lib.rs:951`:
```rust
if label == "bar" {
    let _ = window.hide();
    api.prevent_close();
}
```
Change to:
```rust
if label == "bar" || label == "preview" {
    let _ = window.hide();
    api.prevent_close();
}
```

### Why `inner_size(1.0, 1.0)` at creation

The preview window's size is determined at show-time (Story 6.2 computes `clampedMaxHeight × width`
from `previewGeometry`). Creating it at 1×1 (vs `80×10` for the bar's idle skeleton) ensures it is
truly invisible before 6.2 sets the real geometry. The bar starts at 80×10 because it has a real
idle state; the preview has no idle content.

### IMPORTANT: no region call at creation

`set_window_region_*` calls apply the OS clip mask. Do NOT call this in `create_preview_window` —
there is no meaningful shape yet (the preview is 1×1 hidden). Story 6.2 sets the rounded-rect
region once, right before `show()`, using the computed `clampedMaxHeight × width`.

### Surface-smoke-checklist traps that apply

- **Trap #5 (event wiring):** N/A in this story — no new events, no subscriptions.
- **Trap #4 (geometry/region):** If you add a temporary test show (Task 6.4), set a matching
  region before showing or accept that the test shows an unmasked rectangle. The permanent region
  logic is in 6.2.
- **Trap #3 (separate-window reactivity):** `PreviewPanel` reads NO settings on mount in this story.
  Do not add `getSettings()` to the stub — 6.2/6.3 add reactive reads.

### Windows-only gating discipline

- `create_preview_window` function: `#[cfg(desktop)]` (same as `create_bar_window`)
- Boot call in `setup`: `#[cfg(target_os = "windows")]` (same as the bar's boot call at line 869)
- `ensure_preview_window` command: `#[cfg(desktop)]`
- `set_ignore_cursor_events(true)` works on all desktop platforms, so no extra gate needed for it

### Tauri event naming reminder

The preview window does NOT subscribe to any events in this story. When events ARE added (6.2),
they MUST use colon form: `klarvo://live-preview-chunk`, `klarvo://state-changed`, etc. — never
dot-form. (project-context.md §Framework-Specific Rules)

### No Android change

This is a desktop-only Windows story. The Android overlay bubble is independent (native Kotlin,
no Tauri IPC). Do not touch any Android path.

### Empirical inversion check (Epic-4 retro AI-1 — reviewer-verified, not self-attested)

The reviewer MUST verify. NOTE: 6.1's two inversion targets are window-lifecycle / OS-click-through
behaviours with NO Linux unit test to flip — they are **smoke-time inversions** performed during the
Windows smoke (AC-6 / DoD 6.4), not on Linux. The dev worker must NOT self-attest these:
- Remove the `|| label == "preview"` from the `CloseRequested` guard → close the preview window
  during smoke → app should allow close (preview vanishes) rather than preventing it → RED
- Remove the `preview.set_ignore_cursor_events(true)` call (the real Tauri-2.10.3 click-through API;
  there is NO `cursor_events(CursorEvents::Deny)` builder method) → show the preview full-size →
  clicks land on the preview window instead of passing through → RED
Both must be confirmed RED at smoke time; the dev worker must NOT self-attest these.

### Project Structure Notes

- `create_preview_window` lives in `lib.rs` at the module level, below `create_bar_window` (~line
  730, after the fallback monitor block). Keep the same doc-comment style and `#[cfg(desktop)]` gate.
- `ensure_preview_window` lives in `commands/misc.rs` in the "Window / UI helpers" section (~line
  244), after `ensure_bar_window`.
- `PreviewPanel.tsx` lives in `src/` alongside `FloatingBar.tsx` and `App.tsx`.
- The `invoke_handler!` registration for `ensure_preview_window` goes next to `ensure_bar_window`
  at `lib.rs:1026`.

### References

- `create_bar_window` blueprint: `src-tauri/src/lib.rs:603`
- Bar boot call: `src-tauri/src/lib.rs:869`
- `ensure_bar_window` blueprint: `src-tauri/src/commands/misc.rs:207`
- `CloseRequested` handling: `src-tauri/src/lib.rs:947`
- `invoke_handler` registration: `src-tauri/src/lib.rs:966`
- `main.tsx` current routing: `src/main.tsx:21`
- `tauri-commands.ts` `ensureBarWindow` at end of file: `src/tauri-commands.ts:1033`
- Epic 6 planning + full ACs: `_bmad-output/planning-artifacts/epics-bar-redesign.md#Story 6.1`
- Foundation design + positioning math: `docs/bar-redesign-spec.md`
- Ist-Zustand (race table, effect inventory, file map): `docs/deep-dive-bar-subsystem.md`
- Surface smoke checklist: `docs/surface-smoke-checklist.md`
- Project rules (platform gates, fail-soft, single-writer): `_bmad-output/project-context.md`

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

- Pre-existing `cargo check --target x86_64-pc-windows-gnu` failure: `ort-sys v2.0.0-rc.10` has
  no downloaded binaries for the gnu target; confirmed identical on baseline (without changes).
  Not caused by Story 6.1. The fix would require compiling ONNX Runtime from source for this
  cross-compile target — a pre-existing infrastructure gap.
- `JSX.Element` return type in `PreviewPanel.tsx` caused `tsc error TS2503: Cannot find namespace
  'JSX'`. Fixed by using `React.ReactElement` instead (matches strict-mode React 19 + Vite config).

### Completion Notes List

- Task 1: `create_preview_window` added to `lib.rs` after `create_bar_window` (~line 738).
  `#[cfg(desktop)]` gate, `inner_size(1.0, 1.0)`, all required flags, `shadow(false)` Windows-gated,
  `set_ignore_cursor_events(true)` fail-soft. No `set_position` / `set_window_region_*` called.
- Task 2: Boot call added in `setup` immediately after `create_bar_window` (Windows-gated,
  fail-soft `log::warn`). `CloseRequested` extended to `label == "bar" || label == "preview"`.
- Task 3: `ensure_preview_window` added to `commands/misc.rs`, mirrors `ensure_bar_window` exactly.
  No `saved_x/saved_y` (position derived from pill anchor in 6.2). Registered in `invoke_handler`.
- Task 4: `main.tsx` routing updated with `"preview"` branch (dynamic import pattern identical to
  `"bar"`). `src/PreviewPanel.tsx` created: transparent stub, `RESET_CSS`, `import React`, no subscriptions.
- Task 5: `ensurePreviewWindow(): Promise<boolean>` added to `tauri-commands.ts`, mirrors
  `ensureBarWindow` (returns `false` in preview mode).
- Task 6: Linux `cargo test` = 572 passed / 0 failed. `tsc` + `npm run build` green (PreviewPanel
  built as 0.21 kB separate chunk). Windows smoke pending (AC-6 / DoD 6.4).

### File List

- `src-tauri/src/lib.rs` — added `create_preview_window` helper, boot call in `setup`, extended `CloseRequested`, registered `ensure_preview_window` in `invoke_handler`
- `src-tauri/src/commands/misc.rs` — added `ensure_preview_window` command
- `src/main.tsx` — added `"preview"` routing branch
- `src/PreviewPanel.tsx` — new file (transparent stub, no subscriptions)
- `src/tauri-commands.ts` — added `ensurePreviewWindow()` wrapper
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — status `ready-for-dev` → `in-progress`

## Review Findings

_Code review 2026-06-05 (3 adversarial layers — Blind Hunter / Edge Case Hunter / Acceptance
Auditor, Opus 4.8; conductor-verified against the `ensure_bar_window` / `create_bar_window`
blueprints). Result: 0 decision-needed, 2 patch (both applied), 4 defer, 9 dismissed. No real bug
introduced — every High-flagged item is a faithful mirror of an accepted existing pattern or
by-design for a scaffold story._

- [x] [Review][Patch] `ensure_preview_window` error string less descriptive than its blueprint — `"Failed: {e}"` → `"Failed to recreate preview window: {e}"` [src-tauri/src/commands/misc.rs:283] (AC-3 "mirror `ensure_bar_window` exactly") — APPLIED
- [x] [Review][Patch] Inversion-check section named the phantom `cursor_events(CursorEvents::Deny)` API → corrected to `set_ignore_cursor_events(true)` + flagged both 6.1 inversions as smoke-time (no Linux unit test) [this story file, Empirical inversion check] (Epic-4 retro AI-1 runnability + AC-5) — APPLIED
- [x] [Review][Defer] Recreate path doesn't close the stale handle before `create_preview_window` → in the rare "window exists but unresponsive" case, `WebviewWindowBuilder::new("preview")` errors on the duplicate label and recovery fails (also TOCTOU on concurrent calls) [misc.rs:276] — deferred, **pre-existing: `ensure_bar_window` (misc.rs:234) has the identical pattern**; fix both together
- [x] [Review][Defer] `is_visible()` `Ok(false)` is treated as "alive" — a hidden window passes the liveness probe, and combined with `CloseRequested`-hide it is never re-shown by `ensure_*` [misc.rs:262] — deferred, by-design probe semantics mirroring `ensure_bar_window`; re-show is 6.2's responsibility
- [x] [Review][Defer] `create_preview_window` omits `.visible(false)` — relies on 1×1+transparent for "hidden" [lib.rs ~755] — deferred, story-sanctioned (Dev Notes "truly invisible at 1×1"); add `.visible(false)` + explicit `.show()` when 6.2 introduces show/hide to remove any boot-flash risk
- [x] [Review][Defer] `set_ignore_cursor_events` failure is fail-soft (warn + continue) — harmless at 1×1, but in 6.2 a full-size window with failed click-through becomes a screen-wide click trap [lib.rs ~778] — deferred, story-sanctioned for 6.1; reconsider close-on-fail in 6.2
