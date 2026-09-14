# GATE-4 verdict — Story 7-10 (cleanup failure → raw text clipboard-only)

Run: 2026-09-14 · branch `conductor/story-7-10` · HEAD `d73082d` · baseRef `6cedbb5`

## Self-verification (conductor, mechanical)

| Layer | What ran | Result | Proves |
|---|---|---|---|
| Rust unit suite | `cargo test --lib` (dev worker, fix rounds, re-review re-run) | 691 passed, 0 failed | routing logic: `llm_error` → `copy_only` → `ClipboardOnly`, Enter skipped; `terminal_degrade_msg`; `degrade_msg ⇔ llm_error` invariant (2 of 3 degrade sites) |
| Inversions | re-enable paste on `llm_error` (Rust); `accessibilityConnected=true` + `llmCleanupFailed=true` (Kotlin); D1 text/colour; round-2 stale warning | all RED, then reverted | the tests bind to the production predicate |
| Kotlin JVM gate | `./gradlew :app:testUniversalDebugUnitTest` | 24 suites, 194 tests, 0 failures | `decideDelivery` branch logic + toast literal |
| Frontend | `tsc && vite build` | clean | types |
| D1 proxy smoke | puppeteer vs `npm run preview` in real Chromium (`run-d1-smoke.sh`, `report.txt`) | 13/13 | main-window status line shows the degrade cause verbatim, canon amber `#E9A24C`, model ID survives, clean run resets to teal "Done" (CASE C = degrade run → clean run in one boot) |
| Windows release build | `scripts/windows-build.sh conductor/story-7-10` (`windows-build.log`) | exit 0, exe fresh 10:45:57, contains `d73082d` | the code — including the Windows-gated `native_pill.rs` (`pending_msg` staging, `DoneClipboard` arm rendering `status_msg`) — compiles, links and is installed at `D:\apps\klarvo` |

## NOT exercised by any machine gate (the residual)

- A real dictation through the hotkey pipeline with the cleanup call failing (needs a live microphone).
- The native pill's rendering of the carried message: readability, tail truncation at 200×36, the 4 s hold.
- That NOTHING lands in the focused window and NO Enter is sent with Insert+Send on (SendInput path, real focus).
- Ctrl+V afterwards yields the raw text (real Windows clipboard via `arboard`).
- Android: toast, clipboard, no paste (no emulator on powerhouse; Kotlin twin is JVM-tested only).

## Residual for Andi (real Windows machine, build `d73082d`)

1. Settings → About shows `Build d73082d`.
2. Settings → Advanced → Model IDs: set the DeepSeek model ID to a wrong value (e.g. `deepseek-typo`).
3. Turn Insert+Send ON for the slot you use. Focus a text field (editor or chat box).
4. Dictate one sentence.
   Expected: nothing appears in the field, no Enter is sent. The pill shows amber
   `Model 'deepseek-typo' not found — in clipboard` for about 4 s. The main window status line shows the same text in amber.
5. Press Ctrl+V in the field. Expected: the raw transcript appears.
6. Restore the correct model ID. Dictate again. Expected: cleaned text is pasted as before, Enter is sent, pill shows "Done", main window shows "Done" in teal (no stale amber text).
7. History: the failed run is listed with its raw text (unchanged behaviour).

Verdict: **review-cleared, GATE-4 open (Andi)**. Status stays `review` in both fields until Andi's smoke is green.

---

# GATE-4 verdict — round 2 (after Option A: pill = status light, preview card = message surface)

Run: 2026-09-14 · HEAD `37b2204` · Windows build on `37b2204` (exit 0, see `windows-build-r4.exit`)

## What changed since round 1
Round 1 FAILED on presentation: the 200×36 pill truncated the warning mid-sentence. Decision (Andi, mock
`docs/design/overhaul/mockup-7-10-message-card.html`): the pill shows only static labels; the existing native
preview card carries every pipeline message (cause · "Raw text is in the clipboard · Ctrl+V to paste" · hint),
4 s + 1 s fade, dismissed by a new recording or a click, flips below the pill when it does not fit above.

## Self-verification (mechanical)
| Layer | Result | Proves |
|---|---|---|
| `cargo test --lib` | 707 passed, 0 failed | message model (`overlay_message.rs`): cause/next/hint, chip split, TEXT LOST tone, token mapping, boot-warning merge, `in_history` |
| Inversions | 4 (AC8 dev) + 4 (fix r1) + 2 (fix r2), all RED then reverted | tests bind to production |
| D1 proxy smoke | 13/13 | main-window status line unchanged |
| Windows release build | exit 0 on `d03a4da`, `d778d5c`, `0eedb40`, `37b2204` | `native_pill.rs` + `native_preview.rs` compile, link, install |
| Code review | full review of `8b81864` (3 decisions, 11 patches → fixed), scoped re-reviews of `d778d5c` and `0eedb40`; loop closed on a review | |

## NOT exercised by any machine (Windows-gated, no test module)
Card layout, wrap, fade, click-dismiss, below-pill flip, Geist on the card, Light-theme background, the pill's
static labels as drawn. Residual known: flip band ~127–144 px above the pill (backlog).

## Residual for Andi (Windows, build `37b2204`)
1. Settings → About shows `Build 37b2204`.
2. Wrong DeepSeek model ID (e.g. `deepseek-typo`), Insert+Send ON, focus a text field, dictate.
   Expected: nothing in the field, no Enter. Pill amber `Cleanup failed`. Above the pill a card: header CLEANUP
   FAILED, `Model 'deepseek-typo' not found` (ID on an amber chip), `Raw text is in the clipboard · Ctrl+V to paste`,
   `Check Advanced → Model IDs`. Card visible ~4 s, then fades.
3. Ctrl+V in the field → raw transcript appears.
4. Live preview OFF (Settings) and repeat step 2 → the card still appears.
5. Click the card while it shows → it disappears at once; a click on the spot afterwards reaches the window below.
6. Drag the pill to the top edge of the screen and repeat step 2 → the card appears BELOW the pill.
7. Correct model ID, dictate → cleaned text pasted, Enter sent, pill `Done`, no card, main window `Done` teal.
8. Restart Klarvo with a config warning if you have one (optional) → one card at boot, not several.

Verdict: **review-cleared, GATE-4 round 2 open (Andi)**. Status stays `review` in both fields.
