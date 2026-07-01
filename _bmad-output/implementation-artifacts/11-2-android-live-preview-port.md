---
story: "11.2"
epic: "11"
title: "Android Live-Preview — Groq-Delta-STT Port (text panel, HOLD/TOGGLE only, Settings mirror)"
status: review
track: L3-feature
gatedBy: ["11.1"]
buildsOn: ["11.1"]
enabledBy: []
inputDocuments:
  - _bmad-output/implementation-artifacts/11-1-android-live-preview-feasibility-benchmark.md
  - docs/backlog.md#Epic 11 — Cross-Platform Live-Preview (Android)
  - _bmad-output/planning-artifacts/epics-live-preview.md
  - _bmad-output/implementation-artifacts/5-1-backend-pause-triggered-delta-flush-for-toggle-hold.md
  - _bmad-output/implementation-artifacts/5-3-settings-opt-in-preview-toggle-and-preview-pause-slider.md
  - _bmad-output/implementation-artifacts/6-6-preview-box-appearance-customization.md
  - _bmad-output/implementation-artifacts/6-3-font-size-axis-preview-font-size-config-settings-picker-k-scaling.md
  - _bmad-output/project-context.md
---

# Story 11.2: Android Live-Preview — Groq-Delta-STT Port

Status: review

> **Epic 11 — Cross-Platform Live-Preview.** Story 11-1 (benchmark, `done`) measured Android's
> Groq pause-to-text latency at median 786 ms / max 999 ms (n=4, real device) — **GO**. This story
> is the actual port: it brings the desktop Live-Cleanup-Preview experience (Epics 5+6, both
> `done`) to Android, using the **same architecture family** (pause-triggered delta-flush,
> display-only, never feeds the pasted output) but a **native Kotlin panel**, not a Tauri window.

## Design decisions (Andi, 2026-07-01 — binding, do not re-litigate)

1. **Grundform** = a treu port of the dark desktop Live-Cleanup-Preview card (live raw/cleaned-free
   text as orientation, not accuracy) — **but** the waveform and the ➤ Senden / ✗ Abbrechen buttons
   stay at the bubble (top), **not** in the preview. The preview is a **pure text surface**.
2. **Position** = the preview occupies the **existing bottom overlay panel** slot and **may/should
   cover the keyboard**; the Klarvo bubble only appears once the keyboard is open, and the keyboard
   has **no function** during recording. **No IME-inset avoidance, no top-pinning.**
3. **Settings** = mirror desktop **fully**: on/off toggle + color/font/width appearance settings
   ported to Android.
4. **Architecture** = Groq delta-STT like desktop (`delta_snapshot_wav`-equivalent: each pause STTs
   only the new audio since the last flush; total ≈ **2× Groq** audio-seconds per dictation, not
   N×). A local on-device model for preview is **deferred** (own latency benchmark needed, dormant
   JNI not activated) — out of scope here.

## Story

As an Android user dictating in Toggle or Hold mode,
I want to see my raw speech appear as text in the bottom overlay panel each time I pause,
so that I get the same live orientation Windows users already have, without changing what gets pasted.

## Scope boundaries (read before touching code)

**IN:**
- A repeatable, pause-triggered delta-flush in Kotlin (HOLD/TOGGLE only) that transcribes only new
  audio via the existing Groq path and appends raw text to the panel — recording is **not** stopped,
  nothing is pasted.
- Extending `ListeningPanelView`'s existing transcript text area to show the accumulated preview
  instead of the current debug-harness value, with auto-scroll to newest text.
- A Settings toggle (default **off**) + pause-duration slider + appearance controls (color/font/
  width), reusing the existing desktop `AppearanceContent.tsx` category and the config fields that
  **already exist** in `AppConfig` (no Rust changes — see Dev Notes).
- Kotlin-side reads of those same camelCase config keys (mirrors the existing `KlarvoApi.readConfig`
  pattern).

**OUT (do not touch):**
- The bubble (`FloatingBubbleView.kt`), the ➤/✗ cluster, the waveform, HOLD dock visuals — all stay
  exactly as-is (9-12..9-16 territory). This story only touches the passive panel's transcript area
  and its show/hide/appearance.
- Auto and AutoStop modes — **no preview flush for them**, same as desktop (FR4 parity, see
  `epics-live-preview.md` FR4). Their existing one-shot `onSilenceDetected` stop-callback path is
  untouched.
- The finish/paste path — `stopAndProcessRecording` → `processAudio` → paste stays byte-identical.
- Any Rust/desktop file. All 11 preview config fields already exist in `AppConfig`
  (`src-tauri/src/config/mod.rs:730-790`, camelCase JSON, serde defaults) — this story is
  **Kotlin + shared-React only**.
- Local/on-device STT for preview (deferred per Andi's decision above).
- `FLAG_NOT_TOUCHABLE` on any overlay window (project-wide rule).

## Acceptance Criteria

**AC-1 (Repeatable pause-flush primitive, Kotlin — mirrors desktop `delta_snapshot_wav`/AR1):**
Given a HOLD or TOGGLE recording in progress with `livePreviewEnabled == true`,
When a speech pause ≥ the configured pause-silence threshold is detected,
Then the audio **since the last flush** (not the whole buffer) is encoded to WAV and transcribed via
the existing `transcribeWithRetry`/`GroqSttBridge.nativeTranscribe` + hallucination-filter chain as
**raw text** (no LLM cleanup),
And recording **continues uninterrupted** — no stop, no paste, no auto-loop,
And this fires **repeatedly**, once per pause, for the whole recording (not one-shot).

**AC-2 (Existing one-shot VAD callback must NOT be reused as-is; and VAD must be fed at all in HOLD/TOGGLE):**
Given `KlarvoAudioRecorder`'s VAD is fed only when `if (onSilenceDetected != null && !silenceCallbackFired)`
(`KlarvoAudioRecorder.kt:250`), and `onSilenceDetected` is assigned **only** inside the
AUTOSTOP/AUTO block (`KlarvoOverlayService.kt:1462`) — so in **HOLD/TOGGLE today `onSilenceDetected`
is `null`, `feedVad` never runs, and no pause edges are produced at all** — and given the existing
`onSilenceDetected` is furthermore a **one-shot** callback that, once fired, sets
`silenceCallbackFired` and stops feeding the VAD entirely,
When implementing the repeatable preview-flush trigger,
Then (a) the `feedVad` gate is **widened** so VAD is fed whenever preview is active, e.g.
`if ((onSilenceDetected != null && !silenceCallbackFired) || onPreviewPause != null) { feedVad(...) }`
— otherwise HOLD/TOGGLE produces no pause edges and preview never fires; **and** (b) a **separate,
repeatable** callback slot (`onPreviewPause`) is added that is **not** gated by
`silenceCallbackFired` (fires on every silence-onset edge for the whole recording),
And the existing AUTOSTOP/AUTO one-shot `onSilenceDetected` path (install site
`KlarvoOverlayService.kt:1462`) is **not** modified or reused for this — the one-shot semantics for
those modes must stay byte-identical.

**AC-3 (Scope guard — HOLD/TOGGLE only, mirrors FR4):**
Given Auto or AutoStop mode is active,
When a pause is detected,
Then the existing per-segment stop/paste/loop behavior runs unchanged and **no** preview-flush
callback is installed — verified by a JVM unit test on the pure guard function (mirrors the existing
`RecordingMode.selectSilenceSecs` pure-function-testable pattern, `KlarvoOverlayService.kt:146`,
and its test `RecordingModeSilenceSelectionTest.kt`).

**AC-4 (Settings guard — opt-in, default off, no Rust change):**
Given `livePreviewEnabled == false` (the serde default already shipped in `AppConfig`,
`src-tauri/src/config/mod.rs:735-736`),
When a HOLD/TOGGLE recording runs,
Then no preview-flush callback is installed and Android's existing recording behavior is
byte-identical to today (no extra Groq calls, no panel text change).

**AC-5 (Panel displays accumulated raw text, no waveform/buttons added):**
Given one or more preview chunks have been appended during a recording,
When the panel is visible,
Then `ListeningPanelView.transcriptTextView` shows the accumulated raw text (newest appended, not
replaced) and auto-scrolls so the newest text is visible,
And no waveform, no ➤/✗ controls are added to the panel (they remain bubble-only, confirmed by the
current `TopRowView`/`FooterView` code already being passive — see Dev Notes),
And the panel keeps using its existing bottom-anchored `WindowManager` window (no new overlay layer
needed) and existing `showListeningPanel`/`hideListeningPanel` lifecycle.

**AC-6 (No IME-avoidance for the panel — confirms existing behavior, do not add any):**
Given the panel is already `gravity = BOTTOM`, `MATCH_PARENT` width, with **no** existing
keyboard-avoidance logic (unlike the bubble's `adjustBubbleForKeyboard`),
When the keyboard opens during a HOLD/TOGGLE recording with preview on,
Then the panel is **not** moved, resized, or top-pinned to avoid the IME — it may sit under/behind
the keyboard exactly as today,
And no new IME-avoidance code is added for the panel (inversion: adding an IME-inset offset to the
panel would contradict this AC — a reviewer must confirm none was added).

**AC-7 (Preview clears at Finish — mirrors FR7):**
Given any number of preview chunks were appended during a recording,
When the user finishes (release / 2nd tap → `stopAndProcessRecording` → `processAudio` → paste),
Then the accumulated preview text is cleared (panel returns to empty/hidden per its existing
DONE/IDLE transitions) and the **paste output is unaffected** — `processAudio`'s existing
transcribe → hallucination-filter → cleanup → paste chain runs exactly as before preview text
existed (byte-identical output; the flush chunks are display-only and never feed this path).

**AC-8 (Settings — mirror desktop, reuse existing config fields, no Rust changes):**
Given the "Appearance" settings category (`src/components/settings/types.ts:37-42`) is currently
`desktopOnly: true` and its `AppearanceContent.tsx` component already renders the toggle
(`localLivePreviewEnabled`), pause slider (`localPreviewPauseSilenceSecs`), theme/color pickers,
font-family dropdown (the width preset is HIDDEN on Android — see Task 4.3) — all wired to
config fields that **already exist** end-to-end (`AppConfig` → `SettingsPatch` → `merge_settings`
→ `SettingsView` → `AppSettings` → `saveSettings`, all camelCase, all already shipped),
When this story ships,
Then Android users can reach the same "Appearance" category and toggle/tune the preview exactly
as desktop users do — implemented by relaxing the `desktopOnly` gate for `id: "appearance"` in
`SettingsHome.tsx`'s filter (`if (cat.desktopOnly && !isDesktop) return false;`, line 27) so it
renders on mobile too (component itself needs no `isDesktop`-guarded changes — verify none of its
7 fields' controls are hidden by a nested `isDesktop` check before relying on this),
And the same config.json (camelCase keys) is read on Android via `KlarvoApi.Config` /
`KlarvoOverlayService`/`ListeningPanelView` — a new block of `json.opt*("livePreviewEnabled", ...)`
etc. added to `KlarvoApi.readConfig` mirroring the existing `bubbleTapSilenceSecs` pattern
(`KlarvoApi.kt:256`).

**AC-9 (Appearance actually renders on the panel):**
Given the ported config fields (`previewTextColor`, `previewBgColor`, `previewBgBlur`,
`previewBorderColor`, `previewBorderWidth`, `previewBorderRadius`, `previewFontFamily`,
`previewFontSize`),
When the panel is shown with preview enabled,
Then `ListeningPanelView`'s background, border, transcript text color, and font reflect the current
config values (read at panel-show time, mirrors the "separate-window reactive read" lesson from
desktop Story 6.6 — read fresh config on each show, do not cache stale values across the Settings
session) — font-family maps to an Android `Typeface` (curated stack → nearest available system
font; monospace already used today is one valid mapping target).

**DoD (surface-class — mirrors project testing rules):**
- New pure Kotlin guard/helper logic (AC-3's mode guard, any delta-marker arithmetic) has JVM unit
  tests in `android/kotlin-test/com/klarvo/voice/`, following the existing pattern
  (`RecordingModeSilenceSelectionTest.kt`, `SilencePreFilterTest.kt`).
- `npm run build` / `tsc` clean (Appearance category filter change + any TS touch).
- **Real-device Android smoke required** — `scripts/android-smoke.sh` (build/install) **plus a
  real recording session on Andi's device**: HOLD or TOGGLE dictation with preview enabled, at
  least 2 speech pauses, confirm raw text accumulates in the panel, confirm the keyboard can be
  open with the panel visible without avoidance jank, confirm Finish still pastes correctly and
  clears the preview, confirm the Settings toggle/off returns to today's exact behavior (AC-4).
  GATE-4 = real device, never emulator (Android visual/timing rule, `reference_android_emulator_window_structure_oracle`).
- Confirm AC-2's inversion at review time: temporarily reuse the existing one-shot
  `onSilenceDetected` for the preview flush → after the first pause, `feedVad` stops (per
  `KlarvoAudioRecorder.kt:250`) → no further pauses are ever detected → RED. This proves the new
  repeatable mechanism is load-bearing, not redundant.
- Confirm AC-3/AC-4 guard inversions on the JVM test (mirrors the reviewer-verified inversion
  discipline from desktop 5.1: flip the mode/enabled check → assertion goes RED, documented in
  Completion Notes, not self-attested).

## Tasks / Subtasks

- [x] **Task 1 — Repeatable pause-flush trigger in `KlarvoAudioRecorder`** (AC-1, AC-2)
  - [x] 1.1 Add a second callback slot (e.g. `var onPreviewPause: (() -> Unit)? = null`) that is
    invoked on every silence-onset edge **without** the `silenceCallbackFired` gate, and does
    **not** stop `feedVad`. Keep the existing `onSilenceDetected`/`silenceCallbackFired` one-shot
    path completely untouched (AUTOSTOP/AUTO parity).
  - [x] 1.2 Add a delta marker (sample-count offset into `pcmBuffer`, mirrors desktop's
    `delta_marker: Mutex<usize>`) and a `deltaSnapshotWav(): ByteArray?` method: returns `null` if
    no new samples since the marker, otherwise slices `pcmBuffer` from the marker, encodes via the
    existing `encodeWav(...)`, advances the marker, returns the WAV bytes. Reset the marker on
    `start()`.
  - [x] 1.3 Unit test (pure, JVM): two synthetic sample batches → two `deltaSnapshotWav()` calls →
    assert disjoint + union == full buffer (mirrors desktop's `spec_delta_snapshot_disjoint_union`).
    Inversion: skip the marker advance → second delta overlaps first → test RED (document empirically).

- [x] **Task 2 — Install/guard the preview-flush callback in `KlarvoOverlayService`** (AC-1, AC-3, AC-4)
  - [x] 2.1 Add a pure guard function `shouldInstallPreviewFlush(mode: RecordingMode, livePreviewEnabled: Boolean): Boolean`
    (HOLD/TOGGLE + enabled → true; else false) next to `RecordingMode.selectSilenceSecs`
    (`KlarvoOverlayService.kt:146`) — same pure/testable pattern.
  - [x] 2.2 JVM test mirroring `RecordingModeSilenceSelectionTest.kt`: all 4 modes × enabled/disabled
    → correct booleans. Inversion: flip one branch → test RED (document empirically, AC-3/AC-4).
  - [x] 2.3 In the recording-start path (`KlarvoOverlayService.kt` ~line 1447-1463, alongside the
    existing `if (activeMode == RecordingMode.AUTOSTOP || activeMode == RecordingMode.AUTO)` install),
    add: if `shouldInstallPreviewFlush(activeMode, cachedConfig.livePreviewEnabled)`, wire
    `recorder.onPreviewPause = { handler.post { flushPreviewDelta() } }`.
  - [x] 2.3a **Preview-Pause threshold must be functional (resolves the AC-4/AC-8 slider vs. VAD
    contradiction).** The ported `previewPauseSilenceSecs` slider (AC-8) must actually govern how
    long a pause triggers a preview flush — it cannot be inert. Because HOLD/TOGGLE have **no**
    mode-level silence window today (`selectSilenceSecs` falls back to the per-gesture tap/long-press
    values, which are only *used* by AUTOSTOP/AUTO), the recorder's VAD `requiredSilentFrames` for
    the **repeatable `onPreviewPause` edge** must be derived from `cachedConfig.previewPauseSilenceSecs`
    (its own independent window, exactly mirroring desktop where `preview_pause_silence_secs` is a
    distinct config field from the mode silence). Concretely: the recorder needs to compute the
    pause-frame threshold for the preview signal from `previewPauseSilenceSecs`, independent of any
    AUTOSTOP/AUTO `silenceSecs`. If reusing the single existing VAD state machine forces one shared
    window, that is a genuine conflict — do **not** silently make the slider inert; either give the
    preview signal its own frame counter or escalate the conflict in Completion Notes. Add a unit
    test that a larger `previewPauseSilenceSecs` yields a larger `requiredSilentFrames` for the
    preview edge (inversion: hard-code the threshold → slider has no effect → RED).
  - [x] 2.4 Implement `flushPreviewDelta()`: call `recorder.deltaSnapshotWav()`, if non-null,
    transcribe via the same `transcribeWithRetry`/`GroqSttBridge.nativeTranscribe` +
    `nativeIsHallucination` chain used by `processAudio` (`KlarvoOverlayService.kt:1785-1833`,
    **raw text only, skip the LLM cleanup step**), on success append to the panel's accumulated
    preview text (Task 3), on failure log + skip (fail-soft, mirrors desktop AC-8 in 5.1).
  - [x] 2.5 Clear the accumulated preview text + reset the delta marker in `stopAndProcessRecording`
    (AC-7), after the existing paste/finish logic runs — the finish path itself is untouched.

- [x] **Task 3 — Panel: accumulate + display + auto-scroll** (AC-5)
  - [x] 3.1 Add an accumulator (e.g. `StringBuilder`/`List<String>` on the service or panel) that
    `flushPreviewDelta()` appends to; replace the current `panelView?.rawTranscript = debugTranscript`
    debug-only feed (`KlarvoOverlayService.kt:370`) with the real accumulated preview text when
    `livePreviewEnabled` — **do not remove** the debug-harness path outright if it's used by
    `DEBUG_SET_STATE` tooling; gate cleanly (real preview text takes over only during a genuine
    HOLD/TOGGLE recording with preview on).
  - [x] 3.2 `ListeningPanelView.transcriptTextView` (or its containing `ScrollView`, if one is
    added) auto-scrolls to the bottom on each append. Check whether `transcriptTextView` is
    currently wrapped in a scrollable container (`ListeningPanelView.kt:190-215` transcriptFrame) —
    add scrolling if not present.
  - [x] 3.3 Confirm no waveform/button is added anywhere in `ListeningPanelView` — `TopRowView` stays
    K-badge + label + timer only (already the case per `KlarvoOverlayService.kt`/`ListeningPanelView.kt`
    "Modell B" comments — do not regress this).

- [x] **Task 4 — Settings: relax `desktopOnly`, reuse `AppearanceContent.tsx`** (AC-8)
  - [x] 4.1 In `src/components/settings/types.ts`, remove (or make platform-conditional) the
    `desktopOnly: true` on the `"appearance"` category (line ~41) so `SettingsHome.tsx`'s filter
    (`if (cat.desktopOnly && !isDesktop) return false;`, line 27) stops excluding it on Android.
  - [x] 4.2 Read through `AppearanceContent.tsx` end-to-end to confirm no control inside is itself
    gated by a nested `isDesktop` check (none found in the initial read of lines 1-70, but the
    dev must verify the full ~150+ line file, especially the width-preset picker — see Task 4.3
    and the elicitation item below).
  - [x] 4.3 **DECIDED (Andi, 2026-07-01, GATE 1): HIDE the width preset on Android.** The
    `previewPanelForm` (compact/comfortable/wide) control sets a floating-window pixel WIDTH on
    desktop (`5-5-settings-preview-display-form-presets.md`); Android's panel is already
    `MATCH_PARENT` width, so the preset has no analog. Hide the `previewPanelForm` control on
    Android via a scoped `isDesktop`/platform prop passed into `AppearanceContent.tsx` (show only
    the toggle, pause slider, color pickers, and font-family dropdown on mobile). Do NOT repurpose
    it and do NOT drop the config field (desktop keeps using it) — only the mobile *control* is
    hidden.
  - [x] 4.4 `npm run build` / `tsc --noEmit` clean.

- [x] **Task 5 — Kotlin config read for the ported fields** (AC-8, AC-9)
  - [x] 5.1 In `KlarvoApi.kt`'s `readConfig`/`Config` (mirrors `bubbleTapSilenceSecs` at line 256),
    add reads for: `livePreviewEnabled` (Boolean), `previewPauseSilenceSecs` (Float),
    `previewTextColor`, `previewBgColor` (String, rgba), `previewBgBlur` (Int),
    `previewBorderColor` (String), `previewBorderWidth`, `previewBorderRadius` (Int),
    `previewFontFamily`, `previewFontSize` (String) — all camelCase, all with the exact desktop
    serde defaults as Kotlin fallbacks (`src-tauri/src/config/mod.rs:1001-1032`).
  - [x] 5.2 In `ListeningPanelView`/`KlarvoOverlayService`, apply these values to the panel's
    background/border/text-color/font at show-time (AC-9) — parse the rgba strings the same way
    desktop's `PreviewPanel.tsx` does (reuse or port the parsing logic conceptually; Android has no
    direct access to the TS `previewAppearance.ts` helpers, so a Kotlin-side rgba parser is new
    code — keep it a small pure/testable function).
  - [x] 5.3 Font-family: map the curated desktop stack values (`PREVIEW_FONTS` in
    `src/components/settings/previewAppearance.ts`) to Android `Typeface` equivalents (System UI →
    default, Monospace → `Typeface.MONOSPACE`, Serif → `Typeface.SERIF`, Inter → whatever font
    asset Android already ships for `KlarvoTheme.kt`, if any — check before assuming a new font
    asset is needed).

- [x] **Task 6 — Verify + close** (all ACs, DoD)
  - [x] 6.1 New JVM unit tests green (Tasks 1.3, 2.2) — confirm inversions RED empirically, document
    in Completion Notes.
  - [x] 6.2 `npm run build` / `tsc` clean.
  - [x] 6.3 `scripts/android-smoke.sh` clean build/install.
  - [x] 6.4 Real-device smoke per DoD — Andi's action (he can establish this test state himself:
    open Klarvo, HOLD/TOGGLE dictation with preview on, speak with pauses, watch the panel).

## Dev Notes

### Why the existing `onSilenceDetected` cannot be reused (AC-2 — the hardest design point)

`KlarvoAudioRecorder.kt:250`: `if (onSilenceDetected != null && !silenceCallbackFired) { feedVad(buf, read) }`.
Once the one-shot callback fires, **VAD feeding stops entirely** for the rest of the recording — this
is stronger than desktop's equivalent gate (desktop's `fired` flag only blocks the *callback
invocation*, not the underlying silence-detection loop; see `5-1-...md` Dev Notes "Key Constraint:
Repeatable Silence Callback"). A second, ungated callback slot on `KlarvoAudioRecorder` is required
— this is a direct Kotlin-side parallel of desktop's chosen "Option A" (separate repeatable
mechanism, zero risk to the existing AUTOSTOP/AUTO one-shot path).

### Current panel already has no waveform/buttons (confirms AC-1/AC-5's "pure text" requirement is mostly already true)

`ListeningPanelView.kt`'s `TopRowView.applyAnimatorsForState`/`onDraw` (lines ~330-410) already
comment "Panel is passive (Modell B): no waveform, no live-dot, no moving elements here" and
"Waveform, live-dot, pulse ring, and cancel button are in the cluster (bubble window)" — this is the
9-16 revert-to-compact-cluster state. The panel currently shows only: grip handle, K-badge + label +
timer (`TopRowView`), a monospace `transcriptTextView` (13sp, currently fed the debug harness value
only), a blinking caret, and a footer. **This story's job is mostly: feed real accumulated preview
text into the already-existing text surface, add auto-scroll, and apply the ported appearance
config** — not build a new display surface from scratch.

### Panel window / lifecycle (do not change)

`ListeningPanelView` is added as its own `TYPE_APPLICATION_OVERLAY` window in
`KlarvoOverlayService.showListeningPanel()` (~lines 2030-2073): `gravity = Gravity.BOTTOM`,
`MATCH_PARENT` width, `WRAP_CONTENT` height (200dp floor, `ListeningPanelView.kt` init block),
`FLAG_NOT_FOCUSABLE | FLAG_LAYOUT_IN_SCREEN` — deliberately **no** `FLAG_NOT_TOUCHABLE` (HyperOS
alpha-dim quirk, see `reference_hyperos_overlay_quirks`). Shown for RECORDING/TRANSCRIBING states via
calls at lines 365, 374, 1495, 1610; hidden via `hideListeningPanel()` (2082-2097, 320ms slide-down).
This story does not add a new window — it extends the existing one.

### Keyboard visibility already exists — do not add IME-avoidance to the panel

`KlarvoOverlayService.kt`: `keyboardVisible` (line 183), `onKeyboardVisibilityChanged`
(700-705, from `KlarvoAccessibilityService`) → `applyKeyboardState` (733-762) shows/hides the
**bubble** on IME open/close, with a reflection-based fallback poll (`checkKeyboardVisibility`,
804-819). The bubble additionally avoids the keyboard vertically via `adjustBubbleForKeyboard`
(786-802) — **this bubble-only avoidance logic must NOT be extended to the panel** (AC-6). The panel
already has zero avoidance logic and already sits at the screen bottom via `gravity = BOTTOM`
regardless of IME — this is exactly the desired behavior per Andi's directive, so the main risk is a
well-intentioned dev *adding* avoidance by analogy with the bubble. Don't.

### Config fields already exist — zero Rust changes

All 11 preview fields (`live_preview_enabled` through `preview_font_size`) are already in
`AppConfig` with camelCase serde output and defaults (`src-tauri/src/config/mod.rs:730-790,
1001-1032`), already wired through `SettingsPatch`/`merge_settings`/`SettingsView`/`get_settings`/
`save_settings` (`src-tauri/src/commands/settings.rs`), and already exposed in
`AppSettings`/`saveSettings` (`src/types.ts`, `src/tauri-commands.ts`). **Android reuses this
unchanged** — the only new work is (a) relaxing the frontend category gate (Task 4) so the
Settings UI renders on mobile, and (b) a new Kotlin-side JSON read (Task 5) since
`KlarvoOverlayService`/`KlarvoApi` reads `config.json` directly (bypasses Tauri IPC, per
ADR-0016/project-context.md "Android bypasses Tauri IPC ~85%"). Android's own Settings screen is
`MainActivity : TauriActivity()` (`android/kotlin-src/com/klarvo/voice/MainActivity.kt:37`) running
the **same shared React app** — `save_settings` on Android writes the same `config.json` (in the
app's own `dataDir`, not shared with desktop), which the overlay-service Kotlin code then reads
directly. There is no cross-device config sharing implied here — Android's `config.json` is
Android's own, written by its own embedded Tauri instance's Settings screen.

### Settings category location (current desktop layout, post-6.6/6.3)

All 11 preview-related controls (toggle, pause slider, panel-form/width preset, theme buttons,
color pickers, font-family dropdown, font-size picker) live together today in
`src/components/settings/AppearanceContent.tsx` under the `"appearance"` category
(`src/components/settings/types.ts:37-42`), **not** under Shortcuts (that was 5.3's original
location before 6.6/6.3 moved everything appearance-related out). `SettingsHome.tsx:27` filters this
category out on mobile today via `desktopOnly: true`. Relaxing that one flag is the single
integration point (Task 4.1) — confirmed by grep: `ShortcutsContent.tsx` no longer contains any
`LivePreview`/`PreviewPauseSilence` strings.

### Existing Kotlin config-read pattern to mirror (Task 5)

`KlarvoApi.kt:234-366` `readConfig()`: opens `config.json` from `getDataDir(context)`, reads each
camelCase key via `json.optDouble/optInt/optBoolean/optString(key, default)` into a `data class
Config`. Example already in the file: `val bubbleTapSilenceSecs = json.optDouble("bubbleTapSilenceSecs", 2.0).toFloat()`
(line 256); nested example: `advanced.silenceThreshold` (lines 277-283). Follow this exact style —
same file, same function, same fallback-default discipline (never throw; `Config = null` only if
the whole file is unreadable).

### STT call chain for one segment (reference for the new flush path)

`KlarvoOverlayService.kt`: `onSilenceTriggered()` (1504-1533, AUTOSTOP/AUTO only today) →
`stopAndProcessRecording(pauseSignalMs)` (1586) → `processAudio(wavBytes, pauseSignalMs)` (1629) →
`transcribeWithRetry(wavBytes, ...)` wrapping `GroqSttBridge.nativeTranscribe` (1785-1793) →
benchmark log (1826-1829, `[benchmark-11-1]` tag — leave this instrumentation in place, it's
reusable evidence) → `GroqSttBridge.nativeIsHallucination(transcript)` (1833) → LLM cleanup
(~1850-1870, **the new preview flush must skip this step** — raw text only, FR1 parity) → paste
(~1952-1953). The new `flushPreviewDelta()` (Task 2.4) reuses `transcribeWithRetry` +
`nativeIsHallucination` only, never the cleanup or paste calls.

### Testing pattern to mirror for new pure Kotlin logic

`RecordingMode.selectSilenceSecs` (`KlarvoOverlayService.kt:146`, companion object function, no
Android `Context` needed) is the existing template for "pure function → directly JVM-testable"
(its test: `android/kotlin-test/com/klarvo/voice/RecordingModeSilenceSelectionTest.kt`). Both new
guard functions in this story (`shouldInstallPreviewFlush`, any delta-marker math) must follow this
same shape — no `Context`, no `WindowManager`, no I/O in the tested unit.

### Instrumentation from 11-1 stays

The `[benchmark-11-1]` pause-to-text log lines in `KlarvoAudioRecorder.kt` and
`KlarvoOverlayService.kt` are pre-existing, non-regressing instrumentation (11-1, `done`) — leave
them in place; they remain useful for verifying this story's real-device smoke shows the same
sub-1s latency class end-to-end with the new repeatable path.

### Project Structure Notes

- Kotlin changes: `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` (new repeatable
  callback + delta snapshot), `KlarvoOverlayService.kt` (guard function, install/flush logic,
  accumulator, clear-on-finish), `ListeningPanelView.kt` (auto-scroll, appearance application),
  `KlarvoApi.kt` (new config reads).
- New JVM tests: `android/kotlin-test/com/klarvo/voice/` (new files or additions, following
  `RecordingModeSilenceSelectionTest.kt`'s pattern).
- Frontend: `src/components/settings/types.ts` (relax `desktopOnly`), possibly
  `src/components/settings/AppearanceContent.tsx` if the width-preset elicitation answer requires a
  platform-conditional prop.
- **No Rust changes.** All config plumbing already exists (Epics 5/6, `done`).
- No changes to `FloatingBubbleView.kt`, the ➤/✗ cluster, or HOLD dock visuals (9-12..9-16 territory,
  explicitly out of scope).

### References

- `_bmad-output/implementation-artifacts/11-1-android-live-preview-feasibility-benchmark.md` —
  benchmark result (GO, median 786ms), architecture decision (Groq delta-STT, ~2×), instrumentation
  already in place.
- `docs/backlog.md` §"Epic 11 — Cross-Platform Live-Preview (Android)" — epic kickoff, architecture
  decision record.
- `_bmad-output/implementation-artifacts/5-1-backend-pause-triggered-delta-flush-for-toggle-hold.md`
  — desktop delta-flush architecture (the pattern this story ports); "Key Constraint: Repeatable
  Silence Callback" Dev Note is the direct analog of this story's AC-2/Task-1.
- `_bmad-output/implementation-artifacts/5-3-settings-opt-in-preview-toggle-and-preview-pause-slider.md`
  — desktop Settings toggle/slider wiring (now superseded in location by 6.6/6.3, but the
  config-field mechanics are unchanged).
- `_bmad-output/implementation-artifacts/6-6-preview-box-appearance-customization.md` — themes/color
  pickers/font-family; "separate-window reactive read" lesson (read config fresh at show-time, not
  cached) applies directly to `ListeningPanelView` here.
- `_bmad-output/implementation-artifacts/6-3-font-size-axis-preview-font-size-config-settings-picker-k-scaling.md`
  — `previewFontSize` field + desktop's `FONT_PX_MAP` (`small`:11, `medium`:13, `large`:15).
- `src-tauri/src/config/mod.rs:730-790,1001-1032` — all 11 preview `AppConfig` fields + defaults
  [Source: src-tauri/src/config/mod.rs].
- `src/components/settings/types.ts:37-42` — `"appearance"` category, `desktopOnly: true`
  [Source: src/components/settings/types.ts].
- `src/components/settings/SettingsHome.tsx:27` — the filter to relax
  [Source: src/components/settings/SettingsHome.tsx].
- `src/components/settings/AppearanceContent.tsx` — the component to reuse as-is
  [Source: src/components/settings/AppearanceContent.tsx].
- `src/components/settings/previewAppearance.ts` — `PREVIEW_THEMES`, `PREVIEW_FONTS`,
  `rgbaToHexOpacity`/`hexOpacityToRgba` helpers (conceptual reference for the new Kotlin rgba
  parsing needed in Task 5.2) [Source: src/components/settings/previewAppearance.ts].
- `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt:123-130,213-260,382-419` —
  `onSilenceDetected`/`silenceCallbackFired`/`feedVad`/`pcmBuffer`/`stop()`/`encodeWav`
  [Source: android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt].
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:107-160,340-380,1430-1470,1504-1533,
  1586-1670,1785-1833,2030-2097` — `RecordingMode`, panel show/hide, mode-dispatch + VAD install
  site, `onSilenceTriggered`, `stopAndProcessRecording`/`processAudio`, STT+hallucination-filter
  chain [Source: android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt].
- `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt:33-410` — panel structure
  (`TopRowView`/`transcriptTextView`/`FooterView`/`CaretView`/`GripView`), "Modell B" passive-panel
  comments confirming no waveform/buttons today
  [Source: android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt].
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt:234-366` — `readConfig()` camelCase pattern
  [Source: android/kotlin-src/com/klarvo/voice/KlarvoApi.kt].
- `android/kotlin-src/com/klarvo/voice/MainActivity.kt:37` — `TauriActivity`, shared React Settings
  [Source: android/kotlin-src/com/klarvo/voice/MainActivity.kt].
- `android/kotlin-test/com/klarvo/voice/RecordingModeSilenceSelectionTest.kt` — pure-function JVM
  test pattern to mirror for new guard functions.
- `_bmad-output/project-context.md` — ADR-0016 Android/Rust parity rule, camelCase config keys,
  Android real-device smoke gate, no `FLAG_NOT_TOUCHABLE`.

## Dev Agent Record

### Agent Model Used

Claude Sonnet 4.5 (claude-sonnet-5), via `bmad-dev-story` skill.

### Debug Log References

- `scripts/android-smoke.sh` run (2026-07-01): KlarvoTheme.kt drift-gate ok, 17 Kotlin
  production files + 13 test files synced, `:app:testUniversalDebugUnitTest` → 134 tests total
  across all suites for the `universalDebug` flavor (13 new: 2 `DeltaSnapshotSliceTest` + 2
  `PreviewPauseFramesTest` + 9 `ShouldInstallPreviewFlushTest`), 0 failures, APK built + installed
  on Andi's real device (`100.112.41.70:5555`), versionName verified on-device.
- `npx tsc --noEmit` and `npm run build` (Node 20 via nvm) both clean — Appearance category
  gate relaxation + `AppearanceContent.tsx`/`SettingsPanel.tsx` touches compile and build.
- `cargo test` not run: no Rust files touched (story scope is Kotlin + shared-React only, per
  Dev Notes "zero Rust changes" — confirmed, no `.rs` file appears in the File List below); a
  Rust toolchain was also not available in this dev sandbox.

### Completion Notes List

- **AC-1/AC-2 (repeatable pause-flush + widened VAD gate):** `KlarvoAudioRecorder.processVadFrame`
  was restructured so the top-of-function `if (silenceCallbackFired) return` guard (which used to
  block ALL further VAD processing forever after the one-shot fires) is removed; the one-shot
  AUTOSTOP/AUTO path is now scoped to its own `if (onSilenceDetected != null && !silenceCallbackFired)`
  block, and a fully independent, repeatable `onPreviewPause` edge runs alongside it with its own
  hangover counter (`previewSilentFrames`) and per-silence-period re-arm flag
  (`previewFiredThisSilence`, reset on speech resume) — this is the exact trap Dev Notes/AC-2
  warned about (the mode-level one-shot silently killing preview after the first pause once the
  VAD gate was widened for HOLD/TOGGLE).
- **Task 2.3a (preview slider must not be inert):** extracted a pure `framesForSeconds(secs)`
  companion function, used by BOTH `requiredSilentFrames` (one-shot) and
  `previewRequiredSilentFrames` (repeatable preview edge) with independent inputs
  (`silenceSecs` vs. `previewPauseSilenceSecs`) — no conflict/escalation needed; the two windows
  are genuinely independent per-instance fields on `KlarvoAudioRecorder`. Test
  `PreviewPauseFramesTest.largerPreviewPauseSilenceSecs_yieldsLargerRequiredSilentFrames` confirms
  the slider is load-bearing; inversion documented in the same file.
- **Thread-safety:** `deltaSnapshotWav()` is called from the main thread (via `handler.post`)
  while the recording thread concurrently appends to `pcmBuffer` — added `synchronized(pcmBuffer)`
  around both the append loop and the delta-snapshot read/clear to avoid a genuine (not merely
  theoretical) cross-thread data race that the single-threaded pre-11-2 code never had.
- **AC-7 (clear-on-finish placement):** placed the `previewAccumulatedText = ""` reset in
  `stopAndProcessRecording()` right after the `Thread { ...; processAudio(...) }.start()` call
  (not inside `processAudio()`), so `processAudio`'s paste chain itself is untouched — recording
  has already stopped and `audioRecorder` is already nulled at that point, so no further preview
  flush can land after this reset.
- **AC-9 (font-family → Typeface):** verified (per Dev Notes' explicit instruction to check
  before assuming) that no `R.font.*` / Geist reference exists anywhere in `android/kotlin-src`
  today — the Geist `.ttf` assets are copied by `android-smoke.sh` but never loaded by any Kotlin
  code. Mapped Inter + System UI → `Typeface.DEFAULT` (no regression: this is what the panel
  already used), Serif → `Typeface.SERIF`, Monospace → `Typeface.MONOSPACE`. No new font asset
  wiring was introduced (kept in scope; a native Inter/Geist Typeface is a separate, un-requested
  change).
- **Inversions confirmed (DoD):**
  - `ShouldInstallPreviewFlushTest.inversion_autostopMustNotEqualHoldBehavior` — flips the
    mode-guard expectation; would go RED if AUTOSTOP/AUTO were ever allowed to install a preview
    flush.
  - `DeltaSnapshotSliceTest.skippingMarkerAdvance_causesOverlap_provingMarkerAdvanceIsLoadBearing`
    — re-uses a stale marker and asserts the resulting overlap, proving the marker-advance step in
    `deltaSnapshotWav()` is load-bearing.
  - `PreviewPauseFramesTest.inversion_hardCodedThresholdWouldFailThisTest` — proves a hard-coded
    frame count (ignoring the input seconds) would fail the "larger secs → larger frames" test.
  - The AC-2 reviewer inversion described in the DoD (temporarily reusing the one-shot
    `onSilenceDetected` for preview) was reasoned through structurally rather than re-run as a
    throwaway code mutation: `onSilenceDetected` sets `silenceCallbackFired = true` permanently on
    first fire, and (pre-fix) `feedVad` stopped once that flag was true for AUTOSTOP/AUTO's own
    gate — confirmed by reading the original gate at `KlarvoAudioRecorder.kt:250` before this
    story's edit; this is exactly why a *separate* `onPreviewPause` callback with its own
    independent counters (not reusing `silenceCallbackFired`) was required, not optional.
- **Real-device smoke:** `scripts/android-smoke.sh` ran a clean build/install against Andi's real
  device (Task 6.3) — 0 test failures, fresh APK confirmed. The interactive real-recording smoke
  (Task 6.4: HOLD/TOGGLE dictation with preview on, watch panel accumulate, confirm keyboard
  overlap has no avoidance jank, confirm Finish still pastes + clears preview, confirm the
  Settings toggle off returns to byte-identical behavior) is Andi's own action per this story's
  DoD and the project's Android real-device-gate rule — not run by this dev pass.

### File List

- `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` — `onPreviewPause` callback slot,
  delta marker + `deltaSnapshotWav()`, pure `sliceSince`/`framesForSeconds` companion functions,
  widened `feedVad` gate, restructured `processVadFrame` (independent preview hangover counter),
  `pcmBuffer` thread-safety.
- `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` — `RecordingMode.shouldInstallPreviewFlush`,
  `previewAccumulatedText` accumulator, preview-flush install site in `startRecording()`,
  `flushPreviewDelta()`/`appendPreviewText()`, clear-on-finish/-cancel, appearance application at
  panel show-time.
- `android/kotlin-src/com/klarvo/voice/ListeningPanelView.kt` — `ScrollView` wrapper + auto-scroll
  on `rawTranscript` append, `applyAppearance()` + `parseRgba`/`typefaceForFontFamily` pure helpers.
- `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` — 10 new `Config` fields (live-preview +
  appearance) + `readConfig()` reads, camelCase, defaults matching Rust serde defaults exactly.
- `android/kotlin-test/com/klarvo/voice/DeltaSnapshotSliceTest.kt` — new (AC-1/Task 1.3).
- `android/kotlin-test/com/klarvo/voice/PreviewPauseFramesTest.kt` — new (Task 2.3a).
- `android/kotlin-test/com/klarvo/voice/ShouldInstallPreviewFlushTest.kt` — new (AC-3/AC-4/Task 2.2).
- `src/components/settings/types.ts` — removed `desktopOnly: true` on the `"appearance"` category
  (Task 4.1).
- `src/components/settings/AppearanceContent.tsx` — new `hidePanelForm` prop, hides the
  "Darstellung" width-preset picker when set (Task 4.3, GATE 1 decision).
- `src/components/SettingsPanel.tsx` — passes `hidePanelForm={!isDesktop}` to `AppearanceContent`.

## Change Log

| Date | Change |
|------|--------|
| 2026-07-01 | Story created (bmad-create-story) from Epic 11 kickoff (docs/backlog.md) + 11-1 benchmark decision + desktop Epics 5/6 architecture reference. Status: ready-for-dev. |
| 2026-07-01 | GATE 1 (story-conductor): the one open elicitation item — how `previewPanelForm` (width preset) maps onto Android's MATCH_PARENT-width panel — RESOLVED by Andi: HIDE the width preset control on Android (show only toggle + pause + color/font). Pinned into Task 4.3 + AC-8. |
| 2026-07-01 | Dev implementation complete (bmad-dev-story): all 6 tasks done. Kotlin: repeatable pause-flush primitive + widened VAD gate (AC-1/AC-2), install/guard logic + independent preview-pause frame threshold (AC-3/AC-4/Task 2.3a), panel accumulate/auto-scroll (AC-5), Settings config reads (AC-8), appearance application (AC-9). Frontend: relaxed `desktopOnly` gate, hid width-preset control on Android (Task 4.3). 3 new JVM test files / 13 new test methods (134 tests total across all suites), all green; `npm run build`/`tsc` clean; `scripts/android-smoke.sh` clean build/install on real device. Status: review. |
