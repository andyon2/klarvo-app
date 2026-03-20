# Projektstatus

## Aktueller Stand
Version 0.4.7 released (Windows + Android). 369 Rust-Tests (alle gruen). README komplett gegen Feature-Inventar abgeglichen und ueberarbeitet. BSL 1.1 Lizenz eingefuehrt (LICENSE Datei, CLAUDE.md Regel 13, architecture.md). Pricing-Strategie dokumentiert (EUR 29 Launch, Play Store EUR 14 als 2. Welle). Dreifach-Sicherung gegen Lizenz-Fehler in user-facing Texten.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Signing + Auto-Update** → Grundvoraussetzung fuer Paid Release
2. **License-Gate Code-Aenderungen** → Groq LLM Free machen, Cost Tracking Grundfunktion Free machen (`license/mod.rs`)
3. **Live-Preview als Opt-In** → Nach VAD-Overhaul jetzt moeglich. Whisper-Halluzinationen werden gefiltert.

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.
- [ ] Updater ACL-Fehler: "Command plugin:updater|check not allowed by ACL". Low-Prio.

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
