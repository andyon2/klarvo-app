# Epic 8 (Studio-Dark Desktop) — Conductor Handoff / Resume Doc

**Status: 6/6 Stories done auf Branch `conductor/epic-8`. NICHT nach `v1-ship` gemerged.**
Erstellt 2026-06-15 vom `bmad-epic-conductor`-Lauf. Dieses Dokument reist mit dem Branch — eine spätere
Session, die `conductor/epic-8` auscheckt, findet hier den vollen Stand.

> ⚠️ **Resume-Fallstrick:** `sprint-status.yaml` sagt `8-2…8-6 = done` — aber das gilt nur im
> Commit-Stand **dieses Branches**. Auf `v1-ship` steht es weiter auf `backlog`. Eine frische Session auf
> `v1-ship` würde Epic 8 für unerledigt halten. **Die Wahrheit: alle 5 Stories sind gebaut + committet
> hier, der Branch wartet auf Andys Smoke-Test + Merge.**

---

## Was gebaut wurde (alle auf `conductor/epic-8`, Basis `a607519`)

| Story | Commit | Inhalt |
|-------|--------|--------|
| 8-2 Settings | `cde5d3c` | K-Controls (Toggle/Select/Slider/Segmented), Portal-Dropdown, Focus-Ring-Split (Keyboard-Ring + Maus-Border) |
| 8-3 FloatingBar | `83a633d` | Glass-Pill, amber-Tally + teal-Waveform, Anti-Clip-Badge, Outer-Shadow entfernt (clipped) |
| 8-4 Live-Preview | `03b610d` | Geist-Mono-Transcript, Legibilitäts-Fix (Opacity restored), default-only Migration |
| 8-5 History | `f1125d2` | Mono-Timestamps, amber-Profile-Tags, RecordButton-Glows als `--glow-*` color-mix-Tokens |
| 8-6 Onboarding | `a07945a` | Geist, Glow-Tokens, DT1-Closure re-scoped + Residual in `docs/backlog.md` gehomed |

(8-1 Token-Foundation war vor diesem Lauf done.)

## Verifikationsstand — WICHTIG für Resume

**Mechanisch GREEN (von mir verifiziert, reproduzierbar):**
- `npm run build` (tsc + vite) grün nach jeder Story · `cargo check` host grün · 11 Preview-Tests grün (8-4).
- 8-3 + 8-4: echter WSL-Chromium-Harness (`/tmp/klarvo-bar-harness/8-3-smoke.mjs`, `8-4-smoke.mjs`,
  `8-4-smoke-http.mjs`) — Pixel-Farben (amber/teal/72%/96%), 200px-no-inflate, Geist/Geist-Mono
  resolven in-engine. ⚠️ Diese Harness-Scripts liegen in `/tmp` (flüchtig) — bei Bedarf neu schreiben,
  Muster siehe Story-Change-Logs + [[reference_wsl_chromium_bar_harness]].
- DT1-Hex-Gate clean auf den 5 Epic-8-Surfaces.

**NICHT maschinell verifizierbar → Andys Smoke (der eine Human-Gate, bewusst runtergestuft):**
Ästhetik, Backdrop-Blur über echtem Desktop-Content, Transparenz-Compositing, Spring-Enter-Feel,
WebView2- (vs Chromium-) Font-Resolution, Separate-Window-Reactivity, List-Density-Gefühl.

## Smoke-Test für Andi (kein Dev nötig) — Zustände erreichbar gemacht

Build: normaler `sync-and-build.ps1` (Repo ist auf `conductor/epic-8` ausgecheckt → baut Studio-Dark).

1. **Settings (8-2):** Tray → Settings öffnen. Durch die Sektionen klicken. Einen Toggle/Select ändern,
   Speichern, prüfen dass er bleibt (kein hängender Save-Button). → Controls „instrument-grade"?
2. **FloatingBar (8-3):** Eine Diktat-Aufnahme starten → Pill beobachten: Recording (amber Punkt +
   teal Waveform) → Transcribing (teal Spinner) → Done (grüner Haken). → Sieht premium/transparent aus?
3. **Live-Preview (8-4):** Settings → Appearance → Preview-Theme **„Dark"-Preset klicken** (wichtig: deine
   gespeicherte Config hat evtl. noch alte Farben — der Preset-Klick setzt Studio-Dark). Dann mit aktivem
   Live-Preview diktieren → Roh-Text in Geist Mono, gut lesbar?
4. **History (8-5):** Hauptfenster öffnen → History-Liste: Mono-Timestamps, amber Profil-Tags, Dichte/Hierarchie ok?
5. **Onboarding (8-6) — optional, braucht Config-Wipe:** Du bist schon onboarded, siehst es normal nicht.
   Wenn du's sehen willst: eine Session sichert deine `config.json`, entfernt sie, du startest die App
   (→ frisches Onboarding in Studio-Dark), schaust, schließt OHNE abzuschließen, Session stellt die Config
   zurück. (Mechanisch ist 8-6 verifiziert; nur wenn du's wirklich sehen willst.)

## Merge-Gate (Dev-frei)
**Smoke grün → Merge.** `v1-ship` ← `conductor/epic-8`. Andi muss nichts am Code beurteilen; eine Session
macht den Merge. Findet der Smoke ein Problem → siehe „Resume bei Problem" unten.

## Resume bei Problem (für eine spätere Session)
1. `git checkout conductor/epic-8`. Dieses Doc + die Story-Change-Logs (`_bmad-output/implementation-artifacts/8-*.md`) lesen.
2. Defekt **beobachtbar machen** bevor man Code ändert (Chromium-Harness / Logs — NIE blind rebuilden, [[feedback_observability_before_speculative_fixes]]).
3. Eine Ursache → eine Änderung → mechanisch re-verifizieren → auf den Branch committen.
4. Conductor-Lehre: der Auto-Fix-Loop landet bounded Patches nicht (decision-gate-preempt), Naht selbst ziehen — [[feedback_epic_conductor_decision_gate_preempts_fixes]].

## Offene Produkt-Entscheidungen (blockieren den Merge NICHT)
- **8-2:** Status-Dots (AC#1, descoped) · danger-hi in SPEC-Token-Tabelle · Radius-Sweep · Border-Opacity · KSegmented-Tint.
- **8-3:** echte Pill-Elevation (Outer-Shadow rendert im 200×36-Fenster nicht → braucht Window-Geometrie-Story).
- **8-4:** konditionale „bump-if-unchanged"-Preview-Migration für bestehende User (nicht autonom am ADR-0015-Config-Pfad gemacht).
- **8-6 / DT1:** `docs/backlog.md` → „Epic 8 — DT1 token-closure residual" (AdvancedSettingsPanel-Badge-Palette braucht Per-Kategorie-Farbentscheidung; MobileTextarea/ThemeSwitcher; Alias-Layer-Sweep).

## Nach dem Merge (Andys Entscheidung, vom Conductor bewusst NICHT gemacht)
`epic-8 → done` flippen + `bmad-retrospective` für Epic 8.
