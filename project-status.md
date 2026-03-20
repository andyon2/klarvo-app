# Projektstatus

## Aktueller Stand
Version 0.4.5. 335 Rust-Tests (alle gruen). Onboarding-Wizard implementiert (Cloud/Offline-Weiche, API-Key-Validierung, Test-Diktat, Android-Permissions). Kosten-Dashboard mit Wispr-Flow-Savings. Quick-Tip-System (5 kontextuelle Tipps). Silence-Slider (Duration + Threshold) in Settings. Fenstergroesse auf 480x720 erhoeht. Auto-Skip fuer bestehende Nutzer.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Auto-Mode Cleanup-Bug** → Silence erkennt, STT laeuft, aber Cleanup-Ergebnis fehlt + kein Paste. Vorbestehend, nicht durch Onboarding verursacht.
2. **Onboarding Polish** → Android Smoke-Test (Task 12), UX-Feinschliff nach Tester-Feedback
3. **Signing + Auto-Update** → Grundvoraussetzung fuer Paid Release

## Bekannte Bugs

- [ ] Auto-Mode: Nach Silence-Stop laeuft STT, aber Cleanup-Ergebnis verschwindet, kein Paste, Zyklus startet neu. Vorbestehend.
- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.
- [ ] Updater ACL-Fehler: "Command plugin:updater|check not allowed by ACL". Low-Prio.

## Backlog
- [ ] [desktop] Auto-Updater funktioniert nicht bei Tester. Ursache unklar (Firewall/AV?).
- [ ] [shared] Anthropic-Provider verifizieren und ggf. wieder freischalten
- [ ] [shared] OpenAI-Provider mit echtem Key testen
- [ ] [shared] Chunking-Drift Rust vs Kotlin angleichen
- [ ] [ui] 27 Compiler-Warnings aufraumen
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [frontend] @dnd-kit aus node_modules entfernen
- [ ] [android] Long-Press-Dauer einstellbar machen (aktuell hardcoded 500ms)
- [ ] [feature] User-definierbare Transkript-Blocklist. Phase: Polish.
- [ ] [ux] Live-Preview: Whisper-Halluzinationen ("ZDF 2020") bei Stille filtern. `is_prompt_echo`-Check auf `transcribe_live_preview` anwenden. Phase: Polish.
