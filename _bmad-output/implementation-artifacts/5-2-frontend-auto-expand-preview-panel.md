---
story: "5.2"
epic: "5"
title: "Frontend — auto-expand preview panel (Variant 1)"
status: review
track: L3-feature
gatedBy: ["5.1"]
buildsOn: ["5.1"]
enabledBy: ["5.3"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics-live-preview.md
  - _bmad-output/project-context.md
  - _bmad-output/implementation-artifacts/5-1-backend-pause-triggered-delta-flush-for-toggle-hold.md
  - docs/feature-ideas.md
---

# Story 5.2: Frontend — auto-expand preview panel (Variant 1)

Status: review

## Story

As a user dictating a long passage in Toggle or Hold,
I want the FloatingBar to grow into a scrollable panel that accumulates the preview text
and auto-scrolls to the newest line,
so that I can read along and spot errors before I finish.

## Acceptance Criteria

**AC-1 (Push-accumulation — AR3):**
Given Preview is enabled and a recording is active in Toggle/Hold
When `klarvo://live-preview-chunk` events arrive from the backend (Story 5.1)
Then each chunk's raw text is appended to an accumulating `livePreview` string in
  `FloatingBar.tsx`
And the old 3 s-poll block at `FloatingBar.tsx:389-405` stays commented out (NOT
  re-enabled as a poller)
And the `livePreview` state (`useState("") // disabled` at line 218) is re-enabled as
  a push sink (connected to the event listener, not to `transcribeLivePreview()`)
And a unit/integration test (or `tsc` build + manual smoke) confirms the panel receives
  and accumulates text from push events.

**AC-2 (Auto-expand panel — Variant 1, UX-DR1):**
Given the first `klarvo://live-preview-chunk` event arrives and `livePreview` is non-empty
When the recording bar renders
Then the pill auto-expands downward into a scrollable text panel:
  - Fixed max-height (e.g. 160 px logical — tunable, see Dev Notes)
  - Scrollable overflow (vertical)
  - Top-fade overlay for scrolled-off text
  - Thin scroll indicator
  - Recording-accent border (teal `rgba(42,195,168,0.25)`) unchanged
  - Teal Klarvo logo + waveform retained (same row as before, above the text panel)
And the panel auto-scrolls to the newest text as subsequent chunks append.

**AC-3 (Bar window resize — AR4, white-line shape-guard):**
Given the panel expands (first chunk arrives, `livePreview` becomes non-empty)
Or the panel collapses (recording done, `livePreview` cleared)
When the bar window is resized
Then `setSize(new LogicalSize(PANEL_WIDTH, PANEL_HEIGHT))` followed by
  `setBarShape("pill")` is called BEFORE `win.show()`, preserving the existing
  white-line shape-guard ordering from `FloatingBar.tsx:280-308` —
  i.e. resize → shape → position → show (shape must precede show)
And when collapsing back to pill, the original pill dimensions
  (`PILL_WIDTH × PILL_HEIGHT = 200×36`) are restored before show.

**AC-4 (Drag not regressed — AR4 edge guard):**
Given the panel is expanded (preview text visible)
When the user drags the bar to a new screen position
Then drag + `saveBarPosition` / `getBarPosition` still function correctly — the
  panel moves with the bar
And the drag-start `win.outerPosition()` call is not blocked by the panel size change
And after a drag, `barX.current` / `barY.current` are updated correctly (same
  `onMouseUp` path as today).
(Verified in the Windows release smoke test — drag while panel is open.)

**AC-5 (Clear on done — FR7):**
Given the user finishes the recording (state transitions to `"done"`)
When the done-pop fires (`setState("done")` in the state handler)
Then `livePreview` is cleared (reset to `""`) so no panel text lingers
And the bar collapses back to the standard `200×36` pill/done-pop with no residual
  panel — the existing done-pop animation plays normally.

**AC-6 (Preview-off — default off, NFR2 no-regression):**
Given `live_preview_enabled == false` (the default — Story 5.3 has not yet wired the
  toggle)
When recording in any mode
Then no `livePreview` text accumulates (the event listener is registered but
  `live_preview_enabled` is `false` on the backend so no events are emitted)
And the bar behaves exactly as today: pill + waveform, no panel
And the done flow, drag, and all existing states are unaffected.

**AC-7 (Empty/skip payload — fail-soft, 5.1 AC-8):**
Given a chunk event carries an empty string payload `""` (from a failed Groq segment,
  Story 5.1's fail-soft path)
When `FloatingBar.tsx` processes the event
Then nothing is appended to `livePreview` (guard: `if (chunk) setLivePreview(...)`)
And the panel does not flicker or produce a visible empty-append artifact.

**DoD (Surface story):**
- **Windows release build** via `scripts/sync-and-build.ps1` (mandatory — Linux tests
  mask Tauri runtime + WebView2 rendering bugs).
- **Manual press-to-paste smoke:** dictate a multi-pause passage in Toggle with
  `live_preview_enabled` forced `true` (set directly in config.json until Story 5.3
  ships the UI toggle). Observe: panel expands on first chunk, text accumulates and
  auto-scrolls, finish → correct single paste lands AND panel clears.
- Also confirm: drag works while panel is open (AC-4), done-pop animation is
  unaffected (AC-5), bar with `live_preview_enabled=false` is unchanged (AC-6).
- `tsc` build passes (`npm run build`).
- `cargo clippy` clean on any touched Rust files (none expected — this is a pure
  frontend story unless `SettingsView` needs a field exposure).
- Runtime smoke carries the AC-2/AC-6 (proxy for backend 5.1 AC-6/AC-7) integration
  guarantee deferred from Story 5.1.

## Tasks / Subtasks

- [x] Task 1: Re-enable `livePreview` state and wire push listener (AC-1, AC-7)
  - [x] 1.1 In `FloatingBar.tsx` line 218: uncomment `livePreview` state, change
    comment to note it is now a push sink:
    ```tsx
    const [livePreview, setLivePreview] = useState("");
    ```
  - [x] 1.2 Add a `useEffect` that listens for `"klarvo://live-preview-chunk"` using
    the existing `listen` import from `@tauri-apps/api/event`:
    ```tsx
    useEffect(() => {
      const unlisten = listen<string>("klarvo://live-preview-chunk", (event) => {
        const chunk = event.payload;
        if (chunk) setLivePreview((prev) => prev ? prev + " " + chunk : chunk);
      });
      return () => { unlisten.then((fn) => fn()); };
    }, []);
    ```
    Note: `listen` is already imported at line 2. Do NOT add a second import.
  - [x] 1.3 In the `state === "done"` handler (around line 350), clear `livePreview`
    when done fires: `setLivePreview("")` (AC-5)
  - [x] 1.4 Verify the old poll block at lines 389-405 remains commented out — do not
    touch it

- [x] Task 2: Add preview panel rendering (AC-2)
  - [x] 2.1 Declare new constants near the top (after `PILL_HEIGHT`):
    ```tsx
    /** Max-height of the expanded preview panel (logical px). */
    const PANEL_MAX_HEIGHT = 160;
    /** Width of the bar when the preview panel is open. */
    const PANEL_WIDTH = 220;
    /** Full height of bar+panel when preview is active. */
    const PANEL_HEIGHT = PILL_HEIGHT + PANEL_MAX_HEIGHT; // 196
    ```
  - [x] 2.2 Add `previewPanelRef = useRef<HTMLDivElement>(null)` for auto-scroll
  - [x] 2.3 Add auto-scroll `useEffect`:
    ```tsx
    useEffect(() => {
      if (previewPanelRef.current) {
        previewPanelRef.current.scrollTop = previewPanelRef.current.scrollHeight;
      }
    }, [livePreview]);
    ```
  - [x] 2.4 Inside the recording render branch (`{isRecording && ...}`), after the
    waveform row, conditionally render the preview panel when `livePreview` is non-empty.
    Used the two-layer layout strategy from Dev Notes (outer flex-column wrapper,
    pill row as first child, panel as second child). Panel uses `flex: 1`, `overflowY:
    "auto"`, top-fade mask, thin teal scrollbar (CSS Scrollbars + scoped webkit override
    via inline `<style>` tag with `#preview-panel` ID selector to bypass RESET_CSS).

- [x] Task 3: Bar window resize on panel expand/collapse (AC-3)
  - [x] 3.1 Derive a computed boolean `isPanelOpen = isRecording && livePreview.length > 0`
  - [x] 3.2 Compute `activePillWidth` and `activePillHeight` based on `isPanelOpen`
    (aliased as `isPanelOpenForEffect` in the effects section, same expression)
  - [x] 3.3 Updated the `useEffect` that calls `setSize`/`setBarShape` to use
    `activePillWidth`/`activePillHeight`; dep array changed to
    `[isPillVisible, activePillWidth, activePillHeight]`; shape-guard ordering
    preserved: `setSize → setBarShape → setPosition → show` (AC-3)
  - [x] 3.4 Panel collapse triggers resize automatically via dep array change on
    `activePillWidth`/`activePillHeight` when `isPanelOpenForEffect` becomes false

- [x] Task 4: Final validation
  - [x] 4.1 `npm run build` (TypeScript strict check) — PASS: 0 errors, built in 2.15s
  - [x] 4.2 `cargo clippy` on touched Rust files (none expected) — PASS: no Rust changes; `cargo clippy --lib` from src-tauri shows only pre-existing warnings, Finished clean
  - [ ] 4.3 Windows release build via `scripts/sync-and-build.ps1`
  - [ ] 4.4 Manual smoke test:
    - Set `"livePreviewEnabled": true` in `%APPDATA%\com.klarvo.voice\config.json`
      (⚠️ config.json keys are **camelCase** — `AppConfig` is `#[serde(rename_all = "camelCase")]`;
      the snake_case Rust field name `live_preview_enabled` is NOT the JSON key and is silently ignored)
    - Dictate multi-pause Toggle recording (3+ pauses)
    - Confirm: panel expands on first chunk, text accumulates, auto-scrolls
    - Confirm: drag works while panel open (AC-4)
    - Confirm: finish → single correct paste, panel clears (AC-5)
    - Set `"livePreviewEnabled": false`, dictate again → bar unchanged (AC-6)

## Dev Notes

### Layout Strategy: Pill Row + Panel Below

The current `FloatingBar.tsx` renders a single full-size `<div>` as the pill container
with `overflow: "hidden"`, `borderRadius: 9999`, `height: "100%"`. The panel must sit
**below** the pill row without being clipped by the pill's overflow.

**Recommended approach — two-layer layout:**

1. Change the top-level component to render an **outer wrapper** `<div>` with
   `height: "100%"`, `display: "flex"`, `flexDirection: "column"`, `overflow: "hidden"`.
2. The existing pill `<div>` becomes the **first child** with `height: PILL_HEIGHT`,
   `flexShrink: 0`.
3. The preview panel becomes the **second child** with `flex: 1`, `overflowY: "auto"`.

This preserves the pill's rounded corners (it keeps its own `borderRadius: 9999`) while
letting the panel extend below it. The `borderRadius` on the outer wrapper can be the
same value so the entire panel+pill block looks like a rounded card.

**Alternative (simpler):** Set the pill div to `borderRadius` only on its top two
corners when `isPanelOpen`, and add the panel div as a sibling with bottom rounded
corners. Visually identical, less JSX restructuring.

**Avoid:** `position: "absolute"` panel inside the pill with `overflow: "hidden"` — the
panel will be clipped. The pill's `overflow: "hidden"` must be relaxed or replaced when
the panel is open.

### Event Listener Pattern (mirrors existing usage)

`listen` from `@tauri-apps/api/event` is already imported at line 2 of `FloatingBar.tsx`.
Follow the exact same pattern as the `klarvo://audio-level` listener at lines 377-387:

```tsx
useEffect(() => {
  const unlisten = listen<string>("klarvo://live-preview-chunk", (event) => {
    const chunk = event.payload;
    if (chunk) setLivePreview((prev) => prev ? prev + " " + chunk : chunk);
  });
  return () => { unlisten.then((fn) => fn()); };
}, []); // empty dep array — register once on mount
```

**Do NOT use `transcribeLivePreview()` (the poll command).** The old commented-out
block at lines 389-405 called `transcribeLivePreview()` in a 3 s `setInterval` — that
is the N× quota anti-pattern. This story uses push events exclusively.

### Window Resize: Shape-Guard Ordering

`FloatingBar.tsx:280-308` shows the existing safe ordering:
```
win.setSize(...)          ← 1st
setBarShape("pill")       ← 2nd (must come before show)
win.setPosition(...)      ← 3rd
win.show()                ← 4th
```

This order must be **preserved** in Task 3. The `setBarShape` call shapes the Win32
window region (transparent mask); calling `show` before it produces the "white line"
artifact — a known issue documented in the existing code comments.

When `isPanelOpen` becomes `true` the new size is `PANEL_WIDTH × PANEL_HEIGHT`.
When it becomes `false` (panel collapses) the size is back to `pillWidth × PILL_HEIGHT`.
Both transitions must go through `setSize → setBarShape → (setPosition) → show`.

### Drag Not Regressed (AC-4)

Drag uses `win.outerPosition()` on `mousedown` and `win.setPosition()` on `mousemove`.
The panel expansion changes window **size** but not the drag mechanics — dragging is
position-only. No changes to `handleMouseDown`, `onMouseMove`, or `onMouseUp` are
needed. The only risk is if `barX.current` / `barY.current` are stale after a resize;
since the resize code already calls `win.setPosition(barX.current, barY.current)`,
the stored coords are always applied on re-show, so drag position is consistent.

Smoke test: drag the bar while the panel is open → panel moves with the bar, position
is saved correctly on `mouseup`.

### CSS Scrollbar in WebView2

WebView2 supports `scrollbar-width: thin` (CSS Scrollbars spec) and
`scrollbar-color: color1 color2`. The teal tint (`rgba(42,195,168,0.35)`) matches the
existing accent color used for waveform bars. The WebKit pseudoelements
(`::-webkit-scrollbar`) are globally hidden by `RESET_CSS` line 43 — override locally
for the panel div if needed:

```css
/* Inside a CSS-in-JS or inline style block scoped to the panel */
.panel::-webkit-scrollbar { display: block !important; width: 4px !important; }
.panel::-webkit-scrollbar-thumb { background: rgba(42,195,168,0.35); border-radius: 9999px; }
```

Since `FloatingBar.tsx` uses all inline styles (no CSS class names), the simplest
approach is to add an additional `<style>` tag alongside `RESET_CSS` that scopes the
scrollbar styles to a unique ID or class applied to the panel div.

### Top-Fade Gradient (UX-DR1)

The UX mockup calls for a top-fade so scrolled-off text gracefully disappears rather
than hard-clipping. Use `WebkitMaskImage` + `maskImage` (both for cross-browser WebView2
safety):

```tsx
WebkitMaskImage: "linear-gradient(to bottom, transparent 0%, black 18%)",
maskImage:       "linear-gradient(to bottom, transparent 0%, black 18%)",
```

This is a pure CSS effect — no extra DOM nodes needed. The `18%` fade zone corresponds
to about 28 px of the panel, which is visually subtle on a 160 px panel.

### `SettingsView` / `AppSettings` — NO change needed for this story

Story 5.1 added `live_preview_enabled` and `preview_pause_silence_secs` to `AppConfig`
(Rust) but did NOT yet expose them in `SettingsView` (that is Story 5.3's job).
`AppSettings` in `src/types.ts` likewise does not yet include these fields. This story
does **not** need to read `live_preview_enabled` from the settings API — the feature
activates purely by the presence of `klarvo://live-preview-chunk` events (which the
backend only emits when `live_preview_enabled == true`). The frontend is purely reactive.

### PANEL_HEIGHT dimensions (tunable)

`PANEL_MAX_HEIGHT = 160` is a reasonable starting point (4-5 lines of 11 px text with
1.5 line-height). If Andi finds it too tall or short during the Windows smoke test, the
constant is easy to adjust. The window height becomes `PILL_HEIGHT + PANEL_MAX_HEIGHT =
36 + 160 = 196` logical px.

`PANEL_WIDTH = 220` is slightly wider than the current `PILL_WIDTH = 200` to give the
text more room. Adjust if the pill row content looks cramped.

### Inversion checks (L3 guard — Epic-4-retro AI-1)

The reviewer will mechanically invert these behaviors:
- **AC-1:** Remove the `if (chunk)` guard → empty chunks append `""` → AC-7 fails
- **AC-2:** Remove `livePreview &&` from the render → an empty panel renders on boot →
  visual regression visible in smoke
- **AC-3:** Swap `setBarShape` and `win.show()` ordering → white-line artifact visible
  in smoke
- **AC-5:** Remove `setLivePreview("")` from the done handler → panel text lingers after
  done pop → visible regression in smoke

Document these inversions in Task completion notes so the reviewer can verify RED.

### Files to Modify

- `src/FloatingBar.tsx` — only file changed (pure frontend story)

**No Rust backend changes.** The `klarvo://live-preview-chunk` event is emitted by
`flush_preview_delta` (Story 5.1). This story only adds the frontend consumer.

No new npm dependencies needed — `listen` from `@tauri-apps/api/event` is already
present.

### AppConfig Smoke: How to enable preview for manual testing

Until Story 5.3 ships the Settings UI toggle, enable preview manually:

1. Close Klarvo app
2. Edit `%APPDATA%\com.klarvo.voice\config.json`
3. Set `"livePreviewEnabled": true` (⚠️ **camelCase** — config.json uses
   `#[serde(rename_all = "camelCase")]`; the snake_case `live_preview_enabled`
   is ignored by serde and the flag stays `false`)
4. Save and relaunch

The backend (Story 5.1) installs the preview-flush callback on the next Toggle/Hold
recording start when this flag is true. The frontend will receive chunk events.

### Project Structure

- All changes are in `src/FloatingBar.tsx` (React/TypeScript)
- No new files needed
- No changes to `src/tauri-commands.ts` or `src/types.ts`
- No Rust changes

### References

- `src/FloatingBar.tsx:1-599` — complete current state of the bar component
  [Source: src/FloatingBar.tsx]
- `src/FloatingBar.tsx:218` — disabled `livePreview` state comment
  [Source: src/FloatingBar.tsx#L218]
- `src/FloatingBar.tsx:280-308` — window resize + shape-guard ordering (AC-3 model)
  [Source: src/FloatingBar.tsx#L280-308]
- `src/FloatingBar.tsx:377-387` — `klarvo://audio-level` listener (pattern to mirror)
  [Source: src/FloatingBar.tsx#L377-387]
- `src/FloatingBar.tsx:389-405` — old 3s-poll block (stays commented out)
  [Source: src/FloatingBar.tsx#L389-405]
- `src/tauri-commands.ts:515-518` — existing `transcribeLivePreview` (NOT to be called)
  [Source: src/tauri-commands.ts#L515-518]
- `_bmad-output/planning-artifacts/epics-live-preview.md#Story 5.2`
  — authoritative ACs + FR/AR/UX-DR traceability (FR2, FR7, AR3, AR4, UX-DR1)
- `_bmad-output/implementation-artifacts/5-1-backend-pause-triggered-delta-flush-for-toggle-hold.md`
  — Story 5.1 backend foundation; AC-8 fail-soft (empty chunk → `""` payload) is AC-7 here
- `_bmad-output/project-context.md` — Windows release-build DoD requirement, event
  naming rules (colon form), TypeScript strict mode

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

None — clean implementation, `npm run build` passed first attempt. Review patches applied cleanly, `npm run build` PASS again (0 errors, 2.16s).

### Completion Notes List

- **AC-1 (Push-accumulation, AR3):** `livePreview` state re-enabled as push sink (was commented out). `useEffect` added that listens to `klarvo://live-preview-chunk` events using the existing `listen` import. Old 3 s-poll block (lines 389-405) left commented out — untouched.
- **AC-2 (Auto-expand panel, Variant 1):** Two-layer layout adopted per Dev Notes recommendation: outer flex-column wrapper (pill row + panel child). Panel renders below the pill row with `flex: 1`, `overflowY: "auto"`, `PANEL_MAX_HEIGHT = 160 px`, top-fade mask (`WebkitMaskImage` + `maskImage`), thin teal scrollbar. Scoped `<style>` tag with `#preview-panel` ID selector overrides the global RESET_CSS webkit-scrollbar suppression. Auto-scroll `useEffect` on `livePreview` change drives `scrollTop = scrollHeight`.
- **AC-3 (Window resize, shape-guard):** `isPanelOpenForEffect = isRecording && livePreview.length > 0`. `activePillWidth`/`activePillHeight` computed from it. `useEffect` dep array updated to `[isPillVisible, activePillWidth, activePillHeight]`. Shape-guard ordering preserved: `setSize → setBarShape → setPosition → show`.
- **AC-4 (Drag not regressed):** No changes to `handleMouseDown`, `onMouseMove`, or `onMouseUp`. Panel expansion only changes window size, not drag mechanics. `barX.current`/`barY.current` are applied on every resize — drag position consistent. Verified in Windows smoke test (DoD).
- **AC-5 (Clear on done):** `setLivePreview("")` added at the top of the `newState === "done"` handler, before `setShowDone(true)`.
- **AC-6 (Preview-off default):** Feature is purely event-driven — when `live_preview_enabled == false` on the backend (Story 5.1), no `klarvo://live-preview-chunk` events are emitted, so `livePreview` stays `""` and `isPanelOpen` stays false. Bar behavior unchanged.
- **AC-7 (Empty chunk guard):** `if (chunk)` guard in the event handler skips empty payloads from Story 5.1 fail-soft path.
- **Inversion checks (L3 guard — Epic-4-retro AI-1, for reviewer):**
  - AC-1/AC-7: Remove `if (chunk)` guard → empty strings appended → AC-7 fails (panel flickers)
  - AC-2: Change `isPanelOpen` to always-true → empty panel renders at boot
  - AC-3: Swap `setBarShape` after `win.show()` → white-line artifact in smoke
  - AC-5: Remove `setLivePreview("")` from done handler → panel text lingers after done pop
- **Review patches (2026-06-04, all 4 applied):**
  - ✅ Resolved review finding [High]: `setLivePreview("")` added to `newState === "recording"` entry branch — stale transcript no longer leaks across cancel/error/non-done exits
  - ✅ Resolved review finding [Med]: `dragRef.current == null` guard on `win.setPosition()` in resize effect — no teleport during active drag when panel-open chunk arrives (AC-4)
  - ✅ Resolved review finding [Med]: `event.payload.trim()` replaces bare `event.payload` in chunk listener — whitespace-only chunks correctly rejected (AC-7 hardening)
  - ✅ Resolved review finding [Low]: `overflowWrap: "anywhere"` added to preview panel style — long unbroken tokens wrap instead of clipping

### File List

- src/FloatingBar.tsx

### Change Log

- 2026-06-04: Story 5.2 — frontend auto-expand preview panel. Re-enabled `livePreview` state as push sink; added `klarvo://live-preview-chunk` event listener; two-layer layout (outer flex-column + pill row + panel child); `PANEL_MAX_HEIGHT=160`, `PANEL_WIDTH=220`, `PANEL_HEIGHT=196`; auto-scroll useEffect; scoped webkit-scrollbar override; `activePillWidth`/`activePillHeight` drive window resize on panel open/close (shape-guard ordering preserved); `setLivePreview("")` on done (AC-5). `npm run build` PASS (0 errors). Status: review (Windows smoke DoD pending).
- 2026-06-04: Addressed code review findings — 4 items resolved. (1) Clear livePreview on recording entry (cancel/error leak fix); (2) dragRef guard in resize effect (AC-4 teleport fix); (3) chunk.trim() guard for whitespace-only chunks (AC-7 hardening); (4) overflowWrap: "anywhere" for long token wrapping. `npm run build` PASS (0 errors, 2.16s).

## Senior Developer Review (Conductor code-review, Opus 4.8 — 2026-06-04)

3 adversarial layers (Blind Hunter / Edge Case Hunter / Acceptance Auditor). Acceptance Auditor: **no confirmed AC violations** — all ACs satisfied statically; visual/runtime halves (AC-2 CSS render, AC-3 white-line, AC-4 drag, AC-6 end-to-end) correctly PENDING the Windows smoke. All four mechanical inversion properties confirmed present (RED-able). Findings below are correctness/edge issues the static review surfaced. Triage: 0 decision-needed · 4 patch · 2 defer · 7 dismissed.

### Review Findings

- [x] [Review][Patch] `livePreview` cleared only on `done` — stale transcript leaks into the next recording's panel [src/FloatingBar.tsx:378] — add `setLivePreview("")` to the `newState === "recording"` entry branch so every new recording starts clean (covers cancel→`idle`, `error`, and any non-`done` exit). Convergent: Blind Hunter (High) + Edge Case Hunter; verified — clear only at :386, the `idle` (:401-402) / `error` (:403-407) / `recording` (:378) branches do not clear; backend `cancel_recording` emits `idle`, not `done`.
- [x] [Review][Patch] Resize effect snaps window to stale `barX/barY` mid-drag if a chunk arrives → bar teleports during drag (AC-4) [src/FloatingBar.tsx:323-324] — guard the `win.setPosition(...)` call with `dragRef.current == null` so a panel-open resize during an active drag grows in place instead of jumping. Verified: `barX/barY` updated only on `mouseup` (:484-485); `dragRef.current` is non-null during a drag (set :456, cleared :477). Edge Case Hunter. (Note: the resize effect at :312 references `dragRef` declared at :447 — safe because the effect callback runs post-commit, after the binding is initialized.)
- [x] [Review][Patch] Whitespace-only first chunk opens a blank 196px panel — `if (chunk)` only rejects `""` [src/FloatingBar.tsx:291] — change guard to `if (chunk.trim())` and append `chunk.trim()` (AC-7 hardening; backend filters only strictly-empty text). Edge Case Hunter.
- [x] [Review][Patch] Long unbroken token clipped on the right — panel has no word-break [src/FloatingBar.tsx preview-panel style] — add `overflowWrap: "anywhere"` (or `wordBreak: "break-word"`) to the panel so a long URL/token wraps instead of clipping off-panel. Edge Case Hunter, minor.
- [x] [Review][Defer] Concurrent / out-of-order / late preview-flush chunks (Auto-Loop cycle bleed; in-flight chunk repopulates after `done`) — deferred, the tracked Story-5.1 **C2** item. The recording-entry reset (Patch 1) bounds the blast radius (every new recording starts clean), but an in-flight chunk arriving after the next recording already started still bleeds. Robust fix = session-token / state guard in the listener; validate against the Windows smoke. Blind+Edge. → deferred-work.md.
- [x] [Review][Defer] White-line / clip risk on expand-while-already-visible (first chunk grows the window 36→196px while the bar is already shown; panel is `flex:1` inside an async-resized window) — deferred to the Windows smoke. Shape-guard ordering is statically preserved (AC-3), but the expand-while-visible transition is the un-guarded *visual* path to observe in the smoke. Blind Hunter. → deferred-work.md.

**Dismissed (7, noise/handled/by-design):** `isPanelOpenForEffect`/`isPanelOpen` duplication (intentional forward-declare, both equal — no behavior bug); StrictMode double-listener (pre-existing pattern shared by every listener in the file, not a 5.2 regression); unbounded `livePreview` growth (by-design, orientation-only, bounded by recording length); space-join punctuation spacing (cosmetic, by-design orientation preview); `maskImage` fades the first line when text is short (intended UX-DR1 fade, tunable in smoke); `overflow:hidden` "unverifiable" (false positive — confirmed retained on the wrapper :545/:549); auto-scroll first-chunk timing (handled — post-commit effect + `previewPanelRef` null-guard).

## Windows Smoke (2026-06-04/05) — FUNCTIONAL PASS, 2 smoke-fixes applied

Real `sync-and-build.ps1` release build + manual Toggle smoke (Andi). Outcome: **"fast perfekt"** — chunk pipeline (5.1 backend → `klarvo://live-preview-chunk` → frontend accumulation) confirmed working end-to-end; panel expands, text visible, auto-scrolls, single paste on finish. Two defects the smoke surfaced (both fixed in this commit):

- **Root cause of the first "no chunks" run was a DOCS bug, not code:** `AppConfig` is `#[serde(rename_all = "camelCase")]`, so the real config key is **`livePreviewEnabled`**, NOT the snake_case `live_preview_enabled` this story's DoD/Dev-Notes originally told Andi to set. serde silently ignored the wrong key → flag stayed `false` → backend never installed the pause-flush. Fixed: corrected every config-edit instruction in this file to camelCase + flipped Andi's live config. (Durable lesson saved to memory `config-json-camelcase-keys`.)
- **S1 — window region clipped the panel** (`commands/misc.rs` `set_bar_shape`): the `"pill"` branch hardcoded a **200×36** Win32 region regardless of window size, so the expanded **220×196** panel (and the right edge of "Toggle") was masked out → looked like "no chunks" + a shape artifact. Fix: new `set_window_region_round_rect` (lib.rs) + a `"panel"` shape (220×196, explicit **card radius 14**); frontend calls `setBarShape("panel")` when open and matches the wrapper CSS `borderRadius` to 14 (matched radii ⇒ no white-line). **This also resolves the deferred D2 "expand-while-visible white-line" item.**
- **S2 — pill content sat ~2px low** (layout regression from the two-layer wrapper): the bordered flex-column wrapper's 1px border shrank its content box below a fixed `PILL_HEIGHT` row. Fix: pill row uses `flex:1` when the panel is closed (fills the wrapper → re-centred), fixed `PILL_HEIGHT` only when open.

**Still `review` (NOT done):** Andi has cosmetic-polish change requests (to be given in a fresh session) + final `review→done` sign-off. One defer remains: **5.1-C2 late-chunk race** (Auto-Loop cycle bleed) — not reproduced in this smoke, still tracked in `deferred-work.md`. Config flag `livePreviewEnabled` left `true` on Andi's machine (backup `config.json.pre-livepreview-bak`).
