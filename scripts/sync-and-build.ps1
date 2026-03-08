# Dikta: Sync from WSL and build for Windows
# Usage (from PowerShell): powershell -ExecutionPolicy Bypass -File \\wsl$\Ubuntu\home\andyon2\dikta\scripts\sync-and-build.ps1

$src = "\\wsl$\Ubuntu\home\andyon2\dikta"
$dst = "D:\apps\dikta"

# Ensure cargo is in PATH
$env:PATH = "C:\Users\Andi\.cargo\bin;$env:PATH"

Write-Host "Syncing files from WSL..." -ForegroundColor Cyan

# Sync with robocopy, excluding build artifacts and Android native libs
robocopy $src $dst /E /XD target node_modules .git jniLibs /XF "*.so" /NFL /NDL /NJH /NJS /NP /R:1 /W:1

Write-Host "Installing npm dependencies..." -ForegroundColor Cyan
Set-Location $dst
npm install

Write-Host "Building Dikta..." -ForegroundColor Cyan
npx tauri build

Write-Host "Done!" -ForegroundColor Green
