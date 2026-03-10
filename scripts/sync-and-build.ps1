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

# Load .env for API keys etc (Tauri dotenvy also picks these up from synced .env)
if (Test-Path "$dst\.env") {
    Get-Content "$dst\.env" | ForEach-Object {
        if ($_ -match '^\s*([^#][^=]+?)\s*=\s*(.*?)\s*$') {
            [System.Environment]::SetEnvironmentVariable($matches[1], $matches[2], "Process")
        }
    }
    Write-Host "Loaded .env" -ForegroundColor Yellow
}

# Signing key: read directly from WSL key file to avoid encoding issues with .env
$wslKeyFile = "\\wsl$\Ubuntu\home\andyon2\.tauri\dikta.key"
if (Test-Path $wslKeyFile) {
    $keyContent = (Get-Content $wslKeyFile -Raw).Trim()
    [System.Environment]::SetEnvironmentVariable("TAURI_SIGNING_PRIVATE_KEY", $keyContent, "Process")
    Write-Host "Loaded signing key from $wslKeyFile" -ForegroundColor Yellow
} else {
    Write-Host "WARNING: Signing key not found at $wslKeyFile" -ForegroundColor Red
}

# whisper-rs-sys: force bindgen to target Windows (not Linux)
# Without this, clang may pick up Linux headers and generate incompatible bindings.
$env:BINDGEN_EXTRA_CLANG_ARGS = "--target=x86_64-pc-windows-msvc"
Remove-Item Env:\WHISPER_DONT_GENERATE_BINDINGS -ErrorAction SilentlyContinue

Write-Host "Building Dikta..." -ForegroundColor Cyan
npx tauri build

Write-Host "Done!" -ForegroundColor Green
