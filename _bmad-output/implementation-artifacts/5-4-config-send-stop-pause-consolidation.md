---
story: "5.4"
epic: "5"
title: "Config — Send/Stop-Pause consolidation (Regler B)"
status: review
track: L3-feature
gatedBy: []
buildsOn: []
independent: true
inputDocuments:
  - _bmad-output/planning-artifacts/epics-live-preview.md
  - _bmad-output/project-context.md
  - _bmad-output/implementation-artifacts/5-3-settings-opt-in-preview-toggle-and-preview-pause-slider.md
---

# Story 5.4: Config — Send/Stop-Pause consolidation (Regler B)

Status: review

## Story

As a user,
I want a single "Send/Stop-Pause" slider instead of two separate per-mode controls,
so that the Shortcut settings are simpler — without breaking any platform that reads the underlying keys.

## Acceptance Criteria

**AC-1 (Single slider writes both keys — FR9/D3):**
Given the Shortcut section of the Settings UI (desktop only — inside the `{isDesktop && ...}` block in `ShortcutsContent.tsx`)
When the user adjusts the single "Send/Stop-Pause" slider (Regler B)
Then BOTH existing keys `auto_mode_silence_secs` AND `autostop_silence_secs` are written to the **same** value via the single sanctioned `save_settings` write path (ADR-0015 / Story 4-3 `save_config_locked` choke-point — no second writer)
And the two prior separate per-mode "Silence Duration" controls are removed from the UI.

**AC-2 (No key rename/removal — NFR3/ADR-0016):**
Given the `AppConfig` schema and the Rust `SettingsPatch` / `merge_settings`
When Story 5.4 ships
Then neither `auto_mode_silence_secs` nor `autostop_silence_secs` is renamed or removed from `AppConfig`, `SettingsPatch`, `merge_settings`, `SettingsView`, or `get_settings`
And no config migration is added — the keys keep their identity and serde defaults (2.0)
And a test or documented verification confirms both keys still exist unchanged so Android's `KlarvoApi.kt:239-240` reads are unaffected.

**AC-3 (Android parity verification — ADR-0016 guard):**
Given Android (`KlarvoApi.kt:239-240`) reads `autostopSilenceSecs` and `autoModeSilenceSecs` separately from `config.json`
And `KlarvoOverlayService.kt:807-808` selects between them mode-centrically (AUTO → `autoModeSilenceSecs`, AUTOSTOP → `autostopSilenceSecs`)
When the desktop UI consolidation ships
Then NO Android code change is required
And a unit test asserts that writing the consolidated value writes BOTH keys identically, so both Android reads return the same value (the [[android_silence_field_divergence]] guard).

**AC-4 (Diverged-values edge case — documented behavior):**
Given a user who hand-edited `config.json` such that `autostopSilenceSecs` and `autoModeSilenceSecs` hold different values
When the consolidated slider opens
Then it displays one defined value (the **larger** of the two) and writing the slider re-unifies both keys to the same value
And this behavior is documented in the UI (e.g. a sub-hint: "Sets the pause duration for Auto and Auto Stop modes.")
And it is NOT silent: the slider's displayed initial value visibly shows max(autostop, auto) so the user is aware.

**AC-5 (Dirty-check correctness — no stuck-dirty bug):**
Given the consolidated slider's local state is initialized from `max(autostopSilenceSecs, autoModeSilenceSecs)` from `loadedSettings`
When the user saves without changing the slider
Then the Save button is NOT dirty (isDirty stays false) — the same f32 widening lesson from Story 5.3's review applies: the slider should be compared with `Math.max(...)` from loadedSettings, not a separate stale value
And after a successful save, the `loadedSettings` re-sync useEffect re-initializes `localSendStopPauseSecs` from the updated loadedSettings so the dirty flag clears correctly.

**AC-6 (Round-trip unit test — Rust):**
Given `merge_settings` already handles `autostop_silence_secs` and `auto_mode_silence_secs` as two separate `Option<f32>` fields
When Story 5.4 ships
Then a new unit test `spec_send_stop_pause_writes_both_keys` asserts:
- Calling save with `autostop_silence_secs: Some(3.5)` AND `auto_mode_silence_secs: Some(3.5)` results in both fields being `3.5` in the merged config
- Calling with `None` for both preserves existing distinct values (no accidental clobber from the test side — the consolidation is enforced in the TypeScript save call, not in Rust `merge_settings`)
- **Inversion**: flip one of the `Some(3.5)` to `Some(3.0)` → the equality assert goes RED (empirically verify, document the RED result in Completion Notes — Epic-4-retro AI-1).

**DoD:**
- `cargo test` — all lib tests green (new round-trip test green).
- `tsc` / `npm run build` — PASS: 0 TypeScript errors.
- `cargo clippy` clean on touched Rust files.
- **Windows release build** via `scripts/sync-and-build.ps1` (mandatory — Linux tests mask Tauri runtime + WebView2 rendering bugs).
- **Manual smoke**: move the Send/Stop-Pause slider, confirm **both** `autostopSilenceSecs` AND `autoModeSilenceSecs` change to the same value in `%APPDATA%\com.klarvo.voice\config.json` (⚠️ **camelCase keys**: `autostopSilenceSecs` and `autoModeSilenceSecs`, NOT `autostop_silence_secs` / `auto_mode_silence_secs` — serde `rename_all = "camelCase"` silently ignores snake_case). Confirm Auto + AutoStop modes silence-stop at the new value.

## Tasks / Subtasks

### Task 1: TypeScript surface — new consolidated local state + slider + save wiring (AC-1, AC-4, AC-5)

- [x] 1.1 In `src/components/SettingsPanel.tsx`, add a new local state variable for the consolidated slider:
  ```tsx
  const [localSendStopPauseSecs, setLocalSendStopPauseSecs] = useState(() => {
    const autostop = loadedSettings?.autostopSilenceSecs ?? 2.0;
    const auto = loadedSettings?.autoModeSilenceSecs ?? 2.0;
    return Math.max(autostop, auto);  // AC-4: display the larger of two diverged values
  });
  ```

- [x] 1.2 In `SettingsPanel.tsx`, remove `localSilenceSecs` and `setLocalSilenceSecs` — this state is REPLACED by `localSendStopPauseSecs`. Also remove the mode-conditioned initialization logic at lines ~157-161 and ~276-281 (the `if (mode === "auto") setLocalSilenceSecs(...)` block).

- [x] 1.3 In `SettingsPanel.tsx`, update the `isDirty` useMemo/useEffect (lines ~344-345): replace the two separate mode-gated silence dirty-checks:
  ```ts
  // OLD (remove):
  ((localHotkeyMode === "autostop" || localHotkeyModeSlot2 === "autostop") && localSilenceSecs !== (loadedSettings.autostopSilenceSecs ?? 2.0)) ||
  ((localHotkeyMode === "auto" || localHotkeyModeSlot2 === "auto") && localSilenceSecs !== (loadedSettings.autoModeSilenceSecs ?? 2.0)) ||
  ```
  With a single check:
  ```ts
  // NEW (add):
  localSendStopPauseSecs !== Math.max(loadedSettings.autostopSilenceSecs ?? 2.0, loadedSettings.autoModeSilenceSecs ?? 2.0) ||
  ```

- [x] 1.4 In `SettingsPanel.tsx`, update the `loadedSettings` re-sync `useEffect` (lines ~276-281): replace the old `setLocalSilenceSecs(...)` calls with:
  ```ts
  const autostop = loadedSettings.autostopSilenceSecs ?? 2.0;
  const auto = loadedSettings.autoModeSilenceSecs ?? 2.0;
  setLocalSendStopPauseSecs(Math.max(autostop, auto));
  ```

- [x] 1.5 In `SettingsPanel.tsx`, update `saveCurrentSettings` (lines ~476-486): replace:
  ```ts
  const autostopSecs = localHotkeyMode === "autostop" ? localSilenceSecs : null;
  const autoModeSecs = localHotkeyMode === "auto" ? localSilenceSecs : null;
  ```
  With:
  ```ts
  // AC-1: Regler B writes BOTH keys unconditionally to the same value
  const autostopSecs = localSendStopPauseSecs;
  const autoModeSecs = localSendStopPauseSecs;
  ```
  Update the `useCallback` dependency array: remove `localSilenceSecs`, add `localSendStopPauseSecs`.

- [x] 1.6 Update the `useCallback` deps array for `saveCurrentSettings` — remove `localSilenceSecs`, add `localSendStopPauseSecs`.

### Task 2: Props cleanup — remove `localSilenceSecs` prop threading through `ShortcutsContent` (AC-1)

- [x] 2.1 In `src/components/settings/ShortcutsContent.tsx`, remove from `ShortcutsContentProps`:
  - `localSilenceSecs: number`
  - `setLocalSilenceSecs: (v: number) => void`
  And add:
  - `localSendStopPauseSecs: number`
  - `setLocalSendStopPauseSecs: (v: number) => void`

- [x] 2.2 In `ShortcutsContent.tsx`, remove the **two** existing "Silence Duration" slider blocks (Tab 1 lines ~328-348 and Tab 2 lines ~411-431 — the `{(localHotkeyMode === "autostop" || localHotkeyMode === "auto") && (...)}` guard blocks). Replace with a **single** "Send/Stop-Pause" slider section, placed OUTSIDE the `hotkeyTab` conditional (so it's always visible in the desktop Shortcuts section, regardless of tab), above the Live Preview section. Example placement: after the closing `</>` of the hotkeyTab block (after line ~436) and before the `{/* --- Live Preview --- */}` divider (line ~438):
  ```tsx
  {/* --- Send/Stop-Pause (Regler B) --- */}
  <div className="flex flex-col gap-3 border-t border-klarvo-border/30 pt-3 mt-1">
    <span className="text-xs font-semibold text-klarvo-muted uppercase tracking-wide">Auto Modes</span>
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between">
        <span className={LABEL_CLS}>Send/Stop-Pause</span>
        <span className="text-xs font-mono text-klarvo-primary">{localSendStopPauseSecs.toFixed(1)}s</span>
      </div>
      <input
        type="range"
        min={1.0}
        max={5.0}
        step={0.1}
        value={localSendStopPauseSecs}
        onChange={(e) => setLocalSendStopPauseSecs(parseFloat(e.target.value))}
        className="w-full accent-klarvo-primary"
      />
      <p className="text-[11px] text-klarvo-muted">Sets the pause duration for Auto and Auto Stop modes.</p>
    </div>
  </div>
  ```

- [x] 2.3 In `ShortcutsContent.tsx`, destructure the new props in the component signature (add `localSendStopPauseSecs, setLocalSendStopPauseSecs`, remove `localSilenceSecs, setLocalSilenceSecs`).

- [x] 2.4 In `SettingsPanel.tsx`, update the JSX props passed to `<ShortcutsContent ...>` (lines ~678): replace:
  ```tsx
  localSilenceSecs={localSilenceSecs} setLocalSilenceSecs={setLocalSilenceSecs}
  ```
  With:
  ```tsx
  localSendStopPauseSecs={localSendStopPauseSecs} setLocalSendStopPauseSecs={setLocalSendStopPauseSecs}
  ```

- [x] 2.5 In `ShortcutsContent.tsx`, also remove all usages of `loadedSettings?.autoModeSilenceSecs` and `loadedSettings?.autostopSilenceSecs` that were used to re-initialize `localSilenceSecs` on mode-switch (lines ~302-306). These are no longer needed — the single `localSendStopPauseSecs` value is not mode-dependent.

### Task 3: Rust — new unit test confirming both keys are written (AC-2, AC-3, AC-6)

- [x] 3.1 In `src-tauri/src/commands/settings.rs`, add a new unit test `spec_send_stop_pause_writes_both_keys` (inline `#[cfg(test)]` module):
  ```rust
  #[test]
  fn spec_send_stop_pause_writes_both_keys() {
      // AC-6: Merging the same value into BOTH keys → both fields equal in result
      let existing = AppConfig {
          autostop_silence_secs: 2.0,
          auto_mode_silence_secs: 2.0,
          ..AppConfig::default()
      };
      let patch = SettingsPatch {
          autostop_silence_secs: Some(3.5),
          auto_mode_silence_secs: Some(3.5),
          ..SettingsPatch::default()
      };
      let result = merge_settings(patch, existing);
      assert!(
          (result.autostop_silence_secs - 3.5).abs() < f32::EPSILON,
          "autostop_silence_secs should be 3.5, got {}",
          result.autostop_silence_secs
      );
      assert!(
          (result.auto_mode_silence_secs - 3.5).abs() < f32::EPSILON,
          "auto_mode_silence_secs should be 3.5, got {}",
          result.auto_mode_silence_secs
      );
      // INVERSION: flip one to 3.0 → the equality assert goes RED
      // (empirically verify during implementation; document RED result in Completion Notes)
  }
  ```

- [x] 3.2 `cargo test` — all lib tests green (new test green).
- [x] 3.3 `cargo clippy` on `src-tauri/src/commands/settings.rs` — no new warnings.

### Task 4: Build validation and smoke (AC-1..AC-5, DoD)

- [x] 4.1 `npm run build` — PASS: 0 TypeScript errors.
- [x] 4.2 `cargo test` — all lib tests green.
- [x] 4.3 `cargo clippy` on touched Rust files — no new warnings.
- [ ] 4.4 Windows release build via `scripts/sync-and-build.ps1`.
- [ ] 4.5 Manual smoke:
  1. Open Settings → Shortcuts section
  2. Confirm the two old per-mode "Silence Duration" sliders are **gone** (AC-1)
  3. Confirm a single "Send/Stop-Pause" slider appears (always visible, not gated behind a mode)
  4. Move the slider to e.g. 3.5 s → click Save
  5. Inspect `%APPDATA%\com.klarvo.voice\config.json` — verify **BOTH** `"autostopSilenceSecs": 3.5` AND `"autoModeSilenceSecs": 3.5` are present (⚠️ camelCase — NOT snake_case)
  6. Switch a hotkey to Auto Stop mode → dictate → confirm it silence-stops at ~3.5 s (AC-1 functional check)
  7. Switch to Auto mode → confirm it also silence-stops at ~3.5 s

## Dev Notes

### INDEPENDENT STORY — No dependency on 5.1–5.3 or 5.5

Story 5.4 is the **deferral seam** of Epic 5 — it touches only the Shortcuts Settings surface and the Rust save path. It has ZERO dependency on the preview pipeline (5.1), the FloatingBar panel (5.2), or the preview toggle/Regler A (5.3). It can ship, defer, or drop without touching any preview code.

### camelCase config.json — Critical pitfall (from 5.2/5.3 smoke)

`AppConfig` is `#[serde(rename_all = "camelCase")]`. The JSON keys are:
- `"autostopSilenceSecs"` (NOT `"autostop_silence_secs"`)
- `"autoModeSilenceSecs"` (NOT `"auto_mode_silence_secs"`)

Serde **silently ignores** a snake_case key — the feature would appear to save but the value would never change. The manual smoke in Task 4.5 MUST verify the camelCase keys in `config.json`.

### The Rust merge_settings layer does NOT change behavior

`merge_settings` already handles `autostop_silence_secs` and `auto_mode_silence_secs` as two independent `Option<f32>` fields. No change to `merge_settings` logic is needed. The consolidation is purely a **TypeScript layer concern**: the UI writes both params to the same value. This is the correct layering — the Rust layer stays generic.

The new unit test (Task 3) simply proves that writing `Some(3.5)` to both fields results in both being `3.5` after merge. This is a documentation/verification test, not a behavior change.

### Current save path for silence secs — what changes

Currently in `SettingsPanel.tsx`:
```ts
// Lines ~476-477 — OLD (mode-conditional, one or the other is null):
const autostopSecs = localHotkeyMode === "autostop" ? localSilenceSecs : null;
const autoModeSecs = localHotkeyMode === "auto" ? localSilenceSecs : null;
```
This means today only ONE of the two keys is ever written per save (the other is `null` → not written → preserved at its existing value). After Story 5.4:
```ts
// NEW (always write both to the same value):
const autostopSecs = localSendStopPauseSecs;
const autoModeSecs = localSendStopPauseSecs;
```
Both are now non-null on every save, so both keys are always unified.

### Android reads both keys separately — no code change required

`KlarvoApi.kt:239-240` reads both `autostopSilenceSecs` and `autoModeSilenceSecs` from `config.json`.
`KlarvoOverlayService.kt:807-808` selects mode-centrically (AUTO → `autoModeSilenceSecs`, AUTOSTOP → `autostopSilenceSecs`).

After Story 5.4, both keys hold the same value → Android gets the same threshold regardless of mode. No Android code change needed. The [[android_silence_field_divergence]] guard is satisfied (the fix to Android's bug was in commit `759087f`, which already made Android pick the right key; 5.4 now ensures both keys stay in sync from the desktop).

### `localSilenceSecs` is DELETED — check for lingering references

The existing `localSilenceSecs` / `setLocalSilenceSecs` state serves the per-mode silence slider. **Delete it entirely.** Search for all references in `SettingsPanel.tsx`, `ShortcutsContent.tsx`, and `ShortcutsContentProps`. Missing any reference causes a TypeScript compile error (safe — `tsc` will catch it).

The `ShortcutsContent.tsx` also has a mode-switch handler on lines ~302-306 that calls `setLocalSilenceSecs(loadedSettings?.autoModeSilenceSecs ?? 2.0)` or `setLocalSilenceSecs(loadedSettings?.autostopSilenceSecs ?? 2.0)` when the user clicks a mode button. Remove these calls — the single `localSendStopPauseSecs` is not mode-dependent.

### f32-serde widening — dirty-flag trap (from 5.3 review)

The f32 serde widening lesson from Story 5.3's review applies here too: after a save, `getSettings()` returns `f32` values that are widened to `f64` in JSON (e.g. `3.5` → `3.5000001192092896`). The `isDirty` check must compare `localSendStopPauseSecs` against `Math.max(...)` from `loadedSettings`, and the `loadedSettings` re-sync `useEffect` must re-initialize `localSendStopPauseSecs`. Without the re-sync, the Save button stays stuck dirty after every save (the same bug that bit 5.3's review).

### Slider range — match existing Silence Duration controls

The existing "Silence Duration" sliders use `min={1.0}` `max={5.0}` `step={0.1}`. Use the same range for the consolidated slider (NOT `min={0.5}` — that was the Preview-Pause slider's range).

### Placement — always visible, not mode-gated

The OLD sliders appeared only when the hotkey mode was `autostop` or `auto`. The NEW single slider should be **always visible** in the Shortcuts desktop section, labeled "Send/Stop-Pause" under an "Auto Modes" subsection. This matches the FR9/D3 decision to simplify the UI by removing per-mode controls.

### isDirty note: slot2 mode interaction

The OLD isDirty check also covered slot 2's mode (`localHotkeyModeSlot2 === "autostop"` / `"auto"`). The NEW check `localSendStopPauseSecs !== Math.max(...)` is simpler and correct regardless of slot. No slot-2 special case needed.

### Files to Modify

**TypeScript/React:**
- `src/components/SettingsPanel.tsx` — remove `localSilenceSecs` state; add `localSendStopPauseSecs`; update isDirty check; update loadedSettings re-sync; update saveCurrentSettings; update JSX props to ShortcutsContent
- `src/components/settings/ShortcutsContent.tsx` — remove `localSilenceSecs`/`setLocalSilenceSecs` props; add `localSendStopPauseSecs`/`setLocalSendStopPauseSecs`; remove two per-mode slider blocks; add single Send/Stop-Pause slider section

**Rust:**
- `src-tauri/src/commands/settings.rs` — new unit test `spec_send_stop_pause_writes_both_keys` only; NO changes to `SettingsPatch`, `merge_settings`, `SettingsView`, `get_settings`, or `save_settings` signature

**No Android changes.** `auto_mode_silence_secs` and `autostop_silence_secs` are preserved unchanged. Android reads are unaffected (NFR3 / ADR-0016).

### Inversion Check (L3 guard — Epic-4-retro AI-1)

The reviewer will mechanically invert the unit test AC-6:
- **AC-6 inversion**: In `spec_send_stop_pause_writes_both_keys`, change one `Some(3.5)` to `Some(3.0)` → the equality assert `(result.auto_mode_silence_secs - 3.5).abs() < f32::EPSILON` should go RED.

Document the inversion result in Completion Notes (confirm RED was observed, not just claimed).

### References

- `src/components/SettingsPanel.tsx:157-161` — `localSilenceSecs` current initialization (to remove)
  [Source: src/components/SettingsPanel.tsx#L157-L161]
- `src/components/SettingsPanel.tsx:276-281` — `loadedSettings` re-sync for `localSilenceSecs` (to replace)
  [Source: src/components/SettingsPanel.tsx#L276-L281]
- `src/components/SettingsPanel.tsx:344-345` — isDirty silence checks (to replace)
  [Source: src/components/SettingsPanel.tsx#L344-L345]
- `src/components/SettingsPanel.tsx:476-477` — `autostopSecs`/`autoModeSecs` conditional save (to replace)
  [Source: src/components/SettingsPanel.tsx#L476-L477]
- `src/components/SettingsPanel.tsx:678` — JSX props passed to ShortcutsContent (to update)
  [Source: src/components/SettingsPanel.tsx#L678]
- `src/components/settings/ShortcutsContent.tsx:171-172` — `localSilenceSecs`/`setLocalSilenceSecs` in props interface (to remove)
  [Source: src/components/settings/ShortcutsContent.tsx#L171-L172]
- `src/components/settings/ShortcutsContent.tsx:302-306` — mode-switch setLocalSilenceSecs calls (to remove)
  [Source: src/components/settings/ShortcutsContent.tsx#L302-L306]
- `src/components/settings/ShortcutsContent.tsx:328-348` — Tab 1 Silence Duration slider (to remove)
  [Source: src/components/settings/ShortcutsContent.tsx#L328-L348]
- `src/components/settings/ShortcutsContent.tsx:411-431` — Tab 2 Silence Duration slider (to remove)
  [Source: src/components/settings/ShortcutsContent.tsx#L411-L431]
- `src-tauri/src/commands/settings.rs:139-140` — `autostop_silence_secs` + `auto_mode_silence_secs` in `SettingsPatch` (unchanged, for reference)
  [Source: src-tauri/src/commands/settings.rs#L139-L140]
- `src-tauri/src/commands/settings.rs:320-321` — `merge_settings` for both fields (unchanged)
  [Source: src-tauri/src/commands/settings.rs#L320-L321]
- `src-tauri/src/commands/settings.rs:391-392` — `save_settings` params for both fields (unchanged)
  [Source: src-tauri/src/commands/settings.rs#L391-L392]
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:239-240` — Android reads `autostopSilenceSecs` + `autoModeSilenceSecs` (do NOT change)
  [Source: android/kotlin-src/com/klarvo/voice/KlarvoApi.kt#L239-L240]
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:807-808` — Android mode-centric selection (do NOT change)
  [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt#L807-L808]
- `_bmad-output/planning-artifacts/epics-live-preview.md#Story 5.4` — authoritative ACs + FR9 traceability
- `_bmad-output/project-context.md` — Windows release-build DoD, camelCase config rule, ADR-0015 single-writer, ADR-0016 Android parity

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

(none)

### Completion Notes List

**AC-1 (Single slider):** Replaced the two mode-gated `localSilenceSecs`-driven "Silence Duration" sliders in `ShortcutsContent.tsx` with a single always-visible "Send/Stop-Pause" slider under a new "Auto Modes" section. The slider is placed outside the `hotkeyTab` conditional (visible regardless of active tab), above the "Live Preview" section.

**AC-2/AC-3 (Key preservation):** No changes made to `AppConfig`, `SettingsPatch`, `merge_settings`, `SettingsView`, or `get_settings`. Both `autostop_silence_secs` and `auto_mode_silence_secs` keys remain unchanged. Android's `KlarvoApi.kt:239-240` reads are unaffected. Verified by the new unit test.

**AC-4 (Diverged-values):** Initial state uses `Math.max(autostopSilenceSecs ?? 2.0, autoModeSilenceSecs ?? 2.0)` — the larger of two diverged values is displayed. Sub-hint "Sets the pause duration for Auto and Auto Stop modes." is visible below the slider.

**AC-5 (Dirty-check correctness):** isDirty check uses `localSendStopPauseSecs !== Math.max(loadedSettings.autostopSilenceSecs ?? 2.0, loadedSettings.autoModeSilenceSecs ?? 2.0)`. The loadedSettings re-sync `useEffect` re-initializes `localSendStopPauseSecs` from the updated loadedSettings. f32-serde widening pitfall addressed (same pattern as 5.3).

**AC-6 (Round-trip test + INVERSION):** New test `spec_send_stop_pause_writes_both_keys` added to `src-tauri/src/commands/settings.rs`. Both cases verified:
- `Some(3.5)` for both keys → both fields equal 3.5 in merged config (GREEN)
- `None` for both → preserves distinct existing values (GREEN)
- **INVERSION (empirically verified — NOT self-attested):** Changed `auto_mode_silence_secs: Some(3.5)` → `Some(3.0)` in the patch → test panicked: "auto_mode_silence_secs should be 3.5, got 3" (RED confirmed). File restored to GREEN.

**`localSilenceSecs` deletion:** All references removed from both `SettingsPanel.tsx` and `ShortcutsContent.tsx`. TypeScript compilation confirmed 0 errors. `loadedSettings` in `ShortcutsContent` is now unused (all mode-switch re-init removed) → prefixed with `_loadedSettings` to suppress TS6133.

**Build results:** 569 Rust lib tests / 0 fail; `npm run build` (tsc + vite) PASS; clippy: 0 new warnings on `settings.rs`.

**Smoke (Task 4.4 + 4.5):** Requires Windows release build via `scripts/sync-and-build.ps1` + manual verification that moving the slider writes both `autostopSilenceSecs` AND `autoModeSilenceSecs` (camelCase) to the same value in `%APPDATA%\com.klarvo.voice\config.json`.

### File List

- `src/components/SettingsPanel.tsx` — replaced `localSilenceSecs` state with `localSendStopPauseSecs`; updated isDirty check; updated loadedSettings re-sync; updated saveCurrentSettings to write both keys unconditionally; updated JSX props to ShortcutsContent; updated both dep-arrays
- `src/components/settings/ShortcutsContent.tsx` — replaced `localSilenceSecs`/`setLocalSilenceSecs` props with `localSendStopPauseSecs`/`setLocalSendStopPauseSecs`; removed two per-mode Silence Duration slider blocks; removed mode-switch setLocalSilenceSecs calls; added single Send/Stop-Pause slider under "Auto Modes" section; `loadedSettings` destructured as `_loadedSettings`
- `src-tauri/src/commands/settings.rs` — added unit test `spec_send_stop_pause_writes_both_keys` (AC-6; no production code changes)
- `_bmad-output/implementation-artifacts/5-4-config-send-stop-pause-consolidation.md` — story progress tracking
- `_bmad-output/implementation-artifacts/sprint-status.yaml` — status updated

## Review Findings

**Code review 2026-06-05 (Opus, 3 adversarial layers: Blind Hunter / Edge Case Hunter / Acceptance Auditor).**
Verdict: all 6 ACs satisfied in code — AC-2/AC-3/AC-4/AC-6 verifiable now; AC-1/AC-5 final acceptance owed-on-Windows-smoke. 569 lib tests / 0 fail, tsc + vite green, clippy no-new. **Inversion empirically RED-verified by the reviewer** (flip `auto_mode_silence_secs: Some(3.5)`→`Some(3.0)` → test panicked "should be 3.5, got 3"; file restored GREEN) — not self-attested. Triage: 1 decision-needed, 0 patch, 2 deferred, 4 dismissed.

- [x] [Review][Decision] **RESOLVED 2026-06-05 (Andi): accept as specified** — the collapse-to-max() is the intended consolidation; `max()` keeps the longer/safer pause; niche case. No code change. Diverged silence values collapse to max() on ANY save, without dirty indication — Blind + Edge hunters both flag: if `autostop_silence_secs` ≠ `auto_mode_silence_secs` (reachable via the OLD per-mode UI, not only hand-edits), the slider shows `max()` and AC-1's unconditional both-key write collapses both keys to that value on the next save of *any* field; the silence field is not flagged dirty (the slider value already equals the `max()` basis), so the smaller value is lost silently for a user who never opens the Shortcuts tab. This is the spec's intended consolidation (AC-1 unconditional write + AC-4 display-max/re-unify; the Acceptance Auditor ruled AC-1/AC-4 PASS). A "write-only-when-touched" guard would *violate* AC-1 as written. Needs Andi's product call: accept as specified, or add a safeguard (small spec + dev revision).
- [x] [Review][Defer] Slider init/re-sync not clamped to range (min 1.0 / max 5.0) [src/components/SettingsPanel.tsx:157-160,276-278] — deferred, pre-existing. Hand-edited config below 1.0 / above 5.0 renders thumb clamped while state holds the raw value until first drag. Old code had the same gap; `Math.max` only makes the larger out-of-range value the one that surfaces.
- [x] [Review][Defer] Unused `loadedSettings` prop still passed to ShortcutsContent [src/components/settings/ShortcutsContent.tsx:184] — deferred, low-sev cleanliness. The mode-switch re-init that consumed it was removed; prop aliased to `_loadedSettings` to satisfy tsc. Could be dropped from the interface + the parent call site.

**Dismissed (4):** (1) Rust test doesn't cover the TS consolidation — by design per AC-6 (consolidation lives in the TS save call; no frontend unit-test infra → verified by Windows smoke). (2) Inversion doc-comment "self-attested/unverifiable" — reviewer mechanically verified RED. (3) NaN from `parseFloat` — a native range input cannot emit non-numeric; pre-existing project-wide slider pattern. (4) Cosmetic blank lines left by block removal — no functional/lint impact.
