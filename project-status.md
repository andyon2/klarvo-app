# Projektstatus

## Aktueller Stand
Version 0.4.7 (Windows + Android). Auto-Updater funktioniert (ACL-Fix, Signing-Keypair, latest.json auf GitHub). Groq LLM und Cost Tracking jetzt Free. Versions-Anzeige in Settings dynamisch statt hardcoded. SmartScreen-Messaging fuer Launch ohne Zertifikat vorbereitet (briefings/). Entscheidung: Launch ohne Windows Code Signing, Zertifikat nach ersten Verkaeufen.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Live-Preview als Opt-In** → Nach VAD-Overhaul jetzt moeglich. Whisper-Halluzinationen werden gefiltert.
2. **Launch-Vorbereitung** → SmartScreen-Hinweis in README/Landingpage einbauen, Release-Skill um latest.json-Upload erweitern.

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
