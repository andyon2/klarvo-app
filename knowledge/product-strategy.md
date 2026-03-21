# Product Strategy

Positionierung, Zielgruppe, Monetarisierung und Differenzierung von Dikta.
Diese Datei ist die Source of Truth fuer alle strategischen Produkt-Entscheidungen.

Letzte Aktualisierung: 2026-03-21 (Umbenennung: Dikta → Voxlit entschieden)

## Umbenennung: Dikta → Voxlit

**Status:** Entschieden (2026-03-21). Neuer Produktname ist **Voxlit**.

**Etymologie:** Vox (Lat. "Stimme") + lit (literary/polished). "Voice → polished text."

**Gruende fuer Umbenennung:**
- **dikta iOS App** (rohe technik OÜ / rohe.ai): AI Voice Keyboard im App Store, gleicher Markt
- **dIKta.me**: Private AI Voice Dictation fuer Windows, $20 Einmalkauf, Source-Available -- praktisch unser Zwilling
- **DIKTA INC SRL** (Rumaenien): EU-Bildmarke in Klasse 09 (Software), eingetragen bis 2032 (EUIPO #018737941)
- Drei Produkte mit demselben Namen im selben Markt = nicht verteidigbar, nicht suchbar, nicht vermarktbar.

**Markenstatus:**
- DPMA Klasse 09: Frei (geprueft 2026-03-21)
- Keine Software-Konflikte gefunden (Deep Research + manuelle Recherche)
- Kunstwort -- geringe Gefahr dass jemand unabhaengig darauf kommt

**Domains:**
- [x] **voxlit.app** -- gesichert (2026-03-21, Hostinger, laeuft bis 2027-03-21, auto-renew an)
- voxlit.com: Geparkt bei GoDaddy, $4.995 -- nicht kaufen
- Domain dikta.software: Reservierung bei Hostinger storniert (2026-03-21)

**Markenanmeldung:**
- Jetzt noch nicht (290 EUR DPMA). Spaetestens kurz bevor Landingpage live geht / erster Verkauf.
- Grund: Trademark Squatting -- sobald das Produkt sichtbar wird, koennte jemand den Namen registrieren.

**Umbenennung durchfuehren (wenn bereit):**
- [ ] Codebase (Rust + React + Android)
- [ ] GitHub-Repo (dikta-public → voxlit)
- [ ] README, Branding, Icons
- [ ] Landingpage-Briefing anpassen
- [ ] Portfolio-Text auf Berater-Website anpassen (Website-Agent informieren)

## Positionierung

**Kern-Satz:**
> Sprachdiktat das dir gehoert. Einmal kaufen, ueberall nutzen, online oder offline.

**Englisch (fuer internationale Kommunikation):**
> Voice dictation you own. Buy once, use everywhere.

**Drei Differenzierungs-Achsen** (validiert gegen Wettbewerb, Maerz 2026):

### 1. Einmalkauf vs. Abo (vs. Wispr Flow)
Wispr Flow ist die direkte Inspiration und der staerkste Wettbewerber -- auch auf Android (seit Feb 2026).
Aber: $144/Jahr Abo. Dikta kostet EUR 29 einmalig. Das ist ~2.5 Monate Wispr Flow.
In einem Markt mit wachsender Subscription Fatigue (+6% Einmalkauf-Praeferenz 2025/26)
ist das ein klarer Kaufgrund.

### 2. Shipped vs. Beta (vs. Amical)
Amical ist der naechste Open-Source-Wettbewerber (MIT, offline, Win+Mac).
Aber: Android ist dort nur Private Beta, kein shipped Product.
Dikta hat Android jetzt, mit Floating Bubble, AccessibilityService, systemweitem Paste.
Wer heute Android braucht, hat genau eine Open-Source-Option: Dikta.

### 3. Bezahlprodukt vs. Hobby-Projekt (vs. Open-Source-Alternativen)
OpenWhispr, Amical, Handy -- alle gratis, alle Community-getrieben.
EUR 29 signalisiert: Hier kuemmert sich jemand langfristig. Updates, Support, Roadmap.
Gratis-Tools haben kein Nachhaltigkeitsmodell. Dikta schon.

**Kaufgruende vs. Wettbewerb (aktualisiert):**
- Vs. Wispr Flow: EUR 29 einmalig statt $144/Jahr, offline-faehig, Open Source, kein Cloud-Zwang
- Vs. Amical: Android shipped (nicht Beta), Bezahlprodukt mit Commitment, Tauri/Rust statt Electron
- Vs. Voice Type: Windows + Android statt nur macOS, 5x mehr Features
- Vs. OpenWhispr: Android-Support, native Performance (Tauri statt Electron), poliertes Produkt
- Vs. Dragon: Moderner, guenstiger, nicht Enterprise-aufgeblasen

**Wichtig:** Dikta positioniert sich NICHT ueber "4 Checkboxen die niemand hat" (falsifizierbar),
sondern ueber die Kombination aus Preis, Plattform-Reife und Nachhaltigkeit.

## Zielgruppe

### Primaer: Abo-muede Vieltipper
- Entwickler, Autoren, Journalisten, Wissensarbeiter
- Kennen Wispr Flow, wollen aber kein EUR 130/Jahr-Abo
- Tech-savvy genug um Open Source zu schaetzen
- Marktsignal: Einmalkauf-Praeferenz waechst um 6% (2025/26, Subscription Fatigue Trend)

### Sekundaer: Privacy-bewusste Nutzer
- Wollen keine Sprachdaten in der Cloud
- Berufsgruppen mit Vertraulichkeit: Therapeuten, Anwaelte, Aerzte
- Offline-Modus ist hier der Kaufgrund

### Tertiaer: Android-Nutzer
- Wispr Flow ist seit Feb 2026 auf Android, aber Abo-only und cloud-only
- Dikta ist die einzige Open-Source + Offline-Alternative auf Android
- Amical-Android ist nur Private Beta

## Monetarisierung

### Modell: Open Core mit License Key

Repo bleibt Open Source. Build laeuft ohne Key. Bestimmte Features schalten sich nur mit License Key frei.

**Preis: EUR 29 Einmalkauf**

### Preisbegruendung

| Referenz | Preis | Vergleich |
|----------|-------|-----------|
| Voice Type | ~EUR 18 einmalig | Dikta hat 5x mehr Features, 2 Plattformen |
| Wispr Flow | ~EUR 130/Jahr | EUR 29 = ~2.5 Monate Wispr Flow |
| Voicy Lifetime | ~EUR 200 | Dikta juenger, niedriger Einstieg zum Start |
| VoiceTypr | EUR 32-90 | Aehnliche Range |

EUR 29 ist:
- Niedrig genug fuer Impulskauf
- Hoch genug um Qualitaet zu signalisieren
- ~2.5 Monate Wispr Flow (klares Abo-Killer-Argument im Marketing)
- Steigerbar auf EUR 39-49 wenn Offline-Modus + weitere Features live sind

### Free-Tier (Open Source)
- Kern-Diktat (Hotkey -> Sprechen -> Text)
- Alle STT/LLM-Provider (Groq, DeepSeek, OpenAI, OpenRouter) -- BYOK-Prinzip. Anthropic deaktiviert (nie verifiziert).
- Alle Basis-Cleanup-Stile (Polished, Clean, Chat)
- Cleanup Instructions (Custom Prompt)
- Offline-Modus mit Whisper small-Modell (~488 MB) -- brauchbar, guter erster Eindruck
- Dictionary (limitiert: 20 Eintraege)
- Basis-Statistiken (Diktat-Anzahl, API-Kosten)
- Basis-Settings
- Limitierte History (letzte 50 Eintraege)

### Paid-Tier (License Key, EUR 29)
- Offline-Modus mit groesseren Whisper-Modellen (medium, large-v3) -- deutlich bessere Qualitaet, besonders bei Hintergrundgeraeusch und Fachjargon
- Custom Style Templates (gespeicherte, benannte Prompt-Sets)
- Command Mode
- Text Snippets
- App Profiles
- Unbegrenzte History + Volltextsuche
- Voice Notes
- Cross-Device Sync (Turso)
- Dictionary (unbegrenzt)
- Whisper Mode (leises Diktieren)
- Filler-Word-Analyse + erweiterte Statistiken
- Webhooks
- Integrations (Notion, Todoist -- spaeter)

### Spaetere Preis-Optionen
- Preis auf EUR 39-49 erhoehen nach Offline-Modus-Launch
- Team-Lizenzen (EUR 24/Seat ab 5 Seats)
- Major Version Upgrades als optionale Paid-Upgrades (v2.0 fuer EUR 15)

## Differenzierung

### Strategische Positionierung gegenueber Wispr Flow
Wispr Flow ist die direkte Inspiration fuer Dikta. Diktats Android-Version ist an Wispr Flows
Floating-Bubble-UI angelehnt. Differenzierung laeuft NICHT ueber UX-Ueberlegenheit
(Wispr Flow hat mehr Ressourcen), sondern ueber:
- **Preis:** EUR 29 einmalig vs. $144/Jahr
- **Offline:** Funktioniert ohne Internet (wenn whisper.cpp steht)
- **Transparenz:** Source-available (BSL 1.1), Quellcode einsehbar, keine Black Box
- **Ownership:** Keine Cloud-Abhaengigkeit, Daten bleiben lokal

### Strategische Positionierung gegenueber Amical
Amical ist der naechstliegende Open-Source-Wettbewerber. Gleiche Vision (offline, open source,
Wispr-Flow-Alternative). Differenzierung:
- **Android shipped** vs. Android Private Beta
- **Bezahlprodukt** (EUR 29) vs. kein Monetarisierungsmodell
- **Tauri/Rust** vs. Electron (Performance, Ressourcenverbrauch)
- Amical beobachten -- wenn deren Android public wird, wird der Vorsprung kleiner

### Strategische Positionierung gegenueber OpenWhispr
Nicht bekaempfen, nicht kopieren, **komplementaer positionieren:**
- OpenWhispr = Open-Source-Projekt zum Mitbauen (Community-driven, Tinkerer)
- Dikta = Fertiges Tool das einfach funktioniert (Product-driven, End-User)
- Kein Feature-Race gegen MIT-Community. Stattdessen: Polish, Android, Zuverlaessigkeit.

### Feature-Vorsprung vs. Voice Type
Dikta bietet bereits deutlich mehr als Voice Type (EUR 18):
- 3 Cleanup-Stile + Live Translation + Multi-Format
- Command Mode, Snippets, App Profiles
- History mit Volltextsuche, Filler-Analyse, Kostentracking
- Multi-Provider STT/LLM mit Fallback-Kette
- Whisper Mode, Voice Notes

## Roadmap-Priorisierung (Business-Sicht)

### v1.0 -- Erstes Paid Release

| Prio | Feature | Business-Grund |
|------|---------|----------------|
| 1 | Signing + Auto-Update | Grundvoraussetzung. Unsigned = kein Vertrauen. Ohne Auto-Update keine Iteration. |
| 2 | License-Key-System | Ohne das kein Paid Release moeglich. |
| 3 | Offline whisper.cpp | DAS Differenzierungsmerkmal. Staerkster Kaufgrund vs. Wispr Flow. |
| 4 | Onboarding/Polish | Erster Eindruck entscheidet. Install -> Hotkey -> Go. |

### v1.1 -- Qualitaet (Retention)

| Prio | Feature | Business-Grund |
|------|---------|----------------|
| 5 | VAD (Voice Activity Detection) | Quality-of-Life. Kein Kaufgrund, aber reduziert Churn. |
| 6 | Bubble Size/Opacity Controls | Kosmetik, schnell umsetzbar. |

### v2.0 -- Wachstum

| Prio | Feature | Business-Grund |
|------|---------|----------------|
| 7 | Integrationen (Notion, Todoist) | Oeffnet neue Zielgruppen. Kann Kaufgrund werden. |
| 8 | CI/CD Pipeline | Interne Effizienz. Kein Kunden-Impact, spart Entwickler-Zeit. |
| 9 | macOS-Port | Grosser Markt, grosser Aufwand. Erst wenn Windows + Android monetarisiert. |

## Wettbewerbs-Beobachtung

Regelmaessig pruefen (alle 4-6 Wochen):
- **Amical Android-Beta:** Wird sie public? Feature-Umfang?
- **Wispr Flow Android Pricing:** Aktuell Free Promo -- wann kommt das Abo?
- **OpenWhispr Mobile:** Gibt es Anzeichen fuer Android-Support?

## Quellen
- Wettbewerbsanalyse Voice Type (2026-03-08)
- Wettbewerbsanalyse OpenWhispr (2026-03-09)
- Wispr Flow Pricing-Recherche: $12/mo Pro, $24/mo Enterprise (2026-03-09)
- Wispr Flow Android-Launch: 23. Feb 2026 (TechCrunch)
- Amical: Open Source, MIT, Android Private Beta (2026-03-09)
- Voicy Pricing: $8.49/mo, $220 Lifetime (2026-03-09)
- Subscription Fatigue Trend: +6% One-Time-Purchase-Praeferenz (2025/26)
- Strategy-Session mit Andy (2026-03-09): EUR 29 Preis, Open Core Modell, Roadmap
- Wettbewerbs-Validierung (2026-03-09): "4 Checkboxen"-Claim angepasst, Amical als Wettbewerber identifiziert
