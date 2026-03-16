# Projektstatus

## Aktueller Stand
Version 0.4.3. 311 Rust-Tests (alle gruen). Dual-Hotkey-System fertig: 2 konfigurierbare Hotkey-Slots, jeder mit eigenem Recording-Modus. Tab-UI in Settings (Hotkey 1 / Hotkey 2). Config-Migration von altem Single-Hotkey-Format. Dispatch via on_shortcuts() mit Shortcut-ID-Map.

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
- [ ] [android] Recording-Modi + Bubble Long-Press Modi-Auswahl (nach Windows-Implementierung)
