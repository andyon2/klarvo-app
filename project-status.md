# Projektstatus

## Aktueller Stand
Version 0.4.1. 284 Rust-Tests (alle gruen). Windows-Build erfolgreich (2026-03-11). Advanced Settings: Collapsible Subsections in Text Cleanup (free=offen, paid=zugeklappt). Provider-Direktauswahl, Cloud/Offline STT-Toggle, Modell-Picker. Settings-Panel mit Dirty-State Save-Button, Trial-Keys mit Ablaufdatum.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Manueller Test** → Settings-Umbau + Collapsible Subsections auf Windows verifizieren
2. **Onboarding/Polish** → [Briefing noch zu erstellen]
3. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`

## Bekannte Bugs
Keine.

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 22 Compiler-Warnings aufraumen (dead code, private interfaces)
- [ ] [frontend] @dnd-kit aus node_modules entfernen (npm install nach package.json-Aenderung)
