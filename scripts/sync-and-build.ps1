# Klarvo: Sync from WSL and build for Windows
# Usage (from PowerShell): powershell -ExecutionPolicy Bypass -File \\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo\scripts\sync-and-build.ps1
#   -Clean    force a fresh recompile of the klarvo crate (defeats cargo incremental staleness)
#   -SkipNpm  skip the npm step entirely (faster when JS deps are unchanged)

param(
    [switch]$Clean,
    [switch]$SkipNpm
)

$src = "\\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo"
$dst = "D:\apps\klarvo"
$exe = "$dst\src-tauri\target\release\klarvo.exe"

# Fail loudly: a half-finished build must NOT masquerade as success and leave the
# previous klarvo.exe in place. That exact silent failure shipped a stale binary
# during the Story 1.2 Windows smoke on 2026-05-31 (build error swallowed by a
# `... | Write-Host` pipe, script printed "Done!", the old exe got smoke-tested).
function Fail($msg) {
    Write-Host ""
    Write-Host "BUILD ABORTED: $msg" -ForegroundColor Red
    Write-Host "klarvo.exe was NOT updated -- do not smoke-test, the previous binary is still in place." -ForegroundColor Red
    [System.Environment]::Exit(1)
}

# Ensure cargo and LLVM 18 are in PATH
$env:PATH = "C:\Program Files\LLVM\bin;C:\Users\Andi\.cargo\bin;$env:PATH"

# Record the pre-build exe timestamp so we can prove a fresh build at the end.
$exeBefore = if (Test-Path $exe) { (Get-Item $exe).LastWriteTime } else { $null }

Write-Host "Syncing files from WSL..." -ForegroundColor Cyan

# Sync with robocopy, excluding build artifacts and Android native libs.
# /PURGE deletes dest files that no longer exist in source -- WITHOUT it, a story that
# removes a file (e.g. deleting src/PreviewPanel.tsx in Story 10-2) leaves a stale orphan
# in $dst that breaks the build (or silently compiles dead code). The /XD-excluded dirs
# (target, node_modules, .git, jniLibs) are excluded from purge too, so build outputs and
# the Windows-side npm install are never touched.
# robocopy exit codes: 0-7 = success (1 = files were copied, 2 = extra/purged), 8+ = real error.
robocopy $src $dst /E /PURGE /XD target node_modules .git jniLibs /XF "*.so" /NFL /NDL /NJH /NJS /NP /R:1 /W:1
$rc = $LASTEXITCODE
if ($rc -ge 8) { Fail "robocopy sync failed (exit $rc) -- source not mirrored to $dst." }

# `npm ci`, NOT `npm install`. The JS Tauri plugins must match their Rust crate
# versions exactly (@tauri-apps/plugin-log 2.8.0 <-> tauri-plugin-log 2.8.0, and the
# same for opener/updater). `npm install` re-resolves anything the lock does not
# already satisfy and WRITES the result back -- which is how plugin-log ended up
# absent from the lock and floating to 2.9.0, dragging @tauri-apps/api up to ^2.11
# against a tree pinned at 2.10.1 and breaking this build repo-wide (found at the
# Story 11-6 GATE-4a, 2026-08-11). `npm ci` installs the lock verbatim, never
# writes it, and fails loudly on drift -- the same philosophy as Fail() above.
Set-Location $dst
if ($SkipNpm) {
    Write-Host "Skipping npm step (-SkipNpm)." -ForegroundColor Yellow
} else {
    Write-Host "Installing npm dependencies (npm ci)..." -ForegroundColor Cyan
    npm ci
    if ($LASTEXITCODE -ne 0) {
        Fail ("npm ci failed (exit $LASTEXITCODE). If it reports package.json and package-lock.json " +
              "out of sync, fix it at the SOURCE (run npm install in WSL, commit package-lock.json), " +
              "not here -- this tree is a robocopy mirror and any fix made here is overwritten next run.")
    }
}

# Load .env for API keys etc (Tauri dotenvy also picks these up from synced .env).
# IMPORTANT: skip the Tauri signing secrets. If they reach the build env, `tauri
# build` tries to sign the updater artifacts at bundle time and the built-in
# signer hangs / fails on WSL ("incorrect updater private key password"). Signing
# is deferred to rsign below, so these MUST NOT be present during the build.
if (Test-Path "$dst\.env") {
    Get-Content "$dst\.env" | ForEach-Object {
        if ($_ -match '^\s*([^#][^=]+?)\s*=\s*(.*?)\s*$') {
            $envName = $matches[1].Trim()
            if ($envName -in @('TAURI_SIGNING_PRIVATE_KEY', 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD')) {
                return  # skip: build must stay unsigned, rsign signs afterwards
            }
            [System.Environment]::SetEnvironmentVariable($envName, $matches[2], "Process")
        }
    }
    Write-Host "Loaded .env (Tauri signing keys deliberately skipped)" -ForegroundColor Yellow
}

# NOTE: Tauri's built-in signer hangs on Windows/WSL (confirmed 2026-03-21).
# Signing is done AFTER the build via WSL rsign (scripts/sign-installer.sh).
# Do NOT set TAURI_SIGNING_PRIVATE_KEY here -- it triggers the hanging signer.
#
# Belt-and-suspenders: the .env loader already skips these, but a key could also
# leak in from the parent shell. Strip both before building so the bundler never
# attempts build-time signing (the cause of the silent build failure on 2026-05-31).
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY -ErrorAction SilentlyContinue
Remove-Item Env:\TAURI_SIGNING_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue

# whisper-rs-sys: force bindgen to target Windows (not Linux)
# Without this, clang may pick up Linux headers and generate incompatible bindings.
$env:BINDGEN_EXTRA_CLANG_ARGS = "--target=x86_64-pc-windows-msvc"
Remove-Item Env:\WHISPER_DONT_GENERATE_BINDINGS -ErrorAction SilentlyContinue

# Optional: force a fresh recompile of the klarvo crate. Use when a source change
# must land in the binary and you suspect cargo incremental kept a stale object.
if ($Clean) {
    Write-Host "Forcing fresh recompile (cargo clean -p klarvo)..." -ForegroundColor Cyan
    Push-Location "$dst\src-tauri"
    cargo clean -p klarvo
    Pop-Location
}

# Kill any running Klarvo first: a live instance holds an exclusive lock on
# target\release\klarvo.exe, so the linker can't overwrite it -> the build dies
# with "failed to remove file ... (os error 5) Zugriff verweigert".
Write-Host "Stopping any running Klarvo instances..." -ForegroundColor Cyan
$running = Get-Process klarvo -ErrorAction SilentlyContinue
if ($running) {
    $running | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 800   # let the OS release the file handle
    Write-Host "  stopped $($running.Count) instance(s)." -ForegroundColor Yellow
}

Write-Host "Building Klarvo..." -ForegroundColor Cyan
# Disable Tauri's build-time updater signing for this build. tauri.conf.json has
# createUpdaterArtifacts:true + an updater pubkey, which forces Tauri to sign the
# updater artifacts at bundle time and demand TAURI_SIGNING_PRIVATE_KEY. This
# project defers signing to rsign (sign-installer.sh, below), so we override the
# flag off here. The NSIS installer is still produced; only Tauri's own .sig
# generation is skipped -- rsign creates the matching .sig afterwards.
$buildOverride = "$env:TEMP\klarvo-build-override.json"
'{"bundle":{"createUpdaterArtifacts":false}}' | Set-Content -Path $buildOverride -Encoding ascii
# Run WITHOUT piping to Write-Host: a pipe overwrites $LASTEXITCODE with Write-Host's
# value (always 0), which is exactly how a failed build slipped through before.
npx tauri build --config $buildOverride
if ($LASTEXITCODE -ne 0) { Fail "tauri build failed (exit $LASTEXITCODE) -- scroll up for the compiler error." }

# Prove the binary was actually produced by this run.
if (-not (Test-Path $exe)) { Fail "tauri build returned 0 but $exe does not exist." }
$exeAfter = (Get-Item $exe).LastWriteTime
if ($exeBefore -and $exeAfter -le $exeBefore) {
    Write-Host ""
    Write-Host "NOTE: klarvo.exe timestamp is unchanged ($exeAfter)." -ForegroundColor Yellow
    Write-Host "      Either nothing changed since the last build, or cargo reused a cached object." -ForegroundColor Yellow
    Write-Host "      If you expected a SOURCE change to land in this build, re-run with  -Clean ." -ForegroundColor Yellow
}

# --- WebView2 fixed-runtime pin (see ADR-0020) ---------------------------------
# Klarvo self-pins to a bundled WebView2 runtime in target\release\webview2-runtime
# (lib.rs run()) to dodge the 149.0.4022.69+ occlusion regression that blanks the
# always-on-top overlays. The sync above excludes `target`, so a normal build keeps
# the folder -- but a full `cargo clean` wipes it, and Evergreen eventually deletes
# the Program Files source. So we keep a master copy OUTSIDE the build tree and
# self-heal from it here. Without the runtime, Klarvo silently falls back to the
# broken Evergreen runtime -- exactly the "fix vanished" failure we are guarding.
$wv2Master = "D:\apps\klarvo-webview2-runtime"
$wv2Target = "$dst\src-tauri\target\release\webview2-runtime"
$rc62 = "C:\Windows\System32\robocopy.exe"
if (Test-Path "$wv2Target\msedgewebview2.exe") {
    if (-not (Test-Path "$wv2Master\msedgewebview2.exe")) {
        Write-Host "Seeding WebView2 runtime master copy from build tree..." -ForegroundColor Cyan
        & $rc62 $wv2Target $wv2Master /E /NFL /NDL /NJH /NJS /NC /NS /NP | Out-Null
    }
} elseif (Test-Path "$wv2Master\msedgewebview2.exe") {
    Write-Host "Restoring WebView2 fixed runtime into build tree (was missing)..." -ForegroundColor Yellow
    & $rc62 $wv2Master $wv2Target /E /NFL /NDL /NJH /NJS /NC /NS /NP | Out-Null
} else {
    Write-Host "WARNING: no WebView2 fixed runtime found (neither build tree nor master)." -ForegroundColor Red
    Write-Host "         Klarvo will use the auto-updating Evergreen runtime -> overlay-blank bug returns." -ForegroundColor Red
    Write-Host "         Fix: copy an 'EdgeWebView\Application\<=149.0.4022.62' folder to $wv2Master (see ADR-0020)." -ForegroundColor Red
}

# Sign the installer via WSL rsign (Tauri's signer hangs).
# Non-fatal: the raw klarvo.exe you smoke-test does not depend on signing.
Write-Host "Signing installer via WSL rsign..." -ForegroundColor Cyan
wsl bash ~/workspace/products/klarvo/scripts/sign-installer.sh
if ($LASTEXITCODE -ne 0) {
    Write-Host "WARNING: signing failed (exit $LASTEXITCODE) -- installer is unsigned, but the built klarvo.exe is fine for local smoke." -ForegroundColor Yellow
}

# Copy installer to Dropbox for easy access
$version = (Get-Content "$dst\package.json" | ConvertFrom-Json).version
$dropboxDir = "D:\Dropbox\App Development\klarvo\releases\v$version"
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

Write-Host ""
Write-Host "Done! Fresh build verified." -ForegroundColor Green
Write-Host "  klarvo.exe : $exe" -ForegroundColor Green
Write-Host "  built at   : $exeAfter" -ForegroundColor Green
Write-Host "  size       : $([math]::Round((Get-Item $exe).Length / 1MB, 1)) MB" -ForegroundColor Green
[System.Environment]::Exit(0)
