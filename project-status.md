# Projektstatus

## Aktueller Stand
Version 0.4.1. 230 Rust-Tests (alle gruen). Paid-Feature-Gates ueberarbeitet: Lock-Icons an Cross-Device Sync, Dictionary, Speech Recognition, Webhook, Integrations. Stats/Integrations-Panel zeigen Paid-Nachricht statt leerem Raum. Cleanup Styles (Clean/Chat) gelockt in Settings UND Header-StylePicker. Offline-Modus blendet LLM-Provider-Liste und Cleanup Instructions aus. Projektverzeichnis nach ~/claude-projects/dikta verschoben, alle Pfade aktualisiert.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Uncommitted Changes committen** — Paid-Gates, Offline-UX, Pfad-Fixes (noch nicht committed)
2. **Offline-Whisper end-to-end testen** — Modell laden, diktieren, Ergebnis pruefen
3. **Onboarding/Polish** → [Briefing noch zu erstellen]
4. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`

## Bekannte Bugs
- Statistics-Panel: Leer nach Rebuild — Ursache noch unklar (DB-Pfad? App-Identifier?)

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] STT/LLM Settings UX weiter vereinfachen
- [ ] [ui] 20 Compiler-Warnings aufraumen (dead code, private interfaces)
