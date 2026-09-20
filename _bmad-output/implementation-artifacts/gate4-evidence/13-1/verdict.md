# Story 13-1 — desktop proxy gate (GATE-4), verdict

**Harness:** `debug-rows-smoke.mjs` (throwaway, kept here per project-context;
no harness script lives in the repo tree).
**Target:** `npm run preview` on port 1422, real Chromium via puppeteer.
**Run:** 2026-09-20 · green run exit 0 · inversion run exit 1 (as required).

## Result

| Run | File | Verdict |
|---|---|---|
| green | `report.md` | **PASS** — 23/23 checks |
| inversion | `inversion-report.md` | **FAIL, as designed** — the same equality assertion pointed at the raw `<select>` of the Log Level row reports 8 of 13 properties differing |

The inversion run is the licence for the green one: a computed-style comparison
that has never been shown red proves nothing. Differing properties against the
raw `<select>`: `backgroundColor`, all four `border*Color`, `paddingTop`,
`paddingBottom`, `boxShadow`.

## What the green run proves

1. **Visibility gate.** Settings → Advanced → System carries **0** `KSelect`
   controls with Expert mode off and **exactly 2** with it on, labelled
   `Debug LLM Scenario` and `Debug STT Scenario`.
2. **Wording is derived, not composed.** The LLM option list is exactly
   `["ok","empty","truncated","malformed","http429","http5xx","transport"]`; the
   STT list is that set minus `truncated`. No hint line, no prose.
3. **Picker.** `debug` is absent from every option label in Settings →
   Recording & Audio (3 dropdowns, 8 labels: the four cloud LLM providers,
   `Local (Offline)`, and the three microphone entries) and from the page text of
   Settings → AI & Providers. A cross-page guard fails the run if neither page
   yields a single option label, so the claim cannot pass vacuously.
4. **Control-states contract.** For `idle`, `hover`, `focused`, `open`
   (= `pressed` — the shipped component defines no distinct pressed styling),
   the open `listbox`, its `optionSelected` and its `optionKeyboardFocused`,
   all 13 measured properties equal the **named reference instance**: the
   "Dictation language" `KSelect` at `src/components/settings/LanguageContent.tsx:59`.
   The open state additionally carries `aria-expanded="true"` and the rotated
   chevron. The keyboard-focused option is compared like-for-like (both sides
   land on a non-selected option; recorded in the report line).

Raw measurements: `reference-states.json` vs `measured-states.json`.

## Two harness traps found and fixed during the run (recorded, not hidden)

1. **Escape closes the whole Settings panel.** `KSelect`'s Escape handler does
   not stop propagation, so the key reaches `SettingsPanel`'s document listener.
   The harness closes a listbox by clicking the trigger again.
2. **`className.includes("bg-klarvo-surface-2")` also matches
   `hover:bg-klarvo-surface-2`**, which every enabled option carries. The first
   version therefore read option[0] on both controls and compared the *selected*
   option on one against a plain option on the other — a real false red.
   Fixed to `classList.contains(...)`, plus an explicit like-for-like check.

Neither is a product defect; both are recorded because a harness bug that
produces a green is the same class of failure as a product bug.

## What this run does NOT decide

- **Persistence.** Preview-mode writers are no-ops; "Save" is never clicked and
  nothing reaches `config.json`. The allowlist round-trip is the Rust suite's
  claim (`config::tests::spec_debug_provider_survives_normalization`).
- **Anything Rust.** No Tauri, no provider resolution, no canned wire response,
  no pipeline run.
- **The `disabled` state** of `KSelect`. These two rows never pass `disabled` —
  the Expert-mode gate removes them instead — so the state is unreachable here
  and is recorded in the spec's control-states table as not exercised.
- **Pixels, font rasterisation, the Windows text-scale drift.** Andi's
  real-screen gate.
- **Android.** The same React bundle renders there via
  `MainActivity : TauriActivity`, but nothing on a device was touched.

## Pre-existing defect corroborated, not fixed

`idle.boxShadow` is `rgba(41, 199, 172, 0.28) 0px 0px 0px 3px` on **both** the
new control and the reference instance — the always-on teal ring from
`src/styles.css::.focus-klarvo` (a bare class, not a `:focus-visible` rule).
It is app-wide, predates this story, and is recorded in the spec frontmatter
under `deferred`. The equality gate holds whether or not the ring is correct,
so it blocks no acceptance criterion here.
