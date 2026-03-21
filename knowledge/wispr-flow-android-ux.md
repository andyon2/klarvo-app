# Wispr Flow Android UX Research

Stand: 2026-03-08
Quellen: wisprflow.ai, docs.wisprflow.ai, Android Police, 9to5Google, The Intelligence, TechCrunch, Technobezz

---

## 1. Floating Bubble -- Verhalten

### Wann erscheint die Bubble?
- Erscheint **automatisch**, sobald der Nutzer ein Textfeld antippt und die Soft-Keyboard sichtbar wird.
- Verschwindet automatisch, wenn kein Textfeld mehr fokussiert ist / Keyboard schliesst.
- Gelegentlich bleibt die Bubble kurz haengen, nachdem das Keyboard geschlossen wurde (bekanntes UX-Problem laut Reviews).
- Die Bubble wird **nicht** in Banking-Apps oder Passwort-Managern angezeigt (130+ blockierte Apps in 9 Regionen).

### Positionierung
- Erscheint **ueber dem Keyboard** als Overlay (nicht als Teil des Keyboards).
- Laesst sich per **Press-and-Hold wegziehen/dismissen** (drag to dismiss).
- Keine explizite Dokumentation ueber freie Drag-Positionierung am Bildschirmrand (anders als z.B. Facebook Messenger Bubbles).

### Sichtbarkeits-Kontrolle
- Toggle in den App-Settings zum Ein-/Ausblenden der Bubble.
- Bubble-Groesse einstellbar: **4-Stufen-Slider** (0.7x, 0.85x, 1.0x, 1.15x) mit Reset-Option.

---

## 2. Interaktionsfluss

### Zwei Diktiermodi

**Modus 1: Tap-to-Toggle**
1. Textfeld antippen -> Keyboard oeffnet -> Bubble erscheint
2. Bubble antippen -> Aufnahme startet (Bubble expandiert, Waveform-Animation)
3. Sprechen
4. **Checkmark-Button** antippen -> Text wird in Textfeld eingefuegt
5. Alternativ: **Close-Button** zum Abbrechen

**Modus 2: Press-and-Hold**
1. Textfeld antippen -> Keyboard oeffnet -> Bubble erscheint
2. Bubble **gedrueckt halten** -> Aufnahme laeuft solange Finger auf Bubble
3. **Loslassen** -> Text wird automatisch eingefuegt (kein Checkmark noetig)

### Zusammenfassung
- Tap = manueller Start/Stop mit Bestaetigung
- Hold = automatisches Start/Stop bei Loslassen
- Beide Modi fuegen Text direkt ein, kein Clipboard-Umweg im Normalfall

---

## 3. Visuelle Zustaende der Bubble

### Idle-Zustand
- Kleine, kompakte Bubble mit Mikrofon-Icon
- Farbe: **Lila/Purple** (Wispr-Branding)
- Schwebt ueber dem Keyboard

### Recording-Zustand
- **Waveform-Animation**: Weisse Balken bewegen sich synchron zur Sprachlautstaerke
- Wenn Balken flach bleiben -> Mikrofon empfaengt kein Audio
- Bubble expandiert (groessere Ansicht mit Controls)
- Checkmark-Button und Close-Button werden sichtbar

### Fehler-Zustand
- **Roter Rand** um die Bubble: Mikrofon ist stumm oder getrennt
- Sobald Audio wieder empfangen wird: Roter Rand verschwindet, Waveform-Animation setzt fort

### Processing-Zustand
- Nicht explizit dokumentiert, aber die KI-Verarbeitung (Filler-Entfernung, Formatierung) passiert in Echtzeit waehrend des Sprechens, nicht als separater Schritt danach.

---

## 4. Text-Einfuegung -- Technische Umsetzung

### Primaerer Mechanismus: Accessibility Service
- Wispr Flow nutzt den **Android Accessibility Service** als Kerntechnologie.
- Der Service erkennt Textfelder durch Analyse von `AccessibilityEvent` (Window-Changes, Focus-Changes).
- Text wird ueber die **Accessibility-Input-Connection** direkt in das fokussierte Textfeld eingefuegt.
- **Kein IME (InputMethodService)** -- Flow ersetzt nicht das Keyboard.
- **Kein primaerer Clipboard-Mechanismus** -- Text wird direkt gepastet.

### Fallback-Mechanismus: Clipboard
- Wenn die direkte Einfuegung fehlschlaegt (manche Apps, manche Android-Skins), kopiert Flow den Text **automatisch in die Zwischenablage**.
- Der Nutzer muss dann manuell einfuegen (bekanntes Problem z.B. auf OnePlus-Geraeten).

### Was der Accessibility Service im Detail macht
1. **Textfeld-Erkennung**: Analysiert sichtbaren Bildschirminhalt und Text-Feld-Layouts
2. **Keyboard-Erkennung**: Erkennt ob Soft-Keyboard offen/geschlossen ist -> steuert Bubble-Sichtbarkeit
3. **Sensible Felder vermeiden**: Erkennt Passwort-, PIN- und Kreditkarten-Felder -> Bubble erscheint dort nicht
4. **Kontext-Analyse**: Liest App-Namen und Website-URLs (fuer Analytics/Diagnostik)
5. **Text-Kontext**: Liest begrenzten Text aus aktiver App fuer Transkriptions-Kontext

### Ohne Accessibility-Permission
- Bubble erscheint **nicht**
- Kern-Funktionalitaet ist **komplett deaktiviert**
- Flow funktioniert auf Android ohne Accessibility Service nicht

---

## 5. Permissions-Modell

### Erforderliche Permissions (alle 4 noetig)

| Permission | Zweck | Android-API |
|-----------|-------|-------------|
| **Mikrofon** | Sprachaufnahme | `RECORD_AUDIO` |
| **Accessibility Service** | Textfeld-Erkennung + Text-Einfuegung | `AccessibilityService` |
| **Display over other apps** | Bubble-Overlay anzeigen | `SYSTEM_ALERT_WINDOW` |
| **Batterie-Optimierung deaktivieren** | Verhindert dass Android den Service im Hintergrund beendet | `REQUEST_IGNORE_BATTERY_OPTIMIZATIONS` |

### Hersteller-spezifische Extras
- **OnePlus, Xiaomi, Oppo/Realme, Vivo**: Zusaetzliche **Auto-Start/Auto-Launch**-Permission noetig
- Samsung, Google Pixel, Motorola, Lenovo: Standard-Permissions reichen

### Weitere Manifest-Permissions (aus APK-Analyse)
- `ACCESS_NETWORK_STATE`, `INTERNET` (Cloud-STT)
- `POST_NOTIFICATIONS` (Benachrichtigungen)
- `RECEIVE_BOOT_COMPLETED` (Auto-Start nach Reboot)
- `FOREGROUND_SERVICE` (Hintergrund-Dienst)

---

## 6. Architektur-Entscheidung: Overlay vs. IME

### Warum Wispr Flow Overlay (Accessibility) statt IME gewaehlt hat

**Vorteile des Overlay-Ansatzes:**
- Nutzer muss **Keyboard nicht wechseln** (Gboard/SwiftKey bleibt)
- Weniger Friction: kein Switchen zwischen Diktat-Keyboard und Normal-Keyboard
- Funktioniert **neben** jedem Keyboard, nicht statt
- Auf Android gibt es tieferen System-Zugang als auf iOS

**Nachteile:**
- Abhaengig von Accessibility Service (kann von Android deaktiviert werden, Nutzer bekommt Notification zum Re-Enablen)
- Manche Android-Skins (OnePlus) haben Probleme mit der Text-Einfuegung
- 130+ Apps muessen manuell blockiert werden (Banking etc.)
- Google Play Store Review koennte strenger werden (Accessibility-Missbrauch)

### Vergleich: Typeless (Konkurrent) nutzt IME-Ansatz
- Typeless ersetzt das Keyboard komplett -> grosses Mikrofon-Icon statt Tasten
- Vorteil: Zuverlaessigere Text-Einfuegung (ueber InputConnection)
- Nachteil: Nutzer muss aktiv Keyboard wechseln, Friction beim Zurueckwechseln

### Wispr Flow auf iOS: Keyboard-Ansatz (IME)
- Auf iOS nutzt Flow ein **Custom Keyboard** (Keyboard Extension)
- Grund: iOS erlaubt keine System-Overlays wie Android
- Nutzer muss in iOS-Settings das Flow-Keyboard aktivieren und dorthin wechseln

---

## 7. System-Requirements

- **Android Version:** 13+ (API Level 33)
- **RAM:** mindestens 6 GB
- **Speicher:** mindestens 500 MB frei
- **Geraete:** Smartphones, Tablets, Foldables
- **APK-Groesse:** ~150 MB
- **Internet:** Erforderlich (Cloud-STT, kein Offline-Modus)

---

## 8. KI-Features (Text-Cleanup)

- Filler-Woerter entfernen ("um", "uh", "aeh")
- Automatische Interpunktion (basierend auf Pausen und Tonfall)
- Selbst-Korrekturen: "Treffen wir uns um 4... nein, um 3" -> nur "um 3" wird geschrieben
- 100+ Sprachen (ohne manuelle Umschaltung)
- Kontext-adaptiver Ton: casual in WhatsApp, formal in Gmail
- Custom Dictionary fuer Fachbegriffe
- Snippets: Voice-Shortcuts fuer haeufige Phrasen
- Command Mode: Selektierten Text per Sprache umformatieren

---

## 9. Relevanz fuer Voxlit Android

### Was wir uebernehmen sollten
1. **Floating Bubble** als primaeres UI-Element (nicht IME)
2. **Zwei Diktiermodi**: Tap-to-Toggle + Press-and-Hold
3. **Waveform-Animation** waehrend Aufnahme
4. **Automatisches Ein-/Ausblenden** bei Textfeld-Fokus
5. **Bubble-Groesse** konfigurierbar
6. **Fehler-Zustand** visuell anzeigen (roter Rand)

### Was wir anders machen muessen
1. **Kein Accessibility Service** fuer Text-Einfuegung -- wir nutzen einen **IME (InputMethodService)** fuer zuverlaessigere Einfuegung
2. Unser IME hat direkten Zugang zur InputConnection -> kein Clipboard-Fallback noetig
3. **Offline-Modus** mit whisper.cpp (Wispr Flow ist rein Cloud)
4. **Kein Abo** -- komplett kostenlos und Open Source
5. Kleinere APK-Groesse anstreben (Wispr: 150 MB, wir: aktuell 16 MB)

### Architektur-Frage: Hybrid-Ansatz?
Wispr Flow zeigt: Der Overlay-Ansatz (Accessibility Service) hat UX-Vorteile (kein Keyboard-Wechsel), aber technische Risiken (Accessibility-Missbrauch, unzuverlaessige Text-Einfuegung).

Unser Plan laut `memory/android-ime-plan.md`: IME-basiert. Die Bubble koennte trotzdem als Overlay realisiert werden (SYSTEM_ALERT_WINDOW), waehrend die Text-Einfuegung ueber den IME laeuft. Das waere ein **Hybrid-Ansatz**:
- Overlay-Bubble fuer die Aktivierung (wie Wispr Flow)
- IME fuer die Text-Einfuegung (zuverlaessiger als Accessibility)
- Nachteil: Nutzer muss Voxlit als Keyboard aktivieren (einmaliger Setup-Schritt)

---

## Quellen

- https://wisprflow.ai/android
- https://docs.wisprflow.ai/articles/8858845757-setup-wispr-flow-on-android-android-settings
- https://docs.wisprflow.ai/articles/7669452251-accessibility-permission-on-android
- https://docs.wisprflow.ai/articles/1036674442-supported-devices-and-system-requirements
- https://www.androidpolice.com/wispr-flow-app-android-voice-typing-experience/
- https://9to5google.com/2026/02/23/flow-dramatically-improves-android-voice-typing-without-replacing-gboard/
- https://theintelligence.com/42374/android-voice-typing/
- https://techcrunch.com/2026/02/23/wispr-flow-launches-an-android-app-for-ai-powered-dictation/
- https://www.technobezz.com/news/wispr-flow-launches-its-ai-dictation-app-on-android-with-a-floating-bubble-interface
- https://hothardware.com/news/wispr-flow-ai-dictation-app-android
- https://play.google.com/store/apps/details?id=com.wispr.flowapp
- https://www.makeuseof.com/typeless-ai-voice-typing-android/
