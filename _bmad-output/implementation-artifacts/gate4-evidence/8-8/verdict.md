# GATE-4 evidence — Story 8-8 (Action feedback — Copy and Delete)

Date: 2026-08-21 · Range `90d035e..d8e7abb` · Branch `conductor/epic-8-run2`

This file was rewritten by the story conductor at GATE 4. It supersedes the dev-step
version, which reported `AC1 … PASS` with no geometry behind it and carried two
byte-identical screenshots as separate evidence (both were code-review findings).

## Method

The desktop surface is React in a webview, so the unattended proxy is the real Vite
server (`npm run preview`, port 1422) driven in real Chromium by the **repo-pinned
puppeteer 24.38.0** from `node_modules` — not a floating `npx playwright`, whose
resolved browser build is not in the local cache. `window.__TAURI_INTERNALS__` is
absent, so `tauri-commands.ts` serves its mock data (3 history entries) and no Tauri
runtime is needed. Onboarding was dismissed through its own "Setup überspringen →"
link. `BMAD_CONDUCTOR=1` was exported for every command in the run.

Three passes ran, all on the final commit `d8e7abb`:

- **Pass A — behaviour and list structure (19 checks).** Required one **throwaway
  instrumentation** of the preview-mode `deleteHistoryEntry` mock so the backend call
  becomes countable and the mock dataset actually shrinks: without it a refetch always
  re-serves all three rows, which is why the dev step could only *read* AC7/AC8 in the
  source. The edit was reverted immediately after the run — `src/tauri-commands.ts` is
  unmodified in the committed tree and no probe code is committed.
- **Pass B — the clipboard failure path (7 checks).** An empty `Browser.grantPermissions`
  grant does not reliably gate `navigator.clipboard.writeText` in headless Chromium, so
  the rejection was forced at the API. AC3's premise is literally "the clipboard write
  rejects"; the run asserts the premise held (`observedAtRuntime: true`) before judging
  the reaction.
- **Pass C — evidence re-shoot.** The confirmed, failed and undo-strip states captured
  on the current HEAD at `deviceScaleFactor: 2`.

## Measured (structural + behavioural — this IS the unattended gate)

| AC | Claim | Measured | Verdict |
|---|---|---|---|
| — | history panel shows the 3 mock rows | 3 | PASS |
| AC9 | "Show original" reveals a `Copy Original` control | 1 | PASS |
| AC1 | site ~714 `Copy Original` confirms with `Copied` | `"Copied"` | PASS |
| AC10 | that confirmed state carries `text-klarvo-success` | true | PASS |
| AC2 | sibling Copy controls keep their resting label | 4 unchanged | PASS |
| AC1 | site ~714 returns to its resting label after 1500 ms | 1 | PASS |
| AC1 | site ~727 raw-box overlay Copy confirms with `Copied` | `"Copied"` + `text-klarvo-success` | PASS |
| Task 2.7 | the confirming overlay control is forced visible | `opacity: "1"` | PASS |
| AC1 | neighbouring `Delete` does not move while `Copied` shows | x 479.0625 → 479.0625, dy 0 | PASS |
| AC4 | the undo strip occupies the deleted row's own list slot | index 1 of 3 | PASS |
| AC4 | no backend delete issued at click time | `[]` | PASS |
| AC7 | a refetch commits the pending delete, never resurrects the row | calls `[2]`, strips 0, row absent | PASS |
| AC5 | `delete_history_entry` called exactly once for that id | `[2]` | PASS |
| AC5 | the committed row is gone from the refetched list | 2 rows | PASS |
| AC8 | strip pending, no second backend call yet | `[2]` | PASS |
| AC8 | closing the history panel flushes the pending delete | `[2,1]` | PASS |
| AC8 | no dangling timer fires a second delete 7 s later | `[2,1]` | PASS |
| AC8 | reopened history reflects the committed deletes | 1 row | PASS |
| AC3 | the clipboard write actually rejected in this run | `observedAtRuntime: true` | PASS |
| AC3 | a rejected write shows `Copy failed`, never `Copied` | `"Copy failed"` | PASS |
| AC3 | the failed state carries `text-klarvo-danger` | true | PASS |
| AC3 | the failed confirmation is visible off-hover | `text-klarvo-danger`, parent `opacity-100` | PASS |
| AC1 | neighbouring `Delete` does not move under the **widest** label `Copy failed` | dx 0, dy 0 | PASS |
| AC1 | no history row changes size or position during the swap | dw/dh/dy = 0 on all 3 rows | PASS |
| AC3 | the control returns to its resting label after 1500 ms | `"Copy"` | PASS |

**26/26 passed** across the three passes. Machine-readable logs:
`conductor-measurements.json` (pass A) and `conductor-measurements-failpath.json` (pass B).

**AC1's "without the surrounding layout shifting" is now measured, not asserted.** The
code-review raised it as an open decision because no width is reserved anywhere. The
measurement settles it: under both the `Copied` and the wider `Copy failed` label the
neighbouring `Delete` control and all three list rows stay pixel-identical in position
and size. The acts cluster is right-aligned inside a `justify-between` row, so the
clicked control grows leftward into its own slack. **No width reservation is needed** —
which also means the Dev Notes' "do not resize the existing controls" was never violated.

Both canon rules are confirmed at render, not merely in source: the confirmed state is
`--k-success` green (`ist-copy-copied.png`), and the undo strip is neutral
`surface-2` ground with a dim mono `Deleted` and a teal `Undo`, **not** red
(`ist-delete-strip.png`).

## Not decided here

- **The pixel / aesthetic verdict on the real target.** Chromium is a close relative of
  WebView2, not the same renderer: font rasterisation, DPI scaling and the Windows
  text-scale-factor drift are not reproduced. How 1500 ms and 6000 ms actually *read* on
  screen is a judgement no proxy makes. This stays Andi's gate on the Windows release
  build (`scripts/windows-build.sh`). **A green run here is not a visual pass.**
- **One thing for that gate specifically:** the confirmation now overrides the acts row's
  hover gating, so `Copied` / `Copy failed` stays lit for its 1500 ms even after the
  pointer leaves the card. The canon's `.note .acts { opacity: 0 } / .note:hover .acts
  { opacity: 1 }` (`klarvo.css:441`) states no exception for the confirmed state. AC1's
  "for 1500 ms" was read as governing, since a confirmation the pointer wipes out is not
  a confirmation — but the visible consequence is Andi's call to keep or reject.
- **Sites ~902 and ~918** (the two current-recording Copy controls) were not clicked:
  reaching them needs a completed recording with raw text, which the preview mock cannot
  produce. They share the same hook and the same call shape as the three history sites
  that were driven end-to-end, and `tsc` covers the wiring.
- **A failed backend delete still removes the row from view** — deferred by conductor
  decision to `docs/backlog.md`; both candidate shapes would invent error UI the story
  never specified.

## Artefacts

`ist-copy-copied.png` · `ist-copy-failed.png` · `ist-delete-strip.png` (re-shot on
`d8e7abb`; the previous pair was byte-identical and showed no Copy control at all) ·
`conductor-ist-history.png` · `conductor-ist-strip.png` · `conductor-ist-after-flush.png` ·
`conductor-ist-copy-failed.png` · `conductor-measurements.json` ·
`conductor-measurements-failpath.json` · `conductor-console.txt` ·
`ist-after-undo.png` · `ist-after-commit.png` (from the dev step, still valid).
