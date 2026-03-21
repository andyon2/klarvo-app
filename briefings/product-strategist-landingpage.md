# Briefing: Voxlit-Landingpage

Erstellt: 2026-03-21, Product Strategist
Zweck: Inhaltliches Komplett-Briefing fuer die Voxlit-Produktseite. Kann direkt an den Web-Builder weitergegeben werden.

---

## Zielgruppe der Landingpage

Nicht dieselbe wie die Berater-Website. Hier kaufen Leute ein Tool:

1. **Primaer:** Abo-muede Vieltipper -- Entwickler, Autoren, Wissensarbeiter. Kennen Wispr Flow oder aehnliches, wollen aber kein Abo.
2. **Sekundaer:** Privacy-bewusste Nutzer -- Therapeuten, Anwaelte, Aerzte. Offline-Modus ist der Kaufgrund.
3. **Tertiaer:** Android-Nutzer, die eine echte Diktat-App suchen, die nicht cloud-only ist.

## Ton

Direkt, ehrlich, keine Marketing-Floskeln. Der Besucher ist tech-affin genug um Bullshit zu erkennen. Fakten statt Superlative. Kurze Saetze. Kein "revolutionaer", kein "nahtlos", kein "KI-gestuetzt" als Buzzword.

Voxlit ist ein Indie-Produkt von einem Entwickler -- das darf man merken. Das ist Staerke, nicht Schwaeche.

## Seitenstruktur

### 1. Hero

**Headline:**
> Sprachdiktat, das dir gehoert.

**Subheadline:**
> Einmal kaufen. Windows + Android. Online oder offline.

**CTA-Button:** "Kostenlos testen" (Free-Tier, kein Signup noetig)

**Sekundaerer Link:** "Quellcode auf GitHub" → github.com/andyon2/voxlit-public

**Optionales Element:** Kurzes Demo-GIF oder Screenshot (Hotkey druecken → Sprechen → bereinigter Text erscheint). Kein Video -- die Aufmerksamkeitsspanne ist kurz.

---

### 2. Problem

Kurzer Block, 2-3 Saetze. Der Besucher soll nicken.

> Du sprichst schneller als du tippst. Spracherkennung gibt's genug -- aber was rauskommt, sind Fuellwoerter, fehlende Satzzeichen und Rohtext, den du erst nachbearbeiten musst. Voxlit erkennt deine Sprache und liefert fertigen Text.

---

### 3. Wie es funktioniert

Drei Schritte, visuell nebeneinander. Simpel halten.

1. **Hotkey druecken** -- Voxlit hoert zu (Windows: Tastenkombination, Android: Floating Bubble antippen)
2. **Sprechen** -- Rede wie du willst, mit Fuellwoertern, Pausen, Satzabbruechen
3. **Fertiger Text** -- KI bereinigt automatisch: Grammatik, Interpunktion, Fuellwoerter raus. Text wird direkt eingefuegt.

---

### 4. Features

Nicht als endlose Liste, sondern gruppiert nach Kaufgruenden.

#### Ueberall diktieren
- Funktioniert in jedem Textfeld -- E-Mail, Chat, Code-Editor, Browser
- Windows Desktop + Android Mobile
- Systemweites Einfuegen (kein Copy-Paste noetig)

#### KI-Text-Cleanup
- Drei Stile: Polished (formell), Clean (neutral), Chat (locker)
- Eigene Anweisungen moeglich ("Schreibe in Stichpunkten", "Uebersetze auf Englisch")
- Persoenliches Woerterbuch fuer Fachbegriffe und Namen

#### Online oder Offline
- Online: Groq Whisper (schnell, guenstig) + DeepSeek Cleanup
- Offline: Whisper lokal auf deinem Rechner -- keine Daten verlassen dein Geraet
- Eigene API-Keys, kein Voxlit-Account noetig (BYOK)

#### Deine Daten, dein Rechner
- Keine Cloud, kein Account, keine Telemetrie
- Source-available: Quellcode auf GitHub einsehbar
- Laeuft komplett lokal wenn du willst

---

### 5. Pricing

Einfach, ein Preis, kein Verwirrung.

#### Kostenlos
- Kern-Diktat (Hotkey → Sprechen → Text)
- Alle STT/LLM-Provider (eigene API-Keys)
- Drei Cleanup-Stile + eigene Anweisungen
- Offline-Modus mit Whisper small
- Woerterbuch (20 Eintraege)
- Letzte 50 Diktate in der History

**CTA:** "Jetzt herunterladen"

#### EUR 29 -- einmalig, kein Abo

- Alles aus Kostenlos, plus:
- Offline mit groesseren Whisper-Modellen (deutlich bessere Qualitaet)
- Eigene Stil-Vorlagen
- Command Mode (Befehle statt Diktat)
- Unbegrenztes Woerterbuch
- Unbegrenzte History mit Volltextsuche
- Erweiterte Statistiken
- Kuenftige Premium-Features inklusive

**CTA:** "Lizenz kaufen"

#### Vergleich (optional, aber wirksam)

| | Voxlit | Wispr Flow |
|---|---|---|
| Preis | EUR 29 einmalig | $144/Jahr |
| Nach 2 Jahren bezahlt | EUR 29 | $288 |
| Offline | Ja | Nein |
| Quellcode einsehbar | Ja | Nein |
| Daten lokal | Ja | Cloud |
| Android | Ja | Ja (Cloud-only) |

Kein Bashing, nur Fakten. Der Besucher zieht die Schluesse selbst.

---

### 6. Trust / Social Proof

Hier ist Voxlit noch duenn -- ehrlich damit umgehen, nicht fake.

- **"In Active Development"** -- Versionsnummer (v0.4.x), regelmaessige Updates, Auto-Updater
- **Quellcode offen** -- GitHub-Link, Commit-History als Beweis fuer aktive Entwicklung
- **Spaeter hinzufuegen:** Tester-Zitate, Download-Zahlen, Screenshots von echten Nutzern

---

### 7. FAQ

Kurz, nur die Fragen die wirklich kommen:

**Brauche ich einen Account?**
Nein. Voxlit laeuft ohne Account. Fuer die Online-Funktionen brauchst du eigene API-Keys (Groq, DeepSeek -- beide haben kostenlose Tiers).

**Funktioniert Voxlit wirklich offline?**
Ja. Mit dem lokalen Whisper-Modell laeuft Spracherkennung komplett auf deinem Rechner. Fuer den Text-Cleanup offline brauchst du aktuell noch eine API -- lokale LLMs kommen spaeter.

**Was ist "source-available"?**
Der komplette Quellcode ist auf GitHub einsehbar. Du kannst pruefen was Voxlit tut. Redistribution und kommerzielle Nutzung sind nicht erlaubt (BSL 1.1 Lizenz). Private Nutzung und Modifikation fuer den Eigengebrauch: ja.

**Was passiert nach dem Kauf?**
Du bekommst einen Lizenzschluessel. Einmal eingeben, fertig. Alle kuenftigen Updates in der aktuellen Hauptversion sind inklusive.

**Welche Sprachen werden unterstuetzt?**
Alles was Whisper kann -- 99 Sprachen. Deutsch und Englisch funktionieren am besten.

**Laeuft Voxlit auf macOS/Linux?**
Aktuell nur Windows und Android. macOS ist auf der Roadmap, aber nicht kurzfristig.

---

### 8. Footer-CTA

> Probier's aus. Kostenlos, ohne Account.

**CTA:** "Jetzt herunterladen"

**Sekundaer:** GitHub-Repo | Kontakt

---

## Was NICHT auf die Landingpage gehoert

- Technische Architektur-Details (Tauri, Rust -- hoechstens erwaehnen, nicht erklaeren)
- Roadmap oder "Coming Soon"-Listen (weckt Erwartungen, die enttaeuschen koennen)
- Berater-Profil oder Link zur Berater-Website (andere Zielgruppe, andere Kaufentscheidung)
- "Open Source" (ist BSL 1.1, nicht Open Source -- immer "source-available" verwenden)

## Verbindung zur Berater-Website

Die Berater-Website verlinkt das GitHub-Repo in der Portfolio-Sektion. NICHT die Landingpage. Die Berater-Website verkauft Andy als Berater, nicht Voxlit als Produkt. Wenn jemand vom Repo zur Landingpage findet -- gut. Aber kein aktives Cross-Selling.

Die Landingpage erwaehnt nicht, dass Voxlit von einem KI-Berater gebaut wird. Voxlit steht fuer sich.

## Offene Punkte

- [ ] Domain klären (voxlit.app? getvoxlit.de? Subdomain der Berater-Seite?)
- [ ] Demo-GIF/Screenshot erstellen
- [ ] Zahlungsabwicklung fuer License Keys (Gumroad? Paddle? LemonSqueezy?)
- [ ] Timing: Landingpage live wenn License-Key-System in Voxlit implementiert ist
