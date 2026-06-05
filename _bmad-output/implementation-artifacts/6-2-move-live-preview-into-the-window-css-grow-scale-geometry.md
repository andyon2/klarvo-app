---
story: "6.2"
epic: "6"
title: "Move live preview into the window (CSS-grow, scale geometry)"
status: review
track: L3-feature
gatedBy: ["6.1"]
buildsOn: ["6.1"]
enabledBy: ["6.3", "6.4"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics-bar-redesign.md
  - docs/bar-redesign-spec.md
  - docs/deep-dive-bar-subsystem.md
  - _bmad-output/project-context.md
  - docs/surface-smoke-checklist.md
---

# Story 6.2: Move live preview into the window (CSS-grow, scale geometry)

Status: review

## Story

As a user dictating with preview enabled,
I want the live text to appear in a window above the pill that grows upward and caps + scrolls,
so that I can read along while the pill never moves or resizes.

## Acceptance Criteria

**AC-1 — Show-once on first chunk (geometry set once per recording):**
Given preview is enabled and a recording is active in Toggle/Hold
When the **first** `klarvo://live-preview-chunk` of the cycle arrives
Then `PreviewPanel` calls `previewGeometry(widthPreset, "small")` with the screen clamp (AR3),
sets the window **size + rounded-rect region (radius 14) + position** (centered over the pill,
bottom edge `GAP=8` above the pill top) **once**, then calls `show()`
And `widthPreset` is read reactively from settings at show-time (NOT frozen at mount — separate-window rule)
And the rounded-rect region is applied via a new `set_preview_shape` Rust command (mirrors `set_bar_shape`'s
`"panel"` arm but targets the `"preview"` window)

**AC-2 — CSS-only growth, no per-chunk window IPC (NFR1):**
Given preview chunks continue to arrive
When each chunk is appended
Then the dark card grows **upward via CSS** within the static window (bottom-aligned flex card),
and **no** `setSize` / `setPosition` / region call is issued per chunk
And inversion: re-introducing a per-chunk `setSize` call brings back the cold-expansion / pre-measure
clip (R3/R4) — proving the static-window invariant is load-bearing

**AC-3 — Cap + scroll + top-fade:**
Given the accumulated text exceeds the window's `clampedMaxHeight`
When further chunks arrive
Then the **inner** text area scrolls to the newest line, a **top-fade** masks the oldest lines,
and the window itself does not grow further

**AC-4 — R1 stale-chunk guard retained:**
Given a chunk arrives after the recording ended (stale/out-of-cycle)
When the `klarvo://live-preview-chunk` listener fires
Then the `isRecordingRef` guard (mirrors FloatingBar's R1 pattern) drops it — no text bleeds into the next cycle
And inversion: removing the guard lets a stale post-done chunk repopulate the panel

**AC-5 — Hide and clear on recording end:**
Given the recording ends (`done` / `idle` / `error`)
When `klarvo://state-changed` fires
Then `PreviewPanel` clears the accumulated text, calls `win.hide()` on the preview window, and
resets the `showOnceRef` gate so the geometry sequence runs again on the next recording cycle

**AC-6 — Width preset affects only the preview width; pill is unaffected (FR5):**
Given the width preset is Compact / Comfortable / Wide (260 / 320 / 400 at Small font)
When the preview opens
Then only the preview window width changes; the pill window is NOT resized

**AC-7 — Pill in-window preview path disabled (exactly one preview surface):**
Given the pill (`FloatingBar`) is in any active state
When preview is active
Then the old in-pill grow path is disabled: `isPanelOpen`-triggered geometry / `setBarShape("panel")` /
the `livePreview` listener append in FloatingBar no longer fires; exactly one preview surface exists.
Dead pill code is **not** deleted here (that is Story 6.5) — it is **disabled** by making the pill
stop listening to `live-preview-chunk` events (or by short-circuiting on an always-false gate).

**AC-8 — Smoke: text grows upward, caps + scrolls, no top-clip on first chunk, non-resizing pill:**
Given a real Windows release build
When the smoke is run
Then preview text grows upward, centered above a non-resizing pill, no top-clip on first chunk;
caps and scrolls with top-fade; window transparency / click-through / no white-line verified;
and the 6.1 carried-forward inversions (click-through, CloseRequested) are confirmed at this smoke

**DoD:**
- Real Windows release build via `scripts/sync-and-build.ps1` + manual smoke:
  - Hold/Toggle with preview enabled → text grows upward, centered, no top-clip on first chunk
  - Text fills past cap → scrolls with top-fade, window does not grow
  - Done/idle → preview hides, no stale bleed into next cycle
  - Confirm pill never resizes (Compact / Comfortable / Wide all show at 200×36 pill)
  - Click-through: cursor passes through the preview window to the window beneath (6.1 carried-forward)
  - No white-line / shape artifact at the corners (R11: region radius matches CSS radius)
  - `CloseRequested` on preview window is prevented + hidden (6.1 carried-forward)
- Pre-smoke trap checks from `docs/surface-smoke-checklist.md`:
  - Trap #5 (event push-wiring): `klarvo://live-preview-chunk` → PreviewPanel; `klarvo://state-changed` → PreviewPanel; verify producer emits AND consumer subscribes end-to-end
  - Trap #3 (separate-window reactivity): `previewPanelForm` MUST be re-read at show-time (NOT from a mount-only `getSettings()`); frozen-at-startup = wrong width preset after a Settings save
  - Trap #4 (geometry/region): `set_preview_shape` radius (14) MUST match CSS `borderRadius` (14); mismatch = white-line artifact (R11)
- `cargo check --target x86_64-pc-windows-gnu` green (touches `commands/misc.rs`, `lib.rs`, `PreviewPanel.tsx`, `tauri-commands.ts`)
- Linux `cargo test` green
- `tsc` / `npm run build` green

## Tasks / Subtasks

- [x] Task 1: New Rust command `set_preview_shape` (AC-1)
  - [x] 1.1 In `src-tauri/src/commands/misc.rs`, after `set_bar_shape`, add:
    ```rust
    /// Applies a rounded-rect OS window region to the "preview" window.
    /// Called once per show (size is already set before this is called).
    /// `#[cfg(target_os = "windows")]` body only; no-op on other platforms.
    #[tauri::command]
    pub fn set_preview_shape(handle: AppHandle) -> Result<(), String> {
        #[cfg(target_os = "windows")]
        {
            use tauri::Manager;
            if let Some(preview) = handle.get_webview_window("preview") {
                let scale = preview.scale_factor().unwrap_or(1.0);
                if let Ok(hwnd) = preview.hwnd() {
                    let h = hwnd.0 as isize;
                    if let Ok(size) = preview.inner_size() {
                        let w = size.width as i32;
                        let ht = size.height as i32;
                        let r = (14.0 * scale) as i32;
                        crate::set_window_region_round_rect(h, w, ht, r);
                    }
                }
            }
        }
        let _ = handle; // suppress unused on non-Windows
        Ok(())
    }
    ```
    - Radius 14 MUST match the CSS `borderRadius: 14` of the dark card — R11 invariant
    - Uses `preview.inner_size()` (reads actual size after `setSize` was called) — mirrors `set_bar_shape "panel"` pattern
  - [x] 1.2 Register in `lib.rs` `invoke_handler` (next to `set_bar_shape`):
    ```rust
    commands::misc::set_preview_shape,
    ```
  - [x] 1.3 Add TS wrapper to `src/tauri-commands.ts` (after `setBarShape`):
    ```ts
    export async function setPreviewShape(): Promise<void> {
      if (isPreviewMode) return;
      await invoke("set_preview_shape");
    }
    ```

- [x] Task 2: `previewGeometry` helper (AC-1, AC-6)
  - [x] 2.1 Add to `src/PreviewPanel.tsx` before the component:
    ```ts
    const FONT_PX = { small: 11, medium: 13, large: 15 } as const;
    const BASE_WIDTH = { compact: 260, comfortable: 320, wide: 400 } as const;
    const BASE_MAX_HEIGHT = 600;
    const GAP = 8; // px between preview bottom and pill top
    const CARD_RADIUS = 14; // matches set_preview_shape + CSS borderRadius (R11)

    function previewGeometry(
      widthPreset: string,
      fontSize: "small" | "medium" | "large" = "small",
    ) {
      const fontPx = FONT_PX[fontSize] ?? FONT_PX.small;
      const k = fontPx / FONT_PX.small;
      const baseW = BASE_WIDTH[widthPreset as keyof typeof BASE_WIDTH] ?? BASE_WIDTH.comfortable;
      return {
        fontPx,
        width: Math.round(baseW * k),
        maxHeight: Math.round(BASE_MAX_HEIGHT * k),
      };
    }
    ```
    - Story 6.3 adds the font axis; this story uses `"small"` only
    - `widthPreset` comes from a **reactive settings read at show-time** (see Task 4)

- [x] Task 3: State/refs in `PreviewPanel` (AC-1–AC-5)
  - [x] 3.1 Replace the current stub body with:
    ```tsx
    import React, { useState, useEffect, useRef } from "react";
    import { listen } from "@tauri-apps/api/event";
    import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
    import { LogicalSize, LogicalPosition } from "@tauri-apps/api/dpi";
    import { currentMonitor, monitorFromPoint } from "@tauri-apps/api/window";
    import type { HotkeyMode } from "./types";
    import {
      getSettings,
      getBarPosition,
      setPreviewShape,
      isPreviewMode,
    } from "./tauri-commands";
    ```
  - [x] 3.2 Component state and refs:
    ```ts
    const [livePreview, setLivePreview] = useState("");
    const [panelScrolls, setPanelScrolls] = useState(false);
    // Stale-chunk guard (mirrors FloatingBar's isRecordingRef, R1).
    const isRecordingRef = useRef(false);
    // One-shot gate: set to true after the first-chunk geometry sequence runs,
    // cleared on recording-end so geometry re-runs next cycle.
    const showOnceRef = useRef(false);
    // Max height in logical px set during the show sequence; used for cap/scroll logic.
    const clampedMaxHeightRef = useRef(BASE_MAX_HEIGHT);
    // Saved pill anchor (logical px) — read from getBarPosition at show-time.
    const pillXRef = useRef<number | null>(null);
    const pillYRef = useRef<number | null>(null);
    ```

- [x] Task 4: `klarvo://state-changed` subscription (AC-5)
  - [x] 4.1 Subscribe to `klarvo://state-changed` with `onStateChanged`:
    ```ts
    useEffect(() => {
      const unlisten = onStateChanged((payload) => {
        const s = payload.state as string;
        if (s === "recording") {
          // Arm for the next cycle.
          isRecordingRef.current = true;
          // Clear stale state from the previous cycle.
          setLivePreview("");
          setPanelScrolls(false);
        } else if (s === "done" || s === "idle" || s === "error") {
          isRecordingRef.current = false;
          showOnceRef.current = false;
          setLivePreview("");
          setPanelScrolls(false);
          // Hide the preview window.
          const win = getCurrentWebviewWindow();
          win.hide().catch((e) => console.warn("[preview] hide failed:", e));
        }
      });
      return () => { unlisten.then((fn) => fn()); };
    }, []);
    ```
  - [x] 4.2 Import `onStateChanged` from `./tauri-commands`

- [x] Task 5: `klarvo://live-preview-chunk` subscription with show-once geometry (AC-1–AC-4)
  - [x] 5.1 Subscribe to `klarvo://live-preview-chunk`:
    ```ts
    useEffect(() => {
      const unlisten = listen<string>("klarvo://live-preview-chunk", async (event) => {
        // R1 stale-chunk guard (AC-4).
        // Inversion: remove this → stale post-done chunk repopulates livePreview.
        if (!isRecordingRef.current) return;
        const chunk = event.payload.trim();
        if (!chunk) return;

        // Show-once geometry sequence: runs exactly once per recording cycle.
        // No resize/reposition per chunk — NFR1.
        // Inversion: moving setSize here re-introduces R3/R4 cold-expansion clip.
        if (!showOnceRef.current) {
          showOnceRef.current = true;
          await runShowSequence();
        }

        setLivePreview((prev) => (prev ? prev + " " + chunk : chunk));
      });
      return () => { unlisten.then((fn) => fn()); };
    }, []);
    ```

- [x] Task 6: `runShowSequence` — geometry computed once per cycle (AC-1, AC-2, AC-6)
  - [x] 6.1 Add `runShowSequence` function inside the component (reads live refs):
    ```ts
    async function runShowSequence() {
      const win = getCurrentWebviewWindow();
      try {
        // Read pill anchor (latest position).
        const saved = await getBarPosition();
        if (saved) {
          pillXRef.current = saved.x;
          pillYRef.current = saved.y;
        } else {
          // Pill position unavailable — skip show this cycle.
          console.warn("[preview] runShowSequence: no pill position, skipping");
          showOnceRef.current = false;
          return;
        }
        const pillX = pillXRef.current!;
        const pillY = pillYRef.current!;

        // Read widthPreset reactively (NOT from mount-time state — separate-window rule).
        // Trap #3: a mount-only getSettings() freezes on app-start value.
        let widthPreset = "comfortable";
        try {
          const s = await getSettings();
          widthPreset = s.previewPanelForm ?? "comfortable";
        } catch (e) {
          console.warn("[preview] getSettings failed, using comfortable:", e);
        }

        const geom = previewGeometry(widthPreset, "small");

        // Compute clampedMaxHeight: min(geom.maxHeight, room above pill).
        // Screen clamp so the window never runs off the top (AR3).
        let clampedMaxH = geom.maxHeight;
        try {
          const monitor = await monitorFromPoint(pillX, pillY) ?? await currentMonitor();
          if (monitor) {
            const scale = monitor.scaleFactor || 1;
            const screenTop = monitor.position.y / scale; // logical px
            const room = (pillY - GAP) - (screenTop + 12);
            clampedMaxH = Math.max(40, Math.min(geom.maxHeight, room));
          }
        } catch (e) {
          console.warn("[preview] monitor clamp failed, using unclipped height:", e);
        }
        clampedMaxHeightRef.current = clampedMaxH;

        const W = geom.width;
        const H = clampedMaxH;

        // Center the preview over the pill, clamping to screen edges.
        // preview.left = clamp(pillCenterX - W/2, screenLeft+12, screenRight-W-12)
        // preview.top  = pillY - GAP - H
        // (Screen edge clamp is approximate here; full clamp uses monitor bounds.)
        const PILL_WIDTH = 200;
        const pillCenterX = pillX + PILL_WIDTH / 2;
        const previewLeft = pillCenterX - W / 2; // screen clamp added if monitor available
        const previewTop = pillY - GAP - H;

        // Sequence: setSize → set_preview_shape (region) → setPosition → show.
        // Order is CRITICAL: region must be applied after size is set (inner_size() in Rust reads
        // the actual size). Show must be last.
        await win.setSize(new LogicalSize(W, H));
        await setPreviewShape();
        await win.setPosition(new LogicalPosition(previewLeft, previewTop));
        await win.show();

        console.log(`[preview] shown: ${W}x${H} at (${previewLeft.toFixed(0)}, ${previewTop.toFixed(0)})`);
      } catch (e) {
        console.error("[preview] runShowSequence failed:", e);
        showOnceRef.current = false; // allow retry on next chunk
      }
    }
    ```

- [x] Task 7: Auto-scroll to newest text (AC-3)
  - [x] 7.1 Add a ref for the scrollable panel element and auto-scroll effect:
    ```ts
    const previewPanelRef = useRef<HTMLDivElement>(null);
    useEffect(() => {
      if (panelScrolls && previewPanelRef.current) {
        previewPanelRef.current.scrollTop = previewPanelRef.current.scrollHeight;
      }
    }, [livePreview, panelScrolls]);
    ```
  - [x] 7.2 Update `panelScrolls` when `livePreview` changes (comparing rendered height to cap):
    Use a `useLayoutEffect` that reads `previewPanelRef.current?.scrollHeight` and compares to
    `clampedMaxHeightRef.current`; set `setPanelScrolls(true)` when overflowing:
    ```ts
    useLayoutEffect(() => {
      if (!previewPanelRef.current) return;
      const contentH = previewPanelRef.current.scrollHeight;
      if (contentH > clampedMaxHeightRef.current) {
        setPanelScrolls(true);
      }
    }, [livePreview]);
    ```

- [x] Task 8: Disable the pill's in-window live-preview path (AC-7)
  - [x] 8.1 In `src/FloatingBar.tsx`, locate the `klarvo://live-preview-chunk` listener
    (effect #5 at line ~326). Short-circuit it so it never appends to `livePreview`:
    ```ts
    useEffect(() => {
      const unlisten = listen<string>("klarvo://live-preview-chunk", (_event) => {
        // Story 6.2: preview moved to the "preview" window (PreviewPanel.tsx).
        // This listener is intentionally disabled. Dead code cleaned up in Story 6.5.
        return;
      });
      return () => { unlisten.then((fn) => fn()); };
    }, []);
    ```
  - [x] 8.2 The geometry effect in FloatingBar (effect #11) uses `isPanelOpen` which derives from
    `livePreview.length > 0`. Since `livePreview` in FloatingBar never grows (listener disabled),
    `isPanelOpen` is always `false` during recording → `setBarShape("panel")` never fires →
    pill stays at `PILL_WIDTH × PILL_HEIGHT` (pill shape only).
    **Do NOT delete any FloatingBar state or effects in this story** — that is 6.5.

- [x] Task 9: Render the preview card (CSS-grow, bottom-aligned) (AC-2, AC-3, UX-DR1)
  - [x] 9.1 Replace `PreviewPanel`'s return with a full render:
    ```tsx
    const RESET_CSS_STYLE: React.CSSProperties = {
      margin: 0, padding: 0,
      background: "transparent",
      overflow: "hidden",
      width: "100vw", height: "100vh",
    };

    // Outer wrapper: full window, bottom-aligned flex column so the card
    // sits at the bottom (bottom edge = window bottom = GAP above pill top).
    return (
      <>
        <style>{`
          *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
          html, body, #root {
            width: 100%; height: 100%;
            overflow: hidden !important;
            background: transparent !important;
          }
          ::-webkit-scrollbar { display: none !important; width: 0 !important; height: 0 !important; }
        `}</style>
        <div style={{
          width: "100%", height: "100%",
          display: "flex", flexDirection: "column",
          justifyContent: "flex-end", // card grows upward from bottom
        }}>
          {livePreview && (
            <>
              <style>{`
                #preview-card::-webkit-scrollbar { display: block !important; width: 4px !important; }
                #preview-card::-webkit-scrollbar-thumb { background: rgba(42,195,168,0.35); border-radius: 9999px; }
              `}</style>
              <div
                id="preview-card"
                ref={previewPanelRef}
                style={{
                  background: "rgba(25,25,25,0.96)",
                  backdropFilter: "blur(12px)",
                  WebkitBackdropFilter: "blur(12px)",
                  border: "1px solid rgba(42,195,168,0.25)",
                  borderRadius: CARD_RADIUS,
                  overflow: "hidden",
                  overflowY: panelScrolls ? "auto" : "hidden",
                  // Top-fade when scrolled (oldest lines fade out at top).
                  WebkitMaskImage: panelScrolls
                    ? "linear-gradient(to bottom, transparent 0%, black 18%)"
                    : undefined,
                  maskImage: panelScrolls
                    ? "linear-gradient(to bottom, transparent 0%, black 18%)"
                    : undefined,
                  padding: "8px 14px",
                  fontSize: 11,
                  lineHeight: 1.5,
                  letterSpacing: "0.01em",
                  color: "rgba(220,220,220,0.88)",
                  fontFamily: "'Inter', system-ui, -apple-system, sans-serif",
                  overflowWrap: "anywhere",
                  scrollbarWidth: "thin",
                  scrollbarColor: "rgba(42,195,168,0.35) transparent",
                  userSelect: "none",
                  cursor: "default",
                }}
              >
                {livePreview}
              </div>
            </>
          )}
        </div>
      </>
    );
    ```
  - [x] 9.2 The `borderRadius: CARD_RADIUS` (14) MUST match `set_preview_shape`'s `r = (14.0 * scale) as i32` — R11 invariant.

- [x] Task 10: Update `tauri-commands.ts` imports in PreviewPanel (AC-5)
  - [x] 10.1 Ensure `onStateChanged` is imported from `./tauri-commands` (already exported there).

- [x] Task 11: Verify and close (AC-8, DoD)
  - [x] 11.1 `cargo check --target x86_64-pc-windows-gnu` — confirm no new errors on touched files
    (pre-existing `ort-sys` failure for GNU target is known-pre-existing, not caused by this story)
  - [x] 11.2 `cargo test` (Linux) — green, no regressions
  - [x] 11.3 `tsc` / `npm run build` — green
  - [ ] 11.4 Windows smoke via `scripts/sync-and-build.ps1` — run surface-smoke-checklist traps
    #3/#4/#5 + full manual AC-8 scenario

### Review Findings (code-review 2026-06-05, Opus 4.8 — 3 adversarial layers)

- [x] [Review][Patch] Async-gap in the first-chunk path — append out-of-order + stale-repopulate + orphan-show [src/PreviewPanel.tsx:181-200] (blind+edge, High). In the `klarvo://live-preview-chunk` listener the first chunk sets `showOnceRef.current = true` then `await runShowSequence()` and only calls `setLivePreview(...)` AFTER the await. Three consequences while the multi-IPC show sequence is in flight: (a) later chunks (which skip the show gate and append immediately) commit their `setLivePreview` BEFORE the first chunk's post-await append → opening words render out of order; (b) if recording ends during the await, the post-await `setLivePreview` repopulates `livePreview` AFTER the `done`-handler already cleared it → defeats the AC-4 R1 guard via the async gap; (c) `runShowSequence`'s `await win.show()` can fire AFTER the state-changed handler's `win.hide()` → window left visible (transparent/empty) until next cycle. **Fix:** append the chunk synchronously BEFORE the `await` (preserves arrival order on the single-threaded event loop), and after `await runShowSequence()` re-check `if (!isRecordingRef.current)` → `win.hide()` and skip. One coherent change resolves all three.
- [x] [Review][Patch] Horizontal screen-edge clamp missing — comment overclaims, AC-1/AR3 gap [src/PreviewPanel.tsx:127-132] (blind+edge+auditor, Medium). `previewLeft = pillCenterX - W/2` is used raw in `setPosition`, but the comment says "clamping to screen edges" and AC-1 requires the AR3 screen clamp. Only the vertical axis (`clampedMaxH`) is clamped. A `wide` (400px) preview centered over a pill near the left/right edge runs off-screen. **Fix:** inside the existing monitor block derive logical bounds `screenLeft = workArea.position.x / scale`, `screenRight = (workArea.position.x + workArea.size.width) / scale`, then clamp `previewLeft` into `[screenLeft + 12, screenRight - W - 12]`; correct the misleading comment. Mirror the vertical clamp's `/scale` logical conversion already present.
- [x] [Review][Defer] `monitorFromPoint` receives logical px but the API resolves against physical screen coords [src/PreviewPanel.tsx:109] — deferred, smoke/multi-monitor refinement. Single-monitor @1.0 scale: logical==physical → correct. Wrong-monitor selection only manifests on multi-monitor / HiDPI offsets; `?? currentMonitor()` masks (returns focused-window monitor). Smoke-verify item: test preview clamp on a multi-monitor / fractional-scale setup; refine by converting pill coords to physical before `monitorFromPoint` once scale is known.
- [x] [Review][Defer] `room <= 0` floors `clampedMaxH` to 40 but `previewTop = pillY - GAP - 40` may still overlap the pill / clip at top [src/PreviewPanel.tsx:116-117,132] — deferred, smoke verification. Only reachable when the pill sits near the top of the work area. Smoke-verify item: drag pill near the screen top, confirm preview does not overlap/clip.

## Dev Notes

### Key files to touch

| File | Change |
|---|---|
| `src/PreviewPanel.tsx` | Full implementation: subscriptions, show-once geometry, CSS card render |
| `src-tauri/src/commands/misc.rs` | Add `set_preview_shape` command (mirrors `set_bar_shape "panel"` arm) |
| `src-tauri/src/lib.rs` | Register `set_preview_shape` in `invoke_handler` |
| `src/tauri-commands.ts` | Add `setPreviewShape()` TS wrapper + add `onStateChanged` import to PreviewPanel's usage |
| `src/FloatingBar.tsx` | Disable `live-preview-chunk` append in its listener (short-circuit return) |

### `set_preview_shape` — exact blueprint from `set_bar_shape "panel"` arm

The `set_bar_shape "panel"` arm at `commands/misc.rs:440` is the direct model. The only difference
is the window label (`"preview"` instead of `"bar"`) and the function has no `shape` parameter:

```rust
// In set_bar_shape (existing, reference):
} else if shape == "panel" {
    if let Ok(size) = bar.inner_size() {
        let w = size.width as i32;
        let ht = size.height as i32;
        let r = (14.0 * scale) as i32;
        crate::set_window_region_round_rect(h, w, ht, r);
    }
}
// → set_preview_shape mirrors this exactly, targeting "preview".
```

**Critical:** `set_preview_shape` MUST be called AFTER `setSize` returns (frontend awaits it). The
Rust side reads `preview.inner_size()` which reflects the size that was just set.

### Show sequence — the critical ordering guarantee (R11, race-class killer)

```
await win.setSize(new LogicalSize(W, H));   // 1. Size must land before region
await setPreviewShape();                     // 2. Region reads inner_size() in Rust
await win.setPosition(new LogicalPosition(left, top));  // 3. Position
await win.show();                            // 4. Show LAST — no white-line flash
```

This mirrors FloatingBar's `setSize → setBarShape → setPosition → show` ordering (deep-dive §R10).
Deviating from this order re-introduces the white-line artifact (region applied before size) or
position flash (show before position).

### `previewGeometry` — the scale-factor model (docs/bar-redesign-spec.md §2)

```ts
// Story 6.2 locks fontSize="small" (k=1). Story 6.3 adds the font axis.
const FONT_PX  = { small: 11, medium: 13, large: 15 };
const k        = fontPx / 11;               // 1.0 at small
BASE_WIDTH     = { compact: 260, comfortable: 320, wide: 400 };  // at small
BASE_MAX_HEIGHT = 600;                       // at small
width     = round(BASE_WIDTH[preset] * k)   // → 260/320/400 at small
maxHeight = round(600 * k)                  // → 600 at small
```

Story 6.3 will pass `fontSize` reactively. This story always passes `"small"`.

### Screen-clamp math (AR3, bar-redesign-spec.md §3.3)

```
clampedMaxHeight = min(geom.maxHeight, (bar_y - GAP) - (screenTop + 12))
preview.left     = clamp(pillCenterX - W/2, screenLeft + 12, screenRight - W - 12)
preview.top      = bar_y - GAP - H          // H = clampedMaxHeight
```

Where `screenTop` / `screenLeft` / `screenRight` come from the Tauri 2 `Monitor` type:
- `monitor.position.y / monitor.scaleFactor` = screenTop in logical px
- `monitor.position.x / monitor.scaleFactor` = screenLeft in logical px
- `(monitor.position.x + monitor.size.width) / monitor.scaleFactor` = screenRight in logical px

Use `monitorFromPoint(pillX, pillY)` first (fallback to `currentMonitor()`). Both are imported from
`@tauri-apps/api/window`. `GAP = 8` (px between preview bottom and pill top).

### Separate-window reactivity — CRITICAL Trap #3

`PreviewPanel` is a **separate Tauri WebView window** that mounts ONCE at app start and NEVER
re-mounts when the user changes Settings. Any `getSettings()` call at mount time returns the
app-start value and freezes forever.

`previewPanelForm` (the width preset) MUST be read **inside `runShowSequence`** every recording
cycle — a fresh `getSettings()` call at show-time, not from a `useState` seeded at mount.
Failure symptom: user saves "Wide" in Settings → preview still opens at "Comfortable" width until
app restart.

### `isPreviewMode` guard in `runShowSequence`

`runShowSequence` calls `getBarPosition()`, `getSettings()`, `setPreviewShape()`, window API — all
Tauri IPC. These are already mocked / short-circuited by `isPreviewMode` in their wrapper functions.
The component itself doesn't need an extra `if (isPreviewMode) return` guard at the top, but be
aware that in `npm run preview` mode none of the Tauri calls execute.

### Disabling the pill's live-preview path (AC-7)

The FloatingBar's `live-preview-chunk` listener (effect #5, ~line 326) appends chunks to its local
`livePreview` state. The simplest way to disable this is to replace the body with an early return
comment. The dependent state variables (`panelHeight`, `panelScrolls`, `livePreview`, etc.) and the
probe/panel render stay in FloatingBar for now — Story 6.5 deletes them.

**Do NOT accidentally break**: the FloatingBar's `state-changed` handler (effect #13) still clears
`livePreview` on recording — this is fine since `livePreview` will always be `""` there. The
`isPanelOpen = isRecording && livePreview.length > 0` will always be `false` → pill geometry effect
never calls `setBarShape("panel")` → pill stays static. This is the desired behavior.

### R1 stale-chunk guard — must be in PreviewPanel too

FloatingBar's guard pattern (deep-dive, Race R1):
```ts
const isRecordingRef = useRef(false);
useEffect(() => { isRecordingRef.current = isRecording; }, [isRecording]);
// In listener:
if (!isRecordingRef.current) return;
```

PreviewPanel drives `isRecordingRef` from the `state-changed` listener (set to `true` on
`"recording"`, `false` on `"done"/"idle"/"error"`) rather than from a React `isRecording` state
variable — because PreviewPanel doesn't hold a full `RecordingState` machine. The ref update
happens synchronously in the `state-changed` handler.

### `showOnceRef` — prevents repeated geometry calls

`showOnceRef.current = true` is set at the start of `runShowSequence`. It is reset to `false` only
when the recording ends (`state-changed` → done/idle/error). This ensures:
- First chunk → `runShowSequence` fires (window shown + geometry set)
- 2nd, 3rd, … chunks → early return (`showOnceRef.current === true`)
- Next recording cycle → `showOnceRef` was reset on end → geometry fires again

If `runShowSequence` fails (exception) it resets `showOnceRef.current = false` so the next chunk
retries — fail-soft pattern.

### Monitor API in Tauri 2

```ts
import { currentMonitor, monitorFromPoint } from "@tauri-apps/api/window";

const monitor = await monitorFromPoint(pillX, pillY) ?? await currentMonitor();
// monitor.size: PhysicalSize, monitor.position: PhysicalPosition, monitor.scaleFactor: number
// Convert to logical: monitor.position.y / monitor.scaleFactor
```

`Monitor` interface (from `@tauri-apps/api/window.d.ts`):
```ts
interface Monitor {
  name: string | null;
  size: PhysicalSize;        // width, height in physical px
  position: PhysicalPosition; // top-left in physical px
  workArea: { position: PhysicalPosition; size: PhysicalSize }; // excludes taskbar
  scaleFactor: number;
}
```

Use `monitor.workArea.position` / `monitor.workArea.size` for the screen boundaries if you want to
exclude the taskbar from the clamp (the bar already sits above the taskbar, so `workArea` is more
accurate than `size`).

### `set_window_region_round_rect` is already in `lib.rs`

The helper at `lib.rs:565` is reused by `set_preview_shape`. No new Rust region helpers needed.
It takes `(hwnd, width, height, radius)` in physical pixels (caller applies scale factor).

### Event naming — colon form

All event names MUST use the colon form: `klarvo://live-preview-chunk`, `klarvo://state-changed`.
Never use dot form — Tauri reserves `.` in event strings. (project-context.md §Framework-Specific Rules)

### Inversion check requirements (Epic-4 retro AI-1)

Reviewer must verify these inversions mechanically:

1. **NFR1 static-window invariant (AC-2):** Move the `setSize` call inside the chunk append
   (after `if (!showOnceRef.current)` block, i.e. call `win.setSize` on every chunk) →
   cold first-expansion / pre-measure clip (R3/R4) reappears → RED.
   *Note: this is a smoke-time inversion — no Linux unit test available.*

2. **R1 stale-chunk guard (AC-4):** Remove `if (!isRecordingRef.current) return;` in the
   `live-preview-chunk` listener → after done, a stale chunk (if the backend sends one) would
   repopulate `livePreview`, causing the preview to reappear or bleed text → RED.
   *Note: smoke-time inversion; the guard is a closure-capture defense.*

3. **R11 region/CSS radius match (AC-8):** Change `borderRadius: CARD_RADIUS` to `28` while
   leaving the Rust `r = (14.0 * scale)` → white-line gap at corners → RED.
   *Note: smoke-time inversion.*

4. **6.1 carried-forward: click-through (AC-8):** Remove `preview.set_ignore_cursor_events(true)`
   in `create_preview_window` → cursor clicks land on the preview window instead of passing
   through → RED. *(Smoke-time only — carried from 6.1)*

5. **6.1 carried-forward: CloseRequested (AC-8):** Remove `|| label == "preview"` from the
   `CloseRequested` guard → closing the preview window should be possible (window vanishes) → RED.
   *(Smoke-time only — carried from 6.1)*

### Surface-smoke-checklist traps that apply

- **Trap #3 (separate-window reactivity):** `previewPanelForm` read via `getSettings()` inside
  `runShowSequence` (NOT mount-only state). Verify: change Width Preset in Settings → save →
  next recording → preview opens at the new width (not the old one).

- **Trap #4 (geometry/region):** `set_preview_shape` radius `14` MUST match CSS `borderRadius: 14`
  (CARD_RADIUS constant). Verify no white-line at corners. Also verify the show sequence order
  (`setSize → setPreviewShape → setPosition → show`).

- **Trap #5 (event push-wiring):** Confirm `klarvo://live-preview-chunk` is subscribed in
  PreviewPanel AND the producer emits. Confirm `klarvo://state-changed` is subscribed in
  PreviewPanel. A Linux-green `tsc` does NOT prove the event reaches the separate window.

### No Android change

This is a desktop-only Windows story. Do not touch any Android path.

### `onStateChanged` already exists in `tauri-commands.ts`

`src/tauri-commands.ts:382` exports `onStateChanged(cb)` → `listen("klarvo://state-changed")`.
Import and use this wrapper in PreviewPanel — do NOT subscribe to the raw event string.

### `getBarPosition` returns `{x, y} | null`

`src/tauri-commands.ts:868` returns `Promise<{x: number; y: number} | null>`. When `null` (first
run before the bar has been dragged), the pill is at the Tauri-default position. In this case,
skip the show sequence (return early) — don't attempt to compute `pillY - GAP - H` on `null`.

### Window creation recap from 6.1

The `"preview"` window was created in Story 6.1:
- Started at `inner_size(1.0, 1.0)` — tiny/invisible
- No region set at creation (AC-1 of 6.1 explicitly omitted `set_window_region_*`)
- No `visible(false)` was added (the 1×1 size + transparent makes it effectively invisible)
- `set_ignore_cursor_events(true)` was applied after `build()` — click-through active

Story 6.2 is the first time the preview window is actually shown with real content, so this is
where the 6.1 carried-forward inversions (click-through, CloseRequested) are finally verifiable.

### Project Structure Notes

- `set_preview_shape` in `commands/misc.rs` — "Window / UI helpers" section after `set_bar_shape`
- `setPreviewShape()` TS wrapper in `tauri-commands.ts` — after the existing `setBarShape` wrapper
- `invoke_handler` registration in `lib.rs:1079` (next to `set_bar_shape`)
- `PreviewPanel.tsx` replaces the stub from 6.1 entirely
- `FloatingBar.tsx` — minimal edit: one listener body replaced with `return;`

### References

- Epic 6 planning + full ACs: `_bmad-output/planning-artifacts/epics-bar-redesign.md#Story 6.2`
- Foundation design spec (geometry model, positioning math, race-class): `docs/bar-redesign-spec.md`
- Ist-Zustand (full effect inventory, race table, data flow): `docs/deep-dive-bar-subsystem.md`
- Surface-smoke-checklist: `docs/surface-smoke-checklist.md`
- `set_bar_shape` blueprint: `src-tauri/src/commands/misc.rs:427`
- `set_window_region_round_rect`: `src-tauri/src/lib.rs:565`
- `create_preview_window` (6.1): `src-tauri/src/lib.rs` (~line 738)
- `ensure_preview_window` (6.1): `src-tauri/src/commands/misc.rs:256`
- FloatingBar live-preview chunk listener (to disable): `src/FloatingBar.tsx:326`
- FloatingBar R1 stale-chunk guard pattern: `src/FloatingBar.tsx:317`
- `onStateChanged` wrapper: `src/tauri-commands.ts:382`
- `getBarPosition` wrapper: `src/tauri-commands.ts:868`
- `setBarShape` wrapper (blueprint for `setPreviewShape`): `src/tauri-commands.ts:517`
- `AppSettings.previewPanelForm`: `src/types.ts:84`
- `monitorFromPoint`, `currentMonitor`, `Monitor` interface: `@tauri-apps/api/window` (Tauri 2)
- Project rules: `_bmad-output/project-context.md`

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

### Completion Notes List

- All 11 tasks implemented and verified. PreviewPanel.tsx replaced from stub to full implementation (subscriptions, show-once geometry, CSS card render).
- `set_preview_shape` Rust command added to `commands/misc.rs` (mirrors `set_bar_shape "panel"` arm, targets "preview" window). Registered in `lib.rs` invoke_handler. TS wrapper `setPreviewShape()` added to `tauri-commands.ts`.
- `previewGeometry(widthPreset, "small")` helper: scale factor k=fontPx/11 with BASE_WIDTH compact/comfortable/wide (260/320/400) and BASE_MAX_HEIGHT=600.
- State/refs: `isRecordingRef` (R1 stale-chunk guard), `showOnceRef` (show-once gate), `clampedMaxHeightRef`, `pillXRef`/`pillYRef`, `previewPanelRef`.
- `onStateChanged` subscription: arms isRecordingRef on "recording", clears+hides on done/idle/error, resets showOnceRef.
- `live-preview-chunk` listener: R1 guard first, then chunk appended SYNCHRONOUSLY before show-once await (review-fix High), then show-once geometry sequence + post-await isRecording re-check (review-fix High: orphan-show guard).
- `runShowSequence`: reads `getBarPosition()` (skips cycle if null), reads `getSettings().previewPanelForm` reactively (Trap #3 — NOT mount-only), computes clampedMaxHeight + horizontal screen-edge clamp (review-fix Medium) in single monitor query, executes sequence `setSize → setPreviewShape → setPosition → show`.
- Auto-scroll effect + `useLayoutEffect` overflow detection (setPanelScrolls when scrollHeight > clampedMaxHeight).
- FloatingBar `live-preview-chunk` listener body replaced with short-circuit `return` (AC-7). Dead code NOT deleted (Story 6.5).
- R11 invariant: CARD_RADIUS=14 in CSS borderRadius matches set_preview_shape r=(14.0*scale) as i32.
- Inversion comments placed at all three key inversion points (NFR1, R1, R11).
- ✅ Resolved review finding [High]: Async-gap — chunk now appended synchronously before await; post-await orphan-show guard added (isRecordingRef re-check → win.hide()).
- ✅ Resolved review finding [Medium]: Horizontal screen-edge clamp — previewLeft now clamped into [screenLeft+12, screenRight-W-12] via workArea bounds, same monitor block as vertical clamp; misleading comment corrected.
- `cargo check --target x86_64-pc-windows-gnu`: only pre-existing ort-sys/whisper-rs/llama-cpp-sys build-script failures, no new Rust E-codes.
- `cargo test --lib`: 572 passed / 0 failed.
- `tsc --noEmit`: clean.
- `npm run build`: green (PreviewPanel-Bu4ScR93.js 4.02 kB).
- BLOCKED on Task 11.4: Windows smoke (surface-class hard gate) — requires Andi on Windows with `scripts/sync-and-build.ps1`. Story stays in "review" until smoke confirmed green.

### File List

- `src/PreviewPanel.tsx` — full implementation (replaced 6.1 stub)
- `src-tauri/src/commands/misc.rs` — added `set_preview_shape` command
- `src-tauri/src/lib.rs` — registered `commands::misc::set_preview_shape` in invoke_handler
- `src/tauri-commands.ts` — added `setPreviewShape()` TS wrapper
- `src/FloatingBar.tsx` — disabled `live-preview-chunk` listener body (short-circuit return, AC-7)
- `_bmad-output/implementation-artifacts/6-2-move-live-preview-into-the-window-css-grow-scale-geometry.md` — this story file
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — status updated to review

### Change Log

- 2026-06-05: Story 6.2 implemented — PreviewPanel full wiring (subscriptions, show-once geometry, CSS card render), `set_preview_shape` Rust command + TS wrapper, FloatingBar preview path disabled (AC-7). 572 Rust tests green, tsc clean, vite build green. Blocked on Windows smoke (surface-class DoD, Task 11.4).
- 2026-06-05: Addressed code review findings — 2 items resolved (Date: 2026-06-05). [High] Async-gap fixed: chunk appended synchronously before await + post-await orphan-show guard. [Medium] Horizontal screen-edge clamp added: previewLeft clamped via workArea bounds in single monitor query. 572 tests/0 fail, tsc/vite green.
