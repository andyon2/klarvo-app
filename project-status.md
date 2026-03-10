# Projektstatus

## Aktueller Stand
Version 0.4.1. 222 Rust-Tests (alle gruen). Zwei-Repo-Setup: dikta (privat) + dikta-public (oeffentlich). Offline whisper.cpp Feature komplett implementiert (Backend + UI), aber Windows-Build BLOCKIERT durch LLVM/Clang-Version-Problem.

## Blocker

- **Windows-Build mit whisper-rs:** Clang 22 generiert kaputte Bindings (Codeberg #268). LLVM 18.1.8 muss installiert werden. Danach Build testen. Siehe `knowledge/architecture.md` Abschnitt "Build-Anforderungen whisper-rs".

## Naechste Sessions (in Reihenfolge)

1. **LLVM 18 installieren + Windows-Build testen** — Blocker loesen, dann Offline-Whisper end-to-end testen
2. **Onboarding/Polish** → [Briefing noch zu erstellen]
3. **Bubble Size/Opacity Controls** → `briefings/plan-bubble-appearance.md`

## Bekannte Bugs
- Keine kritischen Bugs (ausser Build-Blocker oben)

## Backlog
- [ ] [shared] VAD -- Voice Activity Detection (Auto-Start/Stopp)
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter existiert)

## Aenderungen Session 2026-03-10b (Offline Whisper + Housekeeping)

- [x] Repo-Trennung: dikta (privat) + dikta-public (oeffentlich), initialer Push
- [x] /release Skill: Releases gehen auf dikta-public, publish.sh integriert
- [x] Offline whisper.cpp: LocalWhisperProvider, Model-Manager, Config, Pipeline-Fallback
- [x] Offline whisper.cpp UI: WhisperModelManager, Download-Button, STT-Priority "local"
- [x] whisper-rs Recherche dokumentiert in knowledge/api-providers.md
- [ ] Windows-Build blockiert: LLVM 18.1.8 installieren, Build erneut testen
