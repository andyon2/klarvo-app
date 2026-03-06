---
name: commit-progress
description: Erstellt einen Git-Commit mit konventioneller Commit-Message. Aufrufen nach abgeschlossenen Teilaufgaben, z.B. "/commit-progress" oder "/commit-progress feat audio capture module".
argument-hint: "[optionale commit-beschreibung]"
allowed-tools: Bash
---

Erstelle einen Git-Commit fuer den aktuellen Fortschritt.

## Vorgehensweise

1. `git status` ausfuehren -- was hat sich geaendert?
2. `git diff --stat` -- Ueberblick der Aenderungen
3. Falls `$ARGUMENTS` eine Beschreibung enthaelt, nutze sie als Basis fuer die Message.
   Falls nicht, leite die Message aus den Aenderungen ab.

4. Commit-Message im Conventional-Commits-Format:
   - `feat: [beschreibung]` -- Neues Feature
   - `fix: [beschreibung]` -- Bugfix
   - `refactor: [beschreibung]` -- Code-Umbau ohne Feature-Aenderung
   - `docs: [beschreibung]` -- Dokumentation
   - `chore: [beschreibung]` -- Build, Dependencies, Config
   - `test: [beschreibung]` -- Tests hinzugefuegt/geaendert

5. Relevante Dateien stagen (NICHT `git add .` -- bewusst auswaehlen, keine .env oder Secrets)
6. Commit erstellen

Melde: "Commit erstellt: [commit-hash kurz] [message]"
