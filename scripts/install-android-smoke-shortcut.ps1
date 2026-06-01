# install-android-smoke-shortcut.ps1
# Rechtsklick -> "Mit PowerShell ausfuehren"
# ODER PowerShell oeffnen und .\install-android-smoke-shortcut.ps1 tippen.

$ErrorActionPreference = "Stop"

try {
    $wslScriptPath = "/home/andyon2/workspace/products/klarvo/scripts/android-smoke.sh"
    $cmdExe        = "$env:SystemRoot\System32\cmd.exe"
    # /c runs the command; & pause keeps the window open after script finishes or fails
    $arguments     = "/c `"wsl.exe bash $wslScriptPath & pause`""

    $shortcutPath  = Join-Path ([Environment]::GetFolderPath("Desktop")) "Klarvo Android Smoke.lnk"

    $shell         = New-Object -ComObject WScript.Shell
    $shortcut      = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath       = $cmdExe
    $shortcut.Arguments        = $arguments
    $shortcut.WorkingDirectory = $env:USERPROFILE
    $shortcut.Description      = "Klarvo Android Smoke Build + adb Install"
    $shortcut.Save()

    Write-Host ""
    Write-Host "OK: Verknuepfung erstellt: $shortcutPath" -ForegroundColor Green
    Write-Host "    Ziel:      $wslExe"
    Write-Host "    Argumente: $arguments"
    Write-Host ""
    Write-Host "Einmalig adb-Verbindung (WiFi):"
    Write-Host "  adb pair IP:PAIR-PORT   (Pin aus Entwickleroptionen)"
    Write-Host "  adb connect IP:5555"
    Write-Host ""
} catch {
    Write-Host ""
    Write-Host "FEHLER: $_" -ForegroundColor Red
    Write-Host ""
}

Read-Host "Enter druecken zum Schliessen"
