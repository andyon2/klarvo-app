# Rueckwaerts-Suche bei strukturellen Aenderungen

Quelle: Architektur-Dialog PB + mr-review (2026-03-11)

Neue Standard-Regel fuer CLAUDE.md `## Regeln`. Bitte aufnehmen:

```
**Rueckwaerts-Suche bei Umbau:** Vor dem ersten Edit bei strukturellen Aenderungen: `grep -r` nach allen Konsumenten des Geaenderten. Erst dann editieren. Strukturell = Entfernen, Umbenennen, Output-Format aendern, Verantwortlichkeit zwischen Komponenten verschieben. Nicht strukturell = Hinzufuegen, Erweitern, neue Datei anlegen.
```

Hintergrund:
- Wenn ein Agent etwas entfernt, umbenennt oder Verantwortlichkeiten verschiebt, muessen alle Konsumenten des Geaenderten geprueft werden
- Ohne diesen Schritt bleiben Skills, Templates und Agents inkonsistent -- faellt oft erst in der naechsten Session auf
- Kein neuer Skill noetig -- es ist ein Grep vor dem Edit, kein Workflow

**Zusaetzlich:** Falls ein Skill existiert, der neue Elemente entwirft oder erstellt (z.B. `extend-team`): Dessen Output-Format um eine Sektion "Betroffene bestehende Elemente (Rueckwaerts-Check)" ergaenzen. Grep-Ergebnisse angeben. Bei reiner Ergaenzung: "Keine -- reine Ergaenzung."
