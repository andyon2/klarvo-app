# 02 — Die zu überarbeitenden Flächen

Jede Fläche mit: Zweck · aktueller Zustand · was schwach wirkt · Redesign-Ziel.
Die zugehörigen Screenshots liegen in `screenshots/` (gleiche Nummerierung).

---

## A. FloatingBar — das schwebende Aufnahme-Overlay  ⭐ höchster Hebel
- **Zweck:** Immer-sichtbarer Status-/Aufnahme-Indikator, der über allen Fenstern schwebt.
- **Zustände:** `idle` (**unsichtbar** — die Pille materialisiert sich nur bei Aktivität),
  `recording` (Live-Waveform, 5 Balken, audio-reaktiv), `transcribing` (Spinner),
  `done` (kurze Bestätigungs-Animation). Die Bar ist also **aktivitätsgetrieben**, kein
  Dauer-Chrome — der Redesign sollte die States als Sequenz/Transition denken.
- **Aktueller Zustand:** ~200×36 logische px, transparenter Fenster-Hintergrund, dunkle
  Pille, teal-Logo, einfache Balken-Waveform. Draggable, Position wird persistiert.
- **Was schwach wirkt:** wirkt eher funktional-utilitaristisch als hochwertig; Waveform und
  State-Übergänge sind simpel; wenig "Premium"-Gefühl (Glas/Tiefe/Politur fehlen).
- **Redesign-Ziel:** soll edel und ruhig wirken, klar lesbare States, schöne Micro-Motion,
  bleibt aber **winzig, unaufdringlich, transparent**. Kein Aufblasen der Fläche.

## B. Main-Window / App-Shell  (History & Voice-Notes)
- **Zweck:** Heimat-Fenster: durchsuchbare History vergangener Diktate, Voice-Notes,
  Statistiken (z.B. Filler-Words, Kosten-Dashboard), Einstieg in Settings.
- **Aktueller Zustand:** dunkles Panel (~494×758 bzw. 1152×661), Listen + Karten, Tailwind.
- **Was schwach wirkt:** Listen/Karten-Hierarchie und Leeres-State-Gestaltung; Typo-Skala
  und Abstände wirken eng; wenig visuelle Führung.
- **Redesign-Ziel:** klare Hierarchie, angenehme Listendichte, schöne Empty-States,
  bessere Such-/Filter-Affordances.

## C. Settings — Home-Liste + Sub-Pages
- **Zweck:** Konfiguration. Home ist eine Kategorie-Liste; jede Kategorie eine eigene Page.
- **Kategorien:** Recording & Audio · AI & Providers · Appearance · Language · Shortcuts ·
  License · Dictionary.
- **Aktueller Zustand:** Header "Settings" + Liste mit `SettingsRow` (Icon, Label, Chevron),
  Sub-Pages mit `SettingsRow`-Feldern, Toggles, Selects, Slidern.
- **Was schwach wirkt:** Form-Felder sehr "stock" (native Selects/Inputs), wenig visuelle
  Gruppierung, Sub-Page-Header dünn.
- **Redesign-Ziel:** ein konsistentes, hochwertiges Settings-Form-System (Rows, Sektionen,
  Toggles, Selects, Slider, Segmented-Controls) — Linear/Raycast-Niveau.

## D. Live-Cleanup-Preview
- **Zweck:** zeigt während des Sprechens den live bereinigten Text — als **Orientierung**,
  nicht als exakte Vorschau.
- **Aktueller Zustand:** separates Panel (~400×600), das auf-/zuklappt; Roh- vs. bereinigter
  Text, ggf. Kommentar-Spalte.
- **Was schwach wirkt:** Lesbarkeit/Rhythmus des Live-Texts; Übergang beim Auf-/Zuklappen.
- **Redesign-Ziel:** ruhiger, gut lesbarer Live-Text, klarer Roh-vs-Clean-Kontrast,
  smoothe Expand/Collapse-Motion.

## E. Onboarding
- **Zweck:** Erstkontakt — API-Key/Provider einrichten, Hotkey verstehen, erste Aufnahme.
- **Aktueller Zustand:** mehrstufiger Flow (große Komponente, ~1.6k Zeilen).
- **Was schwach wirkt:** First-Impression-Politur; Step-Indikatoren; Illustration/Leere.
- **Redesign-Ziel:** vertrauensbildender, eleganter erster Eindruck, der die
  Ernsthaftigkeit des Tools sofort vermittelt (BYOK als Feature framen, nicht als Hürde).

---

### Plattform-Hinweis
Desktop-Screenshots sind die Referenz. **Android** (native Kotlin) teilt sich die
Design-Sprache, hat aber eigene Constraints (System-Nav-Bar-Clearance, Touch-Targets ≥ 48px).
Android hat **zwei** Recording-Oberflächen: (1) **In-App** (`android-05-recording.png`) beim
Aufnehmen in der App selbst, und (2) die **Dictation-Bubble** (`android-06-bubble*.png`) —
das eigentliche Pendant zur Desktop-Pill: ein schwebendes teal „K"-Overlay, das **nur bei
fokussiertem Textfeld + offener Tastatur** erscheint und das Diktat direkt in dieses Feld
auslöst. Beim Bubble-Redesign also den IME-/Textfeld-Kontext mitdenken. Android-Screenshots
sind mit `android-` präfixiert.
