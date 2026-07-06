# Design Anchor — Klarvo Visual Overhaul ("Studio Dark" + Android Bubble)

**This directory is the binding VISUAL source of truth for Epics 8 (Desktop) & 9 (Android).**
Downstream (BMAD spec/story writing, epic-conductor fidelity GATE-4) anchors here — never on a prose
transcription. The truth is the HTML render + the CSS values. Prose (incl. the README) is orientation only.

- **sourceFingerprint:** `bac152993046699c5007612ac916d951` (md5 over `Klarvo Design System.html` + `assets/klarvo.css`)
- **Promoted from:** `design-handoff/design_handoff_klarvo_overhaul/` (gitignored raw inbox)
- **Promoted on:** 2026-06-15
- **mode:** force (first ingest — no prior canon)

### In-repo extensions (provenance — ADR-0019 §5)

This canon is **ahead of the original raw handoff**: the surfaces below were added/changed in-repo by a
design decision, not by a new external download. A future re-ingest of the *old* raw inbox must NOT
clobber these — a re-handoff has to include them.

| Date | Fingerprint after | What | Decision |
|------|-------------------|------|----------|
| (orig) | `a3a5baff3ae56aa62270aa5a736972cb` | first external ingest | — |
| 2026-06-16 | `2bb990323f4fa224d6062b2c24965e37` | Added **`.ab-bubble.recording`** state (teal squircle + amber pulse-ring + send-glyph = "tap to send"); relabeled the listening-panel red square **Stop → Abbrechen**; tightened the `danger` token role to "Abbrechen / Löschen / Fehler (nie Senden)"; refined the bubble state-sequence prose. | [ADR-0019](../../../adr/0019-cross-platform-design-ssot.md) — codifies danger=Abbrechen + Android send = tap-the-bubble. |
| 2026-06-17 | `b95f86f9b480b92c3375093bc2580d9f` | **Modell B** (Andi-approved). recording ist kein Einzel-Bubble mehr: `.ab-bubble.recording` (+ `.send`, `@keyframes abbubblepulse`) **entfernt**, ersetzt durch Steuer-**`.ab-cluster`** (`.ab-cbtn.send` teal ➤ · `.hwave` amber, live · `.ab-cbtn.cancel` rot ✗) am Bubble-Platz = EIN Interaktions-Ort. `.ab-panel.rec` jetzt **passiv** (livedot/hwave/Stop entfernt → nur K·"Aufnahme"·Zeit·Text). `.ab-bubble.done` von teal → **Erfolgs-Grün + Haken**. Idle unverändert. | [ADR-0019 Amendment 2026-06-17](../../../adr/0019-cross-platform-design-ssot.md#amendment-2026-06-17--modell-b-android-aufnahme-cluster) — Cluster am Bubble-Platz, rot verlässt das Panel, Senden = ➤ (nicht Haken). |
| 2026-06-18 | `717d5d3879090a58db4d732f5c35208f` | added `--k-success-hi` token (9-5 Modell B done-green); `KlarvoTheme.kt` regenerated (`SuccessHi = 0xFF62E0A4`). | [Story 9-5 Task 1](../../../_bmad-output/implementation-artifacts/9-5-bubble-state-sequence-listening-panel-waveform.md) — AC9. |
| 2026-06-17 | `efe726c6afa3cc92aff981a2e476e14c` | **Modell B · transcribing-Variante B** (Andi-approved via `docs/design/overhaul/mockup-9-5-transcribing-done.html`). Neuer Surface **`.ab-bubble.proc`** (teal Squircle + `.spinner`) im transcribing-Artboard am Dock-Platz `bottom:228px`: nach ➤ Senden kollabiert der Cluster zu EINER teal Verarbeitungs-Bubble (Dock-Platz bleibt besetzt = Kontinuität). Panel-Spinner bleibt zusätzlich. done-Grün (G1) war bereits Canon — unverändert. | [ADR-0019 Amendment 2026-06-17 §4′-Addendum](../../../adr/0019-cross-platform-design-ssot.md#amendment-2026-06-17--modell-b-android-aufnahme-cluster) — transcribing = Dock-Proc-Bubble (Variante B). |
| 2026-06-21 | `fc9ef7456700d19b8332dd2c34a43b8e` | **§4′-Amendment · 9-5 GATE Follow-ups #2 + #4** (Andi-approved via `docs/design/overhaul/mockup-9-5-followups-2-4.html`). **(#2)** recording-Cluster-Reihenfolge **getauscht** → `[✗ cancel (links) · hwave · ➤ send (RECHTS)]`: ➤ Senden sitzt jetzt am Dock-/Daumen-Platz der idle-K-Bubble (zuvor `[➤ · hwave · ✗]`, ✗ am Daumen-Platz). **(#4)** neue **HOLD-Modus**-Surfaces `.ab-holddock`/`.ab-holdstrip`/`.ab-slidehint`/`.ab-heldbub`/`.ab-lockchip` + Artboard-Sektion „Aufnahme · HOLD-Modus" (aktiv: halten=aufnehmen · loslassen=senden · wegziehen=abbrechen · hoch ziehen=🔒 sperren → normaler Cluster). `.hwave`-Kommentar: RMS-getriebener Live-Cue (Anker für Follow-up #1, nicht idle-Animation). | [ADR-0019 §4′-Amendment 2026-06-21](../../../adr/0019-cross-platform-design-ssot.md#§4-amendment-2026-06-21--9-5-gate-follow-ups-2--4) — Cluster-Tausch (#2) + HOLD-Variante (#4). |
| 2026-06-26 | `bac152993046699c5007612ac916d951` | **Android-MOBILE recording-Steuerung SUPERSEDED → „B-Sprache"** (ADR-0019 Amendment 2026-06-26, Andi-approved nach Real-Device-Design-Failure von 9-14). Der `.ab-cluster`-Klein-Cluster + die HOLD-Surfaces `.ab-holddock/.ab-holdstrip/.ab-slidehint/.ab-heldbub/.ab-lockchip` sind für Android-MOBILE **überholt** (zu klein/„Laptop-Feel" am Gerät) — im tracked Canon als SUPERSEDED-Kommentar markiert (keine Geometrie gelöscht, nur Banner). **Neue bindende Render (eine Ebene über `source/`):** `mockup-mobile-hold-B-refined.html` + `mockup-mobile-recording-states.html` (große tappbare Senden/Abbrechen-Ziele · HOLD-Zwei-Zonen mit grow-on-target + release-to-commit · gesperrt = TAP-Surface · dock-adaptiv). Desktop unberührt. | [ADR-0019 Amendment 2026-06-26](../../../adr/0019-cross-platform-design-ssot.md#amendment-2026-06-26--android-aufnahme-steuerung-mobile-redesign-b-sprache) — Mobile-Redesign; Stories 9-14 (neu) + 9-15. |
| 2026-06-26 | `1bad4e27de1f915105caab15ded26d16` | **HOLD-Gesten-Hint-Animation reconciled** (Story 9-14 GATE-1, Andi-approved). Die Ingest-Transkription hatte `.ab-slidehint .arr` (‹ Abbrechen-Hint) + `.ab-lockchip .upi` (▲ Sperren-Hint) **statisch** übernommen; das abgesegnete Render `mockup-9-5-followups-2-4.html` **pulst** sie (Slide-to-Cancel / Drag-to-Lock-Cue). Canon nun angeglichen: `animation: slidearr/slideup 1.1s ease-in-out infinite` + die beiden `@keyframes` (translateX/Y ±4px, opacity .5↔1) aus dem Approval-Render übernommen. Keine Token-/Geometrie-Änderung. | [Story 9-14](../../../_bmad-output/implementation-artifacts/9-14-hold-mode-push-to-talk-cluster.md) — GATE-1-Design-Klärung (animiert + Lock-Footer), abgesegnetes Render = SOLL. |

## Truth

| File | Role | How to read it |
|------|------|----------------|
| `Klarvo Design System.html` | **render truth** | The rendered gestalt. Single document: Kritik · Direction · Tokens · Desktop surfaces (FloatingBar, Settings, Live-Preview, History) · Android (Bubble states, Listening-Panel, Long-Press menu, keyboard-collapse). File-local CSS (top `<style>`) overrides token sheet for the bubble mockups — read component geometry there. |
| `assets/klarvo.css` | **value truth** | Exact values. `--k-*` custom properties = the token set. Read colors/radii/shadows/motion here, never from prose. |
| `README.md` | **orientation only** | Map of the handoff. NOT a value source. Where it disagrees with HTML/CSS, the render wins (see Contradictions). |

**Render a surface for side-by-side (GATE-4):**
```
node ~/.claude/skills/design-handoff-ingest/render-surface.mjs \
  --html "docs/design/overhaul/source/Klarvo Design System.html" \
  --selector ".ab-bubble.idle" --out /tmp/soll-bubble-idle.png --scale 4
```

## Salient render values — Android Bubble idle (the 9-3 anchor)

Read from the HTML file-local CSS (`.ab-bubble`, `.ab-bubble.idle`) + `klarvo.css` tokens:

- **Shape:** `border-radius: 12px` on a `40×40` box → **rounded square / squircle**. NOT a circle.
  (Form is constant across all states — "KEIN Kreis↔Quadrat-Morph".)
- **Fill:** `linear-gradient(150deg, var(--k-teal-hi) #57DDC7, var(--k-teal-lo) #1B9C88)` → **teal gradient fill**.
- **Glyph:** `<span class="kt">K</span>`, `color: var(--k-on-teal)` `#05201B` → **dark "K" on the teal fill**, weight 700, ~17px.
- **Ring:** faint `0 0 0 3px rgba(41,199,172,.13)` teal ring (the "dezenter Glas-Ring"). On Android-native this is the ~3–4dp teal ring substitute for blur (no WebView backdrop-blur).
- **Size:** responsive `clamp(36dp, 0.11 × min(screenW,screenH)dp, 44dp)` (~40dp on 360–420dp phones); touch target `max(visual, 48dp)` via transparent padding.
- **Elevation:** `0 6px 18px rgba(0,0,0,.5)` + inset hairline.

## Contradictions found (prose-vs-render — render/CSS wins)

**C1 — Bubble idle shape: "circle" (prose) vs squircle (CSS). [root cause of the 9-3 drift]**
- Prose, HTML implementation-notes line ~885: *"Dictation-Bubble: SYSTEM_ALERT_WINDOW-Overlay, **solider Teal-Kreis** statt Glas (kein WebView-blur)."*
- CSS render, HTML file-local: `.ab-bubble { width:40px; height:40px; **border-radius:12px** }` → rounded square, **not a circle**.
- README itself affirms constant non-circular form: *"gleiche Form … KEIN Kreis↔Quadrat-Morph"* / *"Form konstant (kein Kreis↔Quadrat)"*.
- **Verdict: render/CSS wins → 12px-radius squircle.** The loose word "Kreis" in the feasibility prose is the drift.

**C2 — Bubble idle fill: "dark/solid" (built interpretation) vs teal gradient (CSS).**
- Drift built in 9-3 (commit `8c910aa`): **dark fill circle + teal ring** — teal pushed onto the ring, fill left dark.
- CSS render: `.ab-bubble.idle { background: linear-gradient(150deg, var(--k-teal-hi), var(--k-teal-lo)); color: var(--k-on-teal); }` → **teal-gradient FILL with a dark K**; the ring is only a faint `.13`-alpha accent.
- "solider … statt Glas" in the prose means *opaque instead of translucent-glass* (Android has no blur), **not** flat/dark single-colour. The gradient still applies.
- **Verdict: render/CSS wins → teal-gradient fill, dark "K", faint ring.** Likely also seeded by the old-IST screenshot `android-bubble*.png` (the pre-overhaul dark circle).

## Old-IST — NOT reference (quarantined, not promoted)

- `design-handoff/screenshots/*.png` (45 files) — none are referenced by the HTML; they are previous built-app states ("alter Ist"), not the design target.
  - **Especially `android-bubble.png`, `android-bubble-closeup.png`** — show the pre-overhaul **dark circle**; do NOT use as the comparison yardstick (the suspected seed of C1/C2).
- `design-handoff/current-code/*.tsx,*.css` — current source snapshots, not design truth.

## Removed as noise (during promote)

- `design_handoff_klarvo_overhaul/design_handoff_klarvo_overhaul/` — nested extraction copy, **byte-identical** (md5 `e8da6977…` HTML, `4e2ea538…` CSS) → dropped.
- `.thumbnail` — OS auto-thumbnail → dropped.

## Escalations

None — provenance unambiguous (single approved web-design-agent bundle; HTML self-declares "Single Source of Truth").
