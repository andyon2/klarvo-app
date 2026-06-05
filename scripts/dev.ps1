# Klarvo DEV loop: sync from WSL once, then run `tauri dev` (DEBUG build + Vite HMR).
#
# Usage (from PowerShell):
#   powershell -ExecutionPolicy Bypass -File \\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo\scripts\dev.ps1
#   -SkipNpm   skip `npm install` (faster start when deps are unchanged)
#
# Run this ONCE and leave it running. After each edit Claude makes in WSL, run
# scripts/sync.ps1 (fast robocopy only) in a SECOND terminal — Vite HMR reloads
# frontend changes instantly; `tauri dev` recompiles Rust incrementally (debug,
# seconds — not the multi-minute release build). The slow release build via
# sync-and-build.ps1 is then needed ONLY for the final smoke, not every iteration.
#
# PREREQUISITE: quit the installed release Klarvo first (tray -> Quit). Otherwise
# the dev instance cannot register the global hotkey ("HotKey already registered").
#
# The dev app shares identifier com.klarvo.voice, so it uses the SAME
# %APPDATA%\com.klarvo.voice\config.json — your real license, API keys and bar
# position carry over. Webview console output lands in Klarvo.log via the console
# bridge (readable from WSL).

param(
    [switch]$SkipNpm
)

$src = "\\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo"
$dst = "D:\apps\klarvo"

function Fail($msg) {
    Write-Host ""
    Write-Host "DEV ABORTED: $msg" -ForegroundColor Red
    [System.Environment]::Exit(1)
}

# cargo + LLVM 18 (whisper-rs bindgen) in PATH; target Windows headers.
$env:PATH = "C:\Program Files\LLVM\bin;C:\Users\Andi\.cargo\bin;$env:PATH"
$env:BINDGEN_EXTRA_CLANG_ARGS = "--target=x86_64-pc-windows-msvc"
Remove-Item Env:\WHISPER_DONT_GENERATE_BINDINGS -ErrorAction SilentlyContinue

Write-Host "Syncing files from WSL..." -ForegroundColor Cyan
robocopy $src $dst /E /XD target node_modules .git jniLibs /XF "*.so" /NFL /NDL /NJH /NJS /NP /R:1 /W:1
if ($LASTEXITCODE -ge 8) { Fail "robocopy sync failed (exit $LASTEXITCODE)." }

Set-Location $dst

if (-not $SkipNpm) {
    Write-Host "Installing npm dependencies..." -ForegroundColor Cyan
    npm install
    if ($LASTEXITCODE -ne 0) { Fail "npm install failed (exit $LASTEXITCODE)." }
}

Write-Host ""
Write-Host "Starting `tauri dev` (debug build + Vite HMR). Leave this terminal running." -ForegroundColor Green
Write-Host "After each Claude edit, run scripts/sync.ps1 in another terminal." -ForegroundColor Green
Write-Host ""
npm run tauri dev
