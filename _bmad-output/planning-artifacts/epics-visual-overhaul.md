---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories", "step-04-final-validation"]
status: complete
inputDocuments:
  - docs/design/overhaul/SPEC-studio-dark-overhaul.md   # binding tokens/surfaces/states/constraints (the codeable contract)
  - docs/design/overhaul/01-product-brief.md            # values / audience (the "seriousness signal")
  - docs/design/overhaul/02-surfaces.md                 # surface inventory A–E
  - docs/design/overhaul/03-design-tokens-current.md    # current tokens (consolidation baseline)
  - docs/design/overhaul/04-constraints.md              # hard rendering / platform constraints
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-13.md  # epic split 8/9, ADR precursor, DoD
  - _bmad-output/project-context.md                     # code rules (camelCase, ADR-0015/0016, surface DoD)
  - docs/surface-smoke-checklist.md                     # surface-class DoD control
trackType: brownfield-visual-overhaul
featureEpics: [8, 9]
note: >
  Two-epic visual overhaul ("Studio Dark"). Brownfield-v1, NO PRD by design (same as Epics 5/6/7,
  each routed via correct-course off a requirements source). The requirements source is the tracked
  design spec docs/design/overhaul/SPEC-studio-dark-overhaul.md (+ 01..04). The handoff IS the UX
  spec — no separate bmad-create-ux-design run. Epic 8 = Desktop (Tauri/React/Tailwind v4), pure
  visual re-skin. Epic 9 = Android (native Kotlin, View+Canvas today, no Compose): token source
  + bubble interaction redesign (feature work, not re-skin). Epic 9 opens with a load-bearing ADR
  (View+Canvas vs ComposeView). Shares the sprint-status.yaml ledger. Per-story full context via
  bmad-create-story per session. IDs are native to this overhaul track.
---

# klarvo - Epic Breakdown (Visual Overhaul "Studio Dark" · Epics 8 + 9)

## Overview

A high-fidelity visual overhaul handoff ("Studio Dark") arrived from the cloud web-design agent and
is the binding direction: final color/type/spacing/radii/elevation/motion tokens, per-surface specs,
and an Android dictation-bubble **interaction** redesign. Two platforms share one design language:
**Windows-Desktop** (Tauri/WebView2 + React + Tailwind v4) and **Android** (native Kotlin; today
View + Canvas `onDraw`, no Compose).

The current-code gap that motivates the work:
- Desktop: `src/styles.css` has a ~69-line `@theme` block but **317 scattered inline hex** across
  components → the "not from one cast" feeling. Token consolidation has wide blast radius.
- Android: the overlay is **classic View + Canvas** (`FloatingBubbleView.kt` ~478 LOC,
  `KlarvoOverlayService.kt` ~1482 LOC) — **no Compose**, and **no Android color/theme file exists
  yet**. The bubble interaction (state sequence, listening panel, long-press popover) is genuinely
  new behavior, not a re-skin.

Two epics, separated by codebase / risk profile / human-test surface (deliberately not bundled):
- **Epic 8 — Desktop Visual Overhaul.** Foundation = token-ladder consolidation, then surface
  re-skins. Pure visual redesign; same function/IA/flows. Surface-class smoke DoD.
- **Epic 9 — Android Visual Overhaul + Bubble Interaction.** Opens with the rendering-tech ADR,
  then the Android token source, then bubble behavior (state sequence + listening panel + reactive
  waveform + long-press popover that remaps long-press from push-to-talk → menu). Hard IME
  constraints. On-device smoke DoD + a bubble state harness (verifiability symmetry).

## Requirements Inventory

Brownfield visual overhaul: requirements are extracted from `docs/design/overhaul/` (SPEC + 01..04).
IDs are native to this track. Categories: **DT** = design-token/system (the shared language),
**UX-DR** = per-surface visual requirements, **FR** = Android bubble interaction (the feature work),
**NFR** = non-functional/fidelity/DoD, **AR** = additional/architecture.

### Design Token & System Requirements (shared language — both platforms)

- **DT1** *(Desktop foundation)* — Consolidate the token ladder. Replace the ~69-line `@theme` block
  **and** the 317 scattered inline hex in `src/` with the Studio-Dark **named** tokens: graphite
  neutral ladder (`bg-deep #0A0B0C`, `bg #0F1112`, `surface #16181A`, `surface-2 #1B1E20`,
  `elevated #232729`, `border #282C2F`, `border-2 #353A3E`); text (`text #ECEEEF`, `muted #A4A9AC`,
  `dim #6F7479`, `faint #4B4F53`); teal (`teal #29C7AC`, `teal-hi #57DDC7`, `teal-lo #1B9C88`,
  `on-teal #05201B`); amber (`amber #E9A24C`, `amber-hi #F4BA72`); semantic (`danger #EE6F63`,
  `success #4FC58A`). Subtle/line variants via `color-mix`/rgba per spec. **No inline hex left for
  any covered role.**
- **DT2** *(Android foundation)* — Create the Android color/theme **source file** (none exists today)
  with the same named ladder as Kotlin `Color(0xFF…)` values per spec.
- **DT3** — Type system: **Geist** (UI, weights 400/500/600/700) + **Geist Mono** (dictation text,
  keys, IDs, timestamps). Scale (px): 11 label (uppercase, +8% tracking) · 12 · 13 · 14 · 16 · 20 ·
  28 · 40; LH 1.1–1.55. Desktop = Geist/Geist Mono **bundled locally** (no runtime CDN fetch —
  BYOK/no-phone-home, NFR6); **Android = bundled font resources**.
- **DT4** — Spacing (4-base: 2 4 6 8 12 16 20 24 32 40 48), radii (xs 6 · sm 8 · md 12 · lg 16 ·
  xl 20 · full), elevation (e1–e3 + pill, each + inset hairline; focus ring
  `0 0 0 3px rgba(41,199,172,.28)`), motion (micro 120 · state 180 · enter 240 spring
  `cubic-bezier(.34,1.56,.64,1)` · panel 320; standard ease `cubic-bezier(.2,0,0,1)`; respect
  `prefers-reduced-motion`). **Android: no native `backdrop-blur`** → solid `#16181A` +
  `Modifier.shadow`; the "glass ring" = a 4dp teal/amber ring, not real blur.
- **DT5** — Color **semantics** enforced everywhere: **teal** = brand / ready / processing / success
  / focus-ring; **amber** = live / listening (recording only — tally light); **danger/red** = stop /
  delete / error only; success (green) used sparingly.

### UX Design Requirements (Desktop surfaces — Epic 8)

- **UX-DR1** *(highest leverage)* — **FloatingBar** re-skin: transparent 200×36 overlay window;
  state sequence `idle` (invisible — pill materializes only on activity) → `recording` (glass pill,
  **amber** tally-light + **teal** waveform, spring-enter) → `transcribing` (teal spinner) → `done`
  (check). backdrop-blur 16px, 72% graphite fill, inset hairline. Stays **200×36** (no inflate),
  draggable, position persists. States designed as a sequence/transition, not static chrome.
- **UX-DR2** — **Settings Home + sub-pages**: color-coded icon badges, status dots, masked **mono**
  API keys, and a consistent high-quality form system — custom **Select / Segmented / Toggle /
  Slider** (native `<select>` removed; Linear/Raycast level). Categories: Recording & Audio · AI &
  Providers · Appearance · Language · Shortcuts · License · Dictionary.
- **UX-DR3** — **Live-Cleanup-Preview** re-skin (desktop-only): transparent panel, live **raw**
  transcript in mono, bottom-anchored; smooth expand/collapse; calm readable raw-vs-clean contrast.
  (Live LLM-*cleanup* stays off by design — quota; only the raw stream is live.)
- **UX-DR4** — **Main-Window / History**: better list density, **mono** timestamps, profile tags in
  **amber**, clear hierarchy, nice empty-states, better search/filter affordances.
- **UX-DR5** — **Onboarding** re-skin: trustworthy, elegant first impression (step indicators,
  illustration/empty); frame BYOK as a feature, not a hurdle. (Surface E.)

### Functional / Interaction Requirements (Android bubble — Epic 9; feature work)

- **FR1** — Bubble **idle**: one form across all states (**no** circle↔square morph) — a **teal-gradient
  squircle** (rounded square, not a circle) with a **dark "K"** centered, plus a subtle teal ring; **responsive
  size** `visual = clamp(36dp, 0.11 × min(screenW,screenH)dp, 44dp)`, touch target `max(visual, 48dp)` via
  transparent padding. *Visual values are anchored on the canon `docs/design/overhaul/source/` (`.ab-bubble.idle`
  in the HTML + `klarvo.css`), NOT transcribed here — read fill/shape/colors there.*
- **FR2** — **recording**: keyboard **collapses**, a **Klarvo-owned panel** rises (grab handle, K +
  **amber** live-dot + reactive waveform from RMS levels + timer + red stop). Live **raw** transcript
  runs multiline in the panel. Footer: "keyboard paused · returns on insert".
- **FR3** — **transcribing**: same panel, teal spinner + "Cleaning…", raw text dimmed.
- **FR4** — **done**: panel collapses, keyboard returns, **cleaned** text is in the field, bubble
  shows a brief check → idle.
- **FR5** — **Short-press = default gesture**, configurable: **Hold / Toggle / Auto-Stop / Auto** —
  the **same 4 modes as the desktop hotkey-mode** (mirror them).
- **FR6** — **Long-press = quick popover** (opens **inward**, never radial): block "default gesture"
  (4 modes) · "mode" (Polished / Verbatim / Chat) · row Target (field / clipboard) + Language
  (DE / EN / Auto) · footer "open settings". Two distinct axes: **gesture** (how triggered) vs
  **mode** (how cleaned). **This remaps long-press from today's push-to-talk → menu.**
- **FR7** — Bubble **anchoring** preserved: draggable, jumps up with the keyboard, edge-snap +
  remembered side.
- **FR8** — **In-app recording state** (`android-05`) re-skinned to the new design language.

### Non-Functional Requirements

- **NFR1** — **High-fidelity**: colors/type/spacing/radii/elevation/motion built pixel-accurate to
  spec; the two deliberate deviations (deeper/cooler graphite ladder; orange `#FFA344` → amber
  `#E9A24C`) are intentional and in-scope.
- **NFR2** — **Behavior-preserving**: Epic 8 is a **pure visual re-skin** (same function / IA /
  flows; no new features). Epic 9's bubble **interaction** change is in-scope, but the information
  architecture does not change.
- **NFR3** — **Desktop DoD** (surface-class): real **Windows release build** + **objective pixel
  metric** + observability loop + walk `docs/surface-smoke-checklist.md`. Human visual gate — **never
  make the user the rendering oracle** (observe-first; isolate the cause before changing app code).
- **NFR4** — **Android DoD**: on-device smoke (`scripts/android-smoke.sh`) **plus** a bubble **state
  harness** so each state (idle/recording/transcribing/done) is reachable for testing
  (verifiability symmetry — the user cannot otherwise produce the states).
- **NFR5** — Motion respects `prefers-reduced-motion` (desktop). Android renders glass effects as
  solid `#16181A` + `Modifier.shadow` (no native backdrop-blur).
- **NFR6** — **BYOK / privacy**: **no telemetry / tracking UI**. Real labels/providers only (Groq,
  DeepSeek, OpenAI, Anthropic, OpenRouter; verbatim/polished/chat) — no Lorem Ipsum.
- **NFR7** — Config keys are **camelCase** via the single-writer atomic path (ADR-0015). Any bubble
  change that reads/writes **shared** config keys (gesture/mode) must be **mirrored Rust ↔ Kotlin**
  (ADR-0016).

### Additional / Architecture Requirements

- **AR1** *(Epic 9 precursor — load-bearing)* — **ADR: Android bubble rendering tech** — extend the
  existing View + Canvas `onDraw` vs introduce a `ComposeView`. Must land **before** the bubble
  behavior stories; decides the implementation substrate for FR1–FR8.
- **AR2** — The **Android token/theme source** is a **new artifact** (none exists), plus bundled
  Geist / Geist Mono font resources (DT2/DT3).
- **AR3** — The **bubble state harness** is an explicit deliverable (NFR4): a dev-only path to drive
  the bubble through all four states on-device without needing live audio/network.
- **AR4** — **Token-consolidation blast radius**: DT1 touches every desktop surface (visual
  regression risk) → the desktop foundation story runs **first**, surfaces depend on it.
- **AR5** — **Hard IME constraints** (Android): (a) **no in-field preview text** from a
  `SYSTEM_ALERT_WINDOW` overlay — live raw text lives on Klarvo's own surface (the listening panel),
  the foreign field is written **only finally** (a11y `ACTION_SET_TEXT` or clipboard+paste);
  (b) **keyboard-collapse during recording** is via the a11y service — **optional, with fallback**
  (keep the keyboard open), not a default for all apps; (c) touch targets **≥ 48dp**; (d) fixed
  **56px nav-bar clearance** (`env(safe-area-inset-bottom)` is unreliable/0 in the Android WebView;
  native Kotlin handles its own insets).
- **AR6** — **FloatingBar transparent-window constraint** (Desktop): `html/body/#root` must stay
  `background: transparent` (else WebView2 paints its default behind the pill). Window size is set in
  **Rust at creation** (the frontend does **not** call `setSize`); if the redesign needs other
  dimensions, give an **explicit** value and keep it small (it is an overlay, not a panel).

### Requirements Coverage Map

| Req | Epic | Note |
|---|---|---|
| DT1 | 8 | Desktop token consolidation (foundation, blast radius) |
| DT3 (desktop), DT4 (desktop), DT5 | 8 | Type / spacing / radii / elevation / motion + semantics |
| UX-DR1 | 8 | FloatingBar re-skin |
| UX-DR2 | 8 | Settings form system + Home/sub-pages |
| UX-DR3 | 8 | Live-Cleanup-Preview re-skin |
| UX-DR4 | 8 | Main-Window / History re-skin |
| UX-DR5 | 8 | Onboarding re-skin (last/optional — see decision D1) |
| AR4, AR6 | 8 | Token blast-radius ordering; transparent-window constraint |
| NFR1, NFR2, NFR3, NFR5 (desktop), NFR6, NFR7 (desktop camelCase) | 8 | Fidelity / behavior-preserving / desktop DoD / reduced-motion / BYOK / config |
| AR1 | 9 | Precursor ADR: bubble rendering tech |
| DT2, DT3 (android), DT4 (android), AR2 | 9 | Android token/theme source + fonts (foundation) |
| DT5 | 9 | Color semantics (shared; applied on Android too) |
| FR1, FR7 | 9 | Bubble idle re-skin + responsive sizing + anchoring |
| FR2, FR3, FR4 | 9 | Bubble state sequence + listening panel + RMS waveform |
| AR3, NFR4 | 9 | Bubble state harness + on-device DoD |
| AR5 | 9 | Hard IME constraints (no in-field preview; optional keyboard-collapse; ≥48dp; nav-bar clearance) |
| FR5 | 9 | Short-press 4 gesture modes (mirror desktop) |
| FR6 | 9 | Long-press popover menu (remap from push-to-talk) |
| FR8 | 9 | In-app recording state re-skin (see decision D2) |
| NFR1, NFR2, NFR5 (android), NFR6, NFR7 (mirroring) | 9 | Fidelity / behavior / no-blur / BYOK / Rust↔Kotlin mirror |

**Scope decisions baked in (confirmable at the approval gate):**
- **D1 — Onboarding (UX-DR5):** *kept* in Epic 8 as the **last, lowest-priority** story (it is a real
  surface in `02-surfaces.md`, but was not among the four surfaces the proposal named). Droppable
  without affecting the other surfaces.
- **D2 — Android in-app recording state (FR8):** *kept* in Epic 9 as a **small re-skin story**,
  separate from the bubble interaction work (named in `02/04`; the bubble is the headline).
- **D3 — Light theme:** **out of scope.** The constraint says dark is the identity, light only ever
  optional — not part of this overhaul.

## Epic List

### Epic 8: Desktop Visual Overhaul ("Studio Dark")

The user sees a coherent, high-fidelity, instrument-grade dark UI across **every** desktop surface —
identical functions and flows, but now "from one cast": the scattered inline-hex look is gone, the
FloatingBar feels premium, and Settings has a real form system. Standalone: builds only on existing
v1 desktop surfaces; enables nothing downstream but retires the visual-inconsistency debt.

**Covers:** DT1, DT3/DT4 (desktop), DT5, UX-DR1–UX-DR5, AR4, AR6, NFR1, NFR2, NFR3, NFR5 (desktop),
NFR6, NFR7 (desktop).

**Planned story decomposition** (full ACs in Step 3):
- **8.1 — Token & type foundation** *(Wave 1, foundation; AR4)*. Define the Studio-Dark named `@theme`
  token block + type/spacing/radii/elevation/motion primitives, and **bundle Geist / Geist Mono
  locally** (no CDN fetch — NFR6). It does **not** blindly swap all 317 inline hex: a hex becomes a
  *named* token only once its semantic role is decided, and that decision lives with the surface — so
  the hex→token migration is done **per-surface** (8.2–8.6) where it is visually smoke-testable. 8.1
  may do a mechanical pass only for unambiguous global hex. (DT1 [definitions], DT3, DT4, DT5)
- **8.2 — Settings form system + Home/sub-pages** *(after 8.1)*. Custom Select/Segmented/Toggle/Slider
  (native `<select>` removed), color-coded icon badges, status dots, masked mono keys. (UX-DR2)
- **8.3 — FloatingBar re-skin** *(after 8.1)*. State sequence idle→recording→transcribing→done; glass
  pill, amber tally + teal waveform, spring motion; transparent-window constraint preserved; 200×36.
  Surface-class DoD (AR6, NFR3). (UX-DR1)
- **8.4 — Live-Cleanup-Preview re-skin** *(after 8.1)*. Transparent panel, mono raw transcript,
  bottom-anchored, smooth expand/collapse. (UX-DR3)
- **8.5 — Main-Window / History re-skin** *(after 8.1)*. List density, mono timestamps, amber profile
  tags, empty-states, search/filter affordances. (UX-DR4)
- **8.6 — Onboarding re-skin** *(after 8.1; last/optional — D1)*. Trustworthy first impression, step
  indicators, BYOK-as-feature framing. (UX-DR5)

**Dependency flow:** 8.1 first (foundation). 8.2–8.6 each depend only on 8.1 and are otherwise
parallel. Each surface story migrates its own inline hex → named tokens (DT1 [application]); a final
`grep` gate ("no inline hex left for covered roles") rides the **last** surface story to close DT1.
No story depends on a later story. **Risk note:** 8.3 (FloatingBar) is the same transparent-overlay
risk class that burned four build cycles in Epic 6 — it carries the hardest observe-first discipline
(isolate the cause before changing app code; never make the user the rendering oracle, NFR3).

### Epic 9: Android Visual Overhaul + Bubble Interaction

The Android app shares the Studio-Dark design language, **and** the dictation bubble gains the spec'd
behavior: a state sequence with a Klarvo-owned listening panel + reactive waveform, and a long-press
popover menu (the gesture/mode hub). Unlike Epic 8 this is partly real interaction work, not just a
re-skin. Standalone: builds on the existing v1 Android overlay; depends on no other epic.

**Covers:** AR1, AR2, AR3, AR5, DT2, DT3/DT4 (android), DT5, FR1–FR8, NFR1, NFR2, NFR4, NFR5
(android), NFR6, NFR7 (mirroring).

**Planned story decomposition** (full ACs in Step 3):
- **9.1 — ADR: bubble rendering tech** *(precursor, first; AR1)*. Decide extend View+Canvas vs
  introduce ComposeView. **A genuine gate, not a formality** — ComposeView is a far larger substrate
  change than extending Canvas, so the whole Epic-9 effort estimate swings on this decision.
  (Output: an ADR, not UI.)
- **9.2 — Android token/theme source + fonts** *(foundation, after 9.1; AR2)*. Create the Android
  color/theme file (none exists) + bundle Geist/Geist Mono. (DT2, DT3, DT4, DT5)
- **9.3 — Bubble idle re-skin + responsive sizing + anchoring** *(after 9.2)*. Teal K, glass ring
  (4dp), responsive size clamp, ≥48dp touch target, drag/edge-snap/remembered-side. (FR1, FR7, AR5c/d)
- **9.4 — Bubble state harness** *(after 9.3, BEFORE the states; AR3, NFR4)*. A dev-only path to drive
  the bubble through idle/recording/transcribing/done on-device without live audio/network.
  **Sequenced before 9.5 deliberately** — per the verifiability-symmetry rule the states must be
  reachable for test *before* they are built, else 9.5 ships states no one (not even Andi) can
  reproduce to verify.
- **9.5 — Bubble state sequence + listening panel + waveform** *(the big one; after 9.4)*.
  idle→recording→transcribing→done; Klarvo-owned panel (grab handle, amber live-dot, RMS waveform,
  timer, red stop); live **raw** transcript in-panel (IME constraint AR5a); final insert via
  a11y/clipboard. (FR2, FR3, FR4)
- **9.6 — Keyboard-collapse via a11y service** *(after 9.5; optional, own story — AR5b)*. Split out of
  9.5 because it is explicitly optional, per-app-fragile, and carries its own fallback (keep the
  keyboard open). Bundling it into 9.5 would couple a fragile optional behavior to the core state work.
- **9.7 — Short-press gesture modes (mirror desktop)** *(after 9.5)*. Hold/Toggle/Auto-Stop/Auto;
  shared config keys mirrored Rust↔Kotlin. (FR5, NFR7)
- **9.8 — Long-press popover menu** *(after 9.5)*. Inward popover: gesture (4) · mode (Polished/
  Verbatim/Chat) · target + language · settings link; **remaps long-press from push-to-talk → menu**.
  (FR6)
- **9.9 — In-app recording state re-skin** *(after 9.2; small — D2)*. Re-skin `android-05` to the new
  language. (FR8)
- **9.10 — Token codegen: `klarvo.css` → `KlarvoTheme.kt`** *(post-ADR-0019 insertion; sequenced
  BEFORE the 9.5 rebuild; ADR-0019 Decision #2)*. Replace the hand-typed `KlarvoTheme.kt` (the Token-
  Drift surface — proven by the 9.5-F6 AmberLine `.30→.32` copy-error) with a generator that projects
  the canon `--k-*` custom properties into Kotlin token constants, plus a build/CI drift gate so
  hand-edited values can no longer merge. Mechanical, highest leverage, cheap (ADR-0019 §Mitigations
  ordering #1). Foundation for the 9.5 rebuild (the recording state must render against real SSOT
  tokens, not a drifting copy).

**Dependency flow:** 9.1 (gate) → 9.2 (foundation) → 9.3 → **9.4 (harness) → 9.5 (states)** →
{9.6, 9.7, 9.8}; 9.9 after 9.2 (parallel with bubble work). The **harness-before-states** ordering is
load-bearing (verifiability symmetry). **Post-ADR-0019:** 9.10 (token codegen) sequences before the
9.5 *rebuild* (which is re-fashioned against the extended canon — `.ab-bubble.recording`, danger=cancel,
bubble-tap=send). No story depends on a later story.

---

## Epic 8: Desktop Visual Overhaul ("Studio Dark")

The user sees a coherent, high-fidelity dark UI across every desktop surface — identical functions and
flows, now "from one cast". Foundation first, then per-surface re-skins, each gated by a surface-class
smoke.

### Story 8.1: Token & type foundation

As a developer establishing the Studio-Dark design language,
I want the named token block, the type/spacing/radii/elevation/motion primitives, and locally bundled fonts in place,
So that every surface story re-skins against one source of truth instead of scattered hex.

**Acceptance Criteria:**

**Given** `src/styles.css`
**When** the `@theme` block is rewritten
**Then** it defines exactly the Studio-Dark named tokens (graphite ladder bg-deep/bg/surface/surface-2/
elevated/border/border-2; text/muted/dim/faint; teal/teal-hi/teal-lo/on-teal; amber/amber-hi;
danger/success) at the spec hex values
**And** the subtle/line variants (teal/amber/danger bg+line, glass hairline) exist as documented
`color-mix`/rgba utilities.

**Given** the type system
**When** primitives are added
**Then** Geist (400/500/600/700) and Geist Mono are **bundled as local assets** with no Google-Fonts
CDN / no runtime network fetch (NFR6), exposed as font-family tokens
**And** the px scale (11–40), line-heights, and the 11px-label uppercase+tracking rule are available as
utilities.

**Given** spacing / radii / elevation / motion
**When** primitives are added
**Then** the 4-base spacing, radii (xs6…xl20+full), elevation (e1–e3 + pill with inset hairline), the
teal focus ring, and the motion durations/eases are defined as tokens/utilities
**And** `prefers-reduced-motion` is honored by the motion utilities (NFR5).

**Given** the foundation is in place
**When** an existing surface renders unchanged
**Then** the app still builds and runs (tsc/vite green) — 8.1 introduces the **vocabulary**, not the
per-surface re-skin; no surface is migrated here beyond unambiguous global hex.

**And** inversion: a build that fetches Geist from a remote URL at runtime violates NFR6 and must fail
review.

**DoD:** `tsc`/`vite` green; `cargo check --target x86_64-pc-windows-gnu` green; fonts confirmed to
load offline in the Windows build (no font network request).

### Story 8.2: Settings form system + Home & sub-pages

As a user configuring Klarvo,
I want a consistent, high-quality settings form system across Home and every sub-page,
So that configuration feels instrument-grade instead of stock OS widgets.

**Acceptance Criteria:**

**Given** the Settings Home
**When** it renders
**Then** the categories (Recording & Audio, AI & Providers, Appearance, Language, Shortcuts, License,
Dictionary) appear as rows with color-coded icon badges + status dots, using only named tokens.

**Given** any sub-page with form controls
**When** it renders
**Then** every native `<select>` is replaced by the custom Select; toggles, sliders, and
segmented-controls use the new components; API keys render masked in Geist Mono.

**Given** a control bound to config
**When** the user changes and saves it
**Then** the existing value round-trips correctly through the new control (camelCase key, `save_config_locked`,
ADR-0015) with **no stuck-dirty** state — the new control is wired into the settings resync `useEffect`
(known trap).

**Given** the AI & Providers page
**When** providers/keys are shown
**Then** real provider labels are used (Groq, DeepSeek, OpenAI, Anthropic, OpenRouter) and **no
telemetry/tracking UI** exists (NFR6).

**And** the Settings surfaces carry **zero inline hex** for covered roles (DT1 application), and controls
are keyboard-operable with the teal focus ring.

**DoD:** Windows release build + settings smoke (each control type renders + an existing setting still
round-trips, camelCase key in `config.json`); walk `docs/surface-smoke-checklist.md`; `tsc`/`vite` +
`cargo check` win-target green.

### Story 8.3: FloatingBar re-skin

As a user dictating,
I want the FloatingBar to look premium and read its state clearly while staying tiny and transparent,
So that the most-seen surface signals quality without competing for attention.

**Acceptance Criteria:**

**Given** the bar window
**When** idle
**Then** it is invisible (the pill materializes only on activity) and the window background stays
transparent (`html/body/#root` transparent — AR6).

**Given** recording starts
**When** the pill appears
**Then** it renders the glass pill (backdrop-blur 16px, 72% graphite fill, inset hairline) with an
**amber** tally-light + **teal** waveform, entering via the spring motion; the visible pill stays
**200×36** (no inflate), draggable, position persisted.

**Given** the pipeline progresses
**When** state → transcribing → done
**Then** the pill shows a teal spinner then a check, following the state sequence; amber appears **only**
while recording (DT5).

**Given** the redesign needs no new dimensions
**When** the window is created
**Then** the size is still set in **Rust at creation** (the frontend does not call `setSize`); any dim
change is an explicit small value (AR6).

**And** the FloatingBar carries zero inline hex for covered roles (DT1).

**DoD (surface-class, hardest):** real Windows release build + an **objective pixel metric** (e.g.
measured tally color / fill opacity / blur presence) + observability loop; any rendering artifact is
**isolated and named before** any app-code change — never make the user the rendering oracle (NFR3);
walk `docs/surface-smoke-checklist.md` (transparent window, geometry/region).

### Story 8.4: Live-Cleanup-Preview re-skin

As a user reading along while dictating,
I want the live preview panel to present the raw transcript calmly and legibly,
So that I can orient on what's being captured without distraction.

**Acceptance Criteria:**

**Given** preview is enabled and recording
**When** raw chunks arrive
**Then** the live **RAW** transcript renders in Geist Mono on the transparent, bottom-anchored panel
using named tokens (live LLM cleanup stays off by design — only the raw stream is live).

**Given** the panel opens/closes
**When** expand/collapse fires
**Then** the motion uses the panel duration/ease and is smooth; `prefers-reduced-motion` honored.

**Given** the dark-background legibility issue
**When** text renders
**Then** the raw text is clearly legible (brightness/contrast resolves the known dim-text trap).

**Given** the preview is a **separate window**
**When** any appearance value is config-driven
**Then** it is re-read **reactively** (re-read on open / backend event), never frozen at app-start
(separate-window trap).

**And** zero inline hex for covered roles (DT1).

**DoD:** Windows release build + smoke (preview opens during recording, raw text legible, smooth
collapse); separate-window reactivity checked; `tsc`/`vite` + `cargo check` win-target green.

### Story 8.5: Main-Window / History re-skin

As a user reviewing past dictations,
I want the history list and main window to have clear hierarchy and pleasant density,
So that I can scan and find past dictations easily.

**Acceptance Criteria:**

**Given** the History list
**When** it renders
**Then** list density improves, timestamps render in Geist Mono, profile tags render in **amber**, and
hierarchy uses the type-scale + spacing tokens.

**Given** an empty history or no-match filter
**When** nothing matches
**Then** a designed empty-state renders (not a bare blank).

**Given** search/filter
**When** used
**Then** the affordances are clear and styled with named tokens.

**Given** real content
**When** shown
**Then** real dictation text + real labels are used (no Lorem Ipsum).

**And** zero inline hex for covered roles (DT1).

**DoD:** Windows release build + smoke (list density, empty-state, search/filter); `tsc`/`vite` +
`cargo check` win-target green.

### Story 8.6: Onboarding re-skin (last surface; optional — D1)

As a first-time user,
I want an elegant, trustworthy onboarding,
So that the seriousness of the tool is clear and BYOK feels like a feature, not a hurdle.

**Acceptance Criteria:**

**Given** first launch
**When** onboarding renders
**Then** steps use the new type/spacing/tokens with clear step indicators, and API-key/provider setup is
framed as a feature (BYOK), using real provider labels.

**Given** the flow runs
**When** the user completes it
**Then** behavior and IA are unchanged (re-skin only) and the end state matches today's.

**Given** this is the **final** surface story
**When** 8.6 is done
**Then** a closing `grep` gate asserts **no inline hex remains for covered roles** across `src/` (DT1
closure).

**And** zero inline hex for covered roles in onboarding itself (DT1).

**DoD:** Windows release build + smoke (walk the onboarding flow); the DT1 closing grep-gate is green;
`tsc`/`vite` + `cargo check` win-target green.

---

## Epic 9: Android Visual Overhaul + Bubble Interaction

The Android app shares the Studio-Dark language, and the dictation bubble gains the spec'd behavior:
a state sequence with a Klarvo-owned listening panel + reactive waveform, and a long-press popover
menu. Opens with a load-bearing rendering ADR; the state harness lands before the states it verifies.

### Story 9.1: ADR — Android bubble rendering tech (precursor gate)

As an architect,
I want a decision on whether to extend the existing View+Canvas overlay or introduce a ComposeView,
So that all bubble stories build on a settled substrate and the epic estimate is honest.

**Acceptance Criteria:**

**Given** the current overlay (`FloatingBubbleView.kt` View+Canvas, `KlarvoOverlayService.kt`)
**When** the ADR is written
**Then** it records the decision (extend View+Canvas vs introduce ComposeView inside the
`SYSTEM_ALERT_WINDOW` overlay) with rationale covering: motion needs (state sequence + spring), the
listening-panel composition, RMS-waveform rendering, the risk of mixing Compose into an overlay
service, and the effort delta.

**Given** the decision
**When** recorded
**Then** it lands as `docs/adr/00NN-*.md` per the ADR convention + index update and is referenced by the
Epic 9 stories.

**And** the ADR names the verifiability-symmetry implication — how the chosen substrate supports the
9.4 state harness.

**DoD:** ADR committed (own commit per ADR convention). No code.

### Story 9.2: Android token/theme source + fonts (foundation)

As a developer,
I want the Android color/theme source and bundled fonts created,
So that the bubble and in-app surfaces re-skin against named tokens like the desktop.

**Acceptance Criteria:**

**Given** no Android theme file exists today
**When** 9.2 is done
**Then** a Kotlin token source defines the Studio-Dark ladder as `Color(0xFF…)` per spec
(Bg/Surface/Surface2/Elevated/Border/Border2/TextC/Muted/Dim/Teal/TealHi/TealLo/OnTeal/Amber/Danger).

**Given** fonts
**When** bundled
**Then** Geist + Geist Mono ship as font resources (no runtime fetch) and typography matches the scale.

**Given** Android has no native backdrop-blur
**When** glass surfaces are specified
**Then** the tokens encode solid `#16181A` + `Modifier.shadow` and the 4dp-ring approach (DT4 android).

**And** the teal/amber/danger color semantics are documented for Android use (DT5).

**DoD:** Android builds; on-device smoke that the app launches with the new theme on ≥1 reference
surface; APK freshness verified via `scripts/android-build.sh` timestamp gate (no in-UI version).

### Story 9.3: Bubble idle re-skin + responsive sizing + anchoring

As a user with a focused text field,
I want the idle bubble to look right and sit correctly at any screen size,
So that it's reachable and unobtrusive.

**Acceptance Criteria:**

**Given** a focused field + open keyboard
**When** the bubble shows idle
**Then** it renders a **teal-gradient squircle** (rounded square, 12px-equivalent corner radius — NOT a circle)
with a **dark "K"** (OnTeal) centered and a subtle teal ring, per the canon `.ab-bubble.idle`
(`docs/design/overhaul/source/`); the **same form** is used across states (no circle↔square morph).

**Given** varying screen sizes
**When** sized
**Then** `visual = clamp(36dp, 0.11 × min(screenW,screenH)dp, 44dp)` and the touch target =
`max(visual, 48dp)` via transparent padding (AR5c).

**Given** the bubble
**When** dragged
**Then** it is draggable, edge-snaps, remembers its side, and jumps up with the keyboard (FR7); nav-bar
clearance uses fixed px, not `env(safe-area-inset-bottom)` (AR5d).

**DoD:** on-device smoke (bubble appears on field focus, correct size on a real phone, drag/snap/side-
memory work); APK freshness verified.

### Story 9.4: Bubble state harness (verifiability precursor — before 9.5)

As a developer (and as Andi the human tester),
I want a dev-only way to drive the bubble through all four states on demand,
So that the upcoming state UI is verifiable without live audio/network — built BEFORE the states.

**Acceptance Criteria:**

**Given** a dev/debug entry point
**When** invoked
**Then** the bubble can be put into idle / recording / transcribing / done deterministically on-device,
with synthetic RMS levels + synthetic raw-transcript text feeding the panel.

**Given** the harness
**When** used by the human tester
**Then** **Andi can reproduce each state himself** (verifiability symmetry — the gate Andi must pass is
reachable by Andi, not only the agent).

**Given** release builds
**When** shipped
**Then** the harness is dev-only / gated out (no user-facing surface, no telemetry).

**And** the harness exists and is demonstrated **before** Story 9.5 begins (sequencing gate).

**DoD:** on-device demonstration that all four states + the waveform/transcript can be triggered via the
harness.

### Story 9.5: Bubble state sequence + listening panel + waveform (the big one)

As a user dictating from a text field,
I want the bubble to run idle→recording→transcribing→done with a Klarvo-owned listening panel,
So that I see live feedback and the cleaned text lands in my field.

**Acceptance Criteria:**

**Given** recording starts
**When** the panel rises
**Then** a Klarvo-owned panel shows a grab handle, K + amber live-dot, a reactive RMS waveform, a timer,
and a **red square = Abbrechen** (cancel/discard, parity with desktop); the footer reads "keyboard
paused · returns on insert".

**And** the bubble stays visible in its **recording state** (`.ab-bubble.recording`: teal squircle +
amber pulse-ring + send-glyph, NOT the idle K); **tapping the bubble = Senden** (stop → transcribe →
paste). Confirm (bubble-tap) and Cancel (red square) are distinct affordances; **red is never the
send/confirm action** (ADR-0019 colour-semantics rule).

**Given** recording
**When** raw text streams
**Then** the live **RAW** transcript runs multiline **in the panel** — NOT in the foreign field (AR5a:
a `SYSTEM_ALERT_WINDOW` overlay cannot set composing text; only a final write is possible).

**Given** transcribing
**When** cleanup runs
**Then** the same panel shows a teal spinner + "Cleaning…" with the raw text dimmed.

**Given** done
**When** complete
**Then** the panel collapses, the keyboard returns, the **cleaned** text is written to the field (a11y
`ACTION_SET_TEXT` or clipboard+paste), and the bubble shows a brief check → idle.

**And** the states are verified via the 9.4 harness; inversion: attempting in-field live preview text
from the overlay is impossible by AR5a and must not be claimed as done.

**DoD:** on-device smoke (real end-to-end dictation in a 3rd-party app: panel states, reactive waveform,
cleaned text lands in a real field) via `scripts/android-smoke.sh`.

### Story 9.6: Keyboard-collapse via a11y service (optional, own story — AR5b)

As a user who wants the keyboard out of the way during dictation,
I want an optional setting to collapse the keyboard while recording,
So that the listening panel has room — with a safe fallback when it's unreliable.

**Acceptance Criteria:**

**Given** the option is OFF (default)
**When** recording
**Then** the keyboard stays as-is (fallback = keep keyboard open) — no behavior change for non-opt-in
users.

**Given** the option is ON
**When** recording starts
**Then** the a11y service dismisses the IME while keeping the target field focused, and the final insert
still works.

**Given** a per-app case where dismiss fails
**When** recording
**Then** it degrades gracefully to keyboard-open (no broken state).

**And** the toggle is a camelCase config key via `save_config_locked` (ADR-0015); if shared, mirrored
Rust↔Kotlin (ADR-0016 / NFR7).

**DoD:** on-device smoke on ≥2 apps (one where collapse works, one fallback path) via
`scripts/android-smoke.sh`.

### Story 9.7: Short-press gesture modes (mirror desktop)

As a user,
I want short-press to support the same four gesture modes as the desktop hotkey,
So that triggering dictation is consistent across platforms.

**Acceptance Criteria:**

**Given** settings
**When** the user picks a default gesture
**Then** Hold / Toggle / Auto-Stop / Auto are available — the **same four** modes as the desktop
hotkey-mode (FR5).

**Given** a short-press
**When** it fires
**Then** it behaves per the selected mode.

**Given** this is shared behavior
**When** stored
**Then** the config key is camelCase and **mirrored Rust↔Kotlin** (NFR7/ADR-0016); silence/auto-stop
thresholds reuse the existing **mode-centric** fields (avoid the Android silence-field divergence).

**DoD:** on-device smoke (each mode triggers correctly); config round-trip verified; `android-smoke.sh`.

### Story 9.8: Long-press popover menu (remap from push-to-talk)

As a user,
I want long-press to open a quick popover instead of push-to-talk,
So that I can switch gesture/mode/target/language without opening full settings.

**Acceptance Criteria:**

**Given** the bubble
**When** long-pressed
**Then** a popover opens **inward** (never radial) with: a "default gesture" block (4 modes), a "mode"
block (Polished/Verbatim/Chat), a row for Target (field/clipboard) + Language (DE/EN/Auto), and a
footer "open settings" (FR6).

**Given** long-press previously triggered push-to-talk
**When** 9.8 lands
**Then** long-press is **remapped** to the menu; push-to-talk remains reachable via the short-press
gesture modes (no capability lost).

**Given** the two axes
**When** the user changes them
**Then** gesture (how triggered) and mode (how cleaned) are independent and persist (camelCase, mirrored
where shared).

**And** the popover uses tokens with ≥48dp touch targets.

**DoD:** on-device smoke (long-press opens the menu, selections persist + take effect, short-press still
dictates) via `scripts/android-smoke.sh`.

### Story 9.9: In-app recording state re-skin (small — D2)

As a user recording inside the app,
I want the in-app recording surface to match the new design language,
So that the app is visually consistent end-to-end.

**Acceptance Criteria:**

**Given** the in-app recording state (`android-05`)
**When** it renders
**Then** it uses the new tokens/type/motion; behavior and IA are unchanged (re-skin only).

**And** no hardcoded colors for covered roles remain in this surface (DT closure for the Android in-app
surface).

**DoD:** on-device smoke (in-app recording visual) via the build/smoke scripts; APK freshness verified.

### Story 9.10: Token codegen — `klarvo.css` → `KlarvoTheme.kt` (post-ADR-0019; before the 9.5 rebuild)

As a developer maintaining two platform implementations of one design,
I want the Android token file generated from the canon CSS rather than hand-typed,
So that the token layer cannot structurally drift (closing the F6 class of copy-errors) and the 9.5
rebuild renders against the real single-source-of-truth.

**Acceptance Criteria:**

**Given** the canon `docs/design/overhaul/source/assets/klarvo.css` holds the `--k-*` custom properties
**When** the generator runs
**Then** it emits `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` with every canon color token as a
Kotlin constant, with hex `#RRGGBB` → `0xFFRRGGBB` and `rgba(r,g,b,a)` → `0xAARRGGBB` (alpha = round(a×255)),
and **no canon-derived hex is hand-typed** anywhere in platform code.

**Given** the current consumers (`FloatingBubbleView.kt`, `ListeningPanelView.kt`) reference identifiers
like `KlarvoTheme.TextC`, `Border2`, `AmberLine`, `TealBg`
**When** the file is regenerated
**Then** every currently-referenced identifier still resolves with a **byte-identical color value**
(zero visual regression) — an explicit alias map preserves non-mechanical names (e.g. `--k-text` → `TextC`).

**Given** the alpha conversion
**When** the file is generated
**Then** `AmberLine == 0x52E9A24C`, `TealBg == 0x1F29C7AC`, `DangerBg == 0x1FEE6F63` (the F6 class is
produced correctly by the rule, not by hand).

**Given** someone hand-edits a generated token value
**When** the build/smoke flow runs
**Then** a **drift gate** (regenerate to temp + diff against the committed file) fails the build with a
clear "KlarvoTheme.kt drifted from canon — re-run the generator" message.

**And** canon color tokens absent from today's hand-written file (`--k-bg-deep`, `--k-hairline`,
`--k-faint`, `--k-teal-line`, `--k-success`, `--k-info`) are added, so the file is a complete projection
of the canon color set.

**DoD:** generator + drift gate wired into `scripts/android-smoke.sh` (and `scripts/android-build.sh`)
**before** the `kotlin-src` sync; the 60 JVM unit tests still pass; the DEBUG APK builds. **No pixel
changes** (values are byte-identical to today) → the human visual gate is consciously downgraded to an
optional sanity glance; the binding gate is the byte-identity assertion + the drift check (machine-verifiable).

---

_Visual-overhaul planning artifact (Epics 8 + 9). Codeable contract:
`docs/design/overhaul/SPEC-studio-dark-overhaul.md` (+ 01..04). Per-story full context via
`bmad-create-story` per session._
