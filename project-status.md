# Projektstatus

## Aktueller Stand
Version 0.4.1. 284 Rust-Tests (alle gruen). Windows-Build erfolgreich (2026-03-11). UI Preview Mode: `npm run preview` startet Frontend ohne Tauri/Rust (Port 1422, gemockte Commands). Laeuft auf Hetzner-Server und lokal.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Preview-Feedback umsetzen** → Trial-Datum im Preview ("Preview Mode" statt Datum), Profiles im Free sperren (Paid-Modi nicht auswaehlbar)
2. **Manueller Test** → Settings-Umbau + Collapsible Subsections auf Windows verifizieren
3. **Onboarding/Polish** → [Briefing noch zu erstellen]
4. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`

## Bekannte Bugs
- [ ] [ui] App Profiles: Paid-Modi im Free-Tier auswaehlbar (muss geblockt werden)
- [ ] [ui] Preview-Modus: Trial-Ablaufdatum zeigt 2286 statt "Preview Mode"

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 22 Compiler-Warnings aufraumen (dead code, private interfaces)
- [ ] [frontend] @dnd-kit aus node_modules entfernen (npm install nach package.json-Aenderung)
