# Projektstatus

## Aktueller Stand
Version 0.4.6 released (Windows + Android). 335 Rust-Tests (alle gruen). Hotkey-Modi (Toggle/Hold/AutoStop) funktionieren. Waveform reagiert auf echte Audio-Levels. Live-Preview deaktiviert (Groq-Quota-Fix). Onboarding mit Free-Tier-Hinweis. Auto-Modi als Experimental markiert. Pre-Commit-Hooks eingerichtet (non-blocking). Deep-Research-Prompts fuer Silence-Detection vorbereitet.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Silence Detection Deep Research** → Ergebnisse aus Claude Deep Research integrieren, VAD evaluieren (WebRTC VAD, Silero). Briefing liegt unter `briefings/deep-research-silence-detection.md`.
2. **Signing + Auto-Update** → Grundvoraussetzung fuer Paid Release
3. **Onboarding Polish** → Android Smoke-Test, UX-Feinschliff nach Tester-Feedback

## Bekannte Bugs

- [ ] Auto-Mode Silence-Detection unzuverlaessig: Duration-Slider scheint wenig Wirkung, Musik-Bleed-Through verhindert Silence-Erkennung. Wartet auf VAD-Overhaul.
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
- [ ] [ux] Live-Preview als Opt-In wiederbeleben (nach VAD-Overhaul). Whisper-Halluzinationen filtern.
