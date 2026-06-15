# Epic 8 — Studio-Dark Fidelity-Gaps (Mockup vs. Gebaut)

**Warum dieses Doc:** Epic 8 hat die *Re-Skin-Schicht* sauber getroffen (Tokens/Farben/Fonts/Custom-
Controls — mechanisch verifiziert), aber bei der Human-Visual-Abnahme (Andi, 2026-06-15) zeigte sich:
das **„fertige" Gefühl des Design-Entwurfs hängt zusätzlich an Tiefe + Mikro-Verhalten + Badges/Dots**,
die der Story-Scope (NFR2 „reiner Re-Skin, kein Verhaltenswechsel") bewusst ausgelassen hat. Diese Datei
ist die durable Übergabe an eine **frische Session**, die den „Studio-Dark-Fidelity-Pass" ausführt.
(Geschrieben von einer kontext-schweren Session bewusst als Capture-before-loss, NICHT als fertiger Audit.)

## Ground Truth & Methode (wichtig — eine frische Session liest das ZUERST)
- **Ziel-Entwurf = das HTML-Mockup**, nicht die PNGs: `design-handoff/design_handoff_klarvo_overhaul/Klarvo Visual Redesign.html` (+ `assets/klarvo.css`). Es ist eine **Design-Direction-Seite** (Token-System + Rationale + gerenderte Beispiel-Surfaces), kein pixelgenauer Per-Screen-Entwurf.
- ⚠️ **Falle:** `design-handoff/screenshots/desktop-*.png` sind der **IST-Zustand der ALTEN App vom 13.06.** (Input an den Design-Agenten), NICHT der Entwurf. Nicht damit vergleichen.
- **Rendern:** das Mockup via Playwright/Chromium (`~/.cache/ms-playwright/chromium-1223/chrome-linux64/chrome`, `playwright` aus `~/.npm/_npx/e41f203b7505f1fb/node_modules/`) auf `file://…/Klarvo%20Visual%20Redesign.html`, full-page screenshots in ~1600px-Segmenten. Built-Surfaces gegen-rendern: React läuft im Browser via `npm run dev` (isPreviewMode mockt das Backend) — siehe [[reference_wsl_chromium_bar_harness]].
- Der Entwurf sagt selbst: **„Gleiche IA, echte Labels — nur sauber hierarchisiert und mit Tiefe."** Also KEIN Layout-Neubau; die Lücken sind Tiefe/Politur/Mikro-Motion, nicht Struktur.

## BESTÄTIGTE Gaps (von dieser Session gegen Code verifiziert)

### FloatingBar (8-3) — Andi visuell bestätigt „sieht nicht genau so aus"
Gegen die Constraint-Karten des Mockups (`src/FloatingBar.tsx`):
1. **Elevation-Schatten fehlt.** Entwurf: „Elevation — der größte Hebel", Pill = „weicher Schatten `0 8px 28px`". Gebaut: nur `inset 0 1px 0` Hairline — der Outer-Shadow wurde gedroppt (`FloatingBar.tsx:535`), weil er im randlosen **200×36-Fenster** abgeschnitten wird. **Fix braucht Rust-Window-Geometrie:** das Bar-Fenster ein paar px transparenten Rand geben (in `src-tauri/src/lib.rs` `create_bar_window` ~`629-664`: Größe + Region), Pill bleibt 200×36, Schatten kann in den Rand malen. Dann Outer-Shadow zurück.
2. **Amber-Tally = statischer Punkt statt pulsierendem Ring.** Entwurf: „pulsierender Ring = 'hört gerade zu'". Gebaut: statischer 8px-Dot (`FloatingBar.tsx:565-571`). Fix: pulsierende Ring-Animation (ähnlich dem Onboarding-`animate-ping`, aber amber, dezent).
3. **Stop-Button immer sichtbar statt Hover-only.** Entwurf: „Stop-Affordance (rot) erscheint bei Hover". Gebaut: `<StopButton>` unbedingt gerendert (`FloatingBar.tsx:572`). Fix: Stop nur bei `:hover`/Pointer-over einblenden. (⚠️ NFR2 sagte „kein Verhaltenswechsel" — der Entwurf WILL hier aber Verhalten; das ist der Scope-Konflikt, der den ganzen Gap erzeugt.)

### Settings-Home (8-2) — fehlende Badges/Status-Dots
Entwurf zeigt **farb-codierte Icon-Badges + Status-Dots** auf den Kategorie-Rows (grüner Dot = Provider konfiguriert; sichtbar in der AI-&-Providers-Fläche des Mockups). Story 8-2 hat die Dots NICHT gebaut (in der nächtlichen Adjudikation als „Semantik unklar" deferred). **Semantik ist im Mockup klar:** Dot = konfiguriert/aktiv. Fix: `StatusDot` (existiert in `ui.tsx`, ungenutzt) in `SettingsRow` einbauen, Status pro Kategorie ableiten (z.B. AI&Providers = mindestens ein Key gesetzt). Bereits im Backlog notiert (8-2 AC#1).

### Live-Preview (8-4) — Andi visuell bestätigt
Entwurf: „LIVE CLEANUP"-Panel (amber-Dot + Mode-Badge oben), großer Mono-Text, **am Boden verankert, baut sich auf**, Roh-Stream **gedämpft darunter**, Footer „Orientierung — kein exakter Output". ⚠️ **BY-DESIGN-VORBEHALT:** das Mockup zeigt *bereinigten* Live-Text, aber Live-LLM-Cleanup ist **bewusst AUS** (Quota — nur Roh-Stream live, SPEC). Erreichbares Ziel = die **visuelle Behandlung** auf den Roh-Stream: Header mit amber-Dot + Mode-Badge, Mono, bottom-anchored, ruhiges Expand, die Rahmung/Tiefe. NICHT Live-Cleanup bauen.

## NOCH ZU AUDITIEREN (Aufgabe der frischen Session — nicht von dieser Session geprüft)
- **History/Main-Window (8-5):** Mockup gegen gebaut (Karten-Dichte, Hierarchie, Such-/Filter-Affordanz, Mode/Profil-Tags) durchgehen.
- **Onboarding (8-6):** 6-Step-Flow gegen Mockup (Step-Indicators, Hero-Typo, BYOK-als-Feature). Erreichbar nur via Config-Wipe (siehe `epic-8-conductor-handoff.md`).
- **Settings-Sub-Pages:** Tiefe/Spacing/Elevation der Cards gegen das „Linear-Niveau" des Entwurfs.
- **Motion epic-weit:** Spring-Enter, Materialisieren, panel-Expand (Entwurf hat klare Motion-Tokens; gebaut nur teilweise).

## Empfohlener Weg (für die frische Session)
1. Mockup rendern + jede der 5 Surfaces built-vs-mockup gegenstellen → **Punchlist vervollständigen** (Absicht → Ist → Lücke → Fix-Skizze).
2. Als BMAD-Arbeit homen: entweder **Story 8-7 „Studio-Dark Fidelity-Pass"** (ein Bündel) oder per-Surface-Fixes. NFR2 muss neu gefasst werden: der Entwurf VERLANGT etwas Tiefe + Mikro-Verhalten, „reiner Re-Skin" war zu eng.
3. Bauen + **objektiv verifizieren** (Chromium-Harness für Pill/Preview-Pixel; Andi macht den finalen visuellen Abgleich gegen das Mockup — er ist der Maßstab, [[feedback_verify_surface_fix_with_objective_pixel_metric]]).
4. Conductor-Hinweis: der Auto-Fix-Loop landet bounded Patches nicht ([[feedback_epic_conductor_decision_gate_preempts_fixes]]) — bei diesem Fidelity-Pass eher manuell/gezielt arbeiten als blind den Loop füttern.

## Merge-Frage (Andis Entscheidung)
Die Re-Skin-Schicht ist eine **strikte Verbesserung** über die alte Optik + das Fundament für den Fidelity-Pass. Optionen: (A) jetzt `conductor/epic-8 → v1-ship` mergen, Fidelity-Pass als Follow-up drauf; (B) Merge halten, bis der Fidelity-Pass die Mockup-Treue erreicht. Empfehlung: **(A)** — der Stand ist besser als vorher, blockiert nichts, und der Fidelity-Pass baut sauber obendrauf. Gate bleibt Andis Smoke, kein Code-Review nötig.
