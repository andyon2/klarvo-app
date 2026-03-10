# Projektstatus

## Aktueller Stand
Version 0.4.1. 230 Rust-Tests (alle gruen). Windows-Build mit Offline-Whisper funktioniert. Settings-UI ueberarbeitet (Cloud/Offline Toggle, Paid-Feature-Gates). 13 Dateien mit uncommitted Changes aus letzter Session (Offline-Modus, Settings-Redesign, License-Fix, LLM-Chunk-Threshold).

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Uncommitted Changes committen + neuen Build machen** — 13 Dateien, substantielle Aenderungen
2. **Offline-Whisper end-to-end testen** — Modell laden, diktieren, Ergebnis pruefen
3. **Onboarding/Polish** → [Briefing noch zu erstellen]
4. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`

## Bekannte Bugs
- License-Key-Eingabe: maxLength 24 statt 25 (Fix lokal, nicht committed/released, von Andy bestaetigt 2026-03-10)

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] STT/LLM Settings UX weiter vereinfachen
