# Projektstatus

## Aktueller Stand
Version 0.4.6 released (Windows + Android). 335 Rust-Tests (alle gruen). Hotkey-Modi funktionieren. Waveform echtzeit-responsiv. Live-Preview deaktiviert (Groq-Quota-Fix). Onboarding mit Free-Tier-Hinweis. Auto-Modi als Experimental markiert. OpenRouter verifiziert (API-Test erfolgreich). README ueberarbeitet (Quick Start, Provider-Tabellen, Cloud/Offline-Weiche) aber Feature-Sektion unvollstaendig -- wartet auf USP-Analyse.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Feature-Inventar + USP-Analyse** → Vollstaendige Feature-Liste aus Code erstellen. Jedes Feature gegen Wispr Flow und andere Konkurrenten pruefen. Ergebnis in zentralem Dokument (`knowledge/feature-inventory.md`). Dann README-Feature-Sektion daraus ableiten. Briefing liegt unter `briefings/readme-feature-analysis.md`.
2. **Silence Detection Deep Research** → Ergebnisse aus Claude Deep Research integrieren, VAD evaluieren. Briefing: `briefings/deep-research-silence-detection.md`.
3. **Signing + Auto-Update** → Grundvoraussetzung fuer Paid Release

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
- [ ] [feature] OpenRouter Modell-Dropdown in Settings (aktuell hardcoded auf deepseek/deepseek-chat).
