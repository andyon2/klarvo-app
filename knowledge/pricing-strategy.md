# Pricing-Strategie -- Dikta

**Stand: 2026-03-20 | Entscheidungstraeger: Andy | Quelle: Product Strategist Analyse**

## Entscheidung: Szenario B -- Sofort EUR 29, ehrlicher Preis

Kein Early-Bird-Theater. Das Produkt kostet EUR 29, weil es das wert ist. Spaetere Erhoehungen an Feature-Meilensteine geknuepft.

## Preispfad

| Phase | Preis | Trigger | Zeitraum |
|-------|-------|---------|----------|
| v1.0 Launch | EUR 29 Complete (Website) | Signing + Auto-Update + Offline-Whisper stabil | Launch-Tag |
| v1.x (laufend) | EUR 29 | Kein Preiswechsel bei Minor-Updates | 0-12 Monate |
| Play Store (2. Welle) | EUR 14 Android IAP | Play Store Listing + IAP-Integration fertig | Nach stabilem v1.0 |
| v2.0 | EUR 39 Neupreis | macOS-Port ODER Teams-Feature live | ~12-18 Monate nach v1.0 |

## Plattform-Staffelung (wenn Play Store live)

| Lizenz | Preis | Wo | Plattformen |
|--------|-------|----|-------------|
| Android Complete | EUR 14 | Play Store (IAP) | Nur Android |
| Desktop Complete | EUR 19 | Website | Nur Windows (+ macOS wenn verfuegbar) |
| Full Complete | EUR 29 | Website | Windows + Android (+ macOS) |

Upgrade-Pfad: Play-Store-Kaeufer (EUR 14) die Windows wollen, kaufen auf der Website mit Upgrade-Coupon fuer EUR 12 (statt EUR 29).

## Support-Versprechen

> "EUR 29 schaltet alle aktuellen Paid-Features frei. Alle v1.x-Updates inklusive. Bugfixes und Sicherheitsupdates immer, ohne Aufpreis. v2.0 ist ein optionales Upgrade (EUR 12-15) -- deine v1.x-Lizenz laeuft weiter. Die App hoert nie auf zu funktionieren."

Konkret:
- **v1.0 bis v1.9:** Alle Updates kostenlos, neue v1.x-Features inklusive
- **v2.0:** Optionales Upgrade EUR 12-15. Nicht zwingend.
- **Bugfixes:** Immer und fuer jede Version
- **Kein Kill-Switch:** v1.x laeuft weiter, auch wenn v2.0 erscheint

### Was NICHT versprochen wird
- "Updates fuer immer" -- zu vage, nicht einhaltbar
- "Alle zukuenftigen Versionen" -- zerstoert v2.0-Upgrade-Revenue
- "Support fuer immer" -- Ein-Mann-Betrieb kann das nicht leisten

## v2.0 Definition (jetzt festgezurrt)

v2.0 ist gerechtfertigt wenn mindestens zwei dieser Punkte zutreffen:
1. Neue Plattform (z.B. macOS-Port)
2. Komplett ueberarbeitete Architektur (z.B. eigene STT-Engine)
3. Neue Nutzergruppe erschlossen (z.B. Teams-Feature mit Admin-Dashboard)
4. Mindestens 12 Monate seit v1.0

Feature-Updates (Integrationen, UI-Verbesserungen, neue Cleanup-Modelle) sind v1.x, kein v2.0-Grund.

## Google Play Store

### Modell: Freemium + IAP Unlock (EUR 14)
- Free Tier kostenlos im Play Store (Kern-Diktat mit Groq)
- Paid Features als In-App-Purchase (EUR 14)
- Google nimmt 15% (unter $1M Umsatz) -- bei EUR 14 netto ~EUR 11.90

### Strategischer Wert
- **Discovery:** Nutzer suchen "voice dictation" im Play Store
- **Vertrauen:** "Im Play Store erhaeltlich" signalisiert Legitimitaet
- **Reviews:** Oeffentliches Social Proof
- Revenue-Hauptkanal bleibt die Website. Play Store ist Trichter.

### Cross-Platform-Loesung
- Play Store = Android-only (EUR 14)
- Website = Complete (EUR 29, Windows + Android)
- Kein TOS-Verstoss: Unterschiedliche Produkte, nicht unterschiedliche Preise fuer dasselbe

## Offene Punkte (vor Launch klaeren)

- [ ] Play Store parallel oder nach Windows-Launch? (Empfehlung: nach)
- [ ] Play Store Receipt-Verifikation: Backend-Arbeit noetig (License-Key <-> Google Billing)
- [ ] Deutsche Steuer: Kleinunternehmerregelung? EU-OSS? -> Steuerberater, nicht Agent-Team
- [ ] Zahlungsanbieter fuer Website: Gumroad (10%), Lemon Squeezy (5-8%), oder Paddle?

## Warum kein Plattform-Split beim Launch

Analyse vom Product Strategist (2026-03-20): Plattform-Split zum Launch abgelehnt.
- Bricht den Kernsatz "EUR 29 einmal, fertig"
- Toetet Impulskauf (Kalkulation statt Klick)
- Komplexitaet zu hoch fuer Ein-Mann-Betrieb
- Plattform-Staffelung kommt erst wenn Play Store live geht (2. Welle)

## Revenue-Projektion (konservativ, Jahr 1)

| Kanal | Annahme | Brutto | Netto (nach Fees) |
|-------|---------|--------|-------------------|
| Website Complete | 200 Kaeufer x EUR 29 | EUR 5.800 | ~EUR 5.300 |
| Play Store Android | 10 Kaeufer x EUR 14 | EUR 140 | ~EUR 119 |
| **Gesamt** | | **EUR 5.940** | **~EUR 5.420** |

Bei staerkerer Community-Verbreitung (ProductHunt, HN, Reddit): 500-1.000 Kaeufer realistisch = EUR 14.500-29.000 brutto.
