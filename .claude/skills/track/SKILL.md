---
name: track
description: "Projektstatus lesen oder aktualisieren. '/track' fuer kompakten Status, '/track status' fuer ausfuehrlichen Ueberblick, '/track update [was erledigt wurde]' fuer Aktualisierung."
argument-hint: "[update beschreibung]"
allowed-tools: "Read, Edit, Grep, Glob, Bash"
context: fork
model: haiku
---

Du verwaltest den Projektstatus von Dikta.

## Wenn "/track" (ohne Argumente)

1. Lies `project-status.md`
2. Liefere einen kompakten Statusbericht (max 10 Zeilen):
   - Version + aktueller Stand (1 Satz)
   - Top-3 offene Aufgaben (priorisiert)
   - Bekannte Bugs (falls vorhanden)
   - Empfohlener naechster Schritt

## Wenn "/track status"

Ausfuehrlicher Ueberblick. Lies zusaetzlich zu `project-status.md`:
- `briefings/*.md` -- offene Briefings/Feature-Plaene?
- `knowledge/architecture.md` -- aktuelle Phase
- `cargo test --no-run 2>&1 | tail -5` -- kompiliert der Rust-Code?
- `git status --short` -- uncommitted Changes?
- `git log --oneline -3` -- letzte 3 Commits
- `~/project-builder/dispatches.md` -- offene Dispatches fuer dikta?

Zeige diese Tabelle:

| Bereich | Status |
|---------|--------|
| Windows | [Build-Status, offene Bugs] |
| Android | [Build-Status, offene Bugs] |
| Tests | [Anzahl, letzter Lauf OK?] |
| Briefings | [Welche offen, naechstes geplant?] |
| Dispatches | [Offene Wissens-Dispatches?] |
| Git | [Uncommitted Changes? Letzter Commit?] |

Dann:
- **Blocker** -- Was blockiert Fortschritt?
- **Offene Entscheidungen** -- Was muss Andy entscheiden?
- **Naechste Schritte** -- Max 3, priorisiert

Format: Kompakt. Kein Roman. Max 30 Zeilen.

## Wenn "/track update [was erledigt wurde]"

1. Lies `project-status.md`
2. Aktualisiere:
   - **Aktueller Stand:** Passe die 2-3 Saetze an (Version, was funktioniert)
   - **Naechste Sessions:** Aktualisiere Reihenfolge, entferne erledigte
   - **Bekannte Bugs:** Erledigte entfernen, neue ergaenzen
   - **Backlog:** Erledigte entfernen, neue ergaenzen
3. **Karteileichen-Pruefung:** Gleiche JEDE offene Aufgabe gegen den
   aktuellen Stand ab. Wenn eine Aufgabe bereits erledigt wurde,
   entferne sie oder verschiebe sie in den Aenderungen-Block.
4. Aenderungen-Bloecke aelter als 2 Sessions: Entfernen.
   Modul-Referenzen gehoeren in `knowledge/architecture.md`, nicht in den Status.
5. Halte die Datei unter 50 Zeilen.
6. Schreibe die aktualisierte Datei zurueck (Edit, nie Write)

## Regeln
- Keine Meinung, keine Bewertung -- nur Fakten
- Prioritaeten: Bugs vor Features, Blocker zuerst
- Modul-Referenzen, Changelogs, Code-Dokumentation gehoeren NICHT in project-status.md
