---
story: "11.6"
epic: "11"
title: "Line spacing as an Appearance setting"
status: ready-for-dev
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

Status: ready-for-dev

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
   **Residual (still open, and the only thing GATE-4 must judge):** "small"/"large" as a symmetric
   ±0.15 offset (Android 1.55 / 1.7 / 1.85; Desktop 1.475 / 1.625 / 1.775) are **first-pass numbers,
   not a decision.** They must be looked at on a real Android device *and* a real Windows build; the
   step size may need to change. Same precedent as 11-3's "first-pass numbers, confirm at GATE-4".
3. **Labels → German `"Zeilenabstand"` with `"Kompakt" / "Normal" / "Locker"`.** Rejected:
   `"Klein/Mittel/Groß"` (word-for-word reuse of the font-size labels reads wrong for spacing) and
   an English label. The section stays mixed-language as it already is — not this story's job.
4. **Settings preview card → wire it up.** The card at `AppearanceContent.tsx:124-140` must switch
   its hardcoded `className="leading-relaxed"` for an inline `lineHeight` style driven by the new
   field, mirroring the `fontSize` inline-style pattern one property below. Adjusting the setting
   without seeing it change was rejected as a blind flight.

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
- **Design decisions 1-4 are settled (GATE-1, 2026-08-10)** — control type, value semantics, labels
  and preview-card wiring must NOT be re-opened at GATE-4. The single remaining judgement call for
  this round: do the **±0.15 step sizes** look right for "Kompakt"/"Locker" on the real device and
  the real Windows build, or is the step too small/too large?

## Tasks / Subtasks

- [ ] **Task 1 — Rust config field + backend wiring** (AC-1)
  - [ ] 1.1 Add `preview_line_spacing: String` to `AppConfig` (`config/mod.rs`) with
    `#[serde(default = "default_preview_line_spacing")]`, doc comment mirroring
    `preview_font_size`'s (`:787-790`).
  - [ ] 1.2 Add `default_preview_line_spacing() -> String` mirroring
    `default_preview_font_size()` (`:1031-1033`), returning the OPEN-ITEM-2 default tier.
  - [ ] 1.3 Add the field to every `AppConfig` test-fixture construction site that currently lists
    `preview_font_size` (`config/mod.rs:1110, 2167, 3471` and any other constructor found by
    `grep -n preview_font_size config/mod.rs`).
  - [ ] 1.4 Add `spec_preview_line_spacing_config_field_default` test, mirroring
    `spec_preview_font_size_config_field_default` (`:4227-4253`) exactly (camelCase key check,
    missing-key deserialization, no migration write).

- [ ] **Task 2 — `SettingsPatch`/`save_settings`/settings-read wiring** (AC-1, AC-2)
  - [ ] 2.1 Add `preview_line_spacing: Option<String>` to `SettingsPatch` (`settings.rs:164`) +
    its `Default` impl entry (`:223`) + merge line (`:370-371`).
  - [ ] 2.2 Add the parameter to the `save_settings` Tauri command signature (`:472`) and its
    `SettingsPatch` construction (`:570`).
  - [ ] 2.3 Add the field to the settings-read response struct/command (`:684` region).

- [ ] **Task 3 — Desktop native preview rendering** (AC-3)
  - [ ] 3.1 Replace `PreviewConfig`'s implicit dependence on the `PREVIEW_LINE_HEIGHT` const
    (`native_preview.rs:48`) with a `line_height_mult: f32` field, populated in
    `PreviewConfig::from_app_config` from `cfg.preview_line_spacing` (mirror the `font_px` match
    arm at `:95-99`; use the OPEN-ITEM-2 multiplier values).
  - [ ] 3.2 Update the line-height calc at `:657` (and any other `PREVIEW_LINE_HEIGHT` read) to
    use `s.config.line_height_mult` instead of the const. Remove or repurpose the const if no
    longer referenced elsewhere.
  - [ ] 3.3 Update stale doc comments referencing a fixed `1.625`/`leading-relaxed` at `:175,
    419, 454-456, 653-654, 738, 1109-1111` to describe the now-configurable value.

- [ ] **Task 4 — Frontend types + save/load wiring** (AC-1, AC-2)
  - [ ] 4.1 `src/types.ts`: add `previewLineSpacing: string` (mirror `:106-107`).
  - [ ] 4.2 `src/tauri-commands.ts`: default value, save-command parameter (mirror `:78-82, 98,
    312, 369`).
  - [ ] 4.3 `SettingsPanel.tsx`: `localPreviewLineSpacing` state + load-on-mount sync (the
    dirty-forever trap spot, `:339-340`) + dirty-check (`:425`) + save-payload inclusion, + prop
    threading into `AppearanceContent`.

- [ ] **Task 5 — Appearance UI control** (AC-2, DESIGN DECISIONS 1/3/4)
  - [ ] 5.1 Add the new `KSegmented` control block to `AppearanceContent.tsx`, placed after the
    Font-size block (`:319-335`), using the OPEN-ITEM-3 first-pass labels.
  - [ ] 5.2 Wire the live-preview card (`:124-140`) to reflect the new value live (inline
    `lineHeight` style replacing the hardcoded `leading-relaxed` class) per DESIGN DECISION 4.
  - [ ] 5.3 If `hidePanelForm`/`hideBgBlur`-style Android hiding is needed for this control,
    determine and document why (first-pass expectation: **not needed** — unlike panel-form-preset
    and bg-blur, line-spacing is meaningful and renderable on both platforms, so no
    `hideLineSpacing` prop is expected — confirm this holds and don't add one speculatively).

- [ ] **Task 6 — Android config + rendering wiring** (AC-1, AC-4)
  - [ ] 6.1 `KlarvoApi.kt`: add `previewLineSpacing: String = "medium"` to `Config` (mirror the shape of
    `:115`), JSON parse (`:365`), constructor call inclusion (`:436`).
  - [ ] 6.2 `ListeningPanelView.kt`: add a `LINE_SPACING_MULT` map (mirror `FONT_PX_SP`, `:49`,
    using the OPEN-ITEM-2 Android multiplier values), consult it in `applyAppearance` (`:251-269`)
    to call `transcriptTextView.setLineSpacing(0f, mult)`, replacing the hardcoded call at `:355`
    (which becomes the pre-`applyAppearance` bootstrap default only, matching `textSize = 15f`'s
    existing relationship to `FONT_PX_SP` at `:353`).
  - [ ] 6.3 Sync to `src-tauri/gen/android/...` mirrors if that directory is not auto-generated
    by the build scripts (check `scripts/android-build.sh`/`android-smoke.sh` first — do not
    hand-duplicate if it's generated).

- [ ] **Task 7 — Verification**
  - [ ] 7.1 `cargo test` green (new + existing), `cargo check` green.
  - [ ] 7.2 `node scripts/gen-android-theme.mjs --check` clean.
  - [ ] 7.3 `npm run build` (TypeScript strict mode gate) green.
  - [ ] 7.4 `scripts/android-smoke.sh` clean build/install; confirm whether the Settings UI change
    needs a full `tauri android build` (React settings surface) vs. the lighter Kotlin-only smoke
    path — do not assume Kotlin-only given the `.rs`/`.ts`/`.tsx` files this story touches.
  - [ ] 7.5 **GATE-4a — real Windows build**: Andi confirms AC-2 (Settings save/reload/dirty-state)
    and AC-3 (native preview line-spacing renders correctly) on a real Windows build via
    `scripts/sync-and-build.ps1`.
  - [ ] 7.6 **GATE-4b — real Android device**: Andi confirms AC-2 (Settings UI appears/works) and
    AC-4 (preview panel line-spacing renders correctly) on the real Xiaomi/HyperOS device.
  - [ ] 7.7 Confirm the **±0.15 step size** with Andi at GATE-4 (the only design residual — items
    1-4 were settled at GATE-1, 2026-08-10) and record the final multipliers in Completion Notes,
    so this story's history reflects what actually shipped rather than the first-pass numbers.

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
- `src-tauri/src/native_preview.rs:48` (const → per-config field), `:86, 95-99, 154` (analogous
  `font_px` field/mapping to mirror the *shape* of, not the value), `:175, 419, 454-456, 653-654,
  657, 738, 1109-1111` (render/doc-comment sites).
- `src/types.ts:106-107`.
- `src/tauri-commands.ts:78-82, 98, 312, 369`.
- `src/components/SettingsPanel.tsx:81, 216-218, 339-340 (dirty-forever trap!), 425`.
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

### Debug Log References

### Completion Notes List

### File List

## Change Log

| Date | Change |
|------|--------|
| 2026-07-09 | Story created (bmad-create-story) from `docs/backlog.md` §11-6. Source is a single backlog paragraph, not a fully-specced epic entry — 4 design/UI/intent items (control type, concrete tier values, label wording, live-preview-card fidelity) are not pinned and are recorded as OPEN ITEMS rather than defaulted silently. Status: ready-for-dev. |
| 2026-08-10 | GATE-1 with Andi: all 4 open design items settled — 3-tier `KSegmented` (not a slider); platform-tuned multipliers with `"medium"` = today's hardcoded value (identical cross-platform numbers explicitly rejected); labels `"Zeilenabstand"` / `"Kompakt" \| "Normal" \| "Locker"`; Settings preview card wired to the new field. Only residual for GATE-4: the ±0.15 step size. Also corrected the Android `Config` default in Tasks 6.1 from `"small"` to `"medium"` (it contradicted the no-op-default decision). Status stays `ready-for-dev` — no code written. |
