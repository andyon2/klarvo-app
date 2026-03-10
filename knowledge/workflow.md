# Workflow & Konventionen

Wie Andy arbeitet, wie wir zusammenarbeiten, und Lektionen aus vergangenen Sessions.
Diese Datei wird bei jedem Sessionstart gelesen und waechst organisch.

## Andys Entwicklungsumgebung

- Windows-Laptop mit NVIDIA GPU
- GPU nur am Strom, CPU auf Akku
- WSL2 fuer CLI/Git, PowerShell fuer Builds
- Projektpfad Windows: `D:\Apps\dikta\`
- Projektpfad WSL: `~/claude-projects/dikta/`

## Build & Test -- Die 3 Wege

1. **`cargo test`** — Automatisierte Unit-Tests (aktuell 239). Laeuft in WSL.
2. **`tauri dev`** — Dev-Modus mit Hot-Reload. Fuer schnelles Frontend-Testen.
3. **`dikta.exe` direkt** — Andys primaerer Test-Weg. `sync-and-build.ps1` in PowerShell ausfuehren, dann `D:\Apps\dikta\src-tauri\target\release\dikta.exe` starten. Kein Installer noetig. Das ist die fertige App wie sie beim Nutzer laeuft.

**Wichtig:** Andy nutzt fast immer Weg 3. Wenn er fragt "kann ich testen?", meint er: Ist ein frischer Build moeglich? Die Antwort ist immer `sync-and-build.ps1` auf Windows, dann `dikta.exe` starten.

**Vor dem Build:** Immer zuerst `taskkill.exe /IM dikta.exe /F` ausfuehren (geht aus WSL). Sonst schlaegt der Build fehl mit "Zugriff verweigert" weil die .exe noch laeuft. Der Befehl ist harmlos wenn die App nicht laeuft (gibt nur "nicht gefunden" Fehler).

## Lektionen (was schon mal schiefging)

- **2026-03-10:** Mehrfach nicht gewusst, dass Andy ueber `dikta.exe` testet statt ueber `tauri dev` oder Installer. Fuehrte zu falschen Anweisungen ("du musst auf Windows bauen und installieren"). Merke: Die nackte .exe im Release-Ordner ist der Standard-Testweg.
