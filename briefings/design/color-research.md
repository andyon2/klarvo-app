# Color Research -- Dark Mode Paletten hochwertiger Apps

Erstellt: 2026-03-21

## Apps analysiert

### Superhuman Carbon
- **Quelle:** blog.superhuman.com/how-to-design-delightful-dark-themes
- **Background:** Cod Gray #1B1B1B (kein reines Schwarz)
- **Blog-CSS:** --bg-color: #27292D, --color-text: rgba(255,255,255,0.95), --color-cta: #5BBEF5
- **Prinzip:** 5 Graustufen. Naehere Surfaces = heller, entfernte = dunkler. Kein reines Schwarz, kein reines Weiss
- **Accent:** Picton Blue #5BBEF5
- **Kontrast:** Bewusst NICHT 21:1 (pures W auf S), sondern ~15:1. Reduziert Halation bei Astigmatismus

### Notion Dark Mode
- **Quelle:** notionavenue.co/post/notion-color-code-hex-palette
- **Main Window:** #2F3438 (deutlich heller als die meisten Dark Modes!)
- **Sidebar:** #373C3F
- **Hover:** #3F4448
- **Text Grey:** #979A9B
- **Accent Blue:** #529CCA
- **Accent Orange:** #FFA344
- **Danger:** #FF7369
- **Prinzip:** Warm gray-green Toene, nicht kalt. Approachable, nicht klinisch.

### Linear
- **Quelle:** linear.app/now/how-we-redesigned-the-linear-ui
- **System:** LCH-Farbsystem, 3 Variablen (base, accent, contrast) generieren alles
- **Background:** ~#191B22 (blau-getoentes Dunkelgrau, prozedural generiert)
- **Accent:** Indigo ~#5E6AD2 (Signaturfarbe)
- **Prinzip:** "Chrome" (Blau-Anteil) in Berechnungen bewusst limitiert. Restrained accent usage.
- **Kontrast:** Variable 30-100 fuer Accessibility

### Wispr Flow
- **Quelle:** wisprflow.ai/media-kit, wisprflow.ai/rebrand
- **Primary Accent:** #4D65FF (Indigo-Blau)
- **Text:** #FFFFEB (warmes Cream, nicht reines Weiss)
- **Dark Teal:** #034F46
- **Stil:** "Quiet luxury and editorial design" statt klinisch. Soft neutrals.
- **Keine konkreten Background-Hex-Werte oeffentlich verfuegbar**

## Zentrale Erkenntnisse

1. **Kein reines Schwarz.** Superhuman, Notion, Linear -- alle starten bei #191919 bis #1B1B1B, nicht bei #0a0a0c.
2. **Getoente Grays statt neutral.** Linear: blau-getoedt. Notion: gruen-warm. Superhuman: neutral-warm.
3. **Kontrast bewusst reduziert.** Nicht max Kontrast (21:1), sondern ~15:1 fuer Komfort.
4. **Accent sparsam.** Linear nutzt Indigo fast nur fuer interaktive Elemente. Nicht als Flaechenfarbe.
5. **Surface-Abstufungen sind subtil.** 2-4 Stufen, Differenz oft nur 5-8 Luminanz-Punkte.

## Varianten im ThemeSwitcher

| ID | Name | Vorbild | Background | Primary Accent |
|----|------|---------|------------|----------------|
| current | Current | - | #0a0a0c | #10b981 (Emerald) |
| carbon | Carbon | Superhuman | #1B1B1B | #5BBEF5 (Picton Blue) |
| linear | Linear | Linear | #191B22 | #5E6AD2 (Indigo) |
| notion | Notion Warm | Notion | #191919 | #529CCA (Blue) |
| voxlit-navy | Voxlit Navy | Social Preview | #0A0E1A | #22D3EE (Teal) |

Preview: `npm run preview` → http://localhost:1422/ → Theme-Switcher unten rechts
