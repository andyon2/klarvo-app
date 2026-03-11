# Projektstatus

## Aktueller Stand
Version 0.4.1. 284 Rust-Tests (alle gruen). Settings-Panel ueberarbeitet: Text Cleanup in Voice & Recording integriert, STT-Modelle nach API-Keys gefiltert, API Keys Labels mit Einsatzzweck. Save-Button nur bei Dirty-State sichtbar. Lizenz-Activate speichert sofort. Trial-Keys mit Ablaufdatum implementiert. License-Secret aus publish.sh fuer dikta-public gescrubbt. Build-Skill auf Sonnet + Test-Checkliste erweitert.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Build + Test** → Settings-Umbau verifizieren (Code steht, noch nicht gebaut/getestet)
2. **Onboarding/Polish** → [Briefing noch zu erstellen]
3. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`

## Bekannte Bugs
Keine.

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 22 Compiler-Warnings aufraumen (dead code, private interfaces)
- [ ] [frontend] @dnd-kit aus node_modules entfernen (npm install nach package.json-Aenderung)
