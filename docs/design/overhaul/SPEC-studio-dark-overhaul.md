# Handoff: Klarvo Visual Overhaul — „Studio Dark" + Android Dictation-Bubble

> **Provenienz / Tracking-Hinweis.** Diese Datei ist die getrackte Kopie des Cloud-Web-Design-Agent-Outputs
> (high-fidelity Design-Spec, finale Tokens) und dient als UX-Design-Input für die Overhaul-Epics. Die
> begleitenden **HTML/CSS-Referenzprototypen** (`Klarvo Visual Redesign.html`, `Klarvo Android Bubble.html`,
> `assets/klarvo.css`) und die **Ist-Zustand-Screenshots** liegen bewusst nur lokal im gitignorierten Paket
> `design-handoff/` (zu schwer / Referenz, nicht zum Kopieren). Werte hier (Hex, Spacing, Radii, Motion) sind
> verbindlich. Begleit-Kontext: `00-original-prompt.md` … `04-constraints.md` im selben Verzeichnis.

## Overview
Visueller Overhaul von **Klarvo** (Diktier-/Spracheingabe-Tool, BYOK, lokal-first, Dark-Identität).
Reines **Re-Skin / Visual-Redesign + Interaktions-Politur der Android-Bubble** — gleiche Funktion,
gleiche Informationsarchitektur, keine neuen Features/Flows. Zwei Plattformen teilen eine
Design-Sprache: **Windows-Desktop** (Tauri/WebView2 + React + Tailwind v4) und **Android**
(native Kotlin/Compose).

## About the Design Files
Die HTML-/CSS-Dateien in diesem Bundle sind **Design-Referenzen** (Prototypen, die Look &
Verhalten zeigen) — **kein** produktiv zu kopierender Code. Aufgabe: diese Designs in der
**bestehenden Umgebung** nachbauen — Desktop in React + Tailwind v4 (`@theme`-Tokens), Android
nativ in Kotlin/Jetpack Compose. Werte (Hex, Spacing, Radii, Motion) sind verbindlich; die
Umsetzung folgt den vorhandenen Patterns des jeweiligen Codebases.

## Fidelity
**High-fidelity.** Finale Farben, Typografie, Spacing, Radii, Elevation und Motion. Pixelgenau
nachbauen. Die zwei bewussten Token-Abweichungen vom Ist-Zustand sind unten begründet.

---

## Design Tokens

### Tailwind v4 — `@theme` (Desktop, direkt übernehmbar)
```css
@theme {
  /* neutrals — cool graphite ladder */
  --color-klarvo-bg-deep:   #0A0B0C;  /* letterbox / behind windows */
  --color-klarvo-bg:        #0F1112;  /* canvas / window base   (war #191919) */
  --color-klarvo-surface:   #16181A;  /* cards, list containers (war #252525) */
  --color-klarvo-surface-2: #1B1E20;  /* inputs, raised */
  --color-klarvo-elevated:  #232729;  /* popover, hover-raise   (war #2F3438) */
  --color-klarvo-border:    #282C2F;  /* default hairline       (war #373C3F) */
  --color-klarvo-border-2:  #353A3E;  /* strong / active        (war #3F4448) */
  /* text */
  --color-klarvo-text:      #ECEEEF;  /* primary  (war #FFFFFFEB) */
  --color-klarvo-muted:     #A4A9AC;
  --color-klarvo-dim:       #6F7479;
  --color-klarvo-faint:     #4B4F53;
  /* teal — brand / primary / status */
  --color-klarvo-teal:      #29C7AC;  /* primary  (war #2AC3A8 / logo #14B8A6) */
  --color-klarvo-teal-hi:   #57DDC7;  /* hover / accent / highlight */
  --color-klarvo-teal-lo:   #1B9C88;  /* pressed */
  --color-klarvo-on-teal:   #05201B;  /* text on teal fill */
  /* amber — live / activity / recording / warning (war #FFA344) */
  --color-klarvo-amber:     #E9A24C;
  --color-klarvo-amber-hi:  #F4BA72;
  /* semantic */
  --color-klarvo-danger:    #EE6F63;  /* stop / delete / error (war #FF7369) */
  --color-klarvo-success:   #4FC58A;
}
```
Subtle/line variants als `color-mix` oder rgba: teal-bg `rgba(41,199,172,.12)`, teal-line
`rgba(41,199,172,.32)`; amber-bg `rgba(233,162,76,.12)`, amber-line `rgba(233,162,76,.32)`;
danger-bg `rgba(238,111,99,.12)`; glas-hairline `rgba(255,255,255,.055)`.

### Android — Kotlin/Compose
```kotlin
val Bg        = Color(0xFF0F1112)
val Surface   = Color(0xFF16181A)
val Surface2  = Color(0xFF1B1E20)
val Elevated  = Color(0xFF232729)
val Border    = Color(0xFF282C2F)
val Border2   = Color(0xFF353A3E)
val TextC     = Color(0xFFECEEEF)
val Muted     = Color(0xFFA4A9AC)
val Dim        = Color(0xFF6F7479)
val Teal      = Color(0xFF29C7AC)
val TealHi    = Color(0xFF57DDC7)
val TealLo    = Color(0xFF1B9C88)
val OnTeal    = Color(0xFF05201B)
val Amber     = Color(0xFFE9A24C)
val Danger    = Color(0xFFEE6F63)
```

### Color semantics (verbindliche Regeln)
- **Teal** = Marke, „bereit", Verarbeitung/Processing, Erfolg, Fokus-Ring.
- **Amber** = „live / hört zu" (nur während Aufnahme — Tally-Light).
- **Rot/Danger** = nur Stop/Löschen/Fehler.
- Erfolg (grün) sparsam.

### Type
- UI: **Geist** (400/500/600/700). Diktat-Text, Keys, IDs, Timestamps: **Geist Mono**.
- Skala (px): 11 label (uppercase, +8% tracking) · 12 caption · 13 body-sm · 14 body · 16 subhead · 20 heading · 28 title · 40 display. LH 1.1–1.55.

### Spacing (4-Basis), Radii, Elevation, Motion
- Spacing: 2 4 6 8 12 16 20 24 32 40 48.
- Radii: xs 6 (inputs/chips) · sm 8 (buttons/selects) · md 12 (cards/icon-badges) · lg 16 (panels) · xl 20 (windows) · full (pille/dots/toggle).
- Elevation: e1 `0 1px 2px rgba(0,0,0,.45)` · e2 `0 4px 14px …` · e3 `0 12px 32px …` · pill `0 8px 28px …`; jeweils + inset hairline `inset 0 1px 0 rgba(255,255,255,.055)`. Fokus-Ring `0 0 0 3px rgba(41,199,172,.28)`.
- Motion: micro 120ms · state 180ms · enter 240ms (Spring `cubic-bezier(.34,1.56,.64,1)`) · panel 320ms. Standard-Ease `cubic-bezier(.2,0,0,1)`. `prefers-reduced-motion` respektieren.

### Zwei bewusste Abweichungen vom Ist-Zustand (begründet)
1. Neutral-Ladder tiefer & kühler (Graphit statt Warmgrau #252525) → liest „Instrument" und gibt Elevation Raum.
2. Orange `#FFA344` → ruhigeres Amber `#E9A24C` mit klarer Semantik „live/hört zu". Teal bleibt Marke, nur minimal geklärt. **Verstreute Inline-Hex zu benannten Tokens konsolidieren** (war ein Hauptgrund fürs „nicht aus einem Guss"-Gefühl).

---

## Surfaces (Desktop) — in `Klarvo Visual Redesign.html`
- **FloatingBar** (Signature, transparentes 200×36 Overlay-Fenster): States als Sequenz idle (unsichtbar) → recording (Glas-Pille, Amber-Tally + Teal-Waveform, Spring-Enter) → transcribing (Teal-Spinner) → done (Check). Backdrop-blur 16px, 72% Graphit-Fill, inset hairline. Fläche bleibt 200×36 (kein Aufblasen), draggable, Position persistiert.
- **Settings-Home + Sub-Pages** (AI & Providers gezeigt): farbcodierte Icon-Badges, Status-Dots, maskierte mono Keys, Custom-Select/Segmented/Toggle/Slider (native `<select>` raus).
- **Live-Cleanup-Preview** (desktop-only): transparentes Panel, Live-**Roh**-Transkript in Mono, am Boden verankert. (Live-LLM-*Cleanup* ist bewusst aus — Quota; nur Roh-Stream live.)
- **Main-Window / History**: bessere Listendichte, mono Timestamps, Profil-Tags in Amber.

## Surface (Android) — in `Klarvo Android Bubble.html`
Native Kotlin/Compose. IME-gebundene **Dictation-Bubble** (`SYSTEM_ALERT_WINDOW`-Overlay, erscheint bei fokussiertem Feld + Tastatur).

### Bubble — States (gleiche Form, nur Farbe/Inhalt wechselt — KEIN Kreis↔Quadrat-Morph)
- **idle**: Teal-K, dezenter Glas-Ring. **Größe responsiv:** `visual = clamp(36dp, 0.11 × min(screenW,screenH)dp, 44dp)`, Touch-Ziel `max(visual, 48dp)` via transparentes Padding. (~40dp auf 360–420dp-Phones.)
- **recording**: Tastatur **klappt ein**, ein **Klarvo-eigenes Panel** rückt hoch (Griff-Handle, K + Amber-Live-Dot + reaktive Waveform aus RMS-Levels + Timer + roter Stop). Live-**Roh**-Transkript läuft mehrzeilig im Panel. Footer: „Tastatur pausiert · kehrt beim Einfügen zurück".
- **transcribing**: gleiches Panel, Teal-Spinner + „Bereinigt…", Roh-Text gedimmt.
- **done**: Panel klappt zu, Tastatur kommt zurück, **bereinigter** Text steht im Feld, Bubble kurz Check → idle.

### Interaktion
- **Short-Press = Standard-Geste**, konfigurierbar: **Hold / Toggle / Auto-Stop / Auto** (dieselben 4 Modi wie der Desktop-Hotkey-Mode — spiegeln!).
- **Long-Press = Quick-Popover** (öffnet nach innen, nie radial): Block „Standard-Geste" (4 Modi) · „Modus" (Polished/Verbatim/Chat) · Zeile Ziel (Ins Feld / Zwischenablage) + Sprache (DE/EN/Auto) · Footer „Einstellungen öffnen". Zwei getrennte Achsen: **Geste** (wie ausgelöst) vs. **Modus** (wie bereinigt).
- Verankerung bleibt: draggable, springt mit Tastatur hoch, Rand-Snap + gemerkte Seite.

---

## ⚠️ Machbarkeits-Constraints (kritisch für die Implementierung)
1. **Kein In-Field-Vorschautext aus dem Overlay.** Provisorischer „Composing"-Text (grau, verfestigt sich) ist **IME-only** (`InputConnection.setComposingText`) — nur die aktive Tastatur darf das. Ein `SYSTEM_ALERT_WINDOW`-Overlay kann fremde Felder nur **final** beschreiben (Accessibility `ACTION_SET_TEXT` oder Clipboard+Paste). → Live-Roh-Text läuft auf **Klarvos eigener Fläche** (Listening-Panel), nicht im fremden Feld. Echtes In-Field-Grau gäbe es nur, wenn Klarvo selbst ein IME wäre (anderes Architekturmodell, hier nicht gewollt).
2. **Tastatur einklappen während Aufnahme** = optionaler Schalter. Umsetzbar via IME-Dismiss über den Accessibility-Service (den Klarvo fürs finale Einfügen ohnehin nutzt); Ziel-Feld bleibt fokussiert, finaler Insert klappt. Per-App-Robustheit minimal unterschiedlich → als Option, mit Fallback (Tastatur einfach offen lassen), nicht als Default für alle.
3. **Kein `backdrop-blur` nativ** — Glas-Effekte auf Android als solide `#16181A`-Flächen + `Modifier.shadow`; der „Glas-Ring" der Bubble = 4dp Teal/Amber-Ring statt echtem Blur.
4. Touch-Targets ≥ 48dp; feste 56px Nav-Bar-Clearance unten (`env(safe-area-inset-bottom)` ist im Tauri-Android-WebView unzuverlässig/0).
5. Android-IA-Unterschiede: **kein** Appearance/Live-Preview (desktop-only), schlankeres About, kein Microphone-Picker.

## Inhalt
Echte Labels/Provider verwenden (Groq, DeepSeek, OpenAI, Anthropic, OpenRouter; verbatim/polished/chat). **Keine Telemetrie-/Tracking-UI** — widerspricht BYOK/Privacy.

## Assets
Keine externen Bilder. Logo = CSS-gerendertes „K" (Rounded-Square, Teal-Gradient). Icons = inline SVG. Schrift = Geist + Geist Mono (Google Fonts; auf Android als gebündelte Font-Resources).

## Files
- `Klarvo Visual Redesign.html` — Desktop-Direction: Kritik, Tokens, FloatingBar, Settings, Live-Preview, History, Umsetzung.
- `Klarvo Android Bubble.html` — Android-Bubble: State-Sequenz, Listening-Panel, Long-Press-Menü, Tastatur-Einklappen, native Notes.
- `assets/klarvo.css` — die komplette Token-/Komponenten-Sprache (Quelle für alle Werte oben).
