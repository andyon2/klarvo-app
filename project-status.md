# Projektstatus

## Aktueller Stand
Version 0.4.8 (Windows + Android). v0.4.8 auf GitHub released. Build-Signing via `rsign` aus WSL (`scripts/sign-installer.sh`), in `sync-and-build.ps1` integriert. Produktname wird Voxlit (Domain `voxlit.app` gesichert). Rename-Plan mit 12 Tasks in 3 Phasen erstellt (`briefings/rename-plan.md`). Landingpage-Briefing fertig (`briefings/product-strategist-landingpage.md`).

## Blocker

- **Umbenennung Voxlit → Voxlit:** Domain voxlit.app gesichert. Codebase-Rename noch nicht durchgefuehrt (12-Task-Plan bereit). Markenanmeldung DPMA vor Paid Launch.

## Naechste Sessions (in Reihenfolge)

1. **Codebase-Rename Voxlit → Voxlit** → 12 Tasks, 3 Phasen. Plan: `briefings/rename-plan.md`. 6 Vorab-Entscheidungen offen (Repo-Pfade, GitHub-Repo-Name, Social Preview).
2. **Live-Preview als Opt-In** → Nach VAD-Overhaul moeglich. Whisper-Halluzinationen werden gefiltert.
3. **Launch-Vorbereitung** → Landingpage bauen (Briefing fertig), Release-Skill aktualisiert.

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
