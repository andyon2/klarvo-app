# Projektstatus

## Aktueller Stand
Version 0.4.7 released (Windows + Android). 369 Rust-Tests (alle gruen). Silero VAD v5 ersetzt RMS-Silence-Detection (beide Plattformen). Whisper-Hallucination-Blocklist aktiv. Verbatim/Chat Cleanup-Stile und Whisper small jetzt Free. Feature-Inventar mit USP-Analyse liegt unter `knowledge/feature-inventory.md`. README Feature-Sektionen ueberarbeitet aber enthalten noch Fehler -- muss gegen Inventar abgeglichen werden.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **README gegen Feature-Inventar abgleichen** → Jede Zeile in der README gegen `knowledge/feature-inventory.md` pruefen. Bekannte Fehler: Android Bubble-Beschreibung falsch (fehlende Keyboard-Detection, falsche Gestenbeschreibung). Lektion: Inventar als Checkliste nutzen, nicht aus dem Kopf schreiben.
2. **Signing + Auto-Update** → Grundvoraussetzung fuer Paid Release
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
