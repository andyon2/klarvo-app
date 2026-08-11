---
story: "11.6"
epic: "11"
title: "Line spacing as an Appearance setting"
status: done
track: L2-feature
gatedBy: []
buildsOn: ["6.3", "11-2", "11-3"]
enabledBy: []
inputDocuments:
  - docs/backlog.md#11-6 — Zeilenabstand als Appearance-Setting — Source: Andi 2026-07-08 Device-Test
  - _bmad-output/project-context.md
  - android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt — hardcoded `setLineSpacing(0f, 1.7f)`
  - src-tauri/src/native_preview.rs — hardcoded `PREVIEW_LINE_HEIGHT = 1.625`
  - src/components/settings/AppearanceContent.tsx — `previewFontSize` control, the precedent this story mirrors
---

# Story 11.6: Line spacing as an Appearance setting

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

> **Epic 11 — Cross-Platform Live-Preview.** 11-1..11-4 are done; this is the last open Epic-11
> item on record. Source is a single backlog line (`docs/backlog.md` §11-6), not a fully-specced
> epic entry — the design/UI choices it left open were put to Andi and are now **settled**
> (see "DESIGN DECISIONS"). Nothing below needs a taste call during dev.

## ✅ DESIGN DECISIONS — settled by Andi, 2026-08-10 (GATE-1)

The backlog source is one paragraph: *"Zeilenabstand (line-height) als einstellbares Setting in
die Appearance-Kategorie aufnehmen (analog Font-Größe/-Familie/Farbe). Aktuell fix
`setLineSpacing(0f, 1.7f)`. Desktop-Gegenstück (CSS `--k-leading` / `leading-relaxed`) mitdenken
für Cross-Platform-SSOT. Neuer Config-Key (camelCase, Backend-Locale-Files), Android- +
Desktop-Wiring."* It pinned only: a new Appearance-category setting, one new camelCase config key,
wired on both Android and Desktop. The four choices it did not pin were decided as follows —
**implement these, do not re-open them at GATE-4**:

1. **Control type → 3-tier `KSegmented`.** Exactly the `previewFontSize` pattern
   (`AppearanceContent.tsx:319-335`), not a continuous `KSlider`. Rationale: three discrete steps
   are quicker to hit than hunting for a good value on a slider, and the control sits directly under
   "Schriftgröße", which has the same shape. Value domain `"small" | "medium" | "large"`.
2. **Values → same tier semantics, platform-tuned numbers; "medium" is a no-op.** The two renderers
   (Android `TextView.setLineSpacing` vs. Desktop raw GDI line stepping) keep their own multipliers,
   like `previewFontSize` already does (Desktop `FONT_PX_MAP` 11/13/15 vs. Android `FONT_PX_SP`
   13/15/18). "medium" = each platform's current hardcoded value — **Android `1.7`**
   (`ListeningPanelView.kt:355`), **Desktop `1.625`** (`native_preview.rs:48`) — so nothing changes
   visually until the user touches the control. Explicitly rejected: identical multipliers on both
   platforms, because that would silently alter today's Desktop appearance.
   **Residual — settled 2026-08-10 at the code-review gate (finding D2/F1):** the ±0.15 first-pass
   offset was widened to a symmetric **±0.30 em** offset, per-platform normalized because Android's
   `setLineSpacing(0f, mult)` multiplies the font's *natural line height* (~1.2× text size) while
   Desktop's GDI line stepping and CSS `lineHeight` multiply the *font size* directly — so the two
   platforms need different raw deltas to move the same ±0.30 em: Desktop 1.325 / 1.625 / 1.925
   (±0.30), **Android 1.45 / 1.7 / 1.95** (±0.25).
   **Further residual — settled 2026-08-10 at the re-review gate (finding R-D1, Desktop only):**
   the Desktop `"small"` value was raised from 1.325 to **1.35** — Segoe UI (the family the default
   `previewFontFamily` resolves to on Desktop) has a natural line cell of ~1.330 em, and
   `native_preview.rs`'s `DrawTextW` call draws without `DT_NOCLIP` into a rect exactly `line_h`
   high, so a multiplier below ~1.330 risked clipping diacritics/descenders. Final Desktop values:
   **1.35 / 1.625 / 1.925** (an asymmetric -0.275 em / +0.300 em step). Android is UNCHANGED at
   1.45 / 1.7 / 1.95 — this fix is Desktop-only, since the clipping risk is specific to GDI's
   unclipped `DrawTextW` rect. What remains for GATE-4 is no longer choosing a step size — only
   confirming these values look right on a real Android device and a real Windows build.
3. **Labels → German `"Zeilenabstand"` with `"Kompakt" / "Normal" / "Locker"`.** Rejected:
   `"Klein/Mittel/Groß"` (word-for-word reuse of the font-size labels reads wrong for spacing) and
   an English label. The section stays mixed-language as it already is — not this story's job.
4. **Settings preview card → wire it up.** The card at `AppearanceContent.tsx:124-140` must switch
   its hardcoded `className="leading-relaxed"` for an inline `lineHeight` style driven by the new
   field, mirroring the `fontSize` inline-style pattern one property below. Adjusting the setting
   without seeing it change was rejected as a blind flight.

### Addendum — settled at the code-review gate, 2026-08-10 (finding D1)

The Settings preview card (`AppearanceContent.tsx`'s `LINE_SPACING_MULT`) renders on Android with
the **Desktop** multipliers, not Android's own `ListeningPanelView.kt` values — the card and the
Android panel diverge. **Decision: accept as precedent-consistent, do not platform-branch or hide
the card.** This is the same already-shipped divergence `FONT_PX_MAP` (11/13/15) vs. Android's
`FONT_PX_SP` (13/15/18) already has for `previewFontSize`. Consequence for GATE-4: the **GATE-4b
visual judgement of the line-spacing values is made on the real Android preview panel
(`ListeningPanelView`), not on the Settings preview card** — the card is a Desktop-accurate but
Android-inaccurate proxy, same as it already is for font size.

## Story

As a user of the live-preview transcription panel (desktop and Android),
I want to control how airy/compact the transcript's line spacing looks, alongside the existing
font-size/-family/-color Appearance controls,
so that I can tune the panel's readability to my own taste instead of living with the currently
hardcoded, platform-mismatched line-height values.

## Scope boundaries (read before touching code)

**IN:**
- One new config field (`AppConfig.preview_line_spacing: String` in Rust, camelCase
  `previewLineSpacing` on the wire) with a 3-tier value domain (`"small" | "medium" | "large"`,
  mirroring `preview_font_size`'s exact domain shape) and a serde default that preserves today's
  behavior (see DESIGN DECISION 2).
- Full cross-platform wiring for the new field, following the **exact** `preview_font_size`
  precedent end-to-end (this is the load-bearing reference for every file this story touches):
  - `src-tauri/src/config/mod.rs`: struct field + doc comment + `default_preview_line_spacing()`
    fn, mirroring `preview_font_size`/`default_preview_font_size()` (`:789-790`, `:1031-1033`).
  - `src-tauri/src/commands/settings.rs`: `SettingsPatch` field + `Default` impl entry + merge
    (`unwrap_or`) + the `save_settings` command's parameter list + the settings-read response
    struct, mirroring every `preview_font_size` touch point (`:164`, `:223`, `:370-371`, `:472`,
    `:570`, `:684`).
  - `src-tauri/src/lib.rs`: the `SettingsView` DTO field (`:235`, struct declared at `:121`) plus
    the three `SettingsView` literal-construction sites inside `#[cfg(test)] mod tests` (`:1037`)
    that already list `preview_font_size`, mirroring every one of its touch points (`:1155`,
    `:1227`, `:1293`) — missed by the story's original file-by-file plan (added at the 2026-08-10
    code-review gate, finding P3; mislabelled as "`AppConfig`/config-construction sites" and
    corrected to its actual designation and line numbers at the 2026-08-10 re-review, finding
    R-P2/R-P3).
  - `src-tauri/src/native_preview.rs`: replace the hardcoded `PREVIEW_LINE_HEIGHT` const (`:48`)
    with a `line_height_mult` field on `PreviewConfig`, populated from
    `cfg.preview_line_spacing` in `PreviewConfig::from_app_config` (mirrors the `font_px` mapping
    at `:95-99`), and used at every current `PREVIEW_LINE_HEIGHT` call site (`:657` line-height
    calc, and the doc-comment references at `:175`, `:419`, `:454-456`, `:653-654`, `:738`,
    `:1109-1111` — update text where the value is no longer a fixed constant).
  - `src/types.ts`: `previewLineSpacing: string` field, mirroring `previewFontSize` (`:106-107`).
  - `src/tauri-commands.ts`: default value + save-command parameter, mirroring `previewFontSize`
    (`:78-82`, `:98`, `:312`, `:369`).
  - `src/components/SettingsPanel.tsx`: `localPreviewLineSpacing` state (default matches the new
    config default) + load-on-mount sync (the exact spot 6.3's own comment warns about — **"MUST
    be here or Save stays dirty forever"**, `:339-340`) + dirty-check comparison + inclusion in
    the save payload, mirroring every `previewFontSize` touch point (`:81`, `:216-218`, `:339-340`,
    `:425`).
  - `src/hooks/useSettings.ts`: `previewLineSpacing` in the `handleSaveSettings` wrapper's
    positional pass-through between `SettingsPanel`'s `onSaveSettings` prop and
    `tauri-commands.ts`'s `saveSettings`, mirroring `previewFontSize` (`:94`, `:117`) — missed by
    the story's original file-by-file plan (added at the 2026-08-10 code-review gate, finding P3).
  - `src/components/settings/AppearanceContent.tsx`: new prop pair
    (`localPreviewLineSpacing`/`setLocalPreviewLineSpacing`) threaded from `SettingsPanel.tsx`,
    plus a new control block placed directly after the existing "Font-size picker" block
    (`:319-335`, same `KSegmented` pattern per DESIGN DECISION 1) and the live-preview-card wiring per
    DESIGN DECISION 4.
  - `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt`: `previewLineSpacing: String = "medium"` (NOT `"small"` — unlike `previewFontSize`, the default must be the no-op tier per DESIGN DECISION 2)
    field on `Config` + JSON parse (`json.optString(...)`) + inclusion in the config-construction
    call, mirroring `previewFontSize` (`:115`, `:365`, `:436`).
  - `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt`: a `LINE_SPACING_MULT` map
    (mirrors `FONT_PX_SP`, `:49`) consulted in `applyAppearance` (`:251-269`) to call
    `transcriptTextView.setLineSpacing(0f, mult)`, replacing today's hardcoded call in the `init`
    block (`:355`) — the init-block call becomes the pre-`applyAppearance` bootstrap default only
    (same relationship `textSize = 15f` already has to `FONT_PX_SP` in that same init block,
    `:353`).
- New/updated unit tests: one Rust `#[cfg(test)]` spec mirroring
  `spec_preview_font_size_config_field_default` (`config/mod.rs:4227-4253`) — camelCase key
  present, missing-key deserialization succeeds with the default, no migration write fires.
- Sync `src-tauri/gen/android/...` mirrors of the touched Android files if the Android build
  process requires it (check whether `gen/android` is auto-synced by `android-build.sh`/
  `android-smoke.sh` from `android/kotlin-src` before hand-editing the `gen/` copies — 11-2/11-3/
  11-4 precedent treats `android/kotlin-src/` as the single source of truth and `gen/` as
  generated).

**OUT (do not touch):**
- Any other Appearance field (font-size, font-family, colors, blur, border, corner-radius,
  panel-form preset) — this story adds exactly one new field, does not restyle or re-order
  existing ones beyond inserting the new control block.
- The 11-3 fixed-height/scroll/auto-scroll transcript mechanics, the 11-4 bubble Z-order/overlay
  mechanism — unrelated subsystems, not touched by this story.
- Any locale/i18n file — a repo-wide search during story creation found **no `locales/` directory
  or i18n file anywhere in this repo** (the backlog's "Backend-Locale-Files" phrase appears to be
  a generic reminder that does not apply here; there is no such mechanism to update). If the dev
  agent finds one that this search missed, treat it as an OPEN ITEM and flag rather than silently
  skip.
- Desktop's `--k-leading` CSS variable — a repo-wide search found **no `--k-leading` custom
  property anywhere** in `docs/design/` or `src/`; only the Tailwind utility class
  `leading-relaxed` (a fixed `1.625`) is used, in `AppearanceContent.tsx:136` (Settings preview
  card) and unrelated prose blocks (`Onboarding.tsx`, `FeedbackModal.tsx`, etc. — those are plain
  body-text styling, not preview-panel rendering, and are OUT of scope). The backlog's `--k-leading`
  reference appears aspirational/inaccurate against the current codebase; do not invent a design
  token that doesn't exist — wire the new config value directly into `native_preview.rs`'s render
  path and (per DESIGN DECISION 4) `AppearanceContent.tsx`'s inline style, not into a nonexistent CSS var.

## Acceptance Criteria

**AC-1 (New config field exists, cross-platform, camelCase, backward-compatible default):**
Given a `config.json` written before this story (no `previewLineSpacing` key),
When the app loads it (Rust `AppConfig` deserialization, or Android `KlarvoApi.Config` JSON
parse),
Then deserialization succeeds without error, `preview_line_spacing` (Rust) /
`previewLineSpacing` (Android/TS) resolves to a default that reproduces today's hardcoded
line-height behavior (Android `1.7`, Desktop `1.625` — see DESIGN DECISION 2 default choice),
And no config migration write fires for this field alone (same guarantee `preview_font_size` has,
`config/mod.rs:4227-4253`),
And the serialized JSON key is `previewLineSpacing` (camelCase), never `preview_line_spacing`.

**AC-2 (Settings UI: new Appearance control, wired end-to-end, no dirty-forever trap):**
Given the desktop Settings panel's Appearance section with Live Preview enabled,
When the user changes the new line-spacing control and clicks Save,
Then the change persists to `config.json` under `previewLineSpacing` and survives a Settings
panel reload (loaded value matches what was saved — the exact regression class 6.3's own comment
warns about, `SettingsPanel.tsx:339`),
And before Save, the dirty-state indicator (Save button enabled/"unsaved changes") reacts to a
change in the new control exactly as it does for `previewFontSize` today,
And the live-preview card in the Settings panel updates its rendered line spacing immediately
when the control changes, without requiring Save (per DESIGN DECISION 4 — flag if this turns out
infeasible or unwanted).

**AC-3 (Desktop native preview renders the configured line spacing):**
Given `previewLineSpacing` is set to a non-default tier and Live Preview is active,
When the native Win32 overlay renders transcript text (`native_preview.rs`'s GDI line-stepping,
`:657`),
Then the rendered line-to-line vertical spacing reflects the configured multiplier (no longer the
hardcoded `PREVIEW_LINE_HEIGHT = 1.625` constant for all users),
And this is verified via `scripts/sync-and-build.ps1` real Windows build + manual visual check —
Linux `cargo check`/tests cannot observe GDI rendering (project-context.md "Release-Build blind
spot" rule).

**AC-4 (Android preview panel renders the configured line spacing):**
Given `previewLineSpacing` is set to a non-default tier and Live Preview is active on Android,
When `ListeningPanelView.applyAppearance` runs (on panel show, same call site as
`previewFontSize`'s `sizeSp` today, `:251-269`),
Then `transcriptTextView`'s line spacing reflects the configured multiplier via
`setLineSpacing(0f, mult)`,
And this is verified via `scripts/android-smoke.sh` build/install plus a real-device (Xiaomi/
HyperOS, never emulator) visual check per project-context.md's Android testing rule.

**AC-5 (No regression to existing Appearance fields):**
Given the new field and control are added,
When any other Appearance setting (font-size, font-family, colors, blur, border, corner-radius,
panel-form) is changed and saved,
Then its behavior is byte-identical to pre-story — this story is purely additive to the
Appearance category's data model and UI, not a refactor of the existing fields.

**DoD (surface-class — mirrors project testing rules):**
- New Rust unit test (`spec_preview_line_spacing_config_field_default`, mirroring
  `config/mod.rs:4227-4253`) green.
- `cargo check`/`cargo test` green on Linux (necessary but **not sufficient** — see AC-3's own
  blind-spot note).
- `node scripts/gen-android-theme.mjs --check` clean (unaffected — no theme/color changes
  expected).
- `scripts/android-smoke.sh` clean build/install; confirm `.rs`/`.ts`/`.tsx`/`.kt` files touched
  match the "IN" list above (this story is NOT Kotlin-only — it is genuinely cross-cutting, unlike
  11-4 — so the heavier full `tauri android build`/full frontend rebuild may be needed for the new
  Settings UI control to actually appear on Android's shared React settings surface; see
  `reference_android_frontend_needs_full_tauri_build` — confirm this before relying on the lighter
  `android-smoke.sh`-only path for the **UI** portion, though the Kotlin-only render change
  (`ListeningPanelView.kt`) is covered by it).
- **Real Windows build required (GATE-4, AC-3)** — Andi confirms the new line-spacing control
  changes the desktop native preview's rendered line spacing, and that Settings save/reload/dirty
  state work correctly.
- **Real Android device required (GATE-4, AC-4)** — Andi confirms the new control appears in
  Android's Settings/Appearance UI, changes the preview panel's rendered line spacing, and
  survives a save/reload.
- **Design decisions 1-4 are settled (GATE-1, 2026-08-10); step size is settled (review gate,
  2026-08-10, finding D2; Desktop "small" further settled at the re-review gate, finding R-D1)** —
  control type, value semantics, labels, preview-card wiring and the ±0.30 em step size must NOT be
  re-opened at GATE-4. The single remaining judgement call for this round: do the **1.35 / 1.625 /
  1.925 (Desktop)** and **1.45 / 1.7 / 1.95 (Android)** values look right for "Kompakt"/"Locker" on
  the real device and the real Windows build.
- **Mandatory `docs/surface-smoke-checklist.md` items for a new-config-key story
  (project-context.md:62), executed 2026-08-10 during the review-fix pass — mechanical check, not
  a self-attestation:**
  - **#1 camelCase config key:** verified `AppConfig::preview_line_spacing` carries
    `#[serde(default = "default_preview_line_spacing")]` under the struct-level
    `#[serde(rename_all = "camelCase")]` (`config/mod.rs:795-796`), and the inversion-guard test
    `spec_preview_line_spacing_config_field_default` (`:4272-4303`) asserts the on-disk key is
    `previewLineSpacing`, not `preview_line_spacing`. **Pass.**
  - **#3 reactive re-read, not mount-only load:** the native preview overlay is not a persistent
    webview window — `pipeline.rs:751-757` recreates it via `PreviewConfig::from_app_config`
    (fresh config-lock read) every time a recording starts, so a value saved in Settings takes
    effect on the next preview show, not only after an app restart (same recreate-per-recording
    pattern as the rest of `native_preview.rs`). **Pass.**
  - **#6 multi-hop save chain traced end-to-end:** `preview_line_spacing` walked through every hop
    — `SettingsPatch` (`commands/settings.rs:166`) → merge (`:375-376`) → settings-read response
    (`:693`) → `lib.rs`'s `SettingsView` DTO field + three test fixtures (`:235, 1155, 1227,
    1293`) → `types.ts:108-109` → `tauri-commands.ts:99, 314, 372` → `SettingsPanel.tsx`
    state/resync/dirty-check/save-payload
    (`:220-221, 345-346, 432, 571`) → `useSettings.ts:94, 117` → `AppearanceContent.tsx` prop
    threading (`:58-59, 100`). No intermediate hop drops the field. **Pass.**

## Tasks / Subtasks

- [x] **Task 1 — Rust config field + backend wiring** (AC-1)
  - [x] 1.1 Add `preview_line_spacing: String` to `AppConfig` (`config/mod.rs`) with
    `#[serde(default = "default_preview_line_spacing")]`, doc comment mirroring
    `preview_font_size`'s (`:787-790`).
  - [x] 1.2 Add `default_preview_line_spacing() -> String` mirroring
    `default_preview_font_size()` (`:1031-1033`), returning the OPEN-ITEM-2 default tier.
  - [x] 1.3 Add the field to every `AppConfig` test-fixture construction site that currently lists
    `preview_font_size` (`config/mod.rs:1110, 2167, 3471` and any other constructor found by
    `grep -n preview_font_size config/mod.rs`).
  - [x] 1.4 Add `spec_preview_line_spacing_config_field_default` test, mirroring
    `spec_preview_font_size_config_field_default` (`:4227-4253`) exactly (camelCase key check,
    missing-key deserialization, no migration write).

- [x] **Task 2 — `SettingsPatch`/`save_settings`/settings-read wiring** (AC-1, AC-2)
  - [x] 2.1 Add `preview_line_spacing: Option<String>` to `SettingsPatch` (`settings.rs:164`) +
    its `Default` impl entry (`:223`) + merge line (`:370-371`).
  - [x] 2.2 Add the parameter to the `save_settings` Tauri command signature (`:472`) and its
    `SettingsPatch` construction (`:570`).
  - [x] 2.3 Add the field to the settings-read response struct/command (`:684` region).

- [x] **Task 3 — Desktop native preview rendering** (AC-3)
  - [x] 3.1 Replace `PreviewConfig`'s implicit dependence on the `PREVIEW_LINE_HEIGHT` const
    (`native_preview.rs:48`) with a `line_height_mult: f32` field, populated in
    `PreviewConfig::from_app_config` from `cfg.preview_line_spacing` (mirror the `font_px` match
    arm at `:95-99`; use the OPEN-ITEM-2 multiplier values).
  - [x] 3.2 Update the line-height calc at `:657` (and any other `PREVIEW_LINE_HEIGHT` read) to
    use `s.config.line_height_mult` instead of the const. Remove or repurpose the const if no
    longer referenced elsewhere.
  - [x] 3.3 Update stale doc comments referencing a fixed `1.625`/`leading-relaxed` at `:175,
    419, 454-456, 653-654, 738, 1109-1111` to describe the now-configurable value.

- [x] **Task 4 — Frontend types + save/load wiring** (AC-1, AC-2)
  - [x] 4.1 `src/types.ts`: add `previewLineSpacing: string` (mirror `:106-107`).
  - [x] 4.2 `src/tauri-commands.ts`: default value, save-command parameter (mirror `:78-82, 98,
    312, 369`).
  - [x] 4.3 `SettingsPanel.tsx`: `localPreviewLineSpacing` state + load-on-mount sync (the
    dirty-forever trap spot, `:339-340`) + dirty-check (`:425`) + save-payload inclusion, + prop
    threading into `AppearanceContent`.

- [x] **Task 5 — Appearance UI control** (AC-2, DESIGN DECISIONS 1/3/4)
  - [x] 5.1 Add the new `KSegmented` control block to `AppearanceContent.tsx`, placed after the
    Font-size block (`:319-335`), using the OPEN-ITEM-3 first-pass labels.
  - [x] 5.2 Wire the live-preview card (`:124-140`) to reflect the new value live (inline
    `lineHeight` style replacing the hardcoded `leading-relaxed` class) per DESIGN DECISION 4.
  - [x] 5.3 If `hidePanelForm`/`hideBgBlur`-style Android hiding is needed for this control,
    determine and document why (first-pass expectation: **not needed** — unlike panel-form-preset
    and bg-blur, line-spacing is meaningful and renderable on both platforms, so no
    `hideLineSpacing` prop is expected — confirm this holds and don't add one speculatively).

- [x] **Task 6 — Android config + rendering wiring** (AC-1, AC-4)
  - [x] 6.1 `KlarvoApi.kt`: add `previewLineSpacing: String = "medium"` to `Config` (mirror the shape of
    `:115`), JSON parse (`:365`), constructor call inclusion (`:436`).
  - [x] 6.2 `ListeningPanelView.kt`: add a `LINE_SPACING_MULT` map (mirror `FONT_PX_SP`, `:49`,
    using the OPEN-ITEM-2 Android multiplier values), consult it in `applyAppearance` (`:251-269`)
    to call `transcriptTextView.setLineSpacing(0f, mult)`, replacing the hardcoded call at `:355`
    (which becomes the pre-`applyAppearance` bootstrap default only, matching `textSize = 15f`'s
    existing relationship to `FONT_PX_SP` at `:353`).
  - [x] 6.3 Sync to `src-tauri/gen/android/...` mirrors if that directory is not auto-generated
    by the build scripts (check `scripts/android-build.sh`/`android-smoke.sh` first — do not
    hand-duplicate if it's generated). Confirmed: `gen/android` is gitignored and auto-synced by
    `android-build.sh`'s Kotlin-copy step — no hand-duplication needed.

- [x] **Task 7 — Verification**
  - [x] 7.1 `cargo test` green (new + existing), `cargo check` green.
  - [x] 7.2 `node scripts/gen-android-theme.mjs --check` clean.
  - [x] 7.3 `npm run build` (TypeScript strict mode gate) green.
  - [x] 7.4 `scripts/android-smoke.sh` clean build/install — **DONE 2026-08-11 at GATE 4.** The
    signing-key mismatch below was diagnosed rather than worked around: the app on the device
    carries this machine's `~/.android/debug.keystore` (`e6aaad…`), not `voxlit-debug.keystore`;
    re-signing with the matching key installed cleanly over Andi's live app with no data loss.
    `scripts/android-smoke.sh` then ran green end-to-end (3 JVM tests, Kotlin compile, Gradle
    assembly, `adb install` to `100.112.41.70:5555`). The change reaching the build tree was
    verified explicitly (`grep NATURAL_LINE_BOX` in `gen/android/.../ListeningPanelView.kt`) —
    a 4-second build is otherwise indistinguishable from a no-op.
    ~~**build succeeded, install
    and smoke did NOT run** (corrected 2026-08-10 at the code-review gate, finding D3; the previous
    checked-off state overclaimed). What was executed: the heavier full
    `npx tauri android build --target aarch64` (via `android-build.sh`, confirmed necessary per the
    task's own note — this story touches `.rs`/`.ts`/`.tsx` files, so the React Settings surface
    needed the full frontend rebuild, not just Kotlin). Rust aarch64 cross-compile + Kotlin compile
    + Gradle assembly succeeded end-to-end, producing a signed, `apksigner`-verified APK. What did
    NOT run: `adb install` failed with `INSTALL_FAILED_UPDATE_INCOMPATIBLE` (signing-key mismatch
    against the app already on Andi's device — did not force-uninstall his live app to work around
    it), so the app was never installed and no smoke (emulator or device) ever executed. The only
    runtime evidence for the Kotlin render path (`ListeningPanelView.kt`'s `applyAppearance`/
    `LINE_SPACING_MULT`) is the compile-verify below, not an actual run. This task is left unchecked
    — the conductor runs the install/smoke at GATE 4, not this pass.~~
  - [x] 7.5 **GATE-4a — real Windows build — GRÜN, bestätigt von Andi 2026-08-11.** AC-2 und AC-3
    auf einem echten Windows-Release-Build (`D:\apps\klarvo\...\klarvo.exe`, gebaut 14:27, 44,1 MB)
    abgenommen; insbesondere kein Anschneiden von Umlaut-Punkten oder Unterlängen auf der Stufe
    „Kompakt" (die R-D1-Restbeobachtung zu Segoe UIs ≈1,330-em-Zeilenzelle gegen den ungeclippten
    `DrawTextW`-Rect). Desktop wurde nach dieser Abnahme **nicht mehr angefasst** — die Revision
    vom selben Tag ist Android-only. Nebenbefund für den Backlog: `sync-and-build.ps1` war
    repo-weit gebrochen (`package-lock.json` pinnt `@tauri-apps/plugin-log` nicht, und das Skript
    fährt nach dem robocopy sein eigenes `npm install`, das jeden Pin wieder abräumt).
  - [x] 7.6 **GATE-4b — real Android device — GRÜN, objektiv gemessen 2026-08-11.** Evidence:
    `gate4-evidence/11-6/` (verdict.md + 4 Screenshots + `measure_pitch.py`). Auf dem echten
    Redmi Note 12T Pro (HyperOS) wurde der gerenderte Zeilenabstand des Preview-Panels
    pixelgenau gemessen: **Kompakt 68,75 px · Normal 80,74 px · Locker 92,72 px** gegen die aus
    dem Normal-Basiswert VOR der Messung abgeleiteten Vorhersagen 68,86 / — / 92,61 px —
    Abweichung < 0,2 %. Damit ist AC-4 belegt (der konfigurierte Multiplikator erreicht den
    Renderer) und AC-2s Android-Anteil (Kontrolle erscheint unter „Schriftgröße" mit
    Kompakt/Normal/Locker, „Normal" vorausgewählt, Wert überlebt Save). Schrittweite bei der
    ausgelieferten `previewFontSize`-Voreinstellung „Klein": 12 px pro Stufe. Das ästhetische
    Urteil bleibt Andis Auge; die Messung belegt nur die Wahrnehmbarkeit.
    ~~Andi confirms AC-2 (Settings UI appears/works) and
    AC-4 (preview panel line-spacing renders correctly) on the real Xiaomi/HyperOS device. Per
    the DESIGN DECISIONS Addendum (finding D1): the **visual judgement is made on the real Android
    preview panel (`ListeningPanelView`), not on the Settings preview card** — the card
    intentionally shows the Desktop multipliers even on Android (sanctioned precedent, see D1) and
    is a Desktop-accurate but Android-inaccurate proxy, same as it already is for font size.~~
    (Ursprünglicher Wortlaut oben durchgestrichen — als erledigt ersetzt durch den Messbefund.)

    **NACHTRAG 2026-08-11 — die obigen Zahlen sind überholt (Revision nach Andis Augenschein).**
    Die Messung belegte, dass der Multiplikator den Renderer erreicht — nicht, dass die Skala
    richtig *liegt*. Andi sah am Gerät, dass jede Android-Stufe rund zwei Rasten lockerer saß als
    ihre gleichnamige Desktop-Stufe; er stand bereits auf `small`/`small` und hatte nichts
    Kleineres mehr. Ursache war die Annahme, die diese Story selbst als offen markiert hatte
    („to be confirmed at GATE-4"): der natürliche Zeilenkasten wurde mit ~1,2 geschätzt, gemessen
    sind **1,3285** (80,74 px ÷ 35,75 px ÷ 1,70). Siehe Change Log 2026-08-11 und Commit `117e244`.
  - [x] 7.7 **Schrittweite bestätigt — aber das ±0,30-em-Schema ist abgelöst (2026-08-11).** Die
    ±0,30 em waren korrekt für Desktop und sind dort unverändert. Für Android sind sie hinfällig:
    „per-Plattform normalisiert" beruhte auf dem falschen ~1,2-Faktor, weshalb die Normalisierung
    nicht normalisierte. Android leitet seine Multiplikatoren jetzt aus den Desktop-Werten ab
    (`desktop / NATURAL_LINE_BOX`), statt eine eigene Schrittweite zu wählen — die Schrittweite
    ist damit per Konstruktion identisch mit Desktops. Andi hat die Stufen am 2026-08-11 am Gerät
    abgenommen („sieht gut aus").

### Review Findings

_Source: `bmad-code-review` of committed range `86b5dca..HEAD`, 2026-08-10. Three layers ran and all
returned: Blind Hunter (diff only), Edge Case Hunter (diff + repo), Acceptance Auditor (diff + spec +
context docs). 12 further raised items were dismissed as noise/false positives after verification._

- [x] [Review][Decision] **Settings live-preview card shows the DESKTOP multipliers on Android, where the panel renders different ones** — `AppearanceContent.tsx:34` defines `LINE_SPACING_MULT = { small: 1.475, medium: 1.625, large: 1.775 }` and the card is rendered unconditionally on Android (`SettingsPanel.tsx:839-840`, no `hideLineSpacing` per Task 5.3), while the Android panel uses `1.55/1.7/1.85` (`ListeningPanelView.kt:59`). This is the same sanctioned divergence `FONT_PX_MAP` (11/13/15) vs `FONT_PX_SP` (13/15/18) already has, so it is not an AC violation — but at GATE-4b the card is a ~9 % denser preview of what the panel will actually do, and the card is the surface Andi judges the step size on. Options: (a) accept as precedent-consistent, (b) platform-branch the card map, (c) hide the card's spacing effect on Android. **Resolved 2026-08-10 (finding D1): (a) accepted as precedent-consistent, not platform-branched or hidden — see the "Addendum" under DESIGN DECISIONS and GATE-4 tasks.**
- [x] [Review][Decision] **The ±0.15 step size (the story's only open residual, Task 7.7) — objective numbers before the device round** — Two facts to weigh: (1) At the shipped default `previewFontSize = "small"` (`font_px = 11`) and DPI/text-scale 1.0, `native_preview.rs:679` computes `round(11 × mult)` = **16 / 18 / 20 px** — a 2 px step per tier, which may read as "the control does nothing"; the step also scales with font size and `text_scale`, so it is not constant. (2) Android's `setLineSpacing(0f, mult)` (`ListeningPanelView.kt:280`) multiplies the font's *natural line height* (~1.2 × text size), whereas CSS `lineHeight` and the GDI step multiply the *font size* — so a ±0.15 delta moves Android baselines by ≈0.18 em against the desktop's 0.15 em, and "medium" 1.7 vs 1.625 are not the same rendered spacing. Decide whether to widen the step (and by how much per platform) before or after the device round. **Resolved 2026-08-10 (finding D2): widened to a symmetric ±0.30 em, per-platform normalized — Desktop 1.325/1.625/1.925, Android 1.45/1.7/1.95. See DESIGN DECISION 2's Residual and Task 7.7.**
- [x] [Review][Decision] **Task 7.4 is checked as "clean build/install" but no install and no smoke ever ran** — The checked box's own text (`:291-294`) records `INSTALL_FAILED_UPDATE_INCOMPATIBLE`; the Debug Log (`:430-434`) shows no emulator run, although `scripts/android-emulator-smoke.sh` and `scripts/android-emulator.sh` both exist and project-context.md:63 sanctions the emulator for this mechanical gate (the "never emulator" rule at `:184`/`:399` binds the GATE-4 *visual* gate only). The only runtime evidence for `ListeningPanelView.kt:276,:280` is a compile. Options: (a) run the emulator smoke now, (b) accept and let GATE-4b carry it — then un-check 7.4 or reword it so it does not claim an install that did not happen. **Resolved 2026-08-10 (finding D3): (b) — Task 7.4 un-checked and reworded to state plainly what ran (build) and what did not (install, smoke); no emulator smoke run during this review-fix pass, left for the conductor at GATE 4.**

- [x] [Review][Patch] Comments assert a GATE-4 verification that has not happened — "confirmed at GATE-4 on the real device" / "on a real Windows build", while Task 7.7 is open and Completion Notes say the numbers are unconfirmed. Reword to "to be confirmed at GATE-4". [`android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt:52`, `src-tauri/src/native_preview.rs:102`] — **Fixed 2026-08-10.**
- [x] [Review][Patch] DoD omits the mandated `docs/surface-smoke-checklist.md` items for a new-config-key surface story (#1/#3/#6), required by project-context.md:62 and by the checklist's own "How to use it". The consequence materialised: the `useSettings.ts` hop — trap #6, the exact Epic-6 reset-on-save bug — was found by a compile error, not by the mandated chain walk. The chain is in fact correct today (re-verified during this review); record the executed checks in the DoD. [`_bmad-output/implementation-artifacts/11-6-line-spacing-appearance-setting.md:193-216`] — **Fixed 2026-08-10 — #1/#3/#6 re-verified and recorded in the DoD.**
- [x] [Review][Patch] `src-tauri/src/lib.rs` is missing from both the "IN" scope list and the "exhaustive occurrence list", although `preview_font_size` occurs there at `:233, 1154, 1226, 1292` and the diff touches it in 4 places; Completion Notes claim exactly one missed touch point when it was two. Record accuracy only — the code is correct (Task 2.3 covers it in substance). [`_bmad-output/implementation-artifacts/11-6-line-spacing-appearance-setting.md:79-112, 314-327, 441`] — **Fixed 2026-08-10 — `lib.rs` added to both lists and Completion Notes corrected to "two" touch points.**

- [x] [Review][Defer] No whitelist or validation of the tier string at any layer [`src-tauri/src/commands/settings.rs:375-376`] — deferred, pre-existing (`preview_font_size` has the identical gap)
- [x] [Review][Defer] `PreviewConfig` derives `Default`, and a poisoned config mutex takes `.unwrap_or_default()` → `line_height_mult = 0.0` [`src-tauri/src/pipeline.rs:757`] — deferred, pre-existing (the same default already yields `font_px = 0`, `w_base = 0`; the new field adds no new failure mode)
- [x] [Review][Defer] `Object.prototype` key lookup on a `Record<string, number>` map returns a function, which `??` does not catch [`src/components/settings/AppearanceContent.tsx:155`] — deferred, pre-existing (identical exposure at `:154` for `FONT_PX_MAP`)
- [x] [Review][Defer] The tier→multiplier mapping has zero test coverage; `native_preview.rs` has no `#[cfg(test)]` module at all [`src-tauri/src/native_preview.rs:115-119`] — deferred, pre-existing (the `font_px` mapping is untested for the same reason)
- [x] [Review][Defer] The `merge_settings` test fixture pins the new field to `None`, so the `Some(...)` merge branch is never executed [`src-tauri/src/commands/settings.rs:1491`] — deferred, pre-existing (`preview_font_size: None` sits in the same fixture)
- [x] [Review][Defer] Positional argument chains: three consecutive `String` params in the Kotlin `Config(...)` call, and a 20+ all-optional positional chain across three TS boundaries [`android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:443`, `src/components/SettingsPanel.tsx:570`] — deferred, pre-existing architecture (order verified correct today)

### Review Findings — RE-REVIEW of the fix round (2026-08-10)

_Source: `bmad-code-review` re-review of committed range `86b5dca..HEAD` (HEAD = `4f9aba0`), 2026-08-10.
Scoped mandate: verify only that findings D1/F2, D2/F1, D3/F6, P1/F3, P2/F4, P3/F5 are resolved and
that the touched lines regressed nothing — no fresh full adversarial sweep. All three layers ran and
returned (Blind Hunter, Edge Case Hunter, Acceptance Auditor); 10 further raised items were dismissed
as false positives or duplicates after verification against the real files._

**Verification of the six fix-round findings:**

- **D2/F1 — RESOLVED.** Code matches the story on all three surfaces: `native_preview.rs:108-112`
  (1.325 / 1.625 / 1.925), `ListeningPanelView.kt:59` (1.45f / 1.7f / 1.95f),
  `AppearanceContent.tsx:28` (1.325 / 1.625 / 1.925, Desktop mirror). `"medium"` is byte-identical
  to the no-op it replaces on every surface — Desktop `1.625` = the deleted `PREVIEW_LINE_HEIGHT`
  (zero dangling references remain), Android `1.7f` = the surviving init literal at `:367`, card
  `lineHeight: 1.625` = Tailwind v4's `--leading-relaxed` (no `@theme` override in `src/styles.css`;
  React emits unitless). The ±0.30-em normalization is arithmetically sound.
- **D1/F2 — RESOLVED IN SUBSTANCE, cross-reference dangling** → see R-P5.
- **P1/F3 — RESOLVED.** Both comments read "To be confirmed at GATE-4"
  (`ListeningPanelView.kt:57`, `native_preview.rs:107`). No "confirmed at GATE-4" survives.
- **P2/F4 — RESOLVED, one recorded claim inaccurate** → see R-P2. All 17 line citations in DoD #6
  are correct at HEAD and the chain forwards the field at every hop, positional order included.
- **P3/F5 — HALF-CARRIED** → see R-P1.
- **D3/F6 — RESOLVED.** Task 7.4 is `- [ ]` and states plainly that the build succeeded and
  install/smoke did not run. No contradiction in Change Log, Completion Notes or Debug Log.

**Regression on the touched lines: none found.** `cargo test --lib` 657/657 green, `npm run build`
(tsc strict + vite) clean, `node scripts/gen-android-theme.mjs --check` clean, `cargo check --lib`
clean, `npx tsc --noEmit` clean. The disabled-live-preview path still keeps the hardcoded
`setLineSpacing(0f, 1.7f)` (`KlarvoOverlayService.kt:2353-2355` gates `applyAppearance`), so the
byte-identical-to-pre-11-2 guarantee survives. Positional argument order re-checked by hand at all
four TS boundaries and the single Kotlin construction site — correct.

**Note (corroborates D3/F6, not a finding):** the gitignored mirror
`src-tauri/gen/android/.../ListeningPanelView.kt` still carries the pre-fix `1.55f/1.85f`. It is
unconditionally clobbered by the build scripts' Kotlin-copy step, so it is inert — but it is
independent evidence that no Android build has run since `4f9aba0`. Neither edited platform file was
compiler-verified on this host after the fix round (`native_preview.rs` is Windows-gated and no
Windows target is installed; no `kotlin-compiler-embeddable` jar is cached). Both edits are
literal-only, so the risk is low, but it is unverified rather than verified.

- [x] [Review][Decision] **R-D1 — the widened `"small"` tier (1.325) now sits at or below the default GDI font's own line cell, and `line_h` is not clamped** — `native_preview.rs:147-149` deliberately resolves the default `previewFontFamily` (`'Inter', system-ui, …`) to **Segoe UI**, whose natural cell is ≈1.330 em — i.e. *above* the new 1.325. `line_h` (`:672`) is `round(font_px × sc × text_scale × mult)` while the font em is `trunc(font_px × scale × text_scale)` (`:1126`), and `DrawTextW` at `:766-771` draws with `DT_SINGLELINE | DT_VCENTER` into a `line_h`-high rect **without `DT_NOCLIP`**. Worked example: at font_px 11 / DPI 1.0 the rect and the glyph cell come out equal (15 px, zero leading); at an effective em of 22 px the rect is 29 px against a ≈30 px cell — a 1 px shortfall that clips umlaut dots or descenders. The pre-fix 1.475 cleared this band; the widening moved into it. **I could not verify this from Linux — GDI text metrics are not observable here, and the Segoe UI ratio is from font tables, not measured.** Per project-context.md ("never make the user the rendering oracle") this must not be patched on a hypothesis. Options: (a) accept and add one specific observation to GATE-4a — "at Zeilenabstand=Kompakt, do Ä/Ö/g clip at 100 % and at 200 % scaling?"; (b) clamp defensively with `GetTextMetricsW(s.tmp_dc)` → `line_h.max(tm.tmHeight)`; (c) nudge `"small"` to 1.35 and re-derive Android. Note that (b) would silently cap the "Kompakt" tier and make it indistinguishable from "Normal" at small font sizes. **Resolved 2026-08-10 (human decision R-D1): (c) — Desktop `"small"` raised 1.325 → 1.35, keeping headroom above Segoe UI's ≈1.330 em cell (`native_preview.rs:115-119`, `AppearanceContent.tsx:34`). Android's `LINE_SPACING_MULT` (`ListeningPanelView.kt:59`) is UNCHANGED — this was a Desktop-only fix, since the clipping risk is specific to GDI's unclipped `DrawTextW` rect, not Android's `TextView` layout. The Desktop down-step is now asymmetric: -0.275 em (small→medium) vs +0.300 em (medium→large).**

- [x] [Review][Patch] **R-P1 — P3/F5 half-carried: `src/hooks/useSettings.ts` is still absent from the IN list and from the "exhaustive occurrence list", while Completion Notes now claim two missed touch points were added** — Completion Notes `:522-527` name `useSettings.ts` *and* `lib.rs` as the two touch points the story's plan missed, but only `lib.rs` was added to the lists. The IN list has no `useSettings.ts` entry, and the Dev Notes list — introduced as "The **exhaustive** occurrence list" — still omits it, even though the File List and the new DoD #6 chain both include it. The story's own scope-IN section therefore still fails to cover a file the story changed, which is precisely the defect P3/F5 was raised about. [`_bmad-output/implementation-artifacts/11-6-line-spacing-appearance-setting.md:87-139, 376-393`] — **Fixed 2026-08-10 (finding R-P1) — `src/hooks/useSettings.ts:94, 117` added to both the IN list and the Dev Notes "exhaustive occurrence list".**
- [x] [Review][Patch] **R-P2 — the `lib.rs` touch points are described as "`AppConfig`/config-construction sites"; they are the `SettingsView` DTO plus three `#[cfg(test)]` fixtures** — `src-tauri/src/lib.rs` contains no `AppConfig` construction that carries this field: `:235` is a field on `pub struct SettingsView` (`:121`), and `:1155 / :1227 / :1293` are `SettingsView` literals inside `mod tests` (`#[cfg(test)]` at `:1037`). The line numbers are right; the label is wrong twice over, and 3 of the 4 cited sites are test code. This matters most at `:252`, because that sits inside a DoD entry explicitly framed as "a mechanical check, not a self-attestation" — it overstates production coverage of the save chain. [`_bmad-output/implementation-artifacts/11-6-line-spacing-appearance-setting.md:100-101, 252, 528`] — **Fixed 2026-08-10 (finding R-P2) — re-labelled as the `SettingsView` DTO field + three `#[cfg(test)]` fixtures (not `AppConfig`/config-construction) everywhere it was cited: IN list, DoD #6, and the Dev Notes occurrence list.**
- [x] [Review][Patch] **R-P3 — the `Review Findings` section added by `4f9aba0` cites line numbers that the same commit invalidated** — the section was written in `4f9aba0`, which also inserted the comment blocks that shifted those lines, so the citations were never correct in any committed tree. Confirmed drifts: `AppearanceContent.tsx:22`→`:28`, `:143`/`:142`→`:149`/`:148`; `ListeningPanelView.kt:53`→`:59`, `:270,274`→`:276,:280`; `native_preview.rs:667`→`:672`, `:99-106`→`:108-112`; `settings.rs:1490`→`:1491`. The six `[Defer]` bullets are the load-bearing carriers for future work, so their citations cost the next reader directly. Correct as cited: `pipeline.rs:757`, `SettingsPanel.tsx:839-840`, `KlarvoApi.kt:443`, `settings.rs:375-376`. Related drift: DoD #1's `(:4272-4303)` for the new test is off (actual `:4279-4304`), and the precedent citation `config/mod.rs:4227-4253` (used at `:171`, `:214`, `:269`, `:378`) drifted +12 lines — `spec_preview_font_size_config_field_default` now starts at `:4242`. [`_bmad-output/implementation-artifacts/11-6-line-spacing-appearance-setting.md:353-366`] — **Fixed 2026-08-10 — citations corrected against the final committed state (the R-D1/"small"→1.35 fix shifted `native_preview.rs`/`AppearanceContent.tsx` further): `AppearanceContent.tsx:34` (was `:22`/`:28`), `:155`/`:154` (was `:143`/`:142`, then `:149`/`:148`); `ListeningPanelView.kt:59` (Android untouched, so `:59` stands), `:276,:280` (was `:270,274`); `native_preview.rs:679` (was `:667`/`:672`), `:115-119` (was `:99-106`/`:108-112`); `settings.rs:1491` (was `:1490`). The `config/mod.rs`/DoD #1 drift noted above was not re-touched — out of scope for this fix round.**
- [x] [Review][Patch] **R-P4 — the deferred-work ledger's 11-6 block records the superseded ±0.15 multipliers and is uncommitted** — it states the untested mapping is `"small" → 1.475 / "medium" → 1.625 / "large" → 1.775`, which no longer exists anywhere in the repo, and repeats the stale citations from R-P3. `4f9aba0` changed those numbers without updating the ledger entry it had just written, and the file was never committed (`M` in the working tree, absent from `86b5dca..HEAD`) — so the deferral record both disagrees with HEAD and would be lost by a `git checkout`. [`_bmad-output/implementation-artifacts/deferred-work.md:276-283`] — **Fixed 2026-08-10 (finding R-P4) — multipliers and citations updated to the final committed state (Desktop `1.35/1.625/1.925`, Android `1.45/1.7/1.95`); `deferred-work.md` is committed alongside this pass so the deferral record is no longer at risk from a `git checkout`.**
- [x] [Review][Patch] **R-P5 — D1/F2's cross-reference to the "GATE-4 tasks" is dangling; no GATE-4 task or DoD bullet was amended** — `:353` closes D1 with "see the 'Addendum' under DESIGN DECISIONS **and GATE-4 tasks**", and the Change Log asserts "GATE-4b judges the real preview panel, not the card". The Addendum does record the decision, but Task 7.6 is untouched by `4f9aba0`, Task 7.7 was rewritten for the ±0.30 number only, and the DoD GATE-4 bullets say only "on the real device and the real Windows build". Whoever executes GATE-4b from the task list gets no instruction to judge on `ListeningPanelView`'s panel rather than the near-identical-looking Settings card — which is exactly the trap D1 identified. [`_bmad-output/implementation-artifacts/11-6-line-spacing-appearance-setting.md:226-236, 341-345, 353`] — **Fixed 2026-08-10 (finding R-P5) — carried the Addendum's "judge on the real preview panel, not the card" instruction into Task 7.6 and Task 7.7.**
- [x] [Review][Patch] **R-P6 — the rewritten `wrap_text_lines` doc comment omits `text_scale` from the line-step formula** — the comment states the step as `font_px × line_height_mult × scale`; the code is `font_px * sc * text_scale * line_height_mult`. Two independent scale factors, one documented. This exact line was rewritten by this story, so the omission was re-endorsed rather than inherited, and `text_scale` (the Windows TextScaleFactor drift) is the known prior root cause in this very file — Story 10-4. [`src-tauri/src/native_preview.rs:469`] — **Fixed 2026-08-10 (finding R-P6) — comment now reads `font_px × scale × text_scale × line_height_mult`, matching the code (`native_preview.rs:476`, was `:469` pre-R-D1-shift).**

- [x] [Review][Defer] The ±0.30-em cross-platform normalization holds for the reachable default typefaces but drifts on the Georgia preset — `Typeface.DEFAULT` (Roboto) and `MONOSPACE` (DroidSansMono) both have hhea 1900/−500 @ upem 2048 → N = 1.172, so the Android Δ is 0.293 em (2.3 % under the stated 0.30). `SERIF` (Noto Serif, N ≈ 1.36) reaches ≈0.34 em, ~14 % over. The comment's "~1.2×" hedge covers the default case; only the Georgia font-family preset is materially off. [`android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt:53-58, 86`] — deferred, cosmetic and preset-specific

## Dev Notes

### The `preview_font_size` precedent is the authoritative template for every file this story touches

This story is structurally identical to Story 6.3 (`previewFontSize`) plus its 11-2 Android port
— same category of setting (Appearance, 3-tier String enum), same full stack of touch points. Do
not re-derive the wiring pattern from scratch; grep for every `preview_font_size` /
`previewFontSize` occurrence across the repo and add a `preview_line_spacing` /
`previewLineSpacing` line next to each one. The exhaustive occurrence list gathered during story
creation:
- `src-tauri/src/config/mod.rs:787-790` (field), `:1031-1033` (default fn), `:1110, 2167, 3471`
  (test fixtures), `:4220-4253` (inversion-guard spec test).
- `src-tauri/src/commands/settings.rs:164, 223, 370-371, 472, 570, 684`.
- `src-tauri/src/lib.rs:233, 1154, 1226, 1292` — added at the 2026-08-10 code-review gate
  (finding P3); the original list omitted this file though `preview_font_size` occurs there.
- `src-tauri/src/native_preview.rs:48` (const → per-config field), `:86, 95-99, 154` (analogous
  `font_px` field/mapping to mirror the *shape* of, not the value), `:175, 419, 454-456, 653-654,
  657, 738, 1109-1111` (render/doc-comment sites).
- `src/types.ts:106-107`.
- `src/tauri-commands.ts:78-82, 98, 312, 369`.
- `src/components/SettingsPanel.tsx:81, 216-218, 339-340 (dirty-forever trap!), 425`.
- `src/hooks/useSettings.ts:94, 117` (`handleSaveSettings` positional pass-through) — added at
  the 2026-08-10 re-review gate (finding R-P1); the original list omitted this file though it was
  a touch point the story's plan missed and Completion Notes already named.
- `src/components/settings/AppearanceContent.tsx:14-16 (FONT_PX_MAP — analogous, not to be
  touched), 124-140 (live preview card), 319-335 (font-size control block, structural template)`.
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:115, 365, 436`.
- `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt:49 (FONT_PX_SP — structural
  template), 251-269 (applyAppearance), 353-355 (init-block hardcoded default)`.

### `preview_font_size` values already diverge per platform — this is precedent, not a bug to fix

Desktop `FONT_PX_MAP` = `{small:11, medium:13, large:15}` (`AppearanceContent.tsx:16`); Android
`FONT_PX_SP` = `{small:13, medium:15, large:18}` (`ListeningPanelView.kt:49`, explicitly commented
"Story 11-3 device feedback pass rescale... Android-only; desktop's FONT_PX_MAP is untouched").
This establishes that per-platform tuning of the *same tier labels* to different renderer-specific
numeric values is the established, sanctioned pattern in this codebase — apply the same reasoning
to line-spacing's DESIGN DECISION 2 multiplier values rather than forcing numeric identity across
platforms.

### Desktop preview rendering moved to native Win32/GDI in Epic 10 — `native_preview.rs` is the only render path

There is no more webview-based `PreviewPanel.tsx` (a repo-wide search found none) — Epic 10
replaced it with a native layered overlay window drawn via raw GDI (`native_preview.rs`). The
`PREVIEW_LINE_HEIGHT` const's own comment (`:48`) says it "matches SOLL `leading-relaxed`
(AppearanceContent.tsx)" — i.e. the Settings-panel's *preview card* (still React/CSS) and the
*actual floating preview* (native GDI) are two independent renderers that happen to target the
same value today. This story must keep both in sync with the new config value (Task 3 for GDI,
Task 5.2 for the Settings card), not just one.

### No locale/i18n mechanism exists in this repo today

The backlog phrase "(camelCase, Backend-Locale-Files)" does not correspond to anything found
during story creation — no `locales/` directory, no i18n JSON files anywhere in the tracked repo.
Treat this as a stale/generic note from the general config-key-convention pattern (memory
`reference_locale_files_backend_only` refers to `shells/windows/locales/`, which does not exist in
this checkout) rather than an actual task. If dev discovers this search was wrong, escalate rather
than silently building a new locale mechanism.

### Project Structure Notes

- Alignment: this story follows the existing Appearance-field pattern exactly (Rust `AppConfig` →
  `SettingsPatch`/`save_settings` → TS `types.ts`/`tauri-commands.ts` → React
  `SettingsPanel.tsx`/`AppearanceContent.tsx` → Android `KlarvoApi.kt`/`ListeningPanelView.kt`). No
  new architectural pattern introduced.
- No conflicts detected with unified project structure — this is additive, not structural.

### References

- `docs/backlog.md` §"11-6 (Backlog) — Zeilenabstand als Appearance-Setting" — the sole source
  document (one paragraph; see DESIGN DECISIONS for what it pinned and what Andi settled on 2026-08-10).
- `_bmad-output/implementation-artifacts/11-3-android-preview-box-device-feedback-pass.md` —
  precedent for "first-pass numbers, confirm at real-device GATE-4" discipline (the `FONT_PX_SP`
  rescale itself was exactly this kind of first-pass-then-confirmed number).
- `_bmad-output/implementation-artifacts/11-4-bubble-structurally-above-preview-z-order.md` —
  precedent for the "OPEN ITEMS — needs Andi's confirmation" story-section pattern this story
  reuses, and for citing exact pre-existing line numbers so the dev agent doesn't have to
  re-search.
- `_bmad-output/project-context.md` — Release-Build blind spot rule (Linux tests insufficient for
  AC-3/AC-4), Android real-device smoke rule, camelCase config convention, commit hygiene
  (`never git add .`).
- `src-tauri/src/config/mod.rs`, `src-tauri/src/commands/settings.rs`,
  `src-tauri/src/native_preview.rs`, `src/types.ts`, `src/tauri-commands.ts`,
  `src/components/SettingsPanel.tsx`, `src/components/settings/AppearanceContent.tsx`,
  `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt`,
  `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` — see exact line citations
  throughout Dev Notes/Tasks above [Source: repo grep during story creation, 2026-07-09].

## Previous Story Intelligence (11-4)

- **Git pattern:** 11-4 landed as small scoped commits on its own branch
  (`6eca8f0` feat → `678cf4a` review fixes → `43a52ca` GATE-4 fix → `fe8b5c5` close-out). This
  story should branch similarly (e.g. `feat/11-6-line-spacing-setting`) and commit per logical
  step (Rust wiring, then frontend, then Android), never `git add .`.
- **"OPEN ITEMS" section discipline:** 11-4 pinned a first-pass engineering choice while
  explicitly flagging it as unconfirmed, then closed the loop with a real Design Decision entry
  once GATE-4 forced the actual answer (Design Decision 4, added mid-story after a GATE-4
  failure). This story went the cheaper route: its items 1-4 were put to Andi at GATE-1 (2026-08-10)
  and answered before any code was written, so GATE-4 only has to judge the step size. Same lesson,
  applied earlier in the cycle.
- **GATE-4 = real device/build only, never emulator/Linux tests** — 11-2/11-3/11-4 all required
  Andi's real hardware for the final visual confirmation; this story additionally needs a real
  **Windows** build (new territory relative to 11-2..11-4, which were Android-only) because it
  touches `native_preview.rs`'s GDI render path for the first time in this epic.
- **`config/mod.rs`'s inversion-guard test pattern** (`spec_preview_font_size_config_field_default`)
  is copy-adaptable near-verbatim — reuse its structure rather than designing a new test shape.

## Dev Agent Record

### Agent Model Used

claude-sonnet-5 (bmad-dev-story)

### Debug Log References

- `cargo test --lib` (src-tauri): 657/657 green, incl. new `spec_preview_line_spacing_config_field_default`.
- `cargo check` (src-tauri): clean (two earlier runs hit a transient rustc SIGABRT/SIGSEGV in
  unrelated dependency crates — `regex-syntax`, `zvariant`, `rusqlite` — caused by resource
  contention with the concurrently-running full Android build; a clean re-run after the Android
  build finished confirmed this was not code-related).
- `node scripts/gen-android-theme.mjs --check`: `[ok] KlarvoTheme.kt is in sync with canon klarvo.css`.
- `npm run build` (tsc strict + vite): built cleanly, 79 modules transformed.
- Kotlin device-free compile-verify: `./gradlew :app:compileArmDebugKotlin --offline` → `BUILD SUCCESSFUL`
  (confirmed a fresh recompile of the touched files via `armDebug` class-file timestamps).
- Full `npx tauri android build --target aarch64` (via `scripts/android-build.sh`): Rust aarch64
  cross-compile + Kotlin compile + Gradle assembly all succeeded, producing
  `app-universal-release-unsigned.apk`. The script's own post-build step (copying the signed APK to
  `/mnt/d/Dropbox/...`) failed on this host because that path is a WSL/Windows-mount convention that
  does not exist on this native-Linux dev machine (`mkdir: Permission denied` — environment mismatch,
  unrelated to this story's code). Manually zipaligned + signed the already-built unsigned APK with
  the project's `voxlit-debug.keystore` (`apksigner verify` passed) at `/tmp/klarvo-11-6-signed.apk`.
- `adb install -r` of the signed APK onto the connected device (Tailscale-reachable Xiaomi/HyperOS)
  failed with `INSTALL_FAILED_UPDATE_INCOMPATIBLE` — the app currently installed on Andi's device was
  signed with a different key than `voxlit-debug.keystore` produced. Did not uninstall the existing
  app to force the install through, since that would destroy Andi's live app/session state on his
  real device without his say-so — left for Andi to install himself at GATE-4b.

**Review-fix pass, 2026-08-10 (applying findings D1/D2/D3/P1/P2/P3) — re-ran the mechanically
executable gates (Tasks 7.1-7.3); did not re-run Task 7.4's build/install/smoke per instruction,
since that stays the conductor's job at GATE 4:**
- `cargo test --lib`: 657/657 green, including `spec_preview_line_spacing_config_field_default`
  (unaffected by the multiplier-value changes, since it only checks the config-field default).
- `cargo check`: clean, no errors (only pre-existing dead-code warnings unrelated to this story).
- `node scripts/gen-android-theme.mjs --check`: `[ok] KlarvoTheme.kt is in sync with canon klarvo.css`.
- `npm run build` (tsc strict + vite): built cleanly, 79 modules transformed.
- `ListeningPanelView.kt`'s edited `LINE_SPACING_MULT` map literal was reviewed by eye (a
  well-formed 3-entry `Map<String, Float>`, same shape as before); not recompiled via
  `gradlew`/`android-build.sh`, since `src-tauri/gen/android/` is a gitignored auto-sync of
  `android/kotlin-src/` (Task 6.3) that would need a fresh full Android build to reflect this
  edit, and re-running that heavier build/install/smoke path is explicitly out of scope for this
  review-fix pass (F6/D3) — it stays the conductor's job at GATE 4.

**Re-review fix pass, 2026-08-10 (applying findings R-D1/R-P1-R-P6) — re-ran the mechanically
executable gates (Tasks 7.1-7.3); did not re-run Task 7.4's build/install/smoke, same rationale as
the prior fix-round pass:**
- `cargo test --lib`: 657/657 green, including `spec_preview_line_spacing_config_field_default`
  (unaffected by the R-D1 multiplier change, since it only checks the config-field default).
- `cargo check`: clean, no errors (only pre-existing dead-code warnings unrelated to this story).
- `node scripts/gen-android-theme.mjs --check`: `[ok] KlarvoTheme.kt is in sync with canon klarvo.css`.
- `npm run build` (tsc strict + vite): built cleanly, 79 modules transformed.
- `ListeningPanelView.kt` was NOT touched by this pass (R-D1 is Desktop-only per the human
  decision) — no Android recompile needed.

### Completion Notes List

- Implemented the full `preview_font_size` → `preview_line_spacing` mirror across every touch point
  the story enumerated (Rust `AppConfig`/`SettingsPatch`/`SettingsView`, TS `types.ts`/
  `tauri-commands.ts`/`SettingsPanel.tsx`/`AppearanceContent.tsx`, Kotlin `KlarvoApi.kt`/
  `ListeningPanelView.kt`), plus **two** touch points the story's original exhaustive list missed
  (corrected at the 2026-08-10 code-review gate, finding P3 — the original notes here claimed only
  one):
  - `src/hooks/useSettings.ts`'s `handleSaveSettings` wrapper (a positional pass-through between
    `SettingsPanel`'s `onSaveSettings` prop and `tauri-commands.ts`'s `saveSettings`) — found via a
    `cargo build`/`tsc` compile-error sweep, not by re-deriving the wiring from scratch.
  - `src-tauri/src/lib.rs`'s `AppConfig` construction sites (`:233, 1154, 1226, 1292`) — the code
    was implemented correctly (it compiled and the tests passed), but the file was absent from the
    story's IN-scope list and Dev Notes occurrence list; both are now corrected.
- Two additional `AppConfig`/`SettingsPatch` literal-construction sites turned up only under
  `cargo test` (not `cargo build`/`cargo check`, which don't compile the `#[cfg(test)]` module):
  `commands/settings.rs:1441`'s `test_merge_settings_happy_path_full_patch` fixture. Fixed by
  compiling with `cargo test` before declaring Task 1/2 done, not stopping at `cargo build`.
- DESIGN DECISION 2 multipliers, updated 2026-08-10 at the code-review gate (finding D2/F1): the
  first-pass ±0.15 numbers (Desktop 1.475/1.625/1.775, Android 1.55/1.7/1.85) were widened to a
  symmetric **±0.30 em**, per-platform normalized — **Desktop small/medium/large =
  1.325/1.625/1.925** (`native_preview.rs`), **Android = 1.45/1.7/1.95** (`ListeningPanelView.kt`).
  Both keep `medium` byte-identical to today's hardcoded no-op (Desktop 1.625, Android 1.7).
  `AppearanceContent.tsx`'s `LINE_SPACING_MULT` mirrors the Desktop values for the Settings preview
  card, consistent with 6.3's `FONT_PX_MAP` precedent (see the DESIGN DECISIONS Addendum, finding
  D1). **The step size itself is settled — these are no longer first-pass numbers.** What remains
  for GATE-4 (Task 7.7) is only confirming they look right on a real Windows build and a real
  Android device, not choosing a step size.
- Task 7.4 corrected 2026-08-10 at the code-review gate (finding D3): the task was previously
  checked off claiming a "clean build/install" even though the on-device install failed
  (`INSTALL_FAILED_UPDATE_INCOMPATIBLE`) and no smoke — emulator or device — ever ran. Un-checked
  and reworded to state plainly that only the build succeeded; no install, no smoke, and therefore
  no runtime evidence for the Kotlin render path beyond a compile. Did not run an emulator smoke to
  close this during the review-fix pass, per instruction — that is the conductor's job at GATE 4.
- Task 5.3 confirmed: no `hideLineSpacing` prop was needed — line-spacing renders meaningfully on
  both platforms, unlike `hidePanelForm`/`hideBgBlur`.
- Task 6.3 confirmed: `src-tauri/gen/android/` is `.gitignore`d and mechanically resynced from
  `android/kotlin-src/` by `android-build.sh`'s own copy step — no hand-duplication was needed or
  performed.
- AC-3 (Windows GDI render) and AC-4 (Android on-device render) are **not** verifiable from this
  Linux dev environment per project-context.md's "Release-Build blind spot" rule — both remain
  GATE-4 human gates (Tasks 7.5/7.6), unchecked by design, following the exact precedent set by
  Story 11-4's Task 4.4 (dev-agent-executable subtasks checked, the real-device/real-build subtask
  left open, Status still moves to `review`).
- **Re-review fix pass, 2026-08-10 (applying findings R-D1/R-P1-R-P6):**
  - **R-D1 (human decision, Desktop only):** raised Desktop `"small"` from 1.325 to **1.35**
    (`native_preview.rs`, `AppearanceContent.tsx`'s `LINE_SPACING_MULT`) — keeps headroom above
    Segoe UI's ≈1.330 em natural line cell so `DrawTextW`'s unclipped `line_h`-high rect doesn't
    risk clipping diacritics/descenders. Android's `LINE_SPACING_MULT` (`ListeningPanelView.kt`) is
    UNCHANGED. Final Desktop values: 1.35/1.625/1.925 (asymmetric -0.275 em / +0.300 em step).
  - **R-P1:** added `src/hooks/useSettings.ts:94, 117` to the IN list and the Dev Notes
    "exhaustive occurrence list" (previously only named in Completion Notes, per P3/F5).
  - **R-P2:** corrected the `lib.rs` touch-point label from "`AppConfig`/config-construction
    sites" to "`SettingsView` DTO field + three `#[cfg(test)]` fixtures" everywhere it was cited.
  - **R-P3:** corrected the drifted line-number citations in the original "Review Findings"
    section against the final committed state (after the R-D1 shift), not just the `4f9aba0` HEAD
    the finding was scoped to.
  - **R-P4:** updated `deferred-work.md`'s 11-6 block from the superseded ±0.15 multipliers
    (1.475/1.625/1.775) to the final committed values, and committed the file (previously
    uncommitted, at risk from a `git checkout`).
  - **R-P5:** carried the "GATE-4b judges the real preview panel, not the card" instruction from
    the DESIGN DECISIONS Addendum into Task 7.6 and Task 7.7 directly.
  - **R-P6:** completed the `wrap_text_lines` doc comment's line-step formula to include
    `text_scale` (`font_px × scale × text_scale × line_height_mult`), matching the code.
  - Re-ran the mechanically executable gates (Tasks 7.1-7.3); did not re-run Task 7.4's
    build/install/smoke, since that stays the conductor's job at GATE 4, same as the first
    fix-round pass.

### File List

- `src-tauri/src/config/mod.rs`
- `src-tauri/src/commands/settings.rs`
- `src-tauri/src/native_preview.rs`
- `src-tauri/src/lib.rs`
- `src/types.ts`
- `src/tauri-commands.ts`
- `src/hooks/useSettings.ts`
- `src/components/SettingsPanel.tsx`
- `src/components/settings/AppearanceContent.tsx`
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt`
- `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt`

## Change Log

| Date | Change |
|------|--------|
| 2026-07-09 | Story created (bmad-create-story) from `docs/backlog.md` §11-6. Source is a single backlog paragraph, not a fully-specced epic entry — 4 design/UI/intent items (control type, concrete tier values, label wording, live-preview-card fidelity) are not pinned and are recorded as OPEN ITEMS rather than defaulted silently. Status: ready-for-dev. |
| 2026-08-10 | GATE-1 with Andi: all 4 open design items settled — 3-tier `KSegmented` (not a slider); platform-tuned multipliers with `"medium"` = today's hardcoded value (identical cross-platform numbers explicitly rejected); labels `"Zeilenabstand"` / `"Kompakt" \| "Normal" \| "Locker"`; Settings preview card wired to the new field. Only residual for GATE-4: the ±0.15 step size. Also corrected the Android `Config` default in Tasks 6.1 from `"small"` to `"medium"` (it contradicted the no-op-default decision). Status stays `ready-for-dev` — no code written. |
| 2026-08-10 | **bmad-dev-story: implemented Tasks 1-7.1-7.4 (AC-1, AC-2, AC-3 code path, AC-4 code path).** Full cross-platform mirror of the `preview_font_size` precedent landed exactly per the story's file-by-file plan, plus one extra touch point the plan missed (`useSettings.ts`'s save-wrapper) and one extra `SettingsPatch` test fixture only visible under `cargo test`. 657/657 Rust tests green (new spec test included), `cargo check` clean, TS strict build clean, Android theme drift-gate clean, Kotlin `armDebug` compile-verify green, and a full `tauri android build --target aarch64` succeeded end-to-end (manually signed since the script's Dropbox-copy step assumes a WSL host this machine isn't). Device install was blocked by a signing-key mismatch with the app already on Andi's phone — did not force-uninstall his live app to work around it. Status → `review`. **GATE-4a (real Windows build) and GATE-4b (real Android device) are still Andi's action**, including confirming the first-pass ±0.15 step size (Task 7.7) — same precedent as Story 11-4's Task 4.4. |
| 2026-08-10 | **Code-review fix pass** (`bmad-code-review` findings D1/D2/D3/P1/P2/P3, human-decided). **D2/F1:** widened the ±0.15 step size to a symmetric ±0.30 em, per-platform normalized — Desktop 1.325/1.625/1.925, Android 1.45/1.7/1.95 (`medium` unchanged); rationale recorded next to the values. **D1/F2:** accepted the Settings-preview-card Android/Desktop divergence as precedent-consistent (same class as `FONT_PX_MAP`/`FONT_PX_SP`); GATE-4b judges the real preview panel, not the card. **D3/F6:** un-checked and reworded Task 7.4 — the prior checked state overclaimed an install/smoke that never ran (build succeeded, `adb install` failed with `INSTALL_FAILED_UPDATE_INCOMPATIBLE`, no smoke executed); left for the conductor at GATE 4. **P1/F3:** reworded premature "confirmed at GATE-4" comments in `ListeningPanelView.kt`/`native_preview.rs` to "to be confirmed at GATE-4". **P2/F4:** added the mandated `surface-smoke-checklist.md` items #1/#3/#6 to the DoD with executed checks and outcomes (all pass). **P3/F5:** added `src-tauri/src/lib.rs` to the IN-scope and Dev Notes occurrence lists, and corrected Completion Notes to say two missed touch points (`lib.rs`, `useSettings.ts`), not one. Also updated Task 7.7 and DESIGN DECISION 2's Residual to reflect the settled ±0.30 em step size. Status stays `review` — this pass only applies the confirmed findings and re-runs verification gates. |
| 2026-08-10 | **Re-review fix pass** (`bmad-code-review` re-review findings R-D1/R-P1-R-P6, human-decided). **R-D1 (Desktop only):** raised Desktop `"small"` 1.325 → **1.35** to keep headroom above Segoe UI's ≈1.330 em line cell against `native_preview.rs`'s unclipped `DrawTextW` rect; Android's `LINE_SPACING_MULT` is UNCHANGED. Final Desktop values 1.35/1.625/1.925 (asymmetric -0.275 em / +0.300 em step); rationale recorded next to the values in both `native_preview.rs` and `AppearanceContent.tsx`. **R-P1:** added `src/hooks/useSettings.ts:94, 117` to the IN list and Dev Notes occurrence list (P3/F5 was only half-carried). **R-P2:** corrected the `lib.rs` touch points' label from "`AppConfig`/config-construction sites" to "`SettingsView` DTO field + three `#[cfg(test)]` fixtures". **R-P3:** corrected the original "Review Findings" section's drifted line citations against the final committed state. **R-P4:** updated and committed `deferred-work.md`'s 11-6 block (was uncommitted and named the superseded ±0.15 multipliers). **R-P5:** carried the "GATE-4b judges the real panel, not the card" instruction into Tasks 7.6/7.7 directly. **R-P6:** completed the `wrap_text_lines` doc comment's formula to include `text_scale`. Status stays `review` — this pass only applies the confirmed findings and re-runs verification gates. |
| 2026-08-11 | **GATE 4 abgeschlossen — Story done.** **GATE-4a (Windows):** Andi bestätigt AC-2 + AC-3 am echten Release-Build; kein Clipping von Umlaut-Punkten/Unterlängen auf „Kompakt" (die R-D1-Restbeobachtung ist damit erledigt). Der Build war zunächst repo-weit gebrochen — `package-lock.json` pinnt `@tauri-apps/plugin-log` nicht, und `sync-and-build.ps1` fährt nach dem robocopy sein eigenes `npm install`, das jeden Pin wieder abräumt; für diesen Gate mit expliziten `--no-save`-Pins und einem npm-freien Skript umgangen, als Backlog-Punkt notiert. **GATE-4b (Android):** zuerst grün gemessen (68,75/80,74/92,72 px, < 0,2 % gegen die vorab abgeleitete Vorhersage) — was aber nur belegt, dass der Multiplikator den Renderer erreicht, nicht dass die Skala richtig liegt. **Revision nach Andis Augenschein (Commit `117e244`, Android-only):** jede Android-Stufe saß ~2 Rasten lockerer als ihre gleichnamige Desktop-Stufe, und Andi stand bereits auf `small`/`small` ohne kleinere Option. Ursache war die von dieser Story selbst als offen markierte Annahme („to be confirmed at GATE-4"), der natürliche Zeilenkasten von `setLineSpacing(0f, mult)` sei ~1,2× — gemessen **1,3285**. Deshalb leiten sich Androids Multiplikatoren jetzt als `desktop_wert / NATURAL_LINE_BOX` ab statt handgewählt zu sein, und `FONT_PX_SP` geht auf Desktops 11/13/15 zurück (Story 11-3 hatte auf 13/15/18 hochskaliert). Nachgemessen mit vorab notierter Vorhersage: Kompakt 40,84 → **41,4 px**, Normal 49,16 → **49,4 px**, Locker 58,23 → **58,35 px** (≤ 1,4 %); die Vorhersage unterstellte 11 sp und belegt damit die Schriftänderung mit. Andi hat die Stufen am Gerät abgenommen. Desktop nach GATE-4a nicht mehr angefasst. Nebenbei „medium" aus zwei Literal-Wiederholungen (`15f`/`1.7f`) in abgeleitete Defaults überführt — sie wären bei dieser Änderung sonst auseinandergelaufen. Status → `done`. |
