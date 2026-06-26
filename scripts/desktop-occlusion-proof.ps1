# desktop-occlusion-proof.ps1
# Verifies that the native Klarvo pill overlay stays fully painted when occluded
# by a foreground app. Mirrors the ADR-0021 proof harness (native-proof2.ps1).
#
# Usage: powershell.exe -ExecutionPolicy Bypass -File scripts\desktop-occlusion-proof.ps1
#
# PASS criterion: content pixels > 0 in all three samples (before, during, after dwell).
# The evidence PNGs are written to _bmad-output/implementation-artifacts/gate4-evidence/10-1/

[CmdletBinding()]
param(
    [int]$DwellSeconds = 3,
    [string]$EvidenceDir = "$PSScriptRoot\..\_ bmad-output\implementation-artifacts\gate4-evidence\10-1"
)

Add-Type -AssemblyName System.Windows.Forms,System.Drawing

# DPI-aware (required for accurate pixel coordinates at >96 DPI)
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public class DpiHelper {
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
'@
[DpiHelper]::SetProcessDPIAware() | Out-Null

function Get-PillWindow {
    $processes = Get-Process -Name "klarvo" -ErrorAction SilentlyContinue
    if (-not $processes) {
        throw "Klarvo process not found. Start Klarvo before running this harness."
    }
    # Find the native pill window by class name
    Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public class WinApi {
    [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string title);
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L,T,R,B; }
}
'@ -ErrorAction SilentlyContinue
    $hwnd = [WinApi]::FindWindow("KlarvoPillNative", $null)
    return $hwnd
}

function Capture-Region([Drawing.Rectangle]$region, [string]$outPath) {
    $bmp = New-Object Drawing.Bitmap($region.Width, $region.Height)
    $g = [Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($region.Location, [Drawing.Point]::Empty, $region.Size)
    $g.Dispose()
    $bmp.Save($outPath, [Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
}

function Count-ContentPixels([string]$imgPath) {
    $bmp = [Drawing.Bitmap]::FromFile($imgPath)
    $nonBlack = 0
    for ($y = 0; $y -lt $bmp.Height; $y++) {
        for ($x = 0; $x -lt $bmp.Width; $x++) {
            $px = $bmp.GetPixel($x, $y)
            # Count pixels that are not near-black (pill content is teal/amber/etc.)
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

$evidenceDir = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot "..\_ bmad-output\implementation-artifacts\gate4-evidence\10-1"))
$evidenceDir = $evidenceDir.Replace("_ bmad-output", "_bmad-output")
New-Item -ItemType Directory -Force -Path $evidenceDir | Out-Null

Write-Host "=== Klarvo Native Pill Occlusion Harness ===" -ForegroundColor Cyan
Write-Host "Evidence dir: $evidenceDir"
Write-Host ""

# 1. Find pill window
$hwnd = Get-PillWindow
if ($hwnd -eq [IntPtr]::Zero) {
    Write-Warning "Native pill window (KlarvoPillNative) not found."
    Write-Warning "Ensure Klarvo is running and recording is active (pill must be visible)."
    Write-Warning "Trigger a recording via the hotkey, then re-run this script."
    exit 1
}

Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;
public struct RECT2 { public int L,T,R,B; }
public class WinApi2 {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(System.IntPtr h, out RECT2 r);
}
'@ -ErrorAction SilentlyContinue
$rect = New-Object RECT2
[WinApi2]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
$pillRect = [Drawing.Rectangle]::new($rect.L, $rect.T, $rect.R - $rect.L, $rect.B - $rect.T)
Write-Host "Pill window at: $($pillRect.X),$($pillRect.Y)  size: $($pillRect.Width)x$($pillRect.Height)"

# 2. Baseline capture (pill alone)
$baselinePath = Join-Path $evidenceDir "01-baseline.png"
Capture-Region $pillRect $baselinePath
$baseline = Count-ContentPixels $baselinePath
Write-Host "Baseline content pixels: $baseline" -ForegroundColor Green

# 3. Open Notepad maximized over the pill
Write-Host ""
Write-Host "Opening Notepad maximized over pill region..." -ForegroundColor Yellow
$np = Start-Process notepad -PassThru
Start-Sleep -Milliseconds 800

Add-Type -TypeDefinition @'
using System.Runtime.InteropServices;
public class WinHelper {
    public const int SW_MAXIMIZE = 3;
    [DllImport("user32.dll")] public static extern bool ShowWindow(System.IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(System.IntPtr h);
}
'@ -ErrorAction SilentlyContinue
[WinHelper]::ShowWindow($np.MainWindowHandle, [WinHelper]::SW_MAXIMIZE) | Out-Null
[WinHelper]::SetForegroundWindow($np.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 500

# 4. Occluded capture
$occludedPath = Join-Path $evidenceDir "02-occluded.png"
Capture-Region $pillRect $occludedPath
$occluded = Count-ContentPixels $occludedPath
Write-Host "Occluded content pixels:  $occluded" -ForegroundColor $(if ($occluded -gt 0) {"Green"} else {"Red"})

# 5. Dwell
Write-Host "Waiting $DwellSeconds s dwell (the time-delayed WebView2 failure mode)..."
Start-Sleep -Seconds $DwellSeconds

# 6. Post-dwell capture
$dwellPath = Join-Path $evidenceDir "03-after-dwell.png"
Capture-Region $pillRect $dwellPath
$dwell = Count-ContentPixels $dwellPath
Write-Host "Post-dwell content pixels: $dwell" -ForegroundColor $(if ($dwell -gt 0) {"Green"} else {"Red"})

# 7. Close Notepad
Stop-Process -Id $np.Id -Force -ErrorAction SilentlyContinue

# 8. Result
Write-Host ""
$pass = ($baseline -gt 0) -and ($occluded -gt 0) -and ($dwell -gt 0)
if ($pass) {
    Write-Host "RESULT: PASS — pill painted in all 3 samples (baseline=$baseline occluded=$occluded dwell=$dwell)" -ForegroundColor Green
    Write-Host "Evidence PNGs: $evidenceDir"
    exit 0
} else {
    Write-Host "RESULT: FAIL — pixel counts: baseline=$baseline occluded=$occluded dwell=$dwell" -ForegroundColor Red
    Write-Host "Failing samples: $evidenceDir"
    exit 1
}
