# GATE-4 verdict — Story 7-9 (conductor-run, 2026-09-12)

Code under test: `9015a0a` (last code commit; every later commit on `conductor/story-7-9` is docs-only).

## Self-verification (what I ran, objective results)

| Layer | Command | Result | Proves | Does NOT prove |
|---|---|---|---|---|
| Rust logic | `cargo test --lib` (review-4 worker, independent run) | 681 passed / 0 failed | logic on Linux | Windows-gated arms (`local` cleanup, `llm::local`), Tauri runtime, real `Klarvo.log` |
| Kotlin logic | JVM `testUniversalDebugUnitTest --rerun-tasks` (review-4 worker, generated tree `diff -rq` identical to sources) | 186 / 0 / 0 over 23 suites | logic, twin parity (10 fixture entries, 12 sanitize cases) | JNI, device, Android UI |
| Desktop proxy (structure + wiring) | `npm run build` from HEAD → `npm run preview` :1422 → `smoke.mjs` (conductor re-run, `conductor-smoke-run.log`) | 28/28 checks, 0 uncaught page errors | Advanced panel: 11 UI-visible dead rows absent (expert off + on), 4 model-ID inputs present + editable, "Model IDs" title, accordions gone, TrialBadge absent on Text Cleanup while present on STT (positive control), stale copy gone; Shortcuts: Auto-Paste + Auto-Capitalize rows gone, Auto-Send + Paste Delay present and not dimmed/disabled, heading kept | pixels/fonts/Windows text-scale; the save round trip (no backend in preview); the D2 warning text; the REG-3 skip banner |
| Windows real target (build) | `scripts/windows-build.sh conductor/story-7-9` on the laptop (`windows-build-145de64.log`) | exit 0, release build in 2m12s, exe proven fresh, contains `145de64` | the code compiles, links and bundles on Windows incl. the `cfg(windows)` arms the Linux run never compiled (REG-1 Windows branch, `LocalLlmCleanup::model()`) | that it runs correctly on screen |

Structural verdict: **GREEN** on every machine-checkable layer.

## Residual for Andi (the part I provably cannot observe from powerhouse)

Real Windows build at `D:\apps\klarvo` (Settings → About must show `145de64`):
1. Settings → Advanced: only live keys visible (Speech-to-Text: 3 prompts · Text Cleanup: 4 model-ID fields flat under "Model IDs", no accordion · Audio (expert): thresholds · System). Design eye: does the flattened Text Cleanup section look right?
2. Settings → Shortcuts → Paste & Behavior: no Auto-Paste toggle; Auto-Send and Paste Delay usable (not greyed).
3. Set DeepSeek model ID to a wrong value (e.g. `deepseek-chatx`), dictate once → raw text pasted + pill warning `Model 'deepseek-chatx' not found — check Advanced → Model IDs`; `Klarvo.log` carries the model ID on the request line. Then clear the field → cleanup works again without restart (hot-reload).
4. P1 round trip: set a model ID in Advanced → Save; change any Settings value → Save; reopen Advanced → the model ID is still there.

A failure in 1–4 re-opens the story (status → in-progress), cause isolated first, fix via a fresh dev worker.

## Andi's GATE-4 — 2026-09-12, build 145de64 (exe 18:39:46 CEST, About showed `Build nogit`)

| Point | Result | Evidence |
|---|---|---|
| 1 Advanced panel | ✅ passt | Andi |
| 2 Shortcuts panel | ✅ passt | Andi |
| 3 wrong model ID | ❌ **warning unreadable** | see below |
| 4 P1 round trip | ✅ passt | Andi |

**Pre-note on `Build nogit`:** structural, not a stale binary. `sync-and-build.ps1`'s robocopy excludes `.git`, so `build.rs`'s `git rev-parse` always failed on the mirror; the "klarvo.exe enthält 145de64" line in `windows-build.sh` is inferred from the SHA comparison, never read from the exe. Freshness proven by the exe mtime (18:39:46, 4 min after the 18:35 commit). Fixed separately in `fix/about-build-hash` (hash handed over via `KLARVO_BUILD_HASH`).

**Point 3, observed:** model ID `Hallo, hallo` (log: `dsad s`, `odisaf`) → "Transcribing…" → amber warning for a few ms → "Done" → text pasted. Andi read the paste as "cleanup worked anyway".

**Point 3, isolated (log lines 9536–9560 in `…/Local/com.klarvo.voice/logs/Klarvo.log`):**
```
[WARN][klarvo_lib::pipeline] [pipeline] LLM cleanup failed (non-retryable), falling back to raw text:
  API error 400: The supported API model names are deepseek-flash, deepseek-v4-pro, but you passed odisaf.
[INFO][klarvo_lib::paste::windows] [paste] Ctrl+V sent to HWND=0x180baa (12 chars)
```
The backend behaves as D2 specified: non-retryable 400 → degrade → **raw text** pasted (the paste was raw STT output, not a cleanup). Two defects sit in front of that:

- **3a — the pill warning is structurally unreadable on Desktop (root: Epic 12-1 native re-port, `native_pill.rs`).** `process_audio` emits `warn(...)` and returns; the shell pastes and emits `done(...)` milliseconds later. `WM_PILL_SET_STATE` unconditionally replaces the display and clears `warning_at`; the comment in `native_pill.rs` states the assumption: "normally the follow-up Done/Error SET_STATE overrides first". That assumption fits the fallback ladder (warning shown *while* the retry runs) but not the degrade-to-raw path, where no time gap exists. Consequence: **every** "Cleanup failed — raw text inserted" warning on Windows is invisible, not only D2's. 12-1's device verification never covered this path with a stopwatch.
- **3b — DeepSeek's live wording does not match `is_model_not_found_error`.** The live body is `{"error":{"message":"The supported API model names are …, but you passed X.", …}}`; `llm/mod.rs` keeps only `error.message` (the `code` field is dropped before the predicate runs). None of the needles (`model_not_found`, `model not found`, `invalid_model`, `unknown model`, `model`+`does not exist`, `decommissioned`) occur in that message. The 7-9 fixture put the **whole JSON body incl. `"code":"model_not_found"`** into `message`, a shape the live extraction never produces — so `spec_model_not_found_warning_names_the_model` is green against a synthetic input (external-fact verification gap, see feedback memory). Even with 3a fixed, DeepSeek would show the generic degrade text, not `Model 'X' not found — check Advanced → Model IDs`. OpenAI/Groq wording ("The model `x` does not exist…") does match.

**Not yet observed:** the hot-reload half of point 3 (clear the field → cleanup works again without restart). To run on the next build.

**Verdict:** story re-opened (`in-progress`) per the routing rule. Fix via a fresh dev worker after the display decision for 3a is taken (human gate: how long / in which form the degrade warning stays visible).
