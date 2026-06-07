---
story: "6.6"
epic: "6"
title: "Preview-box appearance customization"
status: review
track: L3-feature
gatedBy: ["6.2"]
buildsOn: ["6.2"]
enabledBy: []
inputDocuments:
  - _bmad-output/planning-artifacts/epics-bar-redesign.md
  - docs/bar-redesign-spec.md
  - docs/deep-dive-bar-subsystem.md
  - _bmad-output/project-context.md
  - docs/surface-smoke-checklist.md
---

# Story 6.6: Preview-box appearance customization

Status: review

## Story

As a user,
I want to customize the preview box's text color/brightness, background color/opacity/blur, border
color/brightness/thickness, and corner radius in Settings,
so that the preview is legible on any desktop background.

**Motivation (why this story exists):** During the 6.2 Windows smoke Andi noticed that on a dark
browser page the preview box was barely visible: border `1px rgba(42,195,168,0.25)` = faint thin
line; text `rgba(220,220,220,0.88)` = dim non-pure-white. Appearance settings let the user adapt
the box to whatever background happens to be behind it.

## Acceptance Criteria

**AC-1 — New config fields with correct camelCase defaults (no migration write):**
Given a fresh config (all new fields absent)
When the schema is loaded
Then each field reads its serde default with NO migration write:
- `preview_text_color` → `"rgba(220,220,220,0.88)"`, camelCase key `previewTextColor`
- `preview_bg_color` → `"rgba(25,25,25,0.96)"`, camelCase key `previewBgColor`
- `preview_bg_blur` → `12` (integer px), camelCase key `previewBgBlur`
- `preview_border_color` → `"rgba(42,195,168,0.25)"`, camelCase key `previewBorderColor`
- `preview_border_width` → `1` (integer px), camelCase key `previewBorderWidth`
- `preview_border_radius` → `14` (integer px), camelCase key `previewBorderRadius`
- `preview_font_family` → `"'Inter', system-ui, -apple-system, sans-serif"`, camelCase key `previewFontFamily`

And inversion: remove `#[serde(default = ...)]` from any field → the config-fields round-trip
spec goes RED (missing field does not fallback to the expected default).

**AC-2 — Settings UI: appearance controls shown when livePreviewEnabled:**
Given the Settings panel with `livePreviewEnabled = true`
When the live-preview section is visible
Then there is an "Appearance" sub-section below the existing Darstellung/width preset picker
containing controls for all seven appearance fields (text color, bg color, bg opacity/blur, border
color, border thickness, corner radius, font family).
And the controls persist values via `save_settings` → `save_config_locked` (ADR-0015).

**AC-3 — PreviewPanel reads all appearance fields reactively (Trap #3):**
Given the user saves new appearance values in Settings
When the preview next opens (panel-open transition)
Then `PreviewPanel` applies the saved values to the card CSS — they are NOT frozen at app-start
mount-time values.
And inversion: reading appearance fields from a mount-only `getSettings` call leaves stale values
after a save (same trap as Story 5-5 — proved RED by saving a different color and reopening
without restart).

**AC-4 — Corner radius is coupled to the OS window region (R11 invariant):**
Given the user sets a custom corner radius (e.g. 8 or 20)
When the preview opens
Then `setPreviewShape()` is called AFTER `setSize()` (as in Story 6.2), and the Rust
`set_preview_shape` command reads the saved `preview_border_radius` from config to compute
`r = (preview_border_radius as f64 * scale) as i32` instead of the hardcoded `14`.
And inversion: leaving `r` hardcoded at 14 while CSS uses a different radius → white-line
corner artifact on Windows (R11).

**AC-5 — CSS applies all appearance values to the preview card:**
Given a recording with appearance values saved
When preview chunks arrive and the card renders
Then:
- `color` = `previewTextColor`
- `background` = `previewBgColor` (may include opacity in rgba form)
- `backdropFilter` = `blur(${previewBgBlur}px)` (0 = no blur)
- `-webkit-backdrop-filter` = same
- `border` = `${previewBorderWidth}px solid ${previewBorderColor}`
- `borderRadius` = `${previewBorderRadius}px`
- `fontFamily` = `previewFontFamily`
And `CARD_RADIUS` constant in PreviewPanel is replaced by the live setting value (not a hardcode)
so the R11 inversion check in 6.2 doc remains accurate.

**AC-6 — Config round-trip + camelCase tests:**
Given the new AppConfig fields
When serialized to JSON and deserialized back
Then all seven fields survive the round-trip with exact values, the camelCase keys are correct
in the JSON, and a config missing all seven fields deserializes with the correct defaults.

**DoD:**
- Real Windows release build + manual smoke (`sync-and-build.ps1`):
  - Change text color in Settings → save → open preview → text uses new color
  - Change font family (e.g. `"monospace"`) → save → open preview → text renders in new font
  - Change border thickness from 1 to 3 → preview shows thicker border
  - Change corner radius from 14 to 8 → no white-line artifact (R11)
  - Default appearance (all defaults) must exactly match the current preview card appearance
- `tsc` + `vite build` green (no TS errors)
- `cargo check --target x86_64-pc-windows-gnu` green
- `cargo test` (Linux, lib tests) green + no new clippy warnings on touched files
- Walk `docs/surface-smoke-checklist.md` items #1, #2, #3

## Tasks / Subtasks

- [x] Task 1: Add seven new fields to `AppConfig` in `src-tauri/src/config/mod.rs` (AC-1, AC-6)
  - [x] 1.1 Add seven fields with `#[serde(default = ...)]` + named default fns; defaults match current
        hardcoded values in `PreviewPanel.tsx` (text, bg, blur, border color, border width, radius,
        font family)
  - [x] 1.2 Add the fields to `AppConfig::default()` (the `Default` impl), `TEST_CONFIG_MINIMAL`,
        and all fixture configs used in tests so they compile and the snapshots stay clean
  - [x] 1.3 Add spec `spec_preview_appearance_config_fields_default` (inline `#[cfg(test)]`):
        round-trip + missing-field + camelCase assertions for all seven fields; inversion guard
        must make it RED (document with comment like Story 5-3's pattern)

- [x] Task 2: Add fields to `SettingsPatch` + `merge_settings` + `save_settings` + `get_settings`
      in `src-tauri/src/commands/settings.rs` (AC-2)
  - [x] 2.1 Add seven `Option<…>` fields to `SettingsPatch` struct with `None` defaults
  - [x] 2.2 Add seven `unwrap_or(existing.*)` arms in `merge_settings()`
  - [x] 2.3 Add seven parameters to `save_settings` Tauri command (all `Option<…>`, after
        `preview_panel_form`) and route them into the `SettingsPatch`
  - [x] 2.4 Add seven fields to the `AppSettingsResponse` returned by `get_settings`
  - [x] 2.5 Add spec `spec_preview_appearance_settings_patch_round_trip` in the settings tests
        block: mutate a single field → verify the merge result; inversion test for AC-2

- [x] Task 3: Update TypeScript types + tauri-commands.ts (AC-2, AC-3)
  - [x] 3.1 Add seven fields to the `AppSettings` interface in `src/types.ts`
        (`previewTextColor: string`, `previewBgColor: string`, `previewBgBlur: number`,
        `previewBorderColor: string`, `previewBorderWidth: number`, `previewBorderRadius: number`,
        `previewFontFamily: string`)
  - [x] 3.2 Add seven fields to `MOCK_SETTINGS` in `src/tauri-commands.ts` (with exact same defaults
        as the Rust defaults — must match the hardcoded values in `PreviewPanel.tsx`)
  - [x] 3.3 Add seven optional parameters to `saveSettings()` in `tauri-commands.ts` (after
        `previewPanelForm`); add them to the `invoke("save_settings", {...})` call

- [x] Task 4: Update SettingsPanel + ShortcutsContent to include appearance controls (AC-2)
  - [x] 4.1 Add seven `local*` state variables + setters in `SettingsPanel.tsx`; initialize from
        `loadedSettings` with correct defaults (matching the serde defaults)
  - [x] 4.2 Add the seven new fields to the resync `useEffect` (the `loadedSettings` dependency
        block at line ~288) — **Trap #2 prevention**: every new settings field MUST appear here
        or the Save button stays dirty forever
  - [x] 4.3 Add the seven fields to the `isDirty` computation `useEffect`
  - [x] 4.4 Add the seven fields to the save `onSave(...)` call in `handleSave`
  - [x] 4.5 Pass the seven props down to `ShortcutsContent` via props + types update
  - [x] 4.6 In `ShortcutsContent.tsx`: add an "Appearance" sub-section inside the
        `localLivePreviewEnabled && (...)` block (below the Darstellung/width-preset picker)
        with appropriate input controls for each field (see Dev Notes for control types)

- [x] Task 5: Update `PreviewPanel.tsx` to read appearance reactively + apply CSS (AC-3, AC-4, AC-5)
  - [x] 5.1 In `runShowSequence()`, after reading `s.previewPanelForm`, also read the seven
        appearance fields from `getSettings()` and store them in refs (e.g. `appearanceRef`)
        — reactive, NOT mount-only (**Trap #3 prevention**)
  - [x] 5.2 Replace the hardcoded `CARD_RADIUS = 14` constant with the live
        `previewBorderRadius` value from `appearanceRef.current` so CSS and Rust region are
        always in sync (AC-4, AC-5)
  - [x] 5.3 Apply all seven appearance values as CSS inline styles on the card element
        (replace the hardcoded string literals in the `style` prop); include `fontFamily`
  - [x] 5.4 Log the applied appearance values in the `[preview] shown: ...` console.log line
        so the console-bridge trace confirms the values actually arrived

- [x] Task 6: Update `set_preview_shape` to read radius from config (AC-4)
  - [x] 6.1 Add `preview_border_radius: i32` parameter to `set_preview_shape` Tauri command
        (or make it read from `AppState` — see Dev Notes for preferred approach)
  - [x] 6.2 Replace hardcoded `14.0` with `preview_border_radius as f64` in the region radius
        computation: `r = (preview_border_radius as f64 * scale) as i32`
  - [x] 6.3 Update `setPreviewShape()` in `tauri-commands.ts` to pass the radius
  - [x] 6.4 Update the call site in `runShowSequence()` to pass the radius value
  - [x] 6.5 Inversion check (smoke-time): set CSS `borderRadius` to 8 while passing 14 to
        `set_preview_shape` → white-line artifact → RED (document this inversion target)

- [x] Task 7: Verify DoD + smoke checklist
  - [x] 7.1 Run `tsc --noEmit` + `vite build` green
  - [x] 7.2 Run `cargo test` (Linux) + `cargo check --target x86_64-pc-windows-gnu` green + clippy
  - [ ] 7.3 Windows smoke (Andy, `sync-and-build.ps1`): change each appearance field, save,
        open preview — confirm CSS applies correctly; corner radius 8 → no white-line (R11)

## Dev Notes

### Critical Traps for This Story (from `docs/surface-smoke-checklist.md`)

**Trap #1 (camelCase config keys):** `AppConfig` uses `#[serde(rename_all = "camelCase")]`.
Every new field name must produce the correct camelCase JSON key:
- `preview_text_color` → `previewTextColor` ✓
- `preview_bg_color` → `previewBgColor` ✓
- `preview_bg_blur` → `previewBgBlur` ✓
- `preview_border_color` → `previewBorderColor` ✓
- `preview_border_width` → `previewBorderWidth` ✓
- `preview_border_radius` → `previewBorderRadius` ✓
- `preview_font_family` → `previewFontFamily` ✓

A wrong-case key is silently ignored by serde → the feature is silently off. The config-field spec
(Task 1.3) directly catches this by asserting the deserialized value equals the default.

**Trap #2 (resync useEffect / stuck-dirty Save button):** Every new settings field added to
`SettingsPanel.tsx` MUST be added to the `loadedSettings` resync `useEffect` (around line 288 in
`SettingsPanel.tsx`). Missing a new `String` field (`previewTextColor`, `previewFontFamily`, etc.)
causes the Save button to stay dirty forever after a save because the `isDirty` check sees a stale
value in the local state. This trap is well-established (Story 5-3 f32-resync bug).

**Trap #3 (separate-window reactivity):** `PreviewPanel.tsx` runs in the `"preview"` window, which
is a SEPARATE Tauri window that never re-mounts when the Settings panel saves. Any `getSettings()`
call at mount-time freezes on the app-start value. Appearance fields MUST be read reactively in
`runShowSequence()` (which runs on the closed→open transition for the first chunk of each cycle),
NOT in a mount-time `useEffect`. See Story 5-5 for the prior fix of the same pattern.

**R11 Invariant (corner radius):** The OS window region radius in `set_preview_shape` MUST equal
the CSS `borderRadius`. Currently hardcoded at 14 in both places. When the user sets a custom
radius, BOTH the CSS value and the Rust region radius must use the same number or a white-line
artifact appears at the corners. See `docs/bar-redesign-spec.md §3.7` (R11 "trivial") and
`src-tauri/src/commands/misc.rs:480` (`set_preview_shape`).

### Approach for `set_preview_shape` radius (Task 6)

**Preferred:** Pass `radius: i32` as an explicit parameter to `set_preview_shape` command. This
avoids adding another `State` read and makes the call site transparent about what value is used.

```rust
// src-tauri/src/commands/misc.rs
#[tauri::command]
pub fn set_preview_shape(handle: AppHandle, radius: i32) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        // ...
        let r = (radius as f64 * scale) as i32;
        crate::set_window_region_round_rect(h, w, ht, r);
    }
    Ok(())
}
```

```ts
// src/tauri-commands.ts
export async function setPreviewShape(radius: number): Promise<void> {
  if (isPreviewMode) return;
  await invoke("set_preview_shape", { radius });
}
```

Call site in `runShowSequence()`:
```ts
await setPreviewShape(appearanceRef.current.borderRadius);
```

### Control Types for the Appearance Sub-section (Task 4.6)

The UI pattern follows the existing Darstellung/width-preset picker in `ShortcutsContent.tsx`.
Keep controls simple (no color pickers — too complex for v1):

| Field | Control type | Value range / options |
|---|---|---|
| `previewTextColor` | Text input (string) | CSS color or rgba string |
| `previewBgColor` | Text input (string) | CSS color or rgba string |
| `previewBgBlur` | Range slider | 0–20 px, step 1 |
| `previewBorderColor` | Text input (string) | CSS color or rgba string |
| `previewBorderWidth` | Range slider | 0–5 px, step 1 |
| `previewBorderRadius` | Range slider | 0–24 px, step 1 |
| `previewFontFamily` | Text input (string) | CSS font-family string |

Text inputs for colors and font-family (accept any valid CSS string: named color, hex, rgba, font
stack). Sliders for the numeric fields. This avoids external picker dependencies while still being
functional. Label each control clearly. Show the current value for sliders (like the Preview Pause
slider does: `{localPreviewBgBlur}px`). For font-family, a short placeholder like
`"'Inter', system-ui, sans-serif"` guides the user.

### Files to Touch

| File | Change | Direction |
|---|---|---|
| `src-tauri/src/config/mod.rs` | Add 7 `AppConfig` fields + defaults + spec | UPDATE |
| `src-tauri/src/commands/settings.rs` | `SettingsPatch` + `merge_settings` + `save_settings` + `get_settings` + spec | UPDATE |
| `src/types.ts` | Add 7 fields to `AppSettings` | UPDATE |
| `src/tauri-commands.ts` | `MOCK_SETTINGS` + `saveSettings()` | UPDATE |
| `src/components/SettingsPanel.tsx` | 7 state vars + resync useEffect + isDirty + save call + pass props | UPDATE |
| `src/components/settings/ShortcutsContent.tsx` | Props type + Appearance sub-section | UPDATE |
| `src/PreviewPanel.tsx` | `runShowSequence()` reactive read + appearanceRef + CSS apply + pass radius | UPDATE |
| `src-tauri/src/commands/misc.rs` | `set_preview_shape` gains `radius: i32` param | UPDATE |

### Appearance Defaults (must match current PreviewPanel.tsx hardcodes exactly)

These defaults are what a user sees today (before any settings change) — they MUST be identical in
the Rust defaults, MOCK_SETTINGS, and the initial resync values in SettingsPanel:

```
previewTextColor:   "rgba(220,220,220,0.88)"                   // src/PreviewPanel.tsx line 310: color
previewBgColor:     "rgba(25,25,25,0.96)"                       // line 295: background
previewBgBlur:      12                                           // line 297: blur(12px)
previewBorderColor: "rgba(42,195,168,0.25)"                     // line 296: border color
previewBorderWidth: 1                                            // line 296: 1px solid
previewBorderRadius: 14                                          // CARD_RADIUS constant, line 26
previewFontFamily:  "'Inter', system-ui, -apple-system, sans-serif"  // line 311: fontFamily
```

### AppConfig field declaration pattern (follow Story 5-3 / 5-5 precedents)

```rust
// In AppConfig struct (config/mod.rs), grouped with other preview fields ~line 719:
#[serde(default = "default_preview_text_color")]
pub preview_text_color: String,

#[serde(default = "default_preview_bg_color")]
pub preview_bg_color: String,

#[serde(default = "default_preview_bg_blur")]
pub preview_bg_blur: u8,            // 0–20 px, u8 is sufficient

#[serde(default = "default_preview_border_color")]
pub preview_border_color: String,

#[serde(default = "default_preview_border_width")]
pub preview_border_width: u8,       // 0–5 px, u8 is sufficient

#[serde(default = "default_preview_border_radius")]
pub preview_border_radius: u8,      // 0–24 px, u8 is sufficient

#[serde(default = "default_preview_font_family")]
pub preview_font_family: String,

// ... corresponding default fns near line 927:
fn default_preview_text_color() -> String   { "rgba(220,220,220,0.88)".to_string() }
fn default_preview_bg_color() -> String     { "rgba(25,25,25,0.96)".to_string() }
fn default_preview_bg_blur() -> u8          { 12 }
fn default_preview_border_color() -> String { "rgba(42,195,168,0.25)".to_string() }
fn default_preview_border_width() -> u8     { 1 }
fn default_preview_border_radius() -> u8    { 14 }
fn default_preview_font_family() -> String  { "'Inter', system-ui, -apple-system, sans-serif".to_string() }
```

**Note on u8 for numeric fields:** `preview_bg_blur`, `preview_border_width`,
`preview_border_radius` are all small integers (max ~24). Using `u8` keeps the type honest.
In `SettingsPatch` they become `Option<u8>`; in the merge they `unwrap_or(existing.*)`.
In `save_settings` they arrive as `Option<u8>`. In `get_settings` they are returned as `u8`.
In TypeScript, `AppSettings` fields become `number` (no distinction needed client-side).
`preview_font_family` is a `String` in Rust / `string` in TypeScript — same as the color fields.

### Config round-trip + camelCase spec pattern (Task 1.3)

Follow the exact same pattern as `spec_live_preview_config_fields_default` (line ~3908 in
`config/mod.rs`): serialize a full `AppConfig`, remove the 6 new keys from the JSON object,
deserialize, and assert each field equals its expected default. Also assert the serialized keys
are camelCase (e.g., check `json["previewTextColor"]` is present, not `json["preview_text_color"]`).

### TypeScript `AppSettings` types for new fields

```ts
// src/types.ts — add after `previewPanelForm`:
/** Preview text appearance: CSS color string (e.g. "rgba(220,220,220,0.88)"). */
previewTextColor: string;
/** Preview box background: CSS color string (may include opacity). */
previewBgColor: string;
/** Preview box backdrop-blur radius in px (0 = no blur). */
previewBgBlur: number;
/** Preview border color: CSS color string. */
previewBorderColor: string;
/** Preview border thickness in px. */
previewBorderWidth: number;
/** Preview corner radius in px. MUST match set_preview_shape radius (R11). */
previewBorderRadius: number;
/** Preview font family: CSS font-family string (e.g. "'Inter', system-ui, sans-serif"). */
previewFontFamily: string;
```

### PreviewPanel.tsx — reactive appearance read pattern (Task 5)

In `runShowSequence()`, after reading `s.previewPanelForm`, add:
```ts
appearanceRef.current = {
  textColor:    s.previewTextColor    ?? "rgba(220,220,220,0.88)",
  bgColor:      s.previewBgColor      ?? "rgba(25,25,25,0.96)",
  bgBlur:       s.previewBgBlur       ?? 12,
  borderColor:  s.previewBorderColor  ?? "rgba(42,195,168,0.25)",
  borderWidth:  s.previewBorderWidth  ?? 1,
  borderRadius: s.previewBorderRadius ?? 14,
  fontFamily:   s.previewFontFamily   ?? "'Inter', system-ui, -apple-system, sans-serif",
};
```

Declare `appearanceRef` near the other refs (after `showOnceRef`):
```ts
const appearanceRef = useRef({
  textColor:    "rgba(220,220,220,0.88)",
  bgColor:      "rgba(25,25,25,0.96)",
  bgBlur:       12,
  borderColor:  "rgba(42,195,168,0.25)",
  borderWidth:  1,
  borderRadius: 14,
  fontFamily:   "'Inter', system-ui, -apple-system, sans-serif",
});
```

Then use `appearanceRef.current.*` in the card's `style` prop so each render sees the latest
values (React renders after the state update, refs are always up-to-date).

Remove the `CARD_RADIUS` constant (line 26). It was the old hardcode; `appearanceRef.current.borderRadius` replaces it everywhere.

The hardcoded `fontFamily: "'Inter', system-ui, -apple-system, sans-serif"` at line 311 is replaced
by `appearanceRef.current.fontFamily`.

### Surface-smoke checklist items applicable to this story

- **#1 (camelCase):** 7 new config keys — verify on-disk names after save
- **#2 (resync useEffect):** 4 String + 3 numeric fields all need resync (including `previewFontFamily`)
- **#3 (separate-window reactivity):** `PreviewPanel` reads appearance in `runShowSequence()`,
  not at mount — verify by saving a new color or font-family, opening a new preview without restart

### Dependency note

This story depends on 6.2 (done). It is parallel to 6.3 and 6.4. No overlap with them except
both 6.3 and 6.6 read settings in `runShowSequence()` — when both are merged, the single
`getSettings()` call should read all fields at once to minimize IPC calls.

### References

- `[Source: _bmad-output/planning-artifacts/epics-bar-redesign.md#Story 6.6]` — FR11, FR12, FR13, NFR5, NFR6
- `[Source: docs/bar-redesign-spec.md §3.7]` — R11 region/CSS radius mismatch
- `[Source: src/PreviewPanel.tsx]` — current hardcoded appearance values (lines 293–315)
- `[Source: src-tauri/src/commands/misc.rs:480]` — `set_preview_shape` Rust command
- `[Source: src-tauri/src/config/mod.rs:719]` — existing preview fields location in AppConfig
- `[Source: src-tauri/src/commands/settings.rs:153]` — `SettingsPatch` preview fields pattern
- `[Source: src/components/settings/ShortcutsContent.tsx:433]` — existing preview sub-section
- `[Source: src/components/SettingsPanel.tsx:288]` — resync useEffect (Trap #2 location)
- `[Source: docs/surface-smoke-checklist.md]` — Traps #1, #2, #3
- `[Source: _bmad-output/implementation-artifacts/6-2-move-live-preview-into-the-window-css-grow-scale-geometry.md]` — prior story; established CARD_RADIUS, set_preview_shape call sequence, reactive read pattern

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

None — implementation was straight-forward following the story spec.

### Completion Notes List

- AC-1/AC-6: 7 AppConfig fields added with `#[serde(default)]` + default fns. camelCase keys confirmed by spec test. Defaults match PreviewPanel.tsx hardcodes exactly. No migration write (additive serde defaults).
- AC-2: SettingsPatch, merge_settings, save_settings, SettingsView/get_settings all updated. Spec `spec_preview_appearance_settings_patch_round_trip` added and passing.
- AC-3/Trap #3: PreviewPanel reads all appearance fields in `runShowSequence()` (NOT mount-time). Single getSettings() call reads widthPreset + all 7 appearance fields together (minimal IPC).
- AC-4/R11: `set_preview_shape` now accepts `radius: i32` param. PreviewPanel passes `appearanceRef.current.borderRadius` to it. CSS borderRadius and Rust region radius always use the same value.
- AC-5: CSS card style uses `appearanceRef.current.*` for all 7 values — no more hardcoded rgba strings.
- AC-2/UI: ShortcutsContent.tsx gains "Appearance" sub-section with 4 text inputs (colors/font) and 3 sliders (blur/width/radius), shown when `localLivePreviewEnabled`.
- Trap #1 (camelCase): confirmed by spec test asserting `json["previewTextColor"]` etc.
- Trap #2 (resync useEffect): all 7 fields added to the loadedSettings resync useEffect in SettingsPanel.tsx.
- Trap #3 (separate-window reactivity): appearanceRef populated in runShowSequence(), not at mount.
- Test fixture `test_merge_settings_happy_path_full_patch` in settings.rs updated with `None` for new fields (it was a complete struct literal).
- 3 SettingsView struct literals in lib.rs tests updated with the 7 new fields.
- Build results: cargo test 574/574 pass; tsc exit 0; vite build green; no new clippy errors.
- Task 7.3 (Windows smoke) is the remaining surface-class gate — requires Andi to run sync-and-build.ps1 on Windows.

### File List

- src-tauri/src/config/mod.rs
- src-tauri/src/commands/settings.rs
- src-tauri/src/commands/misc.rs
- src-tauri/src/lib.rs
- src/types.ts
- src/tauri-commands.ts
- src/components/SettingsPanel.tsx
- src/components/settings/ShortcutsContent.tsx
- src/PreviewPanel.tsx

## Review Findings

Code review 2026-06-07 (Opus 4.8, 3 adversarial layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor). 2 patch, 2 defer, 9 dismissed. Acceptance Auditor: all ACs satisfied, `previewFontFamily` complete end-to-end, only Windows smoke (Task 7.3) outstanding.

- [ ] [Review][Patch] Appearance rendered from `useRef`, not state → first-chunk / single-chunk cycle paints stale appearance + region/CSS radius desync (reintroduces R11 white-line on first paint) [src/PreviewPanel.tsx:65,123,178,325-347]
- [ ] [Review][Patch] Empty color/font input persists `""`; `?? default` does not catch `""` → invalid CSS (`color:""`, `border:"1px solid "`, `fontFamily:""`) defeats legibility goal [src/PreviewPanel.tsx:124-130]
- [x] [Review][Defer] R11 region/CSS corner-radius clamp asymmetry for radius beyond UI slider bounds (Rust clamps ellipse to min(2r,w/h), CSS to half-box) [src-tauri/src/lib.rs:587 vs src/PreviewPanel.tsx:332] — deferred, unreachable via UI sliders (0–24)
- [x] [Review][Defer] `u8` preview numeric config (blur/width/radius) accepts 0–255 on deserialize; hand-edited config.json bypasses slider caps (≤20/≤5/≤24) [src-tauri/src/config/mod.rs] — deferred, unreachable via UI; same class as existing unclamped numeric config fields

## Change Log

- 2026-06-07: Code review (Opus 4.8, 3 layers) — 2 patch / 2 defer / 9 dismissed; fix-loop applied the 2 patches (state-driven appearance for R11/first-frame correctness; `|| default` for the 4 string fields). Re-verified green.
- 2026-06-07: Story implemented by claude-sonnet-4-6. All AC-1 through AC-5 satisfied in code (AC-6 round-trip covered by spec tests). 7 AppConfig fields added (preview text/bg/border appearance + font family), plumbed through SettingsPatch/merge_settings/save_settings/get_settings/SettingsView, TypeScript AppSettings + MOCK_SETTINGS + saveSettings() updated, SettingsPanel + ShortcutsContent extended with Appearance sub-section (4 text inputs + 3 sliders), PreviewPanel.tsx refactored to reactive appearanceRef pattern (Trap #3), CARD_RADIUS constant replaced, set_preview_shape gains radius param (R11). cargo test 574/574, tsc exit 0, vite build green. Task 7.3 Windows smoke pending.
