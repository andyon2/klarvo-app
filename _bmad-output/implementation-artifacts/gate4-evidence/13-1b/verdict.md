# Story 13-1b — desktop proxy gate verdict

Harness: `test-provider-rows-smoke.mjs` (extended from 13-1's `debug-rows-smoke.mjs`),
real Chromium via puppeteer against `npm run preview` (port 1422).

| Run | Command | Result |
|---|---|---|
| green | `node …/13-1b/test-provider-rows-smoke.mjs` | **59 checks, 0 failed, exit 0** (`report.md`) — plus one NOT-EXERCISED note, see below |
| inversion | `… --invert` | **4/4 groups RED, 20 ordinary checks, 0 failed, exit 0** (`inversion-report.md`) |

`git status` on `src/tauri-commands.ts` was verified clean of the throwaway
`expertMode: true` edit after both runs.

## What the green run decided

- (a) Both `Test provider (…)` rows absent with Expert mode off, present with it on,
  **exactly two** `KSelect`s added, and all four story-13-1 labels absent **by name** in
  both states.
- (b) No test-provider value in the normal provider picker (`RecordingAudioContent`),
  with Expert mode **off and on**; `AiProvidersContent` positively asserted to carry no
  provider picker (a profile is added first so its dropdowns actually exist).
- (c) Both rendered option lists read element-wise from
  `test-fixtures/test-provider-scenario-vectors.json`.
- (d) Control-states equality against the named reference instance (the "Dictation
  language" `KSelect`) for both rows across idle / focus / press / open / listbox /
  selected option / keyboard-focused option, 13 computed properties each. **`hover` is
  NOT among them — see "What was not exercised" below.**
- (e) Sticky-footer geometry at a **desktop** and a **phone** viewport: the `Save`
  button's rect is fully inside the scroll container at `scrollTop = 0` and at
  `scrollTop = max`. The scroller is asserted to actually overflow first (62 px desktop,
  123 px phone) — a containment check on a scroller that does not scroll is vacuous.
- (f) No `[aria-label="Send feedback"]`, no `[title="Send Feedback"]`, no tooltip text,
  in both viewports.

## What the inversion run proved

| Group | The deliberate break | Red |
|---|---|---|
| INVERSION-1 | the SAME state assertion pointed at the raw `<select>` of the Log Level row | 1/10 (`idle` only — the fake reference differs from the real one in `idle` alone, because a raw `<select>` has no listbox, no options and no hover state to point the rest at) |
| INVERSION-2 | the SAME row-presence predicate with Expert mode OFF | 1/1 |
| INVERSION-3 | the SAME value-leak scan on a page that DOES carry those values | 1/1 (found all 15 labels) |
| INVERSION-4 | the SAME containment predicate with `sticky bottom-0` stripped at runtime | 1/3 (`scrollTop = 0`; at `scrollTop = max` the old footer is visible too — which is precisely why the defect only bit at the top of the scroll range) |

## What was NOT exercised (corrected 2026-09-21 after review)

**`hover` is not measurable in this harness, and the first version of this verdict
reported it green anyway.** Tailwind v4 wraps every `hover:` utility in
`@media (hover: hover)` (confirmed via CDP `CSS.getMatchedStylesForNode`:
`.hover\:border-klarvo-border-2 { &:hover { @media (hover: hover) { … } } }`), and
headless Chromium has no pointing device, so that media query is false and the rule never
applies. Both `measured-states.json` and `reference-states.json` recorded `hover`
byte-identical to `idle`, so the equality compared two idle samples and passed for the
wrong reason — and that, not the raw `<select>`'s missing listbox alone, is part of why
INVERSION-1 scores 1/10.

Four escapes were measured and none works: `Emulation.setEmulatedMedia` ignores the
`hover`/`pointer` feature names, `--blink-settings=…HoverType…` has no effect,
`CSS.forcePseudoState` forces the pseudo-class but not the media query, and headful is
not available on this host. The harness now probes `(hover: hover)`, **drops `hover` from
the computed-style comparison**, records it as NOT EXERCISED in `report.md` instead of as
a pass, and substitutes a structural check that both controls carry the same hover
variant class token (`hover:border-klarvo-border-2`). The rendered hover colour is Andi's
real-screen gate.

## What this run does NOT claim

Chromium only: wiring, structure, computed style and geometry. It cannot observe
persistence (preview writers are no-ops, so `Save` is never pressed), anything Rust (no
provider resolution, no canned wire, no hot reload), real Android (the phone pass is
Chromium at a phone viewport with an Android user agent — the mobile *layout branch*,
not the device), pixels, font rasterisation, or the Windows text-scale drift. Nothing
here ran on the Xiaomi or on Windows. It also cannot open the feedback panel, because
with the FAB off it has no UI trigger; only the FAB's absence is a DOM claim, and that
`FeedbackModal` stays imported and outside the guard is pinned by
`llm::tests::spec_react_feedback_fab_is_off_and_its_modal_stays_mounted`.

Traps carried forward and one new: see the file header of the harness (trap #6 —
`src/platform.ts` reads the user agent at module load, so the phone pass needs a fresh
page whose UA is set **before** `goto`).
