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

**Builds selbst ausfuehren, nie Andy fragen.** Der Windows-Build geht aus WSL:
```bash
powershell.exe -Command "Get-Process dikta -ErrorAction SilentlyContinue | Stop-Process -Force" 2>/dev/null
powershell.exe -ExecutionPolicy Bypass -File '\\wsl$\Ubuntu\home\andyon2\claude-projects\dikta\scripts\sync-and-build.ps1' 2>&1
```
Andy nur einbeziehen wenn etwas interaktive Eingabe auf Windows erfordert.

## Lektionen (was schon mal schiefging)

- **2026-03-10:** Mehrfach nicht gewusst, dass Andy ueber `dikta.exe` testet statt ueber `tauri dev` oder Installer. Fuehrte zu falschen Anweisungen ("du musst auf Windows bauen und installieren"). Merke: Die nackte .exe im Release-Ordner ist der Standard-Testweg.
- **2026-03-11:** Release-Notes nur mit Aenderungen der aktuellen Session erstellt, statt den vollen Changelog seit dem letzten oeffentlichen Release zu pruefen. Merke: Immer `git log v<LETZTE_VERSION>..HEAD` nutzen, um ALLE Aenderungen zu sammeln. Im Release-Skill unter Schritt 8 dokumentiert.
- **2026-03-11:** Dev-Tooling (UI Preview Mode) in Release-Notes aufgenommen. Gehoert nicht rein — Nutzer interessiert nur, was sich fuer sie aendert. Filter-Regel im Release-Skill ergaenzt.
- **2026-03-12:** Bei erstem GitHub-Issue sofort Explore-Agent losgeschickt ohne Andy zu fragen. Zu viel Eigeninitiative. Merke: Bei neuen Issues/Feedback nur zusammenfassen und Andy fragen was passieren soll. Keine eigenmaechtigen Untersuchungen oder Agent-Delegationen.
- **2026-03-15:** Wiederholt Andy gebeten den Windows-Build zu starten, obwohl das aus WSL geht (`powershell.exe -ExecutionPolicy Bypass -File ...`). Die Build- und Release-Skills dokumentieren den Befehl bereits. Merke: Builds immer selbst ausfuehren. Andy nur einbeziehen wenn etwas interaktive Eingabe auf Windows erfordert.
- **2026-03-18:** Agents vergessen bei Trait-Signatur-Aenderungen alle Konsumenten anzupassen (Regel 8). Passierte 2x: `paste_snippet` nach PasteResult-Umbau, `IsWindow(Option<HWND>)` nur auf Windows sichtbar. Merke: Nach Agent-Output immer grep nach geaenderten Signaturen/Typen. Besonders bei `#[cfg]`-Code der auf Linux nicht kompiliert wird.
- **2026-03-18:** Return-to-Current Bug: `capture_foreground_window()` wurde NACH `paste()` aufgerufen — da hatte Focus schon gewechselt. Timing bei Focus-Capture ist kritisch: immer VOR der Operation die den Focus aendert.
