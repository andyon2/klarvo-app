# preview-occlusion-proof.ps1
# Verifies that the native Klarvo preview overlay stays fully painted when occluded
# by a foreground app. Mirrors the pill occlusion harness (desktop-occlusion-proof.ps1)
# but targets the KlarvoPreviewNative window class (Story 10-2).
#
# Usage: powershell.exe -ExecutionPolicy Bypass -File scripts\preview-occlusion-proof.ps1
#
# PASS criterion: content pixels > 0 in all three samples (before, during, after dwell).
# The evidence PNGs are written to _bmad-output/implementation-artifacts/gate4-evidence/10-2/
#
# PRE-CONDITIONS:
#   1. Klarvo is running.
#   2. A recording is active AND live_preview_enabled = true so the preview window is visible.
#   3. At least one preview chunk has been received (some text is showing).

[CmdletBinding()]
param(
    [int]$DwellSeconds = 3,
    [string]$EvidenceDir = "$PSScriptRoot\..\_ bmad-output\implementation-artifacts\gate4-evidence\10-2"
)

Add-Type -AssemblyName System.Windows.Forms,System.Drawing

# DPI-aware (required for accurate pixel coordinates at >96 DPI)
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class DpiHelper10_2 {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[DpiHelper10_2]::SetProcessDPIAware() | Out-Null

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public struct RECT_10_2 { public int L, T, R, B; }
public class WinApi10_2 {
    [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT_10_2 r);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
}
'@ -ErrorAction SilentlyContinue

function Get-PreviewWindow {
    $processes = Get-Process -Name "klarvo" -ErrorAction SilentlyContinue
    if (-not $processes) {
        throw "Klarvo process not found. Start Klarvo before running this harness."
    }
    return [WinApi10_2]::FindWindow("KlarvoPreviewNative", $null)
}

function Capture-Region([System.Drawing.Rectangle]$region, [string]$outPath) {
    $bmp = New-Object System.Drawing.Bitmap($region.Width, $region.Height)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($region.Location, [System.Drawing.Point]::Empty, $region.Size)
    $g.Dispose()
    $bmp.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function Count-ContentPixels([string]$imgPath) {
    $bmp = [System.Drawing.Bitmap]::FromFile($imgPath)
    $nonBlack = 0
    for ($y = 0; $y -lt $bmp.Height; $y++) {
        for ($x = 0; $x -lt $bmp.Width; $x++) {
            $px = $bmp.GetPixel($x, $y)
            # Count pixels that are not near-black (card bg, text, border are all > 20)
            if (($px.R -gt 20) -or ($px.G -gt 20) -or ($px.B -gt 20)) {
                $nonBlack++
            }
        }
    }
    $bmp.Dispose()
    return $nonBlack
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

$evidenceDir = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\_ bmad-output\implementation-artifacts\gate4-evidence\10-2"))
$evidenceDir = $evidenceDir.Replace("_ bmad-output", "_bmad-output")
New-Item -ItemType Directory -Force -Path $evidenceDir | Out-Null

Write-Host "=== Klarvo Native Preview Occlusion Harness (Story 10-2) ===" -ForegroundColor Cyan
Write-Host "Window class: KlarvoPreviewNative"
Write-Host "Evidence dir: $evidenceDir"
Write-Host ""

# 1. Find preview window
$hwnd = Get-PreviewWindow
if ($hwnd -eq [IntPtr]::Zero) {
    Write-Warning "Native preview window (KlarvoPreviewNative) not found."
    Write-Warning "Ensure Klarvo is running, live_preview_enabled=true, and a recording"
    Write-Warning "is active with at least one preview chunk received (text visible)."
    exit 1
}

$rect = New-Object RECT_10_2
[WinApi10_2]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
$previewRect = [System.Drawing.Rectangle]::new($rect.L, $rect.T, $rect.R - $rect.L, $rect.B - $rect.T)
Write-Host "Preview window at: $($previewRect.X),$($previewRect.Y)  size: $($previewRect.Width)x$($previewRect.Height)"

if ($previewRect.Width -lt 10 -or $previewRect.Height -lt 10) {
    Write-Warning "Preview window is too small — it may be hidden. Trigger a recording first."
    exit 1
}

# 2. Baseline capture (preview alone)
$baselinePath = Join-Path $evidenceDir "01-baseline.png"
Capture-Region $previewRect $baselinePath
$baseline = Count-ContentPixels $baselinePath
Write-Host "Baseline content pixels: $baseline" -ForegroundColor Green

# 3. Open Notepad maximized over the preview
Write-Host ""
Write-Host "Opening Notepad maximized (occludes preview)..." -ForegroundColor Yellow
$np = Start-Process notepad -PassThru
Start-Sleep -Milliseconds 800

Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;
public class WinHelper10_2 {
    public const int SW_MAXIMIZE = 3;
    [DllImport("user32.dll")] public static extern bool ShowWindow(System.IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);
}
'@ -ErrorAction SilentlyContinue
[WinHelper10_2]::ShowWindow($np.MainWindowHandle, [WinHelper10_2]::SW_MAXIMIZE) | Out-Null
[WinHelper10_2]::SetForegroundWindow($np.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 500

# 4. Occluded capture (Notepad covering preview)
$occludedPath = Join-Path $evidenceDir "02-occluded.png"
Capture-Region $previewRect $occludedPath
$occluded = Count-ContentPixels $occludedPath
Write-Host "Occluded content pixels:  $occluded" -ForegroundColor $(if ($occluded -gt 0) {"Green"} else {"Red"})

# 5. Dwell (time-delayed failure mode that WebView2 exhibited)
Write-Host "Waiting $DwellSeconds s dwell (the time-delayed WebView2 occlusion failure mode)..."
Start-Sleep -Seconds $DwellSeconds

# 6. Post-dwell capture
$dwellPath = Join-Path $evidenceDir "03-after-dwell.png"
Capture-Region $previewRect $dwellPath
$dwell = Count-ContentPixels $dwellPath
Write-Host "Post-dwell content pixels: $dwell" -ForegroundColor $(if ($dwell -gt 0) {"Green"} else {"Red"})

# 7. Close Notepad
Stop-Process -Id $np.Id -Force -ErrorAction SilentlyContinue

# 8. Result
Write-Host ""
$pass = ($baseline -gt 0) -and ($occluded -gt 0) -and ($dwell -gt 0)
if ($pass) {
    Write-Host "RESULT: PASS — preview painted in all 3 samples" -ForegroundColor Green
    Write-Host "  baseline=$baseline  occluded=$occluded  dwell=$dwell" -ForegroundColor Green
    Write-Host "Evidence PNGs: $evidenceDir"
    exit 0
} else {
    Write-Host "RESULT: FAIL — pixel counts: baseline=$baseline occluded=$occluded dwell=$dwell" -ForegroundColor Red
    Write-Host "Failing samples: $evidenceDir"
    exit 1
}
