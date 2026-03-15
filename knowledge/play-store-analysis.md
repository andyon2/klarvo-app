# Google Play Store Analyse fuer Dikta

Erstellt: 2026-03-15
Autor: Product Strategist

---

## Executive Summary

Play Store Eintritt ist fuer Dikta moeglich, aber nicht trivial. Der groesste Blocker ist der AccessibilityService -- er wird von Google nicht verboten, aber er unterliegt seit Januar 2026 einem deutlich verschaerften Review-Prozess. Wispr Flow ist bereits im Play Store (mit AccessibilityService), das ist das wichtigste Praezedenzfall-Signal: Es geht, wenn man es richtig macht.

Die strategische Frage ist nicht "Schafft Dikta den Play Store?" -- sondern "Wann und in welcher Reihenfolge?" Die Empfehlung steht am Ende des Dokuments.

---

## 1. Realistische Schwierigkeitsbewertung

**Schwierigkeit: Mittel bis hoch -- machbar, aber nicht in einer Woche.**

Drei Permissions machen Dikta zu einem "sensitiven" App-Profil:

| Permission | Risikostufe | Begruendung |
|------------|-------------|-------------|
| `SYSTEM_ALERT_WINDOW` (Floating Bubble) | Mittel | Seit Android 10 kein automatisches Grant mehr, muss vom User manuell erteilt werden -- kein Play-Store-Reviewer-Problem, aber ein Onboarding-Problem |
| `AccessibilityService` | Hoch | Strengstes Review-Verfahren seit Jan 2026. Pflicht: Declaration Form + Video + Begruendung |
| `FOREGROUND_SERVICE_MICROPHONE` | Niedrig | Standard fuer alle Diktat-Apps. Kein besonderes Risiko wenn korrekt deklariert |

Zusaetzlich: Dikta sendet Audiodaten an Drittanbieter (Groq, DeepSeek). Das muss vollstaendig in der Data Safety Section deklariert werden. Kein Blocker, aber Arbeit.

---

## 2. Der AccessibilityService -- Detailanalyse

### Was Google tatsaechlich verbietet

Google verbietet AccessibilityService **nicht** fuer Non-Accessibility-Apps. Was verboten ist:

- Daten aus anderen Apps sammeln (ohne direkten Nutzerzweck in diesem Moment)
- Autonomous actions ohne Nutzerinteraktion ausfuehren
- Irreführende Deklaration (isAccessibilityTool=true obwohl die App keine echte Accessibility-App ist)

### Was Google erfordert (seit Jan 2026, verscharft)

Zwei Wege:

**Weg A: `isAccessibilityTool=true`**
- Nur fuer Apps, die Nutzern mit Behinderungen helfen
- Dikta koennte argumentieren: motorisch eingeschraenkte Nutzer profitieren von Sprachdiktat
- Erfordert: Video-Demo + Erklaerung der Zielgruppe (Menschen mit Behinderungen) + Disability-Fokus in der App-Beschreibung
- Risiko: Wenn Google das als "Missbrauch des Disability-Labels" wertet, harter Rejection
- Wispr Flows Weg laut ihrer Dokumentation: Sie bezeichnen sich explizit als "accessibility tool" in ihrer Beschreibung ("Revolutionizing Accessibility Tools with AI Voice Recognition Technology" -- ihr eigener Blog-Titel)

**Weg B: `isAccessibilityTool=false` + Declaration Form**
- Ehrlicherer Weg fuer Dikta (wir sind kein Disability-Tool)
- Erfordert: Declaration Form in Play Console ausfuellen + Prominent Disclosure in der App (In-App-Erklaerung was der Service tut + User Consent)
- Der Reviewer prueft: Ist der AccessibilityService-Einsatz auf den deklarierten Zweck beschraenkt?
- Diktas Zweck ist klar: Text-Paste in aktives Textfeld nach Diktat. Das ist begrenzt, nicht fishing.

### Wie Wispr Flow es macht

Wispr Flow ist im Play Store, nutzt AccessibilityService, und beschreibt es so:
- Text-Field-Erkennung (wo tippt der User gerade?)
- Text-Insertion nach Diktat (Ergebnis einfuegen)
- Kontext-Lesen fuer bessere Genauigkeit (umliegender Text)
- Security-Feature: Automatisches Verstecken in Banking-Apps

Wispr Flow hat sich als "accessibility tool" positioniert (ihr Marketing nennt es explizit so). Das ist eine bewusste Entscheidung. Ob das Google-seitig als isAccessibilityTool=true deklariert ist, ist von aussen nicht sichtbar, aber ihre Wortwahl deutet darauf hin.

**Fuer Dikta:** Der ehrliche Weg (Weg B, Non-Accessibility-Tool) ist langfristig sicherer. Der Einsatz von AccessibilityService ist bei Dikta klar begrenzt und auf einen Nutzer-initiierten Vorgang beschraenkt (Nutzer drueckt Hotkey -> Diktat -> Text wird eingefuegt). Das ist gut vertretbar.

### Technische Alternative zu AccessibilityService

Ja, es gibt eine: **InputMethodService (IME)** -- d.h. Dikta wird zu einer eigenen Tastatur.

| Ansatz | Pro | Contra |
|--------|-----|--------|
| AccessibilityService (aktuell) | Kein Keyboard-Wechsel noetig, nahtlose UX | Play Store Compliance-Aufwand, strenger Review |
| IME (Dikta als Keyboard) | Kein Compliance-Problem, Standard-API | Nutzer muss Dikta als Tastatur aktivieren + waehrend Diktat zur Dikta-Tastatur wechseln -- schlechte UX |

Der IME-Weg ist technisch moeglich aber kaputt fuer die Core-UX. Wispr Flow hat sich dagegen entschieden, wir sollten es auch nicht tun. Der AccessibilityService-Weg mit korrekter Deklaration ist der richtige Weg.

---

## 3. Konkrete Schritte VORHER (Pre-Release Checklist)

### Schritt 1: Google Play Developer Account ($25, einmalig)
- Einmalige Registrierungsgebuehr
- Benoetigt: Google-Konto, Identity Verification (seit 2023 Pflicht fuer neue Accounts)
- Zeitaufwand: 1-2 Tage bis Freischaltung

### Schritt 2: Data Safety Section ausfuellen
Dikta muss deklarieren:
- **Audio-Daten** werden gesammelt (Sprache des Nutzers)
- Audio wird an **Drittanbieter** uebertragen (Groq fuer STT, DeepSeek fuer Cleanup)
- Daten werden **nicht** gespeichert (wenn das stimmt -- sicherstellen dass Groq/DeepSeek keine Daten persistieren oder das im UI klar kommunizieren)
- Kein Verkauf von Nutzerdaten

Das klingt schlimmer als es ist. Jede Diktat-App muss das machen. Kein Blocker, aber braucht eine Privacy Policy (externe URL noetig).

### Schritt 3: Privacy Policy erstellen
- Externe URL mit Privacy Policy ist Pflicht (kein Inline-Text genuegt)
- Muss erklaeren: Was wird aufgezeichnet, wohin geht es, wie lange wird es gespeichert
- Kostenloses Tool: Termly, PrivacyPolicies.com (beide haben GDPR-konforme Templates)
- Zeitaufwand: 30 Minuten mit Template

### Schritt 4: AccessibilityService Declaration
- Im Play Console: Policy -> Declarations -> Accessibility -> ausfuellen
- Benoetigt: Video-Demo (Screen Recording ca. 1-3 Minuten) der zeigt wie AccessibilityService genutzt wird
- Beschreibung des Zwecks: "Insert dictated text into the active text field after user initiates dictation via hotkey"
- Benoetigt: In-App Disclosure (Onboarding-Screen der erklaert was der Service tut und warum) + User Consent (explizites OK vom User)

### Schritt 5: SYSTEM_ALERT_WINDOW Onboarding
- Kein Play-Store-Blocker, aber Onboarding-Problem
- Android zeigt seit API 30 keinen automatischen Permission-Dialog mehr
- Dikta muss den Nutzer aktiv zur Settings-Seite fuehren ("Display over other apps" aktivieren)
- Ohne das: Floating Bubble erscheint nicht, App wirkt kaputt

### Schritt 6: Target API Level
- Google verlangt aktuell targetSdkVersion 34+ fuer neue Apps (Stand Anfang 2026)
- Pruefen ob Diktas Android-Build das erfuellt

### Schritt 7: App-Beschreibung und Screenshots
- Klar kommunizieren was AccessibilityService macht und warum
- Keine uebertriebene Betonung von "wir sehen alles in anderen Apps" -- Fokus auf den Nutzernutzen

---

## 4. Was NACHHER auf uns zukommt

### Support-Last
- Play Store Nutzer erwarten Reaktion auf Reviews (auch 1-Sterne-Reviews)
- Deutsche + internationale Nutzer -- Sprach-Support entscheidet sich hier
- Realistisch: 2-5 Stunden/Woche Support-Aufwand wenn die App Traktion bekommt

### Policy-Compliance (laufend)
- Google aendert Policies regelmaessig. Oktober 2025 gab es ein grosses Update.
- AccessibilityService-Policies koennen sich verschaerfen -- dann droht Delistung ohne Vorwarnung
- Pflicht: Policy-Announcements regelmaessig lesen (Play Console schickt Mails)
- Wenn Google etwas aendert: Reaktionszeit ist meist 30-90 Tage

### Google Play Billing (wenn Dikta paid wird)
- Wenn Dikta direkt im Play Store verkauft wird: Google nimmt 15% (erste $1M/Jahr) bzw. 30%
- Fuer EUR 29 License Key: 15% = EUR 4.35 pro Verkauf
- Alternative: License Key wird auf eigener Website verkauft (Gumroad, LemonSqueezy), App im Play Store ist kostenlos. Dann umgeht man Google Billing komplett.
- Wichtig: Google erzwingt Google Play Billing nur wenn das In-App-Kaufobjekt ein "digital good" ist das in der App konsumiert wird. Ein License Key der auf einer externen Website gekauft wird und dann in der App eingegeben wird, ist eine Grauzone -- aber Wispr Flow und andere machen es genau so (externe Subscription, App kostenlos im Store).

### Update-Maintenance
- Jedes App-Update durchlaeuft erneut den Review-Prozess (typisch 1-3 Tage)
- Bei AccessibilityService-Aenderungen: Potentially erneuter manueller Review (laenger)
- Plan: Minor Updates bündeln, nicht jeden Bugfix einzeln publishen

### Android-17-Risiko (mittel-langfristig)
- Android Police berichtet: Android 17 bringt "Advanced Protection Mode" der AccessibilityService-Permissions weiter einschraenkt
- Konkret: Apps koennen im Advanced Protection Mode blockiert werden wenn sie AccessibilityService nutzen
- Das betrifft wahrscheinlich Unternehmens-verwaltete Geraete, nicht Consumer-Geraete
- Beobachten, aber kein akuter Handlungsbedarf

---

## 5. Play Store vs. Sideloading -- Abwaegung

### Sideloading (aktueller Stand)
| Faktor | Bewertung |
|--------|-----------|
| Setup-Aufwand | Null (APK auf GitHub) |
| Nutzerfreundlichkeit | Schlecht (Unknown Sources aktivieren, manuell installieren) |
| Zielgruppe | Tech-savvy Early Adopters |
| Update-Verteilung | Manuell oder Auto-Update im App selbst |
| Compliance-Aufwand | Null |
| Marktgroesse | Klein (nur Leute die Sideloading kennen) |

### Play Store
| Faktor | Bewertung |
|--------|-----------|
| Setup-Aufwand | Hoch einmalig (3-6 Wochen fuer erste Submission) |
| Nutzerfreundlichkeit | Sehr gut (normaler App-Install) |
| Zielgruppe | Alle Android-Nutzer |
| Update-Verteilung | Automatisch |
| Compliance-Aufwand | Laufend (Policies beobachten) |
| Marktgroesse | Gross (aber auch mehr Wettbewerb) |

**Fazit:** Sideloading ist richtig fuer jetzt. Play Store ist richtig fuer spaeter.

---

## 6. Verteilungsalternativen

### F-Droid
- F-Droid ist die FOSS-Alternative zum Play Store
- Anforderung: App muss vollstaendig Open Source sein, kein proprietaerer Code, keine Tracking-Libraries
- Diktas Open-Core-Modell ist ein Problem: Der License-Key-Code ist proprietary
- Loesbar durch: F-Droid-Variante ohne License-Key-Features (FOSS-only Build)
- Nutzerschaft: Sehr tech-savvy, Privacy-fokussiert -- passt gut zu Diktas Sekundaerzielgruppe
- Aufwand: Einmaliger Setup (fdroiddata PR) + Build-Reproducibility sicherstellen
- Empfehlung: Mittelfristig attraktiv fuer Privacy-Zielgruppe, aber kein v1.0-Thema

### Eigene Website + direkter APK-Download
- Wie GitHub Releases, aber mit besserer Landing Page
- Kein Compliance-Problem
- Gute Option fuer Privacy-bewusste Nutzer die kein Google haben wollen
- Kann parallel zu Play Store laufen

### Amazon Appstore
- Relevant fuer Fire-Tablets, kleine Nische
- Eigene Review-Prozesse, eigenes Billing -- nicht empfohlen fuer jetzt

---

## 7. Timing-Empfehlung

### Jetzt (v0.4.x - v1.0): Sideloading beibehalten
- Kein Aufwand fuer Compliance
- Early Adopters tolerieren Sideloading
- Feedback-Qualitaet ist besser wenn Nutzer tech-savvy sind
- Fokus: Produkt fertig machen, nicht Vertriebskanal ausbauen

### Nach v1.0 (Paid Release): Play Store vorbereiten
- Wenn License-Key-System steht und Dikta kaufbar ist
- Dann lohnt sich Play Store, weil breitere Zielgruppe erreichbar wird
- Parallel: Privacy Policy fertigstellen, Data Safety Section vorbereiten
- Zeitplanung: 4-6 Wochen fuer ersten Review-Durchlauf einplanen

### Konkrete Voraussetzungen fuer Play Store Start
- [ ] Privacy Policy URL (externe Seite) vorhanden
- [ ] In-App AccessibilityService Disclosure + Consent Screen implementiert
- [ ] Data Safety Section vollstaendig ausgefuellt
- [ ] Video-Demo fuer AccessibilityService Declaration aufgenommen
- [ ] targetSdkVersion 34+ sichergestellt
- [ ] SYSTEM_ALERT_WINDOW Onboarding-Flow implementiert (User wird zur Settings-Seite gefuehrt)
- [ ] Entscheidung getroffen: isAccessibilityTool=true oder false (Empfehlung: false, ehrlicher Weg)

---

## 8. Strategische Empfehlung

**Kurzfassung: Play Store ist das Ziel, aber nicht die naechste Aufgabe.**

Dikta sollte den Play Store nicht als "netter Bonus" behandeln, sondern als strategischen Wachstumskanal -- aber erst dann, wenn das Produkt stabil und paid ist. Der Aufwand fuer den ersten Submission-Durchlauf ist real (4-6 Wochen), der laufende Compliance-Aufwand ist beherrschbar.

Das groesste Risiko ist nicht Ablehnung, sondern **spaetere Delistung** nach einer Policy-Aenderung. Dem kann man mit einem sauberen Implementierungsansatz entgegenwirken:
- AccessibilityService nur waehrend aktivem Diktat, nicht dauerhaft
- Kein Lesen von Daten aus anderen Apps ausser dem aktuellen Textfeld-Kontext
- Klare In-App-Kommunikation was der Service tut

**Wispr Flow als Praezedenz:** Sie sind im Store, sie nutzen AccessibilityService, sie senden Audio an Cloud. Das ist exakt Diktas Profil. Wenn Wispr Flow es schafft, schafft Dikta es auch -- mit dem Unterschied dass Dikta weniger Ressourcen fuer einen ggf. langen Back-and-forth mit Google-Reviewern hat.

**Konkrete naechste Schritte (wenn v1.0 fertig ist):**
1. Privacy Policy auf dikta.app (oder eigene Domain) publizieren -- 1 Stunde
2. AccessibilityService In-App-Disclosure implementieren (Onboarding-Step) -- 2-4 Stunden
3. Google Play Developer Account registrieren -- 1 Tag
4. Data Safety Section ausfuellen -- 2-3 Stunden
5. Video-Demo aufnehmen (Screen Recording, 2 Minuten) -- 30 Minuten
6. Erste Submission -- Ergebnis in 1-5 Tagen

Gesamtaufwand einmalig: ~2-3 Arbeitstage plus Wartezeit.

---

## Quellen

- Google Play Console Help: Use of the AccessibilityService API (2025)
- Google Play Policy Announcement: October 30, 2025
- Wispr Flow Docs: Why Does Google Play Keep Asking About Permissions
- Wispr Flow Docs: Setup Guide (Android AccessibilityService Erklaerung)
- 9to5Google: Wispr Flow Android Launch (Feb 2026)
- Android Police: Android 17 Advanced Protection Mode (AccessibilityService)
- BrowserStack: Impact of Accessibility Permission in Android Apps
- MyAppMonitor: Google Play Accessibility Services Policy Update 2026
- Google Play Developer Fees: $25 one-time registration (2025)
- F-Droid: Free and Open Source Android App Repository
