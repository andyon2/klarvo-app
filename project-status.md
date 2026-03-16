# Projektstatus

## Aktueller Stand
Version 0.4.3. 311 Rust-Tests (alle gruen). Android: 4 Recording-Modi (Hold/Toggle/AutoStop/Auto) via Notification-Actions umschaltbar. Silence-Detection im AudioRecorder. Long-Press = PTT in allen Modi. Direkte Modus-Buttons in Notification (3 nicht-aktive Modi sichtbar).

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Background Paste (Windows)** → HWND beim Recording-Start merken, nach Cleanup dort einfuegen
2. **Clean-Mode Ueberarbeitung** → Andy findet Clean-Modus entfremdet zu sehr das Original
3. **Onboarding/Polish** → [Briefing noch zu erstellen]

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.
- [ ] Whisper-Halluzination "proper punctuation" taucht gelegentlich in der Bar auf (wird gefiltert, kein Effekt aufs Transkript). Low-Prio.

## Backlog
- [ ] [ui] Startgroesse des Windows-Fensters erhoehen — zu klein beim Oeffnen, Settings haben Scrollbalken rechts und unten
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 23 Compiler-Warnings aufraumen (dead code, private interfaces, unused imports)
- [ ] [frontend] @dnd-kit aus node_modules entfernen
- [ ] [android] Recording-Modi via Notification-Actions implementiert (Hold/Toggle/AutoStop/Auto). Offen:
  - [ ] Silence-Threshold in Android-App einstellbar machen (aktuell hardcoded 0.03 / 2s)
  - [ ] Long-Press-Dauer einstellbar machen (aktuell hardcoded 500ms)
  - [ ] Auto-Modus: nur einfuegen (insert), nicht senden (kein Enter/Send nach Paste)
