# Klarvo — Projektuebersicht

Generiert: 2026-04-13 | Projektversion: 0.5.0

---

## Zusammenfassung

**Klarvo** ist eine Desktop- und Mobile-App fuer Sprachdiktat mit KI-Text-Cleanup. Die App wandelt gesprochene Sprache in bereinigten Text um und fuegt diesen systemweit in jedes Textfeld ein — ohne Abo, ohne Cloud-Abhaengigkeit.

**Ehemaliger Name:** Voxlit (umbenannt zu Klarvo im Maerz 2026, Zielmarkt DACH)

**Positionierung:** Freie Alternative zu Wispr Flow. Einmalkauf (EUR 29) statt Abo (EUR 130/Jahr). Source-available unter BSL 1.1.

## Kernfunktionen

### End-to-End Diktat-Pipeline
1. **Aufnehmen** — Mikrofon-Capture (cpal Desktop / native Android)
2. **Transkribieren** — Cloud-STT (Groq Whisper, OpenAI) oder Offline (whisper.cpp)
3. **KI-Bereinigung** — LLM-Cleanup in 3 Stilen (Polished, Verbatim, Chat)
4. **Einfuegen** — Systemweites Paste (Win32 Desktop / AccessibilityService Android)

### Plattform-spezifisch

**Windows Desktop:**
- 2 unabhaengige Hotkey-Slots mit je eigenem Modus
- 4 Aufnahme-Modi: Hold, Toggle, AutoStop, Auto-Loop
- Floating Pill Bar mit Echtzeit-Waveform
- App Profiles (Cleanup-Stil pro Fenster)
- Insert-and-Send (optionales Enter nach Paste)
- Offline-Modus: Lokales Whisper + llama.cpp

**Android:**
- Floating Bubble ueber allen Apps (Tastatur-getriggert oder permanent)
- Per-Geste konfigurierbar (Tap/Long-Press unabhaengig)
- Silero VAD fuer automatische Sprechpausen-Erkennung
- Paste via AccessibilityService
- Banking-App-Erkennung (Bubble wird automatisch ausgeblendet)

### Cross-Platform
- Custom Dictionary (Fachbegriffe, Namen)
- History mit Volltextsuche
- Cross-Device Sync via Turso
- Cost Tracking (STT + LLM Kosten pro Diktat)
- BYOK: Eigene API-Keys (Groq, DeepSeek, OpenAI, OpenRouter, Anthropic)
- Lizenz-Gating: Free (Basis) vs. Paid (Power-Features)

## Technologie-Stack

| Schicht | Technologie | Version |
|---------|------------|---------|
| Desktop-Framework | Tauri v2 | 2.x |
| Frontend | React + TypeScript | 19.1 / 5.8 |
| Build-Tool | Vite | 7.x |
| CSS | Tailwind CSS | 4.x |
| Backend | Rust | 2021 Edition |
| Async Runtime | Tokio | 1.x |
| HTTP Client | Reqwest | 0.12 |
| Datenbank (lokal) | rusqlite (SQLite) | 0.32 |
| Datenbank (sync) | Turso (libsql) | HTTP API v2 |
| Audio Capture | cpal | 0.15 |
| Voice Activity Detection | Silero VAD | via voice_activity_detector 0.2.1 |
| Offline STT | whisper-rs (whisper.cpp) | 0.15.1 |
| Offline LLM | llama-cpp-2 (Windows) / MNN (Android) | 0.1.140 |
| Clipboard | arboard | 3.x |
| Mobile Framework | Tauri v2 Android + Kotlin | — |
| Android VAD | android-vad (Silero) | JitPack |
| JNI (Android LLM) | MNN C++ Wrapper | — |
| Lizenz | HMAC-SHA256 + Lemon Squeezy API | — |
| Auto-Updater | tauri-plugin-updater | 2.x |
| Logging | tauri-plugin-log | 2.x |

## Architektur-Typ

- **Repository:** Monolith (eine Codebase, ein Repository)
- **Pattern:** Desktop-App mit IPC-Bridge (Tauri Commands)
- **Plattformen:** Windows, Android (Linux experimentell)
- **Drei-Repo-Architektur:**
  - `klarvo` (dieses Repo) — App-Code
  - `klarvo-website` — Marketing-Website (Astro + Tailwind, Cloudflare Pages)
  - `teams/klarvo` — Agent-Infrastruktur, Knowledge, Briefings

## Lizenz-Modell

- **Source-available** unter Business Source License 1.1
- **Free Tier:** Volle Diktat-Pipeline, Groq+DeepSeek, 20 Dictionary-Eintraege, 50 History-Eintraege
- **Paid Tier (EUR 29 Einmalkauf):** Unbegrenztes Dictionary/History, OpenAI/OpenRouter, App Profiles, Command Mode, Offline-Modus, Cross-Device Sync, Custom Prompts

## Status

- **Aktuelle Version:** 0.5.0
- **Phase:** Early Access (Phase 0 — Soft Start)
- **Naechste Schritte:** Offline-STT-Qualitaetstest, Onboarding-Rewrite, Lizenz-Integration (Lemon Squeezy)

## Weitergehende Dokumentation

- [Source-Tree-Analyse](./source-tree-analysis.md)
- [v1-Architektur-Snapshot](./v1-architecture-snapshot.md)
- [Komponenteninventar](./component-inventory.md)
- [Entwicklungs-Guide](./development-guide.md)
