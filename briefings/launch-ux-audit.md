# Launch UX Audit -- Klarvo v1.0

Erstellt: 2026-03-25
Basis: knowledge/competitors.md, knowledge/wispr-flow-android-ux.md, knowledge/product-strategy.md, project-status.md

---

## Methodik

Referenzpunkte fuer "was ein Erstnutzer erwartet":
- **Wispr Flow**: Ausgereifteste UX im Markt. Erster Eindruck bei Android = Floating Bubble funktioniert sofort, Bubble-Zustaende klar kommuniziert (Idle / Recording / Error / Processing).
- **Voice Type**: "3-Schritt-Setup: Install -> Hotkey -> Go." Kein Overhead, kein Erklaerungsbedarf.
- **dIKta.me**: 950 Tests, 0 Warnings im Release-Build, CI/CD -- zeigt dass Wettbewerber auch auf Professionalitaet setzen.
- **Unsere Positionierung**: "Sprachdiktat das dir gehoert." Erster Eindruck muss Vertrauen erzeugen -- ein instabiler oder konfuser Erststart widerspricht direkt dem Kern-Versprechen.

---

## Launch-Blocker

Ohne diese Punkte kein Launch. Sie schaffen entweder Vertrauen oder blockieren die Grundfunktion.

### 1. Onboarding mit Permission-Erklaerung (Android)

**Was fehlt:** Android erfordert 4 Permissions fuer Grundfunktion (Mikrofon, Accessibility Service, Display over other apps, Batterie-Optimierung). Jede davon oeffnet einen System-Dialog der ohne Kontext bedrohlich wirkt ("Diese App kann alle deine Eingaben lesen"). Wispr Flow begleitet diesen Prozess mit expliziten Erklaerungen pro Permission -- warum, wozu, was passiert nicht.

**Warum Blocker:** Ohne Erklaerung werden Nutzer Permissions verweigern und die App funktioniert gar nicht (Accessibility Service = komplette Deaktivierung der Kernfunktion laut wispr-flow-android-ux.md Abschnitt 4). Hersteller-spezifische Extras (OnePlus, Xiaomi) brauchen eigene Hinweise.

**Mindest-Anforderung:** Onboarding-Screen der (a) erklaert was jede Permission bewirkt und (b) keine Daten in die Cloud schickt wenn nicht explizit konfiguriert (unser Privacy-Vorteil), (c) bei fehlender Permission mit klarem Hinweis zur Settings-Seite leitet statt stumm zu scheitern.

### 2. Sichtbares Fehler-Feedback wenn kein API-Key konfiguriert

**Was fehlt:** Ein Erstnutzer installiert Klarvo, drueckt den Hotkey -- und nichts passiert, weil kein API-Key konfiguriert ist. Oder es erscheint eine technische Fehlermeldung die keinen Handlungshinweis gibt.

**Warum Blocker:** Voice Type's "3-Schritt-Setup" funktioniert weil Schritt 2 explizit ist. Unser Kern-Versprechen bricht sofort wenn der erste Diktat-Versuch ohne Erklaerung scheitert. API-Key-Konfiguration ist keine optionale Einstellung -- sie ist Voraussetzung.

**Mindest-Anforderung:** Wenn kein funktionierender Provider konfiguriert ist, zeigt die App beim ersten Hotkey-Druck (oder Bubble-Tap) einen klaren Hinweis: "Kein API-Key konfiguriert -- jetzt einrichten" mit direktem Link zu den Settings. Kein stilles Scheitern.

### 3. Klarer Onboarding-Flow Desktop (Hotkey -> API-Key -> Erster Diktat)

**Was fehlt:** Der Onboarding-Persistenz-Bug wurde gefixt (Commit f233140), aber der Flow selbst muss geprueft werden: Kommt ein Erstnutzer ohne externes Dokument oder README zum ersten erfolgreichen Diktat?

**Warum Blocker:** Voice Type's "Install -> Hotkey -> Go" ist die Messlatte. Wenn der Erstnutzer die App oeffnet und nicht innerhalb von 60 Sekunden weiss was er tun soll, ist der erste Eindruck verloren. Besonders relevant weil unsere Zielgruppe (abo-muede Wispr-Flow-Nutzer) erwartet dass es "einfach funktioniert".

**Mindest-Anforderung:** Onboarding-Wizard oder Willkommens-Screen der (a) konfigurierten Hotkey zeigt, (b) auf API-Key-Setup hinweist wenn keiner vorhanden, (c) mit einem Test-Diktat endet das bestaetigt "alles funktioniert".

### 4. Error-States fuer alle kritischen Fehlerpunkte (kein stilles Scheitern)

**Was fehlt:** Aus wispr-flow-android-ux.md (Abschnitt 3): Wispr Flow kommuniziert Fehler-Zustaende visuell -- roter Rand wenn Mikrofon stumm oder getrennt. Aus Backlog: Auto-Updater funktioniert bei Tester nicht (Ursache unklar). FloatingBar Drag nur waehrend Recording.

**Warum Blocker:** Stilles Scheitern (Diktat geht nicht, aber keine Erklaerung warum) ist der haeufigste Grund fuer negative App-Store-Reviews. Jeder kritische Fehlerpfad braucht sichtbares Feedback:
- Mikrofon nicht verfuegbar
- API-Key abgelaufen / Rate Limit
- Netzwerk nicht erreichbar (wenn Cloud-Provider aktiv)
- Textfeld-Focus verloren vor Paste (Android)

**Mindest-Anforderung:** Kein kritischer Fehlerpfad endet in einer leeren UI oder einem technischen Stack-Trace. Jeder Fehler hat eine menschliche Erklaerung und einen Handlungshinweis.

### 5. Android: Erklaerung des IME-Keyboard-Wechsels

**Was fehlt:** Klarvo nutzt einen IME fuer Text-Einfuegung (im Unterschied zu Wispr Flow's Accessibility-Ansatz). Das bedeutet: Nutzer muss Klarvo einmalig als Keyboard aktivieren. Dieser Schritt ist unsichtbar und unbekannt.

**Warum Blocker:** Wenn ein Nutzer die App installiert, die Bubble sieht, auf Aufnahme drueckt, diktiert -- und der Text wird nicht eingefuegt weil das IME nicht aktiviert ist, ist die Core-Experience gebrochen. Ohne Erklaerung weiss der Nutzer nicht was er tun soll.

**Mindest-Anforderung:** Im Android-Onboarding explizit: "Aktiviere Klarvo als Eingabemethode (einmalig, kann jederzeit deaktiviert werden)" mit Step-by-Step-Anleitung zur Android-Settings-Seite.

---

## Dringend empfohlen

Diese Punkte sind kein harter Blocker, aber ein Launch ohne sie wird die ersten Reviews und Weiterleitungen negativ beeinflussen.

### 6. Tastenkuerzel-Uebersicht / Keyboard Shortcuts Cheat Sheet

**Was fehlt:** Klarvo hat mehrere Modi (Toggle/Hold/Auto-Stop/Full-Auto), Voice Commands ("Klarvo, toggle"), Global Hotkey. Ein Erstnutzer findet das nicht ohne Suche.

**Warum dringend:** Unsere Zielgruppe sind Vieltipper und Entwickler -- Menschen die Shortcuts aktiv nutzen. Eine Keyboard-Shortcut-Referenz (erreichbar ueber F1, ueber Tray-Menue oder in den Settings) ist ein Standard-Feature das professionell wirkt und Onboarding-Fragen reduziert.

### 7. About-Dialog mit Version, Lizenz-Hinweis, Changelog-Link

**Was fehlt:** Kein About-Dialog dokumentiert im aktuellen Stand.

**Warum dringend:** Drei Signale:
- Versionsnummer ist wichtig fuer Bug-Reports ("ich nutze v1.0.2")
- Lizenz-Hinweis (BSL 1.1, "source-available, not open source") ist rechtlich relevant und Bestandteil unserer Positionierung
- Changelog-Link signalisiert: aktives Projekt, hier passiert was

### 8. Tooltip-Text fuer nicht-offensichtliche Einstellungen

**Was fehlt:** Settings wie "Cleanup Style (Polished/Verbatim/Chat)", "Voice Activity Detection", "Whisper Mode" sind ohne Erklaerung nicht selbsterklaerend -- besonders fuer Nicht-Entwickler.

**Warum dringend:** Unsere Sekundaerzielgruppe (Privacy-bewusste Berufsgruppen: Therapeuten, Anwaelte) hat weniger technischen Hintergrund. Fehlende Erklaerungen fuehren zu falschen Konfigurationen und Enttaeuschung.

### 9. Bestaetigungs-Feedback nach erfolgreichem Diktat

**Was fehlt:** Aus wispr-flow-android-ux.md: Wispr Flow gibt nach Diktat visuelles Feedback bevor Text eingefuegt wird. Aus unserem Backlog: FloatingBar ist im Idle hidden. Ob es ein "Text wurde eingefuegt"-Signal gibt ist nicht dokumentiert.

**Warum dringend:** Besonders beim ersten Mal ist unklar ob "es funktioniert hat". Ein kurzes visuelles oder auditives Signal ("Text kopiert" / "Text eingefuegt") schliesst die Feedback-Schleife und gibt Sicherheit.

### 10. Einstellungs-Seite: Leerzustand wenn kein API-Key konfiguriert

**Was fehlt:** Wenn ein Nutzer die Settings oeffnet ohne API-Key ist nicht klar was der naechste Schritt ist.

**Warum dringend:** Das ist der erste Settings-Besuch fuer alle Erstnutzer. Eine leere Provider-Liste ohne Handlungsaufforderung ("Fuege deinen ersten API-Key hinzu") wirkt unfertig.

### 11. Android: Bubble-Groessen-Control im UI

**Was fehlt:** Aus Backlog: "Bubble Size/Opacity UI-Controls implementieren (Backend-Config existiert, Frontend fehlt)".

**Warum dringend:** Wispr Flow hat einen 4-Stufen-Slider fuer Bubble-Groesse. Ohne dieses Control ist unsere Bubble nicht anpassbar und wirkt im Vergleich unfertig -- besonders fuer Nutzer die von Wispr Flow kommen und den Slider kennen.

---

## Nice-to-have

Diese Punkte verbessern die Erfahrung nach dem Launch. Kein Kaufhindernis, aber Retention-relevante Details.

### 12. In-App Changelog / What's New Screen

Nach Updates automatisch zeigen was neu ist. Wispr Flow macht das. Zeigt: aktives Projekt.

### 13. Tray-Menue mit Schnellzugriff auf Modi

Hotkey-Modus direkt aus dem Tray-Menue wechseln ohne Settings oeffnen. Convenience.

### 14. Drag-to-Dismiss fuer FloatingBar (Desktop)

Aus Backlog: FloatingBar Drag nur moeglich waehrend Recording/Processing. Idle-Position koennte frei positionierbar sein. Nice-to-have fuer Nutzer die die Bar als storend empfinden.

### 15. Leerer-Zustand in History

Wenn History noch leer ist: "Dein erstes Diktat erscheint hier" statt leere Liste. Kleines Detail, grosse Wirkung auf ersten Eindruck.

### 16. Android: Hersteller-spezifische Setup-Hinweise

Fuer OnePlus / Xiaomi / Oppo: zusaetzliche Auto-Start-Permission ist noetig. Wispr Flow dokumentiert 130+ blockierte Apps und hersteller-spezifische Extras. Ein "known issues" Abschnitt in der App (oder verlinkt auf der Support-Seite) wuerde Support-Anfragen reduzieren.

### 17. Keyboard-Shortcut zum Oeffnen der Settings

Standard-Pattern (z.B. Strg+Comma auf Windows). Erwartet von Tech-affiner Zielgruppe.

---

## Zusammenfassung

**5 Blocker muessen vor Launch geloest sein:** Android Permission-Onboarding, API-Key-Fehler-Feedback, Desktop-Onboarding-Flow, Error-States fuer alle kritischen Pfade, Android IME-Erklaerung.

**Die gemeinsame Logik hinter allen 5 Blockern:** Klarvo hat eine komplexere Setup-Anforderung als z.B. eine reine Cloud-App (API-Keys, Permissions, IME-Aktivierung). Jeder Schritt der unklar bleibt kostet Nutzer die nie zum ersten erfolgreichen Diktat kommen -- und damit nie den Wert des Produkts erfahren. Das ist der einzige UX-Blocker der den Kauf rueckgaengig machen kann.

**Die 6 "Dringend empfohlenen" Punkte** (Tastenkuerzel-Referenz, About-Dialog, Tooltips, Bestaetigung nach Diktat, Leerzustand Settings, Android Bubble Control) sind investitionsarm aber sichtbar. Sie trennen "Beta-Qualitaet" von "Release-Qualitaet" in den Augen der Zielgruppe.

---

Erstellt vom Product Strategist. Naechster Schritt: Tech Lead priorisiert Blocker in aktuelle Sprint-Planung ein.
