---
name: track
description: "Aktualisiert project-status.md mit dem, was in der aktuellen Session erarbeitet wurde. Keine Argumente noetig."
allowed-tools: Read, Edit, Grep, Glob
---

Du aktualisierst den Projektstatus von Dikta basierend auf dem, was in dieser Session passiert ist.

## Vorgehen

1. Lies `project-status.md`
2. Fasse zusammen, was in dieser Session erarbeitet wurde
3. Aktualisiere:
   - **Aktueller Stand**: Passe die 2-3 Saetze an (Version, was funktioniert)
   - **Naechste Sessions**: Aktualisiere Reihenfolge, entferne erledigte
   - **Bekannte Bugs**: Erledigte entfernen, neue ergaenzen
   - **Backlog**: Erledigte entfernen, neue ergaenzen
4. **Karteileichen-Pruefung (PFLICHT):** Alle offenen Aufgaben gegen den aktuellen Stand abgleichen. Erledigte entfernen oder verschieben.
5. Aenderungen-Bloecke aelter als 2 Sessions entfernen
6. Schreibe die aktualisierte Datei zurueck (Edit, nie Write)

## Regeln
- Halte die Datei unter 50 Zeilen
- Keine Meinung, keine Bewertung -- nur Fakten
- Prioritaeten: Bugs vor Features, Blocker zuerst
- Modul-Referenzen, Changelogs, Code-Dokumentation gehoeren NICHT in project-status.md
