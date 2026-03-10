---
name: release
description: Bumpt die Version, baut beide Plattformen, synct nach dikta-public, erstellt GitHub Release dort. Aufrufen mit Version-Bump-Typ, z.B. "/release minor" oder "/release patch" oder "/release 0.6.0".
argument-hint: "[minor | patch | major | X.Y.Z] -- Version bump type or explicit version"
allowed-tools: Read, Edit, Bash, Glob
context: fork
model: haiku
---

Erstelle einen vollstaendigen Release: Version bump, Build, Sync nach Public Repo, GitHub Release.

## Argumente

Aus `$ARGUMENTS` extrahiere den Bump-Typ oder die explizite Version:
- `patch` → 0.4.0 → 0.4.1
- `minor` → 0.4.0 → 0.5.0
- `major` → 0.4.0 → 1.0.0
- `X.Y.Z` → Explizite Version (z.B. `0.6.0`)

Falls kein Argument: Fehler melden. Nie raten.

## Zwei-Repo-Architektur

- `dikta` (privat, `~/claude-projects/dikta`): Arbeitsrepo. Code + Agents. Hier wird entwickelt und gebaut.
- `dikta-public` (oeffentlich, `~/dikta-public`): Produktcode-Mirror. Hier landen Releases fuer Nutzer.

Releases werden IMMER auf `dikta-public` erstellt (--repo andyon2/dikta-public).

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
cd /home/andyon2/claude-projects/dikta && powershell.exe -Command "Get-Process dikta -ErrorAction SilentlyContinue | Stop-Process -Force" 2>/dev/null; powershell.exe -ExecutionPolicy Bypass -File '\\wsl$\Ubuntu\home\andyon2\claude-projects\dikta\scripts\sync-and-build.ps1' 2>&1
```

Android:
```bash
cd /home/andyon2/claude-projects/dikta && bash scripts/android-build.sh 2>&1
```

Falls ein Build fehlschlaegt:
1. **Version-Bump revertern:** `git checkout -- src-tauri/Cargo.toml src-tauri/tauri.conf.json package.json`
2. Strukturierten Fehler melden (welcher Build, welcher Fehler, wahrscheinliche Ursache)
3. NICHT den Release erstellen, NICHT committen
4. Wenn nur EIN Build fehlschlaegt (z.B. Android OK, Windows nicht): Trotzdem komplett abbrechen. Kein partieller Release.

### 5. Updater-Artefakte pruefen und latest.json generieren

Nach dem Windows-Build existieren diese Dateien (durch `createUpdaterArtifacts: true` + Signing Key):
- NSIS Installer: `/mnt/d/Apps/dikta/src-tauri/target/release/bundle/nsis/Dikta_X.Y.Z_x64-setup.exe`
- Signatur: `/mnt/d/Apps/dikta/src-tauri/target/release/bundle/nsis/Dikta_X.Y.Z_x64-setup.exe.sig`

Lies den Inhalt der `.sig`-Datei (das ist die Signatur als String, base64-kodiert).

Erstelle `/tmp/latest.json` mit diesem Inhalt:
```json
{
  "version": "X.Y.Z",
  "notes": "Release vX.Y.Z",
  "pub_date": "<aktuelles ISO-8601 Datum, z.B. 2026-03-09T12:00:00Z>",
  "platforms": {
    "windows-x86_64": {
      "signature": "<Inhalt der .sig-Datei>",
      "url": "https://github.com/andyon2/dikta-public/releases/download/vX.Y.Z/Dikta_X.Y.Z_x64-setup.exe"
    }
  }
}
```

Nutze `date -u +%Y-%m-%dT%H:%M:%SZ` fuer das Datum.

WICHTIG: Die URL zeigt auf `dikta-public`, NICHT auf `dikta`!

Falls die `.sig` Datei NICHT existiert: Warnung ausgeben. Der Build hat vermutlich den Signing Key nicht gefunden. Trotzdem fortfahren, aber in der Ergebnis-Meldung darauf hinweisen.

### 6. Git Commit + Push (privates Repo)

```bash
cd /home/andyon2/claude-projects/dikta
git add src-tauri/Cargo.toml src-tauri/tauri.conf.json package.json
git commit -m "chore: bump version to X.Y.Z"
git push origin master
```

### 7. Public Repo sync + commit + push

```bash
cd /home/andyon2/claude-projects/dikta && bash scripts/publish.sh
```

Falls publish.sh mit Marker-Warnung abbricht: STOPP. Dem Nutzer melden. Nicht weiter machen.

Falls OK:
```bash
cd /home/andyon2/dikta-public
git commit -m "chore: sync release vX.Y.Z"
git push origin main
```

### 8. GitHub Release erstellen (auf dikta-public!)

Pruefe dass alle Artefakte existieren und eine plausible Groesse haben (>1MB fuer Installer/APK).

```bash
gh release create vX.Y.Z \
  "/mnt/d/Apps/dikta/src-tauri/target/release/bundle/nsis/Dikta_X.Y.Z_x64-setup.exe" \
  "/mnt/d/Apps/dikta/src-tauri/target/release/bundle/nsis/Dikta_X.Y.Z_x64-setup.exe.sig#Dikta_X.Y.Z_x64-setup.exe.sig" \
  "/tmp/latest.json" \
  "/mnt/d/Dropbox/App Development/dikta/releases/vX.Y.Z/Dikta-vX.Y.Z.apk" \
  --repo andyon2/dikta-public \
  --title "vX.Y.Z" \
  --notes "Release vX.Y.Z"
```

WICHTIG: `--repo andyon2/dikta-public` (NICHT dikta!)

### 9. Ergebnis melden

```
RELEASE ERSTELLT: vX.Y.Z

Version: X.Y.Z (vorher: A.B.C)
Windows Installer: Dikta_X.Y.Z_x64-setup.exe ([Groesse])
Signatur: Dikta_X.Y.Z_x64-setup.exe.sig
Updater Manifest: latest.json (signiert: ja/nein)
Android: Dikta-vX.Y.Z.apk ([Groesse])
Public Repo: synced + pushed
Release: https://github.com/andyon2/dikta-public/releases/tag/vX.Y.Z

Auto-Update: Nutzer mit v0.4.1+ bekommen Update-Benachrichtigung in den Settings.
Naechster Schritt: Release-Notes ergaenzen (gh release edit vX.Y.Z --repo andyon2/dikta-public --notes "...")
```
