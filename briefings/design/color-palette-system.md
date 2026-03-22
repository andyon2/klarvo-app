# Color Palette System — Voxlit Theme Variants

Erstellt: 2026-03-21
Kontext: Folgeproblem zur `ui-expert-critique.md` — Themes definieren zwar `primary`, `secondary`, `accent`
als Tokens, aber im echten UI ist nur EINE Farbe sichtbar (alles Emerald/Primary).

---

## Bestandsaufnahme: Wo Akzentfarbe im UI vorkommt

### App.tsx
| Element | Farbe (aktuell) | Klasse |
|---|---|---|
| RecordButton idle | Emerald | `bg-emerald-500/15 text-emerald-400 shadow-[0_0_40px_rgba(16,185,129,0.2)]` |
| RecordButton recording | Red | `bg-red-500/20 text-red-400 shadow-[0_0_40px_rgba(239,68,68,0.3)]` |
| RecordButton busy | Amber | `bg-amber-500/15 text-amber-400 shadow-[0_0_30px_rgba(245,158,11,0.2)]` |
| RecordButton border idle | Emerald | `border-emerald-500/25` |
| RecordButton border recording | Red | `border-red-500/40` |
| RecordButton border busy | Amber | `border-amber-500/30` |
| StylePicker active tab | Emerald | `bg-emerald-500/15 text-emerald-400` |
| Nav icon active (alle) | Emerald | `text-emerald-400 bg-emerald-500/10` |
| Logo icon | Emerald | `bg-emerald-500/10 border-emerald-500/20 text-emerald-400` |
| ReformatButton loading state | Amber | `bg-amber-500/10 border-amber-500/20 text-amber-400` |

### CostDashboard.tsx
| Element | Farbe (aktuell) | Klasse |
|---|---|---|
| StatTile highlight border | Emerald | `border-emerald-500/30` |
| StatTile highlight value | Emerald | `text-emerald-400` |
| Savings banner bg | Emerald | `bg-emerald-500/10 border-emerald-500/20` |
| Savings banner title | Emerald | `text-emerald-400` |
| Savings amount | Emerald | `text-emerald-300` |
| Savings fine print | Emerald | `text-emerald-600` |

### ui.tsx
| Element | Farbe (aktuell) | Klasse |
|---|---|---|
| StatusDot active | Emerald | `bg-emerald-500` |
| FillerStatsChart bar fill | Emerald | `bg-emerald-500/50` |
| HighlightedText mark | Emerald | `bg-emerald-500/30 text-emerald-300` |
| StatCard bg/border | Zinc | `bg-[#111113] border-zinc-800/60` (kein Accent) |
| INPUT_CLS focus border | Emerald | `focus:border-emerald-500/40` |

### FloatingBar.tsx
| Element | Farbe (aktuell) | Wert |
|---|---|---|
| Waveform bars | Blue | `rgba(147,197,253,0.85)` |
| Processing spinner | Amber | `#fbbf24` |
| Done icon + label | Teal | `#34d399` |
| Recording border | Blue tint | `rgba(147,197,253,0.25)` |
| Processing border | Amber tint | `rgba(245,158,11,0.2)` |
| Done border | Teal tint | `rgba(52,211,153,0.25)` |
| Error | Red | `#f87171` |
| ClipboardOnly | Amber | `#fbbf24` |

### Onboarding.tsx
| Element | Farbe (aktuell) | |
|---|---|---|
| StepDots active | Emerald | `bg-emerald-400` |
| StepDots past | Emerald dimmed | `bg-emerald-500/40` |
| BTN_PRIMARY | Emerald | `bg-emerald-500/15 border-emerald-500/30 text-emerald-400` |
| ApiKeyField valid border | Emerald | `border-emerald-500/50` |
| ExternalLink text | Emerald | `text-emerald-400` |

---

## Farbrollensystem (5 funktionale Rollen)

Die FloatingBar hat das richtige System bereits intuitiv umgesetzt — es braucht nur
in die App-Breite gehoben und in den ThemeSwitcher integriert werden.

| Rolle | Bedeutung | Elemente |
|---|---|---|
| **Action** | Primäre interaktive CTAs, klickbar, wichtigste Entscheidung | RecordButton idle, StylePicker active tab, Primary Buttons, Nav active icon |
| **Activity** | Laufende Prozesse, "etwas passiert gerade", Busy/Spinner | RecordButton busy, Transcribing/Cleaning spinner, Reformat loading, Waveform bars |
| **Success** | Abgeschlossen, bestätigt, "alles gut" — NICHT gleich wie Action | RecordButton done, "Done" label + checkmark, StatusDot, Savings badge |
| **Info** | Informationale Links, Hotkey-Badge, Sekundär-Hervorhebungen | External links, API key links, Highlight-marks in Suchergebnissen |
| **Warm** | Statistik-Highlights, Tags, visueller Kontrast-Akzent, Cost accent | CostDashboard highlight tile, Savings-Vergleich Banner, Clipboard-Done |
| **Danger** | Destruktiv, Stop, Fehler — immer Rot | RecordButton recording, Error-State (varianten-übergreifend gleich) |

**Schlüsselprinzip:** Action und Success MÜSSEN verschiedene Farben sein.
Wenn der RecordButton (Action) und das "Done"-Feedback (Success) dieselbe Farbe haben,
lernt der User nicht, diese States zu unterscheiden. Premium-Apps machen hier immer
einen bewussten Kontrast.

---

## Palette + Mapping pro Variante

### OBSIDIAN

Konzept: Cyan als Brand-Farbe (direkt aus dem FloatingBar-Logo). Dunkel, präzise, keine Wärme.

```
bg:         #0C0C10
surface:    #131318
elevated:   #1A1A21
text:       #F0F0F4
muted:      #7A7A88
dim:        #4A4A58

Action:     #22D3EE  (Cyan)
Activity:   #FBBF24  (Amber)
Success:    #2DD4BF  (Teal — unterschiedlich von Cyan, aber verwandt)
Info:       #818CF8  (Soft Indigo)
Warm:       #FB923C  (Orange)
Danger:     #EF4444  (Red — konstant)
```

Element-Mapping:
```
RecordButton idle:          Action bg/glow + Action border
RecordButton recording:     Danger bg/glow + Danger border
RecordButton busy:          Activity bg/glow + Activity border
StylePicker active tab:     Action bg tint + Action text
Nav icon active:            Action text + Action bg tint
Logo icon:                  Action tint bg (von Emerald-Placeholder → Cyan)
StatusDot active:           Success
FillerStatsChart bars:      Activity tint (Amber)
HighlightedText mark:       Info bg tint + Info text
INPUT_CLS focus:            Action border
CostDashboard highlight:    Warm border + Warm text
Savings banner:             Warm bg tint + Warm title text
External links:             Info text
StepDots active:            Action
BTN_PRIMARY:                Action tint bg + Action border + Action text
FloatingBar waveform:       Activity (Amber statt Blue — passt besser zu Amber als Activity-Farbe)
FloatingBar done:           Success
FloatingBar processing:     Activity
```

CSS-Properties:
```css
--gradient-bg: radial-gradient(ellipse at 50% 0%, #141420 0%, #0C0C10 65%);
--shadow-card: 0 0 0 1px rgba(255,255,255,0.055), 0 1px 3px rgba(0,0,0,0.45), 0 4px 14px rgba(0,0,0,0.28);
--shadow-panel: 0 0 0 1px rgba(255,255,255,0.05), 0 8px 32px rgba(0,0,0,0.55), 0 2px 8px rgba(0,0,0,0.3);
--glow-primary: 0 0 0 1px rgba(34,211,238,0.2), 0 0 20px rgba(34,211,238,0.15), 0 0 55px rgba(34,211,238,0.07);
--glow-recording: 0 0 0 1px rgba(239,68,68,0.35), 0 0 22px rgba(239,68,68,0.22), 0 0 50px rgba(239,68,68,0.10);
```

---

### SLATE

Konzept: Blau-getöntes Dunkelgrau a la Linear, aber mehr Kontrast und mehr Pop.
Für Nutzer die produktive Apps wie Linear, Notion kennen.

```
bg:         #15171F
surface:    #1B1D27
elevated:   #22253B
text:       #E8ECF4
muted:      #747D8C
dim:        #505866

Action:     #7C8CF8  (Indigo — heller als Linear's #5E6AD2, mehr Pop auf dunklem BG)
Activity:   #FCD34D  (Warm Yellow — Warm/Cool-Kontrast zu Indigo)
Success:    #34D399  (Emerald Green — klassisch, funktioniert gut als "Done")
Info:       #67E8F9  (Cyan — informationell, unterschiedlich von Action)
Warm:       #FB923C  (Orange — Statistik-Highlights)
Danger:     #F87171  (Rose — etwas weicher als pure Red auf blauem BG)
```

Element-Mapping:
```
RecordButton idle:          Action bg/glow
RecordButton recording:     Danger bg/glow
RecordButton busy:          Activity bg/glow
StylePicker active tab:     Action bg + Action text
Nav icon active:            Action text + Action bg tint
Logo icon:                  Action tint
StatusDot active:           Success
FillerStatsChart bars:      Activity tint
HighlightedText mark:       Info bg tint + Info text
CostDashboard highlight:    Warm border + Warm text
Savings banner:             Success tint (Emerald — passt semantisch)
External links:             Info text
StepDots:                   Action
BTN_PRIMARY:                Action
```

CSS-Properties:
```css
--gradient-bg: radial-gradient(ellipse at 45% 0%, #1C2035 0%, #15171F 60%);
--shadow-card: 0 0 0 1px rgba(255,255,255,0.06), 0 1px 4px rgba(0,0,0,0.4), 0 5px 16px rgba(0,0,0,0.25);
--shadow-panel: 0 0 0 1px rgba(255,255,255,0.055), 0 10px 36px rgba(0,0,0,0.5), 0 3px 10px rgba(0,0,0,0.3);
--glow-primary: 0 0 0 1px rgba(124,140,248,0.22), 0 0 18px rgba(124,140,248,0.14), 0 0 50px rgba(124,140,248,0.06);
--glow-recording: 0 0 0 1px rgba(248,113,113,0.3), 0 0 20px rgba(248,113,113,0.18), 0 0 45px rgba(248,113,113,0.08);
```

---

### EMBER

Konzept: Warm, organisch, kein Blau. Für Nutzer die von Notion oder Bear kommen.
Kein Voice-Tool hat Orange als Hauptfarbe — differenzierend.

```
bg:         #13100D
surface:    #1C1814
elevated:   #241F1A
text:       #F5F0E8  (Cream)
muted:      #8C8278
dim:        #5C5550

Action:     #F97316  (Orange — warm, mutig, einzigartig)
Activity:   #A3E635  (Lime — Sharp contrast zu warm orange)
Success:    #34D399  (Emerald — klassisch grüne Bestätigung)
Info:       #67E8F9  (Cyan — cool tones als Info-Gegenpart zum warmen Primär)
Warm:       #FBBF24  (Amber — Statistik-Highlights, harmonisch mit Orange)
Danger:     #F87171  (Rose-Red — passt in warme Palette)
```

Element-Mapping:
```
RecordButton idle:          Action bg/glow (Orange)
RecordButton recording:     Danger bg/glow
RecordButton busy:          Activity bg/glow (Lime)
StylePicker active tab:     Action bg + Action text
Nav icon active:            Action text + Action bg tint
Logo icon:                  Action tint
StatusDot active:           Success (Emerald — "ready")
FillerStatsChart bars:      Warm tint (Amber)
HighlightedText mark:       Info bg tint + Info text (Cyan)
CostDashboard highlight:    Warm border + Warm text
Savings banner:             Action tint (Orange)
External links:             Info text (Cyan)
StepDots:                   Action (Orange)
BTN_PRIMARY:                Action
```

CSS-Properties:
```css
--gradient-bg: radial-gradient(ellipse at 40% 0%, #1A1410 0%, #13100D 65%);
--shadow-card: 0 0 0 1px rgba(255,240,220,0.055), 0 1px 3px rgba(0,0,0,0.45), 0 4px 12px rgba(0,0,0,0.3);
--shadow-panel: 0 0 0 1px rgba(255,240,220,0.04), 0 8px 30px rgba(0,0,0,0.5), 0 2px 8px rgba(0,0,0,0.3);
--glow-primary: 0 0 0 1px rgba(249,115,22,0.25), 0 0 18px rgba(249,115,22,0.18), 0 0 50px rgba(249,115,22,0.08);
--glow-recording: 0 0 0 1px rgba(239,68,68,0.3), 0 0 20px rgba(239,68,68,0.2), 0 0 45px rgba(239,68,68,0.09);
```

---

## Bestehende Varianten: Ergänzung mit neuen Rollen

### CARBON (Superhuman-inspiriert)

```
Action:   #5BBEF5  (Blue — bestehend als primary)
Activity: #FBBF24  (Amber — bestehend als secondary)
Success:  #34D399  (Emerald — neu, war vorher nicht unterschieden von Action)
Info:     #7DD0F8  (Light Blue — bestehend als accent, passt als Info-Farbe)
Warm:     #FB923C  (Orange — neu, für Statistik-Highlights)
```

### LINEAR

```
Action:   #5E6AD2  (Indigo — bestehend)
Activity: #F59E0B  (Amber — bestehend als secondary)
Success:  #2DD4BF  (Teal — neu, visuell klar von Indigo unterschieden)
Info:     #818CF8  (Soft Indigo — bestehend als accent)
Warm:     #FB923C  (Orange — neu)
```

### NOTION WARM

```
Action:   #529CCA  (Blue — bestehend)
Activity: #FFA344  (Orange — bestehend als secondary)
Success:  #4ADE80  (Green — neu, Notion-typisch)
Info:     #6CB4DA  (Light Blue — bestehend als accent)
Warm:     #FFA344  (Orange — gleich wie Activity, Notion hat nur 2 Accents)
```

### VOXLIT NAVY

```
Action:   #22D3EE  (Cyan — bestehend, Brand-Farbe)
Activity: #FBBF24  (Amber — bestehend als secondary, Logo-Farbe)
Success:  #2DD4BF  (Teal — bestehend als accent)
Info:     #38BDF8  (Sky Blue — neu, informationell)
Warm:     #FBBF24  (Amber — gleich wie Activity, kommt aus Logo)
```

---

## Implementation: ThemeSwitcher CSS-Override Strategie

Die neuen Farbrollen werden in `applyTheme()` als CSS-Properties injiziert:

```
--color-voxlit-activity  →  amber-400/500 Klassen
--color-voxlit-success   →  "Done"-State, Checkmarks, StatusDot
--color-voxlit-info      →  Links, Info-Badges, HighlightedText marks
--color-voxlit-warm      →  CostDashboard highlight, Savings banner
```

Zusätzlich werden die Quick-Win CSS-Overrides bei allen Nicht-"Current"-Varianten
aktiv:
1. `--gradient-bg` auf `main`-Element via Body-Override
2. `--shadow-card` ersetzt `border-zinc-800/60` Klassen auf Cards
3. `--glow-primary` auf RecordButton (3-Layer statt Mono-Shadow)

### CSS-Klassen die geupdated werden müssen

**Activity (amber → variablen-definierte Farbe):**
- `.bg-amber-500\/15` → Activity tint
- `.bg-amber-500\/10` → Activity tint (schwächer)
- `.border-amber-500\/30` → Activity border
- `.border-amber-500\/20` → Activity border (schwächer)
- `.text-amber-400` → Activity text
- `.shadow-\[0_0_30px_rgba\(245\,158\,11\,0\.2\)\]` → Activity glow

**Success (emerald in FloatingBar-artigen Done-States):**
- Der "Done"-State in App.tsx und FloatingBar.tsx nutzt bereits `#34d399` / `text-emerald-300`
  (heller als Action-Emerald `#10b981`). Damit besteht de-facto schon ein Unterschied.
  Im ThemeSwitcher-CSS wird `text-emerald-300` → success color gemappt.

**Info:**
- `text-emerald-400` in Link-Kontexten (ApiKeyField "Key erstellen") → info color
  ABER: Das ist dieselbe Klasse wie Action — der ThemeSwitcher kann nicht zwischen
  Link-Kontext und Button-Kontext unterscheiden. Info-Farbe wird über einen neuen
  CSS-Custom-Property-Ansatz eingeführt sobald Komponenten auf `var(--color-voxlit-info)` umgestellt werden.

**Warm:**
- Die CostDashboard-Highlight StatTile nutzt `border-emerald-500/30` und `text-emerald-400`.
  Im ThemeSwitcher-Override: diese Klassen werden umgemappt — Problem ist dass sie
  auch an anderen Stellen verwendet werden. Die sauberste Lösung wäre eine dedizierte
  `.theme-warm-*` Klasse auf den Dashboard-Elementen.
  Im Rahmen des ThemeSwitcher-only Mandats: der Savings-Banner (`bg-emerald-500/10`)
  wird zur Warm-Farbe, da er semantisch als "gute Nachricht" / Highlight fungiert.

---

## Wichtigste Erkenntnis

Die grösste visuelle Verbesserung durch das ThemeSwitcher-Update ist nicht die
Farbrollen-Differenzierung allein — es sind die drei Layer-Techniken (Gradient BG,
Card Shadow, RecordButton Glow), die das UI von "flat tool" zu "räumlichem Interface"
upgraden.

Die Farbrollen-Differenzierung ist der zweite Schritt und wirkt am stärksten wenn
die Komponenten selber auf `var(--color-voxlit-success)` etc. umgestellt werden
(das liegt ausserhalb des ThemeSwitcher-only Mandats dieses Tasks).
