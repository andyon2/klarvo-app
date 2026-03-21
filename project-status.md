# Projektstatus

## Aktueller Stand
Version 0.4.8 (Windows + Android). Rename Dikta → Voxlit vollstaendig abgeschlossen. Domain `voxlit.app` gesichert. Markenanmeldung DPMA vor Paid Launch. Namensaenderung zu "Klarvo" steht bevor — betrifft Trigger-Wortliste in voice_command/mod.rs.

**VAD-Overhaul:** 5/6 Tasks erledigt. Offen: Task 6 (Manueller Test). Hallucination-Blocklist (Task 4) refactored nach `stt/hallucination.rs` mit 18 Tests.

**Voice Command Mode:** 8/8 Tasks implementiert, aber **Debugging noetig.** Architektur steht (Monitor → VAD → Snippet → Groq → Command-Match → Dispatch). Offene Bugs:
- Auto-Start feuert obwohl Config `voiceCommandEnabled: false` sagt (Phantom-Start)
- Toggle-Desync: UI-State und Backend-Runtime-State laufen auseinander
- Build-Sync: `sync-and-build.ps1` uebernimmt manchmal Aenderungen nicht (Cargo-Cache)
- Erkennung noch nicht live getestet (Groq-Pfad noch nie erfolgreich durchlaufen)

Alle eprintln-Debug-Ausgaben sind noch aktiv fuer die naechste Debug-Session.

## Blocker

Voice Command Mode: Debug-Session noetig (siehe oben). Feature ist implementiert aber nicht funktional getestet.

## Naechste Sessions (in Reihenfolge)

1. **Voice Command Mode debuggen** → Auto-Start-Bug fixen, Toggle-Desync loesen, Groq-Pfad end-to-end testen. Dann: eprintln durch log:: ersetzen. Siehe `briefings/voice-command-debug-status.md`.
2. **Live-Preview als Opt-In** → Whisper-Halluzinationen werden gefiltert (Blocklist steht).
3. **Launch-Vorbereitung** → Landingpage bauen (Briefing fertig), Social Preview hochladen (`marketing/social-preview-voxlit.png`).

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
