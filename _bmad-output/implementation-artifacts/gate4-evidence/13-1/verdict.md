# Story 13-1 — desktop proxy gate (GATE-4), verdict

**Harness:** `debug-rows-smoke.mjs` (throwaway, kept here per project-context;
no harness script lives in the repo tree).
**Target:** `npm run preview` on port 1422, real Chromium via puppeteer.
**Run:** 2026-09-20, spec `spec-13-1-debug-test-provider-both-twins-2.md`.

## Result

| Run | File | Checks | Verdict |
|---|---|---|---|
| green | `report.md` | 66 ordinary, 0 failed | **PASS** (exit 0) |
| inversion | `inversion-report.md` | 18 ordinary (0 failed) + 3 inversion groups, each red | **PASS** (exit 0) |

Both counts are printed by the harness from its own result records — nothing here
is hand-counted (the predecessor run reported 23 in the verdict and listed 24 in
the report; that class of mismatch is now impossible).

**An inversion assertion passes by going red**, so the inversion run is scored
separately and exits 0 when every group produced at least one red and no
ordinary check failed. Its three groups:

| Group | What was deliberately wrong | Red evidence |
|---|---|---|
| INVERSION-1 | the **same** `assertStatesEqual` call with the **same** measured states, pointed at the raw `<select>` of the Log Level row | 8 of 13 properties differ (`backgroundColor`, all four `border*Color`, `paddingTop`, `paddingBottom`, `boxShadow`) |
| INVERSION-2 | the **same** row-presence predicate asserted with Expert mode OFF | the four rows are absent, so the predicate discriminates |
| INVERSION-3 | the **same** leak predicate fed by the **same** option reader, aimed at the four rows that DO offer `debug` | `["debug","debug"]` found — the scan can see the leak it rules out elsewhere |

INVERSION-3 reads only the four `KSelect`s, not the page's native `<select>`s: the
Log Level row also offers a `debug` option (a log level), and letting *that*
produce the red would have proved nothing about the picker scan.

## What the green run proves

1. **Visibility gate.** Settings → Advanced → System carries **0** `KSelect`
   controls with Expert mode off and **exactly 4** with it on, labelled
   `LLM Provider`, `STT Provider`, `Debug LLM Scenario`, `Debug STT Scenario`.
2. **Option lists are pinned to the fixture, element-wise** — the same
   `test-fixtures/debug-provider-scenario-vectors.json` the Rust and JVM tests
   read, whose two `provider-options` vectors a Rust test compares against
   `config::VALID_LLM_PROVIDERS` / `::VALID_STT_PROVIDERS`. So the React lists
   are chained to the Rust allowlists and canned-wire tables:
   - `LLM Provider` → `deepseek, openai, anthropic, groq, openrouter, debug`
   - `STT Provider` → `groq, openai, local, debug`
   - `Debug LLM Scenario` → `ok, empty, truncated, malformed, http429, http5xx, transport`
   - `Debug STT Scenario` → that set minus `truncated`
   Wording is derived, not composed: option labels are the config values verbatim,
   no hint line, no prose.
3. **Picker, in BOTH Expert-mode states.** `debug` is absent from every option
   label in Settings → Recording & Audio (3 dropdowns, 8 labels: the four cloud
   LLM providers, `Local (Offline)`, three microphone entries) with Expert mode
   off **and** on. The "on" state is not clickable in preview (writers are
   no-ops), so it is reached with the project's documented technique: a throwaway
   edit to `src/tauri-commands.ts` setting `MOCK_ADVANCED_SETTINGS.expertMode`,
   verified to have taken effect (the four rows render without touching the
   switch), then restored to the original bytes — the harness asserts the restore.
4. **AI & Providers carries no provider picker — positively.** The page is first
   proven to have rendered (its own `Cleanup Instructions` and `App Profiles`
   sections), then a profile is **added** so the only dropdowns this surface can
   ever show actually exist, and those are read: `["Polished","Verbatim","Chat",
   "Auto","DE","EN"]` — a cleanup-style picker and a language picker, no
   provider-shaped label among them. The predecessor run scanned 0 elements here
   and reported that as cleared; this one cannot.
5. **Control-states contract, for EACH of the four rows.** For `idle`, `hover`,
   `focused`, `open` (= `pressed` — the shipped component defines no distinct
   pressed styling), the open `listbox`, its `optionSelected` and its
   `optionKeyboardFocused`, all 13 measured properties equal the **named
   reference instance**: the "Dictation language" `KSelect` in Settings →
   Language (`src/components/settings/LanguageContent.tsx`). The open state
   additionally carries `aria-expanded="true"` and the rotated chevron. The
   keyboard-focused option is compared like-for-like (both sides land on a
   non-selected option; the report line records the aria-selected values).

Raw measurements: `reference-states.json` vs `measured-states.json` (a map keyed
by row label), and `inversion-measured.json` for the red run.

## Harness traps recorded, not hidden

1. **Escape closes the whole Settings panel.** `KSelect`'s Escape handler does
   not stop propagation, so the key reaches `SettingsPanel`'s document listener.
   A listbox is closed by clicking its trigger again.
2. **`className.includes("bg-klarvo-surface-2")` also matches
   `hover:bg-klarvo-surface-2`**, which every enabled option carries — class
   tokens are matched with `classList.contains`, plus an explicit like-for-like
   assertion.
3. **`innerText` applies CSS `text-transform`.** The AI & Providers section
   headings are `uppercase`, so a case-sensitive "did the page render" probe read
   as "page empty". Matched case-insensitively.

None is a product defect; all are recorded because a harness bug that produces a
green is the same class of failure as a product bug.

## What this run does NOT decide

- **Persistence.** Preview-mode writers are no-ops; "Save" is never clicked and
  nothing reaches `config.json`. The round-trip is the Rust suite's claim
  (`config::tests::spec_debug_provider_and_scenarios_survive_a_real_config_file_round_trip`,
  which writes and re-reads a real file).
- **Anything Rust.** No Tauri, no provider resolution, no canned wire response,
  no pipeline run, no fallback ladder.
- **The `disabled` state** of `KSelect`. These four rows never pass `disabled` —
  the Expert-mode gate removes them instead — so the state is unreachable here
  and is recorded in the spec's control-states table as not exercised.
- **Pixels, font rasterisation, the Windows text-scale drift.** Andi's
  real-screen gate.
- **Android.** The same React bundle renders there via
  `MainActivity : TauriActivity`, but nothing on a device was touched.
- **The save ordering.** That the Settings footer must be pressed last (only
  `save_settings` rebuilds the STT slot) is an operational rule in the spec's
  manual checks, not something this harness can observe.

## Pre-existing defect corroborated, not fixed

`idle.boxShadow` is `rgba(41, 199, 172, 0.28) 0px 0px 0px 3px` on **both** the
new controls and the reference instance — the always-on teal ring from
`src/styles.css::.focus-klarvo` (a bare class, not a `:focus-visible` rule).
It is app-wide, predates this story, and is already filed in `docs/backlog.md`.
The equality gate holds whether or not the ring is correct, so it blocks no
acceptance criterion here.
