# Projektstatus

## Aktueller Stand
Version 0.4.3. 300 Rust-Tests (alle gruen). Recording-Modi Pipeline-Integration fertig: AutoStop (silence-basiert), Auto-Loop (endlos mit Restart), Insert+Send (Enter nach Paste). Whisper-Halluzinations-Guard (Prompt-Echo-Erkennung). FloatingBar mit Dikta-Logo und Modus-Badge. Hotkey wird beim Shortcut-Konfigurieren pausiert. Silence Detection wartet auf Speech bevor sie zaehlt.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Dual-Hotkey-System** → 2 konfigurierbare Hotkey-Slots, jeder mit eigenem Modus (Andy-Wunsch)
2. **Background Paste (Windows)** → HWND beim Recording-Start merken, nach Cleanup dort einfuegen
3. **Clean-Mode Ueberarbeitung** → Andy findet Clean-Modus entfremdet zu sehr das Original
4. **Onboarding/Polish** → [Briefing noch zu erstellen]

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.
- [ ] Whisper-Halluzination "proper punctuation" taucht gelegentlich in der Bar auf (wird gefiltert, kein Effekt aufs Transkript). Low-Prio.

## Backlog
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 23 Compiler-Warnings aufraumen (dead code, private interfaces, unused imports)
- [ ] [frontend] @dnd-kit aus node_modules entfernen
- [ ] [android] Recording-Modi + Bubble Long-Press Modi-Auswahl (nach Windows-Implementierung)
