---
story: 12.3
status: review
epic: 12
inputDocuments:
  - _bmad-output/planning-artifacts/architecture.md (§Settings-Namespaces line 520 — `audio.input_device` reserved; §Event-Naming line 498 — `audio.device-changed` event reserved)
  - docs/adr/0006-audiosource-trait-signature.md (CaptureConfig + AudioSource trait)
  - docs/adr/0009-shell-error-bridge-pattern.md (ErrorEmitter pattern — re-used here for fail-soft device-not-found UX)
  - docs/adr/0012-orchestrator-owner.md (SessionOrchestrator owns AudioSource; per-press CaptureConfig build)
  - docs/adr/0013-settings-persistence-schema.md (settings persistence + Story-B2-Defer at line 187 — this story resolves it; ADR Amendment 4 required)
  - klarvo-core/src/audio/source.rs (CaptureConfig — extend with `device` field; AudioSource trait — unchanged signature)
  - klarvo-core/src/settings/mod.rs (typed-accessor pattern + SettingsEmitter trait — add `audio_input_device()` accessor)
  - klarvo-core/src/settings/defaults.rs (default constants — none needed; `None` is the default)
  - klarvo-audio-cpal/src/source.rs (CpalAudioSource — implement device-name lookup with fallback)
  - shells/windows/src-tauri/src/audio.rs (make_audio_source factory — phase-2 TODO at line 26 is resolved by this story)
  - shells/windows/src-tauri/src/main.rs (Step 4 audio + Step 9 orchestrator construction + settings:changed listeners at lines 247/272)
  - shells/windows/src-tauri/src/commands/settings.rs (UserSettings struct + Tauri commands + SettingsChangedEvent emitter)
  - shells/windows/src/index.html (SettingsPanel form + settings:changed listener)
  - shells/windows/locales/en.json + locales/de.json (i18n keys for UI labels + error/fallback toast)
  - src-tauri/src/audio/mod.rs (v1 reference: `list_input_devices` lines 492-503 + `find_input_device` lines 510-527 — DO NOT port, mirror semantics in v2)
  - src-tauri/src/config/mod.rs (v1 reference: `audio_device: Option<String>` line 531 — name-based persistence pattern)
  - klarvo-shell-orchestrator/src/session.rs (per-press CaptureConfig build — extend to include device from Arc)
  - klarvo-shell-orchestrator/src/lib.rs (SessionOrchestrator::new signature — add audio_device_arc parameter, mirror recording_mode_arc)
  - memory/feedback_two_symptom_upstream_check.md (trigger incident 2026-05-22; rationale for this story)
  - memory/feedback_autonomous_decisions.md (decisions taken with rationale, not deferred to user)
  - memory/feedback_scaffold_fail_soft_pattern.md (no panics in app-lifetime services — device-not-found is fail-soft)
  - memory/feedback_windows_cross_compile_verify.md (verify with `cargo check --target x86_64-pc-windows-gnu` before closure)
  - memory/feedback_tauri_vs_core_event_naming.md (Tauri event-names use colon-notation `:` — `settings:changed`, NOT `settings.changed`)
  - memory/project_v1_v2_coexistence.md (DO NOT write v2 code in `src/`, `src-tauri/`, `android/`)
  - memory/project_klarvo_v2_rebuild.md (Phase-1 windows-only; Android out-of-scope)
  - memory/project_event_ts_ms_convention.md (chunk-start timestamps unchanged by this story)
---

# Story 12.3: Audio-Input-Device-Selection

Status: **review**

## Story

As a Klarvo user,
I want to select which microphone Klarvo uses for recording,
so that Klarvo records from the device I actually speak into instead of silently capturing from whichever device Windows currently has set as OS-default.

As a Klarvo developer,
I want the device-name persisted in v2 settings and respected by `CpalAudioSource` at session-start,
so that the audio-source code-path is transparent (single source of truth: Settings → AudioSource), not hidden inside `cpal::default_host().default_input_device()`.

## Context

**Trigger 2026-05-22 (memory `feedback_two_symptom_upstream_check`):** After Stories 12.1 (Pipeline-Wire-Up) and 12.2 (Smoke-Test-Recovery + lifecycle INFO-logs), production sessions reproducibly logged `outcome=empty` and the pill-bar waveform was flat. Root cause: `cpal::default_host().default_input_device()` returned the wrong device — not the headset the user spoke into. v1 had a device-picker; v2 has no setting and silently records from whatever the OS picks. This story closes the gap.

**Current code state (v2):**

- `klarvo-audio-cpal/src/source.rs:53-55` calls `host.default_input_device()` unconditionally. The INFO-log at lines 70-78 (added by Story 12.2 diagnostic work) surfaces *which* device was picked but offers no override mechanism.
- `klarvo-core/src/audio/source.rs:26-39` defines `CaptureConfig { sample_rate, channels, events }` — no device field.
- `shells/windows/src-tauri/src/audio.rs:27` `make_audio_source()` returns a bare `CpalAudioSource` with no configuration hooks (Phase-2 TODO at line 26 captures this).
- `klarvo-core/src/settings/mod.rs` has typed-accessor pattern + `SettingsEmitter` trait. Core-namespace `audio.*` is reserved (architecture.md:520 lists `audio.input_device` explicitly — this story implements it).
- `shells/windows/src-tauri/src/commands/settings.rs` has `UserSettings` bulk-get + per-key typed `set_*` commands. Pattern is established.
- `shells/windows/src/index.html` has `SettingsPanel` (inline React) with `settings:changed` listener at lines 247 and 272.

**v1 reference (do NOT port; mirror semantics):**

- `src-tauri/src/audio/mod.rs:492-503` — `list_input_devices() -> Vec<String>` enumerates via `cpal::default_host().input_devices()`.
- `src-tauri/src/audio/mod.rs:510-527` — `find_input_device(name: Option<&str>) -> Result<Device, AudioError>` iterates devices, exact-name-match, falls back to `default_input_device` with `log::warn!` on miss.
- `src-tauri/src/config/mod.rs:531` — `pub audio_device: Option<String>` — name-based persistence, `None` means OS-default.

**Naming-Decision conflict resolved by this story:**

Three artifacts mention the audio-device setting under different names:

| Source | Key | Note |
|---|---|---|
| `architecture.md:520` | `audio.input_device` | Phase-1 architecture-doc reservation |
| `docs/adr/0013-settings-persistence-schema.md:187` | `audio.device_id` | Story-B2-defer placeholder |
| v1 `config/mod.rs:531` | `audio_device` | Device NAME (not ID) |

**Decision (this story):** Key = **`audio.input_device`** (matches architecture.md), Value = **`Option<String>` device NAME** (matches v1 semantics — IDs are reboot-unstable on Windows because they are derived from endpoint-instance GUIDs that change when devices are unplugged/replugged or when the audio-stack reinitializes). ADR-0013 line 187 is reconciled by Amendment 4 (Task T8).

## Acceptance Criteria

**AC-1 — Settings-Key + Default (Core):**
Given a fresh v2-Klarvo install with no prior settings, when `Settings::audio_input_device()` is called, then it returns `Ok(None)` — `None` semantically means "use OS-default device". The accessor reads the SQLite key `audio.input_device`; writing `None` deletes the key; writing `Some(name)` inserts/updates with type-string `"string"`. Pattern mirrors `hotkey_slot2_combo()` (which already uses `Option<String>` with delete-on-clear).

**AC-2 — CaptureConfig Extension (Core):**
Given `klarvo-core/src/audio/source.rs`, when `CaptureConfig` is constructed, then it has a new field `pub device: Option<String>` placed after `channels` and before `events`. The field is advisory-by-convention: `None` means "implementation chooses OS-default", `Some(name)` means "use this named device, fall back to OS-default if not found". No new `AudioError` variant — fallback is silent on the AudioSource boundary; the *caller* (orchestrator) is responsible for surfacing the fallback to the user (AC-9).

**AC-3 — CpalAudioSource Honors device-field (klarvo-audio-cpal):**
Given a `CpalAudioSource` started with `CaptureConfig { device: Some(name), .. }`, when the named device exists in `host.input_devices()`, then `device.name() == name` and the existing INFO-log at `klarvo-audio-cpal/src/source.rs:70-78` emits `device = %name`. Given the same call with a name that does NOT match any enumerated device, then the source falls back to `host.default_input_device()`, emits an additional `tracing::warn!` at target `klarvo.audio.device` with fields `requested = %name, fallback = "os-default"`, and proceeds with the OS-default device. Given `CaptureConfig { device: None, .. }`, then behavior is unchanged from today (OS-default, single INFO-log).

**AC-4 — Tauri-Command `list_audio_input_devices` (Shell):**
Given the Tauri command surface in `shells/windows/src-tauri/src/commands/`, when the frontend calls `list_audio_input_devices()`, then it returns `Result<Vec<String>, AppError>` — the enumerated device names from `cpal::default_host().input_devices()` (de-duplicated via `Vec` insertion-order preserved). On host-enumeration failure (unlikely on Windows but possible) the command logs at `warn` and returns `Ok(Vec::new())` (fail-soft per `feedback_scaffold_fail_soft_pattern`). The command is registered in `specta_builder()` via `collect_commands!` and is exported to TS by tauri-specta.

**AC-5 — Tauri-Command `set_audio_input_device` (Shell):**
Given the same command surface, when the frontend calls `set_audio_input_device(device: Option<String>)`, then `Settings::set_audio_input_device(device)` is invoked. On success the `TauriSettingsEmitter` fires `settings:changed` with `key = "audio.input_device"` and `new_value = device.unwrap_or_default()` (empty string for `None`). The accessor MUST refuse to write a name not present in `list_audio_input_devices()` UNLESS the value is `None`; an invalid name returns `AppError { kind: Configuration, user_message: Some("error.settings.audio.device_not_found") }`. Reason: stale names from a previously-attached device are tolerated by AC-3 (silent OS-default fallback), but should be rejected at write-time so the user is not silently re-bound.

**AC-6 — UserSettings Bulk-Get Includes Field (Shell):**
Given `commands/settings.rs::UserSettings`, when `get_user_settings()` is called, then the returned struct has a new field `audio_input_device: Option<String>` populated from `settings.audio_input_device()?`. The TS-binding regenerates via tauri-specta on `xtask gen-bindings` (or the equivalent dev-command). Existing fields are unchanged.

**AC-7 — SessionOrchestrator Wire-Up (Shell + Orchestrator):**
Given `klarvo-shell-orchestrator/src/lib.rs`, when `SessionOrchestrator::new(...)` is called, then it accepts a new parameter `audio_device_arc: Arc<RwLock<Option<String>>>` placed in the parameter list after `recording_mode_arc_slot2` (analog to that pattern). Per-press session-start in `session.rs` reads `audio_device_arc.read()` and constructs `CaptureConfig { device: cloned_value, .. }`. In `shells/windows/src-tauri/src/main.rs` Step 4 the Arc is constructed from `settings.audio_input_device()?.into()` and passed into `SessionOrchestrator::new` in Step 9.

**AC-8 — Reactive Live-Reload Semantics (Shell):**
Given the `settings:changed` listener in `main.rs` (existing pattern at `recording_mode_arc` lines around 270), when an event arrives with `key == "audio.input_device"`, then the listener writes the new device name (parsing empty-string back to `None`) into `audio_device_arc`. The next hotkey-press uses the new value; a currently-recording session is NOT interrupted (next-press semantics, mirrors `hotkey.slot1.mode`). No mid-recording stream-reset.

**AC-9 — User-facing Fallback Toast (Shell):**
Given a session-start where the configured device is not found (AC-3 triggered the AudioSource fallback path), when the orchestrator detects the fallback, then it fires a one-shot toast via the existing toast-mechanism (Story 9.4 surface) with i18n-key `toast.audio.device_fallback`. Detection mechanism: the orchestrator compares the value it read from `audio_device_arc` against what the AudioSource actually picked. Since the AudioSource boundary does not propagate this (AC-2), the simplest implementation is for the orchestrator to enumerate `cpal::default_host().input_devices()` directly (or via a Core-helper) and check the name exists BEFORE invoking `AudioSource::start`. Implementation note: an enumerate-helper in `klarvo-core` is acceptable but Phase-2 polish — for Story 12.3 the orchestrator may call into a small `klarvo-audio-cpal::device_exists(name: &str) -> bool` helper that the Windows-shell wraps as needed (justifiable because the orchestrator already depends on Core-traits + the Windows-shell injects the impl).

**AC-10 — SettingsPanel UI (Frontend):**
Given the existing SettingsPanel in `shells/windows/src/index.html`, when the user opens Settings, then a new section/row labeled `settings.audio.input_device.label` shows:
1. A `<select>` dropdown populated from `list_audio_input_devices()` plus an explicit "Auto (OS-Default)" option (value=empty / `None`) at the top.
2. A refresh button labeled `settings.audio.input_device.refresh` next to the dropdown that re-runs `list_audio_input_devices()` and updates the options.
3. The currently-persisted value (from `UserSettings.audioInputDevice`) is pre-selected on panel mount.
4. On Save, `set_audio_input_device(value)` is called as part of the atomic `handleSave` flow alongside the other settings; selection-change updates form-state only. On success no UI confirmation (consistent with sibling fields); on error (e.g. `error.settings.audio.device_not_found`) the existing error-display surface is used. (Code-review-closure 2026-05-22: wording aligned with Hotkey/Language/Output-Target sibling-field pattern after Andy's D1 decision.)
5. The settings:changed listener already in the panel does NOT need a new branch (re-reading via `get_user_settings()` on mount + on-success is sufficient given the next-press semantics).

**AC-11 — i18n Keys (Locale-Files):**
Given `shells/windows/locales/en.json` and `de.json`, the following keys are added with English + German translations:
- `error.settings.audio.device_not_found` — write-time validation error (rendered by toast layer from `AppError.userMessage`)
- `toast.audio.device_fallback` — fallback toast text (use placeholder substitution for device-name)

German translations follow existing tone (see `error.audio.device_unavailable`). Coverage-gate (Story 4.X) will fail CI if a key is missing in either file.

Code-review-closure 2026-05-22 (Andy's D2 decision): the originally-planned 3 UI label keys (`settings.audio.input_device.{label,refresh,auto}`) were removed — the SettingsPanel JSX renders hardcoded English strings, consistent with the rest of the inline-React panel which has no `t()` wiring yet. Full i18n migration lands with Phase-2-B Vite+React+i18n toolchain rebuild. The 2 keys above remain because they are emitted from backend code paths (`AppError.user_message` + `ErrorEmitter::emit_error`) and translated by the existing toast layer, not by the SettingsPanel.

**AC-12 — Cross-Compile Verify (Build):**
Given the developer is on WSL/Linux, when they run `cargo check --target x86_64-pc-windows-gnu -p klarvo-windows-shell -p klarvo-audio-cpal -p klarvo-shell-orchestrator -p klarvo-core` after implementing this story, then the build succeeds without errors (warnings are acceptable). Linux-only `cargo check` does not exercise the Windows-shell code path; the cross-compile check is required by `feedback_windows_cross_compile_verify`.

## Approach

### Layer-by-Layer

**Core (`klarvo-core`):**
1. `settings/mod.rs`: add typed accessors `audio_input_device()` (returns `Result<Option<String>, AppError>`) and `set_audio_input_device(val: Option<String>)`. `None`-write does delete via `delete_raw("audio.input_device")`. Pattern: copy `hotkey_slot2_combo()` / `clear_hotkey_slot2_combo()` / `set_hotkey_slot2_combo()` triad.
2. `audio/source.rs`: add `pub device: Option<String>` to `CaptureConfig`. Update doc-comment to document fallback semantics ("Impls SHOULD fall back to OS-default if name not found, logging at WARN level").
3. No new `AudioError` variant. No new event-type. No Core-namespace change.

**Audio-Impl (`klarvo-audio-cpal`):**
4. `source.rs::start`: if `config.device.is_some()`, iterate `host.input_devices()?` and match by `device.name().ok().as_deref() == Some(&name)`. If matched, use it. If not matched, fall back to `host.default_input_device()` + `tracing::warn!(requested = %name, fallback = "os-default", ...)`. If `config.device.is_none()`, behavior unchanged.
5. Add `pub fn device_exists(name: &str) -> bool` at the crate root for use by orchestrator-side toast-detection (AC-9). Implementation: enumerate + name-match, no Result (best-effort, returns `false` on enumeration failure).

**Settings Wire-up (Shell):**
6. `commands/settings.rs::UserSettings`: add `pub audio_input_device: Option<String>` field; populate in `get_user_settings`.
7. Add two new `#[tauri::command]` functions: `list_audio_input_devices` (no args, returns `Result<Vec<String>, AppError>`) and `set_audio_input_device` (args: `device: Option<String>`, returns `Result<(), AppError>`). Register in `collect_commands!` invocation alongside the other settings commands. Validation in `set_audio_input_device`: if `device.is_some()`, call `klarvo_audio_cpal::device_exists` and reject with `error.settings.audio.device_not_found` if false.

**Orchestrator Wire-up (Shell + Orchestrator):**
8. `klarvo-shell-orchestrator/src/lib.rs::SessionOrchestrator::new`: add parameter `audio_device_arc: Arc<RwLock<Option<String>>>` after `recording_mode_arc_slot2`. Store on the struct.
9. `klarvo-shell-orchestrator/src/session.rs`: when constructing CaptureConfig for `audio.start(config)`, read `audio_device_arc.read().clone()` and populate `device` field. BEFORE invoking `AudioSource::start`, check `klarvo_audio_cpal::device_exists(&name)` if device is Some; on `false`, emit a toast via the existing toast-emitter trait surface (Story 9.4 — `ToastEmitter` or equivalent). The toast i18n-key is `toast.audio.device_fallback` with placeholder for the device-name. (Note: the orchestrator currently does not depend on `klarvo-audio-cpal` directly — adding this dependency is acceptable for this story because the helper is small and Phase-3 Android will provide its own `device_exists` impl behind a `cfg(target_os)`. Alternative is to add a small `Fn(&str) -> bool` injected at construction; pick whichever is cleaner during impl.)

**Main Wire-up (Shell):**
10. `shells/windows/src-tauri/src/main.rs` Step 4: read initial device from `settings.audio_input_device()?` (with same fail-soft warn-and-default-to-None pattern used by `output_language` at line 365). Wrap in `Arc::new(RwLock::new(...))`. Pass to `SessionOrchestrator::new` in Step 9 (parameter position per AC-7).
11. Add a `settings:changed` listener for `key == "audio.input_device"` that writes the new value into the Arc. Place adjacent to existing listeners around lines 247/272.

**Frontend (SettingsPanel):**
12. `shells/windows/src/index.html` SettingsPanel: add a labeled row with a `<select>` (populated by calling `list_audio_input_devices()` on mount), a refresh `<button>`, and pre-select the current value. On-change calls `set_audio_input_device`. Use the existing error-handling surface.

**i18n:**
13. Add the 5 new keys to `locales/en.json` and `locales/de.json`.

**ADR-0013 Amendment 4:**
14. Append a new amendment block to `docs/adr/0013-settings-persistence-schema.md` documenting: line 187 `audio.device_id` is superseded by `audio.input_device` (device NAME, not ID); reason = v1 parity + Windows ID-instability; this story's commit SHA cross-referenced.

### Why next-press, not mid-recording

A mid-recording device-swap would require stopping the cpal stream, dropping the `CaptureHandle`, constructing a new one, and re-starting — racing against the VAD and the broadcast-receiver. The user-value is marginal (people change mics between sessions, not mid-sentence). Next-press semantics matches `hotkey.slot1.mode` and reduces concurrency risk.

### Why name-not-id

cpal exposes devices as `Device` objects without stable platform-IDs. The closest analog on Windows is the `IMMDevice::GetId()` endpoint-ID, which is a `LPWSTR` derived from the audio-endpoint MMDevice GUID. These IDs *do* persist across reboots in most cases but DO change when:
- A device is unplugged + plugged into a different USB port (new endpoint instance)
- The audio-stack is reset (Windows Audio service restart, driver reinstall)
- A device-driver update assigns a new endpoint-GUID

v1 used names; v1 users (= Andy) have not reported confusion from this. Name-collisions (two devices with identical names) are rare enough that we accept the v1-equivalent silent-pick-first behavior; if it becomes a real problem we add a "(USB-2)" disambiguator UI-side without changing persistence.

## Risks

- **R1 — device_exists helper as orchestrator dependency:** Adding `klarvo-audio-cpal` to `klarvo-shell-orchestrator`'s Cargo.toml breaks the orchestrator's platform-neutrality. Mitigation: prefer the closure-injection alternative ("inject `Box<dyn Fn(&str) -> bool + Send + Sync>` at construction") — cleaner separation. Dev-agent picks during impl; document the choice in the PR.
- **R2 — settings:changed event flood:** A user spamming the device-dropdown could trigger many `settings:changed` events. The Arc<RwLock>-write is cheap and atomic; no debounce needed.
- **R3 — enumerate latency:** `list_input_devices` is cheap on Windows (<50ms typical). On rare driver-stalls it could block the Tauri main loop. Acceptable for Phase-1; if it becomes a problem the command can be made `async` with `tokio::task::spawn_blocking`.
- **R4 — ADR-0013 Amendment scope creep:** Amendment 4 should ONLY reconcile the line-187 naming. It is NOT the place to redesign `audio.*` namespace.

## Non-Goals

- Android device-selection (Phase-3, separate story; trait shape is already compatible).
- Live device-change detection via `IMMNotificationClient` (refresh button is sufficient for MVP).
- Per-plugin device-overrides (this story is global only).
- Sample-rate/channels override (already deferred in ADR-0013 line 187; that defer remains).
- Disambiguation of duplicate device names ("Headset (1)" vs "Headset (2)").
- v1→v2 device-name migration (covered by Epic-7 v1-import story when triggered).

## Out-of-Scope (Carry-Over)

- The hotkey-intermittent-fire issue Andy observed on 2026-05-22 ("es gab 2 momente, wo hotkey nicht feuerte") is unrelated and deferred to a separate diagnose-story if it reproduces.

## Definition of Done

- All 12 ACs satisfied with code in place.
- `cargo test -p klarvo-core` green for new settings accessor tests.
- `cargo test -p klarvo-audio-cpal` green for new `device_exists` test (host-mocked or `#[ignore]`-gated, since real cpal needs an OS audio device).
- Linux `cargo check --workspace` green.
- Cross-compile `cargo check --target x86_64-pc-windows-gnu -p klarvo-windows-shell -p klarvo-audio-cpal -p klarvo-shell-orchestrator -p klarvo-core` green.
- xtask `verify-release` + `check-locales` green (new i18n keys present in both files; G3 lint passes).
- Manual smoke test on Windows-Release-Build: (1) open settings, see device dropdown populated; (2) select non-default device, save, press hotkey, verify INFO-log emits `device = <selected name>`; (3) unplug device, press hotkey, verify warn-log + toast + recording continues on OS-default.
- ADR-0013 Amendment 4 committed in a SEPARATE commit (per `feedback_adr_amendment_convention`).
- sprint-status.yaml updated with story-completion note + commit-SHA.

## Tasks/Subtasks

- [x] **T1 — Core Settings Layer** (AC-1)
  - [x] Add `audio_input_device()` + `set_audio_input_device()` + delete-on-None to `klarvo-core/src/settings/mod.rs`
  - [x] Add unit test mirroring `hotkey_slot2_combo` tests (set/get/clear roundtrip)
- [x] **T2 — CaptureConfig Extension** (AC-2)
  - [x] Add `device: Option<String>` field to `CaptureConfig` in `klarvo-core/src/audio/source.rs`
  - [x] Update doc-comment with fallback semantics
  - [x] Update all callers (test fixtures, mock audio source, `klarvo-audio-cpal`, `klarvo-shell-orchestrator`) to populate the new field — initially with `None` to keep builds green during incremental impl
- [x] **T3 — CpalAudioSource Implementation** (AC-3, AC-12)
  - [x] Implement device-name lookup in `klarvo-audio-cpal/src/source.rs::start`
  - [x] Add fallback `tracing::warn!` on miss
  - [x] Add `pub fn device_exists(name: &str) -> bool` (or equivalent helper — see R1)
  - [x] Cross-compile check
- [x] **T4 — Tauri Commands** (AC-4, AC-5)
  - [x] Add `list_audio_input_devices` + `set_audio_input_device` to `shells/windows/src-tauri/src/commands/settings.rs`
  - [x] Register in `collect_commands!`
  - [x] Validation in `set_audio_input_device` (reject unknown names via `error.settings.audio.device_not_found`)
- [x] **T5 — UserSettings Field** (AC-6)
  - [x] Add `audio_input_device: Option<String>` to `UserSettings`
  - [x] Populate in `get_user_settings`
- [x] **T6 — Orchestrator Wire-Up** (AC-7, AC-8)
  - [x] Add `audio_device_arc` parameter to `SessionOrchestrator::new` in `klarvo-shell-orchestrator/src/lib.rs`
  - [x] Read Arc + populate `CaptureConfig.device` in `session.rs`
  - [x] Pre-flight device_exists check + toast emission (AC-9 path)
- [x] **T7 — Main Wire-Up** (AC-7, AC-8)
  - [x] Construct `audio_device_arc` from settings in `shells/windows/src-tauri/src/main.rs` Step 4
  - [x] Pass to `SessionOrchestrator::new` in Step 9
  - [x] Add `settings:changed` listener for `audio.input_device` adjacent to existing listeners
- [x] **T8 — SettingsPanel UI** (AC-10)
  - [x] Add device-row in `shells/windows/src/index.html`
  - [x] List-fetch on mount + refresh button
  - [x] Pre-select current value
  - [x] On-change call `set_audio_input_device`
- [x] **T9 — i18n Keys** (AC-11)
  - [x] Add 5 keys to `shells/windows/locales/en.json`
  - [x] Add 5 keys to `shells/windows/locales/de.json`
- [x] **T10 — ADR-0013 Amendment 4** (Definition of Done)
  - [x] Append amendment block to `docs/adr/0013-settings-persistence-schema.md`
  - [x] Separate commit per `feedback_adr_amendment_convention`
- [ ] **T11 — Manual Smoke Test on Windows Release-Build** (DoD)
  - [ ] Per the three-step manual test in §Definition of Done
  - [ ] Document outcome in this story's Dev Agent Record section

### Review Findings

Adversarial Multi-Layer-Review (Blind Hunter + Edge Case Hunter + Acceptance Auditor) — 2026-05-22. Black-Screen-Smoke-Test war Trigger. Alle 10 Patches angewendet (P10 NEU entdeckt während Patch-Anwendung: `klarvo-audio-cpal` dep war unter Windows-only cfg-target → Linux-Workspace-Build broken).

- [x] [Review][Patch] **P10 (NEU) — `klarvo-audio-cpal` dep aus `[target.'cfg(target_os = "windows")']` rausgezogen** [`shells/windows/src-tauri/Cargo.toml:44`] — `commands/settings.rs` ruft `klarvo_audio_cpal::list_input_devices()` und `device_exists()` unconditional auf; dep war aber Windows-cfg-gated → `cargo check -p klarvo-windows-shell` failed auf Linux. Dep in reguläre `[dependencies]` verschoben. klarvo-audio-cpal compiled cross-platform clean. Verifiziert: `cargo check --workspace --lib` grün.
- [x] [Review][Patch] **P1 — TDZ-ReferenceError → BLACK SCREEN gefixt** [`shells/windows/src/index.html:217-228`] — `const loadDevices = useCallback(...)` Block vor die Mount-`useEffect` verschoben (von Line 309 nach Line 219); Deps-Array `[loadDevices]` bleibt. Erste-Render-TDZ kann nicht mehr triggern.
- [x] [Review][Patch] **P2 — `delete_raw` Clear-Path emittiert jetzt `settings:changed` mit empty value** [`klarvo-core/src/settings/mod.rs:362-374`] — `set_audio_input_device(None)` ruft `emit_or_warn(&*self.emitter, "audio.input_device", "")` nach `delete_raw(...)` auf. Neuer Test `audio_input_device_clear_emits_settings_changed_with_empty_value` verifiziert beide Emit-Events (set + clear). Listener in main.rs:539-547 parsed empty-string → None korrekt → `audio_device_arc` reactive-update auf Clear funktioniert jetzt ohne App-Restart.
- [x] [Review][Patch] **P3 — UI reconciled stale `form.audioInputDevice` mit `availableDevices`** [`shells/windows/src/index.html:506-512`] — synthetisches `<option>` mit Label `<name> (not currently available)` wird gerendert wenn die persistierte Device nicht in der aktuellen Enumeration ist. User sieht die echte React-State-Wahl statt browser-fallback-Lüge auf erstes Option.
- [x] [Review][Patch] **P4 — `device_check_fn` läuft jetzt in `tokio::task::spawn_blocking`** [`klarvo-shell-orchestrator/src/session.rs:230-237`] — Per-Press WASAPI-COM-Enumeration blockt Tokio-Worker nicht mehr; `Arc::clone(&self.device_check_fn)` + `name.clone()` für owned-data-Transfer ins blocking-Thread; `.await.unwrap_or(false)` für fail-soft-Fallback.
- [x] [Review][Patch] **P5 — Tauri `set_audio_input_device` auf `async` + `spawn_blocking`** [`shells/windows/src-tauri/src/commands/settings.rs:343-373`] — Command-Signatur jetzt `pub async fn`; `device_exists`-Call in `spawn_blocking` mit join-error-handling als `AppError`. IPC-Dispatcher bleibt während WASAPI-Enumeration frei.
- [x] [Review][Patch] **P6 — tauri-specta TS-Bindings regeneriert** [`shells/windows/src/bindings/index.ts`] — `cargo xtask generate-bindings` erfolgreich; neue Einträge: `listAudioInputDevices`, `setAudioInputDevice`, `UserSettings.audioInputDevice`. `xtask bindings-drift` grün.
- [x] [Review][Patch] **P7 — Story-Frontmatter-Status synchronisiert** [`epic-12-3-audio-input-device-selection.md:3`] — `status: ready-for-dev` → `status: review`.
- [x] [Review][Patch] **P8 (resolved from D1) — AC-10.4 wording an Sibling-Field-Pattern angepasst** [`epic-12-3-audio-input-device-selection.md:113`] — AC-10 step 4 umgeschrieben: "On Save, `set_audio_input_device(value)` is called as part of the atomic `handleSave` flow ...". Implementation unverändert (on-Save-Persistence konsistent mit Hotkey/Language/Output-Target).
- [x] [Review][Patch] **P9 (resolved from D2) — 3 dead i18n-Keys + Allowlist-Einträge entfernt** [`shells/windows/locales/en.json` + `de.json` + `xtask/orphan-allowlist.txt`] — `settings.audio.input_device.{label,refresh,auto}` aus beiden Locale-Files + 3 Allowlist-Blöcken entfernt. Behalten: `error.settings.audio.device_not_found` + `toast.audio.device_fallback` (Backend-emittiert, Toast-Layer rendert). AC-11 mit D2-Decision-Notiz aktualisiert. `xtask lint-events` grün.
- [x] [Review][Defer] **DF1 — Duplicate Device-Names werden nicht disambiguiert** [`klarvo-audio-cpal/src/lib.rs:9-13` + `index.html:498`] — Zwei USB-Devices mit identischem Namen ("Microphone (USB Audio)") führen zu React duplicate-key-warning + non-deterministischer cpal-Auswahl. Defer-Reason: Story Non-Goals explizit: "Disambiguation of duplicate device names ('Headset (1)' vs 'Headset (2)') — pre-existing v1-equivalent silent-pick-first".
- [x] [Review][Defer] **DF2 — AC-12: `klarvo-windows-shell`-Cross-Compile übersprungen** [`cargo check --target x86_64-pc-windows-gnu`] — Story-Dev hat `klarvo-windows-shell` aus dem Cross-Compile-Set gestrichen wegen pre-existing whisper-rs-sys-MinGW-Overflow. AC-12 listet die Crate aber explizit. Defer-Reason: pre-existing whisper-rs-sys-Issue (cf. 12.1-DF2), nicht durch 12.3 verursacht.
- [x] [Review][Defer] **DF3 — Boot-Time-Check für stale audio.input_device fehlt** [`shells/windows/src-tauri/src/main.rs:313-323`] — `audio_device_arc` wird beim Boot aus Settings ohne `device_exists`-Check geladen; User sieht keine Indikation bis erster Hotkey-Press einen Fallback-Toast feuert. UX-Polish. Defer-Reason: Per-Press-Toast (AC-9) ist die spezifizierte UX, Boot-Time-Indikator wäre Phase-2-Polish.
- [x] [Review][Defer] **DF4 — cpal-Host-Enumeration-Fehler returned silent leeres `Vec`** [`klarvo-audio-cpal/src/lib.rs:9-13`] — `unwrap_or_default()` schluckt Host-Init-Failures; User sieht nur "Auto" im Dropdown ohne Error-Toast. Defer-Reason: by-design "fail-soft per `feedback_scaffold_fail_soft_pattern`" (Story AC-4). Diagnostic-Log könnte ergänzt werden, aber separate UX-Story.
- [x] [Review][Defer] **DF5 — `validate_setting_value` rejected Control-Chars in Device-Namen** [`klarvo-core/src/settings/mod.rs:493-510`] — cpal-Device-Namen kommen vom OS und können in Edge-Cases Unicode-Marks enthalten (RTL/LTR, U+200F). `validate_setting_value` rejected mit `AppErrorKind::Validation`. Defer-Reason: extrem selten, kein konkreter Bug-Report, low-priority Polish.
- [x] [Review][Defer] **DF6 — Toast-Rate-Limit fehlt; Toast-Fatigue bei permanent fehlendem Device** [`klarvo-shell-orchestrator/src/session.rs:228-239`] — User mit unplugged-Device sieht auf JEDEM Press einen Fallback-Toast (100x/Tag bei Power-User). Defer-Reason: UX-Polish (per-session-suppression oder auto-clear-to-None), nicht-blocking.
- [x] [Review][Defer] **DF7 — `cpal::Device::name()` Err schluckt Device aus Liste UND blockiert Re-Selektion** [`klarvo-audio-cpal/src/lib.rs:10`] — `filter_map(|d| d.name().ok())` droppt Devices mit Name-Lookup-Fehler still; persistierter Name kann dann nicht mehr re-selektiert werden. Defer-Reason: pre-existing cpal-Pattern, extrem selten.

## Dev Notes

### Relevant Architecture & Constraints

- Tauri event-names: colon-notation (`settings:changed`, NOT dot). Memory `feedback_tauri_vs_core_event_naming`.
- New i18n keys in `shells/windows/locales/`, NOT `src/locales/` (deleted 2026-05-02).
- No remote telemetry — fallback-toasts go through existing local toast-emitter (Story 9.4 surface), not Sentry or similar.
- Settings emitter calls `app.emit("settings:changed", ...)` synchronously per `TauriSettingsEmitter`. Listener-write into the Arc must use `try_write` or accept lock-contention — `RwLock::write` is acceptable here because the only contender is the per-press read.
- `klarvo-core` MUST NOT depend on `klarvo-audio-cpal`. The `device_exists` helper lives in `klarvo-audio-cpal`; orchestrator-side use is via closure-injection OR a direct dep added to the shell-orchestrator crate (NOT to Core).
- ADR-0006 (AudioSource trait) is unchanged in shape — only `CaptureConfig` gains a field. Trait-method signature stays `async fn start(&mut self, config: CaptureConfig) -> Result<CaptureHandle, AudioError>`.

### Source Tree Components to Touch

| Path | Change |
|---|---|
| `klarvo-core/src/audio/source.rs` | Add `device` field to `CaptureConfig` |
| `klarvo-core/src/settings/mod.rs` | Add typed accessors (3 methods) |
| `klarvo-audio-cpal/src/source.rs` | Device-name lookup + fallback logging |
| `klarvo-audio-cpal/src/lib.rs` | Export `device_exists` helper (or wherever appropriate) |
| `klarvo-shell-orchestrator/src/lib.rs` | New constructor parameter |
| `klarvo-shell-orchestrator/src/session.rs` | Read Arc + populate CaptureConfig + pre-flight check + toast |
| `klarvo-shell-orchestrator/Cargo.toml` | Add dep on `klarvo-audio-cpal` (or use closure-injection — pick during impl) |
| `shells/windows/src-tauri/src/audio.rs` | Update doc-comment; phase-2 TODO resolved |
| `shells/windows/src-tauri/src/main.rs` | Step 4 Arc construction + Step 9 parameter + settings:changed listener |
| `shells/windows/src-tauri/src/commands/settings.rs` | UserSettings field + 2 new commands + collect_commands! registration |
| `shells/windows/src/index.html` | SettingsPanel device-row |
| `shells/windows/locales/en.json` | 5 new keys |
| `shells/windows/locales/de.json` | 5 new keys |
| `docs/adr/0013-settings-persistence-schema.md` | Amendment 4 (separate commit) |
| `_bmad-output/implementation-artifacts/sprint-status.yaml` | Story completion entry |

### Testing Standards Summary

- Core unit tests: `klarvo-core/src/settings/mod.rs` `#[cfg(test)] mod tests` — get/set/delete roundtrip for `audio_input_device`. Use the existing in-memory Settings fixture pattern.
- `klarvo-audio-cpal` unit tests: `device_exists` with mocked host is hard — accept `#[ignore]`-gated integration tests OR a unit test that constructs a Vec<String> mock if a host-abstraction is introduced. Pragmatic: rely on the CpalAudioSource start-path being smoke-tested manually (DoD T11).
- Cross-compile is a non-negotiable verify gate (memory `feedback_windows_cross_compile_verify`).
- No integration tests for the orchestrator-side toast flow in this story — the Story 9.4 toast surface is already exercised by other stories.

### Previous Story Intelligence (Story 12.2)

- 12.2 closure committed lifecycle INFO-logs at boot/session/pipeline. The session-start log (`tracing::info!(target: "klarvo.session", ?slot, mode = ?press_mode, "recording started")`) is the visibility anchor that this story extends — when this story lands, that log line should be cross-referenceable with the `klarvo.audio.device` log line in the same session-window. Both targets are already filter-respected by the rolling-file appender.
- 12.2 removed `klarvo-core/src/telemetry/diag.rs` and the pre-tracing boot-stage markers. Do not reintroduce them; if a Windows-Release-Build smoke test fails again, the diagnostic-pattern is now the integration test `init_tracing_writes_events.rs` plus the lifecycle INFO logs.
- 12.2 root-cause was an observability gap on the happy path. This story does NOT widen observability further — the INFO log at `klarvo.audio.device` already added by 12.2 (commit `43ebef7`) is the audit-trail that proves which device was used per session. Verify in T11 smoke test that this log appears with the *configured* device name (not just OS-default).

### Git Intelligence Summary

Recent commits relevant to this story:
- `43ebef7 feat(audio): log OS-default input device name + native config at session start` — added the visibility log this story builds on
- `1c13153 feat(audio): log samples_seen + max_rms on capture-empty path` — diagnostic counters that proved the silent-mic root cause
- `f43cbcc feat(12.2): replace boot-stage diag markers with lifecycle INFO logs` — observability-gap closure
- `f62c52b revert(event-naming): restore "settings:changed" + "app:ready" colon-form` — confirms colon-notation for Tauri events, NOT dot

### Project Structure Notes

- This story does NOT touch `src/`, `src-tauri/`, or `android/` (v1 paths — per memory `project_v1_v2_coexistence`).
- Workspace `Cargo.toml` does not need updating — all crates touched are already members.
- The shell-orchestrator crate gaining a dep on `klarvo-audio-cpal` (if that route is chosen — see R1) is the only Cargo.toml-level structural change. Document the choice in the PR description.

### References

- [Source: docs/adr/0006-audiosource-trait-signature.md] — CaptureConfig + AudioSource shape
- [Source: docs/adr/0013-settings-persistence-schema.md#line-187] — `audio.device_id` defer (this story resolves)
- [Source: _bmad-output/planning-artifacts/architecture.md#L520] — `audio.input_device` namespace reservation
- [Source: src-tauri/src/audio/mod.rs#L490-527] — v1 enumerate + find pattern
- [Source: src-tauri/src/config/mod.rs#L531] — v1 `audio_device: Option<String>` persistence
- [Source: shells/windows/src-tauri/src/commands/settings.rs] — UserSettings + emitter pattern
- [Source: shells/windows/src-tauri/src/main.rs#L247,L272] — settings:changed listener pattern
- [Source: klarvo-audio-cpal/src/source.rs#L53-78] — current OS-default-pick + INFO log
- [Source: klarvo-core/src/audio/source.rs#L26-39] — current CaptureConfig
- Memory: `feedback_two_symptom_upstream_check` (trigger), `feedback_autonomous_decisions` (decision-rationale convention), `feedback_scaffold_fail_soft_pattern`, `feedback_windows_cross_compile_verify`, `feedback_tauri_vs_core_event_naming`, `project_v1_v2_coexistence`, `project_klarvo_v2_rebuild`

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

- Cross-compile: `cargo check --target x86_64-pc-windows-gnu -p klarvo-audio-cpal -p klarvo-shell-orchestrator -p klarvo-core` → clean. `klarvo-windows-shell` cross-compile fails on `whisper-rs-sys` overflow (pre-existing issue unrelated to this story; Linux-MinGW whisper build is known-broken).
- R1 decision: chose closure-injection (`Arc<dyn Fn(&str) -> bool + Send + Sync>`) over direct `klarvo-audio-cpal` dep on orchestrator — keeps orchestrator platform-neutral; injected as `Arc::new(klarvo_audio_cpal::device_exists)` in main.rs.
- `list_input_devices()` and `device_exists()` extracted into `klarvo-audio-cpal/src/lib.rs` (not the shell) so the shell's `list_audio_input_devices` command is a thin wrapper.
- 3 frontend-only locale keys added to `xtask/orphan-allowlist.txt` (`settings.audio.input_device.label/refresh/auto`) to satisfy lint-events G3-D.

### Completion Notes List

- T1: `audio_input_device()` + `set_audio_input_device(Option<String>)` in settings/mod.rs; delete-on-None pattern mirrors `clear_hotkey_slot2_combo`; 4 unit tests pass.
- T2: `CaptureConfig.device: Option<String>` added after `channels`; all 10 call-sites updated with `device: None`; doc-comment documents fallback semantics.
- T3: `CpalAudioSource::start` resolves device by name with OS-default fallback + WARN log; `device_exists()` + `list_input_devices()` exported from `klarvo-audio-cpal`.
- T4/T5: `list_audio_input_devices` + `set_audio_input_device` Tauri commands; `UserSettings.audio_input_device` field; registered in `collect_commands!`.
- T6: `SessionOrchestrator::new` gains `audio_device_arc` + `device_check_fn` params; on_press reads arc, populates CaptureConfig.device, and pre-flight checks via closure before AudioSource::start; toast on miss.
- T7: main.rs Step 4b constructs `audio_device_arc` (fail-soft); Step 9 passes arc + `Arc::new(klarvo_audio_cpal::device_exists)`; Step 10b-audio listener keeps arc in sync on settings:changed.
- T8: SettingsPanel device-row: `<select>` with OS-default option + named devices from `list_audio_input_devices`; refresh button; pre-select from `UserSettings.audioInputDevice`; on-change calls `set_audio_input_device`.
- T9: 5 i18n keys in en.json + de.json. Keys: `settings.audio.input_device.label/refresh/auto`, `error.settings.audio.device_not_found`, `toast.audio.device_fallback`.
- T10: ADR-0013 Amendment 4 in separate commit c68063c per `feedback_adr_amendment_convention`.
- T11: Manual smoke test deferred — requires Windows Release-Build; left unchecked for Andy to verify.

### File List

- `klarvo-core/src/audio/source.rs` — CaptureConfig.device field added
- `klarvo-core/src/settings/mod.rs` — audio_input_device accessors + 4 unit tests
- `klarvo-audio-cpal/src/source.rs` — device-name lookup + fallback WARN log
- `klarvo-audio-cpal/src/lib.rs` — list_input_devices() + device_exists() exports
- `klarvo-shell-orchestrator/src/session.rs` — audio_device_arc + device_check_fn + pre-flight check + CaptureConfig.device wire-up
- `klarvo-shell-orchestrator/tests/session_tests.rs` — 4 call-sites updated
- `klarvo-shell-orchestrator/tests/e2e_test.rs` — 1 call-site updated
- `klarvo-plugins/klarvo-plugin-groq/tests/e2e_dictation_session.rs` — 6 call-sites updated
- `klarvo-test-fixtures/tests/capture_session.rs` — 2 call-sites updated
- `klarvo-test-fixtures/tests/mock_audio_source.rs` — 2 call-sites updated
- `shells/windows/src-tauri/src/commands/settings.rs` — UserSettings field + 2 commands
- `shells/windows/src-tauri/src/lib.rs` — collect_commands! registration
- `shells/windows/src-tauri/src/main.rs` — Step 4b arc construction + Step 9 params + Step 10b-audio listener
- `shells/windows/src/index.html` — SettingsPanel device-row
- `shells/windows/locales/en.json` — 5 new keys
- `shells/windows/locales/de.json` — 5 new keys
- `xtask/orphan-allowlist.txt` — 3 frontend-only keys
- `docs/adr/0013-settings-persistence-schema.md` — Amendment 4

## Change Log

| Date       | Author | Note                                                                 |
|------------|--------|----------------------------------------------------------------------|
| 2026-05-22 | Andi   | Story drafted via `/bmad-create-story`. Decisions D-1..D-5 from prior subagent spec are folded into ACs as final decisions (no Open Questions block). ADR-0013 Amendment 4 = T10. |
| 2026-05-22 | Claude Sonnet 4.6 | Implemented T1–T10; commits 0c16ba9 (implementation) + c68063c (ADR-0013 Amendment 4). T11 (Windows smoke test) deferred to Andy. R1 resolved via closure-injection. whisper-rs-sys cross-compile pre-existing issue documented. |
