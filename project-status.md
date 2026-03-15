# Projektstatus

## Aktueller Stand
Version 0.4.3. Release auf GitHub (Windows). 281 Rust-Tests (alle gruen). FloatingBar redesigned. Recording-Modi Foundation implementiert: Config (4 Modi: Hold/Toggle/AutoStop/Auto), Silence Detection (RMS-basiert), Frontend-UI (Modus-Auswahl, Stille-Slider, Insert+Send Toggle). Pipeline-Integration steht noch aus.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Recording-Modi Pipeline-Integration** → Tasks 3-6: AutoStop/Auto Pipeline-Logik, Hotkey-Handler, Insert+Send (Enter nach Paste), Commands aktualisieren
2. **Background Paste (Windows)** → HWND beim Recording-Start merken, nach Cleanup dort einfuegen
3. **Clean-Mode Ueberarbeitung** → Andy findet Clean-Modus entfremdet zu sehr das Original
4. **Onboarding/Polish** → [Briefing noch zu erstellen]

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.

## Backlog
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 23 Compiler-Warnings aufraumen (dead code, private interfaces, unused imports)
- [ ] [frontend] @dnd-kit aus node_modules entfernen
- [ ] [android] Recording-Modi + Bubble Long-Press Modi-Auswahl (nach Windows-Implementierung)
