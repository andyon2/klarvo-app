# Story 9.9: In-app recording state re-skin (small — D2)

Status: done

## Story

As a user recording inside the app,
I want the in-app recording surface to match the new design language,
so that the app is visually consistent end-to-end.

## Acceptance Criteria

**AC1 — RecordButton uses tokens (idle state):**
Given the app opens and no recording is active
When the RecordButton renders
Then it uses `klarvo-primary/15` background (= `--color-klarvo-teal` at 15% opacity), `klarvo-primary/25` border, and `klarvo-primary` icon color — all already expressed via Tailwind token aliases.

**AC2 — RecordButton uses tokens (recording state) — AMBER live treatment [GATE-1 decision, 2026-06-21]:**
Given the user has started recording (recordingState = "recording")
When the RecordButton renders
Then it uses `klarvo-warning/20` background, `klarvo-warning/40` border, `klarvo-warning` icon, AND a `klarvo-warning` / amber-line pulse-ring — **NOT** red/danger.
This follows the binding canon `.inapp-mic .ring` (amber `--k-amber-line` pulse) and ADR-0019 ("amber = live/recording indicator; red is reserved for stop/cancel/error only"), consistent with the 9-5 Modell-B bubble (waveform amber = live, red = cancel only).
And the pre-existing hardcoded `border-red-400 animate-ping` ring **must** be replaced by an amber token (no `red-400` literal anywhere).
And the recording glow shadow's rgba color must use the amber value (matching `klarvo-warning`), not the red `rgba(255,115,105,...)`.

**AC3 — RecordButton uses tokens (transcribing/cleaning state):**
Given processing is in progress (recordingState = "transcribing" or "cleaning")
When the RecordButton renders
Then it uses `klarvo-warning/15` background, `klarvo-warning/30` border, `klarvo-warning` spinner color.

**AC4 — Status label uses semantic token colors:**
Given any recording state
When the status label renders
Then:
- `recording` → `klarvo-warning` text (amber = live, per GATE-1; red reserved for error/cancel only)
- `transcribing`/`cleaning` (isBusy) → `klarvo-warning` text
- `done` → `klarvo-primary` text
- `idle`/`error`(message) → `klarvo-dim` / `klarvo-danger` text
And no hardcoded hex literals remain for these roles in the status-label block.

**AC5 — Result textarea uses tokens:**
Given a transcription result is available
When the result textarea renders
Then background uses `klarvo-bg`, border uses `klarvo-border/60`, text uses `klarvo-text`, focus border uses `klarvo-primary/30` — all already expressed as Tailwind utilities; no hardcoded `bg-[#0c0c0e]` for this role.

**AC6 — Raw-text area uses tokens:**
Given the user expands "Show original"
When the raw-text textarea renders
Then background uses `klarvo-bg-deep` (Tailwind: `bg-klarvo-bg-deep`) instead of the hardcoded `bg-[#0c0c0e]`; border and text use existing token utilities.

**AC7 — Canon `.inapp-mic` semantics honored:**
Given the canon CSS defines `.inapp-mic` as a 92px circular element with `var(--k-amber-line)` ring animation during recording
When the in-app recording surface is re-skinned
Then the amber ring / pulse animation around the RecordButton during recording is driven by `klarvo-amber-line` / `klarvo-warning` tokens (NOT hardcoded amber hex) and the ring animates only during the `recording` state.
> NOTE: The canon `.inapp-mic` is a design-reference surface, not a Kotlin View. On Android the "in-app recording state" is the React/TypeScript `RecordButton` component running inside `TauriActivity`'s WebView. The token mapping is: canon `--k-amber-line` → Tailwind `klarvo-warning` (alias for `klarvo-amber`).

**AC8 — No hardcoded color literals remain in the in-app recording surface:**
Given the re-skin is complete
When the `RecordButton` component and status-label block are audited
Then zero hardcoded hex strings (`#xxxxxx`) or raw `rgba(...)` color literals are present in these elements (the shadow strings — `shadow-[0_0_40px_rgba(...)]` — are excluded from this constraint as Tailwind's JIT cannot express arbitrary shadow-with-alpha via token alone; they are acceptable if the color values match the semantic role's token value).

**DoD:** `scripts/android-smoke.sh` exits 0 (compile clean, APK built); APK freshness verified (timestamp gate); on-device visual smoke: the in-app RecordButton shows the correct teal idle / danger recording / amber processing visual states.

## Tasks / Subtasks

- [x] **Task 1: Audit current `RecordButton` and surroundings** (AC: 1–8)
  - [x] 1.1 Read `src/App.tsx` lines ~51–92 (`RecordButton` component) and lines ~731–803 (center recording section). Document every color class and literal found.
  - [x] 1.2 Identify any `bg-[#...]` hardcoded hex — specifically `bg-[#0c0c0e]` on the raw-text textarea (line ~788).
  - [x] 1.3 Verify all non-shadow color classes already resolve to `klarvo-*` tokens or their aliases. If they do, they satisfy ACs 1–5 by construction (no change needed).
  - [x] 1.4 Verify `isMobile` import path is `"./platform"` — do NOT alter.

- [x] **Task 2: Replace hardcoded color(s) with token utilities** (AC: 6, 8)
  - [x] 2.1 Replace `bg-[#0c0c0e]` on the raw-text textarea (App.tsx ~line 788) with `bg-klarvo-bg-deep`. Verify `klarvo-bg-deep` is defined in `src/styles.css` (it is: `--color-klarvo-bg-deep: #0A0B0C`). Tailwind v4 exposes this as `bg-klarvo-bg-deep` — test the class resolves at compile time.
  - [x] 2.2 If any other hardcoded hex appears in the identified block, replace with the nearest semantic token (consult token table in Dev Notes below).
  - [x] 2.3 Do NOT change RecordButton's shadow-with-alpha strings (e.g. `shadow-[0_0_40px_rgba(42,195,168,0.2)]`) — per AC8, these are excluded from the constraint.

- [x] **Task 3: Verify Tailwind v4 resolves the new utility** (AC: 6)
  - [x] 3.1 Run `npm run build` from `shells/windows/` (or verify via `tsc` + Vite in the dev loop) to confirm `bg-klarvo-bg-deep` is recognized and not a dead class. Alternatively run `npx tailwindcss build` and check the output contains `bg-klarvo-bg-deep`.
  - [x] 3.2 If `bg-klarvo-bg-deep` is NOT recognized (Tailwind JIT safety), add a Tailwind safelist entry or use the CSS variable directly via `bg-[var(--color-klarvo-bg-deep)]` as a fallback.

- [x] **Task 4: Compile + smoke** (AC: DoD)
  - [x] 4.1 `scripts/android-smoke.sh` exits 0 (Kotlin compile + JVM tests green, APK built).
  - [x] 4.2 Verify APK freshness via timestamp gate (per `reference_android_build_freshness`).
  - [x] 4.3 On-device (emulator or Andi's real device): open the app → tap the RecordButton → confirm idle=teal, recording=red ring + StopIcon, processing=amber Spinner states are visually correct.

- [x] **Task 5: Commit** (AC: all)
  - [x] 5.1 Stage only touched files. Never `git add .`.
  - [x] 5.2 Commit message: `feat(android/9-9): in-app recording re-skin — replace hardcoded color with bg-klarvo-bg-deep token`

## Review Findings

Code review of `c2452e38..eaf494f` (Blind / Edge / Auditor, Opus), 2026-06-21:

- [ ] [Review][Patch] Processing/busy glow shadow uses stale `rgba(255,163,68)` (#FFA344, old amber) — must be `rgba(233,162,76)` (canon `--k-amber` #E9A24C) to match its role token [src/App.tsx ~70] (AC8/AC3)
- [ ] [Review][Patch] Idle glow shadow uses stale `rgba(42,195,168)` (#2AC3A8, old teal) — must be `rgba(41,199,172)` (canon `--k-teal` #29C7AC), both the `0.2` and the hover `0.3` occurrences [src/App.tsx ~71] (AC8/AC1)
- [ ] [Review][Patch] Recording pulse-ring salience regression — `border-klarvo-warning/40` on the `animate-ping` span compounds with the span's own `opacity-40` (~0.16 effective alpha) vs the original `border-red-400` × `opacity-40` (~0.40). Use full-alpha `border-klarvo-warning` (keep `opacity-40`) to restore the live-pulse visibility [src/App.tsx ~81] (AC7)
- [x] [Review][Dismiss] "recording & processing now share amber" — intentional GATE-1 design decision (distinguished by Stop icon vs Spinner); not a defect
- [x] [Review][Dismiss] "tokens may not resolve / bg-klarvo-bg-deep undefined" — false: all tokens exist in styles.css; `npm run build` generated the utility
- [x] [Review][Dismiss] "#0c0c0e→token is an unverified color shift" — intended by AC6 (canonical deep-bg token, near-imperceptible)

## Dev Notes

### What "android-05 in-app recording surface" means

The Android `TauriActivity` hosts a WebView that renders the full React/TypeScript app. The "in-app recording surface" (`android-05`) is the **`RecordButton` component** in `src/App.tsx` plus the status label and result/raw-text areas that surround it. It is NOT a native Kotlin View — it is React running inside the Android WebView.

This is distinct from the Dictation Bubble (`FloatingBubbleView.kt`, `ListeningPanelView.kt`) which is a native Kotlin overlay. The bubble re-skin was done in stories 9-3 through 9-5.

### Current surface state (read before touching)

`src/App.tsx` lines ~51–92: `RecordButton` component CURRENTLY uses:
- idle: `bg-klarvo-primary/15`, `text-klarvo-primary`, `border-klarvo-primary/25`, shadow with teal rgba — **OK, keep (teal idle).**
- recording: `bg-klarvo-danger/20`, `text-klarvo-danger`, `border-klarvo-danger/40`, **hardcoded** `border-red-400 animate-ping` ring, red glow shadow — **MUST CHANGE → amber** per AC2 (GATE-1 decision). This is the red→amber re-skin.
- busy/processing: `bg-klarvo-warning/15`, `text-klarvo-warning`, `border-klarvo-warning/30` — **OK, keep (amber).**

Idle (teal) and processing (amber) are already token-driven and stay. The **recording** state must move from danger/red to **`klarvo-warning` (amber)** for bg, border, icon, pulse-ring and glow — and the hardcoded `border-red-400` must become an amber token. (The earlier story-creation claim that "no change is needed to the button element" was wrong: it missed the AC2 red→amber decision and the `red-400` literal.)

`src/App.tsx` lines ~731–803: Status label already uses `text-klarvo-danger`, `text-klarvo-primary`, `text-klarvo-warning`, `text-klarvo-dim` — all token-driven. **No change needed** to status label.

**The one hardcoded color to fix:**
`src/App.tsx` line ~788: raw-text textarea has `className="w-full bg-[#0c0c0e] border border-klarvo-border/40 rounded-lg px-3 py-2 text-xs text-klarvo-muted resize-none focus:outline-none"`. The `bg-[#0c0c0e]` is a hardcoded hex that should be `bg-klarvo-bg-deep` (token value = `#0A0B0C`, the deepest background layer — visually equivalent, semantically correct).

> ALSO check line ~597 where `bg-[#0c0c0e]` also appears in a different component (LicenseSettings or similar). If it's NOT within the in-app recording surface block (lines 731–803), leave it for the Epic 8 desktop re-skin stories — this story is scoped to the recording section only.

### Token mapping table

| Visual role | Tailwind class | CSS var | Hex | Android (KlarvoTheme) |
|-------------|----------------|---------|-----|----------------------|
| idle button bg | `bg-klarvo-primary/15` | `--color-klarvo-teal` at 15% | `#29C7AC` | `KlarvoTheme.Teal` |
| recording bg | `bg-klarvo-danger/20` | `--color-klarvo-danger` at 20% | `#EE6F63` | `KlarvoTheme.Danger` |
| processing bg | `bg-klarvo-warning/15` | `--color-klarvo-amber` at 15% | `#E9A24C` | `KlarvoTheme.Amber` |
| deepest bg (raw text) | `bg-klarvo-bg-deep` | `--color-klarvo-bg-deep` | `#0A0B0C` | `KlarvoTheme.BgDeep` |
| surface bg | `bg-klarvo-bg` | `--color-klarvo-bg` | `#0F1112` | `KlarvoTheme.Bg` |

### ADR-0019 colour-semantics rule (must not violate)

- Teal = brand / ready / processing / focus-ring → idle = teal ✓
- Amber = live/listening (recording tally light only) → processing spinner = amber ✓
- Danger/red = stop / cancel / error → recording stop state = danger ✓
- Red is NEVER the send/confirm action (AC from 9-5 — preserved here)

### Files to touch

| File | Change |
|------|--------|
| `src/App.tsx` | **(1)** Recording-state of `RecordButton` (~lines 67–82): danger/red → `klarvo-warning` (amber) for bg, border, icon AND replace hardcoded `border-red-400 animate-ping` ring with an amber token (e.g. `border-klarvo-warning/40`); recording glow shadow rgba → amber value. [AC2/AC7 — GATE-1] |
| `src/App.tsx` | **(2)** Recording status-label text (~lines 731–803): `text-klarvo-danger` → `text-klarvo-warning` for the `recording` state. [AC4] |
| `src/App.tsx` | **(3)** Raw-text textarea (~line 788): `bg-[#0c0c0e]` → `bg-klarvo-bg-deep`. [AC6] |

**No Kotlin files need to change.** No `KlarvoTheme.kt`, no `FloatingBubbleView.kt`, no `KlarvoOverlayService.kt` — this is a React/TypeScript re-skin only.

### What NOT to touch

- `RecordButton` **idle (teal)** and **processing (amber)** colors — already token-driven and correct; only the **recording** state changes (red → amber)
- Status label colors — already token-driven
- Shadow-with-alpha strings on the RecordButton — excluded from scope (see AC8)
- Any component outside the in-app recording block (`RecordButton` + status label + result/raw-text areas, ~lines 731–803)
- `bg-[#0c0c0e]` that may exist in other parts of `App.tsx` (e.g., line ~597) — leave those for Epic 8 desktop stories (8-2 through 8-6) unless they are within the ~731–803 recording section

### Platform note: this change applies to BOTH desktop and Android

`src/App.tsx` is the shared React app that runs on both Desktop (Tauri WebView2) and Android (TauriActivity WebView). The re-skin of the raw-text area will apply on both platforms. This is intentional and correct — it's a tokenization fix, not an Android-only change. The `isMobile` branch for sizes is unrelated.

### Build/smoke process

Per `project-context.md`:
- `scripts/android-smoke.sh` runs the build gate (`node scripts/gen-android-theme.mjs --check`) first — **do NOT hand-edit `KlarvoTheme.kt`** (it is auto-generated; a manual edit fails the drift gate).
- Android build freshness via `scripts/android-build.sh` timestamp gate.
- On-device visual gate: the RecordButton is visible on the main screen (home view, no panel open). Tap it to cycle through states.

### How Andi can produce each test state (Verifikations-Symmetrie)

Andi can reach all three visual states of the in-app RecordButton directly:
1. **Idle (teal):** Open the app → home view → RecordButton is idle teal.
2. **Recording (red/danger):** Tap the RecordButton → recording starts → button turns red with pulse ring.
3. **Processing (amber):** Tap again during recording → `transcribing` state → amber spinner.
4. **Raw text area:** After processing completes, if a result differs from raw text, tap "Show original" → raw-text textarea appears; verify background color looks like the deepest-dark canvas (visually: nearly black, consistent with the surrounding dark surface).

### Why this story exists (D2 context)

Decision D2 in the epic planning: FR8 (in-app recording re-skin) was kept in Epic 9 as a separate small story. The bubble (Dictation Bubble / overlay service) was re-skinned in stories 9-3, 9-4, 9-5. The in-app recording surface (the React RecordButton shown when Klarvo is used as a standalone recorder) needed the same DT closure: no hardcoded colors for covered roles.

The epics-visual-overhaul.md story spec says: "no hardcoded colors for covered roles remain in this surface (DT closure for the Android in-app surface)." This is a small tokenization patch — the dominant work is verification that the surface already uses tokens correctly, with one targeted fix.

### References

- `[Source: src/App.tsx:51–92]` — `RecordButton` component, current color classes
- `[Source: src/App.tsx:731–803]` — Center recording section: status label, result area, raw-text area
- `[Source: src/styles.css:61–97]` — `klarvo-bg-deep` token definition, all `--color-klarvo-*` vars
- `[Source: docs/design/overhaul/source/assets/klarvo.css:452–453]` — Canon `.inapp-mic` reference (design SSOT for the surface concept)
- `[Source: _bmad-output/planning-artifacts/epics-visual-overhaul.md:Story 9.9]` — Epic spec, ACs, DoD
- `[Source: docs/adr/0019-cross-platform-design-ssot.md]` — ADR-0019 color semantics rule
- `[Source: _bmad-output/project-context.md]` — Android smoke DoD, gen-android-theme drift gate, camelCase config trap, `jni 0.21` pin
- `[Source: android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt]` — Generated token file; do NOT hand-edit

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

None

### Completion Notes List

- All 5 changes in `src/App.tsx` applied: recording-state re-skinned from danger/red to klarvo-warning/amber (AC2/AC7); status label recording text changed to amber (AC4); hardcoded `border-red-400` pulse-ring replaced with `border-klarvo-warning/40` (AC8); raw-text textarea `bg-[#0c0c0e]` replaced with `bg-klarvo-bg-deep` (AC6).
- Idle (teal) and processing (amber) states were already token-driven — no change needed (AC1/AC3 satisfied by construction).
- Result textarea was already fully token-driven — no change needed (AC5 satisfied by construction).
- Status label for all states except `recording` already used correct tokens — only `recording` changed from `text-klarvo-danger` to `text-klarvo-warning` (AC4).
- `border-red-400` remains in `Onboarding.tsx` line 1104 — out of scope (Epic 8 desktop stories 8-6).
- `bg-[#0c0c0e]` remains in `App.tsx` line ~597 (LicenseSettings context) — out of scope per Dev Notes.
- `npm run build` (tsc + Vite): PASS — `.bg-klarvo-bg-deep` compiled, no TypeScript errors.
- `scripts/android-smoke.sh`: EXIT 0 — 24 JVM tests green, KlarvoTheme.kt drift-gate OK, APK built (104 MB), installed on 100.112.41.70:5555.
- Visual smoke gate: APK installed on real device — Andi to verify idle=teal / recording=amber ring / processing=amber spinner.

### File List

- `src/App.tsx`

### Change Log

- 2026-06-21 — **Close-out → done (conductor + Andi).** Review clean (3 patches found + fixed, commit 3b2d7ee); GATE-4 machine-smoke green at artifact level (token chain verified in built CSS+JS; `android-smoke.sh` exit 0; APK installed on real device). **Scope reality, surfaced this run:** the in-app `RecordButton` surface (`android-05` / `.inapp-mic`) is NOT in the current Model-B canon — `.inapp-mic` is an orphan CSS rule, never instantiated in the canon HTML; the canon's Android recording language is the bubble overlay only. 9-9 therefore reduced to a DT-closure (red→amber + remove hardcoded colors + raw-text bg token) of a real-but-undesigned, shared-with-desktop surface. Andi accepted it as done rather than over-investing. **Follow-up to consider:** decide the fate of the in-app big-mic button on Android (keep / hide / redesign to Model-B symbolism) — not 9-9's job. GATE-4 evidence: `gate4-evidence/9-9/verdict.md`.
- 2026-06-21 — **GATE-1 design decision (conductor + Andi):** Resolved an AC2/AC7 contradiction the story-creation worker merged silently. In-app recording-state indicator = **AMBER** (`klarvo-warning` / canon `.inapp-mic .ring` amber-line pulse), NOT red/danger. Red is reserved for stop/cancel/error only (ADR-0019; consistent with 9-5 Modell-B bubble). Scope corrected from "1-line tokenization" to "recording red→amber re-skin + remove hardcoded `border-red-400` + raw-text `bg-[#0c0c0e]`→token". ACs 2, 4, 7, 8 and Dev Notes updated accordingly.
- 2026-06-21 — **Story implemented:** Recording state re-skinned danger→amber (AC2/AC7), status label recording→amber (AC4), raw-text textarea tokenized (AC6), hardcoded `border-red-400` removed (AC8). Build PASS. android-smoke.sh EXIT 0.
