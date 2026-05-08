---
name: Story 11.2 — Token Foundation (minimal)
epic: 11
story_number: "11.2"
status: ready-for-dev
dependencies:
  - "11-1-pill-bar-html-reimpl"  # pill-bar.html baseline established; cancel_recording + Capability done
inputDocuments:
  - _bmad-output/planning-artifacts/ux-design-specification.md  # §Design System Foundation + §Visual Design Foundation (Step 8 palette)
  - _bmad-output/planning-artifacts/epics.md                    # Epic 11 full definition
  - shells/windows/src/pill-bar.html                            # file being refactored — read before touching
---

# Story 11.2: Token Foundation (minimal)

Status: ready-for-dev

## Story

Als Klarvo-Entwickler
möchte ich eine single-source-of-truth `design-tokens.toml` mit CSS-Generator (xtask),
damit alle Farbwerte, Timing-Werte und Radii nur an einer Stelle definiert sind — und `pill-bar.html` keine hardcodierten Werte mehr enthält.

## Context & Motivation

`pill-bar.html` nach Story 11.1 enthält mehrere hardcodierte Werte:
- `rgba(13, 15, 20, 0.85)` — Overlay-Hintergrund
- `#14B8A6` — Action-Farbe (K-Logo, Waveform-Bars)
- `rgba(248, 113, 113, 0.9)` — Danger-Farbe (Abort-Button)
- `300ms` — Fade-Transition

UX-Spec §Design System Foundation mandatiert: **"Hard-coded color/spacing values are anti-pattern at the component level — they live only in the source-of-truth `design-tokens.toml`."**

Diese Story etabliert das Token-System in minimaler Form — nur was die Pill Bar jetzt braucht. shadcn-init, `Tokens.kt` (Android), Typography-Tokens und Spacing-Tokens kommen erst in späteren Epics (Settings, Onboarding, Android).

**Prerequisite für Stories 11.3–11.6:** Jede folgende Pill-Bar-Story baut auf `tokens.css` auf. Token Foundation muss done+committed sein bevor 11.3 startet.

## Scope — Was IN dieser Story ist

| Item | Status |
|---|---|
| `design-tokens.toml` (repo root) — Color + Timing + Radius | NEW |
| `xtask/src/gen_tokens.rs` — Generator-Modul | NEW |
| `xtask/src/main.rs` — `gen-tokens` Subcommand registrieren | MODIFY |
| `shells/windows/src/styles/tokens.css` — generierte Ausgabe (committed) | NEW |
| `shells/windows/src/pill-bar.html` — hardcoded → `var(--klarvo-*)` | MODIFY |

## Scope — Was NICHT in dieser Story ist

- Typography-Tokens (`font-family`, `font-size`, `line-height`) → erst Settings/Onboarding-Epics
- Spacing-Tokens → erst Settings-Epics
- `Tokens.kt` für Android Compose → Phase 3 Android-Epic
- shadcn/ui init → Settings-Epic
- `KlarvoLogo` Component → Settings-Epic
- Tailwind v4 setup → Settings-Epic

## Acceptance Criteria

### AC-1: `design-tokens.toml` existiert mit vollständigen Farb-, Timing- und Radius-Werten

**Given** kein `design-tokens.toml` im Repo,
**when** die Story implementiert ist,
**then** existiert `design-tokens.toml` im Repo-Root mit exakt folgenden Werten (verbatim aus UX-Spec §Visual Design Foundation / Step 8):

```toml
# Klarvo Design Tokens — single source of truth
# Generator: cargo xtask gen-tokens
# Outputs: shells/windows/src/styles/tokens.css

[color.surface]
bg       = "#0F1715"   # Page background, deepest layer
surface  = "#161E1C"   # Card / panel surface
elevated = "#1F2926"   # Modal / floating surface
text     = "#E8EFEC"   # Primary text
muted    = "#7A8A85"   # Secondary text, hints
dim      = "#4A5851"   # Tertiary text, disabled state

[color.role]
action   = "#14B8A6"   # Teal — primary CTA, active nav, focus rings, waveform
activity = "#FBBF24"   # Amber — busy/processing/loading states
success  = "#34D399"   # Emerald — completion, "done", positive state
info     = "#60A5FA"   # Sky Blue — informational, links, hotkey-badges
warm     = "#FB923C"   # Orange — statistic highlights, accent contrast
danger   = "#EF4444"   # Red — destructive, errors, abort button

[color.overlay]
bg = "rgba(13, 15, 20, 0.92)"   # Pill Bar + Bubble background

[timing]
fast   = "150ms"   # Hover, focus ring, instant feedback
medium = "250ms"   # Component appear/disappear, Pill Bar fade
slow   = "400ms"   # Cross-screen transitions, Live-Preview grow

[radius]
sm   = "4px"
md   = "8px"
lg   = "12px"
pill = "9999px"   # Pill Bar bars, pill-shaped elements
```

**Wichtig — `color.overlay.bg`:** Der aktuelle Wert in `pill-bar.html` ist `rgba(13, 15, 20, 0.85)`. Die UX-Spec §C1 mandatiert `rgba(13, 15, 20, 0.92)`. Das Token übernimmt den UX-Spec-Wert (0.92). Dies ist eine bewusste visuelle Anpassung (+7% Opazität).

### AC-2: `xtask gen-tokens` generiert `tokens.css` korrekt

**Given** `design-tokens.toml` im Repo-Root,
**when** `cargo xtask gen-tokens` ausgeführt wird,
**then** wird `shells/windows/src/styles/tokens.css` mit folgenden CSS-Custom-Properties generiert (Naming-Convention: `--klarvo-<category>-<key>`):

```css
/* Auto-generated from design-tokens.toml — DO NOT EDIT MANUALLY */
/* Regenerate: cargo xtask gen-tokens */

:root {
  /* Surface */
  --klarvo-color-surface-bg: #0F1715;
  --klarvo-color-surface-surface: #161E1C;
  --klarvo-color-surface-elevated: #1F2926;
  --klarvo-color-surface-text: #E8EFEC;
  --klarvo-color-surface-muted: #7A8A85;
  --klarvo-color-surface-dim: #4A5851;

  /* Roles */
  --klarvo-color-action: #14B8A6;
  --klarvo-color-activity: #FBBF24;
  --klarvo-color-success: #34D399;
  --klarvo-color-info: #60A5FA;
  --klarvo-color-warm: #FB923C;
  --klarvo-color-danger: #EF4444;

  /* Overlay */
  --klarvo-color-overlay-bg: rgba(13, 15, 20, 0.92);

  /* Timing */
  --klarvo-timing-fast: 150ms;
  --klarvo-timing-medium: 250ms;
  --klarvo-timing-slow: 400ms;

  /* Radius */
  --klarvo-radius-sm: 4px;
  --klarvo-radius-md: 8px;
  --klarvo-radius-lg: 12px;
  --klarvo-radius-pill: 9999px;
}
```

**Exit-Code:** `gen-tokens` exits 0 on success, non-zero + descriptive error on any failure (missing file, parse error, write error).

**Naming-Konvention:**
- `[color.surface].*` → `--klarvo-color-surface-<key>`
- `[color.role].*` → `--klarvo-color-<key>` (kein `role`-Infix — die häufigsten Token, kürzest möglich)
- `[color.overlay].*` → `--klarvo-color-overlay-<key>`
- `[timing].*` → `--klarvo-timing-<key>`
- `[radius].*` → `--klarvo-radius-<key>`

### AC-3: `tokens.css` ist committed und liegt im richtigen Verzeichnis

**Given** der Generator wurde ausgeführt,
**then** liegt `shells/windows/src/styles/tokens.css` im Repo und ist tracked (nicht in `.gitignore`).

**Rationale:** Generierte CSS committed = kein Build-Schritt nötig für `cargo tauri dev`. Der Generator ist Pflicht vor jedem Token-Edit, aber kein CI-Blocker in dieser Story. (CI-Gate für CSS-Drift = separates deferred-work Item, nicht hier.)

### AC-4: `pill-bar.html` enthält keine hardcodierten Token-Werte mehr

**Given** `tokens.css` existiert,
**when** `pill-bar.html` refactored ist,
**then** gilt:

1. `pill-bar.html` hat `<link rel="stylesheet" href="styles/tokens.css">` im `<head>` (vor dem `<style>`-Block).
2. Folgende Werte sind ersetzt:

| Vorher (hardcoded) | Nachher (Token) |
|---|---|
| `rgba(13, 15, 20, 0.85)` (pill background) | `var(--klarvo-color-overlay-bg)` |
| `#14B8A6` (k-logo background) | `var(--klarvo-color-action)` |
| `#14B8A6` (bar background) | `var(--klarvo-color-action)` |
| `rgba(248, 113, 113, 0.9)` (abort-square bg) | `var(--klarvo-color-danger)` — volle Opazität, kein Alpha mehr (UX-Spec §C1: `var(--klarvo-color-danger)`) |
| `300ms ease-out` (fade-out transition) | `var(--klarvo-timing-medium) ease-out` (250ms) |
| `border-radius: 9999px` (bars) | `var(--klarvo-radius-pill)` |
| `border-radius: 24px` (pill container) | bleibt hardcoded — kein passender Radius-Token; 24px ist layout-spezifisch (= half of 48px height), nicht semantic |

3. `grep -E '#14B8A6|rgba\(13.*0\.85|rgba\(248' shells/windows/src/pill-bar.html` gibt keinen Match.

**Visueller Smoke-Test:** Nach dem Refactor zeigt die Pill Bar in `cargo tauri dev` optisch dasselbe wie vor dem Refactor — außer dass die Overlay-Opazität von 0.85 auf 0.92 angehoben wurde (bewusste Spec-Anpassung) und der Abort-Button von `rgba(248, 113, 113, 0.9)` auf `#EF4444` (voller Rot) wechselt. Beide Änderungen sind UX-Spec-konform.

### AC-5: `cargo xtask gen-tokens` im CI-Help-Text dokumentiert

**Given** `xtask/src/main.rs` `print_help()` Funktion,
**then** ist `gen-tokens` in der Subcommand-Liste aufgeführt mit kurzer Beschreibung.

### AC-6: `cargo check -p xtask` und `cargo check -p klarvo-windows-shell` grün

Keine neuen Compiler-Warnings durch diese Story.

## Technical Notes & Dev Guardrails

### xtask-Modulstruktur (FOLLOW EXACTLY)

Jedes xtask-Subcommand ist ein eigenes `mod`-File. Pattern aus bestehenden Modulen:

```
xtask/src/
  main.rs           ← Dispatch + print_help (MODIFY)
  gen_tokens.rs     ← NEW (analog zu verify_release.rs, lint_events.rs etc.)
  verify_release.rs ← Referenz für Modulstruktur
  ...
```

`main.rs` dispatch — **nach** `bindings-drift` einfügen (alphabetisch):

```rust
Some("gen-tokens") => reject_unexpected_flags("gen-tokens", &args[1..])
    .unwrap_or_else(gen_tokens::run),
```

Moduldeklaration in `main.rs`:
```rust
mod gen_tokens;
```

### `gen_tokens.rs` — Implementierungsanforderungen

```rust
//! `xtask gen-tokens` — CSS Custom Properties aus design-tokens.toml generieren.
//!
//! Input:  {repo-root}/design-tokens.toml
//! Output: shells/windows/src/styles/tokens.css
```

**Pflichtverhalten:**
- Repo-Root per `env!("CARGO_MANIFEST_DIR")` + `../` bestimmen (xtask liegt in `xtask/`)
- `design-tokens.toml` parsen via `toml`-Crate (bereits in `xtask/Cargo.toml`)
- Naming-Konvention aus AC-2 einhalten
- Output-Dir `shells/windows/src/styles/` anlegen falls nicht vorhanden (`std::fs::create_dir_all`)
- Header-Kommentar in generierter CSS (wie in AC-2 gezeigt)
- Bei Parse-Fehler: `eprintln!` + `ExitCode::FAILURE`

**TOML-Schema für den Parser:** Der Generator muss die Struktur kennen. Empfehlung: einfache `serde`-Structs:

```rust
#[derive(serde::Deserialize)]
struct Tokens {
    color: ColorTokens,
    timing: std::collections::BTreeMap<String, String>,
    radius: std::collections::BTreeMap<String, String>,
}

#[derive(serde::Deserialize)]
struct ColorTokens {
    surface: std::collections::BTreeMap<String, String>,
    role: std::collections::BTreeMap<String, String>,
    overlay: std::collections::BTreeMap<String, String>,
}
```

`BTreeMap` statt `HashMap` → deterministisch sortierte Ausgabe (kein Diff-Rauschen in git).

### `pill-bar.html` — Was NICHT geändert werden darf

Die folgenden Teile von `pill-bar.html` bleiben unverändert:

- Gesamte JS-Logik (`<script type="module">`) — kein Token-Bezug
- `cancel_recording` Tauri-invoke — kein Token-Bezug
- Event-Listener (`pill_bar.waveform_tick`, `pill_bar.show`, `pill_bar.fade_out`)
- `BIN_COUNT = 64`, `BAR_COUNT = 5`, bin-Mapping-Algorithmus
- HTML-Struktur (IDs, Klassen, ARIA-Attribute)
- `width: 320px; height: 48px` Layout-Dimensionen (kein Radius-Token, hardcoded = korrekt)
- `border-radius: 24px` auf `#pill` (layout-spezifisch, nicht semantic — bleibt)

### Verzeichnisstruktur nach Story

```
shells/windows/src/
  pill-bar.html          ← MODIFIED (token-refs, link-tag)
  styles/
    tokens.css           ← NEW (generated, committed)
  bindings/
    index.ts             ← unverändert
  index.html             ← unverändert (Settings-WebView — separates Epic)
design-tokens.toml       ← NEW (repo root)
xtask/src/
  gen_tokens.rs          ← NEW
  main.rs                ← MODIFIED (dispatch + help)
```

### Fade-Timing Diskrepanz — Bewusste Entscheidung

`pill-bar.html` hat `transition: opacity 300ms ease-out`. Der Rust-Backend `FADE_OUT_MS` in `PillBar`-Struct ist ebenfalls 300ms (Story 9.6). Die UX-Spec-Token `medium = 250ms` passen nicht exakt.

**Resolution:** Der `--klarvo-timing-medium`-Token (250ms) wird in `pill-bar.html` gesetzt. Der Rust-`FADE_OUT_MS`-Wert von 300ms ist Backend-Timeout (wie lange das Window offen bleibt nach Fade-Start) — unabhängig von der CSS-Transition-Dauer. CSS 250ms + Backend 300ms ergibt 50ms "stille" Phase nach Ende der CSS-Animation, bevor das Window verschwindet. Das ist akzeptabel (kein visueller Effekt, Window ist bereits opacity=0).

Kein Rust-Backend-Code-Change in dieser Story.

### `toml`-Crate ist bereits in `xtask/Cargo.toml`

Kein neuer Dependency nötig. Prüfen mit:
```
grep "toml" xtask/Cargo.toml
# → toml = { workspace = true }
```

### Windows Cross-Compile Check

`pill-bar.html` und `design-tokens.toml` sind keine Rust-Dateien — kein Cross-Compile-Check nötig. `xtask/src/gen_tokens.rs` ist reines Std-Rust (keine Windows-API), läuft auf Linux. `cargo check -p xtask` auf Linux ist ausreichend.

## Test Plan

1. **`cargo xtask gen-tokens`** → Exit 0, `shells/windows/src/styles/tokens.css` existiert
2. **CSS-Inhalt prüfen:** `grep --klarvo-color-action tokens.css` → `#14B8A6` vorhanden
3. **Keine Hardcodes in pill-bar.html:** `grep -E '#14B8A6|rgba\(13.*0\.85|rgba\(248' shells/windows/src/pill-bar.html` → kein Match
4. **`cargo check -p xtask`** → 0 Errors, 0 Warnings
5. **Visueller Smoke-Test (cargo tauri dev):** Pill Bar öffnet korrekt, K-Logo teal, Bars teal, Abort-Button rot, Fade funktioniert

## Dev Notes (Reihenfolge)

Empfohlene Implementierungsreihenfolge:

1. `design-tokens.toml` im Repo-Root erstellen (AC-1)
2. `shells/windows/src/styles/` Verzeichnis anlegen
3. `xtask/src/gen_tokens.rs` implementieren (AC-2)
4. `xtask/src/main.rs` updaten: `mod gen_tokens`, dispatch, help (AC-5)
5. `cargo xtask gen-tokens` ausführen → `tokens.css` landet
6. `pill-bar.html` refactoren: `<link>`-Tag + hardcoded → `var(--klarvo-*)` (AC-4)
7. `cargo check -p xtask` + visueller Smoke-Test (AC-6)
8. Alles committen (Token-Foundation als eigener Commit vor dem Pill-Bar-Refactor-Commit, oder als ein Commit — nach Präferenz)

## Commit-Konvention

```
feat(11.2): design-token foundation — design-tokens.toml + xtask gen-tokens + pill-bar.html refactor
```
