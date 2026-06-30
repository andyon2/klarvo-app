# Story 10.2: Native Preview Overlay

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a Klarvo user,
I want the live-preview card to stay visible when an app covers it,
so that I can read the transcript-so-far while dictating into that app.

## Context & Why

Story 10-1 replaced the WebView2 pill with a native `WS_EX_LAYERED | WS_EX_TOPMOST` window — the
proof slice that validated the entire Win32 layered-window substrate (shape rasterizer, per-pixel
alpha, GDI text, in-process state drive, drag/position persistence). The preview window (WebView2
`"preview"`, 400×600 transparent click-through) still runs as a WebView2 overlay and suffers the
**same occlusion-blank defect class** as the old pill did — the compositor halts and the preview
goes blank when a foreground window covers it.

This story migrates the preview to the **same substrate** Story 10-1 proved. It is the simpler
half: the hard primitives (layered-window lifecycle, DIB present, platform-gating, thread model,
GDI font loading) already exist in `native_pill.rs` and can be reused. The preview's unique
complexity is **multi-line scrollable text rendering** (the pill only rendered short labels).

**This is a technology migration, NOT a re-skin.** The native preview reproduces the current
`PreviewPanel.tsx` appearance 1:1. The parked Epic 8 Studio-Dark re-skin is explicitly out of scope.

**AR-4 retirement:** Once both overlays are native, the bundled-runtime pin + self-heal machinery
from ADR-0020 is retired and ADR-0021 supersedes ADR-0020 (AC-5). See the ADR context below.

## Acceptance Criteria

**AC-1 — Native layered window replaces the WebView2 `preview`:**
Given Story 10-1 established the native layered-window substrate
When a recording starts (the native preview is created/recreated at recording-start, mirroring the
native pill's recreate-on-start model from Story 10-3 — see AC-6; this replaces the old app-start
`create_preview_window(app)` call at `lib.rs:1035`)
Then a native Win32 top-level window with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW |
WS_EX_NOACTIVATE | WS_EX_TRANSPARENT` is created (hidden, at the saved/default pill-anchored
position), its content presented via `UpdateLayeredWindow(ULW_ALPHA)` from a premultiplied-BGRA DIB
And the WebView2 `"preview"` window (`create_preview_window`, `lib.rs:831`), its
`main.tsx:28` route (`label === "preview" && !isPreviewMode`), and `src/PreviewPanel.tsx` are
removed
And the change is gated `#[cfg(target_os = "windows")]` (same as `native_pill.rs`)
And `capabilities/default.json` `"windows"` array is updated to `["main"]` (removing the now-dead
`"bar"` and `"preview"` entries that were never updated in Story 10-1)

**AC-2 — Renders the current preview look 1:1 — dark card with scrollable text:**
Given live-preview chunks arrive during recording
Then the native preview renders the current `PreviewPanel.tsx` appearance:
- **Background card:** `rgba(25,25,25,0.96)` dark fill (default; user-configurable via
  `previewBgColor`)
- **Border:** `1px solid rgba(42,195,168,0.25)` teal hairline (default; user-configurable via
  `previewBorderColor`/`previewBorderWidth`)
- **Corner radius:** 14 px (default; user-configurable via `previewBorderRadius`)
- **Text:** `rgba(220,220,220,0.88)` (default; user-configurable via `previewTextColor`), rendered
  **Inter-first** via the configured `previewFontFamily` cascade (CORRECTED at GATE-4 — the old
  preview renders Inter on Andi's machine; see Font dev note), size 11 px (default; via
  `previewFontSize` small/medium/large = 11/13/15 px)
- **Card height:** the dark card is sized to the **content** (text height + padding), bottom-aligned
  and growing upward inside the fixed-max-height window — NOT the full window height. With one line
  the card hugs that line just above the pill; the rest of the window stays transparent
  (mirrors the old `PreviewPanel.tsx` flex-end grow-up; see GATE-4 Defects)
- **Text accumulation:** chunks space-joined, oldest at top, newest at bottom (bottom-aligned
  grow-up)
- **Top-fade when text overflows:** `linear-gradient(to bottom, transparent 0%, black 18%)` applied
  at the top edge so oldest text fades out — must be rasterized as an alpha gradient overlay on the
  top portion of the card
- **Backdrop blur: dropped** (ADR-0021 sub-decision 3; `previewBgBlur` config field is read but
  ignored by the native renderer — the ~96%-opaque fill makes this negligible; the Settings UI
  slider for blur is left in place unchanged)
- **Padding:** 2 px outer inset (to keep borders inside the DIB), 8 px/14 px inner (top-bottom /
  left-right)
- Window is hidden until first chunk of each recording cycle

**AC-3 — Click-through + pill-anchored positioning (parity with today):**
Given the preview is visible
Then cursor events pass through it (`WS_EX_TRANSPARENT` — no change from today)
And it is anchored above the pill: center-aligned on the pill's horizontal center
  (`pillX + 100 - previewW/2`), bottom edge at `pillY - 8` (GAP=8 px)
And it repositions when the pill is dragged (by listening to `klarvo://bar-moved {x, y}` in-process,
  the same event the native pill already emits — no new emitter needed)
And horizontal clamping to work-area bounds is applied on position (mirroring `PreviewPanel.tsx`
  monitor-clamp logic, same formula: `max(left+12, min(pillCenter-W/2, right-W-12))`)

**AC-4 — Occlusion-survival, machine-verified:**
Given the preview is visible during recording
When a foreground app is maximized over its screen region, and again after a 3 s dwell
Then it stays fully painted (content pixels > 0) — verified by the ADR-0021 occlusion harness
(adapted from `scripts/desktop-occlusion-proof.ps1`, targeting `KlarvoPreviewNative` window class,
evidence written to `gate4-evidence/10-2/`)

**AC-5 — Retire ADR-0020 machinery:**
Given both overlays are now native
Then:
(a) `WEBVIEW2_BROWSER_ARGS` constant in `lib.rs` (only needed for WebView2 overlay windows; `main`
    uses its own copy in `tauri.conf.json`) is removed or clearly annotated as dead code
(b) `create_preview_window`, `set_preview_shape`, `ensure_preview_window` are removed from
    `lib.rs` / `commands/misc.rs` (they operated on the WebView2 preview)
(c) The dead `create_bar_window` function body in `lib.rs` (~line 681, replaced in Story 10-1 but
    not deleted) is removed
(d) ADR index `docs/adr/README.md` already marks ADR-0020 "Superseded" on this branch; confirm
    that entry is correct and no sync-and-build.ps1 runtime-pin machinery exists (per current state
    of the file — it does not; this is a confirmation step only)
(e) The close-event handler in `lib.rs:1116` that prevents-close on `label == "preview"` is removed

**AC-6 — Standby-resilience: preview recreated at each recording start (RESOLVED at GATE 1 — apply 10-3's proven pattern proactively):**
Given the native preview is a long-lived `UpdateLayeredWindow` layered window — the **same** DWM
present-loss failure class that broke the native pill after Modern Standby (Story 10-3) applies
identically to it
When a recording starts (the preview's creation point per AC-1, in the recording-start path
**before** `emit_pipeline_state(recording())`)
Then a **fresh** `NativePreview` window is created and swapped into `AppState.native_preview`, and
the previous preview (if any) is torn down via its `Drop` (posts `WM_PREVIEW_SHUTDOWN` →
`DestroyWindow`) — mirroring 10-3 AC-1
And the new preview is created **before** the old handle is dropped, so a transient `create` failure
leaves the previous preview in place (never "no preview"); on `Err` the old handle is kept and the
failure is logged at `error`
And any obsolete `is_alive()`/`IsWindow()` liveness gate is **not** used to decide recreation (it
cannot detect the uncomposited-but-alive state) — recreation is unconditional per recording start
And the saved pill position (`config.bar_x/bar_y`) is re-read and the preview re-anchored, so a drag
during recording N is restored for recording N+1
And on the hidden→visible transition (first chunk of the cycle) the preview re-asserts topmost via
`SetWindowPos(hwnd, HWND_TOPMOST, 0,0,0,0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)` — mirroring
10-3 AC-3 (parity the WebView2 overlay had, the native rewrite must not drop)
And the standby survival is **human-verified** in Andi's smoke: record once (preview appears) →
real sleep/standby/lock-resume cycle → record again → preview still appears (the exact transition
that broke the pill on 2026-06-27; sleep/resume is a user-reachable state — verification symmetry,
not machine-claimed)

### DoD (surface-class)

- Real Windows release build via `scripts/sync-and-build.ps1`.
- **Occlusion harness PASS** (machine, agent-run, AC-4) — content pixels survive foreground
  occlusion + 3 s dwell; evidence written to `gate4-evidence/10-2/` before the human gate.
- **Andi smoke on real Windows** (NFR-2): preview card looks right (text visible in **Segoe UI**,
  border, radius), appears anchored above pill, click-through (events pass through to app behind),
  repositions when pill is dragged, survives occlusion, hidden when not recording.
- **Standby smoke (AC-6, NFR-2):** record → real sleep/standby/lock-resume → record again → preview
  still appears (mirrors the 10-3 pill standby gate; user-reachable state, not machine-claimed).
- `cargo check --target x86_64-pc-windows-gnu` via Win32 surface harness (reuse 10-1 recipe at
  `gate4-evidence/10-1/win32-surface-check.md`); Linux `cargo test` green; `tsc` / `npm run build`
  green after PreviewPanel.tsx removal.
- Surface-smoke-checklist traps reviewed: Trap #4 (geometry/region) N/A (native has no Win32 region
  for click-through — `WS_EX_TRANSPARENT` covers it); Trap #5 (event wiring) N/A (in-process drive);
  Trap #1/#2/#6 N/A (no new config keys or Settings chain added).
- Code-review inversion (reviewer-verified, not self-attested) per project rules.

## Tasks / Subtasks

- [x] **Task 1 — Create `native_preview.rs` module with Win32 layered-window substrate** (AC: 1, 4)
  - [x] Add `src-tauri/src/native_preview.rs`, `#[cfg(target_os = "windows")]`
  - [x] Register window class `KlarvoPreviewNative` (unique name for harness FindWindow lookup)
  - [x] `CreateWindowExW` with `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW |
        WS_EX_NOACTIVATE | WS_EX_TRANSPARENT`, `WS_POPUP`; hidden at startup
  - [x] Build top-down 32bpp premultiplied-BGRA DIB section; present via `UpdateLayeredWindow
        (ULW_ALPHA)` — identical pattern to `native_pill.rs`
  - [x] Dedicated OS thread owns HWND + message loop (same model as `native_pill.rs`)
  - [x] Public `NativePreview` handle with `PostMessageW`-based API:
        `NativePreview::create(pill_x, pill_y, config_snapshot)`,
        `set_state(PipelineState)`, `append_chunk(Box<String>)`, `set_pill_pos(x, y)`, `is_alive()`

- [x] **Task 2 — Config snapshot and geometry** (AC: 2, 3)
  - [x] Define `PreviewConfig` struct holding the appearance values read from `AppConfig` at
        recording-start (bgColor parsed to BGRA u32, textColor, borderColor, borderWidth u8,
        borderRadius u8, fontFamily String, fontPx u32)
  - [x] Width presets: compact=260, comfortable=320, wide=400 (logical px); scale factor
        `k = fontPx / 11`; `W = base_w * k`, `H_max = 600 * k` — match PreviewPanel.tsx
        `previewGeometry()` exactly
  - [x] Geometry position: `previewLeft = pill_x + 100 - W/2` (pill 200 wide, centered),
        clamped to `[workAreaLeft+12, workAreaRight-W-12]`; `previewTop = pill_y - 8 - H_max`
        (GAP=8; window sized at max height, card grows within it)
  - [x] Read work-area via `SystemParametersInfoW(SPI_GETWORKAREA)` (same Win32 call pattern as
        the pill's center-bottom fallback, `native_pill.rs`)
  - [x] `UpdateLayeredWindow` result must be checked and logged on failure
  - [x] Log `UpdateLayeredWindow` failure with `GetLastError()` at `log::warn!`

- [x] **Task 3 — CPU rasterizer: dark card with text** (AC: 2)
  - [x] **Background card:** tiny-skia rounded-rect, fill `bgColor` premultiplied, stroke
        `borderColor`×`borderWidth`, radius = `borderRadius`
  - [x] **Multi-line text rendering via GDI:** use `CreateFontW(L"Segoe UI", …)`,
        `DrawTextW(..., DT_WORDBREAK)` into a tmp DIB (white text on black), then
        alpha-composite the B-channel coverage onto the main BGRA DIB
  - [x] **Text buffer:** maintain a `String` accumulating all chunks (space-joined) per recording
        cycle; on each `append_chunk` re-render the full text
  - [x] **Bottom-aligned grow-up / scroll:** measure text height via `DrawTextW DT_CALCRECT`;
        if text fits, render bottom-aligned; if overflows, render from top (newest at bottom)
  - [x] **Top-fade when overflowing:** alpha gradient from alpha=0 at top to alpha=255 at ~18%
        of card height — `apply_top_fade()` writes alpha bytes directly into the BGRA DIB
  - [x] **Outer inset:** 2 px on all sides (OUTER_INSET constant)
  - [x] **Card inner padding:** 8 px top/bottom (INNER_PAD_TB), 14 px left/right (INNER_PAD_LR)

- [x] **Task 4 — In-process state + chunk drive + standby-resilient recreate** (AC: 2, 3, 6)
  - [x] Add `native_preview: Mutex<Option<native_preview::NativePreview>>` to `AppState`
        (`lib.rs`), initialized to `None`
  - [x] **Recreate-on-recording-start (AC-6, mirror 10-3 pill):** in `pipeline.rs` after the
        native-pill recreate block, create a fresh `NativePreview` from config snapshot and swap
        into `AppState.native_preview`; on `Err` keep old handle + log error
  - [x] **Topmost re-assert on show (AC-6, mirror 10-3 AC-3):** `was_visible` edge-gate fires
        `SetWindowPos(HWND_TOPMOST)` on first chunk of each cycle
  - [x] `emit_pipeline_state` (`lib.rs`): call `native_preview.set_state()` in-process (Windows)
  - [x] `flush_preview_delta` (`pipeline.rs`): after `handle.emit("klarvo://live-preview-chunk")`,
        also call `native_preview.append_chunk(text)` in-process
  - [x] `klarvo://bar-moved` repositioning: Rust-side `app.listen` in setup block → `set_pill_pos`
  - [x] `ensure_preview_window` recovery command: updated to native `is_alive()` check

- [x] **Task 5 — Remove the WebView2 preview surface** (AC: 1, 5)
  - [x] Delete `src/PreviewPanel.tsx`
  - [x] Remove `label === "preview" && !isPreviewMode` branch from `src/main.tsx`
  - [x] Remove `create_preview_window` from `lib.rs`; remove its call in setup block
  - [x] Remove the close-event handler guard for `label == "preview"` at `lib.rs`
  - [x] Remove `WEBVIEW2_BROWSER_ARGS` constant (dead — `main` has its own args in tauri.conf.json)
  - [x] Remove dead `create_bar_window` function body from `lib.rs`
  - [x] Remove `set_preview_shape` from `commands/misc.rs` and `lib.rs` invoke_handler
  - [x] Replace `ensure_preview_window` old WebView2 impl with native preview check
  - [x] Update `capabilities/default.json` `"windows"` to `["main"]`
  - [x] `transcribe_live_preview` removed from `commands/recording.rs` and `tauri-commands.ts`

- [x] **Task 6 — Occlusion harness for preview + verification** (AC: 4, 5)
  - [x] Create `scripts/preview-occlusion-proof.ps1` (adapted from pill harness, targets
        `KlarvoPreviewNative` class, evidence dir `gate4-evidence/10-2/`)
  - [x] Linux `cargo test` — 18 passed, 0 failures
  - [x] Win32 surface check: 0 errors via scratch harness (same recipe as 10-1)

### Review Findings (code-review 2026-06-27, range b658320..2b2fbae)

Conductor-confirmed against source. Discriminator: NEW-in-10-2 (fix) vs mirrored-from-`native_pill.rs`
(shipped substrate → defer/dismiss).

**Patches (fix this round — all NEW in 10-2, unambiguous):**
- [x] [Review][Patch] Overflow renders OLDEST text + clips NEWEST off the bottom (AC-2 violation: "newest at bottom in view") — `native_preview.rs:623-637`. On overflow `start_y = inner_top` draws top-down so the newest lines push past `inner_bottom` and the top-fade hides the oldest. Fix: `start_y = inner_bottom - text_h` (negative top; oldest overflows up and is faded, newest sits at `inner_bottom`).
- [x] [Review][Patch] Card colors double-premultiplied → teal hairline ~4× too faint / near-invisible (AC-2 "teal hairline border") — `native_preview.rs:550-573`. RGB is pre-scaled by alpha AND alpha is passed to `Color::from_rgba`, which premultiplies again (`rgb·a²`). `native_pill.rs:525` does it correctly (straight alpha). Fix: pass straight rgb (`bg_r/255.0`, not `bg_r/255.0 * bg_a`) for both bg fill and border stroke.
- [x] [Review][Patch] `rebuild_dibs` use-after-free + partial leak on DIB-create failure — `native_preview.rs:843-876`. Old DCs/bitmaps are deleted FIRST (845-848); the `(Err,_)` arm leaves `s.main_dc/main_bits/tmp_*` dangling and `render_frame` (795) writes into freed GDI memory; partial success leaks the new main DIB. Fix: create both new DIBs FIRST, swap only on full success, delete old after; on failure keep old intact and return.
- [x] [Review][Patch] Reposition-while-hidden leaves `phys_w/phys_h` + DIBs stale → window snaps back / size mismatch on next show (AC-3 path) — `native_preview.rs:796-806`. Hidden branch `SetWindowPos(pw,ph)` resizes the HWND but never updates `s.phys_w/phys_h` nor rebuilds DIBs (vertical clamp makes size depend on `pill_y`). Fix: when `pw!=phys_w || ph!=phys_h`, call `rebuild_dibs` in the hidden branch too (rebuild updates phys + window rect).
- [x] [Review][Patch] Configured text alpha (default 0.88) parsed then discarded → native text more opaque than web (AC-2 "exact values to reproduce") — `native_preview.rs:644-647` + `composite_text_mask`. `parse_css_rgba` alpha dropped; coverage never modulated by `text_a`. Fix: thread `text_a` into `composite_text_mask` and scale glyph coverage by it.

**Deferred (real, reported not fixed this round — residual):**
- [x] [Review][Defer] Geometry degenerate-case guards: `h_max=0` when pill near work-area top → negative card dims fed to Pixmap (`native_preview.rs:1444-1460`); horizontal clamp inverts when window wider than work-area → off-screen-left (`compute_preview_geometry` clamp). Rare; one geometry-hardening follow-up (`.max(1)` clamps + `if min<max` guard).
- [x] [Review][Defer] Multi-monitor: `SPI_GETWORKAREA` is primary-only + DPI sampled once from primary DC → preview on a secondary/mixed-DPI monitor mis-positioned/scaled. Mirrored substrate limitation (native_pill has the same primary-work-area assumption); a shared overlay follow-up if Andi runs multi-monitor.
- [x] [Review][Defer] `parse_css_rgba` accepts only `rgb()/rgba()`; hex/hsl/named silently fall back to defaults — AC-2 defaults (rgba strings) render correctly; only user-customized non-rgba colors affected. Verify the Settings color-picker output format; add hex parsing if it emits hex.
- [x] [Review][Defer] `bar-moved` missing x/y → `unwrap_or(0.0)` snaps preview to (0,0) (`lib.rs:773-778`). Emitter always sends `{x,y}` f64 (Edge-verified), so defensive-only; cheap skip-on-missing guard.
- [x] [Review][Defer] One-time GDI/state leak if `CreateWindowExW` fails (`native_preview.rs:1031-1034`) + per-shutdown-race `Box<String>` leak on a queued append-chunk (`native_preview.rs:750-756`). Rare, bounded; reclaim-on-error fixes.
- [x] [Review][Defer] `line-height:1.5` / `letter-spacing:0.01em` not reproduced (GDI `DrawText` natural leading ~1.2) → text tighter than web. Real 1:1 delta but disproportionate GDI fix cost; **surface to Andi's visual smoke** (story already flagged font-metric fidelity).
- [x] [Review][Defer] Occlusion harness `preview-occlusion-proof.ps1`: dead `$EvidenceDir` param with literal-space typo (`_ bmad-output`); body works via `.Replace` workaround. Cosmetic; clean up. AND validate the PASS-criterion (`content>20` counts the dark card bg) really distinguishes composited-vs-blanked at GATE 4 when the harness runs.

**Dismissed (noise / mirrored-and-benign / false positive):** 32-bit f64-param truncation (Windows target is x64-only, documented); `GetMessageW BOOL(1)` match (mirrored `native_pill.rs:1399`, works with current windows-rs); `compute_preview_geometry` gratuitous `unsafe fn` (style); config double-lock TOCTOU in recreate (negligible); mutex-poisoning fail-soft (codebase-wide `if let Ok(guard)` convention); dangling frontend refs (Edge-verified removal surface clean); `is_alive` recycled-HWND + `UnregisterClassW` recreate race + detached-thread-no-join (all mirrored from shipped `native_pill.rs` substrate, benign under Win32).

### GATE-4 Defects (Andi real-device smoke, 2026-06-27) — story re-opened to in-progress

Two visual defects on the real Windows build (build green; these are render-fidelity, not compile).
Causes named from code + the old `PreviewPanel.tsx` SOLL (read from git `2b2fbae^`). NOT transparency
(that is fine — conductor misread; dropped).

- [x] **[GATE-4][Patch] Card rendered at full max-height instead of content-height.** The window is
  correctly fixed at max-height, but `render_frame` fills the card at the FULL window height
  (`native_preview.rs:546` `card_h = ph - 2*inset`), so one sentence produces a giant card. SOLL
  (`PreviewPanel.tsx`: "dark card grows upward inside a fixed-max window", `justifyContent: flex-end`):
  the opaque card is only content-tall (text height + 2×`INNER_PAD_TB`), bottom-aligned at the window
  bottom (just above the pill), growing up, clamped to max; the rest of the window stays transparent.
  **Fix:** measure text height (DT_CALCRECT) BEFORE drawing the card; compute `card_h =
  min(text_h + 2*INNER_PAD_TB*sc, ph - 2*inset)`; draw the round-rect card + border + text at
  `card_y = (ph - inset) - card_h`; top-fade only when text actually overflows the max.
- [x] **[GATE-4][Patch] Font hardcoded to Segoe UI; must be Inter-first.** `native_preview.rs:989`
  hardcodes `"Segoe UI"`. SOLL = `previewFontFamily` cascade `'Inter', system-ui, …` and Andi's
  machine renders Inter. **Fix:** parse the first family token from `cfg.preview_font_family` (default
  "Inter") and pass it to `CreateFontW`; keep `font_h = font_px * scale`. See corrected Font dev note.

## Dev Notes

### This is a technology migration, anchored to the CURRENT render

The binding appearance SOLL is the **current** `src/PreviewPanel.tsx` — reproduce it 1:1.
Do **not** apply Epic 8 Studio-Dark. ADR-0021 is the binding architecture decision.

### PreviewPanel.tsx appearance — exact values to reproduce

Extracted from `src/PreviewPanel.tsx` (full file read):

```
Outer wrapper: padding: 2 (all sides), flexDirection: column, justifyContent: flex-end
Card element (#preview-card):
  background:   cardAppearance.bgColor      → default "rgba(25,25,25,0.96)"
  backdropFilter: blur(bgBlur px)           → DROPPED in native (sub-decision 3)
  border:       borderWidth px solid borderColor → default "1px solid rgba(42,195,168,0.25)"
  borderRadius: cardAppearance.borderRadius → default 14
  overflow:     hidden (overflowY: auto when scrolling, but no manual scroll — click-through)
  WebkitMaskImage (when scrolling):
    linear-gradient(to bottom, transparent 0%, black 18%)  ← top-fade
  padding:      "8px 14px"
  fontSize:     cardFontPx (11 / 13 / 15 for small/medium/large)
  lineHeight:   1.5
  letterSpacing: 0.01em
  color:        cardAppearance.textColor → default "rgba(220,220,220,0.88)"
  fontFamily:   cardAppearance.fontFamily → default "'Inter', system-ui, -apple-system, sans-serif"
  overflowWrap: anywhere
```

### Geometry — reproduce `previewGeometry()` exactly

```
FONT_PX = { small: 11, medium: 13, large: 15 }
BASE_WIDTH = { compact: 260, comfortable: 320, wide: 400 }
BASE_MAX_HEIGHT = 600
GAP = 8      // px between preview bottom and pill top
PILL_WIDTH = 200

fontPx = FONT_PX[config.previewFontSize] ?? 11
k = fontPx / 11
W = round(BASE_WIDTH[config.previewPanelForm] * k)
H = round(600 * k)   // clamped to monitor room above pill

pillCenterX = pillX + PILL_WIDTH / 2
previewLeft = pillCenterX - W / 2
// Horizontal clamp:
previewLeft = max(workAreaLeft + 12, min(previewLeft, workAreaRight - W - 12))

previewTop = pillY - GAP - H
```

Note: `PreviewPanel.tsx` also applies a **vertical clamp** (limits H to `pillY - GAP - workAreaTop - 12` if less than BASE_MAX_HEIGHT). Replicate this: `H = min(H, pillY - GAP - workAreaTop - 12)`.

### Config values to read from AppConfig

All `previewXxx` fields on `AppConfig` (`lib.rs:200-224`), serde-renamed camelCase:

| Rust field | JSON key | Default |
|---|---|---|
| `preview_bg_color` | `previewBgColor` | `"rgba(25,25,25,0.96)"` |
| `preview_text_color` | `previewTextColor` | `"rgba(220,220,220,0.88)"` |
| `preview_bg_blur` | `previewBgBlur` | `12` (ignored by native renderer) |
| `preview_border_color` | `previewBorderColor` | `"rgba(42,195,168,0.25)"` |
| `preview_border_width` | `previewBorderWidth` | `1` |
| `preview_border_radius` | `previewBorderRadius` | `14` |
| `preview_font_family` | `previewFontFamily` | `"'Inter', system-ui, -apple-system, sans-serif"` |
| `preview_font_size` | `previewFontSize` | `"small"` |
| `preview_panel_form` | `previewPanelForm` | `"comfortable"` |

Read these from `AppState.config.lock()` **at recording-start** (before emitting the Recording state), store in a `PreviewConfig` snapshot on the `NativePreview` struct, and use that snapshot for the whole recording cycle (no per-chunk re-read). Mirror how the pill reads `bar_x/bar_y` from config at recording-start (Story 10-3 `pipeline.rs:599-622`).

### CSS color parsing

Config stores colors as CSS rgba strings (e.g. `"rgba(25,25,25,0.96)"`). Parse these in Rust with a lightweight helper — no external crate needed; regex or manual `sscanf`-style parse suffices. Convert alpha 0.0–1.0 to 0–255 and premultiply (for the BGRA DIB). The `composite_gdi_text` technique in `native_pill.rs` for text shows how to derive premultiplied BGRA from channel values — reuse the same idiom.

### Font: render Inter-first (the configured cascade) — CORRECTED at GATE-4 (2026-06-27)

**⚠️ The earlier GATE-1 "Segoe UI" decision was WRONG and is overturned.** Andi's real-device smoke
showed the native font is visibly different + smaller than his old preview — which proves his machine
HAS Inter installed and the old WebView2 preview rendered **Inter** (the first family in the CSS
cascade), not the Segoe fallback. Hardcoding Segoe was a fidelity regression.

**Decision (Andi smoke, GATE-4 2026-06-27):** Honor the configured `previewFontFamily` cascade —
parse the first family token from `cfg.preview_font_family` (default `"'Inter', system-ui, …"`) and
pass it to `CreateFontW` (default face **"Inter"**). GDI renders Inter when installed (matching the
old preview on Andi's machine), else substitutes — same behavior as the web `font-family` cascade.
Keep `font_h = font_px * scale` (DPI scaling is correct). Do NOT hardcode Segoe; do NOT use Geist.

<details><summary>Superseded GATE-1 reasoning (kept for the record)</summary>

Render with Segoe UI; assumed the web preview fell back to system-ui because Inter is not bundled as
a `.ttf`. This missed that Inter can be **system-installed** (it is, on Andi's machine), so the old
render was Inter, not Segoe. Anchored to "is it bundled" instead of "what actually renders on the
target" — the verify-against-the-real-render rule.</details>

Rationale — this is the faithful "1:1 with the current render", not a compromise:
- The preview card's `fontFamily` is `"'Inter', system-ui, -apple-system, sans-serif"` — it does
  **not** use the app's Geist variable.
- `styles.css` `@font-face` bundles **only Geist**; there is **no Inter `.ttf`/`.woff2` in the repo**.
  Inter is therefore only rendered in the web build if the user has Inter installed system-wide;
  otherwise the CSS stack falls to `system-ui` = **Segoe UI** on Windows. That fallback is the face
  that actually renders today on a stock Windows machine — so reproducing it is what "1:1" means.
- Bundling Inter would make the native preview look **different** from today's web preview (it would
  start rendering Inter where the web build shows Segoe UI) — a fidelity *regression* against the
  binding "reproduce current" SOLL.

Implementation: resolve `previewFontFamily` by honoring the CSS stack to its first realistically
available face; since Inter is neither bundled nor assumed-installed, that face is **Segoe UI**.
`CreateFontW(L"Segoe UI", …)` — no `AddFontMemResourceEx` needed (Segoe UI is a Windows system font).
The config field `previewFontFamily` stays as-is (unchanged); only the native *resolution* of it is
Segoe UI. **Flag the exact rendered face in Andi's smoke** — if his machine has Inter installed and
his current preview looks different, that is the one fidelity point to confirm (separate follow-up if
so, not a 10-2 blocker).

### `native_pill.rs` patterns to reuse

- Thread model (dedicated OS thread, `pill_thread` pattern → `preview_thread`)
- DIB creation (`CreateDIBSection`, top-down, 32bpp)
- `UpdateLayeredWindow(ULW_ALPHA)` presentation
- RGBA→BGRA swap helper (`copy_rgba_to_bgra`)
- GDI text compositing (`composite_gdi_text` or equivalent — B-channel coverage mask)
- `PostMessageW`-based public API (`WM_APP` range: 0x8000–0xBFFF; pick non-overlapping codes,
  e.g. start at 0x8100 for preview)
- Custom shutdown message (`WM_PREVIEW_SHUTDOWN`)
- DPI scaling via `GetDeviceCaps(LOGPIXELSX)` — physical px = logical px × scale
- `Send`/`unsafe impl Send` pattern for the struct
- `log::warn!` on `UpdateLayeredWindow` failure + `GetLastError()` (from Story 10-3 fix)

### `emit_pipeline_state` augmentation (lib.rs:522)

```rust
// Add after the existing native_pill block:
#[cfg(target_os = "windows")]
{
    if let Ok(guard) = handle.state::<AppState>().native_preview.lock() {
        if let Some(preview) = guard.as_ref() {
            preview.set_state(&pipeline_state, clipboard_only);
        }
    }
}
```

The preview's `set_state` handles:
- `Recording` → clear text, read config snapshot from AppState, compute position, show window
- `Done | Idle | Error` → hide window, clear text buffer

### `flush_preview_delta` augmentation (pipeline.rs:1987)

```rust
// After: if let Err(e) = handle.emit("klarvo://live-preview-chunk", text.clone()) { … }
// Add:
#[cfg(target_os = "windows")]
if let Ok(guard) = handle.state::<AppState>().native_preview.lock() {
    if let Some(preview) = guard.as_ref() {
        preview.append_chunk(text.clone());
    }
}
```

### `klarvo://bar-moved` listener (setup block in lib.rs)

Install after `native_preview` is created:

```rust
#[cfg(target_os = "windows")]
{
    let handle_bm = app.handle().clone();
    app.handle().listen("klarvo://bar-moved", move |event| {
        if let Ok(payload) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            let x = payload["x"].as_f64().unwrap_or(0.0);
            let y = payload["y"].as_f64().unwrap_or(0.0);
            if let Ok(guard) = handle_bm.state::<AppState>().native_preview.lock() {
                if let Some(preview) = guard.as_ref() {
                    preview.set_pill_pos(x, y);
                }
            }
        }
    });
}
```

This is cheap (no IPC, same-process event dispatch) and decouples `native_pill` from
`native_preview`.

### Preview visibility: live_preview_enabled guard

The WebView2 preview already checked `live_preview_enabled` at the chunk emission site
(`pipeline.rs:1987`) — the chunk is only emitted when live preview is enabled. The native preview
inherits this behaviour automatically via `flush_preview_delta`. No separate gate needed.

However, the `klarvo://state-changed` event is emitted unconditionally. The native preview's
`set_state(Recording)` should only arm (show on first chunk) if `live_preview_enabled == true`.
Read this flag from the config snapshot at recording-start.

### Removal surface — files / symbols to delete / update

| Item | Action |
|---|---|
| `src/PreviewPanel.tsx` | DELETE |
| `src/main.tsx:28-31` (`label === "preview"` branch) | DELETE (3 lines) |
| `lib.rs` `create_preview_window` fn (~line 831) | DELETE |
| `lib.rs:1035` `create_preview_window(app)` call | DELETE |
| `lib.rs:1116` close handler `label == "preview"` guard | DELETE |
| `lib.rs` `WEBVIEW2_BROWSER_ARGS` const (~line 667) | DELETE (now dead — only `main` remains, it has its own copy in `tauri.conf.json`) |
| `lib.rs` `create_bar_window` fn body (~line 681) | DELETE (dead since Story 10-1) |
| `commands/misc.rs` `set_preview_shape` fn (~line 457) | DELETE |
| `commands/misc.rs` `ensure_preview_window` fn (~line 268) | REPLACE with native preview check |
| `lib.rs` `#[tauri::command]` `set_preview_shape` registration (~line 1189) | DELETE |
| `lib.rs` `#[tauri::command]` `ensure_preview_window` registration (~line 1196) | UPDATE (keep command but point to native impl) |
| `capabilities/default.json` `"windows"` | UPDATE to `["main"]` |
| `lib.rs` `set_window_region_round_rect` fn (~line 610) | DELETE — only caller is `set_preview_shape` (misc.rs:469); becomes dead after set_preview_shape is removed |
| `lib.rs` `set_window_region_pill` fn (~line 590) | DELETE — only caller is inside `create_bar_window` (~line 724); becomes dead when that function is removed |
| `commands/misc.rs` `transcribe_live_preview` Tauri command | DELETE — `transcribeLivePreview()` is defined in `tauri-commands.ts:556` but has **zero callers** in the frontend (grep confirms); remove the TS export and the `#[tauri::command]` registration in `lib.rs:1140` alongside the PreviewPanel removal |

### ⚠️ Win32 surface check from WSL

Reuse the exact WSL-side `cargo check` variant documented in `gate4-evidence/10-1/win32-surface-check.md`. This is the machine-checkable gate for Win32 API correctness without a full Windows build. Run it after Task 1 and again after Task 5.

### Testing standards

- **DoD is surface-class** (`project-context.md`): Linux `cargo test` + lint do NOT satisfy it.
  Hard gate = real Windows release build via `scripts/sync-and-build.ps1` + manual Andi smoke.
- **Occlusion = machine-verified** (AC-4): run `scripts/preview-occlusion-proof.ps1`, evidence to
  `gate4-evidence/10-2/` before Andi's gate.
- **Visual fidelity = Andi's smoke** (NFR-2): preview looks right — text visible, teal border,
  card anchored above pill, click-through (click behind it works), repositions when pill dragged.
- Never make the user the rendering oracle: name any visual/geometry defect before changing app
  code. Use `PrintWindow(PW_RENDERFULLCONTENT)` + WSL `CopyFromScreen` harness if needed.
- Tests are inline `#[cfg(test)]` modules; bind tests to real code paths.

### Project structure notes

- `native_preview.rs` → `src-tauri/src/native_preview.rs` (same location as `native_pill.rs`)
  behind `#[cfg(target_os = "windows")]` — never break the Android/Linux build
- `mod native_preview;` in `lib.rs` (~line 62, alongside `mod native_pill;`)
- `AppState.native_preview: Mutex<Option<native_preview::NativePreview>>` at `lib.rs:307`
- Errors structured `Result`/`AppError`, never `panic!`/`todo!`/`unimplemented!` (fail-soft)
- Code + comments English; commits small + scoped, never `git add .`

### References

- [Source: docs/adr/0021-native-desktop-overlays.md] — binding architecture decision, sub-decisions,
  amendment (10-3 standby-present-loss), occlusion harness pattern
- [Source: _bmad-output/planning-artifacts/epics-native-overlays.md] — Epic 10 requirements
  inventory (AR/VR/IR/NFR), Story 10-2 ACs
- [Source: src/PreviewPanel.tsx:1-454] — appearance SOLL (1:1 target); geometry constants,
  `previewGeometry()`, `runShowSequence()`, `cardAppearance` defaults
- [Source: src-tauri/src/native_pill.rs:1-800+] — substrate to reuse (DIB pattern, thread model,
  PostMessageW API, GDI text compositing, RGBA→BGRA, DPI, `UpdateLayeredWindow`)
- [Source: src-tauri/src/lib.rs:522-537] — `emit_pipeline_state` (augmentation point)
- [Source: src-tauri/src/lib.rs:629-643] — `setup_audio_level_emitter` (dual-emit pattern to copy)
- [Source: src-tauri/src/lib.rs:307, 396] — `AppState` fields (where to add `native_preview`)
- [Source: src-tauri/src/lib.rs:831-886] — `create_preview_window` (the thing being removed)
- [Source: src-tauri/src/lib.rs:1033-1037] — setup call site for preview (replace/remove)
- [Source: src-tauri/src/lib.rs:1110-1128] — close handler (remove preview guard)
- [Source: src-tauri/src/lib.rs:645-668] — `WEBVIEW2_BROWSER_ARGS` (remove)
- [Source: src-tauri/src/lib.rs:681-~820] — `create_bar_window` dead code (remove)
- [Source: src-tauri/src/pipeline.rs:1987] — `flush_preview_delta` emit site (augmentation point)
- [Source: src-tauri/src/commands/misc.rs:268-298] — `ensure_preview_window` (replace with native)
- [Source: src-tauri/src/commands/misc.rs:457-476] — `set_preview_shape` (remove)
- [Source: src/main.tsx:28-31] — `label === "preview"` route (remove)
- [Source: src-tauri/capabilities/default.json] — update windows list to `["main"]`
- [Source: _bmad-output/implementation-artifacts/10-1-native-pill-overlay.md] — 10-1 patterns and
  file list; reuse substrate and `windows 0.61.3` Some-wrap conventions
- [Source: _bmad-output/implementation-artifacts/10-3-native-pill-standby-resilience.md] — log ULW
  result, `SetWindowPos(HWND_TOPMOST)` on show, `was_visible` edge-gate pattern to replicate
- [Source: _bmad-output/project-context.md] — code rules (platform gates, surface DoD,
  no-rendering-oracle, no-panics, camelCase config keys)
- [Source: docs/surface-smoke-checklist.md] — trap ledger (review at DoD time)
- [Source: gate4-evidence/10-1/win32-surface-check.md] — WSL Win32 surface harness recipe

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (2026-06-27, create-story)

### Debug Log References

### Completion Notes List

- ✅ Resolved review finding [Patch]: Overflow text anchor inverted — changed `start_y = inner_top` to `start_y = inner_bottom - text_h` on overflow branch; oldest text now overflows off top (faded), newest sits at inner_bottom.
- ✅ Resolved review finding [Patch]: Card colors double-premultiplied — removed `* bg_a` / `* border_a` scaling from RGB channels in bg fill and border stroke; `Color::from_rgba` receives straight RGB now (matching native_pill.rs:525).
- ✅ Resolved review finding [Patch]: `rebuild_dibs` use-after-free + partial leak — restructured to create both new DIBs first; only on full success free old GDI objects and swap; on any failure free whichever new DIB succeeded and leave s.* untouched.
- ✅ Resolved review finding [Patch]: Reposition-while-hidden stale phys/DIB — hidden branch now calls `rebuild_dibs` when `pw != s.phys_w || ph != s.phys_h`; otherwise plain SetWindowPos to move only.
- ✅ Resolved review finding [Patch]: Configured text alpha discarded — added `text_a: u8` to `PreviewConfig`, threaded through `from_app_config`, updated `composite_text_mask` signature and scales glyph coverage by `text_a/255`; default 0.88 opacity (≈224) now renders correctly.
- Gates after patches: Linux `cargo test` **18 passed, 0 failed**; Win32 surface check **0 errors** (24 pre-existing #[must_use] BOOL warnings); `npm run build` **0 errors**, 78 modules transformed.
- ✅ GATE-4 FIX 1 (content-height card): restructured `render_frame` — `DT_CALCRECT` now runs BEFORE skia card draw; `card_h = content_h.min(max_card_h)` where `content_h = text_h + 2×INNER_PAD_TB×sc`; `card_y = (ph−inset)−card_h` (bottom-aligned, grows upward); transparent above card; top-fade only on overflow. `inner_top` variable removed (unused after restructure).
- ✅ GATE-4 FIX 2 (Inter-first font): added `font_face: String` to `PreviewConfig`; `from_app_config` parses first CSS family token from `cfg.preview_font_family` (strips surrounding quotes, takes pre-comma substring, defaults to "Inter"); `CreateFontW` now receives `font_face` via `font_face_null` wide string — no more hardcoded "Segoe UI".
- Gates after GATE-4 fixes: Linux `cargo test` **630+18 passed, 0 failed**; Win32 surface check **0 errors** (24 pre-existing BOOL warnings); `npm run build` **0 errors**, 78 modules.

- Created `native_preview.rs` (1052 lines): `WS_EX_LAYERED|WS_EX_TOPMOST|WS_EX_TRANSPARENT` window,
  `PreviewConfig` snapshot, `NativePreview` public handle, `PostMessageW` API, tiny-skia card renderer,
  GDI multi-line text compositor, bottom-aligned grow-up, top-fade on overflow, `UpdateLayeredWindow(ULW_ALPHA)`,
  `klarvo://bar-moved` repositioning, standby-resilient recreate-on-start pattern.
- Win32 surface check: **0 errors** (scratch harness, `cargo check --target x86_64-pc-windows-gnu`).
  Fixed 3 API-signature issues: `BOOL` import, `DrawTextW` takes `&mut [u16]` not `&[u16]`.
- Linux `cargo test`: **18 passed, 0 failures**.
- TypeScript/Vite: **0 errors**, 78 modules transformed. `PreviewPanel.tsx` removed cleanly.
- Dead WebView2 code removed: `WEBVIEW2_BROWSER_ARGS`, `create_bar_window`, `create_preview_window`,
  `set_preview_shape`, `transcribe_live_preview`, `label=="preview"` close handler, capabilities entry.
- `ensure_preview_window` Tauri command rewritten to native `is_alive()` check.
- **Pending (DoD gates — not agent-runnable on WSL):**
  - Real Windows release build via `sync-and-build.ps1`
  - Occlusion harness: `scripts/preview-occlusion-proof.ps1` → `gate4-evidence/10-2/` (machine, Andi triggers)
  - Andi smoke: preview card visible (Segoe UI, teal border, anchored above pill, click-through, repositions)
  - Standby smoke (AC-6): record → sleep/resume → record again → preview reappears

### File List

**Created:**
- `src-tauri/src/native_preview.rs`
- `scripts/preview-occlusion-proof.ps1`
- `_bmad-output/implementation-artifacts/gate4-evidence/10-2/` (directory)

**Modified:**
- `src-tauri/src/lib.rs` — `mod native_preview`, `AppState.native_preview`, `klarvo://bar-moved` listener,
  removed dead code: `WEBVIEW2_BROWSER_ARGS`, `create_bar_window`, `create_preview_window`, `label=="preview"` handler,
  removed command registrations: `set_preview_shape`, `transcribe_live_preview`, removed unused `WebviewUrl` import
- `src-tauri/src/pipeline.rs` — NativePreview recreate block in recording-start path; `flush_preview_delta` in-process feed
- `src-tauri/src/commands/misc.rs` — `ensure_preview_window` rewritten to native; `set_preview_shape` removed
- `src-tauri/src/commands/recording.rs` — `transcribe_live_preview` removed
- `src-tauri/capabilities/default.json` — `"windows": ["main"]`
- `src/main.tsx` — removed `label === "preview"` branch, removed `PreviewPanel` import; simplified to always render App
- `src/tauri-commands.ts` — removed `setPreviewShape`, `transcribeLivePreview` exports

**Deleted:**
- `src/PreviewPanel.tsx`

## Change Log

- 2026-06-27 (claude-sonnet-4-6): Implementation complete — native_preview.rs created, WebView2 preview surface removed, Win32 surface check 0 errors, cargo test 18/18, tsc/vite build clean. Status → review.
- 2026-06-27 (claude-sonnet-4-6): Addressed code review findings — 5 Patch items resolved (overflow anchor, double-premultiplication, rebuild_dibs UAF, hidden-reposition stale DIB, text alpha). Gates re-verified: cargo test 18/18, Win32 surface 0 errors, npm build 0 errors.
- 2026-06-27 (conductor, claude-opus-4-8): Code review CLEARED (3-reviewer adversarial: Blind/Edge/Auditor → 5 Patch fixed + re-verified at fix commit `d23db1c`, 7 Defer residual, 9 Dismiss). Surface story: status held at `review` — GATE-4 smoke is the hard residual.
- 2026-06-27 (conductor, claude-opus-4-8): GATE-4 build-break round (Andi's first Windows rebuild). Two misses fixed: (1) `sync-and-build.ps1` robocopy lacked `/PURGE` → deleted `PreviewPanel.tsx` orphaned in `D:\apps\klarvo` and broke `npm build` (fix `96fed45`); (2) `lib.rs:773` `app.listen` needed `use tauri::Listener;` → MSVC `E0599` the tauri-shimming WSL surface harness could not see (fix `c362e73`, machine-verified against REAL tauri/windows-gnu with inversion + 3 unused imports cleaned). Harness-gap + lesson recorded in `gate4-evidence/10-2/verdict.md`. Re-verified: Linux cargo test 630/0, mini-tauri win-gnu EXIT 0. Status stays `review`; awaiting Andi's clean rebuild for the GATE-4 smoke. WSL-observable gates all green (cargo test, Win32 surface compile, tsc/npm). Pending Andi Windows GATE-4: build via sync-and-build.ps1 → occlusion harness `preview-occlusion-proof.ps1` (machine, → gate4-evidence/10-2) → visual smoke (Segoe UI text, teal border now visible, newest-text-at-bottom, click-through, repositions on pill drag) → standby smoke (record → sleep/resume → record → preview reappears, AC-6). Review detail + residuals in `### Review Findings`. Minor introduced lint (unused imports recording.rs:352/lib.rs:77/pipeline.rs:41) — non-blocking, noted.
- 2026-06-27 (claude-sonnet-4-6): GATE-4 defect fixes applied (commit `a8e9f66`). FIX 1 — card content-height/bottom-aligned: restructured `render_frame` so `DT_CALCRECT` runs before skia card draw; `card_h = (text_h + 2×INNER_PAD_TB×sc).min(max_card_h)`; `card_y = (ph−inset)−card_h` (bottom-aligned); top-fade only on actual overflow. FIX 2 — Inter-first font: added `font_face: String` to `PreviewConfig`; `from_app_config` parses first CSS token from `cfg.preview_font_family` (strips quotes, defaults to "Inter"); `CreateFontW` receives `font_face` instead of hardcoded "Segoe UI". Gates: Linux `cargo test` 630+18 passed 0 failed; Win32 surface check 0 errors 24 pre-existing BOOL warnings; `npm run build` 0 errors 78 modules.
- 2026-06-27 (conductor, claude-opus-4-8): GATE-4 defect fixes RE-REVIEWED clean (commit `a8e9f66`). Card now content-height bottom-aligned grow-up (was full max-height); font Inter-first via configured cascade (was hardcoded Segoe — GATE-1 call corrected). Gates green (cargo test 630+18, Win32 0 errors, npm build). Status → review; awaiting Andi re-smoke (small card hugging pill for one line, card grows up, Inter font, + AC-6 standby).
