# Voxlit

Freie Alternative zu Wispr Flow — Sprachdiktat mit KI-Text-Cleanup für Windows und Android.

Sprache in jedem Textfeld systemweit in bereinigten Text umwandeln. Kein Abo, keine Cloud-Abhängigkeit, alles gehört dem Nutzer.

## Inhalt

- [Downloads](#downloads)
- [Was Voxlit kann](#was-voxlit-kann)
- [Free vs. Paid](#free-vs-paid)
- [Voraussetzungen](#voraussetzungen)
- [Tech-Stack](#tech-stack)
- [Lizenz](#lizenz)
- [Feedback](#feedback)

## Downloads

➡️ **[Aktueller Release](https://github.com/andyon2/voxlit-app/releases/latest)**

- **Windows:** `.exe`-Installer herunterladen und ausführen
- **Android:** `.apk` herunterladen, "Aus unbekannten Quellen installieren" erlauben, installieren

> **Windows zeigt eine SmartScreen-Warnung beim ersten Start. Das ist normal.**
> Voxlit hat noch kein Code-Signing-Zertifikat — die kosten $180-300/Jahr. Da Voxlit source-available ist, kannst du den Code selbst prüfen bevor du die App startest. Klick auf "Weitere Informationen" → "Trotzdem ausführen". Zertifikat folgt sobald die ersten Verkäufe es finanzieren.

## Was Voxlit kann

### Kernfunktionen

- **End-to-End Diktat-Pipeline:** Aufnehmen → Transkribieren → KI-Bereinigung → Einfügen
- **3 Schreibstile:** Polished (professionell bereinigt), Verbatim (nah am Original), Chat (locker, mit Emojis)
- **Sauberer Output:** Whisper-Halluzinationen werden automatisch erkannt und entfernt — keine mysteriösen Textfragmente im Ergebnis
- **Custom Dictionary:** Fachbegriffe, Namen und Abkürzungen die STT und LLM korrekt beibehalten sollen
- **History:** Jedes Diktat wird lokal gespeichert, durchsuchbar, löschbar
- **Cross-Device Sync:** Diktat-History über Turso-Cloud zwischen Geräten synchronisieren
- **Output-Sprache wählbar:** Diktat wird via LLM in die gewünschte Sprache übersetzt
- **Cost Tracking:** Zeigt STT- und LLM-Kosten pro Diktat — Überblick was das Diktieren tatsächlich kostet
- **Eigene API-Keys:** Groq, DeepSeek, OpenAI oder OpenRouter — kein Proxy, keine Marge, kein Vendor Lock-in

### Windows

- **Paste ins richtige Fenster:** Voxlit merkt sich das aktive Fenster vor der Aufnahme und fügt das Ergebnis dort ein — egal welches Fenster gerade im Vordergrund ist
- **Insert-and-Send:** Drückt optional Enter nach dem Einfügen, um den Text direkt abzuschicken (Slack, Teams, WhatsApp Web). Pro Hotkey-Slot konfigurierbar
- **2 unabhängige Hotkey-Slots:** Jeder Slot hat eigenen Shortcut, eigenen Aufnahme-Modus und eigene Insert-and-Send-Einstellung. Slot 1 für Hold in Slack, Slot 2 für AutoStop im Dokument — kein Umkonfigurieren
- **4 Aufnahme-Modi:** Hold (halten), Toggle (an/aus), AutoStop (stoppt bei Stille), Auto-Loop (diktiert fortlaufend)
- **Floating Pill Bar:** Transparente Statusanzeige am Bildschirmrand mit Echtzeit-Waveform
- **System-Tray:** Schnellzugriff über das Tray-Icon
- **App Profiles:** Cleanup-Stil und Prompt automatisch pro App anpassen (Window-Title-Matching)
- **Command Mode:** Text selektieren, Sprachbefehl geben, LLM schreibt den Text um (experimentell)
- **Whisper Mode:** Mikrofonverstärkung für leises Diktieren
- **Offline-Modus:** Lokales Whisper mit GPU-Beschleunigung (small/medium/large-v3), kein Internet nötig

### Android

- **Floating Bubble** über allen Apps — erscheint nur wenn die Tastatur sichtbar ist (konfigurierbar: immer oder nur bei Tastatur)
- **Per-Geste konfigurierbar:** Tap und Long-Press jeweils unabhängig einstellbar mit eigenem Modus (Hold, AutoStop, Push-to-Talk, Auto-Loop) und eigener Silence-Dauer
- **5 Bubble-Zustände** mit Animationen: Idle → Recording (Waveform) → Push-to-Talk (rote Blase) → Processing (Spinner) → Done
- **Silero VAD:** Erkennt Sprechpausen automatisch — kein manuelles Stoppen nötig im AutoStop-Modus
- **Einfügen** über AccessibilityService in jedes Textfeld

## Free vs. Paid

Voxlit ist ohne Lizenzkey voll nutzbar — inklusive kostenlosem Groq-Key für STT und LLM. Mit Lizenzkey gibt es zusätzliche Power-Features. Einmalkauf, kein Abo.

| Feature | Free | Paid |
|---------|------|------|
| Diktat-Pipeline (STT + Cleanup + Paste) | ✅ | ✅ |
| Alle 3 Schreibstile | ✅ | ✅ |
| Groq (STT + LLM) + DeepSeek LLM | ✅ | ✅ |
| OpenAI STT, OpenAI/OpenRouter LLM | — | ✅ |
| Alle Aufnahme-Modi (Hold, Toggle, AutoStop, Loop) | ✅ | ✅ |
| 2 Hotkey-Slots + Insert-and-Send | ✅ | ✅ |
| Android Floating Bubble (alle Features) | ✅ | ✅ |
| Cost Tracking (Grundfunktion) | ✅ | ✅ |
| Dictionary | 20 Einträge | Unbegrenzt |
| History | 50 Einträge | Unbegrenzt + Suche |
| Lokales Whisper (Offline) | small (488 MB) | + medium, large-v3 |
| App Profiles, Command Mode, Whisper Mode | — | ✅ |
| Voice Notes, Text Snippets | — | ✅ |
| Savings-Dashboard (Wispr Flow Vergleich) | — | ✅ |
| Cross-Device Sync (Turso) | — | ✅ |
| Custom LLM System-Prompts | — | ✅ |

## Voraussetzungen

Du brauchst einen **Groq API Key** (kostenlos) — damit funktioniert Voxlit komplett: Sprache-zu-Text (Whisper) und Text-Bereinigung (Llama).

Optional: **DeepSeek API Key** (sehr günstig) für bessere Text-Bereinigung. Groq reicht aber alleine.

API-Keys werden beim ersten Start über den Einrichtungs-Wizard eingegeben, oder später in den Settings.

> **Kosten:** Groq ist kostenlos (mit Rate Limit). Mit DeepSeek ~0,001 € pro Diktat. Bei normalem Gebrauch (30-60 Diktate/Tag) unter 0,10 € pro Tag.

## Tech-Stack

| Schicht | Technologie |
|---------|-------------|
| Desktop-Framework | Tauri v2 (Rust-Backend + Web-Frontend) |
| Frontend | React + TypeScript + Tailwind CSS |
| Backend | Rust (Audio, STT, LLM, Paste, Hotkey) |
| Mobile | Tauri v2 Android + Kotlin (Floating Bubble, native Audio) |
| STT | Groq Whisper API (primär), OpenAI Whisper (Fallback) |
| Text-Cleanup | DeepSeek (primär), Groq/Llama, OpenAI, OpenRouter (konfigurierbar) |
| Speicherung | JSON (Config, Dictionary), SQLite (History, Stats) |

## Lizenz

Voxlit ist source-available unter der [Business Source License 1.1](LICENSE). Der Quellcode ist einsehbar — du kannst prüfen was die App tut. Private Nutzung und Modifikation für den Eigengebrauch sind erlaubt. Redistribution und kommerzielle Nutzung sind nicht gestattet.

## Feedback

Bugs und Wünsche gerne als [GitHub Issue](https://github.com/andyon2/voxlit-app/issues) melden.
