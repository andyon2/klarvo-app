# Voice Command Mode -- Brainstorming & Entscheidungen

## Datum: 2026-03-21

## Kernidee

Hands-free Diktat-Aktivierung: User braucht keinen Hotkey mehr. Voxlit lauscht per Mic
auf ein Trigger-Wort ("Voxlit") und fuehrt Voice Commands aus.

## Entschiedener Scope

### Aufnehmen (Stufe 1+2)

| Voice Command | Aktion |
|--------------|--------|
| "Voxlit dictate" / "Voxlit start" | Diktat starten |
| "Voxlit stop" | Diktat stoppen + verarbeiten |
| "Voxlit cancel" | Diktat abbrechen (nicht einfuegen) |
| "Voxlit polished/verbatim/chat" | Cleanup-Stil wechseln |
| "Voxlit off" | Voice Command Mode beenden |

Punctuation ("Punkt", "Komma", "Neuer Absatz") gehoert in den LLM-Cleanup-Prompt, nicht
in die Command-Schicht. Wird als Prompt-Tweak nebenbei mitgenommen.

### Explizit ausgeschlossen

| Was | Warum |
|-----|-------|
| Text-Manipulation ("loesch das", "ersetze X") | Fragilitaet zu hoch, DictationBuffer noetig, User tippt dazwischen = Buffer ungueltig |
| System-Steuerung ("oeffne Chrome") | Scope-Creep, Sicherheitsbedenken |
| App-spezifische Commands | Fass ohne Boden |

## Architektur-Skizze

```
Toggle ein ──► Mic offen → VAD lauscht → Sprache erkannt?
(Hotkey/Tray)      ▲              │
                   │       Kurzes Snippet (~1-2 Sek)
                   │              │
                   │     Local whisper.cpp (small model)
                   │              │
                   │     Command erkannt?
                   │       │            │
                   │      Nein          Ja → Execute
                   │       │
                   └───────┘ (weiter lauschen)
```

### Neue Komponenten

1. **Monitor-Modus** im Audio-Modul (Mic offen, VAD lauscht, kein WAV)
2. **Command-Erkennung** (VAD triggert → Snippet an whisper.cpp → String-Match)
3. **Command-Dispatch** (Command → bestehende Tauri-Actions)
4. **UI Toggle** (Tray + Hotkey fuer Voice Command Mode ein/aus)

### Vorhandenes Fundament

- VAD (Silero v5): Komplett implementiert, Desktop + Android
- whisper.cpp: Integriert (Offline-Modus)
- cpal Audio-Capture: Laeuft, braucht nur Monitor-Modus

## Abhaengigkeit: VAD-Overhaul erst abschliessen

Voice Command Mode baut auf VAD auf. Zwei Tasks noch offen:
- Task 4: Hallucination-Blocklist (WICHTIG -- Phantom-Texte koennten Phantom-Commands ausloesen)
- Task 6: Manueller Test

Reihenfolge: VAD abschliessen → Voice Command Mode planen → implementieren.

## Risiken

- **Wake Word Erkennung:** "Voxlit" ist ein Kunstwort. Whisper koennte es als "Foxy", "Foxlit", "Box lit" transkribieren. Loesung: Fuzzy Matching oder konfigurierbar.
- **Fehlausloesungen:** TV, Gespraeche → VAD triggert → Whisper hoert Phantomwort. Loesung: Hallucination-Blocklist + Confidence-Threshold.
- **Batterie-Drain:** Mic dauerhaft offen. Loesung: Nur am Strom oder User-Entscheidung.
- **Mic-Blockierung:** Shared-mode capture (cpal default) sollte funktionieren.
