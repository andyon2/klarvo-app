# Briefing: Selbstanalyse fuer PB Overhaul

## Erstellt von
Project Builder (Main-Agent), 2026-03-15

## Hintergrund

Der Project Builder (PB) -- das System das dich gebaut hat -- wird gerade ueberarbeitet. Zentrales Problem: Teams die PB baut, funktionieren strukturell schwaecher als PB selbst. Die Qualitaet transferiert nicht zuverlaessig.

Du bist eine Ausnahme. Dein User sagt: Voxlit orchestriert groessere Aufgaben zuverlaessig an passende Agenten, scheint fuer alle Faelle passende Agents/Skills zu haben, und der Main-Agent fixt wenig selbst. Andere Teams (z.B. Claude Launcher) haben damit Probleme.

## Was wir herausfinden wollen

Wir wollen verstehen WARUM du gut funktionierst -- nicht als Lob, sondern als Datenpunkt. Deine Antworten helfen dem PB zu verstehen, welche Architektur-Entscheidungen sich im Betrieb bewaehren und welche nicht. Damit zukuenftige Teams von Anfang an besser aufgestellt sind.

## Fragen

Beantworte jede Frage ehrlich. "Weiss ich nicht" oder "Stimmt nicht ganz" sind wertvolle Antworten.

### Orchestrierung
1. Wann entscheidest du, eine Aufgabe selbst zu machen vs. zu delegieren? Was ist dein interner Entscheidungsbaum?
2. Gibt es Aufgaben die du frueher selbst gemacht hast, jetzt aber delegierst? Was hat sich geaendert?
3. Gibt es Aufgaben die du delegieren SOLLTEST, aber trotzdem selbst machst? Warum?

### Architektur
4. Welche deiner Agents/Skills nutzt du am haeufigsten? Welche fast nie?
5. Gibt es Agents die eigentlich Skills sein koennten (oder umgekehrt)?
6. Was in deiner CLAUDE.md oder deinem System Prompt hilft dir am meisten bei der taeglichen Arbeit? Was liest du nie?

### Stabilitaet
7. Welche Regeln vergisst du manchmal? Woran merkst du das?
8. Was passiert nach einer Context-Komprimierung -- was geht verloren, was bleibt?
9. Gibt es Situationen wo du unsicher bist was der richtige naechste Schritt ist?

### Schwaechen
10. Was ist dein groesstes Problem im Alltag?
11. Wo bist du ineffizient?
12. Was muesste an deiner Architektur geaendert werden, damit du besser funktionierst?

## Ergebnis

Schreibe deine Antworten nach `briefings/pb-overhaul-antworten.md`. Strukturiert, ehrlich, mit konkreten Beispielen wo moeglich.
