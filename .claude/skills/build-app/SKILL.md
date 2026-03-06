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
cd /home/andyon2/dikta && cargo check --manifest-path src-tauri/Cargo.toml 2>&1
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

```bash
cd /home/andyon2/dikta && npm run tauri build 2>&1
```

Bei Erfolg melde:
```
BUILD ERFOLGREICH (Windows)

Binary: [Pfad zur .exe / .msi]
Groesse: [Dateigroesse]
```

Bei Fehler: Strukturierte Fehlermeldung wie oben.

### Wenn plattform = `android`

```bash
cd /home/andyon2/dikta && npm run tauri android build 2>&1
```

Bei Erfolg melde:
```
BUILD ERFOLGREICH (Android)

APK: [Pfad zur .apk]
Groesse: [Dateigroesse]
```

Bei Fehler: Strukturierte Fehlermeldung. Android-Build-Fehler sind oft kryptisch -- versuche die eigentliche Ursache zu identifizieren (fehlende SDK-Version, Gradle-Fehler, NDK-Problem).

### Wenn plattform nicht erkannt

Melde: "Unbekannte Plattform `[plattform]`. Verfuegbar: windows, android, check."
