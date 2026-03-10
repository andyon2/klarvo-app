# Projektstatus

## Aktueller Stand
Version 0.4.1. 250 Rust-Tests (alle gruen). Provider-Priority-System durch direkte Provider-Auswahl ersetzt: `stt_provider` und `llm_provider` in Config, Pipeline-Resolver per Match statt Loop. DnD-Prioritaetsliste entfernt, LLM-Provider-Dropdown (gefiltert nach API-Keys) eingefuehrt. @dnd-kit Dependency entfernt. Alte Config-Felder backward-kompatibel beibehalten.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Onboarding/Polish** → [Briefing noch zu erstellen]
2. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`

## Bekannte Bugs
Keine.

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 20 Compiler-Warnings aufraumen (dead code, private interfaces)
- [ ] [frontend] @dnd-kit aus node_modules entfernen (npm install nach package.json-Aenderung)
