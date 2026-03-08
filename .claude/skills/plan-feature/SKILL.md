---
name: plan-feature
description: Zerlegt ein neues Feature in konkrete Tasks mit Dateien, Abhaengigkeiten und Agent-Zuweisung. Aufrufen mit Feature-Beschreibung, z.B. "/plan-feature global hotkey system".
argument-hint: "[feature-beschreibung]"
allowed-tools: Read, Glob
context: fork
model: sonnet
---

Zerlege ein Feature in einen konkreten Umsetzungsplan.

## Argumente

Aus `$ARGUMENTS` extrahiere die Feature-Beschreibung.

## Vorgehensweise

1. **Kontext lesen:**
   - `CLAUDE.md` -- Projekt-Ueberblick
   - `knowledge/architecture.md` -- bestehende Architektur
   - `project-status.md` -- aktueller Stand
   - Relevante bestehende Code-Dateien (via Glob pruefen was schon existiert)

2. **Feature analysieren:**
   - Was genau soll das Feature tun? (User-Perspektive)
   - Welche Module sind betroffen? (Rust-Backend, Frontend, Android?)
   - Gibt es Abhaengigkeiten zu anderen Features?
   - Was muss zuerst gebaut werden?

3. **Plan erstellen im Format:**

```markdown
# Feature-Plan: [Feature-Name]

## User Story
Als Dikta-Nutzer moechte ich [was], damit [warum].

## Betroffene Module
- [Modul 1]: [Was dort geaendert/erstellt wird]
- [Modul 2]: ...

## Tasks (in Reihenfolge)

### Task 1: [Titel]
- **Agent:** rust-core | ui-dev | android-platform
- **Dateien:** [Welche Dateien erstellt/geaendert werden]
- **Abhaengigkeit:** keine | Task N
- **Beschreibung:** [Konkreter Auftrag fuer den Agent, 2-3 Saetze]

### Task 2: [Titel]
...

## Testplan
- [ ] [Wie testen wir, dass das Feature funktioniert?]
- [ ] [Edge Cases]

## Risiken
- [Moegliche Probleme und wie wir damit umgehen]
```

4. **Plan ausgeben oder persistieren:**
   - Wenn `$ARGUMENTS` mit `--save` endet: Plan in `briefings/plan-[feature-slug].md` schreiben (Feature-Name als Slug: Kleinbuchstaben, Bindestriche statt Leerzeichen). Melde: "Plan gespeichert in briefings/plan-[slug].md"
   - Ohne `--save`: Plan nur im Chat ausgeben (der Main-Agent entscheidet, ob der Plan so umgesetzt wird).
