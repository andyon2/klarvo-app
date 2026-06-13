# 03 — Aktuelle Design-Tokens (Ist-Zustand)

Quelle: `src/styles.css` (`@theme`, Tailwind v4) + inline-Styles. Das ist die heutige
Design-Sprache — als Ausgangspunkt, nicht als Fessel. Der Designer darf sie weiterentwickeln,
soll Änderungen aber begründen.

## Farb-Tokens (Dark, Notion-artig)

| Token | Hex | Rolle |
|---|---|---|
| `klarvo-bg` | `#191919` | App-Hintergrund (fast schwarz, warm) |
| `klarvo-surface` | `#252525` | Karten/Panels |
| `klarvo-elevated` | `#2F3438` | erhöhte Flächen |
| `klarvo-border` | `#373C3F` | Standard-Rand |
| `klarvo-border-active` | `#3F4448` | aktiver/hover Rand |
| `klarvo-text` | `#FFFFFFEB` | Haupttext (~92% weiß) |
| `klarvo-muted` | `#AAACAD` | Sekundärtext |
| `klarvo-dim` | `#8E9093` | Tertiär/Labels |
| `klarvo-primary` | `#2AC3A8` | **Teal — Primär/Marke/aktiv** |
| `klarvo-accent` | `#52D4C4` | helles Teal — Highlights |
| `klarvo-secondary` / `warm` / `activity` | `#FFA344` | **Warm-Orange — Aktivität/Aufnahme** |
| `klarvo-warning` | `#FFA344` | Warnung |
| `klarvo-danger` | `#FF7369` | Fehler/Löschen |
| `klarvo-success` | `#4ADE80` | Erfolg |
| `klarvo-info` | `#52D4C4` | Info |

Logo-Teal (FloatingBar, leicht abweichend): `#14B8A6`.

## Beobachtungen zur heutigen Sprache
- **Zwei Akzentachsen:** Teal (Marke/Status-aktiv) + Orange (Aufnahme/Aktivität). Das ist
  charakteristisch — beim Redesign bewusst behandeln (beibehalten oder gezielt schärfen).
- **Radii:** gemischt — `rounded-lg` (8px), `rounded-xl` (12px), `rounded-full` für Tags/Pille.
- **Typo:** Tailwind-Default-Stack; Größen `text-[11px]` (Labels) bis `text-lg` (Header).
  Labels oft `uppercase tracking-wide`. Keine Custom-Schrift im Einsatz.
- **Spacing:** dicht (`gap-1` bis `gap-3`, `p-3`), passend zum Power-Tool-Anspruch.
- **Elevation:** kaum echte Schatten — Tiefe entsteht nur über Border/Surface-Stufen.
  → Hier liegt sichtbarer Hebel für ein "moderneres" Gefühl (sanfte Schatten/Glas/Tiefe).
- **Scrollbars:** custom, 4px, dezent. Native `<select>`-Dropdowns (heller OS-Stil) brechen
  die Dark-Ästhetik — Kandidat für Custom-Komponenten.
- **Motion:** ein paar Keyframes in der FloatingBar (`done-pop`, `bar-expand/collapse`, `spin`).

## Gemessene Akzent-Häufigkeit (grep über `src/`)
Orange `#FFA344` (33×) und Teal `#2AC3A8` (13×) dominieren als Akzente; viele Einzel-Hex
inline (`#7B8CDB`, `#5BBEF5`, `#22D3EE` …) → **inkonsistente Verstreuung**, ein Grund für
das "nicht ganz aus einem Guss"-Gefühl. Token-Konsolidierung ist Teil des Hebels.
