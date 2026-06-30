# GATE-4 Evidence — Story 10-1 (Native Pill Overlay)

Conductor: bmad-story-conductor (autonomous). Branch `conductor/epic-10`.
Range reviewed: `94a9cc77031e1e8f67150349a3a32a8447ad78d4..HEAD`.
Date: 2026-06-27.

## Verdict: review-cleared, carrying the real-Windows smoke as a hard residual

Both status fields remain `review` (NOT `done`). This is a Windows-only surface story;
its visual + occlusion smoke is Andi's attended real-Windows gate and cannot be run or
claimed from WSL. Do not mark done on the machine gates alone.

## In-scope machine gates — PASSED (verified by the conductor at HEAD `6796358`)

- Linux `cargo test`: **628 + 18 passed, 0 failed** (lib + integration).
- `tsc` (`npx tsc --noEmit`): **green** (exit 0).
- `npm run build` (tsc + vite): **green** — 81 modules, FloatingBar.tsx removal builds clean.
- `tiny-skia 0.11` dependency is correctly placed under `[target.'cfg(windows)'.dependencies]`
  and `native_pill.rs` is `#![cfg(target_os = "windows")]` — Linux/Android builds unaffected.
- All Windows-only symbols referenced by `native_pill.rs` resolve in-tree
  (`save_config_locked`, `lock!`, `recorder.is_recording/stop_recording`, `recording_start`,
  `PipelineEvent::idle/clipboard_only`, `EVENT_STATE_CHANGED`, `bar_x/bar_y`).

### Not achievable in this environment (carried to residual)
- `cargo check --target x86_64-pc-windows-gnu`: **infra-blocked**, NOT a regression. It fails in
  the `whisper-rs-sys` and `ort-sys` build scripts (CMake building whisper.cpp/ggml C++ for a
  Windows target without an MSVC/mingw cmake toolchain). Both crates are untouched by this story;
  the failure is independent of 10-1 and pre-exists in WSL. The first-party `native_pill.rs` never
  reaches the compile step behind those native deps. Its Windows compilation is therefore verified
  only at the real build (`scripts/sync-and-build.ps1`) — see residual. The conductor substituted a
  static `windows`-crate feature-coverage audit (all used `Win32::*` paths covered by the enabled
  features incl. the added `Win32_System_LibraryLoader`).

## Code review (Opus, 3 adversarial layers) — 8 confirmed findings fixed (fix commit `6796358`)

1. CRITICAL — pill was driven only via `emit_pipeline_state` (4 sites); ~16 raw
   `handle.emit(EVENT_STATE_CHANGED, …)` transitions bypassed it → pill stuck on "Recording",
   never advanced to transcribing/cleaning/done, never hid. Fixed: all transitions now route
   through `emit_pipeline_state` (single choke; carries `clipboard_only`).
2. CRITICAL — AC-3 waveform double-boost (boost applied at ingest AND in render). Fixed: applied
   once; render uses the stored sample directly.
3. CRITICAL — waveform ring sampled at fixed absolute indices over a rotating write head. Fixed:
   sampled relative to `waveform_pos` (oldest→newest FIFO, matching FloatingBar); `waveform_pos`
   reset on state change.
4. HIGH — AC-2 mode badge stuck on "Hold" (`set_hotkey_mode` never called). Fixed: fed from the
   `klarvo://active-mode` emit site (Windows-gated), mirroring the deleted FloatingBar listener.
5. HIGH — stop-button cancel emitted raw idle → pill stayed visible. Fixed: routes through
   `emit_pipeline_state(idle)`.
6. MEDIUM — unsound teardown (Drop posted WM_DESTROY without DestroyWindow → zombie window on the
   recreate paths; GDI objects deleted while selected; dangling USERDATA). Fixed: Drop posts a
   custom shutdown msg → `DestroyWindow`; USERDATA nulled before free; no double-free.
7. MEDIUM — `Pixmap::new` failure fell back to a smaller pixmap with a phys-sized copy → OOB
   read/panic at >96 DPI. Fixed: logs + returns (skips frame); no panic.
8. LOW-MED — AC-4 `klarvo://bar-moved` emitted every mouse-move despite "throttled" comment.
   Fixed: ~16 ms time-throttle during drag; final settle emit on release unchanged.

Re-review converged (fixes landed, no regression in touched lines). Linux test + build green at HEAD.

## Structural facts machine-verified (the parts a WSL conductor CAN assert)
- State drive now reaches every `PipelineState` via the single `emit_pipeline_state` choke; tray
  tooltip coupling preserved (AC-6) — `update_tray_tooltip` now reaches its per-state arms as
  designed (Transcribing/Cleaning/Done), a latent-dead-code improvement, not a tray regression.
- AC-3 mapping is byte-identical to FloatingBar and applied exactly once; FIFO ordering restored.
- Preview anchoring contract preserved: `klarvo://bar-moved` + `klarvo://audio-level` still emitted
  (preview stays WebView2 until 10-2).
- Occlusion harness `scripts/desktop-occlusion-proof.ps1` is in the repo (161 lines), matches the
  ADR-0021 native-proof2 template (FindWindow `KlarvoPillNative` → CopyFromScreen baseline →
  maximize Notepad + SetForegroundWindow → occluded capture → 3 s dwell → post-dwell capture;
  PASS = all three pixel counts > 0; PNGs land in this dir; DPI-aware).

## RESIDUAL FOR HUMAN — Andi's attended real-Windows gate (the actual GATE 4)

Build with `scripts/sync-and-build.ps1`, then verify on the real Windows machine:
1. **Occlusion (AC-5, machine harness):** start a recording (pill visible), run
   `powershell -ExecutionPolicy Bypass -File scripts\desktop-occlusion-proof.ps1`. Expect
   `RESULT: PASS` — content pixels > 0 in baseline, occluded, AND after the 3 s dwell (the exact
   scenario where the WebView2 pill measured 0). Evidence PNGs auto-saved here.
2. **Native compile (the infra-blocked machine gate):** confirm `native_pill.rs` compiles in the
   real Windows build (it cannot be cross-checked from WSL).
3. **Visual fidelity (AC-2, NFR2 — your eye):** pill matches the old FloatingBar 1:1 across all
   states — idle (hidden), recording (teal 5-bar waveform + red stop + mode badge reflecting the
   ACTUAL hotkey mode), transcribing/cleaning (amber spinner + label), done (green check + "Done"),
   clipboard-only (amber + "In Clipboard"), error (red "Error"); ~96% dark fill; teal "K".
4. **Drag + persistence (AC-4):** drag the pill, confirm it follows, persists across restart, and
   the preview stays anchored.
5. **No regression (AC-6):** record→transcribe→cleanup→paste, open settings, check tray tooltip.

### Known minor deferrals (backlog — NOT this story's blockers)
- AC-2 visual nuances to confirm/decide at the visual gate: 1px state-colored stadium border
  (FloatingBar:501-511) is not drawn; clipboard state renders a simplified amber square, not the 📋
  glyph.
- Harness PASS asserts pixels > 0 (per the build directive), not AC-5's "≈100% of region" — a future
  refinement could also record the occluded/baseline ratio. Dead `$EvidenceDir` param default has a
  "_ bmad-output" space typo (worked around for the live path).
- Per-monitor DPI / `WM_DPICHANGED`, off-screen drag clamping, register-class-once, lock-poison vs
  not-alive nuance — beyond WebView2-bar parity; deferred.
