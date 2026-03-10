# Dikta

Freie Alternative zu Wispr Flow — Sprachdiktat mit KI-Text-Cleanup für Windows und Android.

Sprache in jedem Textfeld systemweit in bereinigten Text umwandeln. Kein Abo, keine Cloud-Abhängigkeit, alles gehört dem Nutzer.

## Downloads

➡️ **[Aktueller Release](https://github.com/andyon2/dikta-public/releases/latest)**

- **Windows:** `.exe`-Installer herunterladen und ausführen
- **Android:** `.apk` herunterladen, "Aus unbekannten Quellen installieren" erlauben, installieren

## Was Dikta kann

- **End-to-End Diktat-Pipeline:** Aufnehmen → Transkribieren → Bereinigen → Einfügen ins aktive Fenster
- **3 Schreibstile:** Polished (bereinigt), Verbatim (wörtlich), Chat (locker)
- **Live Preview:** Echtzeit-Transkript während der Aufnahme
- **Live-Übersetzung:** Output-Sprache konfigurierbar (13 Sprachen)
- **Custom Dictionary:** Fachbegriffe, die STT und LLM beibehalten sollen
- **App Profiles:** Stil/Sprache/Prompt pro App automatisch anpassen
- **History + Volltextsuche:** Vergangene Diktate durchsuchen
- **Voice Notes:** Aufnahmen speichern statt einfügen
- **Command Mode:** Text selektieren, Sprachbefehl geben (Strg+Shift+E)
- **Whisper Mode:** Verstärkung für leises Diktieren
- **Multi-Provider:** STT und LLM-Provider frei konfigurierbar (Groq, OpenAI, DeepSeek, Anthropic)

### Windows
- Globaler Hotkey (Hold oder Toggle, konfigurierbar)
- Floating Bar am Bildschirmrand (zeigt Aufnahme-Status)
- System-Tray mit Schnellzugriff
- Automatisches Einfügen per Ctrl+V in jedes Textfeld

### Android
- Floating Bubble über allen Apps
- Tap = Aufnahme starten/stoppen, Long-Press = Push-to-Talk
- Einfügen über AccessibilityService in jedes Textfeld

## Voraussetzungen

Dikta nutzt Cloud-APIs für Transkription und Text-Bereinigung. Du brauchst mindestens:

1. **Groq API Key** (kostenlos) — für Sprache-zu-Text (Whisper)
2. **DeepSeek API Key** (sehr günstig) — für Text-Bereinigung

API-Keys werden beim ersten Start über den Einrichtungs-Wizard eingegeben, oder später in den Settings.

> **Kosten:** Bei normalem Gebrauch (30-60 Diktate/Tag) unter 0,10 € pro Tag. Groq hat ein großzügiges Free Tier, DeepSeek kostet ~0,001 € pro Diktat.

## Tech-Stack

| Schicht | Technologie |
|---------|-------------|
| Desktop-Framework | Tauri v2 (Rust-Backend + Web-Frontend) |
| Frontend | React + TypeScript + Tailwind CSS |
| Backend | Rust (Audio, STT, LLM, Paste, Hotkey) |
| Mobile | Tauri v2 Android + Kotlin (Floating Bubble, native Audio) |
| STT | Groq Whisper API (primär), OpenAI Whisper (Fallback) |
| Text-Cleanup | DeepSeek (primär), OpenAI, Anthropic, Groq/Llama (konfigurierbar) |
| Speicherung | JSON (Config, Dictionary), SQLite (History, Stats) |

## Selbst bauen

**Voraussetzungen:** Node.js, Rust/Cargo, Tauri v2 CLI

```bash
# Dependencies installieren
npm install

# .env mit API-Keys anlegen (siehe .env.example)
cp .env.example .env

# Entwicklungsserver starten
npm run tauri dev

# Release-Build (Windows)
npm run tauri build
```

**Android-Build** (aus WSL2):
```bash
scripts/android-build.sh
```

## Lizenz

Noch nicht festgelegt. Der Quellcode ist öffentlich einsehbar.

## Feedback

Bugs und Wünsche gerne als [GitHub Issue](https://github.com/andyon2/dikta-public/issues) melden.
