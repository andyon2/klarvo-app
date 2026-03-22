# Projektstatus

## Aktueller Stand
Version 0.4.8 (Windows + Android). In-App-Name jetzt "Klarvo" (productName, Window-Titel, Tray, License-Texte, Onboarding, Build-Scripts). Projekt-Rename (Ordner, Agents, CLAUDE.md) steht noch aus. UI-Redesign Phase 1 abgeschlossen: Notion Warm v4 Palette implementiert. Voice Command Mode geparkt. 424 Tests gruen, 0 Warnings.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Rename Voxlit → Klarvo (Projektdateien)** → Ordner, CLAUDE.md, Agents, knowledge/, briefings/, CSS-Var-Prefix, Event-Namen, Crate-Name. Kein App-Rebuild noetig.
2. **UI-Redesign Phase 2** → Fenster Landscape (720x540), Sidebar-Navigation, Menueumstrukturierung. Briefing: `briefings/design/plan.md`
3. **Neue Brand-Assets** → Social Preview, App Icon aus v4-Palette ableiten.
4. **Launch-Vorbereitung** → Landingpage, Social Preview hochladen.

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.
- [ ] Onboarding: "Ich kenn mich aus" schliesst das Fenster, aber Onboarding erscheint bei jedem App-Start erneut. Persistenz-Bug.
- [ ] Auto-Stop und Full-Auto: Silence-Detection loest nicht aus, Cleanup passiert nicht.
- [x] ~~**[BLOCKER]** Android-Build: Kotlin-Versionskonflikt~~ → Fix: kotlin-gradle-plugin auf 2.2.0 gebumpt. android-build.sh patcht gen/ automatisch.

## Erledigt (diese Session)

- [x] UI-Redesign Phase 1: Notion Warm v4 Palette implementiert (441 Tailwind-Klassen, FloatingBar Inline-Styles, @theme Block). 29 Dateien geaendert.
- [x] In-App Rename Voxlit → Klarvo: Frontend (Header, Onboarding, Settings, License-Texte), Backend (Window-Titel, Tray, Registry, License-Messages), Build-Scripts (Installer/APK-Dateinamen), tauri.conf.json productName.
- [x] Cost Dashboard: Orange-Akzent fuer Kosten-Sektion (Teal=Nutzung, Orange=Kosten).
- [x] Text-Hierarchie verfeinert: Hints auf muted hochgestuft, dim nur fuer Placeholder/Deko. AdvancedSettings-Subheadings in Teal.

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
