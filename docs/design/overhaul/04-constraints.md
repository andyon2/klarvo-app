# 04 — Harte Constraints für den Redesign

Der Designer muss diese respektieren, sonst ist das Ergebnis nicht umsetzbar.

## Rendering / Tech
- **Desktop:** Tauri v2 + **WebView2** (Chromium-Engine) + **React** + **Tailwind v4**
  (`@theme`-Tokens). Alles, was in modernem CSS geht, geht — moderne Layouts, `backdrop-filter`,
  Transitions, SVG. Kein Canvas/WebGL nötig.
- **Android:** **native Kotlin** (kein WebView, kein React). → Die Design-Sprache muss als
  **Tokens + visuelle Specs** geliefert werden, damit sie nativ nachgebaut werden kann.
  Liefere keine reine React-Lösung, die Android nicht umsetzen kann.

## FloatingBar (das Overlay) — strengste Constraints
- Ist ein **eigenes, transparentes, randloses, always-on-top Tauri-Fenster**. Der
  Fenster-Hintergrund **muss transparent bleiben** (`html/body/#root { background: transparent }`),
  sonst zeigt WebView2 seinen Default-Hintergrund durch → die sichtbare "Pille" ist das
  einzige gerenderte Element.
- Sichtbare Pille aktuell **~200×36 logische px**. Größe wird in Rust beim Fenster-Erzeugen
  gesetzt; das Frontend ruft **nicht** `setSize`. Wenn der Redesign andere Maße braucht,
  als **expliziten Wert** angeben (klein halten — es ist ein Overlay, kein Panel).
- Muss **draggable** sein (Position wird persistiert) und darf den darunterliegenden
  Arbeitsfluss nie verdecken/stören.
- **Android-Pendant zur Pill = die Dictation-Bubble** (`android-06-bubble*.png`): ein
  schwebendes teal „K"-Overlay (`KlarvoOverlayService` + `SYSTEM_ALERT_WINDOW`), das **nur
  bei fokussiertem Textfeld + offener Tastatur** erscheint und das Diktat direkt ins Feld
  triggert (draggable, randverankert). Zusätzlich gibt es einen **In-App**-Recording-State
  (`android-05`). Der Pill-/Bubble-Redesign betrifft also beide Plattformen — auf Android an
  den IME-/Textfeld-Kontext gebunden, kein freistehendes Always-on-Top-Fenster wie am Desktop.

## Theme
- **Dark ist die Identität.** Light-Theme nur als optionale Ergänzung vorschlagen, nie als
  Ersatz. (Es gibt einen `ThemeSwitcher`, aber Dark ist der Default/Charakter.)
- Token-getrieben: neue Farben als benannte Tokens vorschlagen, nicht als verstreute Hex.

## Android-spezifisch
- **Touch-Targets ≥ 48px.** System-Nav-Bar-Clearance: `env(safe-area-inset-bottom)` ist im
  Tauri-Android-WebView unzuverlässig (gibt 0 zurück) — Klarvo nutzt feste Pixel-Clearances
  (56px Footer). Native Kotlin-UI hat eigene Insets-Behandlung.
- Dictation-Bubble ist IME-/textfeld-gebunden, kein freistehendes Always-on-Top-Fenster (s.o.).

## Inhalt
- **Echte Inhalte verwenden**, kein Lorem Ipsum: echte Settings-Labels (siehe `02-surfaces.md`),
  echte History-Einträge (Diktat-Text), echte Provider-Namen (Groq, etc.).
- **Keine Telemetrie/Tracking-UI** erfinden — widerspricht dem BYOK/Privacy-Narrativ.

## Was NICHT Teil des Auftrags ist
- Keine neuen Features, keine geänderte Informationsarchitektur, keine neuen Flows.
- Reines **Visual-Redesign**: Farbe, Typo, Spacing, Komponenten-Politur, Motion, Hierarchie.
