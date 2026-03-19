# Projektstatus

## Aktueller Stand
Version 0.4.5 (released 2026-03-19). 331 Rust-Tests (alle gruen). Alle 4 Android-Audio-Bugs gefixt (Auto-Mode stoppbar, wait-for-speech Silence-Detection, Pipeline-Latenz async Turso + Config-Cache, robuster Accessibility-Check). API-Key-Fallback: LLM-Provider wechselt automatisch auf verfuegbaren Provider wenn konfigurierter keinen Key hat.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Onboarding/Polish** → [Briefing noch zu erstellen]

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.

## Backlog
- [ ] [desktop] Auto-Updater funktioniert nicht bei Tester (v0.4.3). latest.json + Signature korrekt. Ursache unklar (Firewall/AV?). Tester laedt manuell von GitHub.
- [ ] [ui] Startgroesse des Windows-Fensters erhoehen — zu klein beim Oeffnen, Settings haben Scrollbalken rechts und unten
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 27 Compiler-Warnings aufraumen (dead code, private interfaces, unused imports, unused BOOL)
- [ ] [frontend] @dnd-kit aus node_modules entfernen
- [ ] [android] Silence-Threshold in Android-App einstellbar machen (aktuell hardcoded 0.03 / 2s)
- [ ] [android] Long-Press-Dauer einstellbar machen (aktuell hardcoded 500ms)
- [ ] [feature] User-definierbare Transkript-Blocklist: Phrasen, die immer aus dem Transkript entfernt werden sollen (z.B. wiederkehrende Whisper-Artefakte). Phase: Polish.
