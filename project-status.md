# Projektstatus

## Aktueller Stand
Version 0.5.0 (Windows + Android). Binary `klarvo.exe`, Installer `Klarvo_0.5.0_x64-setup.exe`. License-Keys jetzt mit KLARVO-Prefix (VOXLIT/DIKTA weiterhin akzeptiert). In-App-URLs auf klarvo.app. 430 Tests gruen. Projekt-Rename (Ordner, Agents, CLAUDE.md, CSS-Vars, Event-Namen) steht noch aus.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Rename Voxlit → Klarvo (Projektdateien)** → Ordner, CLAUDE.md, Agents, knowledge/, briefings/, CSS-Var-Prefix, Event-Namen. Kein App-Rebuild noetig.
2. **UI-Redesign Phase 2** → Fenster Landscape (720x540), Sidebar-Navigation, Menueumstrukturierung. Briefing: `briefings/design/plan.md`
3. **Neue Brand-Assets** → Social Preview, App Icon aus v4-Palette ableiten.
4. **Landingpage klarvo.app** → Andy baut diese Woche eine erste Version.
5. **Launch-Vorbereitung** → Social Preview hochladen, finale Checks.

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.
- [ ] Auto-Stop/Full-Auto: Stereo-Downmix-Fix ist drin, aber noch nicht mit Stereo-Mic verifiziert.
- [ ] Build-Script: Installer-Kopie nach Dropbox nimmt alphabetisch ersten statt neuesten. Low-Prio.

## Erledigt (diese Session)

- [x] Audio Error-Handling: Ready-Channel fuer sofortige Device-Fehlererkennung, echte Fehlermeldungen statt "thread panicked", Device-Fallback auf System-Default bei unavailable Device.
- [x] STT Provider Auto-Switch: Wenn konfigurierter Cloud-Provider keinen Key hat aber anderer schon → automatisch umschalten.
- [x] API Key Management: Remove-Button (orange, 2-Klick-Bestaetigung) pro Provider, Key-Validierung beim Speichern, Whitespace-Trimming, neuer `clear_api_key` Tauri-Command.
- [x] Onboarding-Persistenz-Bug gefixt: Parameter-Name-Mismatch (`state` vs `onboardingState`) in invoke-Call verhinderte Speicherung.
- [x] Binary-Rename: `voxlit.exe` → `klarvo.exe` via `[[bin]]` in Cargo.toml.
- [x] Remove License / Remove Key Buttons einheitlich orange gestylt.
- [x] VAD Stereo-Downmix: Mono-Downmix vor Resampling im Auto-Stop-Pfad.
- [x] License KLARVO-Prefix: Generierung KLARVO-, Validierung akzeptiert KLARVO/VOXLIT/DIKTA.
- [x] License UI: voxlit.app → klarvo.app, Placeholder KLARVO-XXXX.
- [x] History-Farben: Style-Badge Teal, App-Name-Badge Orange.
- [x] Section-Headers dynamisch: Paid-Sektionen Teal wenn lizenziert, Muted+Lock wenn free.
- [x] Android Notification-Strings Voxlit → Klarvo.

## Backlog
- [ ] [desktop] Auto-Updater funktioniert nicht bei Tester. Ursache unklar (Firewall/AV?).
- [ ] [shared] Anthropic-Provider verifizieren und ggf. wieder freischalten
- [ ] [shared] Chunking-Drift Rust vs Kotlin angleichen
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [android] Long-Press-Dauer einstellbar machen (aktuell hardcoded 500ms)
- [ ] [feature] User-definierbare Transkript-Blocklist. Phase: Polish.
- [ ] [feature] OpenRouter Modell-Dropdown in Settings (aktuell hardcoded auf deepseek/deepseek-chat).
- [ ] [feature] Reformat-Prompts (Email/Bullets/Summary) verbessern -- aktuell schlechte Qualitaet, aus README entfernt.
- [ ] [android] Bubble Size/Opacity UI-Controls implementieren (Backend-Config existiert, Frontend fehlt).
- [ ] [desktop] [paid] SAPI-basierte Command-Erkennung. Phase: Post-Launch.
