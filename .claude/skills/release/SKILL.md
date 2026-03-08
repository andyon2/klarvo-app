---
name: release
description: Bumpt die Version, baut beide Plattformen, erstellt GitHub Release mit Artefakten. Aufrufen mit Version-Bump-Typ, z.B. "/release minor" oder "/release patch" oder "/release 0.6.0".
argument-hint: "[minor | patch | major | X.Y.Z] -- Version bump type or explicit version"
allowed-tools: Read, Edit, Bash, Glob
context: fork
model: haiku
---

Erstelle einen vollstaendigen Release: Version bump, Build, GitHub Release.

## Argumente

Aus `$ARGUMENTS` extrahiere den Bump-Typ oder die explizite Version:
- `patch` → 0.4.0 → 0.4.1
- `minor` → 0.4.0 → 0.5.0
- `major` → 0.4.0 → 1.0.0
- `X.Y.Z` → Explizite Version (z.B. `0.6.0`)

Falls kein Argument: Fehler melden. Nie raten.

## Vorgehensweise

### 1. Aktuelle Version lesen

Lies die Version aus `src-tauri/Cargo.toml` (Zeile `version = "..."`).

### 2. Neue Version berechnen

Wenn Bump-Typ (`patch`/`minor`/`major`):
- Parse die aktuelle Version als MAJOR.MINOR.PATCH
- Inkrementiere den entsprechenden Teil (bei minor/major werden niedrigere Teile auf 0 gesetzt)

Wenn explizite Version:
- Validiere Format X.Y.Z
- Pruefe dass die neue Version HOEHER ist als die aktuelle

### 3. Version in allen drei Dateien aktualisieren

Alle drei muessen synchron sein:

1. `src-tauri/Cargo.toml` → `version = "X.Y.Z"`
2. `src-tauri/tauri.conf.json` → `"version": "X.Y.Z"`
3. `package.json` → `"version": "X.Y.Z"`

Nutze `Edit` (nicht sed) fuer jede Datei. Ersetze NUR die alte Version durch die neue.

### 4. Beide Plattformen bauen

Windows:
```bash
cd /home/andyon2/dikta && powershell.exe -Command "Get-Process dikta -ErrorAction SilentlyContinue | Stop-Process -Force" 2>/dev/null; powershell.exe -ExecutionPolicy Bypass -File '\\wsl$\Ubuntu\home\andyon2\dikta\scripts\sync-and-build.ps1' 2>&1
```

Android:
```bash
cd /home/andyon2/dikta && bash scripts/android-build.sh 2>&1
```

Falls ein Build fehlschlaegt: Abbrechen und strukturierten Fehler melden. NICHT den Release erstellen.

### 5. Git Commit + Push

```bash
cd /home/andyon2/dikta
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json package.json
git commit -m "chore: bump version to X.Y.Z"
git push origin master
```

### 6. GitHub Release erstellen

Finde die Build-Artefakte:
- Windows NSIS: `/mnt/d/Apps/dikta/src-tauri/target/release/bundle/nsis/Dikta_X.Y.Z_x64-setup.exe`
- Android APK: `/mnt/d/Dropbox/App Development/dikta/Dikta.apk`

Pruefe dass beide Dateien existieren und eine plausible Groesse haben (>1MB).

```bash
gh release create vX.Y.Z \
  "/mnt/d/Apps/dikta/src-tauri/target/release/bundle/nsis/Dikta_X.Y.Z_x64-setup.exe" \
  "/mnt/d/Dropbox/App Development/dikta/Dikta.apk#Dikta-vX.Y.Z.apk" \
  --repo andyon2/dikta \
  --title "vX.Y.Z" \
  --notes "Release vX.Y.Z"
```

HINWEIS: Die Release-Notes sind bewusst minimal. Der Main-Agent oder Andy kann sie nachtraeglich ueber die GitHub-Webseite oder `gh release edit` ergaenzen.

### 7. Ergebnis melden

```
RELEASE ERSTELLT: vX.Y.Z

Version: X.Y.Z (vorher: A.B.C)
Windows: Dikta_X.Y.Z_x64-setup.exe ([Groesse])
Android: Dikta-vX.Y.Z.apk ([Groesse])
Release: https://github.com/andyon2/dikta/releases/tag/vX.Y.Z

Naechster Schritt: Release-Notes ergaenzen (gh release edit vX.Y.Z --notes "...")
```
