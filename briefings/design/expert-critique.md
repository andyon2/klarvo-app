# UI Expert Critique: Voxlit Theme Variants

Erstellt: 2026-03-21
Autor: UI/Visual Design Review (Senior Designer Perspective)

---

## 1. Diagnose: Was ist das eigentliche Problem

Das Problem ist nicht "falsche Farben". Das Problem ist fundamentaler: Die Varianten behandeln Farbe als Taxonomie statt als Licht.

Taxonomie bedeutet: "Hintergrund = #1B1B1B, Surface = #222226, Border = #333338." Das ist ein Katalog von Werten. Licht bedeutet: Die Oberfläche verhält sich so, als ob eine diffuse Lichtquelle sie anstrahlt — manche Bereiche reflektieren mehr, manche weniger, und diese Differenz erzeugt räumliche Tiefe ohne einen einzigen Gradient explizit zu definieren.

Alle fünf Varianten machen denselben Fehler: Sie sind **pitch-flat**. Der Hintergrund ist ein uniformer Farbblock. Die Surfaces sind uniformer Farbblock plus dünne Border. Das Ergebnis ist eine Oberfläche, die sich anfühlt wie gestrichene Sperrholzplatten — alle gleich matt, gleich flach, ohne Materialität.

Spezifisch was fehlt und warum es "monofarbig" wirkt:

**Fehlender Hintergrundgradient.** `bg-[#0a0a0c]` auf `main` ist ein harter, uniformer Block. Kein Radial, kein subtiles Vignette. Das Auge hat nirgendwo wo es "landet" — keine Mitte, keine Kanten, kein Brennpunkt. Reale dunkle Oberflächen (Aluminium, Glass, poliertes Metall) haben immer einen Lichtkegel irgendwo.

**Keine Surface-Elevation jenseits von Border.** Die Cards und Panels in Settings/History sind `bg-[#111113]` mit `border border-zinc-800/60`. Das ist 2015-Flat-Design. Elevation entsteht nicht durch hellere Farbe allein — sondern durch `box-shadow` die Tiefe simuliert. Linear, Notion, Superhuman nutzen alle subtile `box-shadow` auf Surfaces. Voxlit hat bei `shadow-xl shadow-black/30` auf Panels angefangen, aber das ist ein Outer Shadow on Container — die inneren Elements (Cards, Inputs) haben kein Elevation-Gefühl.

**Accent wird als Fläche statt als Licht eingesetzt.** `bg-emerald-500/15` auf dem RecordButton ist ein semitransparenter grüner Block. Das sieht aus wie eine tinted Fläche. Premium-CTAs (Superhuman, Linear) nutzen Glow statt Tint: nicht "fillcolor mit Opacity", sondern `box-shadow` mit `blur(40-60px)` die das Licht simuliert, das von einem aktiven Element ausgeht. Das `shadow-[0_0_40px_rgba(16,185,129,0.2)]` auf dem RecordButton ist der einzige Glow im gesamten UI — und selbst der ist zu schwach (0.2 Opacity).

**Einzige Akzentfarbe.** Emerald Green ist der einzige Farbpunkt in der gesamten UI. Kein zweiter visueller Anker. Linear hat Indigo als Primary plus Amber als sekundäre Signal-Farbe für States. Notion hat Blue plus Orange. Die Warning-/Danger-Farben in Voxlit existieren in den Themes als Tokens, aber sie erscheinen nirgendwo im normalen UI — sie sind für Fehlerzustände reserviert. Das bedeutet der gesamte visuelle Informationsraum wird von einem einzigen Grün dominiert.

**Fehlende Typografie-Hierarchie durch Weight.** Alles außer `font-semibold` auf Section Titles ist `font-medium` oder `font-normal` mit Farbabstufung (zinc-300 vs zinc-500) als einziges Unterscheidungsmerkmal. Premium-Apps nutzen aggressivere Weight-Kontraste: `font-bold` oder sogar `font-extrabold` für primäre Labels neben `font-normal` für sekundäre, `font-light` für tertiary. Voxlit hat 2 effektive Weight-Stufen. Linear hat 4.

**Logo-Behandlung in der Sidebar.** Das Logo im Header ist `bg-emerald-500/10 border border-emerald-500/20` — ein Mic-Icon in einer kleinen grünen Box. Das ist funktional aber hat keine visuelle Autorität. Die FloatingBar hat das echte Voxlit-Logo (Cyan + Gold Ringe) — das ist klar besser. Das Haupt-App-Header-Logo wirkt wie ein Platzhalter.

---

## 2. Was Premium-Apps WIRKLICH anders machen

### Linear

Linear macht **Hintergrundfläche aktiv**. Der `#191B22` Background ist nicht neutral — er hat einen minimalen radialen Gradient der die Mitte leicht aufhellt (`radial-gradient(ellipse at 50% 0%, #1E2030 0%, #191B22 70%)`). Das ist so subtil dass du es nicht bewusst siehst, aber dein Auge registriert "hier ist eine Lichtquelle oben". Dazu kommen **kein flacher Separator zwischen Sidebar und Content** — stattdessen unterschiedliche Background-Töne (`#191B22` vs `#14161C`) die mit einem box-shadow getrennt sind.

Linear nutzt Indigo `#5E6AD2` **ausschließlich** für interaktive Elemente: aktive Nav-Items, Focus Rings, Primary CTAs. Nirgendwo sonst. Das erzeugt maximale Signal-Klarheit: Wenn du Indigo siehst, ist da etwas klickbar/aktiv.

**Wichtigster Unterschied:** Linear's aktive Sidebar-Items haben `background: rgba(94,106,210,0.1)` PLUS `box-shadow: inset 0 0 0 1px rgba(94,106,210,0.2)`. Das ist Inset-Shadow als Border-Ersatz. Das wirkt tausendmal hochwertiger als `border: 1px solid` weil es sich integral anfühlt statt aufgesetzt.

### Superhuman

Superhuman macht **Oberflächen-Hierarchie durch 5 Graustufen**. Die eigentliche Technik: sie nutzen keine Borders zwischen Sections innerhalb eines Panels — stattdessen definiert die Hintergrundfarbe die Hierarchie. Primary Content hat `#1B1B1B`, Cards haben `#222226`, Inputs haben `#2A2A2F`, aktive States haben `#333338`, Hover hat `#3E3E44`. Fünf Stufen, null Borders nötig.

Dazu: Superhuman's CTA-Buttons haben keinen Farbblock als Background. Sie haben einen **subtilen Gradient**: `linear-gradient(135deg, #5BBEF5, #4AAEE0)` mit `box-shadow: 0 2px 8px rgba(91,190,245,0.3), 0 0 0 1px rgba(91,190,245,0.15)`. Das sind drei Layer (Gradient + Drop Shadow + Inset Border) auf einem einzigen Button. Ergebnis: der Button sieht aus als würde er leuchten.

### Notion

Notions geheimer Vorteil: **Nicht-schwarz Hintergrund**. `#191919` statt `#0a0a0c` erzeugt sofort mehr Wärme. Combiniert mit dem `#373C3F` Sidebar (das deutlich wärmer ist als Zinc-Töne) entsteht ein Gefühl von "ich sitze in einem dunklen aber warmen Raum" statt "ich starre in ein schwarzes Loch".

Notion nutzt zwei Accent-Farben bewusst als **Pair**: Blau (`#529CCA`) für informationale/navigational Elemente, Orange (`#FFA344`) für destructive/warning-Actions. Diese Farbpaare werden konsistent eingesetzt, nicht gemischt. Das gibt dem UI eine interne Logik die der Nutzer unbewusst lernt.

### Was alle drei gemeinsam haben (und Voxlit fehlt)

1. **Nicht-transparente Borders durch Layer statt durch `border: 1px solid`.** Inset Box-Shadows oder separate Pseudo-Element-Borders die sich mit der Oberfläche integrieren.

2. **Mindestens einen Gradient auf dem Root-Hintergrund.** Kein einziges Premium-UI hat einen uniformen Background-Block auf dem Root-Element.

3. **Fokus-Zustände als Glow, nicht als Outline.** `focus:outline-none` + `focus-visible:box-shadow: 0 0 0 2px rgba(accent, 0.5), 0 0 0 4px rgba(accent, 0.1)`. Voxlit hat `focus-visible:ring-2 focus-visible:ring-white/30` — das ist funktional aber nicht premium.

4. **Schriftschnitt als primäres Hierarchiemittel**, nicht Farbe. Die wichtigste Information ist Bold, nicht nur heller.

---

## 3. Bold Vorschläge

### Background Treatment

**Vorschlag 1: Radialer Hintergrundgradient auf Root.**

```css
/* Ersetzt bg-[#0a0a0c] auf main */
background: radial-gradient(ellipse at 50% 0%, #111116 0%, #0a0a0c 65%);
```

Visueller Effect: Die App-Mitte oben ist minimal heller (~6 Luminanzpunkte), die Ränder fallen ins Dunkel. Das Auge interpretiert das als "Deckenbeleuchtung". Extrem subtil — bei 100% Sättigung nicht sichtbar, nur bei der Gesamtwirkung spürbar.

**Für Voxlit Navy:**

```css
background: radial-gradient(ellipse at 40% 0%, #0f1528 0%, #0A0E1A 70%);
```

**Für Linear-Variante:**

```css
background: radial-gradient(ellipse at 50% -20%, #1f2233 0%, #191B22 60%);
```

### Surface Elevation

**Vorschlag 2: Cards mit dreistufiger Elevation.**

```css
/* Aktuell: */
bg-[#111113] border border-zinc-800/60 rounded-xl p-3

/* Neu: */
background: #111113;
border: none;
box-shadow:
  0 0 0 1px rgba(255,255,255,0.06),  /* subtile Inset-Border durch externe Shadow */
  0 1px 3px rgba(0,0,0,0.4),          /* Base Elevation */
  0 4px 12px rgba(0,0,0,0.25);        /* Tiefe */
border-radius: 12px;
```

Kein `border-zinc-800/60` mehr. Der `box-shadow: 0 0 0 1px rgba(255,255,255,0.06)` erzeugt eine Licht-Kante die lebendiger wirkt als eine harte Border. Die `rgba(255,255,255,...)` statt `rgba(color,...)` ist der Trick — es simuliert Licht das auf die Kante fällt, nicht eine gemalte Linie.

**Vorschlag 3: Panel-Wrapper mit Glow-Basis.**

```css
/* Aktuell auf History/Stats Panels: */
bg-[#0e0e11] border border-zinc-800/60 rounded-2xl shadow-xl shadow-black/30

/* Neu: */
background: #0e0e11;
border-radius: 16px;
box-shadow:
  0 0 0 1px rgba(255,255,255,0.05),
  0 8px 32px rgba(0,0,0,0.5),
  0 2px 8px rgba(0,0,0,0.3);
```

### Accent Color Strategy

**Vorschlag 4: Primary Accent nur für interaktive Elemente, Secondary Accent für States.**

Aktuell: Emerald wird für alles verwendet — aktive Buttons, aktive Icons, aktive Borders, Highlight in Texten.

Vorschlag: Emerald bleibt Primary (Record/Start = grün). Amber `#F59E0B` / `#FBBF24` wird konsequent zur "Activity"-Farbe: Transcribing Spinner, Busy States, Cost Dashboard Highlights. Das ist bereits in den Token-Definitionen vorhanden (`warning: "#f59e0b"`), wird aber nicht als gestalterisches Pair eingesetzt. Die Varianten Carbon, Linear, Notion haben alle eine Secondary Accent Farbe definiert — sie wird nur nie sichtbar genutzt.

**Konkret:** Den `StylePicker` active State von `bg-emerald-500/15 text-emerald-400` auf `bg-emerald-500/10 text-emerald-400 shadow-[inset_0_0_0_1px_rgba(16,185,129,0.2)]` ändern. Der Inset Shadow ersetzt die implizite Border und erzeugt mehr Tiefe.

### Glow und Light Effects

**Vorschlag 5: RecordButton Glow stärker, schichtiger.**

```css
/* Aktuell: */
shadow-[0_0_40px_rgba(16,185,129,0.2)]

/* Neu für Idle State: */
box-shadow:
  0 0 0 1px rgba(16,185,129,0.2),    /* Hard Ring */
  0 0 20px rgba(16,185,129,0.15),    /* Naher Glow */
  0 0 60px rgba(16,185,129,0.08);    /* Weiter Hof */

/* Neu für Recording State: */
box-shadow:
  0 0 0 1px rgba(239,68,68,0.35),
  0 0 20px rgba(239,68,68,0.25),
  0 0 50px rgba(239,68,68,0.12);
```

Drei Schichten: harter Ring (1px), naher Glow (20px blur), weiter Hof (60px blur). Das ist wie ein LED-Licht auf einer dunklen Oberfläche — scharf im Kern, weich nach außen. Linear macht das mit aktiven Sidebar-Items. Superhuman macht das mit CTA Buttons.

**Vorschlag 6: FloatingBar Glow je nach State.**

Die FloatingBar hat `border: 1px solid ${borderColor}` mit RGBA Werten. Gut. Aber es fehlt der externe Glow:

```javascript
/* Zum vorhandenen border ergänzen: */
boxShadow: isRecording
  ? `0 0 0 1px ${borderColor}, 0 4px 20px rgba(147,197,253,0.15), 0 0 40px rgba(147,197,253,0.08)`
  : isDone
  ? `0 0 0 1px ${borderColor}, 0 4px 16px rgba(52,211,153,0.12)`
  : `0 4px 20px rgba(0,0,0,0.6)`,
```

### Typografie

**Vorschlag 7: Weight-Hierarchie ausbauen.**

```
Section Titles:    font-weight: 700 (aktuell: 600) + letter-spacing: 0.1em
Primary Values:    font-weight: 600 (aktuell: 400-500)
Labels/Captions:   font-weight: 400 (aktuell: 400)
Muted/Timestamps:  font-weight: 300 + color: zinc-500 (aktuell: nur Farbe)
```

Der Abstand zwischen `font-semibold` und normalem Text ist zu klein. Ein `font-bold` Section Title neben `font-light` Timestamps erzeugt mehr visuelle Hierarchie als Farbe allein es kann.

**Vorschlag 8: Logo im App-Header.**

Das FloatingBar-Logo (Cyan + Gold Ringe SVG) ist schöner als der grüne Mic-Icon-Placeholder in der App. Den gleichen SVG in den App-Header bringen, mit einer leichten Glow-Behandlung:

```jsx
<div className="w-7 h-7 rounded-lg flex items-center justify-center"
  style={{
    background: 'rgba(26,26,46,0.8)',
    boxShadow: '0 0 0 1px rgba(56,189,248,0.15), 0 0 12px rgba(56,189,248,0.08)',
  }}>
  {/* VoxlitLogo SVG aus FloatingBar.tsx */}
</div>
```

### Micro-Details

**Vorschlag 9: Input Focus Glow statt Focus Ring.**

```css
/* Ersetzt focus:border-emerald-500/40 */
focus:box-shadow: 0 0 0 1px rgba(16,185,129,0.3), 0 0 0 3px rgba(16,185,129,0.08);
focus:border-color: transparent;
```

Das ist weicher und integriert sich in das Surface statt sich aufzusetzen.

**Vorschlag 10: Section Dividers als Gradient statt Border.**

```css
/* Ersetzt border-b border-zinc-800/40 in Panel-Headers */
border: none;
background: linear-gradient(90deg, transparent, rgba(255,255,255,0.07) 20%, rgba(255,255,255,0.07) 80%, transparent);
height: 1px;
```

Gradient Divider mit weichen Enden wirken organischer als harte Borders die an den Kanten abrupt enden.

---

## 4. Verbesserte Theme-Varianten

Die folgenden Varianten ersetzen die bestehenden 5. Sie bauen auf den Hex-Werten der Research auf, ergänzen aber die fehlende Layer-Technik. Die `ThemeSwitcher.applyTheme()`-Funktion müsste erweitert werden um `--gradient-bg`, `--shadow-card` und `--shadow-panel` als CSS Properties zu injizieren.

---

### Variante A: "Obsidian" (Weiterentwicklung von Current + Carbon)

Konzept: Extrem dunkles Fundament, aber nicht flat. Das Licht kommt von oben. Einzige Farbwelt: Cyan/Teal als Accent (statt Emerald — näher an der Brand-Identität des Voxlit-Logos). Amber als Activity-Farbe.

```
bg:           #0C0C10
surface:      #131318
elevated:     #1A1A21
border:       --  (ersetzt durch box-shadow)
text:         #F0F0F4
muted:        #7A7A88
dim:          #4A4A58
primary:      #22D3EE  (Cyan — direkt aus FloatingBar Logo)
accent:       #67E8F9  (heller Cyan für Hover)
secondary:    #F59E0B  (Amber für Activity)
```

**Zusätzliche CSS-Properties:**

```css
--gradient-bg: radial-gradient(ellipse at 50% 0%, #141420 0%, #0C0C10 65%);

/* Card Elevation */
--shadow-card:
  0 0 0 1px rgba(255,255,255,0.055),
  0 1px 3px rgba(0,0,0,0.45),
  0 4px 14px rgba(0,0,0,0.28);

/* Panel Elevation */
--shadow-panel:
  0 0 0 1px rgba(255,255,255,0.05),
  0 8px 32px rgba(0,0,0,0.55),
  0 2px 8px rgba(0,0,0,0.3);

/* Record Button Glow (Idle) */
--glow-primary:
  0 0 0 1px rgba(34,211,238,0.2),
  0 0 20px rgba(34,211,238,0.15),
  0 0 55px rgba(34,211,238,0.07);

/* Record Button Glow (Recording) */
--glow-recording:
  0 0 0 1px rgba(239,68,68,0.35),
  0 0 22px rgba(239,68,68,0.22),
  0 0 50px rgba(239,68,68,0.10);
```

Warum das Premium wirkt: Cyan als Brand-Farbe ist kohärent mit dem FloatingBar Logo und dem "Voxlit Navy" Theme, aber der dunklere Hintergrund gibt mehr Tiefe. Die drei Glow-Schichten auf dem RecordButton machen den primären CTA zu einem echten Lichtpunkt im UI.

---

### Variante B: "Slate" (Weiterentwicklung von Linear)

Konzept: Blau-getontes Dunkelgrau, wie Linear aber mit mehr Kontrast und weniger "flat developer tool". Für Nutzer die professionelle Produktivitäts-Apps kennen.

```
bg:           #15171F
surface:      #1B1D27
elevated:     #22253B
border:       --  (ersetzt durch box-shadow)
text:         #E8ECF4
muted:        #747D8C
dim:          #505866
primary:      #7C8CF8  (Indigo — etwas heller als Linear's #5E6AD2, mehr Pop)
accent:       #A5B0FF  (Lilac für Hover States)
secondary:    #FCD34D  (Warm Yellow für Activity — warm gegen cool contrast)
```

**Zusätzliche CSS-Properties:**

```css
--gradient-bg: radial-gradient(ellipse at 45% 0%, #1C2035 0%, #15171F 60%);

--shadow-card:
  0 0 0 1px rgba(255,255,255,0.06),
  0 1px 4px rgba(0,0,0,0.4),
  0 5px 16px rgba(0,0,0,0.25);

--shadow-panel:
  0 0 0 1px rgba(255,255,255,0.055),
  0 10px 36px rgba(0,0,0,0.5),
  0 3px 10px rgba(0,0,0,0.3);

--glow-primary:
  0 0 0 1px rgba(124,140,248,0.22),
  0 0 18px rgba(124,140,248,0.14),
  0 0 50px rgba(124,140,248,0.06);

--glow-recording:
  0 0 0 1px rgba(248,113,113,0.3),
  0 0 20px rgba(248,113,113,0.18),
  0 0 45px rgba(248,113,113,0.08);
```

Warum anders als der aktuelle "Linear": Der bestehende Linear-Theme ist `#191B22` Background — flach, unspektakulär. "Slate" hat `radial-gradient` oben, der die Decke simuliert. Die Indigo-Farbe ist nach `#7C8CF8` aufgehellt weil auf dunklem Hintergrund die dunkleren Indigo-Töne (#5E6AD2) zu wenig Pop haben. Warm Yellow als Secondary erzeugt echten Warm/Cool-Kontrast.

---

### Variante C: "Ember" (New — Warm Dark für Nicht-Entwickler-Zielgruppe)

Konzept: Warm, organisch, nicht klinisch. Keine Blau-Töne. Basiert auf Wispr Flow's "quiet luxury" Ansatz. Für Nutzer die von Notion oder Bear kommen.

```
bg:           #13100D
surface:      #1C1814
elevated:     #241F1A
border:       --  (ersetzt durch box-shadow)
text:         #F5F0E8  (Cream — wie Wispr's #FFFFEB aber etwas weniger grell)
muted:        #8C8278
dim:          #5C5550
primary:      #F97316  (Orange — warm, eindeutig, kein grünes Diktat-Tool-Klischee)
accent:       #FB923C  (Heller Orange für Hover)
secondary:    #A3E635  (Lime — sharp contrast zu warm orange)
```

**Zusätzliche CSS-Properties:**

```css
--gradient-bg: radial-gradient(ellipse at 40% 0%, #1A1410 0%, #13100D 65%);

--shadow-card:
  0 0 0 1px rgba(255,240,220,0.055),
  0 1px 3px rgba(0,0,0,0.45),
  0 4px 12px rgba(0,0,0,0.3);

--shadow-panel:
  0 0 0 1px rgba(255,240,220,0.04),
  0 8px 30px rgba(0,0,0,0.5),
  0 2px 8px rgba(0,0,0,0.3);

--glow-primary:
  0 0 0 1px rgba(249,115,22,0.25),
  0 0 18px rgba(249,115,22,0.18),
  0 0 50px rgba(249,115,22,0.08);

--glow-recording:
  0 0 0 1px rgba(239,68,68,0.3),
  0 0 20px rgba(239,68,68,0.2),
  0 0 45px rgba(239,68,68,0.09);
```

Warum das funktioniert: Orange als Primary ist mutig aber differenzierend. Kein Voice Tool hat Orange als Hauptfarbe. Die warmen Grautöne fühlen sich an wie gutes Papier, nicht wie ein Terminal. Cream-Text reduziert Halation bei langen Sessions (Superhuman's Kernargument für nicht-weißes Text). Lime als Secondary schafft einen visuellen Schock-Kontrast der sparingly eingesetzt sehr effektiv ist.

---

## 5. Priorisierung: Impact vs. Aufwand

Geordnet nach visuellem Bang-for-Buck. Jede dieser Änderungen kann unabhängig und inkrementell eingebaut werden.

### Sofort umsetzbar (< 30 Minuten, größter Impact)

**1. Hintergrund-Gradient auf Root-Element.**
Eine Zeile CSS-Änderung auf `main` in `App.tsx`. Von `bg-[#0a0a0c]` zu einem Inline-Style mit `radial-gradient`. Visuelle Wirkung: sofort deutlich wahrnehmbarer Tiefeneffekt.

**2. Card-Border durch Box-Shadow ersetzen.**
In `ui.tsx` die `StatCard` und in `App.tsx` die History-Cards. Border weg, Shadow mit drei Schichten (1px Licht-Ring + Drop) rein. Das ist die schnellste Weg von "flat tool" zu "premium product".

**3. RecordButton Glow von mono auf drei Schichten.**
In `App.tsx` Zeile 69. Die eine Shadow-Klasse durch drei gestaffelte Shadows ersetzen. Das ist ein Einzeiler.

### Mittelfristig (< 2 Stunden, hoher Impact)

**4. Logo im App-Header.**
Das FloatingBar-SVG-Logo als wiederverwendbare Komponente herausziehen und in den App-Header bringen. Ersetzt den semantisch schwachen Mic-Icon-in-grüner-Box-Placeholder.

**5. Input Focus Glow.**
In `ui.tsx` `INPUT_CLS` updaten: `focus:ring-0 focus:border-transparent focus:[box-shadow:0_0_0_1px_rgba(16,185,129,0.3),0_0_0_3px_rgba(16,185,129,0.08)]`. Erfordert Tailwind arbitrary value Syntax oder direkte `style` Props.

**6. StylePicker Active State auf Inset-Shadow.**
Der aktive Tab-Indicator im StylePicker ist `bg-emerald-500/15` — eine schwache tinted Fläche. Mit `box-shadow: inset 0 0 0 1px rgba(16,185,129,0.25)` plus dem Background wird er plastischer.

### Mittelfristig als Theme-Architektur-Upgrade

**7. `ThemeSwitcher.applyTheme()` um Gradient + Shadow Tokens erweitern.**
Aktuell injiziert der ThemeSwitcher nur Farbwerte. Eine saubere Erweiterung: `--gradient-bg`, `--shadow-card`, `--shadow-panel`, `--glow-primary` als CSS-Properties ergänzen. Dann `App.tsx` auf diese Properties umstellen statt Hardcoded Shadows. Das macht die drei neuen Varianten vollständig austauschbar.

**8. Section Dividers als Gradient.**
Panel-Header `border-b border-zinc-800/40` durch ein Pseudo-Element mit horizontalem Gradient ersetzen. Erfordert eine kleine Wrapper-Komponente `<PanelHeader>` die konsistent eingesetzt wird.

### Niedrigste Priorität (polish, nicht Fundament)

- Letter-Spacing auf Section Titles von `tracking-widest` zu `tracking-[0.1em]` (schon nah genug)
- Font-Weight Hierarchie (`font-bold` für primäre Werte statt `font-semibold`)
- FloatingBar: Glow-Schichtung je State (bereits funktional, nur polish)

---

## Kernaussage für den Gründer

Die Themes wirken monofarbig weil sie ausschließlich auf Farbwerte setzen. Premium-UIs bauen räumliche Tiefe durch Licht-Simulation: Gradients die eine Lichtquelle implizieren, Box-Shadows die Elevation zeigen, Glow-Effekte die aktive Elemente anstrahlen. Das sind keine Farbentscheidungen — das sind Entscheidungen über Materialität.

Die grösste Einzelverbesserung mit dem kleinsten Aufwand: Hintergrund-Gradient auf dem Root-Element, und Card-Borders durch drei-schichtige Box-Shadows ersetzen. Das allein bringt Voxlit visuell von "gut gestaltet" in die Nähe von Linear und Superhuman. Alles andere ist Verfeinerung darauf.
