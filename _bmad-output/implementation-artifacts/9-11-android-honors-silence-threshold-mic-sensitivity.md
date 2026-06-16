# Story 9.11: Android honors the silence_threshold (mic sensitivity) setting

Status: ready-for-dev

## Story

As a user on Android,
I want the app to honor the "Silence threshold" sensitivity setting (and ship a sane default),
so that quiet speech is not silently dropped and I can tune sensitivity to my environment myself.

## Context — root cause (device-evidenced, Story 9-7 follow-up, 2026-06-16)

On-device VAD telemetry (instrumented build, real Xiaomi) showed Auto mode mostly works but ~10% of
quiet utterances are missed: the Silero VAD **detects** the speech (`vadTrue=31`), but Android's RMS
energy pre-gate vetoes most frames, so the 3-consecutive-frame speech onset never confirms and that
utterance gets merged into the next louder one instead of auto-flushing.

The pre-gate is **hard-coded** `SILENCE_THRESHOLD = 0.02f` in `KlarvoAudioRecorder.kt:58`. The desktop
exposes this as a **user setting** (`advanced.silence_threshold`, slider "Silence threshold (advanced
audio setting)" in `ShortcutsContent.tsx`) with default **0.005** — Android's hard-coded 0.02 is **4× the
desktop default**, far too aggressive for this device's quiet mic (speech peaks ~0.026–0.037). Android
ignores the setting entirely — same divergence class as the old Android silence-field bug
(`[project_android_silence_field_divergence]`).

Device evidence: `/tmp/9-7-auto-vad.log` (this session) — see e.g. 18:40:50 `rmsMax=0.027 vadTrue=31
speechFrames=3 speechDetected=false onsetFrames=0` (VAD heard it; energy gate rejected it).

## Acceptance Criteria

**AC1 — Android reads `silence_threshold` from config instead of the hard-coded const.**
Given the config `advanced.silenceThreshold` value (JSON: `{"advanced":{"silenceThreshold":<f>}}` —
NOTE it is **nested** under `advanced`; Android's `KlarvoApi` currently reads only flat top-level keys,
so navigate the `advanced` object, e.g. `json.optJSONObject("advanced")?.optDouble("silenceThreshold", 0.005)`)
When a recording session starts
Then `KlarvoAudioRecorder`'s energy pre-gate uses that configured value, not `SILENCE_THRESHOLD = 0.02f`.
The hard-coded `0.02f` const is removed (or kept only as the fallback default if config is absent).

**AC2 — Default matches desktop (0.005).**
Given no user has changed the setting
When Android records
Then the energy gate defaults to **0.005** (= Rust `default_silence_threshold()`), not 0.02 — so out of
the box Android sensitivity matches desktop and the quiet-speech miss is gone by default.

**AC3 — The setting is live-adjustable on Android (the slider actually works).**
Given the user changes the "Silence threshold" slider in Settings and saves
When the next Android recording starts
Then the new value is read (via `loadBubbleControls()` / the per-session config read) and applied — the
user can tune sensitivity to their environment without a rebuild. (Verifikations-Symmetrie: the user can
produce both states — low and high threshold — from the slider.)

**AC4 — Multi-fire guard: silence fires at most once per recording session.**
Given silence is detected after speech
When the threshold frame is crossed mid audio-buffer
Then `onSilenceDetected` is invoked **exactly once** (today the `!silenceCallbackFired` check sits only at
the per-buffer level in `start()`, so remaining frames in the same buffer re-fire — observed 62→67 in the
device log; harmless today only because a downstream state guard swallows the extras). Add the
`silenceCallbackFired` (or equivalent) check inside `processVadFrame` so it cannot re-fire.

**AC5 — Tests.**
A JVM test (runs under `scripts/android-smoke.sh`) covering: (a) the energy-gate value is taken from the
configured threshold (independent expected values, not SUT-vs-itself); (b) the multi-fire guard — a frame
sequence that crosses the threshold and continues yields exactly one fire. Mirror the Rust↔Kotlin config
contract (ADR-0015/0016): `silenceThreshold` is camelCase and read via the single config path.

**DoD (device smoke — Andi's gate, states he can produce himself):**
- Default build: in Auto mode, speak a **quiet** sentence then pause → it now **auto-flushes on its own**
  (the 9-7 miss is gone at default 0.005).
- Move the "Silence threshold" slider up → quiet speech is gated again; move it down → caught. (Proves the
  slider is live.)
- `scripts/android-smoke.sh` exits 0 (build + JVM tests green).

## Out of scope (follow-up story)

The **sensitivity hint** — the app detecting the "VAD heard speech but the energy gate vetoed it" pattern
and proactively telling the user "speech detected but dropped as too quiet — adjust sensitivity?" — is a
separate story (9-12, see backlog). 9-11 is its precondition (the adjustable control).

## Dev Notes

- Energy gate: `android/kotlin-src/com/klarvo/voice/KlarvoAudioRecorder.kt` — const at :58, used in
  `processVadFrame()` (`energyAboveGate = normalizedRms >= SILENCE_THRESHOLD`) and the diagnostic config
  log at :195. Thread the configured value via the recorder constructor (alongside the existing
  `silenceSecs` param), set from `KlarvoOverlayService.startRecording()`.
- Config read: `KlarvoApi.kt` `readConfig()` — add the nested `advanced.silenceThreshold` read; carry it
  on the config object the service already consumes in `loadBubbleControls()`.
- Desktop reference: `src-tauri/src/config/mod.rs:111-112,209-211` (field + default 0.005);
  `src-tauri/src/pipeline.rs:475-481,636-710` (how desktop applies it to autostop/auto).
- Keep behavior parity: the gate is used in BOTH onset and hangover — lowering the default to 0.005 is the
  proven desktop value; the device log shows Silero returns `vadTrue=0` on true silence (rms ~0.001), so
  the hangover still fires.
- Leave the diagnostic `VAD ~1s` / `VAD config` logging in place (it is observation-only and useful) OR
  gate it behind a debug flag — dev's call, note it.

## Change Log

- 2026-06-16: Story created (story-conductor, off the 9-7 on-device finding). Root cause device-evidenced.
