# ADR-0021: Native Desktop Overlays — replace WebView2 overlay windows with native layered windows

**Status:** Accepted
**Date:** 2026-06-26

## Context

The two transparent, always-on-top desktop overlays — the **pill** (`bar`, 200×36) and the
**live-preview** card (`preview`, dynamic) — **go blank the moment a foreground window covers their
screen region** (i.e. exactly when you dictate *into* the app you are typing in). Symptom over
weeks: "the pill shows briefly after a restart, is gone after a few minutes, restart helps only
briefly."

The bug was declared fixed four times and returned every time:

1. `983aad6` — re-assert bar topmost on each recording start (z-order only).
2. `aad2e82` — identical WebView2 args on all three windows.
3. `2294008` / `03e1e9f` — disable Chromium `CalculateNativeWinOcclusion`.
4. `ed219f1` + [ADR-0020](0020-webview2-fixed-runtime-pin.md) — pin a bundled fixed-version
   WebView2 runtime (`149.0.4022.62`).

**Why each "fix" was a false positive — measured 2026-06-26:** freshly-started renderers paint the
overlays until the *first real occlusion*, then go blank. The runtime pin was nailed down at the
log: the pin was **active** (`[webview2] runtime: …\webview2-runtime`) and the bundled runtime was
genuinely **.62** (the `149.0.4022.62.manifest` is present), yet the overlays went blank again in
the **same continuously-running session**. `PrintWindow(PW_RENDERFULLCONTENT)` returns the full
overlay content (self-render OK, `alpha=255`, not DWM-cloaked) while the foreground app shows
through. **The occlusion-present halt lives inside the WebView2/Chromium compositor** — it stops
delivering the swapchain to DWM while it considers the window occluded. That is *how WebView2
behaves*, not a switch or a runtime version we can set correctly. This is why "fixed" never held.

**Empirical proof, run before committing to the rebuild** (because the bug had faked four fixes): a
native `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` window, teal-filled via
`UpdateLayeredWindow`, placed at the pill position, then **Notepad maximized in the foreground over
the same region** (the exact repro that drove the WebView2 pill to `screenTeal=0`):

| Sample | teal px (of 7600) |
|---|---|
| overlay alone | **7600** (100%) |
| Notepad maximized + foreground over it | **7600** |
| + 3 s dwell (the time-delayed WebView2 failure mode) | **7600** |

A native topmost layered window stays fully composited when occluded. Harness:
`…\Temp\native-proof2.ps1` (+ PNGs in `…\Temp\klarvo-native-proof\`).

## Decision

**Render the pill and preview overlays as native Win32 layered, always-on-top windows drawn from
Rust — not as Tauri/WebView2 webviews.** The `main` settings window stays WebView2 (React/Tailwind);
it is not a transparent occluded overlay and has no such problem.

### Rationale

| Concern | WebView2 overlay (today) | Native layered window |
|---|---|---|
| Survives foreground occlusion | **No** — compositor halts (this ADR's whole problem) | **Yes** — proven 7600/7600 + dwell |
| Depends on Evergreen runtime | Yes (the moving variable that broke it) | No |
| New dependency | — | Win32 FFI already in repo (`windows` 0.61, `Win32_Graphics_Gdi` + `WindowsAndMessaging`); a CPU rasterizer crate |
| Transparency | per-pixel alpha via webview | per-pixel alpha via `UpdateLayeredWindow(ULW_ALPHA)` (native fit) |
| Backdrop acrylic blur | yes (CSS `backdrop-filter`) | not with `UpdateLayeredWindow` (see sub-decision 3) |
| State/data path | Rust → `klarvo://` event → JS → DOM | Rust → native draw (no IPC round-trip) |

A CPU-rastered layered window is the simplest substrate that solves the problem: per-pixel alpha is
exactly what a transparent overlay needs, no GPU swapchain / DirectComposition machinery, and a
~15 Hz re-render of a 200×36 (or preview-sized) bitmap is trivially cheap. The pipeline **state** and
the **RMS** amplitude already live in Rust (`emit_pipeline_state`, `setup_audio_level_emitter`), so
the native window is driven in-process — no new IPC, and the two React overlay entry points
(`FloatingBar.tsx`, `PreviewPanel.tsx`) are retired as their windows are migrated.

### Numbered sub-decisions

1. **Substrate:** a Win32 top-level window with extended styles
   `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` (preview adds
   `WS_EX_TRANSPARENT` for click-through). Content is presented with `UpdateLayeredWindow(ULW_ALPHA)`
   from a top-down 32bpp premultiplied-BGRA DIB section, rasterized on the CPU. No GPU/swapchain, no
   DirectComposition.
2. **Fidelity = reproduce the *current* overlay look 1:1**, anchored to the current `FloatingBar.tsx`
   / `PreviewPanel.tsx` render (teal `#2AC3A8` recording, amber `#FFA344` processing, green `#4ADE80`
   done, red `#FF7369` error; pill 200×36; 5-bar RMS waveform; preview dark card). This is a
   **technology migration, not a re-skin** — it does **not** also apply the parked Epic 8 Studio-Dark
   re-skin (that remains Epic 8's scope). Avoids the cross-platform drift ADR-0019 warns about by
   *removing* the WebView2 renderer rather than maintaining two.
3. **Backdrop acrylic blur is dropped.** `UpdateLayeredWindow` composites per-pixel alpha over
   whatever is behind it but cannot live-blur the backdrop. The current fill is ~96% opaque, so the
   blur is barely perceptible; the loss is accepted. Real blur (DirectComposition + DWM backdrop) is
   deferred unless the smoke gate says it matters.
4. **State + RMS drive the native overlays in-process** (the Rust pipeline calls the native renderer
   directly). The existing `klarvo://state-changed` / `klarvo://audio-level` emitters may remain for
   any other consumer, but the native overlays do not depend on the JS round-trip.
5. **Interaction parity:** pill drag-to-move (manual drag via `WM_NCHITTEST`/`WM_LBUTTONDOWN`) +
   position persistence via the existing `config.bar_x/bar_y`; preview click-through via
   `WS_EX_TRANSPARENT`, bottom-aligned grow-up, anchored to the pill.
6. **Removal:** once a native overlay lands, its WebView2 window + React entry point are removed
   (pill first as the proof slice, then preview). The `main.tsx` label router loses the `"bar"` /
   `"preview"` branches as each migrates.
7. **Supersedes ADR-0020.** The runtime-pin (and its bundled-runtime + self-heal machinery in
   `sync-and-build.ps1`) existed *only* to dodge this occlusion regression. With native overlays the
   driver is gone; ADR-0020's mechanism is retired once both overlays are native (tracked per story).

## Consequences

### Positive

- **Occlusion-blank defect class is killed by construction** — a native layered window has no
  separate renderer to background; DWM always composites its stored bitmap.
- **Independent of WebView2 Evergreen updates** — no MS patch can break overlay visibility again.
- **Removes the overlay IPC round-trip** (Rust → event → JS → DOM) and the dual React overlay entry
  points; state flows straight to the native draw.
- **Retires ADR-0020's complexity** (bundled runtime, master copy, build-script self-heal).

### Negative

- **Overlay visuals are re-authored in native raster** — the waveform, spinner, icons, text, and
  rounded-pill/preview-card shapes must be redrawn in the rasterizer. One-time authoring cost; the
  waveform + anti-aliased text are the non-trivial bits.
- **Backdrop blur is dropped** (sub-decision 3).
- **A native render path now exists for the overlays** — but it *replaces* the WebView2 one rather
  than running alongside it, so there is no ongoing two-renderer divergence (the React overlays are
  deleted as each migrates).

### Mitigations

- **Proof-first:** the pill (small, all the hard primitives — shape, waveform, text, states) ships
  first and validates the whole substrate before the preview reuses it.
- **Occlusion is machine-verified every story** (harness below); **visual fidelity is Andi's smoke
  on real Windows** — never claimed from machine output alone.

## Verifiability Symmetry — occlusion harness

The occlusion property is **machine-checkable** and is verified by the conductor/agent before Andi's
human gate: fill/observe the overlay region, raise a foreground app maximized over it
(`Start-Process notepad` + `ShowWindow SW_MAXIMIZE` + `SetForegroundWindow`), capture the region via
`CopyFromScreen`, count overlay-content pixels, and re-sample after a multi-second dwell (the
time-delayed WebView2 failure mode). PASS = content pixels remain > 0 while occluded, including
after dwell. Reference harness: `native-proof2.ps1` (DPI-aware; run from WSL via the full path
`C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`; WinPS 5.1 → `-ReferencedAssemblies`).

**Visual fidelity** (does the native pill *look* like the approved pill across all states) is **not**
machine-decidable here — it is Andi's eye on a real Windows build. This mirrors the GATE-4 lesson:
structural/occlusion properties = machine; appearance = human.

## Branch / numbering note

This ADR is authored on `feat/native-desktop-overlays` (off `v1-ship`). ADR-0019 (cross-platform
design SSOT) and ADR-0020 (WebView2 runtime pin) live on `conductor/epic-8` / `conductor/epic-9` and
are **not present on this branch**; numbering continues globally (per the README's "global numeric
uniqueness" convention) — the index reconciles at merge.

Related: memory `project_webview2_overlay_backgrounding` (full measurement saga + proof), ADR-0020
(superseded), ADR-0019 (design-SSOT / drift risk), ADR-0018 (Android bubble render-tech — the
analogous "native vs framework rendering" decision on the Android side).
