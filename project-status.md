# Projektstatus

## Aktueller Stand
Version 0.4.8 (Windows + Android). Rename Dikta → Voxlit abgeschlossen. Domain `voxlit.app` gesichert. Voice Command Mode geparkt (Architektur-Limitation, wird mit SAPI neu aufgesetzt). Kernfunktion (Hotkey-Diktat) stabil.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Launch-Vorbereitung** → Landingpage bauen (Briefing fertig), Social Preview hochladen.

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.

## Backlog
- [ ] [desktop] Auto-Updater funktioniert nicht bei Tester. Ursache unklar (Firewall/AV?).
- [ ] [shared] Anthropic-Provider verifizieren und ggf. wieder freischalten
- [ ] [shared] OpenAI-Provider mit echtem Key testen
- [ ] [shared] Chunking-Drift Rust vs Kotlin angleichen
- [ ] [rust] 43 Clippy-Warnings + 27 Compiler-Warnings aufraumen. Pre-Commit-Hooks auf blocking umstellen danach.
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [frontend] @dnd-kit aus node_modules entfernen
- [ ] [android] Long-Press-Dauer einstellbar machen (aktuell hardcoded 500ms)
- [ ] [feature] User-definierbare Transkript-Blocklist. Phase: Polish.
- [ ] [feature] OpenRouter Modell-Dropdown in Settings (aktuell hardcoded auf deepseek/deepseek-chat).
- [ ] [feature] Reformat-Prompts (Email/Bullets/Summary) verbessern -- aktuell schlechte Qualitaet, aus README entfernt.
- [ ] [android] Bubble Size/Opacity UI-Controls implementieren (Backend-Config existiert, Frontend fehlt).
- [ ] [desktop] [paid] SAPI-basierte Command-Erkennung: Windows Speech Recognition API fuer Echtzeit-Befehle waehrend Diktat. Custom Grammar mit "Klarvo" Phonem-Definition. Parallel zu cpal Recording, on-device, ~50ms Latenz. Phase: Post-Launch.
