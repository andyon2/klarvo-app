# Projektstatus

## Aktueller Stand
Version 0.4.4. 328 Rust-Tests (alle gruen). Cleanup-Modi ueberarbeitet (Verbatim/Polished/Chat mit verbesserter Worttreue, Code-Switching-Schutz, Anti-Substitution). UI-Label "Clean" → "Verbatim". Android: Bubble Controls Settings-Sektion (Tap/Long Press unabhaengig konfigurierbar), Notification-Actions fuer Mode-Switching entfernt, performEnter() im AccessibilityService.

## Blocker

Keine.

## Naechste Sessions (in Reihenfolge)

1. **Android Audio-Bugs fixen** → Silence-Detection, Auto-Mode-Loop, API-Latenz (siehe Bekannte Bugs)
2. **Onboarding/Polish** → [Briefing noch zu erstellen]

## Bekannte Bugs

- [ ] FloatingBar: Drag nur moeglich waehrend Recording/Processing (Bar im Idle hidden). Low-Prio.
- [ ] [android] Auto-Mode laesst sich nicht stoppen: Tap/LongPress waehrend PROCESSING werden ignoriert. User kann Loop nur beenden wenn Bubble gerade NICHT im Cleanup ist. Fix: Tap waehrend PROCESSING soll `autoLoopActive = false` setzen.
- [ ] [android] Auto-Mode greift Hintergrundgeraeusche auf: Silence-Detection zaehlt sofort los ohne auf Speech zu warten. Windows hat den Fix (wait-for-speech, Commit 5f9660e), Android nicht. DiktaAudioRecorder braucht gleiche Logik.
- [ ] [android] Cleanup dauert 10-20 Sekunden (Groq STT + DeepSeek ueber WLAN). Ursache unklar — koennte API-Latenz oder Overhead durch mehrfache Config-Reads sein. Muss profiled werden.
- [ ] [android] Accessibility-Permission-Popup erscheint bei jedem App-Start obwohl Service bereits aktiviert. `isAccessibilityServiceEnabled()` Check fehlerhaft.

## Backlog
- [ ] [desktop] Auto-Updater funktioniert nicht bei Tester (v0.4.3). latest.json + Signature korrekt. Ursache unklar (Firewall/AV?). Tester laedt manuell von GitHub.
- [ ] [ui] Startgroesse des Windows-Fensters erhoehen — zu klein beim Oeffnen, Settings haben Scrollbalken rechts und unten
- [ ] [shared] Integrationen: Notion, Todoist (Platzhalter in Advanced Settings)
- [ ] [ui] 27 Compiler-Warnings aufraumen (dead code, private interfaces, unused imports, unused BOOL)
- [ ] [frontend] @dnd-kit aus node_modules entfernen
- [ ] [android] Silence-Threshold in Android-App einstellbar machen (aktuell hardcoded 0.03 / 2s)
- [ ] [android] Long-Press-Dauer einstellbar machen (aktuell hardcoded 500ms)
- [ ] [feature] User-definierbare Transkript-Blocklist: Phrasen, die immer aus dem Transkript entfernt werden sollen (z.B. wiederkehrende Whisper-Artefakte). Phase: Polish.
