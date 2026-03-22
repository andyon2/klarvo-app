# Projektstatus

## Aktueller Stand
Version 0.4.8 (Windows + Android). Rename Voxlit → Klarvo geplant (naechste grosse Aenderung). Voice Command Mode geparkt. Kernfunktion (Hotkey-Diktat) stabil. Code-Hygiene abgeschlossen (0 Warnings). UI-Redesign Farbpalette gefunden: Notion Warm v4 (Teal #2AC3A8 + Orange #FFA344 auf warmen Grau-Surfaces).

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **UI-Redesign Phase 1** → Notion v4 Farbpalette in echten Code implementieren (Tailwind-Klassen umschreiben, styles.css). Briefing: `briefings/design/plan.md`
2. **Rename Voxlit → Klarvo** → Repo, Code, Tauri Config, UI, Marketing.
3. **UI-Redesign Phase 2** → Fenster Landscape (720x540), Sidebar-Navigation, Menueumstrukturierung.
4. **Neue Brand-Assets** → Social Preview, App Icon aus v4-Palette ableiten.
5. **Launch-Vorbereitung** → Landingpage, Social Preview hochladen.

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.

## Erledigt (diese Session)

- [x] Code-Hygiene: 0 Clippy-Warnings, 0 Compiler-Warnings (vorher 47+32). 424 Tests gruen.
- [x] UI-Preview-Workflow: ThemeSwitcher + Puppeteer-Screenshot-Tool (`briefings/design/screenshot-tool.ts`)
- [x] Farbpalette-Research: Superhuman, Linear, Notion, Wispr Flow analysiert
- [x] Farbpalette entschieden: Notion Warm v4 (bg #191919, primary #2AC3A8, warm #FFA344)
- [x] workflow.md: 4. Testweg (npm run preview) dokumentiert, Testanzahl 239→424

## Backlog
- [ ] [desktop] Auto-Updater funktioniert nicht bei Tester. Ursache unklar (Firewall/AV?).
- [ ] [shared] Anthropic-Provider verifizieren und ggf. wieder freischalten
- [ ] [shared] OpenAI-Provider mit echtem Key testen
- [ ] [shared] Chunking-Drift Rust vs Kotlin angleichen
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [android] Long-Press-Dauer einstellbar machen (aktuell hardcoded 500ms)
- [ ] [feature] User-definierbare Transkript-Blocklist. Phase: Polish.
- [ ] [feature] OpenRouter Modell-Dropdown in Settings (aktuell hardcoded auf deepseek/deepseek-chat).
- [ ] [feature] Reformat-Prompts (Email/Bullets/Summary) verbessern -- aktuell schlechte Qualitaet, aus README entfernt.
- [ ] [android] Bubble Size/Opacity UI-Controls implementieren (Backend-Config existiert, Frontend fehlt).
- [ ] [desktop] [paid] SAPI-basierte Command-Erkennung. Phase: Post-Launch.
