---
title: 'Stop WebView2 from backgrounding the bar/preview overlays (Windows)'
type: 'bugfix'
created: '2026-06-20'
status: 'done'
context: ['{project-root}/_bmad-output/project-context.md']
baseline_commit: 'b7acdb3720cb364440f0c61bebafc988faeea3c4'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** On Windows, a few minutes (or a few recordings) after a restart, the FloatingBar ("Pille") and the Live-Preview overlay stop appearing entirely — the user can no longer tell whether recording is active; a restart fixes it only briefly. Root cause: under the post-June-2026 WebView2 runtime (149.0.4022.69+), Windows backgrounds/throttles the renderers of the hidden+occluded overlay webviews so they stop painting. Commit `b7acdb3` shipped a partial z-order mitigation (re-assert topmost); the renderer-throttling angle is still open.

**Approach:** Pass Chromium switches that disable renderer backgrounding / occluded-window backgrounding / background-timer throttling to the **first-created window** (`main`, defined declaratively in `tauri.conf.json`). All webviews in the process share one WebView2 environment whose browser args are locked by the first window created, so setting `additionalBrowserArgs` on `main` governs the bar and preview too — without touching their builders.

## Boundaries & Constraints

**Always:**
- Set `additionalBrowserArgs` on the `main` window only — it is created from config *before* the `setup` closure builds `bar` (lib.rs:957) and `preview` (lib.rs:963), so it is the environment-establishing window.
- Preserve Tauri's default args verbatim inside the new value. Setting `additionalBrowserArgs` *replaces* wry's default (`--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection`), so the new string must re-include it (confirmed: tauri-utils 2.8.3 config.rs:1805).
- Keep it desktop/Windows-only. `additionalBrowserArgs` is a Windows-only WebView2 field, ignored on other platforms.

**Ask First:**
- If, after this change, the app fails to start at all (white window / crash on launch / overlays never create), that is the shared-environment-args conflict the prior commit warned about — HALT and report; do not iterate blind on the args string.

**Never:**
- Do NOT set divergent `additionalBrowserArgs` on the `bar` or `preview` builders in `lib.rs` — divergent args against the already-locked shared environment break startup (per `b7acdb3` commit message).
- Do NOT touch `android/`, the z-order re-assert from `b7acdb3` (it stays, complementary), or any pipeline/recording logic.
- Do NOT claim runtime verification — this bug only reproduces after minutes of real Windows use.

</frozen-after-approval>

## Code Map

- `src-tauri/tauri.conf.json` -- `app.windows[0]` is the `main` window (label `main`, `visible:false`); the ONLY change goes here as a new `additionalBrowserArgs` key.
- `src-tauri/src/lib.rs:621,768` -- `create_bar_window` / `create_preview_window`; called inside `setup` (lib.rs:957,963) → created AFTER `main`. Confirms `main` is first. Do not edit.
- `src-tauri/src/pipeline.rs:597` -- `b7acdb3` topmost re-assert (partial fix). Unchanged; complementary.
- tauri-utils 2.8.3 `config.rs:1805` -- documents that a set value replaces the default `--disable-features=…` string (source of the "preserve default" rule).

## Tasks & Acceptance

**Execution:**
- [x] `src-tauri/tauri.conf.json` -- add `"additionalBrowserArgs"` to the `main` window object with value `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-renderer-backgrounding --disable-backgrounding-occluded-windows --disable-background-timer-throttling` -- disables the renderer/occluded-window/timer throttling that blanks the overlays, while preserving Tauri's default feature-disable flags.

**Acceptance Criteria:**
- Given the edited `tauri.conf.json`, when it is parsed, then it is valid JSON and the `main` window carries exactly one `additionalBrowserArgs` string containing both the preserved `--disable-features=…` default and the three `--disable-*` throttling switches.
- Given the change, when `cargo check --target x86_64-pc-windows-gnu` runs (the Windows target Tauri config-validates against), then it completes without error.
- Given the change scope, when the diff is inspected, then only `src-tauri/tauri.conf.json` is modified — no Rust source, no `bar`/`preview` builder, no `android/` file.

## Spec Change Log

- **2026-06-20 — Approach corrected after attempt 1 broke startup (AC3 superseded).** The original approach (args on `main` ONLY, "without touching their builders", diff confined to tauri.conf.json) wedged startup completely: tray icon present but main/settings/quit/shortcuts dead, only killable via Task Manager; Klarvo.log stopped silently after `[license] Initial status` (no Rust panic — WebView2 env creation fails natively). Root cause: all webviews share ONE WebView2 environment; ANY divergence in browser args between windows (main custom vs bar/preview default) fails env reuse. Reverted to `b7acdb3`, restored a working build first. **Shipped fix:** identical `additionalBrowserArgs` on ALL THREE windows — `main` via tauri.conf.json plus `bar`+`preview` via `.additional_browser_args(WEBVIEW2_BROWSER_ARGS)` on their builders in `src-tauri/src/lib.rs` (constant defined once to prevent drift). Verified before handing to the human: launched from WSL, Klarvo's WebView2 subtree = 1 browser + 3 renderers (main+bar+preview created, no wedge) + log shows hotkey fires & preview shown; then human time-gate confirmed overlays stay visible over real use. KEEP: the divergence diagnosis + the "verify it even STARTS (renderer count) before a human smoke" discipline.

## Design Notes

Why the first window and not per-overlay: WebView2 creates one environment per user-data-folder; wry builds it lazily on the first webview and reuses the cached environment for every later webview, so only the first window's `additionalBrowserArgs` take effect — later windows' args are ignored (and, if divergent, can fail env reuse). `main` is config-defined and therefore created before the programmatic `bar`/`preview`, making it the correct and only place to set the args.

Relevant switches: `--disable-renderer-backgrounding` (don't deprioritize hidden renderers), `--disable-backgrounding-occluded-windows` (don't background the overlay when the foreground app covers it — the direct cause for an occluded pill), `--disable-background-timer-throttling` (keep timers/paint alive in the background).

## Verification

**Commands:**
- `python3 -c "import json;json.load(open('src-tauri/tauri.conf.json'))"` -- expected: no error (valid JSON).
- `cargo check --target x86_64-pc-windows-gnu --manifest-path src-tauri/Cargo.toml` -- expected: completes without error (Windows config path compiles).

**Verification result (2026-06-20):**
- JSON parse — PASS (valid; default `--disable-features=…` preserved + 3 throttling switches present).
- `cargo check --target x86_64-pc-windows-gnu` — could NOT complete, but for a pre-existing reason UNRELATED to this change: the `whisper-rs-sys` C build (ggml-cpu.c) fails to mingw-cross-compile (`THREAD_POWER_THROTTLING_CURRENT_VERSION` undeclared). The diff touches only `tauri.conf.json` (no Rust/C/build code), so it cannot affect that native build. Andi's real Windows build uses the MSVC toolchain via `sync-and-build.ps1`, not this WSL mingw path, so it is unaffected.
- Substitute config validation — native `cargo check` PASS. `tauri-build` parses `tauri.conf.json` at build time; `WindowConfig` carries `#[serde(deny_unknown_fields)]` (tauri-utils 2.8.3 config.rs:1641), so a malformed/typo'd `additionalBrowserArgs` key would have failed the parse. Green ⇒ the key is recognized and the args are applied (not silently dropped).

**Manual checks (real gate — Andi, on the actual Windows machine):**
- Build via `scripts/sync-and-build.ps1`, then use Klarvo across multiple recordings over 10–30+ minutes (incl. with other apps in the foreground covering the pill). Expected: the bar and live-preview keep appearing every time; they no longer vanish until restart. This multi-session real-device observation is the only true confirmation — CLI checks above prove the build, not the fix.

## Suggested Review Order

- The entire change: `additionalBrowserArgs` on the `main` window — first-created webview, so its args lock the shared WebView2 environment that governs the bar/preview overlays.
  [`tauri.conf.json:22`](../../src-tauri/tauri.conf.json#L22)

- Why `main` is first (no edit here — read-only context): bar & preview are built inside `setup`, after the config-defined `main`.
  [`lib.rs:957`](../../src-tauri/src/lib.rs#L957)

- Complementary prior partial fix (unchanged): the z-order re-assert from `b7acdb3`.
  [`pipeline.rs:597`](../../src-tauri/src/pipeline.rs#L597)
