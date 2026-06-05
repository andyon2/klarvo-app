# Fast dev loop for surface work (bar / preview / UI)

Surface bugs (Tauri runtime, WebView2, multi-window geometry, click-through) are
invisible to Linux `cargo test` + `tsc` — they only show on a real Windows run.
Historically that meant: Claude edits in WSL → full **release** rebuild on Windows
(minutes) → manual visual check → repeat. Two fixes cut that loop down.

## 1. `tauri dev` instead of release rebuilds

Debug build + Vite HMR. Frontend changes hot-reload instantly; Rust changes
recompile incrementally (seconds). Release build (`sync-and-build.ps1`) is then
only needed for the **final** smoke.

**Setup (Windows PowerShell):**

1. Quit the installed release Klarvo (tray → Quit) — otherwise the dev instance
   can't register the global hotkey.
2. Start the dev app (run once, leave running):
   ```
   powershell -ExecutionPolicy Bypass -File \\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo\scripts\dev.ps1
   ```
3. After each edit Claude makes in WSL, in a **second** terminal:
   ```
   powershell -ExecutionPolicy Bypass -File \\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo\scripts\sync.ps1
   ```
   Vite HMR reloads frontend (`src/`) changes; `tauri dev` recompiles Rust
   (`src-tauri/`) incrementally.

The dev app shares identifier `com.klarvo.voice`, so it uses the same
`%APPDATA%\com.klarvo.voice\config.json` — real license, API keys, bar position
all carry over. Dev mode also has webview devtools (right-click → Inspect on the
main window).

## 2. Console-to-log bridge (read webview output from WSL)

The `"bar"` and `"preview"` overlay windows are transparent / click-through /
tiny, so their devtools can't be opened — their `console.*` was invisible. The
bridge (`src/console-bridge.ts`, installed per-window in `main.tsx`) forwards
`console.log/info/warn/error` to the `frontend_log` Tauri command, which writes
them to the Rust log tagged `[fe:<label>]`.

**Claude reads that log from WSL:**
```
/mnt/c/Users/Andi/AppData/Local/com.klarvo.voice/logs/Klarvo.log
```

So after a Windows run, Claude can `grep '\[fe:preview\]'` the log and see exactly
what the preview window's JS did — turning *guess → rebuild → look* into
*read log → targeted fix*. Local-only (on-disk log, no network — BYOK/no-telemetry).

The preview path emits a lifecycle trace: `PreviewPanel mounted` → `state-changed: <s>`
→ `first chunk received → running show sequence` → `shown: WxH at (x,y)`. A missing
link pinpoints the break (e.g. no `mounted` line ⇒ the webview never initialized).
