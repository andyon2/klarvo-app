# Prompt — Klarvo UI Overhaul (zum Einfügen in Claude.ai)

> Kopiere den folgenden Block in eine neue Claude-Konversation (mit Artifacts).
> Hänge die Bilder aus `screenshots/` an und füge die Dateien `01`–`04` als Kontext bei
> (oder kopiere ihren Inhalt darunter).

---

Du bist Senior Product Designer mit Fokus auf moderne, ruhige Desktop-Tool-UIs (Linear, Raycast, Notion, Arc, Superhuman als Referenzklasse). Ich gebe dir eine bestehende App namens **Klarvo** und möchte einen **visuellen Overhaul**: optisch ansprechender, moderner, hochwertiger — aber **dieselbe Funktion, dieselbe Informationsarchitektur**. Es ist ein Re-Skin / Visual-Redesign, kein Feature- oder Flow-Redesign.

**Was Klarvo ist:** eine Diktier-/Spracheingabe-App für Power-User, Entwickler und Institute (Kanzlei, Praxis, Redaktion). Hotkey drücken → sprechen → der bereinigte Text landet im Zielfeld. Bring-your-own-API-Key, lokal-first, keine Telemetrie. Ernsthaftes Werkzeug, kein Mass-Market-Spielzeug. Details in `01-product-brief.md`.

**Plattformen:** Windows-Desktop (Tauri/WebView2 + React + Tailwind v4) **und** Android (native Kotlin). Beide teilen sich die Design-Sprache.

**Die zu überarbeitenden Flächen** (Screenshots beigefügt, Details in `02-surfaces.md`):
1. **FloatingBar** — das schwebende, immer-sichtbare Aufnahme-Overlay. Die Signature-Fläche. Winzig, transparent, draggable, always-on-top.
2. **Main-Window** — App-Shell mit History/Voice-Notes.
3. **Settings** — Home-Liste + Sub-Pages (Recording, AI/Providers, Appearance, Language, Shortcuts, License, Dictionary).
4. **Live-Cleanup-Preview** — Panel, das während des Sprechens den bereinigten Text live zeigt.
5. **Onboarding** — Erstkontakt-Flow.

**Harte Constraints (nicht verhandelbar — siehe `04-constraints.md`):**
- Die FloatingBar ist ein **transparentes, randloses Overlay-Fenster**. Hintergrund muss transparent bleiben; die sichtbare Pille ist ~200×36 logische px. Sie darf nie aufdringlich werden.
- **Dark-Theme ist die Produktidentität.** Wenn du ein Light-Theme vorschlägst, als optionale Ergänzung, nicht als Ersatz.
- Desktop rendert als React/Tailwind; **Android ist native Kotlin** → liefere die Design-Sprache als **Tokens + visuelle Specs**, nicht nur als React-Code, damit beide Plattformen sie umsetzen können.
- Bestehende Design-Tokens in `03-design-tokens.md` — du darfst die Palette weiterentwickeln, aber begründe Änderungen.

**Was ich von dir will (in dieser Reihenfolge):**
1. **Design-Kritik** des Ist-Zustands: 5–8 konkrete Schwächen, je 1 Satz, nach Hebelwirkung sortiert.
2. **Eine Design-Direction**: Moodboard-in-Worten + überarbeitete Token-Palette (Farben, Typo-Skala, Spacing, Radii, Elevation/Schatten, Motion-Prinzipien). Als Tabelle.
3. **Mockups als HTML/CSS-Artifact** (ein einzelnes, in sich geschlossenes Artifact, dark, pixelgenau) für mindestens: FloatingBar (idle + recording), Settings-Home + eine Sub-Page, Live-Preview. Nutze echte Klarvo-Inhalte aus den Screenshots, kein Lorem Ipsum.
4. **Umsetzungshinweise**: was davon 1:1 in Tailwind-Tokens übersetzbar ist vs. was Android-seitig nachgebaut werden muss.

Stelle mir **zuerst** alle Rückfragen, die die Richtung wesentlich ändern (max. 4), bevor du Mockups baust. Geh nicht von Mass-Market-Konventionen aus — das ist ein dichtes, ernsthaftes Power-Tool.
