# Competitor Analysis

Letzte Aktualisierung: 2026-03-25

## Wispr Flow

- **URL:** https://wisprflow.ai
- **Plattform:** macOS, Windows, **Android (seit Feb 2026)**, **iOS (seit Mitte 2025, Custom Keyboard Extension)**
- **Preis:** Free (2000 Woerter/Woche), Pro $12/mo ($144/Jahr), Enterprise $24/user/mo
- **Student/Non-Profit:** $8/mo
- **Kernfeature:** Cloud-basierte Voice-to-Text mit proprietaerem Modell
- **Positionierung:** Premium, enterprise-ready (SOC 2 Type II, ISO 27001, HIPAA)
- **Beziehung zu Voxlit:** Wispr Flow ist die direkte Inspiration. Diktats Android-Version (Floating Bubble, systemweites Paste) ist an Wispr Flows UX angelehnt.

### Android-Launch (23. Feb 2026)
- Floating Bubble UI
- Cloud-basiert, gleiches Abo-Modell
- Aktuell gratis unlimited Dictation (Promo)
- 30% schnellere Infrastruktur durch Rewrite
- Hinglish-Support (Hindi+Englisch Mix)
- Quelle: TechCrunch, 9to5Google, HotHardware

### iOS-Ansatz (recherchiert 2026-03-25)
- Custom Keyboard Extension (Apples offizieller Weg fuer Third-Party-Keyboards)
- Installiert sich als zusaetzliche Tastatur neben der Apple-Tastatur
- "Start Flow" oeffnet kurz die Haupt-App fuer Mikrofon-Zugang, springt dann zurueck
- Flow Sessions mit konfigurierbarer Dauer (5 min, 15 min, 1h, unbegrenzt)
- **Relevanz fuer Klarvo:** Beweis dass Voice Dictation via iOS Keyboard Extension im App Store akzeptiert wird. Praezedenzfall fuer Klarvos iOS-Roadmap (v2.0).

### Staerken
- Ausgereifteste UX im Markt
- Enterprise-Features (SSO/SAML, Admin-Dashboard, Compliance)
- Proprietaeres Modell (nicht auf 3rd-Party-API angewiesen)
- Jetzt auch Android (erster grosser Player auf der Plattform)
- iOS via Custom Keyboard Extension (alle 4 grossen Plattformen abgedeckt)

### Schwaechen aus Voxlit-Sicht
- Abo-Modell ($144/Jahr) -- Subscription Fatigue
- Cloud-only (kein Offline)
- Closed Source (keine Transparenz)
- Android-Promo wird irgendwann enden -> Abo-Huerde

### Voxlit-Differenzierung
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
- MIT-Lizenz, gleiche Vision wie Voxlit (offline, open source, privacy-first)
- Context-aware Dictation (intelligentes Feature)
- Desktop bereits shipped (Win + Mac)

### Schwaechen aus Voxlit-Sicht
- Android nur Private Beta (nicht shipped)
- Kein Monetarisierungsmodell (Nachhaltigkeit fraglich)
- Electron-basiert (Performance)
- GitHub-Releases nur Desktop-Versionen

### Strategische Einordnung
Naechster Open-Source-Wettbewerber. Gleiche Vision, aehnlicher Feature-Umfang.
Voxlit-Vorsprung: **Android shipped**, **Bezahlprodukt mit Commitment**, **Tauri/Rust statt Electron**.
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
Voxlit bietet 5x mehr Features und laeuft auf 2 Plattformen statt nur macOS.

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

### Schwaechen aus Voxlit-Sicht
- Electron (schwergewichtiger als Tauri/Rust)
- Kein Android-Support
- Community-getrieben (kein garantierter Support/Roadmap)
- Kein Bezahl-Modell (Nachhaltigkeit?)

### Strategische Einordnung
Nicht als direkter Wettbewerber behandeln, sondern **komplementaer positionieren:**
OpenWhispr = Open-Source-Projekt zum Mitbauen (Tinkerer/Entwickler).
Voxlit = Fertiges Produkt das einfach funktioniert (End-User).
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
Voxlit positioniert sich mit EUR 29 deutlich darunter -- aggressiverer Einstieg.

---

## Markt-Uebersicht (Stand Maerz 2026)

| Tool | Preis | Plattform | Offline | Source-Available | Shipped |
|------|-------|-----------|---------|-----------------|---------|
| **Klarvo** | EUR 29 Early Bird / EUR 39 (geplant) | Windows, Android | Ja (geplant) | Ja (BSL 1.1) | **Ja** (v0.5.0) |
| Wispr Flow | $12/mo ($144/Jahr) | macOS, Windows, Android, iOS | Nein | Nein | Ja |
| Amical | Gratis (MIT) | macOS, Windows, Android (Beta) | Ja | Ja (MIT) | Ja (Desktop) |
| dikta.me | $20-25 einmalig | nur Windows | Ja | Ja (unklar) | **Nein** (Waitlist) |
| Voice Type | $19.99 einmalig | nur macOS | Ja | Nein | Ja |
| OpenWhispr | Gratis (MIT) | macOS, Windows, Linux | Ja | Ja (MIT) | Ja |
| Voicy | $8.49/mo oder $220 Lifetime | macOS, Windows, Browser | Nein | Nein | Ja |
| Dragon | $14.99/mo oder $700 | Windows, macOS | Ja | Nein | Ja |

## Beobachtungsliste

Regelmaessig pruefen (alle 4-6 Wochen):
- [ ] **Amical Android-Beta:** Wird sie public? Feature-Umfang? Monetarisierung?
- [ ] **Wispr Flow Android Pricing:** Aktuell Free Promo -- wann kommt das Abo?
- [ ] **OpenWhispr Mobile:** Anzeichen fuer Android-Support?
- [ ] **dikta.me Launch-Status:** Noch Waitlist? Wann shipped? Feature-Umfang bei Release?
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

## dIKta.me -- direkter Wettbewerber (ehemals Namenskonflikt, durch Umbenennung zu Klarvo geloest)

- **URL:** dikta.me
- **GitHub:** github.com/geckogtmx/diktame
- **Plattform:** nur Windows (10 2004+, 11); macOS/iOS/Android auf Roadmap ("Soon")
- **Preis:** $20 Early Bird / $25 Regular (Power Version), Free Trial mit Cloud-Credits
- **Lizenz:** "Source-Available" auf Website, aber kein Lizenzfile im Repo (rechtlich unklar)
- **Tech-Stack:** C# / .NET 8, WinUI 3 (Fluent Design), SQLite
- **Entwickler:** Solo-Entwickler, 246 Commits. Vorheriges Projekt war Python + Electron, dann nativer Rewrite.
- **Status:** Noch im Waitlist-Stadium (Stand 2026-03-25). "Download"-Button fuehrt zu /waitlist, kein oeffentlicher Installer.
- **Recherche-Datum:** 2026-03-25 (Landingpage-Analyse)

### Pricing-Struktur (3 Tiers)
1. **Free Trial** ($0): Cloud-Credits (Deepgram STT + Gemini Flash LLM), kein Offline
2. **Power Version** ($20 Early Bird / $25 Regular): Alles lokal, alle Modi, unbegrenzt
3. **Build It Yourself** (kostenlos): Source Code + Build Guide

### Features (48+ beworben)
- 7 Workflow-Modi: Dictate, Refine, Ask, Translate, Oops, Note, Read Selection (TTS)
- Cloud STT (Deepgram, Gemini) + lokal (Whisper V3 Turbo mit Vulkan GPU)
- TTS: Read Selection mit Kokoro-ONNX lokal
- LLM: Gemini, Anthropic, OpenAI, Ollama (eingebettetes Management), lokale Small Models (Gemma 3, Llama 3)
- Dual-Profile-System mit 16 Custom Prompts
- Voice Macros (Trigger-basierte Makro-Expansion)
- Quick Chat (Floating LLM-Overlay)
- Pay-As-You-Go Cloud Wallet ($5 ≈ 65.000 Woerter)
- 90-Tage SQLite-Historie
- Sicherheit: DPAPI-Secrets (AES-256), PII-Scrubber, API-Key-Validierung, Zero Telemetry
- ~150MB portable .exe (Self-Contained Binary inkl. Python Runtime + Ollama)
- Startup <3 Sekunden, Memory ~60 MB

### Qualitaet
- 1,014 Tests (Stand Landingpage, vorher 950 beim GitHub-Audit)
- 0 Fehler, 0 Warnungen im Release-Build
- CI/CD Pipeline (Lint → Build → Test → Publish)

### Landingpage-Analyse (2026-03-25)

**Was sie gut machen:**
- Starke Hero-Rotation ("STOP TYPING. START TALKING/THINKING/WORKING/WINNING")
- Vergleichstabelle "vs The Cloud" direkt auf der Seite
- "Use it with..." Logo-Carousel (Terminal, Cursor, VS Code, Slack, Excel, Discord)
- Privacy als Hero-Story ("100% Air-Gapped by Default", "Works in a Bunker")
- Technische Metriken als Trust (1,014 Tests, ~60 MB Memory, <3 sec Startup)
- Early Bird Badge auf Pricing-Card
- Drei-Tier-Pricing mit "Build It Yourself" als Transparenz-Signal

**Wo sie schwach sind:**
- Noch nicht shipped (Waitlist!) -- keine echten Nutzer, kein Installer
- Feature-Overload (48+ Features aufgelistet, ueberwaeltigend)
- Kein Social Proof (null Testimonials, null Reviews, null Download-Zahlen)
- "Zero Censorship" als Selling Point (zieht falsche Zielgruppe an)
- Technischer Jargon dominiert ("Vulkan GPU", "ONNX Runtime")
- Kein Android/iOS/macOS -- alles nur "Soon" auf Roadmap

### Staerken
- Professionelle Codequalitaet (1,014 Tests, CI/CD)
- Natives Windows-UI (WinUI 3, kein Electron)
- 7 Modi + TTS (Feature-Tiefe auf Windows)
- Ollama eingebettet mit Health-Checks und Model-Library
- Lokale LLMs out-of-the-box (Gemma 3, Llama 3)
- Gut designte Landingpage mit klarem Messaging

### Schwaechen aus unserer Sicht
- **Nicht shipped** -- Waitlist, kein oeffentlicher Download (Stand 03/2026)
- **Nur Windows** -- kein Android, kein Mobile (alles "Soon")
- **Keine Distribution** -- Portable .exe, kein Installer, kein Auto-Updater
- **Kein Lizenzfile im Repo** -- "Source-Available" ohne rechtliche Grundlage
- **.NET 8 + eingebettete Python Runtime** -- schwergewichtiger als Tauri/Rust
- **Credit/Wallet-System** -- geht Richtung Mikrotransaktionen, weg vom klaren Einmalkauf
- **Kein Social Proof** -- keine Nutzer, keine Reviews

### Strategische Einordnung
Naechster direkter Wettbewerber nach Amical. Validiert unser Geschaeftsmodell (Einmalkauf + Source-Available + Voice Dictation) und unser Early Bird Pricing. Aber: **nicht shipped** -- das ist der entscheidende Unterschied. Klarvo hat echte Tester, signierte Builds, Auto-Updater, Android. dikta.me hat eine schoene Landingpage.

Klarvo-Kern-USP gegenueber dikta.me: "Das Produkt das dikta.me verspricht -- aber tatsaechlich liefert."

## Fazit (aktualisiert 2026-03-25)

Der Markt hat sich seit Anfang 2026 bewegt: Wispr Flow ist auf Android und iOS, Amical naehert sich
mit Mobile-Beta, dikta.me baut eine schoene Landingpage aber hat noch nicht shipped.

Klarvos Differenzierung laeuft ueber drei Achsen:
1. **Einmalkauf** (EUR 29 Early Bird / EUR 39) vs. Abo (Wispr Flow) oder kein Modell (Amical/OpenWhispr)
2. **Shipped auf 2 Plattformen** vs. Waitlist (dikta.me), Beta (Amical), oder Cloud-only (Wispr Flow)
3. **Source-Available + Bezahlprodukt** = nachhaltig UND transparent

Neuer Differenziator gegenueber dikta.me: **Shipped vs. Versprochen.** dikta.me validiert unser
Geschaeftsmodell und Early Bird Pricing, ist aber noch nicht als Produkt erhaeltlich. Klarvo liefert.
