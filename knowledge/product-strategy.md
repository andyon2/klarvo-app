# Product Strategy

Positionierung, Zielgruppe, Monetarisierung und Differenzierung von Voxlit.
Diese Datei ist die Source of Truth fuer alle strategischen Produkt-Entscheidungen.

Letzte Aktualisierung: 2026-03-25 (Launch-Strategie, Distribution, Early Bird Pricing, dikta.me-Analyse)

**Hinweis:** Produktname ist offiziell **Klarvo** (seit 2026-03-21). Rename in Projektdateien steht noch aus.
Referenzen auf "Voxlit" in diesem Dokument werden schrittweise aktualisiert.

## Umbenennung: Voxlit → Voxlit

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
- [ ] GitHub-Repo (voxlit-app → voxlit)
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
Aber: $144/Jahr Abo. Voxlit kostet EUR 29 einmalig. Das ist ~2.5 Monate Wispr Flow.
In einem Markt mit wachsender Subscription Fatigue (+6% Einmalkauf-Praeferenz 2025/26)
ist das ein klarer Kaufgrund.

### 2. Shipped vs. Beta (vs. Amical)
Amical ist der naechste Open-Source-Wettbewerber (MIT, offline, Win+Mac).
Aber: Android ist dort nur Private Beta, kein shipped Product.
Voxlit hat Android jetzt, mit Floating Bubble, AccessibilityService, systemweitem Paste.
Wer heute Android braucht, hat genau eine Open-Source-Option: Voxlit.

### 3. Bezahlprodukt vs. Hobby-Projekt (vs. Open-Source-Alternativen)
OpenWhispr, Amical, Handy -- alle gratis, alle Community-getrieben.
EUR 29 signalisiert: Hier kuemmert sich jemand langfristig. Updates, Support, Roadmap.
Gratis-Tools haben kein Nachhaltigkeitsmodell. Voxlit schon.

**Kaufgruende vs. Wettbewerb (aktualisiert 2026-03-25):**
- Vs. Wispr Flow: EUR 29 einmalig statt $144/Jahr, offline-faehig, Source-Available, kein Cloud-Zwang
- Vs. Amical: Android shipped (nicht Beta), Bezahlprodukt mit Commitment, Tauri/Rust statt Electron
- Vs. Voice Type: Windows + Android statt nur macOS, 5x mehr Features
- Vs. OpenWhispr: Android-Support, native Performance (Tauri statt Electron), poliertes Produkt
- Vs. dikta.me: Shipped vs. Waitlist, 2 Plattformen vs. 1, signierter Installer vs. portable .exe
- Vs. Dragon: Moderner, guenstiger, nicht Enterprise-aufgeblasen

**Wichtig:** Voxlit positioniert sich NICHT ueber "4 Checkboxen die niemand hat" (falsifizierbar),
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
- Voxlit ist die einzige Open-Source + Offline-Alternative auf Android
- Amical-Android ist nur Private Beta

## Monetarisierung

### Modell: Open Core mit License Key

Repo bleibt Open Source. Build laeuft ohne Key. Bestimmte Features schalten sich nur mit License Key frei.

**Preis: EUR 29 Early Bird → EUR 39 Regular**

### Early Bird Pricing (entschieden 2026-03-25)

| Phase | Preis | Zeitraum |
|-------|-------|----------|
| **Early Bird** | EUR 29 | Bis v1.0 oder erste ~200 Lizenzen (was zuerst kommt) |
| **Regular** | EUR 39 | Ab dann dauerhaft |

Validierung: dikta.me nutzt dasselbe Modell ($20 Early Bird → $25 Regular).

**Early Bird EUR 29:**
- ~2.5 Monate Wispr Flow -- starkes Abo-Killer-Argument
- Niedrig genug fuer Impulskauf
- Belohnt fruehe Unterstuetzer die das Risiko eines jungen Produkts tragen

**Regular EUR 39:**
- ~3.3 Monate Wispr Flow -- immer noch ueberzeugend
- Gerechtfertigt sobald macOS dazukommt (3 Plattformen fuer EUR 39)
- Hoch genug um Qualitaet und Nachhaltigkeit zu signalisieren

**Umschaltzeitpunkt:** Nach Meilenstein, nicht nach Datum. "Erste 200 Lizenzen" oder "v1.0 Release"
schafft echte Knappheit und ist ehrlicher als ein kuenstliches Deadline.

### Preisbegruendung

| Referenz | Preis | Vergleich |
|----------|-------|-----------|
| Voice Type | ~EUR 18 einmalig | Klarvo hat 5x mehr Features, 2 Plattformen |
| dikta.me | $20-25 einmalig | Nur Windows, noch nicht shipped (Waitlist) |
| Wispr Flow | ~EUR 130/Jahr | EUR 29 = ~2.5 Monate, EUR 39 = ~3.3 Monate |
| Voicy Lifetime | ~EUR 200 | Klarvo juenger, niedriger Einstieg zum Start |
| VoiceTypr | EUR 32-90 | Aehnliche Range |

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

### Lizenzmodell (entschieden 2026-03-25)

**Eine Lizenz = alle Plattformen.** Keine getrennten Windows/Android/macOS-Lizenzen.

Gruende:
- Ist die Positionierung: "Einmal kaufen, ueberall nutzen"
- EUR 29/39 ist niedrig genug -- niemand erwartet Plattform-Aufpreis
- Cross-Platform ist DER Differenziator vs. dikta.me (nur Windows) und Voice Type (nur macOS)
- Weniger Komplexitaet: Ein Produkt in Lemon Squeezy, ein Key-Typ, eine Checkout-Seite

Technisch:
- Ein Lizenzschluessel, in Klarvo Windows oder Android eingeben
- Activation Limit: 3 Geraete (realistisch: 1-2 Desktop + 1 Mobile)
- Lemon Squeezy License API fuer Validierung

### Zahlungsabwicklung: Lemon Squeezy (entschieden 2026-03-25)

**Warum Lemon Squeezy (nicht Paddle, nicht Stripe direkt, nicht AppSumo):**

| Kriterium | Lemon Squeezy | Paddle | Stripe | AppSumo |
|-----------|---------------|--------|--------|---------|
| Gebuehren | 5% + 50c | 5% + 50c | 2.9% + 30c | 70% (!!) |
| Bei EUR 29 | ~EUR 27 netto | ~EUR 27 netto | ~EUR 28 netto | ~EUR 9 netto |
| License Keys | Built-in | Nicht dabei | Nicht dabei | N/A |
| Merchant of Record | Ja | Ja | Nein | Ja |
| Onboarding | Sofort | Approval (Tage) | Sofort | ~20% Annahmequote |

- License Keys out of the box (Generierung, Activation Limits, API-Validierung)
- Merchant of Record: Lemon Squeezy handelt VAT/Umsatzsteuer
- Gehoert seit 2024 zu Stripe (Backup-Pfad falls noetig)
- Sofortiges Onboarding, kein Approval-Prozess

**AppSumo wurde bewusst ausgeschlossen** (evaluiert 2026-03-25):
- SaaS-Marktplatz, nicht fuer Desktop-Apps (Plattform-Mismatch)
- 70% Revenue Share bei EUR 29 = ~EUR 9 pro Sale (untragbar)
- Deal-Hunter-Audience passt nicht zur Zielgruppe (kein API-Key-Setup, kein Offline-Modell)
- Support-Peak bei Launch als Solo-Entwickler nicht stemmbar
- 60-Tage-Refund-Window, nur ~20% Annahmequote

### Naechste Schritte Zahlungsabwicklung
1. Lemon Squeezy Account anlegen
2. Produkt "Klarvo License" (One-Time Purchase, EUR 29 Early Bird)
3. License Key Template (KLARVO-Prefix, Activation Limit 3)
4. Checkout-Link auf klarvo.app einbetten
5. Webhook/API-Integration fuer Key-Validierung im Client

### Spaetere Preis-Optionen
- Team-Lizenzen (EUR 24/Seat ab 5 Seats)
- Major Version Upgrades als optionale Paid-Upgrades (v2.0 fuer EUR 15)

## Distribution (entschieden 2026-03-25)

### Windows: klarvo.app + signierter Installer
- Download direkt von klarvo.app
- Lemon Squeezy Checkout fuer License Key
- Auto-Updater bereits implementiert

### Android Phase 1: APK-Sideloading ueber klarvo.app (Launch)
- APK-Download direkt von klarvo.app
- Nutzer aktiviert "Unknown Sources" (einmalig)
- In-App Auto-Updater fuer Updates
- Passt zur Positionierung: "Dein Geraet, deine Kontrolle"

### Android Phase 2: Google Play Store (langfristig)
- Wispr Flow ist im Play Store MIT AccessibilityService -- Beweis dass es geht
- Strategie: Accessibility-Framing ("Voice-to-Text Accessibility Tool")
- Permission Declaration Form sauber begruenden
- Erst wenn App stabil + polished (nach v1.0)
- Kein regulatorisches Risiko, aber gruendlicher Review-Prozess

### Lemon Squeezy Rolle bei Android
Lemon Squeezy verkauft die Lizenz, nicht die App. Die App ist kostenlos downloadbar (Free Tier).
Wer Premium will, kauft Key ueber Lemon Squeezy und gibt ihn in der App ein -- egal ob Windows oder Android.

Flow: klarvo.app → "Download fuer Android" → APK (kostenlos)
Flow: klarvo.app → "Lizenz kaufen" → Lemon Squeezy → Key → in App eingeben

### Launch-Sichtbarkeit (ohne AppSumo)
- **Product Hunt Launch** -- kostenlos, genau die richtige Zielgruppe
- **Hacker News "Show HN"** -- Source-Available + Rust + Indie = starke Resonanz
- **Reddit** (r/SideProject, r/androidapps, r/productivity)
- **GitHub README** mit Link zu klarvo.app

## Differenzierung

### Strategische Positionierung gegenueber Wispr Flow
Wispr Flow ist die direkte Inspiration fuer Voxlit. Diktats Android-Version ist an Wispr Flows
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
- Voxlit = Fertiges Tool das einfach funktioniert (Product-driven, End-User)
- Kein Feature-Race gegen MIT-Community. Stattdessen: Polish, Android, Zuverlaessigkeit.

### Strategische Positionierung gegenueber dikta.me (aktualisiert 2026-03-25)

dikta.me ist der naechste direkte Wettbewerber: Solo-Entwickler, Einmalkauf, Source-Available,
Voice Dictation fuer Windows. Validiert unser Geschaeftsmodell. Aber:

**Kern-USP:** Klarvo ist das Produkt das dikta.me verspricht -- aber tatsaechlich liefert.

| | dikta.me | Klarvo |
|---|---|---|
| Status | Waitlist (nicht shipped, Stand 03/2026) | Shipped (v0.5.0, echte Tester) |
| Plattformen | Nur Windows | Windows + Android |
| Distribution | Portable .exe, kein Installer | Signierter Installer + Auto-Updater |
| Preis | $20-25 | EUR 29-39 |
| Positionierung | Privacy-Maximalist, Feature-Heavy (48+) | "Funktioniert einfach", zwei Plattformen |
| Mobile | "Soon" (Roadmap) | Android shipped |

Differenzierung laeuft NICHT ueber Feature-Anzahl (dikta.me hat 48+, Feature-Race verlieren wir),
sondern ueber: Shipped-Status, Mobile, Einfachheit, Distribution-Reife.

### Landingpage-Learnings von dikta.me (2026-03-25)

**Uebernehmen:**
1. Vergleichstabelle direkt auf die Seite -- "Klarvo vs Wispr Flow" (den kennt die Zielgruppe)
2. "Use it with..." Logo-Carousel (VS Code, Slack, Discord, Excel) -- zeigt Kompatibilitaet ohne Worte
3. Konkrete technische Metriken als Trust-Signal (Tests, Startup-Time)
4. Drei Pricing-Tiers inkl. "Source Code einsehbar" als eigener Tier
5. Privacy mit konkreten Claims statt vagen Versprechen ("AES-256" statt nur "sicher")
6. Early Bird Badge auf dem Pricing-Card (Kaufdringlichkeit)

**Nicht uebernehmen:**
1. Feature-Overload (48+ Features aufgelistet) -- Klarvo zeigt 6-8 Kernfeatures, kommuniziert "einfach"
2. Waitlist als Launch -- Klarvo launcht mit echtem Download
3. "Zero Censorship" als Selling Point -- zieht die falsche Zielgruppe an
4. Technischer Jargon ueberall ("Vulkan GPU", "ONNX Runtime") -- Zielgruppe will diktieren, nicht Benchmarks lesen
5. Kein Social Proof -- dikta.me hat null Testimonials. Klarvo sollte Tester-Feedback frueh einbauen

### Feature-Vorsprung vs. Voice Type
Voxlit bietet bereits deutlich mehr als Voice Type (EUR 18):
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

### v1.x -- macOS-Port (aktualisiert 2026-03-25)

| Prio | Feature | Business-Grund |
|------|---------|----------------|
| 7 | **macOS-Port (Tauri)** | Grosser neuer Markt. Tauri unterstuetzt macOS -- gleicher Codebase wie Windows. Loest teilweise das iPhone-Problem (Mac-Nutzer bekommen Desktop-Diktat). Mittlerer Aufwand. |
| 8 | Google Play Store | Wispr Flow beweist: AccessibilityService wird akzeptiert. Erhoehte Reichweite fuer Android. |

### v2.0 -- Wachstum

| Prio | Feature | Business-Grund |
|------|---------|----------------|
| 9 | Integrationen (Notion, Todoist) | Oeffnet neue Zielgruppen. Kann Kaufgrund werden. |
| 10 | **iOS (Custom Keyboard Extension)** | Komplett neuer Codebase (Swift/SwiftUI), hoher Aufwand. Aber: kein regulatorisches Risiko (Apple App Store akzeptiert Keyboard Extensions, Wispr Flow ist dort). Komplettiert das Plattform-Angebot. |
| 11 | CI/CD Pipeline | Interne Effizienz. Kein Kunden-Impact, spart Entwickler-Zeit. |

### Plattform-Roadmap Uebersicht (entschieden 2026-03-25)

| Phase | Plattform | Aufwand | Ansatz |
|-------|-----------|---------|--------|
| v1.0 | Windows + Android | Done | Tauri + Kotlin |
| v1.x | **macOS** | Mittel | Tauri-Port (gleicher Codebase) |
| v1.x | Android Play Store | Mittel | Accessibility-Framing, Wispr Flow als Praezedenz |
| v2.0 | **iOS** | Hoch | Native Swift Keyboard Extension (neuer Codebase) |

**macOS vor iOS** weil: weniger Aufwand (Tauri-Port vs. native Swift), groesserer sofortiger Impact,
und es loest teilweise das iPhone-Problem (Mac+iPhone-Nutzer bekommen wenigstens Desktop-Diktat).

**iOS ist machbar** -- kein regulatorisches Risiko (Custom Keyboard Extensions sind Apples offizieller Weg,
Wispr Flow ist damit im App Store). Aufwand ist rein Build-Aufwand, keine Sondergenehmigung noetig.

**iPhone-Markt-Einschaetzung (2026-03-25):**
- DACH: ~70% Windows Desktop, ~30-35% iPhone → ~22-25% nutzen Windows + iPhone
- USA: ~71% Windows, ~58% iPhone → ~40% Ueberlappung
- Dieses Segment bekommt volles Desktop-Diktat, nur kein Mobile
- EUR 29/39 ist fuer Windows allein gerechtfertigt (Voice Type: $20 fuer macOS-only mit weniger Features)
- iOS-Nachfrage tracken (jede Anfrage notieren) um Prioritaet zu validieren

## Wettbewerbs-Beobachtung

Regelmaessig pruefen (alle 4-6 Wochen):
- **Amical Android-Beta:** Wird sie public? Feature-Umfang?
- **Wispr Flow Android Pricing:** Aktuell Free Promo -- wann kommt das Abo?
- **OpenWhispr Mobile:** Gibt es Anzeichen fuer Android-Support?
- **dikta.me Launch-Status:** Noch Waitlist? Wann shipped? Feature-Umfang bei Release?
- **iOS-Nachfrage:** Jede Anfrage von Nutzern/Interessenten tracken

## Quellen
- Wettbewerbsanalyse Voice Type (2026-03-08)
- Wettbewerbsanalyse OpenWhispr (2026-03-09)
- Wispr Flow Pricing-Recherche: $12/mo Pro, $24/mo Enterprise (2026-03-09)
- Wispr Flow Android-Launch: 23. Feb 2026 (TechCrunch)
- Wispr Flow iOS: Custom Keyboard Extension im App Store (seit Mitte 2025)
- Amical: Open Source, MIT, Android Private Beta (2026-03-09)
- Voicy Pricing: $8.49/mo, $220 Lifetime (2026-03-09)
- Subscription Fatigue Trend: +6% One-Time-Purchase-Praeferenz (2025/26)
- Strategy-Session mit Andy (2026-03-09): EUR 29 Preis, Open Core Modell, Roadmap
- Wettbewerbs-Validierung (2026-03-09): "4 Checkboxen"-Claim angepasst, Amical als Wettbewerber identifiziert
- Strategy-Session mit Andy (2026-03-25): Early Bird EUR 29 → EUR 39, Lemon Squeezy, eine Lizenz alle Plattformen
- dikta.me Landingpage-Analyse (2026-03-25): 48+ Features, $20-25, Waitlist-Stadium, Landingpage-Best-Practices
- AppSumo evaluiert und ausgeschlossen (2026-03-25): SaaS-Plattform, 70% Revenue Share, Zielgruppen-Mismatch
- Google Play AccessibilityService Policy (2026-03-25): Verschaerft seit Jan 2026, aber Wispr Flow als Praezedenz
- iPhone-Markt-Analyse (2026-03-25): DACH ~25% Windows+iPhone Ueberlappung, macOS vor iOS priorisiert
- Lemon Squeezy vs. Paddle vs. Stripe evaluiert (2026-03-25): LS gewinnt wegen built-in License Keys + MoR
