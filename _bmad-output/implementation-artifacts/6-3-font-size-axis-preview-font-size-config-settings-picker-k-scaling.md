---
story: "6.3"
epic: "6"
title: "Font-size axis — previewFontSize config + Settings picker + k-scaling (Increment B)"
status: ready-for-dev
track: L3-feature
gatedBy: ["6.2", "6.6"]
buildsOn: ["6.2", "6.6"]
enabledBy: []
inputDocuments:
  - _bmad-output/planning-artifacts/epics-bar-redesign.md
  - docs/bar-redesign-spec.md
  - docs/surface-smoke-checklist.md
  - _bmad-output/project-context.md
---

# Story 6.3: Font-size axis — previewFontSize config + Settings picker + k-scaling (Increment B)

Status: review

## Context

This story is **Increment B** of the unified "Vorschau-Darstellung" appearance panel that Story 6.6
(Increment A) built. 6.6 added the themes, color pickers, font-family dropdown, and live in-panel
preview card inside `ShortcutsContent.tsx`. This story adds the **font-SIZE control** into that same
panel — a 3-way Klein / Mittel / Groß picker — and wires it through config + `previewGeometry` so
the preview box scales proportionally.

**Why sequenced after 6.6 smoke:** font-SIZE is geometry/k-coupled (it scales the window's physical
width + height), which is a higher risk class than appearance (pure-CSS). Landing it on a stable,
smoke-verified appearance panel avoids geometry regressions inside an actively-changing component.

## Story

As a user,
I want to choose among three preview font sizes (Klein / Mittel / Groß) in Settings,
so that the preview box is readable at my preferred size with the whole box scaling proportionally.

## Acceptance Criteria

**AC-1 — Config field + serde default (camelCase, no migration):**
Given a fresh install or an existing config.json without the `previewFontSize` key
When the app loads the config
Then `preview_font_size` deserializes with serde default `"small"`, camelCase JSON key
`previewFontSize` (NOT `preview_font_size`) — mirrors every other preview config field.
No migration write fires for this field.
And inversion: add a `serde(rename_all = "camelCase")` parent-level test that checks
`previewFontSize` is present in the serialized JSON → RED if the field uses snake-case on disk.

**AC-2 — SettingsPatch + merge + save command wire-up:**
Given a valid `previewFontSize` value (`"small"` | `"medium"` | `"large"`)
When `save_settings` is called with a `preview_font_size` param
Then `SettingsPatch.preview_font_size` is applied via `merge_settings` and written via
`save_config_locked` (ADR-0015); the value round-trips through `get_settings` → `AppSettings`.

**AC-3 — TypeScript type + MOCK_SETTINGS + saveSettings wrapper:**
Given the frontend TS layer
When `AppSettings` is extended with `previewFontSize: string`
Then `MOCK_SETTINGS` includes `previewFontSize: "small"`, `saveSettings` accepts a new optional
`previewFontSize?: string | null` param at the end of its signature, and `handleSaveSettings`
in `useSettings.ts` forwards the param through the chain.
And inversion: omit the field from `MOCK_SETTINGS` → `getSettings()` in preview mode returns an
object without `previewFontSize` → `PreviewPanel.runShowSequence` fallback to `"small"` → but
the Settings picker seeds with `undefined` → picker shows no selection → RED.

**AC-4 — Settings picker (Klein / Mittel / Groß) in the appearance panel:**
Given the appearance sub-section in `ShortcutsContent.tsx` (built by 6.6)
When the user picks Klein / Mittel / Groß
Then a 3-button segmented control (identical style to the existing "Compact / Comfortable / Wide"
width-preset control) sets `localPreviewFontSize` and marks the form dirty. The live preview card
immediately reflects the new font size (font-size CSS updated inline in the live card).
The picker is placed ABOVE the width-preset picker but BELOW the "Appearance" heading and live card
(font-size affects card geometry, so group with display controls, not with color pickers).

**AC-5 — PreviewPanel reads previewFontSize reactively and scales geometry:**
Given a font size was saved and the preview opens
When `runShowSequence` fires (already reads `getSettings()` reactively — Trap #3)
Then `widthPreset` AND `fontSize` are both read from the same `getSettings()` call;
`previewGeometry(widthPreset, fontSize)` is called (replaces the hardcoded `"small"` at line 152);
width and maxHeight scale by `k = fontPx / 11`:
  - Small:  k=1.00 → width 260/320/400, maxHeight 600
  - Medium: k≈1.18 → width ~307/378/473, maxHeight ~709
  - Large:  k≈1.36 → width ~354/436/545, maxHeight ~818
And the `fontPx` from `previewGeometry` is applied as `fontSize: geom.fontPx` in the card's
inline style (replaces the hardcoded `fontSize: 11` at PreviewPanel.tsx line ~367).
And inversion: hard-code `fontSize="small"` (leave `previewGeometry(widthPreset, "small")`) →
Medium/Large show the same box size as Small → RED.

**AC-6 — SettingsPanel state wiring (dirty + resync + saveCurrentSettings):**
Given the SettingsPanel.tsx component
When `previewFontSize` is added
Then:
- `localPreviewFontSize` state is initialized from `loadedSettings?.previewFontSize ?? "small"`.
- It appears in the `loadedSettings` resync `useEffect` (Trap #2 — without this, Save stays dirty
  forever: after save, loadedSettings.previewFontSize is updated but local state is not synced back).
- It appears in the `isDirty` computation.
- `saveCurrentSettings` passes it to `onSave(...)`.
- `onSave` (SettingsPanelProps) accepts `previewFontSize?: string | null`.
And inversion: omit from resync `useEffect` → change font size → Save → button stays highlighted
(dirty forever because local state diverges from the just-saved loadedSettings value) → RED.

**AC-7 — ShortcutsContent props interface:**
Given `ShortcutsContent.tsx`
When `localPreviewFontSize` and `setLocalPreviewFontSize` are added to `ShortcutsContentProps`
Then the component receives and uses them; the SettingsPanel renders the picker correctly.

**AC-8 — Smoke: pick a size → config.json → preview reflects it on next open:**
Given a real Windows build (dev HMR or release via sync-and-build.ps1)
When the smoke is run
Then:
1. Open Settings → Shortcuts → preview section. Klein/Mittel/Groß picker is visible.
2. Pick Groß → live preview card in Settings shows larger text.
3. Save → `config.json` at `%APPDATA%\com.klarvo.voice\config.json` shows `"previewFontSize":"large"`.
4. Trigger a recording with preview on → preview box opens noticeably wider/taller (k≈1.36).
5. Switch back to Klein → Save → preview box opens at the original compact size.
6. Pill never resizes (baseline DoD from 6.2).
And inversion for AC-5: while testing, the dev can temporarily hard-code `fontSize="small"` in
`previewGeometry(widthPreset, "small")` → verify Medium/Large box size equals Small → RED.

## Tasks / Subtasks

- [x] **Task 1 — Rust: `preview_font_size` field in `AppConfig`** (AC-1)
  - [x] 1.1 In `src-tauri/src/config/mod.rs`, inside the `AppConfig` struct, add after `preview_font_family`:
    ```rust
    /// Preview font size: "small" | "medium" | "large". Default = "small".
    /// camelCase JSON key = "previewFontSize".
    /// MUST NOT trigger a migration write (serde default is sufficient).
    #[serde(default = "default_preview_font_size")]
    pub preview_font_size: String,
    ```
  - [x] 1.2 Add the default function (near the other `default_preview_*` functions, ~line 985):
    ```rust
    fn default_preview_font_size() -> String {
        "small".to_string()
    }
    ```
  - [x] 1.3 Add the field to `AppConfig::default()` (the impl block with all preview fields, ~line 1060):
    ```rust
    preview_font_size: default_preview_font_size(),
    ```
  - [x] 1.4 Add a spec test (pattern: copy `spec_preview_appearance_config_fields_default` structure):
    ```rust
    #[test]
    fn spec_preview_font_size_config_field_default() {
        let default_cfg = AppConfig::default();
        let mut json: serde_json::Value = serde_json::to_value(&default_cfg).unwrap();
        // AC-1: camelCase key present in serialized output.
        assert!(
            json.as_object().unwrap().contains_key("previewFontSize"),
            "expected camelCase key 'previewFontSize' (NOT 'preview_font_size')"
        );
        // Strip the key to simulate a pre-6.3 config.json.
        json.as_object_mut().unwrap().remove("previewFontSize");
        let stripped = serde_json::to_string(&json).unwrap();
        // Must deserialize without error and fill in the default.
        let result: Result<AppConfig, _> = serde_json::from_str(&stripped);
        assert!(result.is_ok(), "Deserializing without previewFontSize must succeed");
        let cfg = result.unwrap();
        assert_eq!(cfg.preview_font_size, "small", "default must be 'small'");
        // No migration write must fire.
        let mut warnings: Vec<String> = Vec::new();
        let (_, writes) = migrate_and_normalize(cfg, &std::path::PathBuf::from("/tmp"), &mut warnings);
        assert!(writes.is_empty(), "No migration write for preview_font_size");
    }
    ```

- [x] **Task 2 — Rust: `SettingsPatch` + `merge_settings` + `save_settings` command** (AC-2)
  - [x] 2.1 In `src-tauri/src/commands/settings.rs`, add to `SettingsPatch` struct (after `preview_font_family`):
    ```rust
    pub preview_font_size: Option<String>,
    ```
  - [x] 2.2 In the `Default` impl for `SettingsPatch` (the `..Default::default()` block), ensure `preview_font_size: None` is present (the `Default` derive should handle it automatically if all fields use `Option`).
  - [x] 2.3 In `merge_settings`, add after the `preview_font_family` merge:
    ```rust
    preview_font_size: patch.preview_font_size
        .unwrap_or(existing.preview_font_size),
    ```
  - [x] 2.4 In the `save_settings` Tauri command signature (the `#[tauri::command]` fn), add `preview_font_size: Option<String>` after `preview_font_family: Option<String>`.
  - [x] 2.5 In the `save_settings` body, build the patch with `preview_font_size`.
  - [x] 2.6 In the `get_settings` response struct (the `SettingsView` or equivalent), add `preview_font_size: cfg.preview_font_size.clone()` mirroring the other preview fields.
  - [x] 2.7 Find all locations in settings.rs that construct a full `AppConfig` literal for tests (grep for `preview_font_family:`) — add `preview_font_size: default_preview_font_size()` (or the test value) to each. (Note: test fixtures using `..SettingsPatch::default()` / `..AppConfig::default()` picked up the field automatically; the one explicit `SettingsPatch` literal in `test_merge_settings_happy_path_full_patch` was updated with `preview_font_size: None`; the three `SettingsView` literals in lib.rs tests were updated with `preview_font_size: "small".to_string()`; the two `AppConfig` literals in config/mod.rs tests were updated.)

- [x] **Task 3 — TypeScript: `AppSettings`, `MOCK_SETTINGS`, `saveSettings`** (AC-3)
  - [x] 3.1 In `src/types.ts`, inside `AppSettings`, add after `previewFontFamily`:
    ```ts
    /** Preview font size: "small" | "medium" | "large". camelCase key: previewFontSize. */
    previewFontSize: string;
    ```
  - [x] 3.2 In `src/tauri-commands.ts`, add to `MOCK_SETTINGS` (after `previewFontFamily`):
    ```ts
    previewFontSize: "small",
    ```
  - [x] 3.3 In `saveSettings` function signature, add as the last optional param (after `previewFontFamily`):
    ```ts
    previewFontSize?: string | null,
    ```
  - [x] 3.4 In the `invoke("save_settings", {...})` body, add:
    ```ts
    previewFontSize: previewFontSize ?? null,
    ```

- [x] **Task 4 — `useSettings.ts`: forward param through `handleSaveSettings`** (AC-3, AC-6)
  - [x] 4.1 In `src/hooks/useSettings.ts`, add `newPreviewFontSize?: string | null` after `newPreviewFontFamily` in `handleSaveSettings`'s parameter list.
  - [x] 4.2 Forward it in the `saveSettings(...)` call: add `newPreviewFontSize ?? null` after `newPreviewFontFamily ?? null`.

- [x] **Task 5 — `SettingsPanel.tsx`: state, resync, dirty, save, props** (AC-6)
  - [x] 5.1 Add state declaration (after `localPreviewFontFamily`):
    ```tsx
    const [localPreviewFontSize, setLocalPreviewFontSize] = useState(
      loadedSettings?.previewFontSize ?? "small"
    );
    ```
  - [x] 5.2 Add to the `loadedSettings` resync `useEffect` (after `setLocalPreviewFontFamily(...)`):
    ```tsx
    setLocalPreviewFontSize(loadedSettings.previewFontSize ?? "small");
    ```
    **CRITICAL — Trap #2:** omitting this line → after Save, `loadedSettings.previewFontSize`
    is updated but `localPreviewFontSize` still holds the old value → isDirty stays true
    → Save button stays highlighted forever. This is the most common new-field mistake.
  - [x] 5.3 Add to the `isDirty` computation (after the `localPreviewFontFamily` line):
    ```tsx
    || (loadedSettings?.previewFontSize ?? "small") !== localPreviewFontSize
    ```
  - [x] 5.4 Add `localPreviewFontSize` to the `isDirty` `useEffect` deps array.
  - [x] 5.5 Add `localPreviewFontSize` to `saveCurrentSettings`'s `onSave(...)` call (after `localPreviewFontFamily`).
  - [x] 5.6 Add `localPreviewFontSize` to `saveCurrentSettings`'s `useCallback` deps array.
  - [x] 5.7 In `SettingsPanelProps.onSave` signature, add `previewFontSize?: string | null` after `previewFontFamily?: string | null`.
  - [x] 5.8 Pass `localPreviewFontSize` and `setLocalPreviewFontSize` to `ShortcutsContent` (wherever the other `localPreview*` props are passed).

- [x] **Task 6 — `ShortcutsContent.tsx`: props + picker UI** (AC-4, AC-7)
  - [x] 6.1 Add to `ShortcutsContentProps` interface (after `localPreviewFontFamily`/`setLocalPreviewFontFamily`):
    ```tsx
    localPreviewFontSize: string;
    setLocalPreviewFontSize: (v: string) => void;
    ```
  - [x] 6.2 Destructure in the function signature.
  - [x] 6.3 In the appearance sub-section, add the font-size picker **above the width-preset picker**
    (between the "Appearance" heading block and the "Darstellung" / width picker). Match the
    existing segmented-button style exactly (same `className` pattern as Compact/Comfortable/Wide):
    ```tsx
    {/* Font-size picker (Story 6.3 Increment B).
        Klein/Mittel/Groß maps to small/medium/large.
        Affects card geometry (k-scaling) — placed with display controls. */}
    <div className="flex flex-col gap-1.5">
      <span className={LABEL_CLS}>Schriftgröße</span>
      <div className="flex gap-0.5 bg-klarvo-bg rounded-lg p-0.5 border border-klarvo-border/60">
        {(["small", "medium", "large"] as const).map((size) => (
          <button
            key={size}
            onClick={() => setLocalPreviewFontSize(size)}
            className={[
              "flex-1 py-1 rounded-md text-xs font-medium transition-all duration-100 whitespace-nowrap",
              localPreviewFontSize === size
                ? "bg-klarvo-primary/15 text-klarvo-primary"
                : "text-klarvo-dim hover:text-klarvo-muted",
            ].join(" ")}
          >
            {size === "small" ? "Klein" : size === "medium" ? "Mittel" : "Groß"}
          </button>
        ))}
      </div>
      <p className="text-[11px] text-klarvo-muted">
        Skaliert Breite, Höhe und Schrift der Vorschau proportional.
      </p>
    </div>
    ```
  - [x] 6.4 Update the **live preview card** (Task 5 in 6.6, the sample card at the top of the
    appearance sub-section): add `fontSize` to its inline style driven from `localPreviewFontSize`:
    ```tsx
    const FONT_PX_MAP: Record<string, number> = { small: 11, medium: 13, large: 15 };
    // In the live card's style prop:
    fontSize: FONT_PX_MAP[localPreviewFontSize] ?? 11,
    ```
    This ensures the live card immediately reflects the picked size before Save.
    Note: also moved Darstellung (width-preset) picker INSIDE the Appearance section (after font-size picker, as item 6 per Dev Notes ordering).

- [x] **Task 7 — `PreviewPanel.tsx`: read fontSize reactively, pass to previewGeometry, apply fontPx** (AC-5)
  - [x] 7.1 In `runShowSequence`, extend the `getSettings()` read block to also extract `previewFontSize`:
    ```ts
    // After: widthPreset = s.previewPanelForm ?? "comfortable";
    const fontSize = (s.previewFontSize || "small") as "small" | "medium" | "large";
    ```
  - [x] 7.2 Replace the hardcoded `previewGeometry(widthPreset, "small")` at line ~152 with:
    ```ts
    const geom = previewGeometry(widthPreset, fontSize);
    ```
  - [x] 7.3 Replace the hardcoded `fontSize: 11` in the card's inline style (~line 367) with:
    ```ts
    fontSize: cardFontPx,
    ```
    (`cardFontPx` is a new React state initialized to `FONT_PX.small` and set from `geom.fontPx` in `runShowSequence`.)
  - [x] 7.4 In the catch block (getSettings failure), add a `fontSize` fallback:
    `fontSize` is declared as `let fontSize: "small"|"medium"|"large" = "small"` before the `try` block; the catch block includes a comment confirming the fallback to "small".

- [x] **Task 8 — Verify and close** (AC-8, DoD)
  - [x] 8.1 `cargo test --lib` — 575 passed; 0 failed. New test `spec_preview_font_size_config_field_default` PASSES. No regression in `spec_preview_appearance_config_fields_default`.
  - [x] 8.2 `cargo check --target x86_64-pc-windows-gnu` — no new Rust errors from klarvo_lib; pre-existing whisper-rs/ggml MinGW C build failures are unchanged.
  - [x] 8.3 `tsc --noEmit` + `npm run build` — TypeScript exit 0; Vite build clean (✓ built in 7.26s).
  - [ ] 8.4 Windows settings-smoke (Andi, real build) — OPEN, gate for done:
    - Open Settings → Shortcuts → preview section
    - Klein/Mittel/Groß picker is visible and styled correctly
    - Pick Groß → live card in Settings shows larger text immediately (before Save)
    - Save → check `config.json` → key `"previewFontSize":"large"` present
    - Trigger recording + preview → box is noticeably wider/taller (k≈1.36 vs Small)
    - Switch to Klein → Save → preview box returns to normal compact size
    - Pill never resizes at any point (NFR2 regression baseline from 6.2)

## Dev Notes

### The ONE key change in PreviewPanel.tsx

Story 6.2 **intentionally** hardcoded `"small"` in `previewGeometry(widthPreset, "small")` and
`fontSize: 11` in the card style, with a comment: "Story 6.3 adds the font axis." **This story is
the sole owner of removing those two hardcodes.** The `previewGeometry` helper already accepts the
`fontSize` parameter — no new helper logic needed.

Current state (src/PreviewPanel.tsx, ~line 152):
```ts
const geom = previewGeometry(widthPreset, "small");  // ← replace "small" with fontSize variable
```

Current state (src/PreviewPanel.tsx, ~line 367):
```ts
fontSize: 11,  // ← replace with geom.fontPx
```

### Scale-factor model (do not re-derive)

From `docs/bar-redesign-spec.md §2` and `PreviewPanel.tsx`:
```ts
const FONT_PX = { small: 11, medium: 13, large: 15 } as const;
const BASE_WIDTH = { compact: 260, comfortable: 320, wide: 400 } as const;
const BASE_MAX_HEIGHT = 600;
// k = fontPx / 11: Small=1.0, Medium≈1.18, Large≈1.36
// width = round(BASE_WIDTH[preset] * k)
// maxHeight = round(600 * k)
```

These constants are ALREADY in `PreviewPanel.tsx`. Do NOT change them. Just pass `fontSize` through.

### Trap #1 — camelCase: `previewFontSize` NOT `preview_font_size`

`AppConfig` uses `serde(rename_all = "camelCase")`. The JSON key on disk is **`previewFontSize`**.
Verify by running the spec test (Task 1.4) which asserts the camelCase key is present. Any
manual edit to `config.json` must use `"previewFontSize"`. A wrong snake_case key is silently
ignored by serde → feature stays silently on "small" forever → smoke shows "no scaling".
(See Trap #1 in `docs/surface-smoke-checklist.md`.)

### Trap #2 — resync `useEffect` (critical — the most common new-field mistake)

`loadedSettings` in SettingsPanel is a prop that changes when settings are saved. The resync
`useEffect` at ~line 317 updates all local state from the new `loadedSettings`. Every new Settings
field MUST be in this `useEffect` or the Save button stays perpetually dirty after a save.
The 6.6 story comment at that block says explicitly: "ALL new settings fields MUST appear here."

### Trap #3 — separate-window reactivity (already handled, just pass fontSize)

`PreviewPanel` mounts ONCE at app start and never re-mounts. `runShowSequence` already calls
`getSettings()` reactively at show-time (not at mount). Story 6.3 only needs to extract
`previewFontSize` from the SAME `getSettings()` call that already reads `previewPanelForm`.
No new reactive mechanism needed — just read the field from the existing `s` response object.

### SettingsPatch construction in settings.rs — all test literals must be updated

`settings.rs` contains multiple `AppConfig` literals in test cases (grep: `preview_font_family:`).
Each one must gain a `preview_font_size` field. If any literal is incomplete, `cargo test` fails
to compile. Check for:
- The `SettingsView` / `get_settings` response construction (~line 652)
- The `save_settings` default patch construction (~line 1448)
- Any test fixtures that construct `AppConfig` directly (~line 2073, 3406, etc.)

### Placement in the appearance panel

The font-size picker is **Increment B** of the same panel 6.6 built. It belongs ABOVE the
width-preset ("Darstellung") picker in `ShortcutsContent.tsx`, ordered as:
1. "Appearance" heading + live card (AC-1 of 6.6) — keep as-is
2. Theme buttons (AC-2 of 6.6) — keep as-is
3. Color pickers + opacity sliders (AC-3 of 6.6) — keep as-is
4. Font-family dropdown (AC-4 of 6.6) — keep as-is
5. **NEW: Schriftgröße picker (this story AC-4)** ← insert here
6. Darstellung (width-preset picker) — shift below font-size

Rationale: font-SIZE and width-preset are both geometry/display controls (not color/style);
grouping them together is cleaner UX. Font-size goes first because it's the primary new axis.

### No Android change

This is a desktop-only Windows story. Do not touch any Android path.

### No new Rust commands

No new Tauri commands are needed. `set_preview_shape` already accepts a `radius` param and is
unchanged. The font-size scaling is pure frontend geometry: `previewGeometry` computes new
`width`/`maxHeight`, which are then passed to the existing `win.setSize(new LogicalSize(W, H))`.

### Inversion checks (must be verifiable)

Per Epic-4 retro AI-1: every guard spec must prove it goes RED when the invariant is flipped.

1. **AC-1 camelCase (Rust spec test):** Remove `#[serde(default = "default_preview_font_size")]`
   → `serde_json::from_str` errors → `assert!(result.is_ok())` goes RED.
2. **AC-5 k-scaling (smoke-time):** Temporarily hardcode `previewGeometry(widthPreset, "small")`
   → pick Groß → preview opens at same Small size → RED.
3. **AC-6 resync (manual check):** Omit `setLocalPreviewFontSize(...)` from resync `useEffect`
   → change size → Save → Save button stays highlighted (isDirty=true) → RED.

### Key files to touch

| File | Change |
|---|---|
| `src-tauri/src/config/mod.rs` | Add `preview_font_size` field + default fn + spec test |
| `src-tauri/src/commands/settings.rs` | Add to `SettingsPatch`, `merge_settings`, `save_settings` cmd, `SettingsView`, test fixtures |
| `src/types.ts` | Add `previewFontSize: string` to `AppSettings` |
| `src/tauri-commands.ts` | Add to `MOCK_SETTINGS` + `saveSettings` signature + invoke body |
| `src/hooks/useSettings.ts` | Add `newPreviewFontSize` param to `handleSaveSettings` + forward |
| `src/components/SettingsPanel.tsx` | State + resync useEffect + isDirty + saveCurrentSettings + onSave prop |
| `src/components/settings/ShortcutsContent.tsx` | Props + picker UI + live card fontSize |
| `src/PreviewPanel.tsx` | Replace `"small"` with `fontSize` var + replace `fontSize: 11` with `geom.fontPx` |

### References

- Scale-factor geometry model: `docs/bar-redesign-spec.md §2`
- `previewGeometry` helper (already complete, just pass fontSize): `src/PreviewPanel.tsx:31-43`
- Existing `FONT_PX`, `BASE_WIDTH`, `BASE_MAX_HEIGHT` constants: `src/PreviewPanel.tsx:22-28`
- Pattern for `preview_font_size` Rust field: mirror `preview_font_family` at `config/mod.rs:754`
- Pattern for `SettingsPatch` field: mirror `preview_font_family` at `commands/settings.rs:163`
- Pattern for `merge_settings`: mirror `preview_font_family` at `commands/settings.rs:355`
- Pattern for `SettingsView` response: mirror `preview_font_family` at `commands/settings.rs:655`
- Pattern for spec test: `spec_preview_appearance_config_fields_default` at `config/mod.rs:4069`
- `MOCK_SETTINGS` in tauri-commands.ts: lines 87–94
- `saveSettings` signature: `src/tauri-commands.ts:255` (add `previewFontSize` after `previewFontFamily`)
- `handleSaveSettings` in useSettings.ts: lines 71–127 (add `newPreviewFontSize` after `newPreviewFontFamily`)
- SettingsPanel resync `useEffect` comment (Trap #2): `src/components/SettingsPanel.tsx:317`
- SettingsPanel isDirty block: `src/components/SettingsPanel.tsx:354–420`
- ShortcutsContent appearance section: `src/components/settings/ShortcutsContent.tsx:506–730`
- `runShowSequence`: `src/PreviewPanel.tsx:98–203`
- Surface smoke checklist: `docs/surface-smoke-checklist.md`
- Epic 6 planning + full ACs: `_bmad-output/planning-artifacts/epics-bar-redesign.md#Story 6.3`
- Ist-Zustand: `docs/deep-dive-bar-subsystem.md`
- Project rules: `_bmad-output/project-context.md`

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

No debug log issues — all changes were mechanically straightforward following the story spec.

### Completion Notes List

- AC-1: `preview_font_size: String` field added to `AppConfig` with `serde(default = "default_preview_font_size")`. `default_preview_font_size()` returns `"small"`. Spec test `spec_preview_font_size_config_field_default` added — verified camelCase key `previewFontSize` in JSON, serde-default fallback, and zero migration writes. Test PASSES.
- AC-2: `SettingsPatch.preview_font_size: Option<String>` added; `SettingsPatch::default()` updated; `merge_settings` extended; `save_settings` Tauri command signature + body updated; `SettingsView` struct + `get_settings` response updated. All explicit `SettingsView` literals in lib.rs tests updated.
- AC-3: `previewFontSize: string` added to `AppSettings` interface; `MOCK_SETTINGS` updated; `saveSettings` signature + invoke body updated.
- AC-4 (Task 4): `handleSaveSettings` in `useSettings.ts` extended with `newPreviewFontSize` param; forwarded to `saveSettings`.
- AC-6 (Task 5): `localPreviewFontSize` state added to `SettingsPanel.tsx`; resync `useEffect` updated (Trap #2 covered); `isDirty` computation + deps updated; `saveCurrentSettings` onSave call + useCallback deps updated; `onSave` prop signature updated; `ShortcutsContent` receives `localPreviewFontSize`/`setLocalPreviewFontSize`.
- AC-4+AC-7 (Task 6): `ShortcutsContentProps` extended with `localPreviewFontSize`/`setLocalPreviewFontSize`; function destructures them; Klein/Mittel/Groß segmented picker added inside the Appearance section after font-family dropdown; Darstellung (width-preset) picker moved INSIDE the Appearance section (after font-size picker, per Dev Notes ordering — "shift below font-size"); live preview card updated with `fontSize: FONT_PX_MAP[localPreviewFontSize] ?? 11`; `FONT_PX_MAP` constant added at file top.
- AC-5 (Task 7): `PreviewPanel.tsx` — `let fontSize: "small"|"medium"|"large" = "small"` declared before try block; `fontSize = (s.previewFontSize || "small")` extracted from same `getSettings()` call; `previewGeometry(widthPreset, fontSize)` replaces hardcoded `"small"`; `cardFontPx: number` React state added; `setCardFontPx(geom.fontPx)` called in `runShowSequence`; card inline style uses `fontSize: cardFontPx` (replaces `fontSize: 11`).
- 575 Rust lib tests / 0 fail; tsc exit 0; vite build clean.
- Windows smoke (AC-8 Task 8.4) is OPEN — surface-class hard gate, Andi on Windows.

### File List

- `src-tauri/src/config/mod.rs` — `preview_font_size` field + `default_preview_font_size()` fn + `AppConfig::default()` + 2 AppConfig literal tests + `spec_preview_font_size_config_field_default` spec test
- `src-tauri/src/commands/settings.rs` — `SettingsPatch.preview_font_size` + Default impl + `merge_settings` + `save_settings` signature + body + `get_settings` response + 1 explicit SettingsPatch literal updated
- `src-tauri/src/lib.rs` — `SettingsView.preview_font_size` field + 3 SettingsView literal tests updated
- `src/types.ts` — `previewFontSize: string` in `AppSettings`
- `src/tauri-commands.ts` — `previewFontSize: "small"` in `MOCK_SETTINGS` + `saveSettings` signature + invoke body
- `src/hooks/useSettings.ts` — `newPreviewFontSize` param in `handleSaveSettings` + forwarded to `saveSettings`
- `src/components/SettingsPanel.tsx` — `localPreviewFontSize` state + resync useEffect + isDirty + saveCurrentSettings + onSave prop + ShortcutsContent prop pass-through
- `src/components/settings/ShortcutsContent.tsx` — `ShortcutsContentProps` + destructure + Schriftgröße picker + live card fontSize + FONT_PX_MAP + Darstellung moved inside Appearance section
- `src/PreviewPanel.tsx` — `cardFontPx` state + `fontSize` var in `runShowSequence` + `previewGeometry(widthPreset, fontSize)` + `setCardFontPx(geom.fontPx)` + `fontSize: cardFontPx` in card inline style

### Change Log

- 2026-06-07: Story 6.3 implemented — previewFontSize config field (Rust+TS), Klein/Mittel/Groß picker in Settings appearance panel, live card fontSize, and k-scaling in PreviewPanel. 575 tests/0 fail, tsc+vite green. Windows smoke pending.
- 2026-06-07: Code-review PASS (Opus 4.8, 3 adversarial layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor). CLEAN — 0 patch, 0 decision-needed, 2 defer, 5 dismiss. All 3 named traps (camelCase `previewFontSize` / resync-useEffect SettingsPanel:305 / separate-window reactivity via same getSettings() call) auditor-verified satisfied with evidence. 5 dismissed = Blind Hunter false positives refuted by the two repo-access layers (string fallbacks exist, FONT_PX imported + tsc green, /tmp test Linux-only, inline-fontSize intentional, runShowSequence reactivity correct). 2 deferred → deferred-work.md (FONT_PX_MAP duplication = story-sanctioned micro-refactor; card-width not clamped to work-area at wide+large ~545px = largely pre-existing geometry, negligible desktop trigger). No fix-loop needed. Status stays `review` — surface-class story, AC-8 Windows smoke (GATE 4) is the real final gate.
