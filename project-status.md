# Projektstatus

## Aktueller Stand
Version 0.4.1. 239 Rust-Tests (alle gruen). Free/Paid-Aufteilung finalisiert: Cleanup Styles alle free, Dictionary free mit 20-Limit, Offline-Whisper small free / medium+large-v3 paid. tiny/base Modelle entfernt (Qualitaet zu niedrig). Offline-Whisper E2E getestet und funktioniert. Voice & Recording Sektion redesigned: Cloud/Offline Toggle oben, Modellauswahl mit Provider-Preisen. Pipeline-Bug gefixt (com.dikta.app → com.dikta.voice). knowledge/workflow.md eingefuehrt fuer session-uebergreifendes Lernen.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Provider Priority entfernen** — Modell-Dropdown oben reicht, Priority-Liste ist redundant. STT+LLM Dropdown dynamisch nach hinterlegten Keys. Backend-Pipeline anpassen.
2. **Onboarding/Polish** → [Briefing noch zu erstellen]
3. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`

## Bekannte Bugs
Keine.

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 20 Compiler-Warnings aufraumen (dead code, private interfaces)
