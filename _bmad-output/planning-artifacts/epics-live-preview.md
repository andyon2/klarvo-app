---
stepsCompleted: ["step-01-validate-prerequisites", "step-02-design-epics", "step-03-create-stories", "step-04-final-validation"]
status: complete
inputDocuments:
  - docs/feature-ideas.md  # "Live-Cleanup-Preview" block — ✅ ENTSCHIEDEN 2026-06-03 + 7 resolved forks
  - _bmad-output/project-context.md
  - docs/adr/0016-android-path-parity-strategy.md  # NFR3 cross-platform config safety
trackType: brownfield-feature
featureEpic: 5
note: >
  Separate planning artifact by design — epics.md is the CLOSED robustness-remediation
  breakdown (Epics 1-4). This is the first FEATURE epic (Epic 5), built via the L3 feature
  route. There is no PRD/Architecture/UX doc; the requirements source is the resolved
  decision block in docs/feature-ideas.md (the WAS/WARUM + all 7 detail forks closed this
  session, grounded in a current-code audit). Shares the sprint-status.yaml ledger.
---

# klarvo - Epic Breakdown (Live-Cleanup-Preview · Epic 5)

## Overview

The first **feature** epic on `v1-ship` (Epics 1-4 were robustness remediation). It adds an
**opt-in orientation preview**: while dictating in **Toggle/Hold**, speech-pause-triggered raw
Groq segments accumulate in an auto-expanding FloatingBar panel so the user can read along
during a long dictation. The preview is **purely display** — it never feeds the pasted output
(Variant B: at finish the existing whole-buffer path runs unchanged). German-accuracy risk is
therefore out of scope by construction.

There is no PRD/Architecture/UX document. Requirements below are extracted from the resolved
decision in `docs/feature-ideas.md` ("✅ ENTSCHIEDEN 2026-06-03" + the 7 closed forks **D1–D7**)
and grounded in a current-code audit performed this session. IDs are kept native (FR/NFR + the
D-fork they trace to) so traceability back to the decision stays intact.

**L3 guards (carried into every story):** (G-A) a characterization test pinning the *existing*
behavior BEFORE the additive code is written; (G-B) runtime integration lives **in the
acceptance criteria**, not just unit-green. Surface/UI stories require a **Windows release build
+ manual press-to-paste smoke** in the DoD (project-context.md testing rules).

## Requirements Inventory

### Functional Requirements

- **FR1 (D1, D2)** — While recording in **Toggle or Hold** with Preview enabled, on each
  detected speech pause ≥ the Preview-Pause threshold, the audio **delta since the last pause**
  is transcribed via Groq as **raw text** (no per-segment cleanup) and **appended** to an
  accumulating preview. Recording continues — the flush does **not** stop recording, does **not**
  paste, and does **not** loop.
- **FR2 (D5)** — The accumulated preview renders in the FloatingBar as **Variant 1**: the pill
  **auto-expands** downward into a **scrollable text panel** (fixed max-height) on the first
  chunk and **auto-scrolls to the newest** text.
- **FR3 (D2)** — The preview is **display-only and never feeds the output**. At **Finish** (key
  release / 2nd tap / shortcut), the **existing finish path runs unchanged**: whole WAV →
  `process_audio` → cleanup → **single paste**. Output for Toggle/Hold is **byte-identical to
  today**.
- **FR4 (D7)** — The preview is active **only in Toggle and Hold**. **Auto and AutoStop never
  show a preview feed** (they already transcribe+paste per segment; a feed would double).
- **FR5 (D4)** — The preview is **disabled in the offline/local-STT path** (`stt_provider ==
  "local"`): no Groq flush fires there. Waveform feedback remains.
- **FR6** — The preview is **opt-in** via a **Settings toggle** (default **off**).
- **FR7 (D6)** — At Finish the accumulated preview **clears with the done-pop** (does not
  persist for review).
- **FR8 (D3 — Regler A)** — A new general **"Preview-Pause"** slider in the Shortcut settings
  section sets the Preview-Pause threshold, stored in a **new** config key
  `preview_pause_silence_secs` (default **2.0**). Drives FR1's flush timing for Toggle+Hold.
- **FR9 (D3 — Regler B)** — A single general **"Send/Stop-Pause"** slider in the Shortcut
  settings section replaces the two per-mode controls and writes **both existing** keys
  `auto_mode_silence_secs` **and** `autostop_silence_secs` to the **same** value. **No key is
  renamed or removed.**

### NonFunctional Requirements

- **NFR1 (cost — the core constraint)** — The flush transcribes **only the delta** since the
  last pause, **never the growing whole buffer**. This is the exact failure of the old
  live-preview poller (3 s poll → `snapshot_wav()` of the whole buffer → re-transcribe →
  "10-20x Groq quota", which got it disabled). Per-segment STT during recording must total
  **~1× audio**, not N×. (Variant B's finish re-Groq adds the documented, accepted **~2× total**
  — segments + one whole-buffer pass — at finish.)
- **NFR2 (no-regression)** — The finish/paste path is **not touched**. The preview is a parallel,
  additive display path. Disabling Preview ⇒ behavior is exactly today's.
- **NFR3 (cross-platform config safety — ADR-0016)** — No existing `*_silence_secs` key is
  renamed/removed. Android (`KlarvoOverlayService.kt`) keeps reading `auto_mode_silence_secs` +
  `autostop_silence_secs` unchanged. `preview_pause_silence_secs` is **desktop-only**; Android
  ignores it. **Zero migration** of existing `config.json` keys.
- **NFR4 (event naming — G3)** — The new preview-chunk event uses **colon** form
  (`klarvo://live-preview-chunk`), never dots (Tauri reserves `.`).
- **NFR5 (threading)** — The pause-flush dispatches **async, off** the cpal OS audio-callback
  thread, non-blocking — like the existing pipeline.
- **NFR6 (BYOK / no telemetry)** — The preview adds **no** network calls beyond the user's
  configured Groq STT endpoint.

### Additional Requirements (from the current-code audit)

- **AR1 — Delta-snapshot primitive.** `audio/mod.rs:416 snapshot_wav()` returns the **whole**
  accumulated buffer (no marker/cursor). The feature needs an audio-since-last-pause slice:
  a sample-position marker captured at each pause + a slice→WAV encode. This is **net-new** and
  is the load-bearing backend primitive.
- **AR2 — Flush-without-stop silence callback for Toggle/Hold.** Today Toggle/Hold have **no**
  silence detection (`pipeline.rs:2008-2023` — stop only on user action). Auto/AutoStop's silence
  callbacks **stop** (and Auto loops). The feature installs a **new** callback in Toggle/Hold that
  fires the delta-flush and **keeps recording**.
- **AR3 — Push, not poll.** The old `FloatingBar.tsx` preview (commented at :389-405) **polled**
  the live `transcribe_live_preview` command (still live at `commands/recording.rs:346`) every
  3 s and whole-buffered. The feature replaces poll-whole-buffer with **event-push of deltas**
  (`klarvo://live-preview-chunk`). Re-enable the `livePreview` state (`FloatingBar.tsx:218`).
- **AR4 — Bar window resize for the panel.** The bar window `setSize`/`setBarShape("pill")` logic
  (`FloatingBar.tsx:280-308`) sizes a fixed 200×36 pill; Variant 1 needs a taller panel size +
  shape and a resize-back on collapse.
- **AR5 — Settings surface.** Add the opt-in toggle + Regler A/B sliders to the Shortcut section
  of the settings UI and persist via the single sanctioned `save_config` write path (ADR-0015 /
  Story 4-3); the React strings live in the existing settings UI.

### UX Design Requirements

- **UX-DR1 — Variant 1 (Auto-Expand-Panel).** Decided by Andy 2026-06-03 from rendered mockups
  (`/tmp/klarvo-mockups/bar.png`, faithful to `FloatingBar.tsx`): pill grows into a scrollable
  panel, fixed max-height, auto-scroll to newest, top-fade for scrolled-off text, thin
  scroll-indicator. Recording-accent border/teal logo/waveform unchanged. Collapses back to the
  pill on done-pop (FR7).

### FR Coverage Map

- **FR1** → Epic 5 (Story 5.1) — pause-triggered delta Groq flush, no stop/paste/loop
- **FR2** → Epic 5 (Story 5.2) — auto-expand scrollable panel (Variant 1)
- **FR3** → Epic 5 (Story 5.1, characterization) — finish path unchanged; preview never feeds output
- **FR4** → Epic 5 (Story 5.1) — Toggle/Hold only; Auto/AutoStop excluded
- **FR5** → Epic 5 (Story 5.1) — disabled in offline/local-STT path
- **FR6** → Epic 5 (Story 5.3) — opt-in Settings toggle (default off)
- **FR7** → Epic 5 (Story 5.2) — preview clears with done-pop
- **FR8** → Epic 5 (Story 5.3) — Regler A new key `preview_pause_silence_secs`
- **FR9** → Epic 5 (Story 5.4) — Regler B one slider → both existing keys (separable / deferral seam)

All NFR1–NFR6 and AR1–AR5 are cross-cutting within Epic 5 (see per-story ACs).

## Epic List

### Epic 5: Live-Cleanup-Preview

When dictating a long passage in **Toggle or Hold**, the user can turn on an **opt-in live
preview**: raw Groq segments accumulate at speech pauses in an **auto-expanding FloatingBar
panel**, so they can **read along and spot errors before finishing** — while the **final pasted
text is produced exactly as today** (the preview never feeds the output). Standalone: builds only
on existing v1 recording/pipeline/FloatingBar surfaces; enables no future epic but closes the
long-parked "Live-Overlay" feature.

**FRs covered:** FR1, FR2, FR3, FR4, FR5, FR6, FR7, FR8, FR9
**NFRs:** NFR1–NFR6 · **AR:** AR1–AR5 · **UX:** UX-DR1

**Planned story decomposition** (detailed in Step 3 — shown here for shape review):

- **5.1 — Backend: delta-flush core** *(Wave 1, foundation)*. AR1 delta-snapshot primitive +
  AR2 flush-without-stop silence callback for Toggle/Hold + raw Groq segment transcribe +
  `klarvo://live-preview-chunk` event (NFR4). Scope guards FR4 (Toggle/Hold only) + FR5 (offline
  off). **G-A characterization test FIRST:** pin today's Toggle/Hold finish path (stop →
  `process_audio` → single paste) so FR3/NFR2 no-regression is provable. Covers FR1, FR3, FR4,
  FR5, NFR1, NFR2, NFR4, NFR5.
- **5.2 — Frontend: auto-expand preview panel** *(depends on 5.1's event)*. AR3 push-not-poll
  accumulation + AR4 bar window resize + Variant 1 panel (UX-DR1) + clear-on-done. Surface story
  → Windows release build + manual press-to-paste smoke in DoD. Covers FR2, FR7.
- **5.3 — Settings: opt-in toggle + Preview-Pause slider (Regler A)**. FR6 toggle (default off)
  gating the whole feature + FR8 new key `preview_pause_silence_secs` via the sanctioned
  `save_config` path (ADR-0015). Surface story → smoke. Covers FR6, FR8.
- **5.4 — Config: Send/Stop-Pause consolidation (Regler B)** *(separable — the deferral seam)*.
  FR9 one slider writes both existing `auto_mode_silence_secs` + `autostop_silence_secs`; **no
  key rename/removal**, Android unaffected (NFR3, ADR-0016). Independent of 5.1–5.3 — can ship,
  defer, or drop without touching the preview. Covers FR9, NFR3.

**Dependency flow:** 5.1 → 5.2; 5.3 parallel to 5.2 (independent surfaces); 5.4 fully
independent. No story depends on a *later* story.

## Epic 5: Live-Cleanup-Preview

When dictating a long passage in Toggle or Hold, the user can enable an opt-in live preview:
raw Groq segments accumulate at speech pauses in an auto-expanding FloatingBar panel so they can
read along and spot errors before finishing — while the final pasted text is produced exactly as
today (the preview never feeds the output).

### Story 5.1: Backend — pause-triggered delta-flush for Toggle/Hold

As a developer extending the recording pipeline,
I want a delta-snapshot + flush-without-stop path that transcribes only the new audio since the
last pause and emits it as a preview chunk in Toggle/Hold,
So that the live preview can accumulate raw text at ~1× STT cost without touching the finish/paste path.

**Acceptance Criteria:**

**Given** the current v1 Toggle and Hold finish behavior (stop → `process_audio` → single paste)
**When** a characterization test drives a fixed WAV fixture through the Toggle and Hold finish path with Preview disabled
**Then** it pins the produced `cleaned_text`/`raw_text` and single-paste outcome as a golden assertion
**And** this test is written and green BEFORE any preview code is added — it is the G-A no-regression baseline for FR3/NFR2 (the L3 "characterization-test-before-touching-existing-code" guard).

**Given** a recording in progress with an accumulating live buffer (`audio/mod.rs` live_buffer)
**When** the new audio API is asked for a delta snapshot at a pause boundary
**Then** it returns a WAV of only the samples captured since the previous delta marker (not the whole buffer, unlike today's `snapshot_wav()`)
**And** it advances the marker so the next delta starts where this one ended
**And** a unit test on a synthetic sample stream asserts two consecutive deltas are disjoint and together equal the full buffer (NFR1 — proves ~1× not N×).

**Given** Story 5.1 owns the two new `AppConfig` fields — `live_preview_enabled` (default `false`) and `preview_pause_silence_secs` (default `2.0`) — added with serde defaults so 5.1 is self-contained (no UI yet → flush never fires for a real user until Story 5.3 wires the toggle; tests set the fields directly)
**When** the schema is loaded
**Then** both fields read with their defaults and trigger NO migration write (additive defaults), so 5.1 has no forward dependency on 5.3.

**Given** Toggle or Hold mode is active, `live_preview_enabled == true`, and `stt_provider != "local"`
**When** a speech pause ≥ `preview_pause_silence_secs` is detected
**Then** the delta segment is transcribed via the configured Groq STT provider as raw text (no per-segment cleanup, FR1/D1)
**And** recording continues uninterrupted — no stop, no paste, no auto-loop
**And** the raw segment text is emitted on event `klarvo://live-preview-chunk` as an append payload (NFR4 — colon form, never dots).

**Given** the pause is detected on the cpal OS audio-callback thread
**When** the flush is triggered
**Then** the Groq transcription runs on an async task off the callback thread (non-blocking), mirroring the existing pipeline dispatch (NFR5).

**Given** Auto or AutoStop mode (not Toggle/Hold)
**When** a pause is detected
**Then** the existing per-segment stop/paste/loop behavior runs unchanged
**And** NO `klarvo://live-preview-chunk` event is emitted (FR4 scope guard — no double feed).

**Given** `stt_provider == "local"` (offline path, `is_offline()` true, `pipeline.rs:450`)
**When** recording in Toggle/Hold with Preview enabled
**Then** no delta flush fires and no chunk event is emitted (FR5 — preview disabled offline)
**And** waveform feedback is unaffected.

**Given** a delta-segment Groq transcription fails (network / 429 / 5xx) mid-recording
**When** the flush completes
**Then** the failing chunk is skipped (no append, or an explicit empty-skip payload), recording continues, and no error is surfaced to the user mid-stream (fail-soft — matches the existing `transcribe_live_preview` error-swallow at `commands/recording.rs`).

**Given** any number of preview chunks were emitted during a Toggle/Hold recording
**When** the user finishes (release / 2nd tap / shortcut)
**Then** the finish path runs the existing whole-WAV → `process_audio` → single paste, unchanged (FR3)
**And** the AC-1 characterization test still passes — output byte-identical to Preview-off (NFR2).

**DoD:** Backend story. Linux `cargo test` (characterization + delta-snapshot unit + guard logic) + `clippy` clean on touched files. End-to-end runtime is exercised by Story 5.2's smoke gate (the event has no user-visible effect until the frontend consumes it).

### Story 5.2: Frontend — auto-expand preview panel (Variant 1)

As a user dictating a long passage in Toggle or Hold,
I want the FloatingBar to grow into a scrollable panel that accumulates the preview text and auto-scrolls to the newest line,
So that I can read along and spot errors before I finish.

**Acceptance Criteria:**

**Given** Preview is enabled and a recording is active in Toggle/Hold
**When** `klarvo://live-preview-chunk` events arrive
**Then** each chunk's raw text is appended to an accumulating preview string in the bar (push, not poll — AR3)
**And** the old 3 s-poll `transcribe_live_preview` caller stays removed (the commented block at `FloatingBar.tsx:389-405` is NOT re-enabled as a poller; the `livePreview` state at :218 is re-enabled as a push sink).

**Given** the first preview chunk arrives
**When** the bar renders
**Then** the pill auto-expands downward into a scrollable text panel — Variant 1: fixed max-height, top-fade for scrolled-off text, thin scroll-indicator, recording-accent border, teal logo + waveform retained (UX-DR1, FR2)
**And** the panel auto-scrolls to the newest text as chunks append.

**Given** the panel expands or collapses
**When** the bar window is resized
**Then** `setSize` + `setBarShape` are applied before `show`, preserving the white-line shape-guard ordering (`FloatingBar.tsx:280-308`)
**And** drag/position persistence (`saveBarPosition`/`getBarPosition`) is NOT regressed — manual smoke confirms drag still works while the panel is expanded (AR4 edge guard).

**Given** the user finishes (done state)
**When** the done-pop fires
**Then** the accumulated preview clears and the bar collapses back to the pill/done-pop with no lingering panel (FR7).

**Given** Preview is disabled (default)
**When** recording in any mode
**Then** no panel appears and the bar behaves exactly as today — pill + waveform (FR6 interaction, NFR2).

**Given** a chunk event carries an empty/skip payload (from a failed segment flush, 5.1 fail-soft)
**When** the bar processes it
**Then** nothing is appended and the panel does not flicker or error.

**DoD:** Surface story → **Windows release build + manual press-to-paste smoke**: dictate a multi-pause passage in Toggle with Preview on, watch the panel accumulate and auto-scroll, finish, confirm the correct single paste lands AND the panel clears. Linux `cargo test` + `tsc`/`npm run build` + `clippy` on touched.

### Story 5.3: Settings — opt-in Preview toggle + Preview-Pause slider (Regler A)

As a user,
I want a Settings toggle to turn the live preview on/off and a Preview-Pause slider to set how long a pause triggers a flush,
So that the preview is opt-in and I can tune its responsiveness.

**Acceptance Criteria:**

**Given** the Shortcut section of the settings UI, and the `live_preview_enabled` field already exists in `AppConfig` (introduced by Story 5.1)
**When** the user toggles "Live Preview"
**Then** the existing `live_preview_enabled` field is written via the single sanctioned `save_config` write path (ADR-0015 / Story 4-3 — no second writer)
**And** the value gates the Story 5.1 flush (off → no flush, no event, FR6).

**Given** the Shortcut section
**When** the user adjusts the "Preview-Pause" slider (Regler A)
**Then** the `preview_pause_silence_secs` field (introduced in 5.1; range matching the existing silence sliders, e.g. 0.5–5.0) is written via `save_config`
**And** Story 5.1's flush uses this value as the pause threshold (FR8/D3).

**Given** a fresh or existing `config.json` with neither field set by the user
**When** it is loaded
**Then** `live_preview_enabled` reads `false` and `preview_pause_silence_secs` reads `2.0` via the serde defaults from 5.1
**And** NO migration write is triggered (additive defaults only — existing users see zero behavior change, NFR2).

**Given** the Preview-Pause slider
**When** it is shown
**Then** it carries the trade-off hint: short = more responsive + more Groq calls + shorter context per segment; long = less responsive + fewer calls + better context (decision doc).

**DoD:** Surface story → **Windows release build + manual smoke**: toggle on, set the slider, confirm the flush timing changes; toggle off, confirm the preview is gone. `tsc` build + `cargo test`.

### Story 5.4: Config — Send/Stop-Pause consolidation (Regler B)

As a user,
I want a single "Send/Stop-Pause" slider instead of two separate per-mode controls,
So that the Shortcut settings are simpler — without breaking any platform that reads the underlying keys.

**Acceptance Criteria:**

**Given** the Shortcut section
**When** the user adjusts the single "Send/Stop-Pause" slider (Regler B)
**Then** BOTH existing keys `auto_mode_silence_secs` AND `autostop_silence_secs` are written to the same value via `save_config` (FR9/D3)
**And** the two prior separate per-mode controls are removed from the UI.

**Given** the `AppConfig` schema
**When** Story 5.4 ships
**Then** neither `auto_mode_silence_secs` nor `autostop_silence_secs` is renamed or removed
**And** no config migration is added — the keys keep their identity and defaults (NFR3 — zero migration risk).

**Given** Android reads `auto_mode_silence_secs` + `autostop_silence_secs` mode-centrically (`KlarvoOverlayService.kt:807-808`)
**When** the desktop UI consolidation ships
**Then** a test or documented verification confirms both keys still exist with unchanged names/defaults so the Kotlin reads are unaffected
**And** NO Android code change is required by this story (ADR-0016 parity; [[android_silence_field_divergence]] guard).

**Given** the two keys currently hold different values (edge: hand-edited `config.json`)
**When** the consolidated slider opens
**Then** it displays one defined value (the larger of the two) and writing re-unifies both — documented behavior, not a silent pick.

**DoD:** Config-surface story → **Windows release build + manual smoke**: move the slider, confirm both keys change in `config.json`, confirm Auto + AutoStop still silence-stop at the new value. `cargo test` for the write-both behavior.
