# Projektstatus

## Aktueller Stand
Version 0.4.3. 326 Rust-Tests (alle gruen). Background Paste mit Verify-or-Clipboard-Only: Text wird nur eingefuegt wenn Zielfenster verifiziert, sonst Zwischenablage + FloatingBar-Hinweis. Insert+Send pro Hotkey-Slot statt global. Return-to-Current bei Autosend (zurueck zum Fenster wo User gerade war).

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Clean-Mode Ueberarbeitung** → Andy findet Clean-Modus entfremdet zu sehr das Original
2. **Onboarding/Polish** → [Briefing noch zu erstellen]

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.

## Backlog
- [ ] [ui] Startgroesse des Windows-Fensters erhoehen — zu klein beim Oeffnen, Settings haben Scrollbalken rechts und unten
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 27 Compiler-Warnings aufraumen (dead code, private interfaces, unused imports, unused BOOL)
- [ ] [frontend] @dnd-kit aus node_modules entfernen
- [ ] [android] Recording-Modi via Notification-Actions implementiert (Hold/Toggle/AutoStop/Auto). Offen:
  - [ ] Silence-Threshold in Android-App einstellbar machen (aktuell hardcoded 0.03 / 2s)
  - [ ] Long-Press-Dauer einstellbar machen (aktuell hardcoded 500ms)
  - [ ] Auto-Modus: nur einfuegen (insert), nicht senden (kein Enter/Send nach Paste)
- [ ] [feature] User-definierbare Transkript-Blocklist: Phrasen, die immer aus dem Transkript entfernt werden sollen (z.B. wiederkehrende Whisper-Artefakte). Phase: Polish.
