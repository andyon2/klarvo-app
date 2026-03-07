# Competitor Analysis

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

### Technische Architektur

- Lokale Spracherkennung (macOS-native, kein eigenes Modell)
- Kein Backend-Server (Marketing-Site ist static export)
- Keine Analytics, keine Crash-Logs, keine Telemetrie
- BYO-LLM Rewrites gehen direkt vom Geraet zum Provider
- Mac App Store Distribution (MDM-kompatibel)

### Enterprise-Positionierung

- HIPAA-ready im Offline-Modus (kein BAA noetig)
- SOC 2: Keine Kundendaten-Verarbeitung, minimaler Questionnaire
- Security Whitepaper auf Anfrage
- Priority Email Support fuer Volume-Kaeufe

### Wettbewerbsvergleich (deren eigene Darstellung)

| Aspekt | Voice Type | Wispr Flow | Otter |
|--------|-----------|-----------|-------|
| Verarbeitung | On-device | Cloud | Cloud |
| Compliance | HIPAA-ready offline | SOC 2 Type II + BAA | SOC 2, HIPAA ab Juli 2025 |
| Modell-Kontrolle | Lokal + BYO | Proprietaer | Proprietaer |

### Was wir daraus lernen

**Staerken von Voice Type:**
1. Offline-first als Hauptargument -- Privacy/Compliance ohne Kompromisse
2. Extreme Einfachheit -- minimale UI, 3-Schritt-Onboarding
3. Enterprise/Compliance-Messaging -- HIPAA, SOC 2, MDM
4. Kein Abo-Modell -- einmalige Zahlung

**Wo Dikta schon besser ist:**
1. Multi-Provider STT/LLM mit konfigurierbarer Fallback-Kette
2. 3 Cleanup-Stile + Live Translation + Multi-Format Output
3. Voice Notes, Text Snippets, App Profiles
4. History mit Volltextsuche, Filler-Word-Analyse, Kostentracking
5. Command Mode (Sprachbefehle auf selektierten Text)
6. Whisper Mode (leises Diktieren)
7. Kostenlos & Open Source
8. Multi-Plattform (Windows + Android geplant)

**Inspiration fuer Dikta:**
1. **whisper.cpp Offline-Modus priorisieren** -- staerkstes Differenzierungsmerkmal von Voice Type
2. **Privacy-Messaging** -- wenn Offline-Modus steht, koennen wir aehnlich argumentieren
3. **Enterprise-Features** -- spaeter relevant (MDM, Compliance-Docs)
4. **Einfachheit bewahren** -- trotz vieler Features muss das Kern-Erlebnis (Hotkey -> Sprechen -> Text) simpel bleiben

---

## Wispr Flow

- **Plattform:** macOS, Windows
- **Preis:** Abo-Modell
- **Kernfeature:** Cloud-basierte Voice-to-Text mit proprietaerem Modell
- **Positionierung:** Premium, enterprise-ready
- **Dikta-Vergleich:** Wispr Flow ist die direkte Inspiration fuer Dikta. Dikta bietet aehnliche Features ohne Abo-Zwang.

---

## Fazit

Voice Type ist ein solides, minimalistisches Tool mit Privacy-Fokus. Fuer den Mac-Port von Dikta waere der Offline-Modus (whisper.cpp) ein starkes Differenzierungsmerkmal -- dann kann Dikta beides: offline UND cloud mit besserer Qualitaet. Die Feature-Tiefe von Dikta ist bereits deutlich groesser als Voice Type.
