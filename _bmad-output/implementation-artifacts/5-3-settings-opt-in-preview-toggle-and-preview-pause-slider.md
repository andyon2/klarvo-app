---
story: "5.3"
epic: "5"
title: "Settings — opt-in Preview toggle + Preview-Pause slider (Regler A)"
status: review
track: L3-feature
gatedBy: ["5.1"]
buildsOn: ["5.1"]
enabledBy: ["5.5"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics-live-preview.md
  - _bmad-output/project-context.md
  - _bmad-output/implementation-artifacts/5-1-backend-pause-triggered-delta-flush-for-toggle-hold.md
  - _bmad-output/implementation-artifacts/5-2-frontend-auto-expand-preview-panel.md
---

# Story 5.3: Settings — opt-in Preview toggle + Preview-Pause slider (Regler A)

Status: review

## Story

As a user,
I want a Settings toggle to turn the live preview on/off and a Preview-Pause slider to set
how long a pause triggers a flush,
so that the preview is opt-in (default off) and I can tune its responsiveness without
any behavior change if I haven't enabled it.

## Acceptance Criteria

**AC-1 (Live Preview toggle — FR6):**
Given the Shortcut section of the Settings UI (desktop only — `isDesktop` guard)
And the `live_preview_enabled` field already exists in `AppConfig` (introduced by Story 5.1,
  serde default = `false`) and in `SettingsPatch` / `merge_settings` (added by this story)
When the user toggles "Live Preview"
Then the toggle's local state flips
And on Save the field is written via the **single sanctioned `save_settings` write path**
  (ADR-0015 / Story 4-3 `save_config_locked` choke-point — no second writer)
And with `live_preview_enabled = false` the backend emits NO `klarvo://live-preview-chunk`
  events, so the Story 5.2 panel never appears — FR6 off-by-default contract.

**AC-2 (Preview-Pause slider — FR8/D3 — Regler A):**
Given the Shortcut section
When the user adjusts the "Preview-Pause" slider
Then the `preview_pause_silence_secs` field (introduced in 5.1; range 0.5–5.0 s, step 0.1,
  default 2.0 matching Story 5.1's serde default) is included in the Save payload
And on Save it is written via `save_settings` → `SettingsPatch` → `merge_settings`
And Story 5.1's flush uses this value as the pause threshold (FR8/D3).

**AC-3 (Defaults — no migration write — NFR2/NFR3):**
Given a fresh or existing `config.json` with neither `livePreviewEnabled` nor
  `previewPauseSilenceSecs` present
When settings are loaded via `get_settings`
Then `live_preview_enabled` reads `false` and `preview_pause_silence_secs` reads `2.0`
  via Story 5.1's serde defaults
And NO migration write is triggered (additive defaults — existing users see zero behavior
  change, NFR2).

**AC-4 (Slider trade-off hint — epics spec):**
Given the Preview-Pause slider is shown
When the user reads the hint text beneath it
Then it communicates the trade-off: short pause = more responsive + more Groq calls +
  shorter context per segment; long pause = less responsive + fewer calls + better context.

**AC-5 (Backend round-trip — Rust `SettingsPatch` / `merge_settings` / `get_settings`):**
Given the two fields `live_preview_enabled: Option<bool>` and
  `preview_pause_silence_secs: Option<f32>` are added to `SettingsPatch`
And the placeholder lines in `merge_settings` (currently:
  `live_preview_enabled: existing.live_preview_enabled`) are updated to use
  `patch.live_preview_enabled.unwrap_or(existing.live_preview_enabled)` — same pattern as
  all other optional fields
And `SettingsView` gains two new fields (`live_preview_enabled: bool`,
  `preview_pause_silence_secs: f32`) and `get_settings` populates them from `cfg`
When `save_settings` is called with the new params and then `get_settings` is called
Then the round-trip test (a unit test on `merge_settings`) asserts:
  - writing `live_preview_enabled = Some(true)` results in `true` in merged config
  - writing `live_preview_enabled = None` preserves the existing value
  - writing `preview_pause_silence_secs = Some(1.5)` results in `1.5`
  - the inversion test: swap `Some(true)` to `Some(false)` → the spec goes RED

**AC-6 (TypeScript surface — `AppSettings`, `saveSettings`, `tauri-commands.ts`):**
Given `AppSettings` in `src/types.ts` does not yet include `livePreviewEnabled` or
  `previewPauseSilenceSecs`
When this story ships
Then both fields are added to `AppSettings` (matching the camelCase serde output from Rust:
  `livePreviewEnabled: boolean` and `previewPauseSilenceSecs: number`)
And `saveSettings(...)` in `src/tauri-commands.ts` gains two new optional parameters:
  `livePreviewEnabled?: boolean | null` and `previewPauseSilenceSecs?: number | null`
And the invoke call includes them: `livePreviewEnabled: livePreviewEnabled ?? null` /
  `previewPauseSilenceSecs: previewPauseSilenceSecs ?? null`
And `tsc` / `npm run build` passes with no new type errors.

**AC-7 (Desktop-only display — FR4 / isDesktop guard):**
Given the toggle and slider are added inside the existing `isDesktop` block in
  `ShortcutsContent.tsx`
When the app runs on Android (where there is no preview feature)
Then neither the toggle nor the slider is rendered
And no Android code change is required (NFR3 / ADR-0016).

**DoD (Surface story):**
- **Windows release build** via `scripts/sync-and-build.ps1` (mandatory — Linux tests mask
  Tauri runtime + WebView2 rendering bugs; project-context.md testing rule).
- **Manual smoke**: toggle on → save → verify `livePreviewEnabled: true` written in
  `%APPDATA%\com.klarvo.voice\config.json` (⚠️ **camelCase** key — `AppConfig` uses
  `#[serde(rename_all = "camelCase")]`; the snake_case `live_preview_enabled` is silently
  ignored by serde). Move the Preview-Pause slider → save → verify
  `previewPauseSilenceSecs` value changed. Toggle off → save → verify
  `livePreviewEnabled: false` and preview panel no longer appears during dictation.
- `cargo test` on touched Rust files (merge_settings round-trip test green).
- `tsc` / `npm run build` passes.
- `cargo clippy` clean on touched Rust files.

## Tasks / Subtasks

### Task 1: Rust backend — add fields to `SettingsPatch`, `merge_settings`, `SettingsView`, `get_settings`, and `save_settings` (AC-5)

- [x] 1.1 In `src-tauri/src/commands/settings.rs`, add to the `SettingsPatch` struct:
  ```rust
  pub live_preview_enabled: Option<bool>,
  pub preview_pause_silence_secs: Option<f32>,
  ```
  And in `impl Default for SettingsPatch`, initialize both to `None`.

- [x] 1.2 In `merge_settings`, replace the two placeholder lines (currently passing through
  `existing.*` verbatim) with the standard `Option::unwrap_or` pattern:
  ```rust
  live_preview_enabled: patch.live_preview_enabled
      .unwrap_or(existing.live_preview_enabled),
  preview_pause_silence_secs: patch.preview_pause_silence_secs
      .unwrap_or(existing.preview_pause_silence_secs),
  ```

- [x] 1.3 In `src-tauri/src/lib.rs`, add two new fields to `pub struct SettingsView`:
  ```rust
  /// Whether live-preview is enabled (opt-in, default false).
  pub live_preview_enabled: bool,
  /// Silence duration (seconds) that triggers a preview flush in Toggle/Hold mode.
  pub preview_pause_silence_secs: f32,
  ```

- [x] 1.4 In `get_settings` in `settings.rs`, add to the `SettingsView { ... }` literal:
  ```rust
  live_preview_enabled: cfg.live_preview_enabled,
  preview_pause_silence_secs: cfg.preview_pause_silence_secs,
  ```

- [x] 1.5 In the `save_settings` command signature, add two new optional parameters at the
  end (after `openrouter_api_key`):
  ```rust
  live_preview_enabled: Option<bool>,
  preview_pause_silence_secs: Option<f32>,
  ```
  And in the `SettingsPatch { ... }` construction block inside `save_settings`, include:
  ```rust
  live_preview_enabled,
  preview_pause_silence_secs,
  ```

- [x] 1.6 Write a `merge_settings` round-trip unit test (inline `#[cfg(test)]` in
  `settings.rs`):
  ```rust
  #[test]
  fn spec_live_preview_settings_patch_round_trip() {
      // AC-5: patch Some(true) / Some(1.5) round-trips correctly
      // AC-5: patch None preserves existing value
      // INVERSION: change Some(true) to Some(false) → asserted true becomes false → RED
  }
  ```
  Empirically verify the inversion (flip `Some(true)` → `Some(false)` makes the test RED).

- [x] 1.7 `cargo clippy` clean on `src-tauri/src/commands/settings.rs` and
  `src-tauri/src/lib.rs` — no new warnings.
  `cargo test` on `src-tauri/src/` — all lib tests green (expected: ~566+).

### Task 2: TypeScript surface — `AppSettings`, `saveSettings` (AC-6)

- [x] 2.1 In `src/types.ts`, add two fields to `AppSettings`:
  ```ts
  /** Whether live preview is enabled (opt-in, default false). Desktop only. */
  livePreviewEnabled: boolean;
  /** Silence duration (seconds) that triggers a preview flush in Toggle/Hold. */
  previewPauseSilenceSecs: number;
  ```
  Note: these must be camelCase to match `AppConfig`'s `#[serde(rename_all = "camelCase")]`
  — the JSON keys are `livePreviewEnabled` and `previewPauseSilenceSecs` exactly.

- [x] 2.2 In `src/tauri-commands.ts`, update `saveSettings(...)`:
  - Add two new optional parameters at the end:
    ```ts
    livePreviewEnabled?: boolean | null,
    previewPauseSilenceSecs?: number | null,
    ```
  - Add to the `invoke("save_settings", { ... })` call:
    ```ts
    livePreviewEnabled: livePreviewEnabled ?? null,
    previewPauseSilenceSecs: previewPauseSilenceSecs ?? null,
    ```
  - Update the `MOCK_SETTINGS` constant (add `livePreviewEnabled: false,
    previewPauseSilenceSecs: 2.0` to its definition — prevents TS strict-mode errors).

- [x] 2.3 `npm run build` (TypeScript strict check) — PASS: 0 errors.

### Task 3: Settings UI — toggle + slider in `ShortcutsContent.tsx` and `SettingsPanel.tsx` (AC-1, AC-2, AC-4, AC-7)

- [x] 3.1 In `src/components/settings/ShortcutsContent.tsx`:
  - Add two new props to `ShortcutsContentProps`:
    ```ts
    localLivePreviewEnabled: boolean;
    setLocalLivePreviewEnabled: (v: boolean) => void;
    localPreviewPauseSilenceSecs: number;
    setLocalPreviewPauseSilenceSecs: (v: number) => void;
    ```
  - Inside the `{isDesktop && (...)}` block, add a new section **after** the Silence Duration
    slider (which is inside the `autostop`/`auto` guard) and **before** the closing
    `</div>` — this keeps it in the Shortcuts section but visible regardless of hotkey mode.
    The section should contain:
    - **"Live Preview" subsection heading** (same style as "Paste & Behavior")
    - A toggle row for "Live Preview" (same toggle pattern as Auto-Paste in SettingsPanel:
      `role="switch"`, `aria-checked`, teal-on/elevated-off, sliding thumb)
    - A descriptive sub-line: "Show raw transcription while you dictate in Toggle/Hold mode."
    - Conditionally (when `localLivePreviewEnabled` is `true`): the Preview-Pause slider:
      - Label: "Preview Pause" with current value display (`{localPreviewPauseSilenceSecs.toFixed(1)}s`)
      - `type="range"` `min={0.5}` `max={5.0}` `step={0.1}` `className="w-full accent-klarvo-primary"`
      - Trade-off hint text below (AC-4): "Short = more responsive, more Groq calls, less context per segment. Long = less responsive, fewer calls, better context."

- [x] 3.2 In `src/components/SettingsPanel.tsx`:
  - Initialize two new local state variables (after the `localVoiceCommandEnabled` state):
    ```tsx
    const [localLivePreviewEnabled, setLocalLivePreviewEnabled] = useState(
      loadedSettings?.livePreviewEnabled ?? false
    );
    const [localPreviewPauseSilenceSecs, setLocalPreviewPauseSilenceSecs] = useState(
      loadedSettings?.previewPauseSilenceSecs ?? 2.0
    );
    ```
  - Include both fields in the `isDirty` calculation (the `useMemo` or `useEffect` that
    computes `isDirty`) — add:
    ```ts
    (loadedSettings?.livePreviewEnabled ?? false) !== localLivePreviewEnabled ||
    (loadedSettings?.previewPauseSilenceSecs ?? 2.0) !== localPreviewPauseSilenceSecs ||
    ```
  - Pass both through to `ShortcutsContent` in the JSX:
    ```tsx
    localLivePreviewEnabled={localLivePreviewEnabled}
    setLocalLivePreviewEnabled={setLocalLivePreviewEnabled}
    localPreviewPauseSilenceSecs={localPreviewPauseSilenceSecs}
    setLocalPreviewPauseSilenceSecs={setLocalPreviewPauseSilenceSecs}
    ```
  - In `saveCurrentSettings`, pass both new fields to `onSave(...)` / `saveSettings(...)`:
    append `localLivePreviewEnabled` and `localPreviewPauseSilenceSecs` to the parameter list.
  - Add both to the `useCallback` dependency array of `saveCurrentSettings`.

- [x] 3.3 Propagate through `SettingsPanel`'s `onSave` prop type and the `App.tsx` call-site:
  - The `onSave` prop type in `SettingsPanel` already passes through all saveSettings params;
    verify that adding two new params to the `saveSettings(...)` call in `SettingsPanel`
    correctly propagates through to the Tauri command call in App.tsx's `handleSave` (it
    should auto-propagate because App.tsx just passes `saveSettings` directly to `onSave`).

- [x] 3.4 `npm run build` after UI changes — 0 TypeScript errors.

### Task 4: Final validation (AC-1..AC-7, DoD)

- [x] 4.1 `npm run build` — PASS: 0 TypeScript errors.
- [x] 4.2 `cargo test` — all lib tests green (merge_settings round-trip test green, AC-5).
- [x] 4.3 `cargo clippy` on touched Rust files — no new warnings introduced.
- [ ] 4.4 Windows release build via `scripts/sync-and-build.ps1`.
- [ ] 4.5 Manual smoke:
  1. Open Settings → Shortcuts section
  2. Confirm "Live Preview" toggle is **off** by default (AC-3)
  3. Toggle **on** → click Save → check `%APPDATA%\com.klarvo.voice\config.json` for
     `"livePreviewEnabled": true` (⚠️ camelCase — see Dev Notes)
  4. Set Preview-Pause slider to 1.5 s → Save → check config for
     `"previewPauseSilenceSecs": 1.5`
  5. Start a Toggle dictation with ≥1 pause → confirm panel appears (5.2 end-to-end check)
  6. Toggle **off** → Save → dictate again → confirm panel **does not** appear

## Dev Notes

### camelCase config.json — Critical pitfall from 5.2 smoke

`AppConfig` is `#[serde(rename_all = "camelCase")]`. The JSON keys are:
- `"livePreviewEnabled"` (NOT `"live_preview_enabled"` — serde silently ignores the wrong key)
- `"previewPauseSilenceSecs"` (NOT `"preview_pause_silence_secs"`)

This bit the 5.2 smoke (the "no chunks" mystery was the wrong key being used). The manual
smoke in Task 4.5 MUST use the camelCase keys when verifying `config.json` directly.

### SettingsPatch `Option` pattern — mirror existing fields

Every existing Optional field in `SettingsPatch` follows the same pattern:
```rust
// In SettingsPatch struct:
pub autostop_silence_secs: Option<f32>,

// In impl Default:
autostop_silence_secs: None,

// In merge_settings:
autostop_silence_secs: patch.autostop_silence_secs.unwrap_or(existing.autostop_silence_secs),
```
Follow this exact pattern for `live_preview_enabled` and `preview_pause_silence_secs`.
The placeholder in `merge_settings` (lines ~317-319) currently passes `existing.*` directly;
replacing them with `patch.*.unwrap_or(existing.*)` is the entire change.

### SettingsView — `live_preview_enabled` and `preview_pause_silence_secs` NOT yet in it

`get_settings` in `settings.rs` builds a `SettingsView { ... }` literal. As of 5.1/5.2,
these two config fields are deliberately absent from `SettingsView` because the frontend
had no UI for them yet (comment in 5.1 says "Story 5.3 will add..."). This story adds them.
The `SettingsView` struct is in `src-tauri/src/lib.rs` starting at line 115.

### `save_settings` command parameter order — append at the end

The `save_settings` Tauri command has a fixed parameter order. New parameters MUST be
appended at the end (after `openrouter_api_key: Option<String>`), both in the Rust command
signature and in the TypeScript `invoke` call. Breaking the order would silently mis-map
arguments.

### TypeScript `MOCK_SETTINGS` in `tauri-commands.ts`

`MOCK_SETTINGS` around line 45 is a fallback object for preview mode. It must be updated
to include `livePreviewEnabled: false` and `previewPauseSilenceSecs: 2.0`, otherwise
TypeScript strict mode will complain about missing properties on `AppSettings`.

### Toggle UI pattern — mirror Auto-Paste in SettingsPanel

The existing toggle pattern from `SettingsPanel.tsx` lines ~577-588:
```tsx
<button
  role="switch"
  aria-checked={localAutoPaste}
  onClick={() => setLocalAutoPaste(!localAutoPaste)}
  className={[
    "relative flex-shrink-0 w-9 h-5 rounded-full transition-colors duration-200 focus:outline-none",
    localAutoPaste ? "bg-klarvo-primary/40" : "bg-klarvo-elevated",
  ].join(" ")}
>
  <span className={[
    "absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200",
    localAutoPaste ? "translate-x-4" : ""
  ].join(" ")} />
</button>
```
Apply the same pattern for the Live Preview toggle with `localLivePreviewEnabled`.

### Slider pattern — mirror Silence Duration in ShortcutsContent

The existing silence slider pattern (ShortcutsContent.tsx lines ~320-335):
```tsx
<div className="flex items-center justify-between">
  <span className={LABEL_CLS}>Silence Duration</span>
  <span className="text-xs font-mono text-klarvo-primary">{localSilenceSecs.toFixed(1)}s</span>
</div>
<input
  type="range"
  min={1.0}
  max={5.0}
  step={0.1}
  value={localSilenceSecs}
  onChange={(e) => setLocalSilenceSecs(parseFloat(e.target.value))}
  className="w-full accent-klarvo-primary"
/>
<p className="text-[11px] text-klarvo-muted">Seconds of silence before auto-stop</p>
```
Mirror this pattern for the Preview-Pause slider with `min={0.5}` (not 1.0 — shorter
pauses are valid for preview), `max={5.0}`, `step={0.1}`.

### isDirty tracking in SettingsPanel

`SettingsPanel.tsx` has a `useMemo` or `useEffect` (around lines 320-365) computing
`isDirty`. It compares `loadedSettings.*` to the local state values. Both new fields
must be included here, otherwise the Save button stays disabled even when the user changes
the toggle or slider.

### `ShortcutsContentProps` interface — adding props

`ShortcutsContentProps` is an exported interface in `ShortcutsContent.tsx`. Adding props
requires:
1. Adding them to the interface definition
2. Destructuring them in the `ShortcutsContent` function signature
3. Passing them from `SettingsPanel.tsx` in the JSX

This is the same pattern used for all existing props (e.g. `localSilenceSecs` was added
this way).

### Desktop-only (isDesktop guard) — AC-7

The entire Hotkey section in `ShortcutsContent.tsx` is already wrapped in `{isDesktop && (...)}`.
Adding the Live Preview section inside this block automatically ensures it never renders
on Android. No separate guard needed for the new controls.

### Inversion checks (L3 guard — Epic-4-retro AI-1)

The reviewer will mechanically invert these for the Rust test:
- **AC-5**: In `spec_live_preview_settings_patch_round_trip`, change `Some(true)` to
  `Some(false)` → the assert `assert!(result.live_preview_enabled)` should go RED.
- **AC-5**: Change `None` patch to `Some(false)` for the "preserve existing" case →
  the "existing value preserved" assert should go RED.

Document the inversion result in the Completion Notes so the reviewer can verify RED was
actually observed, NOT just claimed.

### Files to Modify

**Rust:**
- `src-tauri/src/commands/settings.rs` — `SettingsPatch`, `Default`, `merge_settings`,
  `save_settings` signature, new unit test
- `src-tauri/src/lib.rs` — `SettingsView` struct (2 new fields), `get_settings` return

**TypeScript/React:**
- `src/types.ts` — `AppSettings` (2 new fields)
- `src/tauri-commands.ts` — `saveSettings` (2 new params + `MOCK_SETTINGS`)
- `src/components/settings/ShortcutsContent.tsx` — new props + toggle + slider UI
- `src/components/SettingsPanel.tsx` — local state, dirty check, save call, props passthrough

**No Android changes.** `preview_pause_silence_secs` is desktop-only; Android continues
reading its existing silence keys unchanged (NFR3 / ADR-0016).

### References

- `src-tauri/src/commands/settings.rs:104-198` — `SettingsPatch` struct + `Default` impl
  (pattern to extend)
  [Source: src-tauri/src/commands/settings.rs#L104-L198]
- `src-tauri/src/commands/settings.rs:209-335` — `merge_settings` function; lines ~317-319
  are the placeholder for this story
  [Source: src-tauri/src/commands/settings.rs#L209-L335]
- `src-tauri/src/commands/settings.rs:352-514` — `save_settings` command signature +
  patch construction + `save_config_locked` call (ADR-0015 choke-point)
  [Source: src-tauri/src/commands/settings.rs#L352-L514]
- `src-tauri/src/lib.rs:115-199` — `SettingsView` struct definition
  [Source: src-tauri/src/lib.rs#L115-L199]
- `src-tauri/src/commands/settings.rs:517-589` — `get_settings` command
  [Source: src-tauri/src/commands/settings.rs#L517-L589]
- `src-tauri/src/config/mod.rs:705-712` — `live_preview_enabled` + `preview_pause_silence_secs`
  field definitions in `AppConfig` (serde defaults, from Story 5.1)
  [Source: src-tauri/src/config/mod.rs#L705-L712]
- `src/types.ts:29-79` — `AppSettings` interface (fields to extend)
  [Source: src/types.ts#L29-L79]
- `src/tauri-commands.ts:244-327` — `saveSettings` TypeScript function (signature + invoke
  call to extend)
  [Source: src/tauri-commands.ts#L244-L327]
- `src/components/settings/ShortcutsContent.tsx:159-213` — `ShortcutsContentProps` interface
  [Source: src/components/settings/ShortcutsContent.tsx#L159-L213]
- `src/components/settings/ShortcutsContent.tsx:317-337` — Silence Duration slider pattern
  to mirror for Preview-Pause slider
  [Source: src/components/settings/ShortcutsContent.tsx#L317-L337]
- `src/components/SettingsPanel.tsx:140-186` — local state initialization pattern
  [Source: src/components/SettingsPanel.tsx#L140-L186]
- `src/components/SettingsPanel.tsx:454-518` — `saveCurrentSettings` + `handleSave` flow
  [Source: src/components/SettingsPanel.tsx#L454-L518]
- `src/components/SettingsPanel.tsx:577-617` — toggle UI pattern (Auto-Paste, Auto-Send)
  [Source: src/components/SettingsPanel.tsx#L577-L617]
- `_bmad-output/planning-artifacts/epics-live-preview.md#Story 5.3`
  — authoritative ACs + FR/AR traceability (FR6, FR8, D3)
- `_bmad-output/implementation-artifacts/5-1-backend-pause-triggered-delta-flush-for-toggle-hold.md`
  — Story 5.1 AC-3 (AppConfig fields), AC-4 (flush uses preview_pause_silence_secs)
- `_bmad-output/project-context.md` — Windows release-build DoD requirement, camelCase
  config rule, ADR-0015 single-writer constraint, no second config writer

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

None.

### Completion Notes List

- Tasks 1.1–1.7 (Rust backend): Added `live_preview_enabled: Option<bool>` and `preview_pause_silence_secs: Option<f32>` to `SettingsPatch` struct + `Default` impl; replaced placeholder pass-through lines in `merge_settings` with `patch.*.unwrap_or(existing.*)` pattern; added both fields to `SettingsView` in `lib.rs` + populated them in `get_settings`; appended both params to `save_settings` command signature and `SettingsPatch` construction block.
- Task 1.6 — unit test `spec_live_preview_settings_patch_round_trip` written and verified:
  - `cargo test` PASS: 567 lib tests / 0 fail (previously 566 + 1 new).
  - INVERSION EMPIRICALLY VERIFIED:
    - Inversion 1: `Some(true)` → `Some(false)` → `assert!(result.live_preview_enabled)` went RED (FAILED). ✅
    - Inversion 2: `None` → `Some(false)` in "preserve" case → `assert!(result2.live_preview_enabled)` went RED (FAILED). ✅
    - Both inversions byte-restored before commit.
- Task 1.7 — clippy: no new warnings introduced in `settings.rs` or `lib.rs`; pre-existing project-wide warnings are unchanged.
- Also updated 3 existing `SettingsView` test literals in `lib.rs` to include the new fields (compile fix).
- Updated `test_merge_settings_happy_path_full_patch` to include `live_preview_enabled: Some(true), preview_pause_silence_secs: Some(1.0)` (compile fix).
- Tasks 2.1–2.3 (TS surface): Added `livePreviewEnabled: boolean` and `previewPauseSilenceSecs: number` to `AppSettings` in `types.ts`; updated `MOCK_SETTINGS`; extended `saveSettings` function in `tauri-commands.ts` with 2 new optional params + invoke call; `npm run build` PASS: 0 errors.
- Tasks 3.1–3.4 (UI): Added 4 new props to `ShortcutsContentProps` + destructuring; added "Live Preview" subsection inside the `isDesktop` block with toggle (same pattern as Auto-Paste) and conditional Preview-Pause slider (same pattern as Silence Duration slider) with trade-off hint (AC-4); updated `SettingsPanel.tsx` with 2 new local state vars, isDirty calculation, saveCurrentSettings call + dependency array; extended `onSave` prop type; propagated via JSX; extended `handleSaveSettings` in `useSettings.ts`; `npm run build` PASS: 0 errors.
- AC-7: Desktop-only guard satisfied — Live Preview section is placed inside the existing `{isDesktop && (...)}` block; no Android code changed.
- AC-3: Defaults satisfied by Story 5.1's serde defaults (`live_preview_enabled = false`, `preview_pause_silence_secs = 2.0`) — no migration write triggered.
- DoD items 4.4 (Windows release build) and 4.5 (manual smoke) are SURFACE-class and require Andy to run on Windows.

### File List

- `src-tauri/src/commands/settings.rs` (modified)
- `src-tauri/src/lib.rs` (modified)
- `src/types.ts` (modified)
- `src/tauri-commands.ts` (modified)
- `src/components/settings/ShortcutsContent.tsx` (modified)
- `src/components/SettingsPanel.tsx` (modified)
- `src/hooks/useSettings.ts` (modified)

### Change Log

- 2026-06-05: Story 5.3 implemented — Settings opt-in Preview toggle + Preview-Pause slider (Regler A). Rust: SettingsPatch + merge_settings + SettingsView + get_settings + save_settings. TS: AppSettings + saveSettings + MOCK_SETTINGS. UI: ShortcutsContent Live Preview section + SettingsPanel wiring. 567 Rust tests / 0 fail; npm build PASS; inversion checks RED verified.
- 2026-06-05: Addressed code review findings — 1 item resolved (Date: 2026-06-05). Added missing `setLocalLivePreviewEnabled` and `setLocalPreviewPauseSilenceSecs` to `loadedSettings` useEffect in SettingsPanel.tsx (fix for Save-button-stuck-dirty after non-round slider value). npm build PASS; 567 Rust tests / 0 fail.

### Review Findings

Code review 2026-06-05 (3 adversarial layers, Opus 4.8; conductor-run). AC-5 inversion empirically re-verified RED by the reviewer (flip `Some(true)`→`Some(false)` → `spec_live_preview_settings_patch_round_trip` FAILED; file byte-restored). 1 patch, 2 deferred, 7 dismissed as noise.

- [x] [Review][Patch] New preview fields missing from the `loadedSettings` re-sync `useEffect` → Save button stays stuck "dirty" after a successful save [src/components/SettingsPanel.tsx:248-288] — Every other persisted local is re-synced from `loadedSettings` in this effect (ends at `setLocalVoiceCommandEnabled`, :286); `localLivePreviewEnabled` and `localPreviewPauseSilenceSecs` are not. After save, `getSettings()`→`setLoadedSettings()` fires the effect; because `preview_pause_silence_secs` is an `f32`, serde widens it (e.g. slider `1.3` round-trips as `1.2999999523162842`), so the added dirty check `(loadedSettings?.previewPauseSilenceSecs ?? 2.0) !== localPreviewPauseSilenceSecs` stays `true` indefinitely — the panel reports unsaved changes forever after saving any non-round slider value. Also leaves the two locals stale if `loadedSettings` changes from another source while the panel is mounted. Fix: add `setLocalLivePreviewEnabled(loadedSettings.livePreviewEnabled ?? false)` and `setLocalPreviewPauseSilenceSecs(loadedSettings.previewPauseSilenceSecs ?? 2.0)` to the effect, mirroring the established pattern. Source: Edge Case Hunter (#1) + Blind Hunter. RESOLVED: Added both `setLocalLivePreviewEnabled` and `setLocalPreviewPauseSilenceSecs` to the `loadedSettings` useEffect at line 287-288, mirroring the established pattern. npm build PASS, 567 Rust tests / 0 fail.
- [x] [Review][Defer] No backend range validation on `preview_pause_silence_secs` in `merge_settings` [src-tauri/src/commands/settings.rs:322-323] — deferred, pre-existing pattern (no `*_silence_secs` field is backend-clamped; the UI constrains [0.5,5.0] and the downstream VAD path clamps `hangover_ms.max(200)` in `audio/mod.rs`, so a hand-edited config is contained, not dangerous).
- [x] [Review][Defer] Custom switch uses `focus:outline-none` with no replacement focus ring (a11y) [src/components/settings/ShortcutsContent.tsx] — deferred, repo-wide pattern (mirrors the sanctioned Auto-Paste/Auto-Send toggles); a11y focus-ring is a cross-cutting improvement for all custom switches, not this story.
