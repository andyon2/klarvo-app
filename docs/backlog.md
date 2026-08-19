# klarvo — Backlog (deferred-work SSOT)

Single source of truth for deferred work. Every scope-cut / phase-deferral lands here with a source ref
(per backlog-discipline). Created 2026-06-10 by `bmad-correct-course` off the cross-platform drift audit.

---

## PROZESS-REGEL — Fixes zeitnah in die kanonische Linie (`v1-ship`) propagieren (2026-07-06, Epic-12-Retro)

Source: Epic-12-Retro (`epic-12-retro-2026-07-06.md`). **Wurzel der 2026-07-06-Overlay-Regression:**
Overlay-/Produkt-Fixes wurden auf dem gerade heißen Conductor-Branch gemacht und **nie konsequent in
eine kanonische Linie gezogen** → jeder Build aus einer Linie, die vor dem Fix abzweigt, verliert ihn
wieder (Epic 10 nativ-Overlays gingen auf `conductor/epic-12` verloren, weil der 16 Tage vorher
abzweigte).

**Regel (Andi 2026-07-06):** Ein abgeschlossenes Epic **nicht** auf seinem Conductor-Branch liegen
lassen — nach dem Epic-Abschluss (Retro done) **zeitnah in `v1-ship` mergen/fast-forwarden**, damit
`v1-ship` die einzige aktuelle Produktlinie bleibt und der nächste Bau-Branch von dort abzweigt.
`v1-ship` = kanonische Produktlinie (2026-07-06 bestätigt: `main` ist eine disjunkte v0.4.x-Release-
Historie, kein Merge-Ziel). Verwandt: [[project_webview2_overlay_backgrounding]] (Symptom-Klasse),
Branch-Hygiene beugt Wieder-Divergenz vor.

---

## Native Desktop-Overlays nach `main` nachziehen — Fix-Propagation (2026-07-06, Andi)

> **✅ GELÖST 2026-07-06** durch Integration `f53da68` (`conductor/epic-12` × `v1-ship`, in dieser
> Session gefahren): native Overlays + Epic 9/11/12 auf EINER Linie, `v1-ship` = Produkt. `main`
> bleibt disjunktes Archiv (Andis Call). Historie unten bleibt als Beleg.

Source: Andi meldete die Pillbar/Preview-Regression auf Windows wieder. Phase-0-Diagnose (Log `[fe:bar]`/
`[fe:preview]` = Frontend-Webview + kein `KlarvoPillNative`-Fenster + Branch-Topologie) ergab **eindeutig**:
kein neuer Bug — der laufende Build ist der **alte WebView2-Overlay-Stand von vor Epic 10**. `conductor/epic-12`
zweigte am **2026-06-14** von `v1-ship` ab, 16 Tage **bevor** der native Win32-Overlay-Fix (Epic 10, `678094c`,
30.06.) auf `v1-ship` landete. `v1-ship` **hat** den Fix; `main` und die aktiven Arbeitszweige **nicht**.

**Kernursache = Prozess, nicht Compositing:** Overlay-Fixes wurden historisch auf dem gerade heißen Branch
gemacht und nie konsequent nach `main` propagiert → jeder Build aus einer Linie, die vor dem Fix abzweigt,
verliert ihn wieder. Ein WebView2-Flag-Quick-Fix auf epic-12 wäre die 6. Iteration an der falschen Schicht.

**Entscheidung (Andi 2026-07-06):** Epic 12 abgeschlossen — **12-1 + 12-2 DONE + device-verified beide Plattformen**
(12-3 deferred). **NÄCHSTER SCHRITT = diese Integration**, in **frischer Session** aufzusetzen.

### ⏭️ PHASE 2 — Integration `conductor/epic-12` × `v1-ship` (NÄCHSTE SESSION, entscheidungs-komplett)

Ziel: EINE Linie, die BEIDES trägt — die nativen Desktop-Overlays (Pill-Fix, Epic 10, auf `v1-ship`) UND
Epic 9/11/12 (auf `conductor/epic-12`). **KEIN trivialer Merge** — Trial-Merge (`git merge-tree v1-ship
conductor/epic-12`, 2026-07-06) zeigt: die beiden Linien haben **dieselben Overlay-Surfaces unterschiedlich**
implementiert (Epic 10 ersetzte die WebView2-Pille/Preview durch native `native_pill.rs`/`native_preview.rs`;
Epic 12 änderte die alte WebView2-`FloatingBar.tsx`). Divergenz: 174 Commits (epic-12) vs. 60 (v1-ship).

**Konfliktfläche + Auflösungs-Plan:**
- **Mechanisch (Union/beide-behalten):** `sprint-status.yaml`, `backlog.md`, `deferred-work.md`,
  `bmad-sprint-status.toml`, `docs/adr/README.md`.
- **Echte Code-Konflikte:** `pipeline.rs`, `lib.rs`, `commands/recording.rs` — Epic-12-Logik (Fallback-Leiter +
  `save_pending_wav` + Audio-Retry) auf die native-Overlay-Basis (recreate-per-recording, In-Process-State)
  zusammenführen. Beide Seiten behalten, nicht eine gewinnen lassen.
- **⚠️ SEMANTISCHER KNACKPUNKT / Surface-Nachbau:** `src/FloatingBar.tsx` ist **auf v1-ship gelöscht**
  (native Pille), **auf epic-12 geändert** (12-1 FR4 Pill-Status-Signal „⚠ DeepSeek → OpenAI" etc.). Die native
  Pille (Epic 10, 30.06.) hat den Epic-12-Status-Signal **gar nicht**. → `FloatingBar.tsx` bleibt gelöscht, und
  das **Pill-Status-Signal (12-1 FR4) muss in `native_pill.rs` neu portiert** werden. Das ist ein SURFACE-Feature
  → **Andis Geräte-Gate** (Windows). `12-2` selbst ist unkritisch (Hauptfenster-History, nicht im Konflikt).

**Vorgehen (frische Session):** Integrations-Branch off `v1-ship` → `conductor/epic-12` mergen → obige Konflikte
lösen + Pill-Status nach `native_pill.rs` portieren → **Build-Verify BEIDE Plattformen** (`sync-and-build.ps1` +
`android-smoke.sh`) → **Andis Geräte-Gate**: (a) native Pille bleibt bei Verdeckung sichtbar (Occlusion-Repro),
(b) 12-1 Fallback-Status erscheint auf der nativen Pille, (c) 12-2 Pending/Re-process weiter grün. Dann wird das
Ergebnis `v1-ship`; Andi baut daraus; optional `v1-ship`→`main`. Groß genug für eine eigene Story / `bmad-quick-dev`
bzw. Conductor-Runde mit Build-Verify — **nicht** als Anhängsel durchwinken.

Voll-Kontext: Memory `project_webview2_overlay_backgrounding`.

---

## Mobile-Overlay-Design durchgefallen — Real-Device-Verdikt (2026-06-26, Andi)

Source: Andis erster echter Daumen-Test des HOLD-Modus (Story 9-14) am echten Gerät. GATE-4 Maschinen-Ebene
war grün, **Design fiel durch**. 9-14 → `in-progress`, Rebuild blockiert auf einen Mobile-Overlay-Design-Rethink.
Voll-Kontext: Memory `project_mobile_overlay_design_rejected`.

**Systemisch (nicht nur 9-14):** Das gesamte mobile **Overlay** (Bubble + alle Recording-States) **und die
Live-Preview** wirken „wie ein Laptop-Feature, nicht wie ein echtes Handy-Feature" — zu klein, Qualität
ungenügend („andere Apps kriegen das besser hin"). In-App-Settings-UI ist okay; nur die Overlay-Surfaces.
Wurzel-Hypothese: Canon als Browser-Mockups approved → nie im Daumen-/Geräte-Maßstab → Touch-Target-Größe,
Finger-Occlusion, echter Hintergrund nie im Approval-Loop. → SOLL-Approval für mobile Overlays muss im
Geräte-Maßstab passieren.

**Konkrete Items (9-14, gelten als Muster fürs neue mobile Design):**
1. Drag-links-Abbrechen: Farb-**Gradient amber→rot** während des Ziehens.
2. Auslöser = **Loslassen im gezogenen Zustand** (nicht Schwellwert-während-Bewegung); **Undo** möglich
   (zurückziehen vor dem Loslassen = kein Auslösen). Gilt für Abbrechen UND Hoch-Sperren.
3. Klare **visuelle Drag-Cues** (links + hoch).
4. Dock **deutlich größer + weiter auseinander** — Finger verdeckt aktuell zu viel; gezogener Zustand muss
   sichtbar bleiben.
5. Lock-Affordance braucht **soliden Hintergrund** (aktuell transparent → über wuseligem Wallpaper unlesbar).
6. **Live-Preview** ebenfalls mobil neu denken (gleicher „Laptop-Feel").

**Design-Inputs (Andi, 2026-06-26 — Phase A, Vorgehen „Prinzipien + Referenzen zuerst"):**
- **Referenz = Wispr Flow** (mobiles Diktat-Overlay). DNA aus Recherche: Floating-Bubble über der Tastatur;
  Tap → expandiert zu [✗ · Live-Waveform · ✓]; **Halten = Push-to-Talk**; ruhige, premium-minimalistische
  Optik mit fließender Live-Waveform als „ich höre"-Signal. Pixel-Optik nicht agent-sichtbar → Treue = Andis Auge.
- **Dock-Positionen dynamisch** für alle wichtigen Plätze: rechts-mittig (aktuell), links-angedockt-mittig,
  oben, unten, frei. Das Overlay + die Drag-Cues müssen **position-adaptiv** sein.
- **Rechtshänder, meist rechter Daumen** → Occlusion-Regel: aktive Cues wachsen WEG vom Daumen / von der
  angedockten Kante ins Display hinein.
- Approval-Surface = **Geräte-Maßstab** (Redmi 1080×2460 px @ 440dpi / Faktor 2.75), über belebtem Wallpaper —
  nie Laptop-Browser-Mockup.
- Scope: Recording-/HOLD-Overlay als Leit-Surface zuerst, dann übrige Bubble-States + Preview.

**Stand 2026-06-26 — Richtung gewählt:** Aus 3 device-scale-Mockup-Richtungen (A Slide-Spur · B Zwei-Zonen ·
C Ruhe-Sheet) hat Andi **B (verfeinert)** gewählt: held bubble = Daumen-Anker rechts; zwei große runde Ziele
(Sperren teal oben-links · Abbrechen rot unten-links), aufgeräumt/nicht-überlappend, Waveform-Chip an der
Bubble; **Ziel wächst + leuchtet, sobald der Finger drauf ist**, **Loslassen löst aus** (release-to-commit + Undo).
Mockups: `docs/design/overhaul/mockup-mobile-hold-explore.html` (A/B/C) + `mockup-mobile-hold-B-refined.html`
(Ruhe + Treffer), gerendert via Playwright @ 1080×2460. NÄCHSTE: volle HOLD-Flow-Zustandsfolge + Dock-Varianten
in B-Sprache → neuer Canon → 9-14-Rebuild. OFFEN: Scope (nur HOLD vs. auch Tap-Cluster/9-13 vs. ganzes Overlay+Preview)
+ Post-Lock-Zustand (gesperrt = große tappbare Ziele statt altem Klein-Cluster?).

---

## Cross-Platform Drift — Sorte-2 (deferred, ADR-0016 Amendment 1 = accepted asymmetry → backlog, NOT hard won't-fix)

Source: `docs/cross-platform-drift-audit.md` · routed by `sprint-change-proposal-2026-06-10.md`.
These are pure feature-ports / marginal asymmetries — the ADR-0016 ROI rationale ("don't deepen the
~2000-LOC duplicate for marginal benefit; v2 is the dedup answer") still holds. Listed so they are not
re-filed as bugs and not lost. The fixed (Sorte-1) rows live in Epic 7 (`epics-cross-platform-parity.md`).

| ID | Divergence | ADR-0016 | Class |
|----|------------|----------|-------|
| C2 | Whisper-mode amplify (3 keys + audio gain) — no-op on Android | — | feature port |
| H4 | `outputLanguage` translation — Android never translates | DIV-07 | feature |
| H5 + Recall #2 | Anthropic provider absent on Android (struct field + branch); `llmProvider=anthropic` → silent DeepSeek | — | feature port |
| H8 | `audioDevice` selection ignored (always MIC) | — | feature port |
| H11 | Local cleanup system-prompt abbreviated on Android | DIV-09 | quality |
| H12 + M7 | No provider fallback on 429/5xx; different fallback order | DIV-06 | resilience |
| H15 | Per-app profiles ignored on Android | — | feature port |
| H16 | `sttProvider=openai` silently hits Groq / refuses | DIV-13 | feature |
| M5 | Full 4-state VAD state-machine parity (heavy algo rewrite) | DIV-14 | algorithm |
| M6 | Pre-STT WAV float support (latent; both PCM16 today) | — | latent |
| M14 + Recall #3 | Webhook delivery + `webhookHeaders` never read on Android | — | feature port |
| M15 | Desktop adopt STT retry/backoff (Android is the better side) | — | Desktop enhancement |
| L2 | Waveform amplitude transform differs | — | cosmetic |
| L6 | Empty-transcript toast text differs (same outcome) | — | cosmetic |
| L7 | Voice command (`voiceCommandEnabled`) unimplemented on Android | DIV-10 | feature |
| Recall #4 | Live-preview delta (`PreviewFlushConfig`, Epic 5) — no Android counterpart | — | feature port |
| Dead-config cluster | `advanced.llmTemperature`/`llmMaxTokens`, `chunkThreshold`/`chunkTargetSize`, `sttTemperature`, `llmModel*`/`llmSystemPrompt*` overrides, `autoCapitalize`/`autoPaste` — settable, consumed nowhere | — | latent landmine |

> The dead-config cluster's *current* state (both sides hardcode) is **locked by Epic 7 Story 7.7**
> golden-vectors. The *wire-when-needed* implementations (actually honoring these keys on a platform)
> stay backlog until a key is genuinely needed.

### OPEN-DECISION — M12 (dictionary-in-Chat-style)

**Source:** drift audit M12. Android adds the dictionary for ALL cleanup styles incl. Chat
(`KApi:591-593,749`); Desktop omits `{dict_section}` in the Chat arm (`llm/mod.rs ~232-263`). **Opposite
direction.** Canonical direction is a product call. To be resolved in **Epic 7 Story 7.6**; until then both
sides' current behavior is golden-vector-locked, not changed.

---

## STT re-scope 2026-06-12 — deliberate deferrals + tooling

Source: `sprint-change-proposal-2026-06-12.md` + ADR-0017 (shared-core STT path).

- **ADR-0017 consolidation extension — VAD gate:** moving the live auto-stop VAD gate into the
  Rust core over JNI (realtime frame stream, ~31 Hz). Deliberately deferred — large lift,
  speech-truncation risk; the felt problem (text hallucination) doesn't touch it. Story 7.2 narrows
  the gap in Kotlin instead.
- **ADR-0017 consolidation extension — chunking/LLM path:** extending the "shared logic only in
  Rust" Hard Rule beyond STT (chunking, LLM routing). Deferred; 7.1/7.5 fix per-row, 7.7 pins
  against re-drift. Candidate for a future decision (or v2).
- **TOOLING — `scripts/dictation-quality-audit.py`:** commit the `raw_text` hallucination-marker
  detectors from `docs/dictation-quality-android-vs-desktop-2026-06-12.md` as a repo tool; run
  manually on a cadence (monthly / pre-release — needs the phone over adb/Tailscale, no cloud cron).
  Quality-layer sibling of the 7.7 parity net. Attached to Story 7.7.

---

## Other deferred work

### License (from B1, `sprint-change-proposal` predecessor work)
- `ls_client::validate()` is dead code (no prod caller) — periodic re-validation never wired; only the
  `activate()` gate is live. Source: `project_v1_feature_roadmap` memory, `deferred-work.md`.
- Live-key acceptance in a release build is unverified (needs a real purchase) — close at real launch.

## Epic 10 — Native Desktop Overlays — Story 10-1 residuals (deferred)

Source: `10-1-native-pill-overlay.md` Change Log + `gate4-evidence/10-1/verdict.md`. Story 10-1 DONE
(Andi gate passed 2026-06-27); these are accepted residuals, not blockers.

- **Visual polish (low):** 1px state-colored stadium border not drawn on the native pill (SOLL has a
  subtle per-state border ring); the `done(clipboard-only)` state renders a simplified amber clipboard
  square instead of the 📋 glyph. Both below Andi's smoke threshold.
- **Show/hide animations:** SOLL bar-expand (220ms) / collapse (180ms) / done-pop (280ms) easing not
  reproduced in the native pill (state changes are instant). Cosmetic; revisit only if it grates.
- **Robustness (low):** no per-monitor DPI handling (`WM_DPICHANGED`); off-screen drag not clamped to a
  visible monitor; window class registered per-create (not once); `save_config_locked` lock-poison path
  vs not-alive not distinguished; recreate-mid-recording doesn't replay current state.
- **Harness rigor:** `desktop-occlusion-proof.ps1` asserts content pixels `> 0` rather than AC-5's
  "≈100% of region"; one dead `$EvidenceDir` path typo to clean up.

> Older per-feature deferrals tracked in `_bmad-output/implementation-artifacts/deferred-work.md` remain
> valid; migrate them here opportunistically.

## Epic 10 — Native pill interaction + visual fidelity vs WebView2 (OWN STORY — Andi 2026-06-27)

Source: Andi observation 2026-06-27 (during Story 10-3 work). **Andi's words:** the red cancel/stop
button no longer animates on mouse-over — there is *no hover feedback at all*; it did this normally
before. More broadly: "the pill doesn't behave and look exactly like it did before" since the native
rewrite. **Andi wants this as a dedicated story** (not folded into 10-3, which is the standby-resilience
lifecycle fix).

**Concrete reported gap — stop/cancel button hover feedback (the trigger):**
- The native pill is a static `UpdateLayeredWindow` raster. `native_pill.rs` only does a *click*
  hit-test on the stop region (`WM_LBUTTONDOWN`); there is **no `WM_MOUSEMOVE` hover tracking, no
  `TrackMouseEvent` for mouse-leave, and no re-render on hover**, so the WebView2 FloatingBar's CSS
  `:hover` affordance (grow/highlight on the red cancel target) is gone entirely.
- Fix direction (for the story, not now): track hover state in `PillWindowState`, add `WM_MOUSEMOVE` +
  `TrackMouseEvent(TME_LEAVE)` → `WM_MOUSELEAVE`, re-render the stop button with a hover style
  (scale/opacity/ring) matching the old SOLL, and set the hand cursor over the hit region.

**Broader scope — a native-pill ↔ WebView2 fidelity pass.** This story is the natural home to also
reconcile the already-logged Story 10-1 visual residuals (above): the per-state 1px stadium border,
the `done(clipboard-only)` 📋 glyph, and the show/hide/expand/collapse/done-pop easing animations
(state changes are currently instant). Together these are the "not 1:1 like before" drift Andi is
reacting to. Suggested first step: a structured fidelity audit (each FloatingBar.tsx interaction +
transition vs the native pill) to size the story before building.

Scope note: this is appearance/interaction fidelity, NOT the Epic 8 Studio-Dark re-skin (still parked).

## Epic 10 — Blur slider becomes a no-op for the native preview (Story 10-2 GATE-1 defer, 2026-06-27)

Source: Story 10-2 GATE-1 decision (conductor + Andi). ADR-0021 VR3 drops backdrop blur from the
native preview, but the Settings UI still shows the `previewBgBlur` slider. **Decision: leave the
slider in place (silent no-op) for now** — removing it would pull 10-2 into the WebView2 `main`
Settings surface (scope creep), and VR3 notes real blur could return as a follow-up, in which case
the slider regains meaning. If the no-op slider proves confusing, a small follow-up either (a) hides
it while blur is dropped, or (b) re-implements blur natively. Not a 10-2 blocker.

## Epic 10 — Native-Overlay-Skalierung zu klein + Appearance-Wiring-Audit (EIGENE STORY — Andi 2026-06-28)

Source: Andi Real-Device-Smoke nach Story 10-2 (native Preview). **Andis Befund:** Seit dem nativen
Umbau (Epic 10: Pille 10-1/10-3 + Preview 10-2) ist **alles kleiner** als bei den alten WebView2-
Overlays — **beide** Overlays: Preview-Karte UND Pille (Logo + roter Abbrechen-Button zu klein). Die
**Schriftgrößen-Settings wirken viel schwächer**: „large" schon mega klein, „medium" viel zu klein,
„small" ultra klein. Niemand hat die absolute Skalierung beim nativen Umbau geprüft. **Andi will das
NICHT als Quick-Fix, sondern als eigene, harte Story** — inkl. der Frage: ist die Settings-Appearance-
Rubrik überhaupt korrekt mit der Preview verschaltet?

**Read-only-Diagnose (2026-06-28, Conductor — NICHT gefixt):**
- **Settings→Preview-Wiring ist korrekt verschaltet.** `previewFontSize` → `font_px` Mapping ist
  identisch zum alten `PreviewPanel.tsx`: small=11 / medium=13 / large=15 (`native_preview.rs:94-97`).
  Die Settings erreichen den Renderer. Das ist NICHT der Defekt.
- **Leitende Hypothese (NICHT auf Windows verifiziert): falsche DPI-Skala — betrifft BEIDE Overlays
  über denselben Mechanismus.** Pille (`native_pill.rs:1259`) und Preview (`native_preview.rs:942`)
  berechnen `scale = GetDeviceCaps(screen_dc, LOGPIXELSX) / 96`. Unter Per-Monitor-DPI-Awareness
  (tao/Tauri setzt per-monitor-v2 im embedded Manifest) liefert `GetDeviceCaps(screen_dc, LOGPIXELSX)`
  typisch **96** zurück (nicht die echte Monitor-DPI) → `scale = 1.0`, egal ob der Monitor auf 125/150 %
  steht. Die alten WebView2-Fenster waren via Tauri korrekt DPI-skaliert (rendern bei echten 150 %).
  Ergebnis: native Overlays rendern ~1.0× statt ~1.5× → **uniform zu klein, Pille + Preview**, und die
  11/13/15-Stufen wirken bei 1.0× klein + ihre Abstände gestaucht („Settings wirken schwächer").
  Korrekte API wäre `GetDpiForWindow(hwnd)` / `GetDpiForMonitor` statt `GetDeviceCaps(screen_dc)`.
  **Zu verifizieren in der Story** (z. B. echte scale am Gerät loggen).

**Story-Scope (Vorschlag):** (1) Root-Cause der absoluten Skala bestätigen (echte DPI am Gerät
loggen — `GetDeviceCaps` vs `GetDpiForWindow`); (2) Skala für BEIDE nativen Overlays korrigieren, sodass
sie 1:1 zur alten WebView2-Größe rendern (Andi-Referenz: „vorher war alles größer"); (3) Appearance-
Settings end-to-end gegen die Preview prüfen (alle `previewXxx` + Größen-/Breiten-Presets sichtbar
wirksam); (4) Größen-Presets so kalibrieren, dass small/medium/large spürbar unterschiedlich sind.
Querbezug: die bereits gelistete „Native-Pille-Hover/Fidelity"-Story oben — Skalierung kann dort
mit reinspielen (gemeinsamer Pass erwägen). Maßstabs-Referenz = alte WebView2-Optik (git vor Epic 10).

### Native-Overlay Multi-Monitor Mixed-DPI (deferred aus 10-4 Code-Review 2026-06-28)

Source: 10-4 Code-Review (Edge-Case + Blind Hunter). Auf Andis Single-Monitor-Setup **gegenstandslos**
→ bewusst deferred, eigene Story falls jemals Multi-Monitor-Mixed-DPI relevant wird. Zwei Funde, gleiche
Wurzel = logical-vs-physical-Koordinaten-Verwechslung über Monitor-Grenzen:
- **F2 — Monitor-Auswahl nutzt logische Koordinaten, wo `MonitorFromPoint` physische erwartet**
  (`native_pill.rs` candidate_pt, `native_preview.rs` candidate_pt). Nahe einer Monitor-Grenze kann der
  falsche Monitor (und damit die falsche Skala) gewählt werden. Single-Monitor: irrelevant.
- **F3 — `SPI_GETWORKAREA` liefert nur den PRIMÄR-Monitor, gemischt mit der Skala des Sekundär-Monitors**
  (`native_preview.rs` work_area → `compute_preview_geometry`-Clamp). Preview kann auf einem Sekundär-
  Monitor falsch clampen. Teilweise vorbestehend (work_area war immer primär-only). Single-Monitor: irrelevant.
- F1 (stale saved-coordinate re-scaling) wurde in 10-4 via Work-Area-Clamp in `compute_initial_pos`
  gefixt (`795d5b3`) — schließt zugleich den oben gelisteten Robustness-Fund "off-screen drag not clamped".

## Epic 8-Abhängigkeit — native Overlays beim Studio-Dark-Reskin in Rust/GDI nachziehen (aus Epic-10-Retro 2026-06-30)

Source: Epic-10-Retrospektive (`epic-10-retro-2026-06-30.md`) · ADR-0021 (native Win32-Overlays).
**Kreuzungs-Abhängigkeit, kein Defekt.** Pille (`native_pill.rs`) und Preview (`native_preview.rs`) sind
**bewusst an den AKTUELLEN Render verankert, NICHT an Epic 8 Studio-Dark** (so in jeder 10-x-Story
festgeschrieben: „This is a technology migration, anchored to the CURRENT render. Do not apply Epic 8
Studio-Dark."). Folge: Wenn der **Desktop-Visual-Overhaul (Epic 8)** drankommt, erfasst der CSS-Reskin
diese beiden Overlays **nicht** — sie zeichnen in tiny-skia/GDI, nicht im WebView. Die Studio-Dark-Tokens
(Farben, Radien, Typo, Border) müssen für Pille + Preview **von Hand in den Rust-Renderer portiert**
werden, sonst bleiben sie nach einem Epic-8-Reskin optisch zurück (alte Farben/Border auf neuem Rest).
Beim Epic-8-Desktop-Start zu berücksichtigen — nicht „der Reskin macht das automatisch". (Story erst
schreiben, wenn Epic 8 Desktop live ist; jetzt würde sie verrotten.)

---

## Conductor verification — GATE-4 structural-assertion smoke (Android overlay)

Source: spike 2026-06-16 (memory `reference_android_emulator_window_structure_oracle`); postmortem
`docs/postmortem-2026-06-15-epic-conductor.md`; `project-context.md` Testing Rules (the "emulator =
structural-not-pixel oracle" rule). The conductor skills (`bmad-epic-conductor` / `bmad-story-conductor`,
E9 + GATE-4) now **assume** this assertion exists — this story makes it real.

- **STORY — harden `scripts/android-smoke.sh` to assert overlay-window STRUCTURE at GATE-4.** Today the
  unattended smoke drives to a state and screencaps; the emulator's *pixels* are an unreliable oracle
  (software GPU, cold-boot SystemUI ANR, `mForceHideNonSystemOverlayWindow`, no HyperOS skin) — that is
  why 9-5 went "emulator-green" while the real Xiaomi was broken. But the emulator reports overlay-window
  **structure** faithfully: `adb shell dumpsys window windows` exposes each `ty=APPLICATION_OVERLAY`
  window's requested size / gravity / visibility. Add a step that, per driven state, asserts the expected
  window structure (e.g. recording = exactly 1 panel ~1080×525 bottom + 1 idle bubble 162×162) and FAILS
  the smoke on a mismatch; emit `structure-<state>.txt` into the GATE-4 evidence dir. This is the machine
  oracle that would have caught 9-5 unattended; the residual (HyperOS pixel aesthetics) stays Andi's
  real-device gate. **AC:** a deliberately re-introduced 9-5-class regression (bubble in recording form
  instead of idle squircle) makes the structural assertion RED on the emulator.

- **RELATED-BUT-SEPARATE — real-device state-driving harness (already open).** `DEBUG_SET_STATE` broadcast
  is dead on HyperOS (background restrictions block the manifest receiver) → Andi's real-device morning
  gate can't be scripted, blocks 9-6/9-8. Replace with a HyperOS-survivable trigger. Tracked in the
  postmortem; listed here so it is not conflated with the structural-assertion story above.

## Story 9-6 (keyboard-collapse via a11y service) — PARKED (obsolete)

Source: Andi's decision 2026-06-16 (this session). 9-6 was scoped to collapse the soft keyboard via the
accessibility service before showing recording UI. With the new pop-up **preview window**, the preview
simply lays *over* the keyboard — so the a11y keyboard-collapse mechanism is no longer needed. Parked,
not won't-fixed: revisit only if the overlay-over-keyboard approach turns out insufficient on a real
device. Status stays `backlog` in sprint-status (no formal "parked" state); the routing hook steers the
cascade away from it. **9-8 is NOT parked** — the long-press popover still needs verifying and stays
blocked on the real-device state-driving harness (DEBUG_SET_STATE dead on HyperOS, above).

## Story 9-7 follow-up — make Android silence-selection swap-safe (call-site wiring)

Source: Story 9-7 code-review (story-conductor, 2026-06-16), one Medium finding accepted as residual by
Andi at GATE 3. The AC6 regression test locks the *pure* `RecordingMode.selectSilenceSecs()` mapping,
but the **call site** in `KlarvoOverlayService.startRecording()` passes four same-typed `Float`
arguments (`tapSilenceSecs` / `longPressSilenceSecs` / `autostopSilenceSecs` / `autoModeSilenceSecs`).
A swap of two of those at the call site would regress production silence-selection while every JVM test
stays GREEN — and because the production defaults are all `2.0f`, such a swap is value-invisible until a
user sets the fields to different values. NOTE: this is NOT the original silence-field divergence
(wrong-field-*read*), which 9-7 DID lock; it is a new, low-probability surface introduced by the
testability extraction itself.

- **STORY — close the call-site gap.** Either (a) give the four silence durations distinct value-class
  types so a call-site swap fails to **compile**, or (b) add a test that exercises the real
  `startRecording()` field→param wiring (heavier — needs Android-context test infra). Option (a) is the
  by-construction fix and is preferred. **AC:** a deliberately swapped call-site argument fails to
  compile (a) or turns a test RED (b).

## Epic 9 — on-device defects found 2026-06-16 (Andi real-device test, after 9-7)

Source: Andi's real-device smoke 2026-06-16 (the device test I wrongly skipped at 9-7 GATE-4). Three
findings; the first is a real functional bug, the other two Andi explicitly flagged for "later".

- **RESOLVED into Story 9-11** — Android Auto mode quiet-speech miss. Initially reported as "Auto totally
  broken"; instrumented-build device telemetry (2026-06-16) showed that was a **stale build** — Auto works
  ~90%. The real residual: the RMS energy pre-gate is hard-coded `0.02` on Android (4× the desktop default
  `0.005`) and ignores the user's `silence_threshold` setting, so quiet speech (mic peaks ~0.026–0.037)
  fails the onset and merges into the next utterance. Root cause device-evidenced (`/tmp/9-7-auto-vad.log`).
  → **Story 9-11** (Android honors `silence_threshold` + default 0.005 + multi-fire guard), being built now.
  9-7 closed (its mode-mirroring scope was met; this was never a mode bug).

- **STORY 9-12 (follow-up to 9-11) — proactive sensitivity hint.** Andi's idea: rather than chase a magic
  threshold, make the recurring pre-filter problem **self-serviceable**. When the app detects the pattern
  "Silero VAD reports speech but the energy pre-gate vetoes it → onset never confirms → utterance dropped/
  merged" (exactly the `vadTrue` high / `speechFrames` low / `onsetFrames=0` signature in the device log),
  surface a user-facing hint: *"Speech detected but discarded as too quiet — adjust the sensitivity?"* with
  a shortcut to the slider. Precondition: **9-11** (the adjustable control must exist first). Cross-platform:
  consider whether desktop wants the same hint. Scope as a proper story.

- **NEXT STORY — bubble must show which mode is active.** On-device, the bubble's appearance does not make
  the current gesture mode (Hold / Toggle / AutoStop / Auto) visible at all — the user can't tell what is
  about to happen on a tap. Andi: "müssen wir in einer nächsten Story auf jeden Fall planen." Scope a
  story that surfaces the active mode on the bubble (a badge/affordance — note the `RecordingMode` enum
  already carries a one-letter `badge` H/T/S/A, currently unused on the bubble surface). Cross-platform:
  check desktop parity.

## Accessibility — canvas-drawn listening panel has no TalkBack labels

Source: Story 9-5 code-review (story-conductor, 2026-06-16). AC1 / Task 2.2 said "relabel
`contentDescription` to Abbrechen", but the listening panel (`ListeningPanelView`) is a custom
`View` that draws its controls (red Abbrechen square, K-badge, waveform, timer) directly on a
`Canvas` — there is no child `View` to carry a `contentDescription`, and there never was one to
"relabel". The red square (and the whole panel) is therefore invisible to TalkBack. Pre-existing
limitation of the canvas-drawn approach, NOT introduced by 9-5 — deferred rather than faked.

- **STORY — give the listening panel a real accessibility surface.** Add an
  `AccessibilityNodeProvider` / `ExploreByTouchHelper` virtual view hierarchy over
  `ListeningPanelView` so TalkBack exposes the Abbrechen square (`contentDescription="Abbrechen"`)
  and the live transcript/timer. Scope is the whole canvas panel, not just the red square. **AC:**
  TalkBack focuses and announces the Abbrechen control; activating it cancels recording.

## Android live-preview parity + open dictation-end interaction question

Source: Andi's 9-5 real-device review (2026-06-16). 9-5 function approved ("erstmal abgesegnet");
these are **design decisions for later — NOT functional gaps in 9-5**, captured so they don't get lost.
Not yet scoped into stories.

- **Android live cleanup-preview is not working / likely not built yet.** The desktop has the live
  cleanup preview (Epics 5/6). On Android it does not work. Confirm whether it's simply unbuilt on
  Android vs. broken, then scope. (Note: 9-5's listening panel shows the live RAW transcript inside the
  panel per AC2 — distinguish that from the desktop's *cleaned* live-preview when scoping.)
  - **Re-confirmed 2026-06-16 (Andi real-device, after 9-7):** the preview display itself does not work
    on Android, AND the **waveform inside the preview/listening panel does not work** either. Both stay
    "later" per Andi. Scope together with the preview-build investigation above.

- **No Android Settings control for the live-preview.** Desktop exposes a preview opt-in toggle + pause
  slider (Stories 5-3 / 5-5); Android Settings has no equivalent. A parity story once the Android
  preview itself exists.
  - **Re-confirmed 2026-06-19 (Andi 9-5 GATE-4):** the desktop-style *cleaned* live-preview is still
    not built on Android (answers "in which story is live-preview planned?" — it is **not yet a story**,
    only this backlog item). Needs its own story via on-device Whisper (benchmark-first; 3 design gates:
    local-vs-cloud ADR / paused-vs-continuous / raw-preview). 9-5 ships caret-only RECORDING feedback.

## In-app big-mic-button on Android — fate undecided (surfaced 2026-06-21 during 9-9)

Source: 9-9 conductor run + Andi. While re-skinning the in-app `RecordButton` (`android-05`), found that
this surface is **not in the current Model-B design canon**: `.inapp-mic` (klarvo.css:452) is an orphan
CSS rule, never instantiated in `docs/design/overhaul/source/Klarvo Design System.html`. The canon's
Android recording language is the bubble overlay only. The big round in-app mic button (App.tsx:51/734)
still **exists and is reachable on Android** (open app → tap mic → in-app record with Mic/Stop/Spinner),
but it is shared-desktop React, not an Android-designed surface.

- 9-9 closed it as a pure DT-closure (red→amber + remove hardcoded colors), NOT a design realization.
- **Open product decision (not 9-9's job):** should the in-app big-mic button on Android be **kept** as a
  standalone-recorder fallback, **hidden** on Android (recording = bubble only), or **redesigned** to the
  Model-B symbolism? If kept/redesigned, the canon needs an explicit in-app-recording surface (Phase A)
  first — today it has none.

## Story 9-5 GATE-4 green — Modell B interaction follow-ups (2026-06-19)

Source: Andi's 9-5 real-device GATE-4 (2026-06-19) — **9-5 approved → done**. These refine the *passing*
Modell B build; they are NOT 9-5 gaps. #2 and #4 change the recording-cluster **interaction** → each needs
its design decision settled in the **canon first (ADR-0019 §4′ amendment), design-gate = human**, before
build. Anchor: `docs/design/overhaul/source/` + `mockup-bubble-preview-modelB.html`.

> **✅ DESIGN-GATE AUFGELÖST 2026-06-21 (Andi-approved).** #2 + #4 sind im Canon entschieden:
> [ADR-0019 §4′-Amendment 2026-06-21](adr/0019-cross-platform-design-ssot.md#§4-amendment-2026-06-21--9-5-gate-follow-ups-2--4)
> + Canon-Quelle (Fingerprint `fc9ef745…`, MANIFEST 2026-06-21) + Render `mockup-9-5-followups-2-4.html`.
> **#2** = Cluster getauscht `[✗ links · Waveform · ➤ Senden rechts]`. **#4** = HOLD-Variante (halten=aufnehmen ·
> loslassen=senden · wegziehen=abbrechen · hoch=🔒→normaler Cluster); eigene Build-Story, 9-7 NICHT still erweitern.
> **#1** war immer design-frei. → Alle drei sind jetzt **build-reif** (kein Design-Gate mehr). #1 (design-frei)
> eignet sich direkt für den story-conductor; #2/#4 brauchen je eine Build-Story gegen den neuen Canon.

- **(1) Cluster waveform must be RMS-reactive (voice-driven), like the desktop app.** Today the amber
  waveform in the recording cluster animates but does not track the live voice amplitude — it looks
  static/idle. AC4 already specified "bars driven by RMS amplitude (reuse `drawWaveformBarsInZone()`)",
  so the amplitude feed into the **cluster** waveform zone is evidently unwired (or always falls back to
  the flat-idle `abwv` animation). **Fidelity fix** — trace the RMS stream into the cluster waveform.

- **(2) Swap ➤ Send and ✗ Cancel positions in the cluster.** Human habit expects the **➤ Send button at
  the same screen position as the idle "K" bubble** (the dock spot the thumb just tapped). Current layout
  is `[➤ send · waveform · ✗ cancel]` with the idle bubble at the right dock edge → Send should move to
  the idle-bubble side. **Design change to ADR-0019 §4′ cluster geometry** (Andi-decided direction; exact
  canon + `mockup-bubble-preview-modelB.html` update belongs to the fresh session).

- **(4) HOLD (push-to-talk) mode needs different bubble behavior.** In HOLD mode, releasing the hold
  **already sends**, so a separate ➤ Send button is redundant AND the ✗ Cancel button is effectively
  unusable (you'd have to keep holding to reach it, and releasing sends before you can cancel). The Modell
  B cluster (designed for tap/toggle) doesn't fit HOLD. **Design decision needed** — likely a per-mode
  cluster variant (e.g. HOLD shows only a slide/hold-to-cancel affordance, no ➤). Amends ADR-0019 §4′ for
  the HOLD gesture mode; relates to 9-7 (gesture modes) — do **not** silently expand 9-7.

- **OPEN DESIGN DECISION — where the "end dictation" affordance lives.** 9-5 (per ADR-0019) put *Senden*
  on the bubble (tap), with the panel's red square = *Abbrechen*. Andi is reconsidering whether the
  end-dictation control should instead sit **in the preview view next to the stop button**. This would
  revisit the ADR-0019 Android interaction model — handle via correct-course / an ADR amendment if
  pursued, not an ad-hoc change. Decision pending; no action now.

---

## Story 9-12 — Waveform-Feinschliff (deferred 2026-06-21, Andi)

Source: 9-12 close-out. Andi nahm den Cluster-Waveform am echten Gerät ab („ja, passt") — Desktop-Paritäts-
Port (scrollende 20er-Pegel-Historie, still bei Stille, weicher Fade). **„Feinschliff noch nötig, aber nicht
jetzt"** — nicht spezifiziert. Wenn Andi es aufgreift, konkret machen. Bekannte Kandidaten:

- **Cross-Mic-Robustheit der Magic-Numbers.** `smoothedAmplitude()` ist auf Andis Gerät kalibriert
  (`noiseFloor=0.012`, Band `[0.012..0.15]`, Gain ×4.0). Andere Mikros/Distanzen können clippen oder zu
  schwach sein. Evtl. adaptive Normalisierung statt fester Konstanten (Reviewer-Flag).
- **`ListeningPanelView`-Waveform-Abgleich.** Dieser View hat noch den ALTEN Cosinus-Sweep
  (`barAnimator`/`barPhaseOffsets`, `ListeningPanelView.kt:294/468`) und wird via `panelView?.amplitude`
  gefüttert. In Modell B ist das Panel passiv (nur Text+Zeit) → vermutlich dormant/nicht gezeichnet. **Prüfen:**
  zeichnet das Panel je eine Waveform? Falls ja, hat es das alte (abgehackte) Verhalten → auf Desktop-Parität
  ziehen. War bewusst NICHT in 9-12-Scope (Cluster-only).
- **Scroll-Glätte bei 64ms-Update-Takt** (15 fps) — falls bei genauem Hinsehen steppig, interpolieren.
- **Reaktions-Latenz vs. Desktop (NEU 2026-06-26, Andi real-device nach 9-13).** Die Cluster-Waveform
  reagiert auf die Stimme mit **~100-200ms Verzögerung**; die Windows-Pill reagiert **fast instant** —
  spürbar träger und irritierend. Kandidaten: 64ms-Update-Takt (15 fps) + die `smoothedAmplitude()`-Glättung
  (EMA/Fade) addieren Latenz; und der Amplituden-Feed-Pfad (AudioRecord-Buffer-Größe / Chunk-Latenz auf
  Android vs. der cpal-Pfad auf Desktop). Messen: Stimm-Onset → erste Balken-Reaktion auf beiden Plattformen,
  dann den dominierenden Latenz-Term kürzen. Ziel = Desktop-Parität (near-instant); Anker `src/FloatingBar.tsx`.

GATE = echtes Gerät, Live-Mikro (Emulator kann Bewegung nicht beurteilen). Anker: `src/FloatingBar.tsx`
(Desktop-SOLL), ADR-0019 §4′-Amendment #1-Anker „Realisiert 2026-06-21".

## Tooling — klarvo BMAD-Version auf 6.8 ziehen (eigene, bewusste Entscheidung; NICHT jetzt)

Source: BMAD-Internals-Session 2026-06-16 (Skill-Inventar-Diff klarvo 6.6.1-next.2 ↔ awos 6.8.0).

klarvo läuft BMAD `6.6.1-next.2`, awos `6.8.0`. Die Versions-Differenz erzeugt Skill-Namens-Drift
(6.8 hat Umbrella-Skills `bmad-prd`/`bmad-ux` + neue `bmad-investigate`/`bmad-spec`, die klarvo fehlen).
Andis Instinkt war, die Versionen anzugleichen — bewusst **entkoppelt** vom Routing-Guide-Cleanup, weil:
(a) es Cause-A (optionale Module CIS/TestArch) NICHT löst, nur die Namens-Drift; (b) ein Update **mitten
im Conductor-Lauf riskant** ist — unsere handgebauten Conductors hängen an genau den Skill-Internas
(On-Activation-Boilerplate, code-review-Step-Struktur, sprint-status, customize.toml-Schema), die ein
6.6→6.8-Sprung verschieben kann. **Vorbedingung vor Durchführung:** Blast-Radius-Prüfung (was ändert
sich an den Skills, an denen die Conductors hängen) + ein sicherer Zeitpunkt (kein laufender Epic-Lauf).
Referenz-Mechanik: `~/.bmad/guides/bmad-internals.md`. Status: **geparkt, Decision pending.**

## Android-Transkription langsamer geworden — Regression (2026-06-26, Andi real-device)

Source: Andi real-device, nach dem 9-13-Install. Das **Transkribieren in der Android-App ist „seit ein paar
Rebuilds" spürbar langsamer** geworden — „das hatten wir mal deutlich schneller". Welche Änderung das bewirkt
hat, ist offen → **Regressions-Untersuchung**, nicht spekulativ fixen.

- **Erst messen:** end-to-end STT-Latenz (Aufnahme-Ende → eingefügter Text) auf Android über die letzten
  Builds vergleichen; Timestamp-Logging um den Groq-STT-Call (Request-Start → Response → Paste).
- **Verdächtige (seit „schnell" dazugekommen):** der Min-Length-/Silence-Pre-Filter (Story 2-2) + Hallucination-
  Filter (2-1) als zusätzliche Vor-/Nachverarbeitung; Änderungen am STT-Request-/Guard-Pfad via JNI (7-3);
  Audio-Encoding / WAV-Payload-Größe; Chunking-Parameter; Netzwerk/Region zum Groq-Endpoint.
- **Wichtig:** Android-STT = **Groq Cloud** `large-v3-turbo` (NICHT on-device; `reference_android_stt_is_groq_cloud`)
  → die Latenz ist Netzwerk + Payload + Vor/Nachverarbeitung, **nicht** lokale Inferenz. Erst lokalisieren
  (welcher Term dominiert), dann gezielt kürzen; ggf. `git bisect` über die Android-Builds seit „schnell".

GATE = echtes Gerät (Latenz-Empfinden ist Andis). Status: **geparkt, Untersuchung offen.**

## WebView2 Fixed-Runtime-Pin — Distribution + Durability-Follow-ups (2026-06-26)

Source: Overlay-Occlusion-Regression gelöst durch Runtime-Pin auf `149.0.4022.62` ([ADR-0020](adr/0020-webview2-fixed-runtime-pin.md);
Mess-Saga in Memory `project_webview2_overlay_backgrounding`). Der Fix ist im lokalen Dev-Build verankert (Code-Pin in
`lib.rs` + gebündelte Runtime in `target\release\webview2-runtime` + Self-Heal in `sync-and-build.ps1` aus
Master-Kopie `D:\apps\klarvo-webview2-runtime`). Offen:

- **Distribution-Pfad:** Für den ausgelieferten Installer (NSIS/MSI) auf Tauris natives
  `bundle.windows.webviewInstallMode: { type: "fixedRuntime", path: "…" }` umstellen, statt die exe-relative Kopie.
  Aktuell bringt nur der lokale Build die Runtime mit; ein frisch installierter Endnutzer-Build hätte den Pin nicht.
- **Runtime-Bump-Prozess:** Bewusster Weg dokumentieren/scripten, um die gepinnte Runtime zu aktualisieren (neue
  Version testen mit dem Probe-Skript → Master-Kopie ersetzen → Build). Gepinnt = **kein** Auto-Security-Update.
- **Re-Verifikation über Zeit:** Andis Abnahme war „vorerst grün". Bei Wiederauftreten der unsichtbaren Pille
  ZUERST Klarvo.log-Zeile `[webview2] runtime: …` prüfen (Pin aktiv? `webview2-runtime` vs „Evergreen (not pinned)")
  und ob Master-Kopie + `target\…\webview2-runtime` existieren — erst dann tiefer graben.

GATE = echtes Gerät über Tage (Andi). Status: **Kern-Fix gebaut + maschinen-verifiziert; Distribution + Bump-Prozess offen.**

---

## Story 9-15 (Mobile TAP-Surface) — Code-Review-Defers (2026-06-30)

Source: Conductor-Code-Review von Story 9-15 (baseRef `ed219f10..a3d0233`, 3 Reviewer). Findings im Story-File
`9-15-mobile-tap-recording-surface-reskin.md` → „Review Findings". Story selbst geht trotzdem auf `done`
(GATE-4 visuell = Andis Real-Device-Gate). Diese Punkte sind real, aber nicht close-blockierend:

- **Per-Frame-Allocations auf dem RECORDING-Draw-Pfad** — `drawTapChip`/`drawTapSendCircle`/`drawTapCancelCircle`
  allokieren je Frame `BlurMaskFilter`, `LinearGradient`, `Path`, `RectF` (FloatingBubbleView.kt:663/697/702/741/768),
  obwohl ein Kommentar „pre-alloc to avoid GC on each amplitude-driven invalidate()" das Gegenteil behauptet.
  ~15fps während Aufnahme. Fix nicht-trivial: Gradient/Path sind positions-abhängig (variieren mit Dock-Seite) →
  naives Cachen würde Rendering-Bugs einführen; korrekter Fix = Cache + Invalidierung bei Geometrie-Wechsel.
- **Cancel-Label 13sp vs Render-SOLL 15px** (`.ztap .lab{font-size:15px}` gilt Send+Cancel; Send nutzt 15sp korrekt).
  Bewusst geschrumpft, damit „Abbrechen" passt. 15sp riskiert Clipping → **Andis Visual-Gate entscheidet**, ob die
  SOLL-Größe passt oder der Kreis/das Layout angepasst werden muss.
- **TAP-Fenster 340dp ohne Width-Clamp** — auf <340dp-Screens (sw320, Split-Screen, Foldable-Cover) überläuft das
  Fenster + Off-Dock-Kreis die Bildschirmkante (teils untappbar). Sauberer Fix = Durchmesser+Gap auf `screenW` skalieren,
  nicht nur clampen.
- **Drag der RECORDING-TAP-Surface → Edge-Snap mit idle-Breite** — beim Loslassen snapt der Edge-Snap mit der idle-72dp-
  Breite statt der 340dp-Fensterbreite → Surface landet großteils off-screen, schlechtes x wird via `savePosition` persistiert.
- **AC6 oben/unten-Dock-Mirroring nicht implementiert** — `getDockSide()` löst nur links/rechts auf. Render spezifiziert
  nur `tapLeft`/`tapRight` (kein oben/unten-Frame) → render-unspezifiziert; bei späterem oben/unten-Bedarf nachziehen.

---

## Story 9-15 Re-Scope (konfigurierbare Button-Größe) — Code-Review-Defers (2026-06-30)

Source: Conductor-Re-Review nach der Re-Scope (range 218ee5d..67f20b6, 3 Reviewer). Funktional clean; diese Robustheits-Punkte sind real, aber nicht close-blockierend (UI sendet nur {60,72,88}; nur hand-editierte/alte config.json triggert sie):

- **Diskret-Set {60,72,88} nirgends erzwungen, nur range-geclampt [60,88]** — `recordingButtonSizeDp` wird in Kotlin (`coerceIn(60,88)`) und Rust (kein Clamp) nur auf den Bereich begrenzt; ein hand-editierter Wert wie 70/75 rendert off-spec, und das React-Segment-Control zeigt dann KEINE aktive Auswahl (kein `aria-pressed`). Fix (eine Stelle = SSOT): snap-to-nearest-of-{60,72,88} im Rust `merge_settings`/`save_settings`.
- **Rust-Schicht validiert/clampt den Wert nicht** beim Persistieren (Android coerced erst beim Lesen; Desktop ignoriert das Feld). Persistierte SSOT kann out-of-range sein.
- **`.toInt()`-Truncation** in `tapVisualWidthDp/HeightDp` (`(size*320f/132f).toInt()`) floored die Fenster-Region <1dp unter den Float-Inhalt; aktuell vom 10dp-Schatten-Pad absorbiert (kein Clip), aber latent bei künftiger Pad-Reduktion.
- **Cross-Layer-Default 72 durch keinen Test gepinnt** — 72 ist an 5 Stellen dupliziert (TS types/mock, Rust default-fn, Kotlin data-class, Kotlin optInt-Fallback, TAP_BUTTON_SIZE_DEFAULT); aktuell konsistent, aber ein künftiger Edit an einer Stelle würde nicht gefangen.
- **Visuell (Andis Geräte-Gate, am 60dp-Ende):** Waveform-Chip-Bars skalieren nicht mit der Button-Größe (AC5 schützt drawClusterWaveform) → proportional groß bei 60dp; Label „Abbrechen" ~6.8sp bei 60dp. Beide nur mit device-validierten Werten anfassen, falls Andi sie bemängelt.

---

## Story 9-14 HOLD — Defer: HOLD-Größen an recordingButtonSizeDp koppeln (2026-06-30)

Source: GATE-1-Designentscheidung im 9-14-story-conductor-Lauf (Andi). 9-14 baut die HOLD-Ziele mit **fixen**, geräte-skaliert abgenommenen Mockup-dp (Ruhe 112 · aktiv 148 · Daumen-Bubble 82) — bewusst **keine** Kopplung an 9-15s nutzer-konfigurierbaren Regler `recordingButtonSizeDp` {60,72,88}.

- **Defer:** HOLD-Ziel-Proportionen optional an `recordingButtonSizeDp` koppeln (Konsistenz mit der TAP-Surface, Nutzer-Kontrolle über beide Surfaces).
- **Warum jetzt nicht:** außerhalb aktueller Scope (keine neuen Config-Keys in 9-14); die komplexe HOLD-Geometrie (zwei wachsende Ziele weg vom Daumen) bei 60dp neu zu validieren vergrößert die Geräte-Test-Fläche + „zu-klein"-Risiko. Reduktion vor Konstruktion: erst die fixe Variante am echten Gerät validieren, dann erst koppeln — falls Andi nach dem Real-Device-Test mehr Kontrolle will.

---

## Story 9-14 HOLD — Code-Review-Defers (2026-06-30)

Source: Conductor-Code-Review nach dev (range 20e74c6..ce20bb0, 3 Reviewer Blind/Edge/Auditor). A/B/C als Patch behoben (siehe Fix-Commit). Folgende real, aber nicht close-blockierend — Fidelity-Items gehören laut Story-DoD Andis Real-Device-GATE-4:

- **Erster ACTION_MOVE liest stale idle-Square-Width (1 Frame)** — `adjustLayoutForState` → `updateViewLayout` ist async; ein ACTION_MOVE vor dem Layout-Pass liest die alte `bubbleView.width` → `holdTargetCenters` für ein/zwei Frames falsch (v.a. left-dock). Transient/selbst-korrigierend. Sauberer Fix nur falls am Gerät spürbar.
- **[GATE-4 Fidelity] Drag-Ghost-Bubble + Origin-Fade fehlt** — Render `bHit` (`mockup-mobile-hold-B-refined.html:159/161`) zeichnet eine finger-folgende `.ghost`-Bubble (74×74, dashed, ~50% alpha) + faded die Origin-`.heldbub` auf opacity .32 beim Ziehen. Implementierung zeichnet die Anker-Bubble fix am Dock, voll opak, ohne Ghost. In Dev Notes geflaggt, aber nicht gebaut/deferred. Finger-Positions-Feedback fehlt damit in der Drag-Phase. **Andi am Gerät entscheiden: bauen oder akzeptabel ohne.**
- **[GATE-4 Fidelity] Live-Caption wechselt nicht beim Ziel-Treffer** — `bHit` ändert `.reccap`-Text auf „Finger auf Abbrechen · loslassen löst aus"; Implementierung rendert statisch „Aufnahme · loslassen = senden". Das Ziel-eigene Zwei-Zeilen-Label wechselt korrekt (erfüllt explizite AC3/AC4-Prosa) → nur Render-Fidelity-Lücke.
- **[GATE-4 Fidelity] `.heldbub .finger`-Indikator + inner amber `.ring` weggefallen** — Anker-Bubble zeichnet Schatten + teal-Gradient + äußeren 5dp-amber-Ring + „K", aber nicht den `.finger`-Child (in `bRest`+`bHit`) noch den inneren `.ring` (inset −11px, 2.5px amber, opacity .55). Minor.
- **[GATE-4 Fidelity] Caption-Clip + Chip↔Abbrechen-Nähe (AC2)** — die am Bubble zentrierte Caption kann bei Dock-Nah die Fensterkante überlaufen (clip); Waveform-Chip-Unterkante sitzt evtl. wenige dp vom Abbrechen-Kreis (AC2 „kein Überlapp", low confidence) → am Gerät prüfen.

---

## Story 9-14 — Re-Scope auf vereinfachtes HOLD (2026-07-01): vorige Defers AUFGELÖST

Source: Andis Real-Device-Gate + Design-Rethink (ADR-0019 Amendment 2026-07-01). Das Zwei-Ziel-HOLD wurde verworfen → vereinfachtes Ein-Button-Modell (ein Abbrechen-Button, Senden=Loslassen, kein Sperren). Damit sind folgende frühere Einträge **hinfällig/aufgelöst**:
- „HOLD-Größen an recordingButtonSizeDp koppeln" (2026-06-30) → **umgesetzt** im neuen Scope (Abbrechen am Regler; Anker an Idle-Größe).
- „[GATE-4 Fidelity] Drag-Ghost-Bubble + Origin-Fade / Caption-Update / .finger / inner-ring" (2026-06-30) → **in das neue Scope gezogen** (Dynamik wird gebaut, nicht mehr vertagt).
- „Erster ACTION_MOVE liest stale idle-Width" → bleibt offen, gilt weiter im neuen Build.
NEU offen: `recordingButtonSizeDp`-Regler um mehr + kleinere Stufen erweitern (Teil 9-14-Scope, berührt 9-15-Settings-UI).

---

## Story 9-14 vereinfacht — Code-Review-Defers (2026-07-01)

Source: Conductor-Re-Review (range 4d2bc9b..d01381a, 3 Reviewer). A/B/D/E/F als Patch behoben; C refuted (getBubbleSizeDp = reiner Getter). Folgende minor/edge, nicht close-blockierend:
- **Multi-Touch: primärer Finger hebt via ACTION_POINTER_UP ab, zweiter bleibt** → Commit (send/cancel) verzögert bis zum letzten Finger-Lift (KlarvoOverlayService ~1142). Selten; Primary-Pointer-Lock ist Absicht. Fix nur falls am Gerät störend.
- **Ghost-Squircle folgt rohem Finger ohne Bounds-Clamp** → kosmetisches Clipping nahe Fensterkante (FloatingBubbleView ~560). Teilweise durch E (alpha) entschärft.
- **Sub-48dp Idle-Bubble: ~wenige dp X-Drift** beim idle→hold-Übergang (idle-Fenster ≥48dp gefloort). Edge.
- **Left-Dock Anker-X (low confidence):** rechts-Kanten-Anker-Formel klemmt links auf 0 → ~Zehner-px Anker-Shift nur bei Links-Dock. AC2 am Gerät prüfen (rechts-Dock exakt).

---

## Epic 11 — Cross-Platform Live-Preview (Android) — Kickoff 2026-07-01

Source: Andi-Entscheidung + Story 11-1 Benchmark (Machbarkeits-Spike, DONE 2026-07-01). Desktop-Live-Preview-Box (Epics 5/6) auf Android bringen; benchmark-first.

- **11-1 DONE (GO/grün):** Groq-Pause-to-Text am echten Gerät = median 786 ms (n=4, alle < 1 s, keine Retry-Kontamination). Caveat: max 999 ms an der 1-s-Kante, ~½ der Latenz = Groq-Netz-RTT → Preview muss gelegentliches Sample > 1 s vertragen.
- **11-2 (NÄCHSTE STORY, noch nicht geschrieben) = eigentlicher Android-Preview-Port.** Architektur ENTSCHIEDEN (Andi 2026-07-01): **Groq-Delta-STT** wie Desktop — `delta_snapshot_wav()`-Äquivalent (jede Pause STT't nur neues Audio seit letzter Pause) → Gesamt ≈ **2× Groq-Audio-Sekunden** pro Diktat, NICHT N×. Braucht Andis Design-Input beim Schreiben (spiegelt Windows-Box? Overlay-Position? Settings-Toggle-Verhalten wie Desktop?).
- **DEFERRED — lokales Modell für Preview:** 0 Groq-Kosten/offline, aber (a) mein Groq-Benchmark gilt NICHT (lokale On-Device-Whisper-Latenz unbekannt → eigener Benchmark nötig), (b) Aktivierung des schlafenden Android-Whisper-JNI + Modell-Binary (App-Größe). Nur aufgreifen, falls ~2× Groq das Free-Tier real sprengt (Volumen-abhängig).
- **Instrumentierung aus 11-1 bleibt im Code** (`[benchmark-11-1]`-Logs, `KlarvoOverlayService`/`KlarvoAudioRecorder`) — für 11-2 evtl. nützlich, sonst später entfernen.
- **11-2 DEFERRED — akzeptierte Low-Findings aus dem Code-Review (2026-07-01, story-conductor).** Bewusst nicht in 11-2 gefixt (Orientierungs-Surface, nicht Genauigkeit), Source = 3-Reviewer-Review von `32a3770..beb005c`:
  - *Failed/blank Preview-Chunk lässt sein Audio-Fenster fallen* — `deltaSnapshotWav()` schiebt den Marker synchron vor der Transkription; blank/hallucination/error-Chunk → dauerhafte Lücke im Preview-Text (fail-soft, aber Inhalts-Lücke). Fix-Idee: Marker erst nach erfolgreicher Transkription vorschieben.
  - *Font `sp` vs Desktop `px`* — Android nutzt `FONT_PX_SP` als sp (korrekte Android-Einheit, aber Größen matchen Desktop-px nicht 1:1). Akzeptiert (sp ist Android-richtig).
  - *Caret bleibt oben im scrollenden Transcript* — blinkender Caret `Gravity.TOP|START` im auto-scrollenden ScrollView; folgt dem neuesten Text nicht.
  - *Chunk-Join immer mit einem Space* — `"$acc $text"` unabhängig von Interpunktion.
- **11-2 F5-Entscheidung (Andi-override möglich):** Bg-Blur-Regler auf Android **versteckt** (wie Breiten-Regler), weil `RenderEffect` API 31 braucht (minSdk 24) → sonst inerter Regler. Config-Feld bleibt für Desktop. Falls Blur auf Android gewünscht: eigene Story (RenderEffect ≥ API31 + No-op-Fallback < 31).

### 11-3 (Follow-up) — Android Preview-Box Geräte-Feedback-Pass — Source: Andi device-verify 2026-07-02
> **⚠️ HOCHGESTUFT 2026-07-07 (Andi): nicht mehr nur Politur — Bedienbarkeits-Blocker.** Andi meldet: die
> mitwachsende Preview-Box (Punkt 3) füllt in der Praxis den ganzen Bildschirm; weil sie als Overlay ganz
> oben liegt UND die Aufnahme-Controls im *Bubble*-Fenster darunter verdeckt/deren Touches schluckt, wird
> das Gerät während der Aufnahme unbenutzbar (Bubble = Senden/Abbrechen nicht mehr erreichbar). Geerdet im
> Code 2026-07-07: Preview-Fenster ist `WRAP_CONTENT` ohne Max (`KlarvoOverlayService.kt:2282`) → der in
> 11-2 gebaute ScrollView ist inert, weil das Fenster selbst mitwächst; Bubble + Preview haben denselben
> Fenstertyp/Flags/PID → Z-Order = Add-Reihenfolge → später hinzugefügte Preview liegt über der Bubble.
> Punkt 3 (rollendes Fenster mit fester Höhe) + neuer Punkt 6 (Bubble-Z-Band) lösen zusammen die Wurzel.
> **11-3 ist damit die nächste Android-Story, VOR Epic 8** (Andi 2026-07-07). Verify = Andi-Real-Device.

11-2 DONE + real-device-verified; folgende Politur-Punkte aus Andis Geräte-Runde (nicht 11-2 wieder aufmachen):
1. **Box-Header „Aufnahme" → „Live-Preview".** (Copy; `ListeningPanelView`-Header.)
2. **„Ich höre zu…" unten entfernen** — unnötig im Preview-Kontext.
3. **Feste Box-Größe statt Mitwachsen** — das Desktop-Verhalten „Box wächst mit Text" ergibt auf Android keinen Sinn; Box bleibt fest über dem Keyboard. **ENTSCHIEDEN (Andi 2026-07-02): rollendes Fenster** — Box zeigt nur die letzten Zeilen, Älteres rollt oben raus (sanft ausgefadet), kein Scrollen. (Ersetzt den gebauten ScrollView-Auto-Scroll → auf rollendes Last-N-Fenster mit Top-Fade umbauen.)
4. **Griff-Linie (GripView) oben mitte entfernen** — suggeriert Resize, den es nicht gibt; entfernt → macht Platz, Header-Elemente rücken näher an den Rand.
5. **Font-Skala verschieben (Andi: klein ist viel zu klein)** — **ENTSCHIEDEN (Andi 2026-07-02): `FONT_PX_SP` = klein/mittel/groß → 13 / 15 / 18 sp, Android-only (Desktop-`FONT_PX_MAP` unberührt).** (Ersetzt den akzeptierten „Font sp vs px"-Residual oben.)
6. **Bubble muss IMMER über der Preview liegen — Z-Order-Fix (NEU 2026-07-07, Andi).** Bug: Bubble (`bubbleParams`, `KlarvoOverlayService.kt:980`) und Preview-Panel (`KlarvoOverlayService.kt:2280`) nutzen denselben `overlayType` + identische Flags → gleiches Z-Band → Reihenfolge entscheidet → Preview (später addiert) liegt über der Bubble und verdeckt die Aufnahme-Controls (Senden/Abbrechen sitzen im Bubble-Fenster, Kommentar `:2302-2304`). **ENTSCHIEDEN (Andi 2026-07-07): Bubble strukturell in ein HÖHERES Z-Band** heben (höherer Fenster-Typ), sodass sie unabhängig von der Add-Reihenfolge immer oben liegt — nicht bloß Re-Add nach der Preview. Typ-Wechsel → real-device-smoke Pflicht (Overlay-Permission/HyperOS-Quirks beachten, vgl. `reference_hyperos_overlay_quirks`). Hängt eng an Punkt 3: sobald die Box feste Höhe hat, überlappt sie ohnehin weniger, aber die Bubble-Erreichbarkeit muss strukturell garantiert sein.

> **UPDATE 2026-07-07 — Punkte 1–5 gebaut (Story 11-3, AC-1..AC-5), Punkt 6 abgespalten → Story 11-4.**
> 11-3 lieferte die feste Höhe + rollendes Fenster (Punkt 3) und Copy/Font/Grip (1,2,4,5); damit ist
> der **katastrophale Bug (Box füllt Bildschirm → Handy unbedienbar) weg**. Story-Conductor-Lauf
> `fix/11-3-android-preview-box`, code-review-cleared, 163 JVM-Tests grün. Punkt 6 (Z-Order) → **11-4**.

### 11-4 — Bubble strukturell über der Preview (Z-Order) — Split aus 11-3 Punkt 6 (2026-07-07)
Source: 11-3 GATE-2-Entscheidung (Andi 2026-07-07). Der Dev-Investigation in 11-3 ergab: **kein leichter,
erlaubnisfreier Weg** existiert — das einzige Mittel, das `TYPE_APPLICATION_OVERLAY` strukturell überragt,
ist `TYPE_ACCESSIBILITY_OVERLAY`, nur vom Bedienungshilfe-Dienst erzeugbar → ~350 Zeilen Bubble-Logik in
`KlarvoAccessibilityService` umhängen + **Regressionsgefahr: Bubble verschwände für Nutzer ohne aktivierten
a11y-Dienst** (heute best-effort/optional).
- **INTENT GEKLÄRT (Andi 2026-07-08 Device-Test): echtes Z-LAYERING gewollt, NICHT Positionierung.**
  Der 11-3-Build zeigt aktuell einen ungewollten „Trick": zieht Andi die Bubble in die Box, wird sie
  *geometrisch* über die Preview versetzt (Bubble bleibt über der Panel-Oberkante). Andi will das NICHT —
  er will, dass die Bubble **layer-technisch über der Preview** liegen kann (auch während der Aufnahme)
  und die Box damit **effektiv teilverdeckt**. D.h. der frühere „Positionierungs-Fix zuerst"-Plan ist
  VERWORFEN: 11-4 = **echte Z-Schichtung** (Bubble über Panel, darf überlappen), plus die ungewollte
  vertikale Repositionierungs-Klammer ENTFERNEN.
- **Mechanik (zu untersuchen):** gleiche Fenster-Typen → Z = Add-Reihenfolge. Leichtester Weg evtl.
  Bubble nach `showListeningPanel` erneut oben einhängen (Re-Add) statt des a11y-Umbaus — der Dev hatte
  Re-Add als „fragil" verworfen, aber für „Bubble über Panel" ist genau Add-Order das Mittel; a11y
  (`TYPE_ACCESSIBILITY_OVERLAY`, ~350 LOC Reparenting + Regressionsrisiko Bubble ohne a11y-Dienst weg)
  nur, falls Re-Add die Drag-/Touch-Wege nicht sauber trägt. Verweise: `reference_hyperos_overlay_quirks`.

### 11-3 akzeptierte Low-Residuals (Andi „passt" 2026-07-08, nicht 11-3 wieder aufmachen)
- **Blinkende Caret-Linie scrollt mit dem Inhalt oben raus** (sitzt im scrollenden Transkript,
  `ListeningPanelView.kt` caret in `transcriptFrame`). Bei langem Text meist unsichtbar. Andi hat
  „passt" gesagt; falls später gewünscht: Caret als Sibling der ScrollView pinnen ODER entfernen
  (mein AC-3-Pivot erlaubte Vereinfachen). Kandidat, mit 11-4 (Bubble-Overlay) oder 11-6 zu bündeln.
- **Scroll-Stick-Schwelle `SCROLL_STICK_THRESHOLD_DP = 24`** ist ~1 Zeile — device-tunable, falls
  Auto-Follow bei kleinen Hoch-Wischern zu aggressiv zurückreißt.

### 11-6 (Backlog) — Zeilenabstand als Appearance-Setting — Source: Andi 2026-07-08 Device-Test
Preview-Box: **Zeilenabstand (line-height) als einstellbares Setting** in die Appearance-Kategorie
aufnehmen (analog Font-Größe/-Familie/Farbe). Aktuell fix `setLineSpacing(0f, 1.7f)`
(`ListeningPanelView.kt`). Desktop-Gegenstück (CSS `--k-leading` / `leading-relaxed`) mitdenken für
Cross-Platform-SSOT. Neuer Config-Key (camelCase, Backend-Locale-Files), Android- + Desktop-Wiring.

---

## Epic 12 — Cloud-Resilienz: robuste Fallback-Leiter + Audio-Retry-Historie — Kickoff 2026-07-02

Source: Live-Vorfall 2026-07-02 (DeepSeek-API-Ausfall) + Design-Durchgang mit Andi. `api.deepseek.com` war ~08:24–09:39 (Log-Zeit 10:24–10:39) tot/degradiert → Cleanup 25–29 s bzw. 30-s-Timeouts, dann Roh-Text. STT (Groq) lief normal weiter. Entscheidungs-KOMPLETT (alle Design-Calls unten getroffen), aber noch nicht als BMAD-Stories geschrieben.

### Verifizierter Ist-Zustand (Code, 2026-07-02)
- **Fallback existiert schon, feuerte aber nicht:** `resolve_fallback_provider` (pipeline.rs:193) läuft deepseek→groq→openai→openrouter. Auslöser ist NUR `is_retryable_llm_error` = `ApiError{status}` mit 429/≥500 (pipeline.rs:178). Die Ausfall-Fehler waren **Transport-Fehler** (`error sending request for url` = Timeout/Connection-refused), KEIN HTTP-Status → landen im non-retryable-Zweig (pipeline.rs:1184) → direkt Roh-Text, Fallback nie versucht. **← eigentlicher Bug.**
- **Warn-Nachricht existiert, UI wirft sie weg:** Backend sendet bei Degradierung `PipelineEvent::warn(degrade_warn_msg(...))` = „Cleanup failed — raw text inserted. <Grund>" (pipeline.rs:973/1163/1176/1188). `FloatingBar.tsx:335` verwirft `warning`-Events bewusst (`if (newState === "warning") return;`) → Nutzer sieht nichts.
- **Cleanup degradiert immer auf Roh-Text** (nie Absturz). STT kann das NICHT (kein Text → nichts zum Degradieren). Groq ist heute STT-Provider UND Cleanup-Fallback → Cleanup-Fallback auf Groq frisst STT-Kontingent.
- **Audio wird NIRGENDS persistiert:** WAV-Bytes leben nur transient (`last_recording`); `history`-Tabelle hat nur Text (`text, raw_text, style, language, is_note, app_name, uuid, device_id`) — keine Audio-Spalte/Blob/Pfad, kein Re-Processing. Andis „zweite Historie" ist wirklich neu.
- Bausteine vorhanden: lokaler Whisper (`build_local_whisper_provider`, Windows+Android, pipeline.rs:84) heute nur bei explizitem Offline-Modus; lokaler LLM-Cleanup (llama.cpp) existiert ebenfalls.

### Entschiedene Design-Calls (Andi, 2026-07-02)
- **Cleanup-Fallback-Kette:** DeepSeek → (OpenAI/OpenRouter, falls Key) → **ROHTEXT**. **NIE Groq für Cleanup** (schont STT-Kontingent — definitiv). Terminal = Roh-Text, nie Absturz.
- **STT-Fallback:** Groq → **lokaler Whisper** (Auto-Fallback, JA — bisher nur Offline-Modus) → falls kein Modell: Audio in Retry-Queue + klare Fehlermeldung. Terminal = nie stiller Verlust.
- **Fallback-Auslöser erweitern:** Transport-Fehler (Timeout, Connection-refused) müssen fallback-auslösend werden, nicht non-retryable. (Kern-Fix des Vorfalls.)
- **Pillbar-Statusanzeige (JA, Andi will es):** Warn-Event nicht mehr verwerfen, sondern kurz einblenden. Generisch-informativ, ein Satz, kein Stacktrace. Vorschlag-Taxonomie: Fallback lief `⚠ DeepSeek langsam → OpenAI` · Roh-Text `⚠ Cleanup nicht verfügbar → Rohtext eingefügt` · STT-Notanker `⚠ Groq am Limit → lokale Transkription` · alles tot `✗ Transkription fehlgeschlagen — Audio gesichert`.
- **Audio-Retry-Historie:** Variante **A jetzt** (Audio nur bei terminalem Fehlschlag speichern, nach erfolgreichem Nachverarbeiten löschen), **Datenmodell B-fähig** (Status-Feld pending/done/failed + Audio-als-Datei, damit B ohne Umbau draufsitzt). Speicherort: **Datei auf Platte, rohes WAV, Windows UND Android** (~2 MB/min; **Kompression für B später vermerkt**). Referenz per uuid. Re-Processing **manuell zuerst** (Button am geparkten Eintrag), Auto-Retry später.
- **B (Nordstern, NICHT jetzt):** dieselbe Aufnahme durch verschiedene Anbieter/Settings → Ergebnis-Vergleich; braucht dauerhafte Audio-Retention + Kompression + Vergleichs-UI.

### Story-Landschaft (Titel + Outcome; volle ACs bei bmad-create-story)
- **12-1 — Robuste LLM/STT-Fallback-Leiter + Pillbar-Statusanzeige.** Transport-Fehler lösen Fallback aus; Cleanup-Kette ohne Groq; STT→lokaler-Whisper-Auto-Fallback; Warn-Events in der Pille sichtbar (Windows + Android). Kern-Fix des Vorfalls, höchste Priorität.
- **12-2 — Audio-Retry-Historie (Primitiv A + manuelles Nachverarbeiten).** Bei terminalem Fehlschlag WAV auf Platte + „zweite Historie"-Eintrag (Status pending); manueller Re-Process-Button; Löschung nach Erfolg. Datenmodell B-fähig. Windows + Android.
- **12-3 (Nordstern, später) — Anbieter/Settings-Vergleich auf derselben Aufnahme.** Baut auf 12-2-Primitiv; Retention-Politik + Kompression + Vergleichs-UI.

### Backlog-Notizen aus 12-1-Geräte-Verifikation (2026-07-02)
- **[UX, minor] Invalid-Key-Rejection unklar:** Der Settings-Save validiert Keys (`validate_api_key`) und weigert sich korrekt, einen ungültigen Key zu persistieren — aber die Ablehnung liest sich als „Save-Button blinkt, wird nie ‚saved'" statt als klares „Invalid API key". Andi hielt es zunächst für einen kaputten Save. Kein Funktions-Bug; Klarheit verbessern (deutliche Inline-Fehlermeldung / Button-Zustand „Ungültig"). Auch: Key-Feld zeigt nach abgebrochenem Save den getippten (nicht persistierten) Wert → verstärkt die Verwirrung.
- **[Verifikations-Symmetrie] Outage-Testzustand nicht UI-herstellbar:** Weil die Validierung ungültige Keys blockt, lässt sich „Provider-Ausfall" nicht per Key-Editieren in der App testen. Für künftige Fallback-Tests: invaliden Key direkt in die `config.json` schreiben (on-device via `run-as`, mit Backup) ODER Netzwerk zum Provider blocken. In 12-1 so gemacht (verifiziert).
- **[12-1 offen] STT→lokaler-Whisper-Fallback am Gerät nicht exerziert:** Braucht invaliden Groq-Key + vorhandenes lokales Modell; Android-JNI evtl. inaktiv (Memory `reference_android_stt_is_groq_cloud`). Kein 12-1-Blocker, aber vor „done" prüfen, ob der lokale Pfad auf Android real greift oder nur Terminal-Fehler zeigt.
- **[12-1 Follow-up, UX] Windows-Pille zu schmal für volle Status-Meldung:** Andi device-verified (2026-07-03) — die längere STT-Terminal-Meldung „✗ Transkription fehlgeschlagen — Audio gesichert" wird in der ~200px-Pille zu „✗ Transkription fehl…" abgeschnitten (GATE-1-Entscheidung war Ellipsis-Kürzung; reicht für die kurzen Cleanup-Meldungen, nicht für die lange STT-Meldung). Andi „passt aber" → nicht done-blockierend. Optionen: (a) Pille für den Warn-/Fehler-Zustand temporär verbreitern/umbrechen; (b) kürzere Meldung („✗ STT-Fehler — Audio gesichert"); (c) Status ins Preview-Panel statt Pille. Android (Toast, LENGTH_LONG) zeigt die volle Meldung — nur die Windows-Pille kürzt.

---

## Epic 8 „Studio Dark" — RE-SCOPE (Andi 2026-07-06: Design abgenommen, „erstmal so" bauen)

**Quelle:** Session 2026-07-06. Andi hat die Studio-Dark-Ziel-Optik (Mockup `docs/design/overhaul/source/Klarvo Design System.html`) abgenommen und will die Basis-Erneuerung jetzt umgesetzt haben, den Feinschliff später.

**Entscheidung — der alte Branch wird NICHT gemergt.** `conductor/epic-8` (auf origin gesichert) ist 238 Commits hinter `v1-ship`; ein Merge produziert tote Konflikte auf inzwischen ersetztem Code. Stattdessen dient der gebaute Re-Skin als **Referenz-Vorlage**, und die noch lebenden Flächen werden **pro Fläche neu gegen den heutigen `v1-ship` portiert** (Story-Zyklus je Fläche). Fundament (Tokens/Fonts, 8-1) liegt bereits auf `v1-ship`.

**Scope-Aufteilung:**
- **✅ 8-2 Einstellungen / 8-5 Verlauf / 8-6 Onboarding — werden gebaut** (re-port). Referenz-Qualität laut `epic-8-fidelity-audit.md` (15.06.): 100 % Token-Treue, History „starker Match". 8-5: History lebt in `App.tsx`. **KORREKTUR 2026-08-18 (Story 8-5, am Baum geprüft): die frühere Angabe „lebt heute in `VoiceNotesPanel.tsx` (nicht mehr `App.tsx`)" war falsch** — Zustand, Suche, Liste und Leerzustand stehen sämtlich in `App.tsx`; `VoiceNotesPanel.tsx` ist ein anderes, im Header auskommentiertes Feature („Voice Notes"). 8-6: Testzustand nur nach Config-Wipe sichtbar → Erreichbarkeit als Vorlauf mitbauen (Verifikations-Symmetrie).
- **❌ 8-3 FloatingBar / 8-4 Live-Preview — SUPERSEDED** durch native Overlays (Epic 10, `native_pill.rs`/`native_preview.rs`). Kein React-Port. Der nötige native Token-Nachzug in tiny-skia/GDI ist bereits separat gelistet (siehe „Epic 8-Abhängigkeit — native Overlays … in Rust/GDI nachziehen", oben) — das ist die richtige Heimat, nicht dieser Re-Skin.
- **⏸ 8-7 Studio-Dark Fidelity-Pass — VERTAGT** (nicht „erstmal so"). Die 3–4 nicht-angewandten Affordances aus `epic-8-fidelity-audit.md`: Settings-Home Status-Dots, Datumsformat DE-kompakt in History, sowie die Pill-seitigen (Elevation/Amber-Ring/Stop-Hover) — letztere gehören ohnehin in die native Pille, nicht React. Eigene Story wenn die Basis-Flächen live sind.

**Fahrweise:** je Fläche ein `bmad-story-conductor`-Lauf, nacheinander, mit Andis Real-Device-Smoke dazwischen (visuelle Epics NICHT unbeaufsichtigt durchfahren — Postmortem 2026-06-15). Vor Andis Blick: objektiver Chromium-Harness-Abgleich gegen das Mockup.

---

## [Defekt, vorbestehend aus 8-1] `.focus-klarvo` ist ein Dauer-Ring statt Fokus-Ring (entdeckt bei 8-2 GATE-4, 2026-07-06)

**Quelle:** GATE-4-Smoke Story 8-2 (`gate4-evidence/8-2/verdict.md`). Beim Chromium-Render der gebauten Settings fiel auf: jedes Input/Select trägt einen permanenten Teal-Ring.

**Ursache:** `src/styles.css:253` definiert `.focus-klarvo { box-shadow: 0 0 0 3px rgba(41,199,172,.28); }` als **bare Klasse ohne `:focus-visible`**. Da die Komponenten `focus-klarvo` statisch in der className tragen, ist der Ring **immer an**. Der Mockup (`.input:focus`/`.select:focus`) zeigt den Ring nur bei Fokus. Stammt aus **8-1** (Kommentar „Task 5.3"), NICHT aus 8-2 — 8-2 hat die Referenz byte-identisch nachgezogen.

**Fix (klein, aber eigener Scope):** `.focus-klarvo` → `.focus-klarvo:focus-visible` (oder die Nutzung auf einen `:focus`-Modifier umstellen). ACHTUNG: `focus-klarvo` wird surface-übergreifend genutzt (Bar, Preview, Onboarding, Settings) — der Fix berührt die geteilte Token-Schicht und braucht einen eigenen Smoke über alle Flächen. Darum **eigene Story / Teil von 8-7**, nicht in 8-2 geschmuggelt.

---

## Story 11-6 GATE-4 — Nebenbefunde (2026-08-11, alle bewusst AUSSERHALB 11-6 gehalten)

**Quelle:** GATE-4-Lauf Story 11-6 auf powerhouse (`gate4-evidence/11-6/verdict.md`, Abschnitte
„Nebenbefunde" und „Messfallen"). Vier Umgebungs-/Infrastruktur-Defekte, die den Gate Zeit gekostet
haben. Keiner davon ist ein Produktfehler, deshalb kein Schmuggeln in die Story.

### 1. `gen/android/` frisst handplatzierte Dateien — Icons + Debug-Harness-Manifest ⚠️ höchste Priorität

`src-tauri/gen/android/` ist gitignored und wird von `tauri android init` **vollständig regeneriert**.
Zwei Dinge lebten nur dort und verschwanden lautlos:

- **`app/src/debug/AndroidManifest.xml`** deklariert `DebugHarnessReceiver`. Ohne die Datei ist die
  Klasse einkompiliert, aber in KEINEM Manifest deklariert → `am broadcast` läuft auf `Enqueued: 0`.
  Der Harness war dadurch **strukturell tot**, nicht wegen HyperOS (siehe Punkt 2).
- **`app/src/main/res/mipmap-*/ic_launcher*.png`** (15 Dateien) — Andis „K"-Icon wich kommentarlos
  dem Tauri-Default. Er hat es als „irgendwas ist mit meinen Commit-Ständen durcheinander" erlebt.

Beide sind am 2026-08-11 wiederhergestellt — **die Wiederherstellung ist aus demselben Grund wieder
flüchtig.** Fix: Dateien im versionierten `android/`-Baum beheimaten und beim Build mitsynchronisieren,
analog zu `android/kotlin-src/`. Nicht groß, aber es braucht eine Stelle im Build, die sie kopiert.

### 2. Conductor-Contract begründet den toten Harness falsch

`_bmad/custom/bmad-epic-conductor.toml` sagt, der Harness sei „dead on HyperOS (background
restrictions)". Er war nie HyperOS-bedingt tot, sondern manifest-los (Punkt 1) — der Beleg ist, dass er
nach dem Wiederherstellen des Manifests auf genau diesem HyperOS-Gerät einwandfrei lief. Der Satz
schickt die nächste Sitzung in die falsche Richtung. Korrigieren, sobald Punkt 1 dauerhaft ist.

### 3. Android-Skripte tragen WSL-/Laptop-Annahmen

- `scripts/android-build.sh`: Build gelingt, danach Abbruch beim Kopieren nach
  `/mnt/d/Dropbox/App Development/klarvo/releases/` — existiert auf powerhouse nicht. Die APK ist
  fertig, das Skript meldet trotzdem Fehler.
- `scripts/android-emulator.sh`: die `klarvo-emu`-AVD gibt es nur auf dem Laptop; hier hängt es
  endlos in `adb wait-for-device`.

### 4. `sync-and-build.ps1` ist repo-weit gebrochen (Windows-Build) — ✅ BEHOBEN 2026-08-17

> Behoben als Retro-Aktionspunkt AI-1 (`epic-11-retro-2026-08-17.md`), vor Story 8-5.

**Ursprüngliche Beschreibung (2026-08-11):** Zwei Ursachen, die sich gegenseitig verdecken:
`package-lock.json` enthält **keinen** Eintrag für `@tauri-apps/plugin-log` (`package.json` fordert
`^2`), sodass `npm ci` eine Version zieht, die gegen die Rust-Crate-Version mismatcht — und
`sync-and-build.ps1` fährt **nach** dem robocopy sein eigenes `npm install`, das jeden vorher
gesetzten Pin wieder abräumt. Für den 11-6-Gate mit expliziten `--no-save`-Pins plus npm-freiem
Skript umgangen; das ist keine Lösung. Fix: die Tauri-npm-Pakete im Lock pinnen und das
`npm install` im Skript hinter einen Schalter legen.

**Korrektur der Diagnose beim Fix.** `npm ci` zog keine falsche Version — es brach ab
(`EUSAGE: package.json and package-lock.json are not in sync`). Das Skript fuhr `npm install`, nicht
`npm ci`. Der gemessene Mechanismus: `@tauri-apps/plugin-log` fehlt im Lock → `npm install` löst
`^2` frisch auf → heute **2.9.0**, das `@tauri-apps/api ^2.11.0` fordert → hebt `api` gegen den auf
2.10.1 gepinnten Rest des Baums an. Die Kaskade über `api` ist der eigentliche Bruch, nicht
plugin-log allein.

**Die Regel, die dahinter steht:** JS-Plugin-Version == Rust-Crate-Version, exakt. Belegt an den
beiden Plugins, die korrekt im Lock stehen — `plugin-opener` 2.5.3 / `tauri-plugin-opener` 2.5.3,
`plugin-updater` 2.10.0 / `tauri-plugin-updater` 2.10.0. `tauri-plugin-log` ist 2.8.0, also gehört
JS auf 2.8.0.

**Fix:**
1. `package.json`: `@tauri-apps/plugin-log` exakt auf `2.8.0` gepinnt (kein `^`) — die Kopplung an
   die Rust-Crate steht damit sichtbar in der Datei, nicht nur im Lock.
2. `package-lock.json` regeneriert. Delta: genau ein neuer Eintrag (plugin-log 2.8.0) plus die
   veraltete Wurzel-Identität (`voxlit` 0.4.8 → `klarvo` 0.5.0). Kein anderes Paket bewegt sich.
3. `sync-and-build.ps1` fährt jetzt **`npm ci`** statt `npm install`. `npm ci` installiert den Lock
   wörtlich, schreibt ihn nie und bricht bei Drift laut ab — es kann einen Pin strukturell nicht
   mehr abräumen. Neuer Schalter `-SkipNpm` für unveränderte Deps (Vorbild: `dev.ps1`).

Derselbe Defekt saß in `scripts/dev.ps1` (gleiches `npm install` auf demselben Spiegelbaum) und ist
mitgefixt — sonst hätte der Dev-Build einen anderen Abhängigkeitsbaum als der Release-Build gehabt.

**Verifiziert (2026-08-17):**
- Linux: `npm ci` grün, `npm run build` (tsc strict + vite) grün.
- **Windows: `sync-and-build.ps1` läuft wieder end-to-end durch** — auf dem Laptop gegen `v1-ship`,
  ohne einen einzigen `--no-save`-Pin und ohne npm-freies Ersatzskript. `npm ci` in 37 s (der
  Puppeteer-Chromium-Cache bleibt erhalten, es gibt keinen Neu-Download), Rust-Release-Build sauber,
  `klarvo.exe` frisch (44,1 MB), rsign-Signatur „Signature verified OK", Installer nach Dropbox
  kopiert, „Done! Fresh build verified."
- Nach dem Windows-`npm ci` decken sich alle vier JS-Tauri-Pakete exakt mit ihren Rust-Crates, und
  `package-lock.json` im Spiegelbaum ist byte-identisch zur Quelle — der Pin ist strukturell nicht
  mehr abräumbar.

### 5. Messfallen (Doku, kein Code)

`monkey -p ... 1` injiziert nach dem Start einen *zufälligen* Tap · `adb shell am broadcast --es`
mit Leerzeichen im Text braucht Quoting des gesamten Remote-Kommandos · Harness-States sind
kleingeschrieben · `am force-stop` bringt das Onboarding zurück und verdeckt das Panel · ein
4-Sekunden-Kotlin-Build ist von einem No-op nicht unterscheidbar, ohne im *generierten* Baum zu
prüfen. Details in `gate4-evidence/11-6/verdict.md`.

---

## Android-Zwilling — Copy ohne Rückmeldung (2026-08-19, aus dem 8-5-Smoke)

Source: `_bmad-output/planning-artifacts/sprint-change-proposal-2026-08-19.md` · Auslöser: Andis
Windows-Smoke von Story 8-5 am 2026-08-19.

`android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` schreibt über den `ClipboardManager` in
die Zwischenablage, ohne dem Nutzer eine Bestätigung zu zeigen — **dieselbe Lücke, die Story 8.8 auf
dem Desktop schließt.**

**Warum getrennt:** Die bestehende Epic-Grenze lautet Epic 8 = Desktop, Epic 9 = Android. Andis Befund
kam vom Desktop-Verlauf. Der Kotlin-Zwilling gehört damit in **Epic 9**, nicht in 8.8 — die Grenze für
nichts zu überschreiten hätte die Epic-Struktur verwischt.

**Wenn es drankommt:** Zuerst prüfen, ob Copy auf Android überhaupt ein sichtbares Steuerelement hat
oder nur im Overlay-Cluster steckt. Die Desktop-Lösung („Copied" am Knopf) lässt sich nicht blind
übertragen; ein Toast oder eine kurze Cluster-Rückmeldung kann die passendere Form sein. Das ist eine
Design-Entscheidung für Andi, keine Umsetzungsfrage. Verwandt: ADR-0016 (Android-Pfad-Parität) — die
Rückmeldung ist geteiltes VERHALTEN, auch wenn die Form je Plattform verschieden ausfällt.
