---
name: build-app
description: Baut Dikta fuer eine Zielplattform (Windows oder Android). Meldet Fehler strukturiert. Aufrufen mit Plattform, z.B. "/build windows" oder "/build android".
argument-hint: "[plattform] -- windows | android | check"
allowed-tools: Read, Bash, Glob
context: fork
model: haiku
---

Baue Dikta fuer die angegebene Zielplattform.

## Argumente

Aus `$ARGUMENTS` extrahiere die Plattform: `windows` | `android` | `check` (nur Kompilier-Check ohne Build)

## Vorgehensweise

### Voraussetzungen pruefen

1. Pruefe ob `src-tauri/Cargo.toml` existiert
2. Pruefe ob `package.json` existiert
3. Pruefe ob `node_modules/` existiert, sonst: `npm install`

### Wenn plattform = `check`

```bash
cd /home/andyon2/claude-projects/dikta && cargo check --manifest-path src-tauri/Cargo.toml 2>&1
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

WICHTIG: Nutze das PowerShell-Build-Skript, NICHT direkt `tauri build`:
```bash
cd /home/andyon2/claude-projects/dikta && powershell.exe -File scripts/sync-and-build.ps1 2>&1
```

Falls PowerShell nicht verfuegbar (z.B. reines WSL ohne Windows-Zugriff), Fallback:
```bash
cd /home/andyon2/claude-projects/dikta && npm run tauri build 2>&1
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
cd /home/andyon2/claude-projects/dikta && bash scripts/android-build.sh 2>&1
```

Bei Erfolg melde:
```
BUILD ERFOLGREICH (Android)

APK: [Pfad zur .apk aus Script-Output]
Groesse: [Dateigroesse]
```

Bei Fehler: Strukturierte Fehlermeldung. Android-Build-Fehler sind oft kryptisch -- versuche die eigentliche Ursache zu identifizieren (fehlende SDK-Version, Gradle-Fehler, NDK-Problem, fehlende Kotlin-Quellen).

### Wenn plattform nicht erkannt

Melde: "Unbekannte Plattform `[plattform]`. Verfuegbar: windows, android, check."
