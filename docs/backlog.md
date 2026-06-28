# klarvo — Backlog (deferred-work SSOT)

Single source of truth for deferred work. Every scope-cut / phase-deferral lands here with a source ref
(per backlog-discipline). Created 2026-06-10 by `bmad-correct-course` off the cross-platform drift audit.

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
