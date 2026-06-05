---
story: "5.5"
epic: "5"
title: "Settings — Preview display-form presets (Compact/Comfortable/Wide)"
status: review
track: L3-feature
gatedBy: ["5.3"]
buildsOn: ["5.3"]
inputDocuments:
  - _bmad-output/planning-artifacts/epics-live-preview.md
  - _bmad-output/project-context.md
  - _bmad-output/implementation-artifacts/5-3-settings-opt-in-preview-toggle-and-preview-pause-slider.md
  - _bmad-output/implementation-artifacts/5-2-frontend-auto-expand-preview-panel.md
---

# Story 5.5: Settings — Preview display-form presets (Compact/Comfortable/Wide)

Status: review

## Story

As a user,
I want to pick the live-preview's display form from a few curated presets (Compact / Comfortable / Wide),
so that I can size the read-along panel to my taste without fiddling with raw pixels.

## Acceptance Criteria

**AC-1 (New `AppConfig` field — additive, no migration — NFR2):**
Given `AppConfig` in `src-tauri/src/config/mod.rs`
When Story 5.5 ships
Then a new string field `preview_panel_form` (serde default `"comfortable"`) is added
And the default reproduces the **exact** shipped 5-2 look (width 320, screen-cap 320 — `comfortable` == today's shipped constants in `FloatingBar.tsx`)
And loading a `config.json` without this key reads `"comfortable"` and triggers **no migration write** (additive default — zero behavior change for existing users, NFR2)
And any unknown/garbage value (e.g. a hand-edited config) falls back to `"comfortable"` (fail-soft — a `match` with `_ =>` arm, no panic).

**AC-2 (Rust backend: `SettingsPatch`, `merge_settings`, `SettingsView`, `get_settings`, `save_settings` — same pattern as Story 5.3):**
Given `SettingsPatch` in `src-tauri/src/commands/settings.rs`
When Story 5.5 ships
Then `preview_panel_form: Option<String>` is added to `SettingsPatch` and its `Default` impl (= `None`)
And `merge_settings` uses `patch.preview_panel_form.unwrap_or(existing.preview_panel_form)` — identical pattern as all other Option fields
And `SettingsView` in `src-tauri/src/lib.rs` gains `pub preview_panel_form: String`
And `get_settings` populates it from `cfg.preview_panel_form`
And `save_settings` gains a new parameter `preview_panel_form: Option<String>` appended **after** `preview_pause_silence_secs` (the current last param) and includes it in the `SettingsPatch` construction block
And a unit test `spec_preview_panel_form_patch_round_trip` in `settings.rs` asserts:
  - writing `Some("compact".to_string())` round-trips to `"compact"` in merged config
  - writing `None` preserves the existing value
  - **INVERSION**: change `Some("compact")` to `Some("wide")` → the `assert_eq!(result, "compact")` goes RED (empirically verified — document result in Completion Notes)

**AC-3 (TypeScript surface — `AppSettings`, `saveSettings`, `MOCK_SETTINGS`):**
Given `AppSettings` in `src/types.ts` does not yet include `previewPanelForm`
When this story ships
Then `previewPanelForm: string` is added to `AppSettings` (camelCase matching serde `rename_all = "camelCase"`)
And `saveSettings` in `src/tauri-commands.ts` gains a new optional parameter `previewPanelForm?: string | null` appended after `previewPauseSilenceSecs`
And the `invoke` call includes `previewPanelForm: previewPanelForm ?? null`
And `MOCK_SETTINGS` in `tauri-commands.ts` is updated with `previewPanelForm: "comfortable"`
And `tsc` / `npm run build` passes with no new type errors.

**AC-4 (FloatingBar.tsx: replace hardcoded constants with a form→appearance map):**
Given `FloatingBar.tsx` currently hardcodes `PANEL_WIDTH = 320` and `PANEL_ABS_MAX = 320`
When Story 5.5 ships
Then those constants are replaced by a `FORM_MAP` object (or similar) mapping each preset to a `{ width: number; screenCap: number }` pair:
  - `compact`:     `{ width: 260, screenCap: 240 }` (illustrative — final values are left for dev judgment, but must be ≤ Comfortable)
  - `comfortable`: `{ width: 320, screenCap: 320 }` (**must** equal today's shipped values — zero visual change for default users, NFR2)
  - `wide`:        `{ width: 400, screenCap: 400 }` (illustrative — final values are left for dev judgment, but must be > Comfortable)
And the bar reads `previewPanelForm` from `getSettings` on mount **and** re-reads it on the settings-changed event path (so a changed preset applies to the **next** preview open without an app restart)
And every consumer of the old `PANEL_WIDTH` / `PANEL_ABS_MAX` constants (`activePillWidth`, `activePillHeight`, `panelHeight` computation in `useLayoutEffect`, the hidden measure probe `width`, and `setSize` calls) uses the resolved values from the map instead
And `comfortable` resolves to the same values as the old hardcoded constants (regression guard for NFR2).

**AC-5 (Settings UI — segmented/radio control inside the Live Preview section from Story 5.3):**
Given the Live Preview section in `ShortcutsContent.tsx` (added by Story 5.3), and `localLivePreviewEnabled` is `true`
When the user opens Settings → Shortcuts
Then a "Darstellung" (or "Display Form") labeled control shows the three presets — Compact / Comfortable / Wide — with the currently saved preset selected
And the control is conditionally rendered only when `localLivePreviewEnabled` is `true` (same guard as the Preview-Pause slider)
And selecting a preset updates local state `localPreviewPanelForm` which is included in the Save payload.

**AC-6 (SettingsPanel.tsx wiring):**
Given `SettingsPanel.tsx` manages local state for all Settings fields
When Story 5.5 ships
Then a new local state `localPreviewPanelForm` / `setLocalPreviewPanelForm` is initialized from `loadedSettings?.previewPanelForm ?? "comfortable"`
And it is included in the `isDirty` calculation: `(loadedSettings?.previewPanelForm ?? "comfortable") !== localPreviewPanelForm`
And it is added to both the `loadedSettings` re-sync `useEffect` (mirrors the 5.3 patch — **must not** omit this or the Save button stays stuck dirty) and the `saveCurrentSettings` call
And it is passed through to `ShortcutsContent` as `localPreviewPanelForm` / `setLocalPreviewPanelForm` props
And `useSettings.ts` (if it bridges `handleSaveSettings`) is updated to include the new param.

**AC-7 (FR4/FR6 boundary unchanged — Preview-off and Auto/AutoStop modes):**
Given Preview is disabled (`live_preview_enabled == false`), or the active mode is Auto/AutoStop (no preview feed there)
When any display form is selected
Then there is no visual or behavioral effect on the panel — the form only sizes the Toggle/Hold preview panel
And no Auto/AutoStop behavior is changed.

**DoD (Surface story):**
- **Windows release build** via `scripts/sync-and-build.ps1` (mandatory — Linux tests mask Tauri runtime + WebView2 rendering bugs; project-context.md testing rule).
- **Manual smoke**: cycle all three presets, confirm the panel renders at visibly different widths; confirm `comfortable` looks identical to the pre-5.5 shipped look; confirm the chosen preset persists across a Settings close→reopen (reads `config.json`) and an app relaunch; confirm `comfortable` is the default for a fresh config.
- `cargo test` on touched Rust files (round-trip test green, all lib tests green).
- `tsc` / `npm run build` passes — 0 TypeScript errors.
- `cargo clippy` clean on touched Rust files (no new warnings).
- Desktop-only — **no Android change** (preview is Groq-only desktop; NFR3 / ADR-0016).

## Tasks / Subtasks

### Task 1: AppConfig — add `preview_panel_form` field (AC-1)

- [x] 1.1 In `src-tauri/src/config/mod.rs`, locate the `preview_pause_silence_secs` field (around line 711) and add immediately after it:
  ```rust
  /// Display form preset for the live-preview panel (FR10/D8).
  /// Values: `"compact"` | `"comfortable"` | `"wide"`.  Unknown values fall
  /// back to `"comfortable"` in `FloatingBar.tsx` — the backend stores the raw
  /// string without validation so it is forward-compatible with future presets.
  /// Default: `"comfortable"` (reproduces the shipped 5-2 look, width=320/cap=320).
  #[serde(default = "default_preview_panel_form")]
  pub preview_panel_form: String,
  ```

- [x] 1.2 Add the default function near the other `default_*` preview functions (around line 915):
  ```rust
  fn default_preview_panel_form() -> String {
      "comfortable".to_string()
  }
  ```

- [x] 1.3 Add the field to the `AppConfig::default()` impl (around line 981) and any `AppConfig { .. }` struct literals in tests (search for `preview_pause_silence_secs:` and add `preview_panel_form: "comfortable".to_string()` immediately after each occurrence):
  ```rust
  preview_panel_form: default_preview_panel_form(),
  ```

- [x] 1.4 `cargo test` — all lib tests green (expected: ~567+). `cargo clippy` clean on `config/mod.rs`.

### Task 2: Rust backend — `SettingsPatch`, `merge_settings`, `SettingsView`, `get_settings`, `save_settings` (AC-2)

- [x] 2.1 In `src-tauri/src/commands/settings.rs`, add to the `SettingsPatch` struct (after `preview_pause_silence_secs: Option<f32>`):
  ```rust
  pub preview_panel_form: Option<String>,
  ```
  And in `impl Default for SettingsPatch`, add:
  ```rust
  preview_panel_form: None,
  ```

- [x] 2.2 In `merge_settings`, add after the `preview_pause_silence_secs` line (currently around line 322-323):
  ```rust
  preview_panel_form: patch.preview_panel_form
      .unwrap_or(existing.preview_panel_form),
  ```

- [x] 2.3 In `src-tauri/src/lib.rs`, add to `pub struct SettingsView` (after `preview_pause_silence_secs: f32`):
  ```rust
  /// Display form preset for the live-preview panel ("compact" | "comfortable" | "wide").
  pub preview_panel_form: String,
  ```

- [x] 2.4 In `get_settings` in `settings.rs`, add to the `SettingsView { ... }` literal:
  ```rust
  preview_panel_form: cfg.preview_panel_form.clone(),
  ```

- [x] 2.5 In the `save_settings` command signature, append **after** `preview_pause_silence_secs: Option<f32>`:
  ```rust
  preview_panel_form: Option<String>,
  ```
  And in the `SettingsPatch { ... }` construction block, add:
  ```rust
  preview_panel_form,
  ```

- [x] 2.6 Write unit test `spec_preview_panel_form_patch_round_trip` (inline `#[cfg(test)]` in `settings.rs`):
  ```rust
  #[test]
  fn spec_preview_panel_form_patch_round_trip() {
      // AC-2: Some("compact") round-trips to "compact"
      // AC-2: None preserves existing value ("comfortable" default)
      // INVERSION: change Some("compact") to Some("wide") → assert_eq fails → RED
      // Document the RED result in Completion Notes (never self-attest without proof).
  }
  ```
  **Empirically verify the inversion** (flip `Some("compact")` to `Some("wide")` → the `assert_eq!(result.preview_panel_form, "compact")` fails). Document the RED result in Completion Notes.

- [x] 2.7 Update any existing `SettingsView { .. }` or full `AppConfig { .. }` struct literals in `lib.rs` tests or other test files to add `preview_panel_form: "comfortable".to_string()` — required to keep them compiling.

- [x] 2.8 `cargo test` — all lib tests green. `cargo clippy` clean on `settings.rs` and `lib.rs` — no new warnings.

### Task 3: TypeScript surface — `AppSettings`, `saveSettings` (AC-3)

- [x] 3.1 In `src/types.ts`, add to `AppSettings` after `previewPauseSilenceSecs`:
  ```ts
  /** Display form preset for the live-preview panel. "compact" | "comfortable" | "wide". Desktop only. */
  previewPanelForm: string;
  ```

- [x] 3.2 In `src/tauri-commands.ts`:
  - Update `MOCK_SETTINGS` to add: `previewPanelForm: "comfortable",`
  - Add to `saveSettings` signature (after `previewPauseSilenceSecs?: number | null`):
    ```ts
    previewPanelForm?: string | null,
    ```
  - Add to the `invoke("save_settings", { ... })` call:
    ```ts
    previewPanelForm: previewPanelForm ?? null,
    ```

- [x] 3.3 `npm run build` — PASS: 0 TypeScript errors.

### Task 4: FloatingBar.tsx — form→appearance map (AC-4)

- [x] 4.1 Replace the two hardcoded constants with a form map. After the existing `SCREEN_TOP_MARGIN` constant:
  ```tsx
  /** Appearance lookup for each preview display-form preset (FR10/D8).
   *  "comfortable" MUST equal the old hardcoded PANEL_WIDTH/PANEL_ABS_MAX values
   *  so existing users see no visual change (NFR2). */
  const FORM_APPEARANCES: Record<string, { width: number; screenCap: number }> = {
    compact:     { width: 260, screenCap: 240 },
    comfortable: { width: 320, screenCap: 320 }, // shipped 5-2 look — do NOT change
    wide:        { width: 400, screenCap: 400 },
  };
  const DEFAULT_FORM = "comfortable";

  function getFormAppearance(form: string): { width: number; screenCap: number } {
    return FORM_APPEARANCES[form] ?? FORM_APPEARANCES[DEFAULT_FORM];
  }
  ```
  Remove the old `const PANEL_WIDTH = 320;` and `const PANEL_ABS_MAX = 320;` lines.

- [x] 4.2 Add `previewPanelForm` state (initialized from `getSettings` on mount):
  ```tsx
  const [previewPanelForm, setPreviewPanelForm] = useState<string>(DEFAULT_FORM);
  ```
  Update the existing `getSettings` mount effect (around line 285) to also read `s.previewPanelForm`:
  ```tsx
  useEffect(() => {
    getSettings()
      .then((s) => {
        setHotkeyMode(s.hotkeyMode);
        setPreviewPanelForm(s.previewPanelForm ?? DEFAULT_FORM);
      })
      .catch((e) => console.warn("[bar] getSettings failed (non-critical):", e));
  }, []);
  ```

- [x] 4.3 Derive the active appearance values from the form state:
  ```tsx
  const { width: PANEL_WIDTH, screenCap: PANEL_ABS_MAX } = getFormAppearance(previewPanelForm);
  ```
  Place this derivation at the component-body level, before the `activePillWidth` / `activePillHeight` computations, so all downstream consumers automatically use the resolved values.
  
  **Critical:** The hidden measure probe (around line 610) uses `width: PANEL_WIDTH - 2` — this must read the derived variable, not a removed constant. Verify this is the case.

- [x] 4.4 `npm run build` — 0 TypeScript errors. Verify that the `comfortable` preset still computes the same pixel values as before (spot-check: `getFormAppearance("comfortable").width === 320`).

### Task 5: Settings UI — preset picker in `ShortcutsContent.tsx` + `SettingsPanel.tsx` wiring (AC-5, AC-6)

- [x] 5.1 In `src/components/settings/ShortcutsContent.tsx`:
  - Add two new props to `ShortcutsContentProps`:
    ```ts
    localPreviewPanelForm: string;
    setLocalPreviewPanelForm: (v: string) => void;
    ```
  - Inside the `{localLivePreviewEnabled && (...)}` block (after the Preview-Pause slider, before the closing `</div>`), add the display-form picker:
    ```tsx
    {/* Display Form preset picker (AC-5) */}
    <div className="flex flex-col gap-1.5">
      <span className={LABEL_CLS}>Darstellung</span>
      <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
        {(["compact", "comfortable", "wide"] as const).map((preset) => (
          <button
            key={preset}
            onClick={() => setLocalPreviewPanelForm(preset)}
            className={[
              "flex-1 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
              localPreviewPanelForm === preset
                ? "bg-klarvo-primary/15 text-klarvo-primary"
                : "text-klarvo-dim hover:text-klarvo-muted",
            ].join(" ")}
          >
            {preset === "compact" ? "Compact" : preset === "comfortable" ? "Comfortable" : "Wide"}
          </button>
        ))}
      </div>
      <p className="text-[11px] text-klarvo-muted">
        Changes the width of the preview panel.
      </p>
    </div>
    ```
  - Destructure the two new props in the `ShortcutsContent` function signature.

- [x] 5.2 In `src/components/SettingsPanel.tsx`:
  - Add local state (after `localPreviewPauseSilenceSecs`):
    ```tsx
    const [localPreviewPanelForm, setLocalPreviewPanelForm] = useState(
      loadedSettings?.previewPanelForm ?? "comfortable"
    );
    ```
  - Add to the `loadedSettings` re-sync `useEffect` (after the `setLocalPreviewPauseSilenceSecs` line at ~288):
    ```tsx
    setLocalPreviewPanelForm(loadedSettings.previewPanelForm ?? "comfortable");
    ```
    **This is the 5.3-review-patch pattern — omitting it causes Save button to stay stuck dirty.**
  - Add to the `isDirty` calculation (after `localPreviewPauseSilenceSecs` line):
    ```ts
    || (loadedSettings?.previewPanelForm ?? "comfortable") !== localPreviewPanelForm
    ```
  - Add to the dependency arrays of both `isDirty` `useEffect` and `saveCurrentSettings` `useCallback`:
    ```ts
    localPreviewPanelForm,
    ```
  - Append to the `onSave(...)` call (after `localPreviewPauseSilenceSecs`):
    ```tsx
    localPreviewPanelForm,
    ```
  - Pass through to `ShortcutsContent` JSX:
    ```tsx
    localPreviewPanelForm={localPreviewPanelForm}
    setLocalPreviewPanelForm={setLocalPreviewPanelForm}
    ```

- [x] 5.3 In `src/hooks/useSettings.ts` (if it wraps `saveSettings` calls), add `previewPanelForm` to the bridge function — same tail-append pattern as 5.3 added `livePreviewEnabled`/`previewPauseSilenceSecs`.

- [x] 5.4 `npm run build` — 0 TypeScript errors.

### Task 6: Final validation (AC-1..AC-7, DoD)

- [x] 6.1 `cargo test` — all lib tests green (round-trip test + new AppConfig field compilations).
- [x] 6.2 `cargo clippy` on touched Rust files — no new warnings introduced.
- [x] 6.3 `npm run build` — PASS: 0 TypeScript errors.
- [ ] 6.4 Windows release build via `scripts/sync-and-build.ps1`.
- [ ] 6.5 Manual smoke:
  1. Open Settings → Shortcuts → confirm "Compact / Comfortable / Wide" picker visible when Live Preview is on.
  2. Select **Compact** → Save → check `config.json` for `"previewPanelForm": "compact"`.
  3. Dictate in Toggle with Preview on → confirm panel is narrower than before.
  4. Select **Comfortable** → Save → dictate → confirm panel looks exactly as it did before this story (pixel-identical to the shipped 5-2 look).
  5. Select **Wide** → Save → dictate → confirm panel is visibly wider.
  6. Close and relaunch app → confirm saved preset persists.
  7. Confirm `comfortable` is the default for a fresh/missing config key.

## Dev Notes

### camelCase config.json — Critical pitfall (carried from 5.2 and 5.3)

`AppConfig` uses `#[serde(rename_all = "camelCase")]`. The JSON key will be `"previewPanelForm"` (NOT `"preview_panel_form"`). Serde silently ignores the wrong-cased key — the field reads as the default `"comfortable"` with no error. When verifying `config.json` in the smoke test, use `"previewPanelForm"` exactly.

### Save-button stuck-dirty after save — Critical pattern from 5.3 review

The 5.3 code review caught that new float fields added to `SettingsPanel.tsx` but **not** added to the `loadedSettings` re-sync `useEffect` cause the Save button to stay stuck "dirty" after a successful save (because `f32→f64` serde widening means the round-tripped value ≠ the local state). For `preview_panel_form` (a String) the widening issue does not apply, but the re-sync pattern is still required for correctness: without it the local state goes stale if `loadedSettings` changes from another source while the panel is mounted. **Always add new local settings state to the `loadedSettings` useEffect** (pattern at `SettingsPanel.tsx:287-288`).

### SettingsPatch `Option` pattern — identical to all previous additions

Every optional field in `SettingsPatch` follows:
```rust
// Struct:
pub preview_panel_form: Option<String>,
// Default:
preview_panel_form: None,
// merge_settings:
preview_panel_form: patch.preview_panel_form.unwrap_or(existing.preview_panel_form),
```
This is the same pattern as `live_preview_enabled` and `preview_pause_silence_secs` in Story 5.3.

### `save_settings` parameter order — append at the end

The `save_settings` Tauri command has a positional parameter list. New params MUST go **after** `preview_pause_silence_secs: Option<f32>` (currently the last param), both in the Rust signature and the TypeScript `invoke` call. Breaking the order silently mis-maps arguments.

### FloatingBar: no window-width feedback loop — the hidden probe

`FloatingBar.tsx` uses a **hidden off-screen `div` (the "measure probe")** to measure text height at `PANEL_WIDTH - 2` (the bordered inner width) without triggering a resize feedback loop. After this story, the probe must use `PANEL_WIDTH - 2` where `PANEL_WIDTH` is the **resolved value from `getFormAppearance`**, NOT the removed constant. The `- 2` border subtraction still applies regardless of form (it accounts for the 1px CSS border on each side of the card). Missing this causes the last line of text to be clipped (the 5.2 "hidden-mess-sonde" lesson from MEMORY.md).

### FloatingBar: `previewPanelForm` MUST apply on the next preview open without an app restart (AC-4)

**CORRECTED 2026-06-05 (code-review GATE, conductor-verified):** an earlier draft of this note
claimed "mount-load is sufficient (a Settings panel re-open re-mounts the bar)". That is **false** and
contradicts AC-4. The bar is a **separate Tauri overlay window** (`main.tsx` mounts `FloatingBar` as the
window-`"bar"` Root; created once at startup `lib.rs` `create_bar_window`). `SettingsPanel` lives in the
**main** window (`App.tsx`). Re-opening the Settings panel is a view-toggle *inside the main window* — it
does **not** re-mount the separate bar window. A mount-only `getSettings()` therefore freezes
`previewPanelForm` at app-startup value, so a saved preset stays inert until a full app **restart** — an
AC-4 violation.

**Required:** the bar must pick up a newly-saved preset on the **next preview open without an app restart.**
Implement a reactive update using the bar's established pattern (it already listens to backend events such
as `klarvo://active-mode`). Either approach satisfies AC-4 — pick the idiomatic one:
  - **(a) Re-read on panel-open:** when the preview panel transitions closed→open, call `getSettings()` and
    refresh `previewPanelForm` before the panel sizes. Frontend-only, directly delivers "next preview open".
  - **(b) Backend settings-changed event:** `save_settings` emits an event to the `"bar"` window
    (`get_webview_window("bar")`, precedent in `misc.rs`); `FloatingBar` listens and updates
    `previewPanelForm`. Matches AC-4's literal "settings-changed event path" wording.
Whichever is chosen, ensure the resolved `PANEL_WIDTH` is reflected when the panel next sizes (the measure
`useLayoutEffect` currently keys on `PANEL_ABS_MAX` only — add `PANEL_WIDTH` to its deps so a width change
that doesn't change the screen-cap still re-measures).

### AppConfig struct literal completeness in tests

`AppConfig` is constructed with all fields in multiple test fixtures in `config/mod.rs` (search for `preview_pause_silence_secs:` — each occurrence that has the full `AppConfig { ... }` literal needs `preview_panel_form: "comfortable".to_string()` added immediately after it). There are also `SettingsView { ... }` literals in `lib.rs` tests that need `preview_panel_form: "comfortable".to_string()`. Compile errors from missing fields will guide discovery.

### Desktop-only — no Android change

`preview_panel_form` is desktop-only. The preview feature itself is Groq-only desktop (Toggle/Hold). Android never shows a preview panel. No Android Kotlin code or `KlarvoOverlayService.kt` change is required (NFR3 / ADR-0016).

### Inversion checks (L3 guard — Epic-4-retro AI-1)

The reviewer will mechanically invert the Rust test:
- In `spec_preview_panel_form_patch_round_trip`, change `Some("compact".to_string())` to `Some("wide".to_string())` → `assert_eq!(result.preview_panel_form, "compact")` must go **RED**.
- Document the RED result in Completion Notes. Do NOT self-attest without having run the inversion.

### Files to Modify

**Rust:**
- `src-tauri/src/config/mod.rs` — new `preview_panel_form` field + `default_preview_panel_form()` fn + `AppConfig::default()` + test literals
- `src-tauri/src/commands/settings.rs` — `SettingsPatch` + `Default` + `merge_settings` + `save_settings` signature + new unit test + `SettingsView`-test literals
- `src-tauri/src/lib.rs` — `SettingsView` struct (1 new field) + `get_settings` return + test literals

**TypeScript/React:**
- `src/types.ts` — `AppSettings` (1 new field)
- `src/tauri-commands.ts` — `saveSettings` (1 new param + `MOCK_SETTINGS`)
- `src/FloatingBar.tsx` — replace `PANEL_WIDTH`/`PANEL_ABS_MAX` constants with `FORM_APPEARANCES` map + `previewPanelForm` state + mount effect
- `src/components/settings/ShortcutsContent.tsx` — 2 new props + preset picker UI
- `src/components/SettingsPanel.tsx` — local state + dirty check + re-sync effect + save call + deps arrays + props passthrough
- `src/hooks/useSettings.ts` — if it bridges `saveSettings`, add `previewPanelForm` param

**No Android changes.**

### References

- `src-tauri/src/config/mod.rs:701-712` — `live_preview_enabled` + `preview_pause_silence_secs` fields (pattern to mirror for `preview_panel_form`)
- `src-tauri/src/commands/settings.rs:104-155` — `SettingsPatch` struct (extend after `preview_pause_silence_secs`)
- `src-tauri/src/commands/settings.rs:157-200` — `impl Default for SettingsPatch`
- `src-tauri/src/commands/settings.rs:209-338` — `merge_settings`; current last preview line ~322-323
- `src-tauri/src/commands/settings.rs:354-523` — `save_settings` (append new param at end)
- `src-tauri/src/lib.rs:115-203` — `SettingsView` struct (extend after `preview_pause_silence_secs: f32` at line 202)
- `src-tauri/src/commands/settings.rs:517-589` — `get_settings` (add `preview_panel_form`)
- `src/FloatingBar.tsx:37-43` — `PANEL_WIDTH`/`PANEL_ABS_MAX` constants to replace
- `src/FloatingBar.tsx:283-288` — `getSettings` mount effect to extend
- `src/FloatingBar.tsx:337-342` — `useLayoutEffect` for panel height: uses `PANEL_ABS_MAX` (will use derived value after 4.3)
- `src/FloatingBar.tsx:355-359` — `activePillWidth`/`activePillHeight`: use `PANEL_WIDTH` (will use derived)
- `src/FloatingBar.tsx:604-622` — hidden measure probe: uses `PANEL_WIDTH - 2` (must use derived, keep the `-2`)
- `src/types.ts:79-82` — `livePreviewEnabled` + `previewPauseSilenceSecs` fields (add `previewPanelForm` after)
- `src/tauri-commands.ts:84-85` — `livePreviewEnabled`/`previewPauseSilenceSecs` in `MOCK_SETTINGS` (add `previewPanelForm` after)
- `src/tauri-commands.ts:285-333` — `saveSettings` function (append `previewPanelForm` param + invoke)
- `src/components/settings/ShortcutsContent.tsx:213-236` — `ShortcutsContentProps` + destructuring (extend)
- `src/components/settings/ShortcutsContent.tsx:454-471` — existing Live Preview section (add picker inside the `localLivePreviewEnabled &&` block)
- `src/components/SettingsPanel.tsx:287-288` — `loadedSettings` re-sync useEffect (add `setLocalPreviewPanelForm`)
- `src/components/SettingsPanel.tsx:360-373` — `isDirty` useEffect (add `previewPanelForm` line + dep)
- `src/components/SettingsPanel.tsx:481-531` — `saveCurrentSettings` useCallback (add param + dep)
- `_bmad-output/planning-artifacts/epics-live-preview.md#Story 5.5` — authoritative ACs + FR10/D8 traceability
- `_bmad-output/implementation-artifacts/5-3-settings-opt-in-preview-toggle-and-preview-pause-slider.md` — pattern for Rust SettingsPatch + SettingsView + TS wiring; review-patch for loadedSettings re-sync
- `_bmad-output/project-context.md` — Windows release-build DoD requirement, camelCase config rule, ADR-0015 single-writer

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

None.

### Completion Notes List

- Implemented all AC-1..AC-6 + AC-7 (pass-through; no Auto/AutoStop behaviour changed).
- `preview_panel_form` field added to `AppConfig` with `#[serde(default = "default_preview_panel_form")]` returning `"comfortable"` — additive, no migration write fires (verified by updated `spec_live_preview_config_fields_default`).
- `SettingsPatch` / `merge_settings` / `SettingsView` / `get_settings` / `save_settings` all extended following the exact 5.3 `Option<String>` pattern.
- `spec_preview_panel_form_patch_round_trip` written and passes: `Some("compact")` → `"compact"`, `None` → preserves `"comfortable"`.
- **INVERSION EMPIRICALLY VERIFIED** (Epic-4-retro AI-1 — reviewer control): changing `Some("compact".to_string())` to `Some("wide".to_string())` in the test → `assert_eq!(result.preview_panel_form, "compact")` → FAILED (left: "wide", right: "compact") — test went RED. Restored to `"compact"`, test GREEN. Proof is in the terminal output, NOT self-attested prose.
- TypeScript surface: `AppSettings.previewPanelForm: string` added; `MOCK_SETTINGS` updated; `saveSettings` parameter appended; `invoke` call includes `previewPanelForm: previewPanelForm ?? null`.
- `FloatingBar.tsx`: `PANEL_WIDTH`/`PANEL_ABS_MAX` constants removed. `FORM_APPEARANCES` map introduced (`compact: 260/240`, `comfortable: 320/320` — unchanged from shipped look, `wide: 400/400`). Derived `PANEL_WIDTH`/`PANEL_ABS_MAX` placed BEFORE the `useLayoutEffect` in source order (closure runs after render, values are bound). `PANEL_ABS_MAX` added to `useLayoutEffect` dependency array so height re-clamps when form changes. Hidden probe `width: PANEL_WIDTH - 2` still uses derived `PANEL_WIDTH`.
- **AC-4 CORRECTED (2026-06-05 code-review directive):** The original implementation used a mount-only `getSettings()` load, which freezes `previewPanelForm` at bar-startup value (bar is a separate Tauri window, never re-mounts when Settings panel toggles). Fixed by adding a `useEffect` that detects the `isPanelOpen` false→true transition and calls `getSettings()` to refresh `previewPanelForm` before the panel sizes. This ensures that a preset saved in Settings applies to the **next preview open without an app restart** (AC-4 literal requirement).
- **`PANEL_WIDTH` added to `useLayoutEffect` deps (2026-06-05 code-review directive):** The `useLayoutEffect` previously listed only `PANEL_ABS_MAX`; a form change that alters width without altering screenCap would not have triggered re-measurement. `PANEL_WIDTH` now in deps so any form change re-measures.
- `ShortcutsContent.tsx`: two new props (`localPreviewPanelForm`, `setLocalPreviewPanelForm`) added to interface + destructuring + picker UI (segmented buttons: Compact/Comfortable/Wide) inside `localLivePreviewEnabled &&` block, after the Preview-Pause slider.
- `SettingsPanel.tsx`: `localPreviewPanelForm` local state added; `loadedSettings` re-sync `useEffect` updated (the 5.3-review-patch pattern — prevents stuck-dirty bug); `isDirty` + `saveCurrentSettings` deps arrays extended; `onSave(...)` call extended; `ShortcutsContent` JSX props passed through; `onSave` prop type updated.
- `useSettings.ts` `handleSaveSettings` bridge extended with `newPreviewPanelForm?: string | null`.
- `cargo test --lib`: **568 passed; 0 failed**.
- `cargo clippy`: no new errors on touched files.
- `npm run build` (tsc + vite): **0 TypeScript errors**, build SUCCESS.
- Tasks 6.4 (Windows release build) and 6.5 (manual smoke) are DoD gates that require Andy's Windows machine — marked open for review.

### File List

- `src-tauri/src/config/mod.rs`
- `src-tauri/src/commands/settings.rs`
- `src-tauri/src/lib.rs`
- `src/types.ts`
- `src/tauri-commands.ts`
- `src/FloatingBar.tsx`
- `src/components/settings/ShortcutsContent.tsx`
- `src/components/SettingsPanel.tsx`
- `src/hooks/useSettings.ts`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/5-5-settings-preview-display-form-presets.md`

### Change Log

- 2026-06-05: Story 5.5 implemented — `preview_panel_form` (compact/comfortable/wide) added to AppConfig, SettingsPatch, merge_settings, SettingsView, save_settings; FloatingBar replaces hardcoded 320/320 constants with FORM_APPEARANCES map; Settings UI adds picker under Live Preview toggle; 568 Rust tests pass, 0 TS errors. Status: review.
- 2026-06-05: AC-4 reactive-update fix applied (code-review directive) — added `isPanelOpen` closed→open refresh of `previewPanelForm` via `getSettings()` in FloatingBar.tsx; added `PANEL_WIDTH` to `useLayoutEffect` deps. 568 Rust tests still green, 0 TS errors. Status: review.
- 2026-06-05: 1st Windows smoke (Andy) found the preview panel clipped at the TOP on the FIRST chunk, self-correcting on the next chunk. Root cause (pre-existing from the 5-2 grow-upward logic, not 5-5-specific, but blocking the 5-5 smoke): the window-resize `useEffect` fires once in the pre-measure render (`panelHeight === 0`, sizes the window to pill height while the panel is already in the DOM) and once after measurement; both are async Tauri IPC sequences, and the stale pre-measure one can land last, leaving the window too short — the wrapper is `justify:flex-end` + `overflow:hidden`, so a too-short window clips the panel's TOP until the next chunk's resize. Fix (round 1): guard the resize effect to skip the pre-measure transient (`if (isPanelOpen && panelHeight === 0) return;`) so only the correct measured resize is applied. `npm run build` green (tsc + vite). Status: review (re-smoke owed on Windows).
- 2026-06-05: 2nd Windows smoke (Andy) — round-1 fixed **comfortable** but **wide** still clipped on the first-ever chunk per app launch (heals on the next chunk, never recurs). Sharper root cause: the FIRST pill→panel expansion after a launch is "cold" — the OS window under-applies the height of that first async `setSize`, worse for the bigger 200→400 (Wide) jump, leaving the window shorter than `PILL_HEIGHT + panelHeight` → top-clip (probe measures at a FIXED `PANEL_WIDTH-2` while the real `#preview-panel` fills the live window width). Fix (round 2): added `geomTick` — a state bumped on the next animation frame and again ~120ms after the panel opens, included in the resize effect's deps, so the resize (`setSize`/`setBarShape`/`setPosition`) re-fires and the window converges to the correct geometry without waiting for another chunk. Idempotent for warm opens. `npm run build` green (tsc + vite). Status: review (re-smoke owed on Windows).
