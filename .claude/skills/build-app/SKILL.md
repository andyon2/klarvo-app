---
name: build-app
description: Baut Voxlit fuer eine Zielplattform (Windows oder Android). Meldet Fehler strukturiert. Aufrufen mit Plattform, z.B. "/build windows" oder "/build android".
argument-hint: "[plattform] -- windows | android | check"
allowed-tools: Read, Bash, Glob
context: fork
model: sonnet
---

Baue Voxlit fuer die angegebene Zielplattform.

## Argumente

`$ARGUMENTS` enthaelt die Plattform: `windows` | `android` | `check` (nur Kompilier-Check ohne Build).

WICHTIG: Frage NICHT nochmal nach der Plattform. Sie steht bereits in `$ARGUMENTS`. Extrahiere sie und fuehre den Build SOFORT aus. Kein Dialog, keine Rueckfragen.

## Vorgehensweise

### Voraussetzungen pruefen

1. Pruefe ob `src-tauri/Cargo.toml` existiert
2. Pruefe ob `package.json` existiert
3. Pruefe ob `node_modules/` existiert, sonst: `npm install`

### Wenn plattform = `check`

```bash
cd /home/andyon2/claude-projects/voxlit && cargo check --manifest-path src-tauri/Cargo.toml 2>&1
```

Falls Fehler: Parse die Compiler-Ausgabe und melde strukturiert:
```
BUILD CHECK FEHLGESCHLAGEN

Fehler 1: [Datei:Zeile] [Fehlermeldung]
  -> Wahrscheinliche Ursache: [kurze Erklaerung]

Fehler 2: ...

Empfehlung: [Was als naechstes tun -- z.B. "rust-core Agent beauftragen mit Fix fuer X"]
```

### Wenn plattform = `windows`

WICHTIG: Zuerst laufende Instanz beenden, dann das PowerShell-Build-Skript nutzen, NICHT direkt `tauri build`:
```bash
taskkill.exe /IM voxlit.exe /F 2>/dev/null; powershell.exe -Command "cd D:\Apps\voxlit; .\scripts\sync-and-build.ps1" 2>&1
```

Falls PowerShell nicht verfuegbar (z.B. reines WSL ohne Windows-Zugriff), Fallback:
```bash
cd /home/andyon2/claude-projects/voxlit && npm run tauri build 2>&1
```

Bei Erfolg melde:
```
BUILD ERFOLGREICH (Windows)

Binary: [Pfad zur .exe / .msi]
Groesse: [Dateigroesse]
```

Bei Fehler: Strukturierte Fehlermeldung wie oben.

### Wenn plattform = `android`

WICHTIG: Nutze das Build-Skript, NICHT direkt `tauri android build`. Das Skript kopiert Kotlin-Quellen aus `android/kotlin-src/` nach `gen/android/`, signiert und deployt. Ohne dieses Skript fehlen die Kotlin-Dateien und der Build schlaegt fehl.

```bash
cd /home/andyon2/claude-projects/voxlit && bash scripts/android-build.sh 2>&1
```

Bei Erfolg melde:
```
BUILD ERFOLGREICH (Android)

APK: [Pfad zur .apk aus Script-Output]
Groesse: [Dateigroesse]
```

Bei Fehler: Strukturierte Fehlermeldung. Android-Build-Fehler sind oft kryptisch -- versuche die eigentliche Ursache zu identifizieren (fehlende SDK-Version, Gradle-Fehler, NDK-Problem, fehlende Kotlin-Quellen).

### Nach erfolgreichem Build: Test-Checkliste ausgeben

Nach JEDEM erfolgreichen Build (windows oder android), ermittle was sich seit dem letzten Build geaendert hat und gib eine Testliste aus.

1. Lies die relevanten Commits. Fuer Windows:
```bash
git log --oneline --since="$(stat -c '%Y' /mnt/d/Apps/voxlit/src-tauri/target/release/voxlit.exe 2>/dev/null | xargs -I{} date -d @{} --iso-8601=seconds 2>/dev/null || echo '1 week ago')" HEAD 2>/dev/null
```
Falls der Timestamp nicht ermittelbar, nimm die letzten 10 Commits: `git log --oneline -10`

2. Analysiere die Commit-Messages und fasse die nutzer-sichtbaren Aenderungen als Testliste zusammen. Ignoriere reine Refactorings, Docs, CI-Aenderungen -- nur was man in der App sehen/testen kann.

3. Gib die Liste so aus (direkt im Terminal, KEINE Datei):
```
🧪 Teste bitte die neuen Features:

1. [Feature/Fix-Beschreibung in 1 Satz, aus Nutzersicht]
2. [...]
3. [...]

Starte: D:\Apps\voxlit\src-tauri\target\release\voxlit.exe
```

Falls keine nutzer-sichtbaren Aenderungen: "Keine neuen Features -- nur interne Aenderungen."

### Wenn plattform nicht erkannt

Melde: "Unbekannte Plattform `[plattform]`. Verfuegbar: windows, android, check."
