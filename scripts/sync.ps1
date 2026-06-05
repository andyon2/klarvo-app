# Klarvo FAST sync: mirror WSL -> D: with robocopy, NO build.
#
# Usage (from PowerShell):
#   powershell -ExecutionPolicy Bypass -File \\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo\scripts\sync.ps1
#
# Run this after each edit Claude makes in WSL, while scripts/dev.ps1 (`tauri dev`)
# is running in another terminal. Vite HMR picks up frontend (src/) changes and
# hot-reloads instantly; `tauri dev` recompiles Rust (src-tauri/) incrementally.
# Takes a couple of seconds — no release rebuild needed for iteration.

$src = "\\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo"
$dst = "D:\apps\klarvo"

robocopy $src $dst /E /XD target node_modules .git jniLibs /XF "*.so" /NFL /NDL /NJH /NJS /NP /R:1 /W:1
if ($LASTEXITCODE -ge 8) {
    Write-Host "robocopy sync failed (exit $LASTEXITCODE)." -ForegroundColor Red
    exit 1
}
Write-Host "Synced WSL -> $dst. Vite HMR / tauri dev will pick up the change." -ForegroundColor Green
