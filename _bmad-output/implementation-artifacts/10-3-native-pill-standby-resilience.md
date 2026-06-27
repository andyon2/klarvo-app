# Story 10.3: Native Pill Survives Power/Session Transitions (Standby Resilience)

Status: ready-for-dev

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a Klarvo user who leaves the machine on standby between dictations,
I want the recording pill to reappear after the laptop wakes from Modern Standby / sleep / lock,
so that it works **permanently, not until the next restart** — closing the gap Story 10-1's own user story promised.

## Context & Why

Story 10-1 replaced the WebView2 pill with a native `WS_EX_LAYERED | WS_EX_TOPMOST` window presented via `UpdateLayeredWindow(ULW_ALPHA)`. That killed the WebView2 **occlusion-present** halt (AC-5, machine-verified). But a different defect class was exposed in real daily use and **measured live** on 2026-06-27:

**Symptom:** After the machine sits in Modern Standby for hours, the pill stops appearing during recording — even when *unoccluded* (e.g. with the Klarvo main window open). Same process, no restart. A restart fixes it until the next standby.

**Root cause (measured, not guessed — full diagnosis in the ADR-0021 amendment):**
- The pill window still exists, is `WS_VISIBLE`, has the `WS_EX_TOPMOST` bit, is on-screen, is **not** DWM-cloaked.
- `PrintWindow(PW_RENDERFULLCONTENT)` returns the pill's **correct** bitmap (teal K + waveform) → the tiny-skia/GDI/DIB/`UpdateLayeredWindow` render path works perfectly.
- `CopyFromScreen` of the same rect shows **zero** pill pixels → DWM is **not compositing** the window onto the desktop.
- Re-asserting `HWND_TOPMOST` raised it above the (maximized) foreground window in z-order, leaving **0 overlapping windows above it**, yet it still showed nothing on screen → **not occlusion, not z-order, not cloaking**.
- Windows System event log confirms the trigger: repeated **Modern Standby** enter/exit (events 506/507/566) during the 9-hour gap between the last working recording and the broken one.

**Mechanism (Microsoft-documented, KB 2667241; corroborated by independent internal + external expert analysis):** `UpdateLayeredWindow` pushes the bitmap into DWM's composition surface **once**; a layered window never receives `WM_PAINT`. Across a power/session transition DWM rebuilds its composition surfaces — a normal window repaints into the new surface, but the long-lived layered pill has nothing to re-push its bitmap, so it silently stops compositing. Two code facts ensure it never self-heals:
1. `UpdateLayeredWindow`'s return value is discarded (`let _ =`, `native_pill.rs:714`), so a failed/empty present is invisible.
2. The existing recovery gate at `pipeline.rs:603-621` recreates the pill only when `is_alive()` is false — but `is_alive()` == `IsWindow()` returns **true** for the existing-but-uncomposited window, so recovery never fires.

**Empirical narrowing of the fix:** Each diagnostic `set_state` posted to the pill re-issued `UpdateLayeredWindow` into the **existing** DC (= the cheap "just re-present" fix) and did **not** restore the screen. So re-presenting alone is insufficient; the window's composition surface must be re-established. **Recreating the window** re-establishes it — which is exactly what a process restart (the known workaround) does.

**Decision (this story):** Reuse the existing recovery primitive (`NativePill::create` + the `Drop` teardown that already exist) and **recreate the native pill window at each recording start**, replacing the ineffective `is_alive()` gate. A freshly created layered window always has a live DWM composition surface, so the standby present-loss cannot accumulate. Recordings are discrete, infrequent, user-initiated events, so the per-start cost (one short-lived OS thread + window) is negligible and imperceptible. This kills the defect class **by construction** without any fragile power/session-broadcast plumbing on the pill's background thread (which Microsoft's `SMTO_ABORTIFHUNG` broadcast semantics can skip at the worst moment, per the external analysis).

**Scope:** Pill only. The live-preview overlay is still WebView2 (Story 10-2, backlog) and has its own compositor behaviour across standby; it is addressed when the preview goes native, not here. No appearance/design change — this is a lifecycle fix.

## Acceptance Criteria

**AC-1 — Pill window is recreated at each recording start (kills stale-composition by construction):**
Given the app is on Windows and a recording starts (the native-pill block in `pipeline.rs` `start_recording` ~line 599-622, which runs **before** `emit_pipeline_state(recording())`)
When the recording-start path runs
Then a **fresh** `NativePill` window is created via `NativePill::create(handle, saved_x, saved_y)` and swapped into `AppState.native_pill`, and the previous pill (if any) is torn down via its `Drop` (which posts `WM_PILL_SHUTDOWN` → `DestroyWindow`)
And the new pill is created **before** the old handle is dropped, so a transient `create` failure leaves the previous pill in place (never "no pill") — on `Err`, the old handle is kept and the failure is logged at `error`
And the obsolete `is_alive()`/`IsWindow()` liveness gate is removed from this path (it cannot detect the uncomposited-but-alive state and is now subsumed by unconditional recreate)
And the saved position (`config.bar_x/bar_y`) is read and passed so the recreated pill restores the user's dragged position

**AC-2 — `UpdateLayeredWindow` result is observed (no more silent present failures):**
Given the pill presents a frame (`render_frame`, `native_pill.rs:714`)
When `UpdateLayeredWindow(...)` returns
Then its `BOOL`/`Result` is checked, and on failure a `log::warn!` records it with `GetLastError()` — the present result is no longer discarded with `let _ =`
And on success behaviour is unchanged (no functional change in the healthy path)

**AC-3 — Topmost is re-asserted on show (restores the parity dropped from the WebView2 bar):**
Given the pill transitions from hidden to visible inside `render_frame` (after `UpdateLayeredWindow`, around `ShowWindow(SW_SHOWNOACTIVATE)`, `native_pill.rs:726`)
When the pill is shown
Then `SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE)` is called so the pill is pinned to the top of the topmost band (the old WebView2 bar re-asserted topmost on every recording start, commit `b7acdb3`; the native rewrite dropped it — the live diagnosis found the pill at z-index 133, far below the topmost band)
And this does not activate or move the window (drag/position behaviour unchanged)

**AC-4 — No regression to render, drag, position persistence, RMS, or the recording pipeline:**
Given the recreate-on-start + topmost re-assert + ULW-result changes are in place
When recording, transcribing, cleaning, done, error states drive the pill, RMS feeds the waveform, the pill is dragged, and the next recording starts
Then all of Story 10-1's behaviour is preserved: every state renders 1:1 as before (AC-2 of 10-1), the waveform responds in-process (AC-3 of 10-1), drag + `config.bar_x/bar_y` persistence + `klarvo://bar-moved` emission work (AC-4 of 10-1), and the pipeline/hotkeys/paste/main window are unaffected (AC-6 of 10-1)
And because each recording starts from a fresh pill, position is re-read from config on every start (a drag during recording N still persists and is restored for recording N+1)

**AC-5 — Standby-resilience, human-verified on a real reproducible state (the whole point):**
Given a real Windows release build
When Andi (a) records once and confirms the pill appears, (b) puts the machine through a real Modern Standby / sleep / lock-and-unlock cycle (close lid or wait for standby, then resume), and (c) records again
Then the pill appears on the second recording too — i.e. it survives the exact transition that broke it on 2026-06-27
And this is the testable state Andi can produce himself (sleep/resume is user-reachable), satisfying verification symmetry — it is **not** machine-claimed

## Dev Notes

**Files (all Windows-gated):**
- `src-tauri/src/pipeline.rs` ~599-622 (`start_recording`): replace the `is_alive()` recovery block with unconditional create-new-then-swap. Keep the existing `saved`/`(sx, sy)` read. Structure so the new `NativePill` is built into a local first; only on `Ok` acquire the `native_pill` lock and assign (the old handle drops there). On `Err`, keep the old handle and `log::error!`.
- `src-tauri/src/native_pill.rs` ~714 (`render_frame`): capture the `UpdateLayeredWindow` result; `log::warn!` + `GetLastError()` on failure (AC-2). Around the `ShowWindow(SW_SHOWNOACTIVATE)` at ~726: add the `SetWindowPos(HWND_TOPMOST, …)` re-assert (AC-3). Note the `windows 0.61.3` Some-wrap convention from 10-1 — `SetWindowPos(hwnd, Some(HWND_TOPMOST), …)`, `GetLastError()` import from `Win32::Foundation`.
- `src-tauri/src/commands/misc.rs` `ensure_bar_window` (~210): leave as-is (a separate explicit recovery command); optionally note that `is_alive()` there has the same blind spot, but it is not on the hot path — out of scope, tracked as a note.

**Why recreate the whole window (not just rebuild the DC/DIB in place):** the live measurement showed re-presenting into the existing DC does not restore composition; whether recreating only the DC+DIB on the same `HWND` (vs the whole window) suffices is **unverified** and would need its own build to test. Recreating the window is the guaranteed fix (it is restart-equivalent, and restart is the proven workaround), reuses primitives that already exist, and is cheap given recording frequency. If per-start churn or pill-appearance latency shows up in smoke, a conditional/lighter recreate is a documented follow-up — do not pre-optimize.

**Cross-driver safety:** every driver of the pill (`emit_pipeline_state` → `set_state`, `setup_audio_level_emitter` → `feed_rms`) re-locks `AppState.native_pill` on each call, so swapping the handle at recording start is safe — subsequent calls transparently target the new pill. Each `NativePill` owns its own thread + window; there is no shared mutable state between old and new.

**Ordering:** recreate happens at `pipeline.rs:599-622`, before `emit_pipeline_state(recording())` at ~624. The fresh pill starts `Idle` (hidden); the subsequent `recording()` emit drives `set_state(Recording)` → `render_frame` → show. Confirmed correct ordering.

## DoD (surface-class)

- Real Windows release build via `scripts/sync-and-build.ps1`.
- `cargo check --target x86_64-pc-windows-gnu` green via the Win32 surface harness (reuse the 10-1 recipe at `gate4-evidence/10-1/win32-surface-check.md`); Linux `cargo test` green.
- **Andi smoke on real Windows (AC-5):** record → standby/sleep/lock cycle → record again → pill appears. This is the load-bearing gate; standby-resume is not machine-reproducible unattended.
- Andi smoke also re-confirms 10-1 parity (appearance across states, drag, occlusion) is intact (AC-4).
- Code-review inversion (reviewer-verified, not self-attested) per project rules.
- ADR-0021 amendment recording the standby-present-loss root cause + the recreate-on-start decision.

## Change Log

| Date | Change |
|---|---|
| 2026-06-27 | Story authored from live diagnosis of the post-Modern-Standby pill-blank regression (internal + external expert analysis converged on DWM composition-surface loss for long-lived `UpdateLayeredWindow` windows). |
