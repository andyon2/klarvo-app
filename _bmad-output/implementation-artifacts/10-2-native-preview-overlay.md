# Story 10.2: Native Preview Overlay

Status: review

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
  with **Segoe UI** (the system-ui fallback the web preview renders today — RESOLVED at GATE 1, see
  Font dev note; not Inter, not bundled), size 11 px (default; via `previewFontSize`
  small/medium/large = 11/13/15 px)
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

### Font: reproduce the CURRENT rendered face = Segoe UI (system-ui) — RESOLVED at GATE 1

**Decision (Andi/conductor, GATE 1 2026-06-27 — do not re-litigate):** Render the preview text with
**Segoe UI** via `CreateFontW`. Do **NOT** bundle Inter, do **NOT** use Geist for the preview.

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
