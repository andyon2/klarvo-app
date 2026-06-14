# install-adb-pin-shortcut.ps1
# Erstellt den Desktop-Shortcut "Klarvo ADB Pin".
# Rechtsklick -> "Mit PowerShell ausfuehren"
# ODER PowerShell oeffnen und .\install-adb-pin-shortcut.ps1 tippen.
#
# Der Shortcut startet scripts/adb-pin.sh im interaktiven Modus: das Fenster
# fragt nur nach dem Port (aus 'Drahtloses Debugging' am Handy; die stabile
# Tailscale-IP setzt das Script automatisch davor) und pinnt das Gerät auf den
# Festport 5555, sodass "Klarvo Android Smoke" danach per Tailscale auto-connectet.

$ErrorActionPreference = "Stop"

try {
    $wslScriptPath = "/home/andyon2/workspace/products/klarvo/scripts/adb-pin.sh"
    $cmdExe        = "$env:SystemRoot\System32\cmd.exe"
    # /c runs the command; & pause keeps the window open after the script finishes.
    # Kein Argument -> adb-pin.sh fragt IP:Port interaktiv im Fenster ab.
    $arguments     = "/c `"wsl.exe bash $wslScriptPath & pause`""

    $shortcutPath  = Join-Path ([Environment]::GetFolderPath("Desktop")) "Klarvo ADB Pin.lnk"

    $shell         = New-Object -ComObject WScript.Shell
    $shortcut      = $shell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath       = $cmdExe
    $shortcut.Arguments        = $arguments
    $shortcut.WorkingDirectory = $env:USERPROFILE
    $shortcut.Description      = "Handy nach Neustart auf adb-Festport 5555 pinnen (nur Port eingeben)"
    $shortcut.Save()

    Write-Host ""
    Write-Host "OK: Verknuepfung erstellt: $shortcutPath" -ForegroundColor Green
    Write-Host "    Ziel:      $cmdExe"
    Write-Host "    Argumente: $arguments"
    Write-Host ""
    Write-Host "Benutzung nach einem Handy-Neustart:" -ForegroundColor Cyan
    Write-Host "  1. Handy: Entwickleroptionen -> 'Drahtloses Debugging' einschalten"
    Write-Host "  2. Doppelklick 'Klarvo ADB Pin' -> nur den Port vom Screen eingeben"
    Write-Host "  3. Danach 'Klarvo Android Smoke' wie gewohnt starten"
    Write-Host ""
} catch {
    Write-Host ""
    Write-Host "FEHLER: $_" -ForegroundColor Red
    Write-Host ""
}

Read-Host "Enter druecken zum Schliessen"
