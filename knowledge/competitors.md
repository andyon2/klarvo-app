# Competitor Analysis

Letzte Aktualisierung: 2026-03-09

## Wispr Flow

- **URL:** https://wisprflow.ai
- **Plattform:** macOS, Windows, **Android (seit Feb 2026)**
- **Preis:** Free (2000 Woerter/Woche), Pro $12/mo ($144/Jahr), Enterprise $24/user/mo
- **Student/Non-Profit:** $8/mo
- **Kernfeature:** Cloud-basierte Voice-to-Text mit proprietaerem Modell
- **Positionierung:** Premium, enterprise-ready (SOC 2 Type II, ISO 27001, HIPAA)
- **Beziehung zu Dikta:** Wispr Flow ist die direkte Inspiration. Diktats Android-Version (Floating Bubble, systemweites Paste) ist an Wispr Flows UX angelehnt.

### Android-Launch (23. Feb 2026)
- Floating Bubble UI
- Cloud-basiert, gleiches Abo-Modell
- Aktuell gratis unlimited Dictation (Promo)
- 30% schnellere Infrastruktur durch Rewrite
- Hinglish-Support (Hindi+Englisch Mix)
- Quelle: TechCrunch, 9to5Google, HotHardware

### Staerken
- Ausgereifteste UX im Markt
- Enterprise-Features (SSO/SAML, Admin-Dashboard, Compliance)
- Proprietaeres Modell (nicht auf 3rd-Party-API angewiesen)
- Jetzt auch Android (erster grosser Player auf der Plattform)

### Schwaechen aus Dikta-Sicht
- Abo-Modell ($144/Jahr) -- Subscription Fatigue
- Cloud-only (kein Offline)
- Closed Source (keine Transparenz)
- Android-Promo wird irgendwann enden -> Abo-Huerde

### Dikta-Differenzierung
Nicht ueber UX-Ueberlegenheit (Wispr hat mehr Ressourcen), sondern:
- **EUR 29 einmalig vs. $144/Jahr**
- **Offline-faehig vs. Cloud-only**
- **Open Source vs. Black Box**
- **Ownership: Daten lokal vs. Cloud-Abhaengigkeit**

---

## Amical

- **URL:** https://amical.ai
- **GitHub:** https://github.com/amicalhq/amical
- **Preis:** Gratis, Open Source (MIT)
- **Plattform:** macOS, Windows; Android + iOS in **Private Beta**
- **Framework:** TypeScript (vermutlich Electron)
- **Recherche-Datum:** 2026-03-09

### Features
- Lokale STT (Whisper) + Cloud (BYOK)
- Lokale LLMs fuer Text-Cleanup
- Context-aware Dictation (erkennt aktive App, passt Format an)
- 100+ Sprachen
- Floating Widget
- Positioniert sich als "Open Source Wispr Flow / Superwhisper / Granola Alternative"

### Staerken
- MIT-Lizenz, gleiche Vision wie Dikta (offline, open source, privacy-first)
- Context-aware Dictation (intelligentes Feature)
- Desktop bereits shipped (Win + Mac)

### Schwaechen aus Dikta-Sicht
- Android nur Private Beta (nicht shipped)
- Kein Monetarisierungsmodell (Nachhaltigkeit fraglich)
- Electron-basiert (Performance)
- GitHub-Releases nur Desktop-Versionen

### Strategische Einordnung
Naechster Open-Source-Wettbewerber. Gleiche Vision, aehnlicher Feature-Umfang.
Dikta-Vorsprung: **Android shipped**, **Bezahlprodukt mit Commitment**, **Tauri/Rust statt Electron**.
Amical beobachten -- wenn deren Android public wird, schrumpft der Vorsprung.

---

## Voice Type (Careless Whisper)

- **URL:** https://carelesswhisper.app
- **Preis:** $19.99 USD einmalig (Mac App Store)
- **Plattform:** nur macOS
- **Bewertung:** 4.8/5 (27 Reviews)
- **Entwickler:** Careless Whisper Inc.

### Kernkonzept

Komplett offline, Privacy-first Voice-to-Text. Kein Backend, keine Telemetrie, kein Cloud-Zwang. Positioniert sich als HIPAA-ready Alternative zu Wispr Flow.

### Features

- Offline Voice-to-Text (macOS-native Speech Recognition)
- Hold-to-Dictate Hotkey
- Funktioniert in allen Textfeldern systemweit
- Optional: BYO-LLM (Bring your own key) fuer Rewrites (OpenAI, Groq etc.)
- Kein Proxy -- Nutzer kommuniziert direkt mit LLM-Provider
- 3-Schritt-Setup: Install -> Hotkey -> Go

### Enterprise-Positionierung

- HIPAA-ready im Offline-Modus (kein BAA noetig)
- SOC 2: Keine Kundendaten-Verarbeitung, minimaler Questionnaire
- Security Whitepaper auf Anfrage

### Strategische Einordnung
Voice Type zeigt: Offline + Einmalkauf + Privacy funktioniert als Geschaeftsmodell.
Dikta bietet 5x mehr Features und laeuft auf 2 Plattformen statt nur macOS.

---

## OpenWhispr

- **URL:** https://openwhispr.com
- **GitHub:** https://github.com/OpenWhispr/openwhispr (~1640 Stars)
- **Preis:** Gratis, Open Source (MIT)
- **Plattform:** macOS, Windows, Linux
- **Framework:** Electron
- **Recherche-Datum:** 2026-03-09

### Features (v1.6.0)
- Lokale STT (Whisper GGML, NVIDIA Parakeet) + Cloud (BYOK)
- Multi-Provider LLM (OpenAI, Anthropic, Gemini, Groq, Mistral, Local)
- Agent Mode mit Streaming Chat Overlay
- Google Calendar Integration (automatische Meeting-Erkennung)
- Live Meeting-Transkription (OpenAI Realtime API)
- Notes-System mit Volltextsuche + Cloud Sync
- Cross-Platform (macOS, Windows, Linux)

### Staerken
- MIT-Lizenz, grosse Community
- Breites Feature-Set (Agent Mode, Meeting-Transkription)
- Linux-Support
- Kein Vendor Lock-in

### Schwaechen aus Dikta-Sicht
- Electron (schwergewichtiger als Tauri/Rust)
- Kein Android-Support
- Community-getrieben (kein garantierter Support/Roadmap)
- Kein Bezahl-Modell (Nachhaltigkeit?)

### Strategische Einordnung
Nicht als direkter Wettbewerber behandeln, sondern **komplementaer positionieren:**
OpenWhispr = Open-Source-Projekt zum Mitbauen (Tinkerer/Entwickler).
Dikta = Fertiges Produkt das einfach funktioniert (End-User).
Kein Feature-Race gegen MIT-Community fuehren.

---

## Voicy

- **URL:** https://usevoicy.com
- **Preis:** $8.49/mo, Jahresabo mit 20% Rabatt, Lifetime $220
- **Plattform:** macOS, Windows, Browser-Extension
- **Recherche-Datum:** 2026-03-09

### Features
- 99%+ Accuracy in 50+ Sprachen
- Automatische Interpunktion
- Funktioniert in allen Websites und Apps
- Privacy: Nur der Nutzer sieht Transkripte

### Strategische Einordnung
Voicy zeigt die Preis-Obergrenze fuer Lifetime-Deals ($220).
Dikta positioniert sich mit EUR 29 deutlich darunter -- aggressiverer Einstieg.

---

## Markt-Uebersicht (Stand Maerz 2026)

| Tool | Preis | Plattform | Offline | Open Source |
|------|-------|-----------|---------|-------------|
| **Dikta** | EUR 29 einmalig (geplant) | Windows, Android | Ja (geplant) | Ja |
| Wispr Flow | $12/mo ($144/Jahr) | macOS, Windows, **Android** | Nein | Nein |
| Amical | Gratis (MIT) | macOS, Windows, Android (Beta) | Ja | Ja |
| Voice Type | $19.99 einmalig | nur macOS | Ja | Nein |
| OpenWhispr | Gratis (MIT) | macOS, Windows, Linux | Ja | Ja |
| Voicy | $8.49/mo oder $220 Lifetime | macOS, Windows, Browser | Nein | Nein |
| Dragon | $14.99/mo oder $700 | Windows, macOS | Ja | Nein |

## Beobachtungsliste

Regelmaessig pruefen (alle 4-6 Wochen):
- [ ] **Amical Android-Beta:** Wird sie public? Feature-Umfang? Monetarisierung?
- [ ] **Wispr Flow Android Pricing:** Aktuell Free Promo -- wann kommt das Abo?
- [ ] **OpenWhispr Mobile:** Anzeichen fuer Android-Support?
- [ ] **Neue Wettbewerber:** Handy (Tauri-basiert, kein Android), NotelyVoice (Compose Multiplatform, kein Windows)

---

## dikta (rohe.ai) -- Namenskonflikt

- **URL:** dikta.net
- **App Store:** apps.apple.com/us/app/dikta-ai-voice-keyboard/id6759268402
- **Entwickler:** rohe technik OÜ (rohe.ai)
- **Plattform:** iOS
- **Preis:** Abo (dikta pro monthly / yearly)
- **Recherche-Datum:** 2026-03-21
- **Status:** Kaum Bewertungen im App Store, relativ neu
- **Relevanz:** Namenskonflikt. Gleicher Name, gleicher Markt (AI Voice Keyboard). Kein direkter Produkt-Wettbewerber (nur iOS, Abo).

---

## dIKta.me -- Namenskonflikt + direkter Wettbewerber

- **URL:** dikta.me
- **GitHub:** github.com/geckogtmx/diktame
- **Plattform:** nur Windows (10 2004+, 11)
- **Preis:** $20 Einmalkauf (Power Version), Free Trial mit Credits
- **Lizenz:** "Source-Available" auf Website, aber kein Lizenzfile im Repo (rechtlich unklar)
- **Tech-Stack:** C# / .NET 8, WinUI 3 (Fluent Design), SQLite
- **Entwickler:** Solo-Entwickler, 246 Commits. Vorheriges Projekt war Python + Electron, dann nativer Rewrite.
- **Recherche-Datum:** 2026-03-21

### Features (detailliert)
- 7 Workflow-Modi: Dictate, Refine, Ask, Translate, Oops, Note, Read Selection
- Cloud STT (Deepgram, Gemini) + lokal (Whisper.net mit Vulkan GPU)
- TTS: Read Selection mit Kokoro-ONNX Fallback
- LLM: Gemini, Anthropic, OpenAI, Ollama (eingebettetes Management)
- Dual-Profile-System mit 16 Custom Prompts
- Voice Snippets (Trigger-basierte Makro-Expansion)
- Quick Chat (Floating LLM-Overlay)
- 90-Tage SQLite-Historie
- Sicherheit: DPAPI-Secrets, PII-Scrubber, API-Key-Validierung
- ~173MB unkomprimiert, ~70MB komprimiert

### Qualitaet
- 950 Tests (479 in CI), 74% Line-Coverage, 52% Branch-Coverage
- 0 Fehler, 0 Warnungen im Release-Build
- CI/CD Pipeline (Lint → Build → Test → Publish)

### Staerken
- Professionelle Codequalitaet (Tests, Coverage, CI/CD)
- Natives Windows-UI (WinUI 3, kein Electron)
- 7 Modi vs. unsere 3-4
- TTS (Read Selection) -- Feature das wir nicht haben
- Ollama eingebettet mit Health-Checks und Model-Library

### Schwaechen aus unserer Sicht
- **Nur Windows** -- kein Android, kein Mobile
- **Distribution noch nicht fertig** -- Installer steht aus, kein Auto-Updater
- **Kein Lizenzfile im Repo** -- "Source-Available" ohne rechtliche Grundlage
- **.NET 8** -- schwergewichtiger als Tauri/Rust
- **Credit/Wallet-System geplant** -- geht Richtung Mikrotransaktionen, weg vom klaren Einmalkauf

### Strategische Einordnung
Naechster direkter Wettbewerber nach Amical. Validiert unser Geschaeftsmodell (Einmalkauf + Source-Available + Voice Dictation). Aber: nur Windows, Distribution nicht shipped, kein Mobile. Unser Vorsprung: Android shipped + Auto-Updater + signierte Builds + echte Tester. Namenskonflikt macht Umbenennung zwingend.

## Fazit

Der Markt hat sich seit Anfang 2026 bewegt: Wispr Flow ist auf Android, Amical naehert sich
mit Mobile-Beta. Diktats Differenzierung laeuft nicht mehr ueber "einziger auf Android",
sondern ueber drei Achsen:
1. **Einmalkauf** (EUR 29) vs. Abo (Wispr Flow) oder kein Modell (Amical/OpenWhispr)
2. **Shipped Android** vs. Beta (Amical) oder Cloud-only (Wispr Flow)
3. **Offline + Open Source + Bezahlprodukt** = nachhaltig UND transparent
