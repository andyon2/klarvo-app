# Story 10-4 — HANDOVER to a fresh session (2026-06-29)

**Status: UNSOLVED.** Andi stopped the session: the native live-preview overlay still does NOT
visually match its design source, and the real root cause has NOT been found. The fixes committed
this session did not work (Andi confirmed on a fresh real-machine build — screenshot
`screenshots/ist-after-fidelity-fix-still-wrong-2026-06-29.png`).

Do **not** trust the `review` status or my prior "fixed" claims. Treat 10-4 as **in-progress, root
cause unknown.**

---

## The actual goal (unchanged, now correctly framed)

The native GDI floating **live-preview** overlay must look like its React design source. Before Epic 10's
native rebuild (Story 10-2), the floating preview *was* a React component and matched the Settings
"Live-Vorschau" exactly. The 10-2 GDI re-implementation (`src-tauri/src/native_preview.rs`) drifted.
Andi's acceptance test: **the floating preview must match the Settings → Appearance "Live-Vorschau"
card.**

- **SOLL (settings card):** `src/components/settings/AppearanceContent.tsx:118-134`.
- **TRUEST SOLL (under-used — DO THIS):** the OLD floating preview that 10-2 deleted —
  `git show e6e05b9~1:src/PreviewPanel.tsx` (the commit just before native_pill landed). It is the
  *exact* thing that was replaced; compare `native_preview.rs` against IT element-by-element, not only
  against the settings card. I only compared against the settings card.
- **Native port:** `src-tauri/src/native_preview.rs`.

---

## RULED OUT — do not re-investigate (each was verified)

1. **DPI scale = 1.0 bug → REFUTED.** Andi's device log: `GetDpiForMonitor=120 GetDeviceCaps(legacy)=120
   scale_was=1.250 scale_real=1.250`. Both APIs agree on his single monitor; the scale was always
   correct. The `GetDpiForMonitor` fix was a **no-op**. (Reverted.)
2. **`overlayScale` size-knob → wrong approach.** Coupled model scaled position+geometry together →
   pill drifted, preview failed to render at >1.0. (Reverted.)
3. **Pill size → fine.** Measured **249 px** on a 1918 px screenshot = designed 200×1.25 = **250 px**.
   The pill renders 1:1. The problem is the **PREVIEW**, not the pill. Stop measuring the pill.
4. **font-size mapping (small/medium/large = 11/13/15)** matches between native (`native_preview.rs`
   `from_app_config`) and SOLL (`AppearanceContent.tsx:15`). Not the drift.
5. **Appearance settings wiring** (Settings → preview): Andi's smoke confirmed settings ARE effective
   (colors/size change the preview). The chain works.

## TRIED THIS SESSION but INSUFFICIENT (committed `bba8347`, Andi says still wrong)

Three fidelity changes to `native_preview.rs` — they did NOT make it match. **Re-evaluate whether to
keep or revert them; they are not the answer (or not the whole answer):**
- line-height → 1.625 (manual word-wrap + per-line `DT_SINGLELINE|DT_VCENTER` draw; helper
  `wrap_text_lines`). Matches SOLL `leading-relaxed`.
- font: `CreateFontW("Inter")` → resolve Inter/system-ui → **Segoe UI**, mono → Consolas. Rationale:
  the web app bundles only Geist (`public/fonts/Geist-*.woff2`), so the SOLL's `'Inter', system-ui, …`
  stack falls back to Segoe UI on Windows — it never shows real Inter.
- padding: `INNER_PAD_LR` 14 → 12 (SOLL `padding: 8px 12px`).

## STRONGEST UNEXPLORED LEADS (my observations from screenshot #7 — UNVERIFIED hypotheses)

1. **Border looks far more prominent in the native preview than the SOLL.** SOLL border is faint
   (`rgba(42,195,168,0.25)` = 25 % alpha). In screenshot #7 the native border reads bright/saturated.
   → Suspect the **border alpha (or width) handling in the GDI/tiny-skia compositing** is wrong
   (`native_preview.rs` step 3 border stroke + `composite_text_mask` / `UpdateLayeredWindow`
   premultiplied-alpha path). This is the most likely real cause and was NOT checked.
2. **Width never compared.** Native preview width = `w_base × k` (`w_base` 260/320/400 logical,
   `compute_preview_geometry`). The old `PreviewPanel.tsx` floating width was never pulled from git —
   the native may simply be a different width than the original. Get it from `e6e05b9~1:src/PreviewPanel.tsx`.
3. **Possible premultiplied-alpha / blending bug** generally: the card bg is `rgba(25,25,25,0.96)` and
   GDI layered windows use premultiplied BGRA. If alpha is mishandled, the whole box reads
   different-weight than CSS. Check `copy_rgba_to_bgra`, `composite_text_mask`, the `BLENDFUNCTION`.

---

## Git state (branch `conductor/epic-10`)

- baseRef (before any 10-4 code): `60b9e71`.
- Clean pre-fidelity baseline (after reverting overlayScale/DPI): `297837d` — native files identical to
  baseRef. **If you want a clean slate, reset the code to here.**
- Fidelity attempt (insufficient): `bba8347` (native_preview.rs only).
- HEAD: `e69d8fd`. Full range: `git log --oneline 60b9e71..HEAD`.
- `conductor/epic-10` is NOT merged into `feat/native-desktop-overlays`.

## Verification constraints (IMPORTANT — these bit me repeatedly)

- **Full `cargo check --target x86_64-pc-windows-gnu` is INFEASIBLE in WSL** — `whisper-rs-sys`/`llama`
  C++ deps don't cross-build (ggml `stdbool.h`/`THREAD_POWER_THROTTLING_STATE`). Use the **scratch
  harness** at `scratchpad/win32-check-10-4/` (recipe: `gate4-evidence/10-1/win32-surface-check.md`) —
  it compiles `native_pill.rs` + `native_preview.rs` in isolation with `fake_tauri` shims.
- **Harness blind spot:** it compiles the native files alone → it does NOT catch caller mismatches. If
  you change a `#[cfg(windows)]` fn **signature**, `grep -rn "FnName::" src-tauri/src/` ALL call sites
  (this cost two of Andi's build cycles: `NativePill::create` had 3 call sites — `lib.rs`,
  `pipeline.rs`, `commands/misc.rs`). See memory `feedback_windows_cross_compile_verify`.
- **The VISUAL result is ONLY verifiable on Andi's real Windows build** (`scripts/sync-and-build.ps1`
  + `rsign`). WSL cannot render the GDI layered overlay. Don't claim visual fidelity from WSL.

## Process lesson for the fresh session (please internalise)

I repeatedly **guessed a cause → built → claimed verified → was wrong**, three times. Do the opposite:
1. Get BOTH renderings as images at the SAME scale (Andi's screenshot #7 has both; ask for a tight crop
   of just the two boxes if needed). **Measure**, don't eyeball.
2. Anchor to `e6e05b9~1:src/PreviewPanel.tsx` (the real old floating preview), and read the GDI
   compositing path for an **alpha/border bug** before touching layout again.
3. Change ONE thing, have Andi rebuild ONCE, compare. No stacking of unverified guesses.

## Key paths

- Native preview: `src-tauri/src/native_preview.rs` (`from_app_config` ~94, `compute_preview_geometry`
  ~406, render fn ~540+, `wrap_text_lines` helper added this session).
- SOLL card: `src/components/settings/AppearanceContent.tsx`.
- Old floating preview (git): `git show e6e05b9~1:src/PreviewPanel.tsx`.
- Live config: `/mnt/c/Users/Andi/AppData/Roaming/com.klarvo.voice/config.json` (preview* fields).
- Log: `/mnt/c/Users/Andi/AppData/Local/com.klarvo.voice/logs/Klarvo.log`.
- Evidence + screenshots: this dir (`gate4-evidence/10-4/`).
- Full saga detail: this story's Change Log + `docs/backlog.md` (Epic-10 scaling section).
