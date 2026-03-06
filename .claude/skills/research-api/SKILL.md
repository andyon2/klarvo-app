---
name: research-api
description: Recherchiert API-Dokumentation oder Library-Docs und schreibt eine strukturierte Summary in knowledge/. Aufrufen mit Thema, z.B. "/research-api groq whisper" oder "/research-api tauri v2 mobile".
argument-hint: "[thema] -- z.B. 'groq whisper api', 'tauri v2 commands', 'deepseek chat api'"
allowed-tools: Read, Write, Bash, WebFetch, WebSearch
context: fork
model: sonnet
---

Recherchiere API- oder Library-Dokumentation und erstelle eine praxistaugliche Summary.

## Argumente

Aus `$ARGUMENTS` extrahiere das Thema der Recherche.

## Vorgehensweise

1. **Bestehende Knowledge pruefen:** Lies `knowledge/api-providers.md` und `knowledge/architecture.md` -- ist das Thema schon dokumentiert?

2. **Recherchieren:**
   - WebSearch nach offizieller Dokumentation
   - WebFetch der relevanten Docs-Seiten
   - Fokus auf: Endpoints, Authentication, Request/Response-Format, Rate Limits, Pricing, Code-Beispiele

3. **Summary schreiben** in die passende Knowledge-Datei:
   - API-Provider (Groq, DeepSeek, etc.) -> `knowledge/api-providers.md` (Sektion hinzufuegen/aktualisieren)
   - Libraries/Frameworks (Tauri, whisper-rs, cpal) -> `knowledge/architecture.md` (Sektion hinzufuegen/aktualisieren)
   - Plattform-spezifisch (Android IME, Windows APIs) -> `knowledge/platform-notes.md`

4. **Format der Summary:**

```markdown
## [Thema] -- Recherchiert [Datum]

### Ueberblick
[1-2 Saetze: Was ist das, wofuer nutzen wir es?]

### Setup / Authentication
[API-Key, Base-URL, Header]

### Relevante Endpoints / APIs
[Nur die, die wir brauchen -- mit Request/Response-Beispielen]

### Code-Beispiel (Rust)
[Minimales funktionierendes Beispiel]

### Limits & Kosten
[Rate Limits, Pricing, Free Tier]

### Gotchas
[Bekannte Fallstricke, Einschraenkungen]
```

5. **Melde zurueck:** "Recherche zu [Thema] abgeschlossen. Summary geschrieben in [Datei]. Wichtigste Erkenntnis: [1 Satz]."
