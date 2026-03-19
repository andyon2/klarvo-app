# Projektstatus

## Aktueller Stand
Version 0.4.5 (released 2026-03-19). 332 Rust-Tests (alle gruen). Android-Pipeline von 10-20s auf ~0.4s (async Turso, Config-Cache). Multi-Provider LLM Cleanup: DeepSeek, Groq, OpenAI, OpenRouter (Windows + Android). Anthropic aus UI entfernt (ungetestet, anderes API-Format). API-Key-Fallback auf beiden Plattformen.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Onboarding/Polish** → [Briefing noch zu erstellen]

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.

## Backlog
- [ ] [desktop] Auto-Updater funktioniert nicht bei Tester. latest.json + Signature korrekt. Ursache unklar (Firewall/AV?).
- [ ] [shared] Anthropic-Provider verifizieren und ggf. wieder freischalten (anderes API-Format, nie getestet)
- [ ] [shared] OpenAI-Provider mit echtem Key testen
- [ ] [shared] Chunking-Drift Rust vs Kotlin angleichen (Threshold 400/800, Join \n/\n\n)
- [ ] [ui] Startgroesse des Windows-Fensters erhoehen
- [ ] [ui] 27 Compiler-Warnings aufraumen
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [frontend] @dnd-kit aus node_modules entfernen
- [ ] [android] Silence-Threshold einstellbar machen (aktuell hardcoded 0.03 / 2s)
- [ ] [android] Long-Press-Dauer einstellbar machen (aktuell hardcoded 500ms)
- [ ] [feature] User-definierbare Transkript-Blocklist. Phase: Polish.
