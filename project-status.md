# Projektstatus

## Aktueller Stand
Version 0.4.3. Release auf GitHub (Windows). 270 Rust-Tests (alle gruen). FloatingBar redesigned: groessere Pill (200x36) mit Mic-Icon, Idle hidden, Drag, Position-Persistierung, Work Area API. Play-Store-Analyse in knowledge/. Konvention: README, Release-Notes und nutzerseitige Texte auf Deutsch. Code bleibt Englisch.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Clean-Mode Ueberarbeitung** → Andy findet Clean-Modus entfremdet zu sehr das Original
2. **Onboarding/Polish** → [Briefing noch zu erstellen]
3. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`
4. **Englisch als UI-Sprache** → Zweite Sprache fuer die UI (Zukunfts-Feature)

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio, Settings-Reposition als spaeteres Feature geplant.

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 23 Compiler-Warnings aufraumen (dead code, private interfaces, unused imports)
- [ ] [frontend] @dnd-kit aus node_modules entfernen (npm install nach package.json-Aenderung)
