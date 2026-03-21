# Briefing: Feature-Inventar + USP-Analyse + README-Vollendung

## Kontext

Die README auf voxlit-app wurde in der Session vom 2026-03-20 ueberarbeitet:
- Quick Start mit Cloud/Offline-Weiche ✅
- Provider-Tabellen (STT + LLM) mit Plattform-Support ✅
- Groq als einziger Pflicht-Key, DeepSeek empfohlen ✅
- OpenRouter verifiziert (API-Test erfolgreich) ✅

**Was FEHLT:** Die Feature-Sektion ("Was Voxlit kann") ist unvollstaendig. Viele Features die Voxlit von der Konkurrenz abheben sind nicht erwaehnt. Der Grund: Vorherige Sessions haben Features eingebaut ohne sie zu dokumentieren. Es gibt kein zentrales Dokument das alle Features auflistet.

## Das Problem

Andy hat erkannt dass die README nicht fertiggestellt werden kann, bevor wir wissen:
1. Welche Features Voxlit TATSAECHLICH hat (Code-Audit, nicht Erinnerung)
2. Welche davon echte USPs sind (Konkurrenz-Vergleich)
3. Welche davon in die README gehoeren (Marketing-Relevanz)

### Beispiele fuer undokumentierte Features (von Andy erwaehnt)

- **Return-to-Window:** Aufnahme in Fenster A starten, zu Fenster B wechseln, Pipeline laeuft, Ergebnis wird in Fenster A eingefuegt, Fokus kehrt zu Fenster B zurueck. Ideal fuer Multitasking.
- **Auto-Send:** Nach dem Einfuegen automatisch Enter druecken (z.B. Chat-Nachrichten abschicken).
- **Clipboard-Fallback mit UI-Hinweis:** Wenn Paste fehlschlaegt, wird Text in die Zwischenablage kopiert und die FloatingBar zeigt einen Hinweis.
- **Dual-Hotkey-System:** 2 unabhaengige Hotkey-Slots mit je eigenem Modus (z.B. Slot 1 = Hold fuer kurze Diktate, Slot 2 = Toggle fuer lange).
- **Android <1s Latenz:** Turso-Sync in Hintergrund ausgelagert, Pipeline blockiert nicht mehr.
- **Kosten-Dashboard mit Wispr-Flow-Vergleich:** Zeigt wieviel der User gegenueber Wispr Flow spart.

Es gibt wahrscheinlich noch mehr. Das muss systematisch aus dem Code extrahiert werden.

## Aufgabe (3 Schritte)

### Schritt 1: Feature-Inventar erstellen

Systematisch alle Features aus dem Code extrahieren. Quellen:
- `src-tauri/src/lib.rs` — Alle 58 Tauri-Commands (autoritativ)
- `src/components/SettingsPanel.tsx` — Alle Settings die der User sehen kann
- `src/App.tsx` — UI-Features
- `src/FloatingBar.tsx` — Bar-Features
- `android/kotlin-src/` — Android-spezifische Features
- `src-tauri/src/pipeline.rs` — Pipeline-Features (Return-to-Window, Auto-Send, Clipboard-Fallback)

Ergebnis: `knowledge/feature-inventory.md` mit ALLEN Features, gruppiert nach Kategorie.

### Schritt 2: USP-Analyse

Fuer jedes Feature im Inventar:
1. Hat Wispr Flow das? (Infos in `knowledge/competitors.md` und `knowledge/wispr-flow-android-ux.md`)
2. Haben andere Konkurrenten das? (WebSearch wenn noetig)
3. Ist es ein USP? (Ja/Nein/Teilweise)

Ergebnis: USP-Spalte im Feature-Inventar.

### Schritt 3: README Feature-Sektion neu schreiben

Basierend auf dem Inventar die "Was Voxlit kann" Sektion in der README neu schreiben:
- USP-Features prominent
- Nicht-USP-Features als "auch dabei"
- Platform-Sektionen (Windows/Android) mit den plattform-spezifischen Highlights

README liegt auf voxlit-app: `~/voxlit-app/README.md`
Aenderungen committen und pushen auf voxlit-app.

## Wichtig

- **Code ist Wahrheit.** Kein Feature listen das nicht im Code existiert.
- **Groq-Limit** muss ueberall erwaehnt werden wo Groq steht (Konsistenz zum Onboarding).
- **OpenRouter** ist verifiziert aber Modell ist hardcoded auf deepseek/deepseek-chat. Kein Versprechen von Modellauswahl.
- **Anthropic** ist NICHT im UI (nur Backend). Nicht in der README listen.
- Die README ist ein Enduser-Dokument, kein Dev-Dokument. Keine Build-Anleitung, kein Source-Code-Verweis.

## Referenzen

- Feature-Audit (vollstaendig): Wurde in der Session vom 2026-03-20 als Explore-Agent durchgefuehrt. Ergebnis im Chat-Kontext, nicht als Datei gespeichert. Muss in neuer Session wiederholt werden.
- `knowledge/competitors.md` — Bestehende Wettbewerbsanalyse
- `knowledge/wispr-flow-android-ux.md` — Wispr Flow Android-Recherche
- `knowledge/product-strategy.md` — Positionierung und Differenzierung
