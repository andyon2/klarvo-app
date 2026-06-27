# Story 10-1 — Win32 Surface Compile Gate

**Date:** 2026-06-27  
**Target:** `x86_64-pc-windows-gnu`  
**Result:** ✅ 0 errors (30 warnings, all pre-existing `#[must_use]` BOOL/Result patterns)

---

## Why a scratch harness?

The real crate includes `whisper-rs-sys` and `ort-sys` which have C++ build-system deps
unavailable in WSL. These block `cargo check --target x86_64-pc-windows-gnu` on the
whole workspace. The harness isolates `native_pill.rs` + `windows = "0.61"` so the
Win32 API surface can be type-checked without triggering C++ compilation.

---

## Harness recipe

**Location (ephemeral, scratchpad only — never committed):**
```
/tmp/claude-1000/.../scratchpad/win32-check/
├── Cargo.toml
└── src/
    ├── lib.rs          # shim types for tauri::AppHandle, crate::hotkey::*, crate::AppState
    └── native_pill.rs  # copy of src-tauri/src/native_pill.rs, two harness-only patches:
                        #   - remove #![cfg(target_os="windows")] at top
                        #   - use crate::fake_tauri::{...} instead of use tauri::{...}
```

**Cargo.toml deps** (exact same features as `src-tauri/Cargo.toml` windows block):
```toml
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = [
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_Graphics_Gdi",
    "Win32_System_Registry",
    "Win32_Security",
    "Win32_System_Threading",
    "Win32_System_LibraryLoader",
    "Win32_Foundation",
] }
tiny-skia = "0.11"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
log = "0.4"
```

**Shim approach:** `src/lib.rs` defines `pub mod fake_tauri` (with `AppHandle`, `Emitter`, `Manager`
as zero-cost stubs), `pub mod hotkey` (with real `PipelineState`/`PipelineEvent` shapes),
`pub struct AppState` (with `recorder.is_recording()`, `recorder.stop_recording()`,
`recording_start`, `save_config_locked()`), and `pub fn emit_pipeline_state`.
The harness copy of `native_pill.rs` changes only two lines at the top; all Win32 calls are
type-checked for real against `windows 0.61.3`.

**Check command:**
```bash
cd /tmp/claude-1000/.../scratchpad/win32-check
cargo check --target x86_64-pc-windows-gnu
```

---

## Fix categories applied (all in `src-tauri/src/native_pill.rs`)

| Category | Count | Change |
|---|---|---|
| `PostMessageW` Some-wrap | 4 | `HWND(x)` → `Some(HWND(x))` |
| `SetTimer`/`KillTimer` Some-wrap | 3 | `hwnd` → `Some(hwnd)` in all timer calls |
| `SetWindowPos` Some-wrap | 1 | `HWND_TOPMOST` → `Some(HWND_TOPMOST)` |
| `GetDeviceCaps` Some-wrap | 1 | `screen_dc` → `Some(screen_dc)` |
| `DeleteObject` `.into()` (HFONT→HGDIOBJ) | 4 | font_k/label/mode/label_lg |
| `DeleteObject` `.into()` (HBITMAP→HGDIOBJ) | 4 | main_bmp/tmp_bmp × 2 error paths + WM_DESTROY |
| `SelectObject` `.into()` | 6 | bmp (HBITMAP) + all font selects (HFONT) |
| `TextOutW` signature (5→4 args, &[u16]) | 6 | drop PCWSTR wrapper + count; pass `&text[..n]` |
| `CreateFontW` typed params | 4 | `.0 as u32` → pass enum directly (FONT_CHARSET etc.) |
| Missing imports | 2 | `use windows::core::{BOOL, PCWSTR}` + `SetCapture`/`ReleaseCapture` from `KeyboardAndMouse` |

**Total distinct error sites fixed: 35 (was 49 errors, collapsed by category).**

No Cargo.toml feature changes were required — all needed features were already enabled.

---

## Control-flow rewrite note

**`BOOL(1)` match arm** (line ~1287): `GetMessageW` returns `windows_core::BOOL`.
`BOOL` was not in scope via the `Foundation::*` glob (it lives in `windows::core`).
Adding `use windows::core::BOOL` brings it into scope; the match `BOOL(1) => { TranslateMessage; DispatchMessageW }` / `_ => break` compiles unchanged.
Semantically unchanged: positive return = message received/processed, 0/-1 = quit/error → break.

**`ReleaseCapture()` return**: now `Result<()>` (was void). Left without `let _ =` — Rust emits
a `#[must_use]` warning (included in the 30 warnings). Control-flow intent preserved exactly.

---

## Reuse for Story 10-2

Run the same `cargo check --target x86_64-pc-windows-gnu` command in the harness after editing
`native_pill.rs`. Sync the two harness-only patches (`#![cfg...]` removal + `use crate::fake_tauri`)
each time you re-copy the file.
