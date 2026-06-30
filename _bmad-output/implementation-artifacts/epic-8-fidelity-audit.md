# Epic 8 — Studio-Dark Fidelity-Audit (Soll vs. Ist, gerendert)

**Erstellt 2026-06-15. Methode: WSL-Chromium-Render von Mockup UND gebauter App, Side-by-Side.**
Dies ist der *vollständige, gerenderte* Audit, den `epic-8-fidelity-gaps.md` als „noch zu tun" markiert hatte.
Vergleichsbilder: `/tmp/klarvo-fidelity/sidebyside/{c-bar,c-settings,c-history,c-onboard}.png`
(flüchtig — Render-Scripts in `/tmp/klarvo-fidelity/render-*.mjs`, Muster wiederverwendbar).

## Ground Truth (3 Ebenen)
- **Soll (verbindlich):** `design-handoff/design_handoff_klarvo_overhaul/Klarvo Design System.html` + `assets/klarvo.css`.
  (Re-Export 2026-06-15: zwei alte Dateien → eine SSOT-HTML; Token-CSS **byte-identisch**, Desktop-Surfaces gerendert **pixel-identisch** zum alten Stand → Soll unverändert. Alter Stand unter `design-handoff/ARCHIVED - …`.)
  README sagt wörtlich: **„High-fidelity. Finale Farben, Typografie, Spacing, Radii, Elevation und Motion. Pixelgenau nachbauen."**
- **Ist:** gebaute React-App `npm run dev` (Preview-Mode, Backend gemockt), Fenster 480×720; FloatingBar via Temp-Harness (revertiert).
- ⚠️ NICHT `design-handoff/screenshots/*.png` — das ist der ALTE Ist-Zustand (Input an den Design-Agenten).

## Kernbefund — beantwortet „haben die Agenten das Design wirklich umgesetzt?"
**Ja — das Design-SYSTEM wurde pixelgenau übernommen. Nein — die ANWENDUNG von Tiefe/Motion/Status an einzelnen Call-Sites blieb aus, weil die Story-Regel NFR2 („reiner Re-Skin, kein Verhaltenswechsel") genau das verbot, was der Handoff verlangt.**

Token-Abgleich `klarvo.css` ↔ `src/styles.css` = **100% exakt**:
- Alle Farben (bg-deep…faint, teal/teal-hi/-lo/on-teal, amber/-hi, danger, success) — Hex identisch.
- Elevation-Schatten e1/e2/e3 **+ `--shadow-klarvo-pill: 0 8px 28px`** — definiert.
- Radii xs6/sm8/md12/lg16/xl20 — definiert. Motion 120/180/240/320 + Spring-Ease `cubic-bezier(.34,1.56,.64,1)` + reduced-motion — definiert.

→ Der Drift liegt **nicht** in den Werten. Er liegt in 3-4 nicht-angewandten Affordances.

## Bestätigte Gaps (gerendert + gegen Code verifiziert)

### FloatingBar (8-3) — Signature, die meisten Gaps
1. **▲ Pill-Elevation fehlt.** Token `--shadow-klarvo-pill` EXISTIERT, wird aber nicht benutzt — `FloatingBar.tsx:535` setzt nur inset-Hairline, weil der Outer-Shadow im randlosen **200×36-Fenster** geclippt wird. **Fix braucht Rust-Window-Geometrie** (`src-tauri/src/lib.rs` `create_bar_window`: ein paar px transparenten Rand), dann Token anwenden.
2. **▲ Amber-Tally statischer Punkt** (`FloatingBar.tsx:565-571`) statt pulsierendem Ring „hört zu". Fix: amber Ping-Animation.
3. **▲ Stop-Button immer sichtbar** (`FloatingBar.tsx:572`) statt Hover-only. ⚠️ NFR2-Konflikt: Mockup WILL hier Verhalten.
- ✓ Korrekt: Glas-Fill 72%, Blur 16px, teal Waveform, Mode-Badge, Geist.

### Settings-Home (8-2)
4. **▲ Keine Status-Dots auf Kategorie-Rows.** Mockup: grüner Dot = konfiguriert. `StatusDot` existiert in `ui.tsx` (ungenutzt). Status pro Kategorie ableiten.
- ✓ AI-&-Providers-Sub-Page HAT Status-Dots (grau ohne Keys → grün mit Keys), maskierte Keys, TRIAL-Badge, Presets, Custom-Controls.

### History (8-5) — starker Match
- ✓ Mono-Timestamps, amber Profil-Tags, teal Mode-Tags, Such+App-Filter, Dichte korrekt.
- ▲ klein: Datums-Format US (`6/15/2026, 11:57:56 AM`) vs. Mockup DE-kompakt (`13.6., 16:17`).

### Onboarding (8-6)
- ✓ Display-Typo „Sprich. Klarvo tippt.", Step-Indikatoren, Teal-Glow, Button — on-brand.
- ⚠️ **Kein Pixel-Vergleich möglich:** das Mockup hat KEINE Onboarding-Surface, nur Hero-Typo/Tokens spezifiziert. Nur Token-Treue beurteilbar.

## Noch nicht gegen-gerendert (klein, niedrige Wahrscheinlichkeit großer Lücken)
- **Live-Cleanup-Preview (8-4):** visuelle Behandlung (Header amber-Dot + Mode-Badge, Mono, bottom-anchored) auf den ROH-Stream — Mockup vorhanden, Ist-Render via PreviewPanel-Harness noch offen. (Live-Cleanup bleibt by-design AUS.)
- **Settings-Sub-Pages-Tiefe** (Card-Spacing/Elevation) und **Motion-Anwendung** (feuert Spring-Enter real?) — Token da, Anwendung stichprobenartig zu prüfen.

## Sizing-Urteil: KLEIN — eine Story, kein Epic
Die confirmed Gaps sind **4** (3 davon FloatingBar-Cluster) + 1-2 Mini + 2 Rest-Audits. Das Fundament ist exakt.
→ **Story 8-7 „Studio-Dark Fidelity-Pass"** (ein Bündel), NICHT ein Epic. NFR2 muss neu gefasst werden:
der Handoff verlangt Tiefe + Mikro-Verhalten; „reiner Re-Skin" war die Fehl-Constraint.
Bulk = FloatingBar (inkl. kleiner Rust-Window-Geometrie-Änderung für #1).
