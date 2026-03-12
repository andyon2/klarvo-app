# Projektstatus

## Aktueller Stand
Version 0.4.2. Release auf GitHub (Windows + Android). 284 Rust-Tests (alle gruen). UI Preview Mode auf Hetzner-Server und lokal. Konvention: README, Release-Notes und nutzerseitige Texte auf Deutsch. Code bleibt Englisch.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Config-Migration Fix** → GitHub Issue #1: Auth-Fehler nach Update (fehlende sttPriority→sttProvider Migration)
2. **Sessionstart: GitHub-Issue-Check** → Ablauf in main-agent.md + workflow.md festhalten
3. **Onboarding/Polish** → [Briefing noch zu erstellen]
4. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`
5. **Englisch als UI-Sprache** → Zweite Sprache fuer die UI (Zukunfts-Feature)

## Bekannte Bugs

- [ ] GitHub #1: Auth schlaegt nach Update auf v0.4.2 fehl (Config-Migration sttPriority→sttProvider fehlt)

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 22 Compiler-Warnings aufraumen (dead code, private interfaces)
- [ ] [frontend] @dnd-kit aus node_modules entfernen (npm install nach package.json-Aenderung)
