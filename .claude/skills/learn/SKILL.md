---
name: learn
description: "Integriert neue Wissensquellen in knowledge/. Aufrufen mit Dateiname aus sources/inbox/ oder 'alle' fuer alle Dateien in der Inbox."
argument-hint: "[dateiname | alle]"
allowed-tools: "Read, Write, Edit, Glob, Grep, WebFetch, WebSearch, Bash"
context: fork
model: sonnet
---

Du integrierst neue Wissensquellen in Diktas Knowledge-Base.

## Argumente

Aus `$ARGUMENTS`:
- `alle` → Alle Dateien in `sources/inbox/` verarbeiten
- `[dateiname]` → Einzelne Datei aus `sources/inbox/` verarbeiten

Falls kein Argument: Fehler melden.

## Vorgehen

### 1. Quelle lesen und verstehen

Lies die Datei aus `sources/inbox/`. Wenn die Datei nur eine URL enthaelt:
- YouTube-URL → Versuche WebFetch auf das Transcript
- Artikel-URL → WebFetch fuer den Inhalt

### 2. Relevanz pruefen

Dikta-relevantes Wissen:
- Voice Dictation, STT, TTS, Audio-Processing
- Tauri, Rust (Desktop/Mobile), React
- API-Provider (Groq, DeepSeek, OpenAI, Whisper)
- Android Overlay/Accessibility/Kotlin
- Wettbewerber (Wispr Flow, Voice Type, OpenWhispr)
- Produkt-Strategie fuer Desktop-Tools / Indie-Software
- UX-Patterns fuer Dictation/Voice-Tools

Nicht relevant → Melde es und ueberspringe.

### 3. In Knowledge verdichten

Finde die passende Knowledge-Datei:
- `knowledge/architecture.md` → Tech-Entscheidungen, Patterns
- `knowledge/api-providers.md` → API-Details, Pricing, Limits
- `knowledge/competitors.md` → Wettbewerber-Infos
- `knowledge/product-strategy.md` → Positionierung, Pricing, Markt
- `knowledge/wispr-flow-android-ux.md` → Wispr-Flow-spezifisch
- Neue Datei nur wenn kein bestehendes Ziel passt

Verdichtungs-Logik:
- **Bestaetigung:** Quelle bestaetigt bestehendes Wissen → Quellen-Referenz ergaenzen, Inhalt nicht duplizieren
- **Erweiterung:** Neues Detail zu bestehendem Thema → An passender Stelle einfuegen
- **Widerspruch:** Quelle widerspricht bestehendem Wissen → Beide Sichten dokumentieren, als offen markieren
- **Veraltung:** Neue Quelle ersetzt veraltetes Wissen → Altes ersetzen, Quelle angeben

### 4. Archivieren

1. Verschiebe die Quelldatei nach `sources/archive/` mit Namensformat `YYYY-MM-DD_[slug].md`
2. Ergaenze `sources/log.md` um eine Zeile

### 5. Ergebnis melden

```
QUELLE INTEGRIERT: [Dateiname]
Typ: [Artikel/Video/Recherche/...]
Ziel: [knowledge/xyz.md]
Aktion: [Bestaetigung/Erweiterung/Widerspruch/Neue Datei]
Zusammenfassung: [1-2 Saetze was integriert wurde]
```
