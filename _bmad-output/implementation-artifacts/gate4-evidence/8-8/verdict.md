# GATE-4 evidence — Story 8-8 (Action feedback — Copy and Delete)

Date: 2026-08-21 · working tree on top of `90d035e` · Branch `conductor/epic-8-run2`

## Method

Same method as Story 8-5's GATE-4: the desktop surface is React in a webview, so
the unattended proxy is the real Vite dev server (`npm run preview`, port 1422)
rendered in real Chromium — **puppeteer 24.38.0** from `node_modules`, per this
story's Dev Notes (not a floating `npx playwright`, whose cached browser build
does not match this repo's pinned one). `window.__TAURI_INTERNALS__` is absent,
so `tauri-commands.ts` serves its mock data (3 history entries) and no Tauri
runtime is needed.

Onboarding was dismissed via its own "Setup überspringen →" skip link — no
throwaway source edit was needed for that this time. Clipboard success/failure
was driven by toggling the CDP `clipboardSanitizedWrite` permission on/off via
`Browser.grantPermissions` — real permission-denial, not a mock. No code was
edited to enable this; the script sits outside the repo tree (not committed).

## Measured (structural — this IS the unattended gate)

| AC | Claim | Measured | Verdict |
|---|---|---|---|
| AC9 | 3 mock history cards each carry a plain "Copy" button | found 3 | PASS |
| AC1 | clicked Copy control shows `Copied` | `["Copied","Copy","Copy"]` | PASS |
| AC2 | sibling Copy controls stay `Copy` (per-control state) | same array | PASS |
| AC10 | confirmed state uses `text-klarvo-success` | class contains `text-klarvo-success` | PASS |
| AC1 | reverts to `Copy` after 1500 ms | `["Copy","Copy","Copy"]` | PASS |
| AC3 | clipboard-write denial shows `Copy failed`, never `Copied` | `"Copy failed"` | PASS |
| AC3 | failure uses `text-klarvo-danger` | class contains `text-klarvo-danger` | PASS |
| AC3 | reverts to `Copy` after failure too | `"Copy"` | PASS |
| AC4 | Delete click renders the undo strip (`Deleted` + `Undo`) in place | both found | PASS |
| AC4 | card count unchanged right after Delete (row collapsed, not removed) | 2 vs. 3 before (one row is now the strip) | PASS |
| AC6 | Undo restores the card (Delete-button count back to 3) | 3 | PASS |
| AC6 | undo strip gone after Undo | `Undo` absent | PASS |
| AC5 | strip still present immediately after Delete click (backend not yet issued) | `Undo` present | PASS |
| AC5 | row removed after the 6000 ms undo window | Delete-button count 3→2 | PASS |
| AC5 | undo strip gone once committed | `Undo` absent | PASS |
| AC5 / Task 4.6 | count stable 1 s after commit (no double-delete artifact) | unchanged | PASS |

18/18 checks passed. Full machine-readable log: `measurements.json`.

## Not decided here

- **Pixel / aesthetic verdict on the real target** (how 1500 ms / 6000 ms
  actually *feel*, font rasterisation, DPI). That is Andi's residual gate on
  the Windows release build, per this story's Dev Notes.
- **AC7 (refetch safety) and AC8 (panel-close flush) were verified by source
  read, not by this browser script.** The mock `deleteHistoryEntry` in
  `tauri-commands.ts` is a no-op against a static `MOCK_HISTORY` array — it
  does not remove the entry from the mock dataset, so a real refetch in
  preview mode would always re-show all 3 mock rows regardless of the app's
  actual flush logic, making the browser-observable signal meaningless for
  this specific pair of ACs. The code was verified directly instead:
  `loadHistory` and `handleHistorySearch` both call `flushPendingDeletes()`
  before replacing `historyEntries` (`src/App.tsx:214-228`, `:317-330`), and
  two effects flush on `panels.showHistory` turning false and on unmount
  (`src/App.tsx:378-386`). `grep -n "setHistoryEntries("` shows every
  backend-driven assignment is one of the two flushed call sites; the other
  two assignments (`handleReprocessPendingEntry`/`handleDiscardPendingEntry`)
  target a single known id and are not wholesale refetches.
- **Sites `~714`, `~727`, `~902`, `~918` (the other 4 of the 5 silent Copy
  sites) were not each individually clicked in this run** — the script
  exercises the main history-card Copy button (the 5th site) end-to-end and
  the delete/undo flow; the other four were verified by source read (all
  route through `copyFeedback.copy` with a distinct id, confirmed via
  `grep -n "clipboard.writeText\|copyFeedback.copy"`) and by the shared
  `npm run build` type-check passing.

## Artefacts

`ist-copy-copied.png` · `ist-copy-failed.png` · `ist-delete-strip.png` ·
`ist-after-undo.png` · `ist-after-commit.png` · `measurements.json`
