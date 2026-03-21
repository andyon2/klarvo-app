# Voxlit: Sync from WSL and build for Windows
# Usage (from PowerShell): powershell -ExecutionPolicy Bypass -File \\wsl$\Ubuntu\home\andyon2\claude-projects\voxlit\scripts\sync-and-build.ps1

$src = "\\wsl$\Ubuntu\home\andyon2\claude-projects\voxlit"
$dst = "D:\apps\voxlit"

# Ensure cargo and LLVM 18 are in PATH
$env:PATH = "C:\Program Files\LLVM\bin;C:\Users\Andi\.cargo\bin;$env:PATH"

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

# NOTE: Tauri's built-in signer hangs on Windows/WSL (confirmed 2026-03-21).
# Signing is done AFTER the build via WSL rsign (scripts/sign-installer.sh).
# Do NOT set TAURI_SIGNING_PRIVATE_KEY here — it triggers the hanging signer.

# whisper-rs-sys: force bindgen to target Windows (not Linux)
# Without this, clang may pick up Linux headers and generate incompatible bindings.
$env:BINDGEN_EXTRA_CLANG_ARGS = "--target=x86_64-pc-windows-msvc"
Remove-Item Env:\WHISPER_DONT_GENERATE_BINDINGS -ErrorAction SilentlyContinue

Write-Host "Building Voxlit..." -ForegroundColor Cyan
npx tauri build 2>&1 | Write-Host

# Sign the installer via WSL rsign (Tauri's signer hangs)
Write-Host "Signing installer via WSL rsign..." -ForegroundColor Cyan
wsl bash ~/claude-projects/voxlit/scripts/sign-installer.sh

# Copy installer to Dropbox for easy access
$version = (Get-Content "$dst\package.json" | ConvertFrom-Json).version
$dropboxDir = "D:\Dropbox\App Development\voxlit\releases\v$version"
$nsisDir = "$dst\src-tauri\target\release\bundle\nsis"
$installer = Get-ChildItem "$nsisDir\*.exe" -Exclude "*.exe.sig" | Select-Object -First 1

if ($installer) {
    New-Item -ItemType Directory -Force -Path $dropboxDir | Out-Null
    Copy-Item "$($installer.FullName)" "$dropboxDir\"
    Copy-Item "$($installer.FullName).sig" "$dropboxDir\" -ErrorAction SilentlyContinue
    Write-Host "Installer copied to $dropboxDir\" -ForegroundColor Green
} else {
    Write-Host "WARNING: No installer found to copy" -ForegroundColor Yellow
}

Write-Host "Done!" -ForegroundColor Green
[System.Environment]::Exit(0)
