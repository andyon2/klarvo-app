# install-android-smoke-shortcut.ps1
# Legt "Klarvo Android Smoke" als Desktop-Verknüpfung an.
# Startet android-smoke.sh im WSL-Terminal und hält das Fenster offen.
#
# Ausführen (einmalig, als normaler User — kein Admin nötig):
#   powershell -ExecutionPolicy Bypass -File \\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo\scripts\install-android-smoke-shortcut.ps1

$ErrorActionPreference = "Stop"

$wslScript = "/home/andyon2/workspace/products/klarvo/scripts/android-smoke.sh"

# wsl.exe: bash ausführen, Fenster nach Abschluss offen halten (wie bei Rebuild-Shortcut)
$target    = "wsl.exe"
$arguments = "bash -c `"$wslScript; echo ''; read -n1 -p 'Drücke eine Taste zum Schließen...'; echo`""

$shortcutPath = Join-Path ([Environment]::GetFolderPath("Desktop")) "Klarvo Android Smoke.lnk"

$shell    = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath       = $target
$shortcut.Arguments        = $arguments
$shortcut.WorkingDirectory = "\\wsl$\Ubuntu\home\andyon2\workspace\products\klarvo"
$shortcut.Description      = "Klarvo Android Smoke Build + adb Install"
$shortcut.Save()

Write-Host ""
Write-Host "Verknüpfung angelegt: $shortcutPath" -ForegroundColor Green
Write-Host ""
Write-Host "Was das Script macht:" -ForegroundColor Cyan
Write-Host "  1. Kotlin-Quellen → gen/android/ sync"
Write-Host "  2. JVM-Unit-Tests (Logik-Gate, schlägt fehl wenn Tests rot)"
Write-Host "  3. Gradle Debug-APK (~2-3 Min, Rust aus Cache)"
Write-Host "  4. adb install -r direkt aufs Gerät"
Write-Host "  5. versionName-Verifikation auf dem Gerät (AI-1 Freshness-Gate)"
Write-Host ""
Write-Host "Einmalig: adb per WiFi aus WSL2 einrichten" -ForegroundColor Yellow
Write-Host "  Drahtloses Debugging im Handy aktivieren (Entwickleroptionen)"
Write-Host "  Dann im WSL-Terminal:"
Write-Host "    ~/workspace/tools/android-sdk/platform-tools/adb pair <ip>:<port>"
Write-Host "    ~/workspace/tools/android-sdk/platform-tools/adb connect <ip>:5555"
Write-Host "  Danach funktioniert der Shortcut ohne USB-Kabel."
