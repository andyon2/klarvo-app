---
project_name: 'klarvo'
user_name: 'Andi'
date: '2026-08-17'
sections_completed:
  [
    'technology_stack',
    'language_rules',
    'framework_rules',
    'testing_rules',
    'quality_rules',
    'workflow_rules',
    'anti_patterns',
  ]
status: 'complete'
rule_count: 40
optimized_for_llm: true
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

> **Active branch is `v1-ship` — this is a brownfield v1 product (Tauri desktop + native Kotlin Android).** The repo also contains a v2 BMAD-greenfield blueprint (plugin architecture, uniffi/manifest executor, `jni 0.22`). **v2 is archive/reference — do NOT apply its patterns, versions, or APIs to v1 code.** When a doc or memory mentions plugins, manifest executor, Phase-N, or Epic-N greenfield work, it is v2-historical.

---

## Technology Stack & Versions

**Desktop backend (Rust):** Tauri 2 · tokio 1 · reqwest 0.12 (`rustls-tls`, `default-features = false`) · rusqlite 0.32 (`bundled`) · serde 1 · anyhow 1 + thiserror 2 · hound 3.5 (WAV) · cpal 0.15 (desktop audio) · arboard 3 (clipboard) · voice_activity_detector 0.2.1 · windows 0.61 (Win32 APIs) · tiny-skia 0.11 + encoding_rs 0.8 (Windows only)

**Offline inference (target-gated):** whisper-rs 0.15.1 (Windows + Android targets) · llama-cpp-2 0.1.140 (**Windows only** — needs libclang + CMake)

**Frontend:** React 19.1 · TypeScript 5.8.3 · Vite 7 · TailwindCSS 4.2 (`@tailwindcss/vite`) · `@tauri-apps/api` 2 · ESM (`"type": "module"`)

**Android:** native Kotlin (minSdk 24) · `jni 0.21` (raw JNI bridge to Rust) · once_cell 1 (global model cache) · AudioRecord + AccessibilityService + overlay (`TYPE_APPLICATION_OVERLAY`)

**Test deps:** insta 1 (snapshots) · wiremock 0.6 (HTTP doubles)

**Crate layout:** bin `klarvo` (`src-tauri/src/main.rs`, thin) → lib `klarvo_lib` (crate-type `staticlib`+`cdylib`+`rlib`). App version 0.5.0. Tauri identifier `com.klarvo.voice`.

**Version constraints:** `jni` is pinned at **0.21** (NOT 0.22 — that is v2). `reqwest` MUST keep `default-features = false` + `rustls-tls` (no native OpenSSL). whisper-rs/llama-cpp-2 are **not** available on Linux/macOS builds. `@tauri-apps/plugin-log` is pinned at exactly **2.8.0** — each JS Tauri plugin must match its Rust crate; a re-resolve breaks the Windows build (Story 11-6 GATE-4a).

**ADRs in force:** 0015 state-file writes · 0016 Android path parity · 0017 shared-core STT path · 0018 Android bubble rendering tech · 0019 cross-platform design SSOT · 0020 WebView2 fixed-runtime pin · 0021 native desktop overlays.

## Critical Implementation Rules

### Language-Specific Rules

- **Platform-gate heavy deps.** `whisper-rs`, `llama-cpp-2`, `cpal`, `arboard`, `jni`, `windows` are all behind `#[cfg(...)]` targets in `Cargo.toml`. Never add an unconditional dependency or `use` that breaks the Android or Linux build. Mirror the existing `cfg(target_os = ...)` / `cfg(windows)` gates.
- **Errors are structured `Result`, never panics.** Use `thiserror`/`anyhow`. Scaffolds and not-yet-implemented paths return a structured `AppError` — **never** `todo!()`, `unimplemented!()`, or `panic!()` (fail-soft pattern).
- **No `debug_assert!` with side-effects.** It compiles out in release → silent behavior divergence on Windows.
- **TypeScript runs in strict mode** (`tsc` gates `npm run build`). ESM only.

### Framework-Specific Rules

- **Tauri event names use colons, never dots:** `klarvo:state-changed`, not `klarvo.state-changed`. Tauri reserves `.` in event strings. (The ADR-0002 dot-rule applies only to Core-Bus `Event::*` variants, which are v2 — not present in v1.)
- **Desktop overlays are native Win32 layered windows** — `src-tauri/src/native_pill.rs` and `native_preview.rs`, painted with tiny-skia (ADR-0021, Epic 10). They are NOT WebView2/Tauri windows. Never reintroduce a Tauri window for the pill or the preview; the transparent-webview route was measured and abandoned (WebView2 backgrounding).
- **Android bypasses Tauri IPC (~85%).** Kotlin (`KlarvoApi.kt`, `KlarvoOverlayService.kt`) talks to the Rust JNI bridge and HTTP directly — NOT through Tauri commands. **Any change to shared behavior (config keys, silence/VAD thresholds, paste logic) must be mirrored in BOTH the Rust path AND the Kotlin path.** This is the #1 source of cross-platform drift. See ADR-0016 (Android path parity).
- **The shared Rust core covers STT and license ONLY** (ADR-0017). Cleanup, chunking, VAD and LLM routing are Rust↔Kotlin **twins**. Before fixing a bug on one platform, determine which of the two it is: shared → fix once; twinned → fix twice.
- **Android STT runs through `GroqSttBridge.nativeTranscribe`** — the shared Rust Groq path (`large-v3-turbo`), same model as desktop. `LocalWhisperInference` is dormant fallback code: the whisper-rs Android target compiles, but it is not the live path. Do not treat Android as an offline-STT platform.
- **Kotlin sources live in `android/kotlin-src/`, tests in `android/kotlin-test/`.** `gen/android/` is generated by `tauri android init` and gitignored — files hand-placed there disappear silently on the next regeneration. Put every durable source, resource and manifest edit in the tracked tree and let `scripts/android-smoke.sh` / `android-build.sh` copy it in.
- **Cleanup LLM defaults to DeepSeek** (`llm_provider: "deepseek"`). Groq serves STT only — never route cleanup to Groq.
- **Config is the single source of truth, in `config.json`** (in AppData, loaded via `load_config_reporting`) — NOT SQLite and NOT Tauri storage. All settings (`groq_api_key`/other API keys, `license_key`, `hotkey_slots`, languages) live in `AppConfig` → `config.json`. The only SQLite DB is `history.db` (recording history). API keys are currently plaintext in `config.json`; an OS-keystore is a noted future improvement, not yet implemented. (A v2-era note claimed a SQLite `config.db` + a "hotkey boot bypasses DB" bug — that was the abandoned v2 architecture; v1's boot path reads `hotkey_slots` from the same `config.json` the UI save writes.)
- **The hotkey fires an async pipeline**, it does not block in the OS callback. Pipeline lives in `src-tauri/src/pipeline.rs` (STT → cleanup → paste), emitting a Tauri event per stage.
- **Provider trait pattern:** STT/LLM providers are swappable via config. The config keys are `stt_provider` and **`llm_provider`** (`cleanup_provider` is the runtime `AppState` field, NOT a config key). Add new providers behind the existing trait, don't special-case in the pipeline.

### Testing Rules

- **Tests are inline `#[cfg(test)]` modules**, not a separate `tests/` tree. Snapshot tests use `insta`; snapshots live in `src-tauri/src/snapshots/`. Accept with `cargo insta review`.
- **Linux `cargo test` + lint do NOT satisfy the DoD for surface/UI stories.** Hard gate: a real **Windows release build + manual press-to-paste smoke** is required. (`cargo check` and Linux tests mask Tauri-runtime bugs and Windows-only code paths.) **Before the smoke, run the applicable items of `docs/surface-smoke-checklist.md`** — the running ledger of traps that are green on Linux (camelCase config keys, Settings resync-`useEffect`, FloatingBar separate-window reactivity, window-geometry/region clip, event push-wiring). Mechanical check, not a self-attestation (Epic-5 retro AI-1).
- **Android changes require an on-device/emulator smoke before a story is done** via `scripts/android-smoke.sh`. Split the device gate: **you** install the fresh APK and verify the behaviour; the user's real Xiaomi/HyperOS device is the *aesthetic* judgement only. `android-smoke.sh` runs a build-time gate (`node scripts/gen-android-theme.mjs --check`) that fails if `KlarvoTheme.kt` drifted from the canon CSS (ADR-0019) — **regenerate, don't hand-edit**.
- **Boot the unattended emulator ONLY via `scripts/android-emulator.sh` — never hand-roll `emulator -avd …`.** The script boots the AVD detached (`nohup`, so it survives the booting shell and is shared warm across steps) and arms a self-limiting **watchdog**: a hard TTL (`KLARVO_EMU_TTL_SECS`, default 7200s/120min) kills the AVD even if the run crashes or forgets — so a forgotten emulator can't peg ~8 cores indefinitely (born 2026-06-17, an orphaned `klarvo-emu` reparented to `init` did exactly that). **Stop it explicitly when done with `scripts/android-emulator.sh stop`** (conductor runs do this automatically at `conductor-guard release`). A direct `emulator -avd` call bypasses the reaper → orphan risk. Caveat for **shared/concurrent** use: the TTL is a blunt wall-clock backstop anchored to the *first* boot, **not** usage-aware — it can kill an emulator another agent is actively using at the boundary; the usage-aware option is the opt-in idle-reaper (`KLARVO_EMU_IDLE_SECS` + periodic `android-emulator.sh bump`).
- **Kotlin compiles and unit-tests run DEVICE-FREE.** `scripts/android-smoke.sh` copies `android/kotlin-src/` + `android/kotlin-test/` into the generated project and runs `./gradlew :app:testUniversalDebugUnitTest` as a hard gate. Logic regressions get caught there, not on the device — never claim "this needs a device" for a pure-logic change.
- **The real-device Android gate is YOURS to run, not the user's.** Pin the phone with `scripts/adb-pin.sh` (Tailscale IP + `adb tcpip 5555`), install the fresh APK yourself, then verify freshness before judging anything. Do not assume the user builds (Story 11-3/11-4 lesson).
- **Overlay occlusion has an objective proof:** `scripts/desktop-occlusion-proof.ps1` and `scripts/preview-occlusion-proof.ps1`. Run them instead of eyeballing a screenshot.
- **Bind tests to the real code paths/files they cover**, not to a parallel mock — divergence otherwise goes undetected (Epic-1 lesson; a real paste-path leak was caught this way).

### Code Quality & Style Rules

- **Code and comments are English. Chat/discussion is German.** Commit subjects English.
- **Naming:** module files `snake_case.rs`, traits `PascalCase`, functions `snake_case`. Kotlin classes `PascalCase` under `com.klarvo.voice`.
- No `rustfmt.toml` / `clippy.toml` — defaults apply. Match surrounding code style.
- **Factor out only on proven duplication** (≥2 real consumers). No premature abstraction; keep helpers module-local until a second consumer appears.

### Development Workflow Rules

- **Canonical Windows build is `scripts/sync-and-build.ps1`** (robocopy from `\\wsl$\…\products\klarvo` → `D:\apps\klarvo` → Tauri build). Dev runs in WSL; the build runs on Windows. **Signing gotcha:** the Tauri signer hangs → run `rsign` afterwards (`scripts/sign-installer.sh`).
- **The Windows build runs `npm ci`, never `npm install`.** Each JS Tauri plugin must match its Rust crate; `npm install` re-resolves and breaks the build. If the lock is out of sync, fix it at the SOURCE (run `npm install` in WSL, commit `package-lock.json`) — never work around it on the Windows side.
- **Build freshness differs per platform.** Desktop: Settings → About shows `Build <hash> · <timestamp>`, served by the `get_build_info` command. Android: there is **no in-UI version screen** — verify a fresh APK landed via `scripts/android-build.sh`'s timestamp gate + APK filename (or `adb … lastUpdateTime`), NOT a version number.
- **Unattended multi-story runs (epic-/story-conductor) read their orchestration contract from `_bmad/custom/bmad-epic-conductor.toml`** — run-guard, smoke surface, `BMAD_CONDUCTOR` flag, and visual-oracle posture live there (generator-immune, conductor-only), NOT in this file. Do not re-add conductor mechanics here.
- **`v1-ship` is the canonical line.** Branch every build branch off `v1-ship` and merge back into it. `main` carries a **disjoint** v0.4.x history (`v1-ship` is 665 commits ahead, `main` 20 ahead) — it is NOT a merge target. Check `fetch` + ahead/behind at session start; the checkout has silently trailed `origin` for weeks before.
- **Commits:** small and scoped, **never `git add .`**. Keep BMAD planning/story artifacts committed per-story.

### Critical Don't-Miss Rules

- **Release-Build blind spot.** `cargo check` + Linux tests are green while Tauri runtime, Windows-only paths, and signing are still broken. Treat "compiles on Linux" as nearly zero signal for surface features.
- **Never make the user the rendering oracle ("ich bin die Test-Maschine").** For a visual / rendering / geometry defect that is only observable on the real build (transparent windows, OS compositing, DPI, native widgets, separate-window paint): do **NOT** change app code on a hypothesis and have the user build + test to find out — that turns the user into the test machine and burns a whole build cycle per guess. Instead: (1) first get the defect into something **you** can observe or deterministically isolate — a self-contained reproduction the user flips through once, a zoomed screenshot, instrumented logging of the computed geometry/styles — and **name the cause**; (2) only then change app code, **once**; (3) if you genuinely cannot observe it and have no isolated cause, **say so and request the one specific observation you need** — never iterate blind. A failed surface smoke re-enters the gated dev flow (`bmad-dev-story` / `bmad-quick-dev`); it is **never** hot-patched from the bare main loop. (Born 2026-06-07: the Epic-6 preview-corner artifact was debugged via repeated guess→build→test cycles — exactly this anti-pattern. Memory `feedback-surface-feature-operable-ux`.)
- **BYOK is non-negotiable — no remote telemetry.** No Sentry, no analytics calls. Logging is local files + a user-triggered zip export only. Do not add network calls that phone home.
- **Never hardcode API/license keys.** Dev keys live in `.dev-keys/` (gitignored). License validation is offline HMAC + LemonSqueezy.
- **State-file writes are single-writer + atomic** (write-temp-then-rename, one owner). Don't add a second writer to a config/state file. See ADR-0015.
- **Multi-symptom upstream check.** When several subsystems report the same failure ("X doesn't arrive"), find the shared upstream cause — don't debug each subsystem in isolation.
- **A green measurement can prove the wiring and still miss the design claim.** "The value reaches the renderer" is a *wiring* statement; "the scale is right" is a *design* statement. They need separate evidence. (Story 11-6: Android's `setLineSpacing` multiplies the natural line box — measured 1.3285 — while desktop multiplies the font size. Both numbers measured green; the platforms still diverged.) When two platforms carry the same setting, compare both against one stated target.
- **Verbatim is the default output style.** The "Polished" cleanup style over-edits and loses user intent — be conservative when changing cleanup behavior.

## Usage Guidelines

- AI agents implementing code in this repo should read this file first.
- This file is a **lean rules digest**, not the authority. For decisions and rationale see `docs/adr/` (0015 state writes · 0016 Android parity · 0017 shared-core STT · 0018 bubble rendering · 0019 design SSOT · 0020 WebView2 pin · 0021 native overlays). For the visual canon see `docs/design/overhaul/source/` (HTML + `klarvo.css`, ADR-0019). For deferred work see `docs/backlog.md`. For the audit/remediation context see `docs/robustness-audit-2026-05-30.md` and `_bmad-output/planning-artifacts/epics.md`.
- Keep this file lean. Add a rule only when it is unobvious AND prevents a real mistake. Remove rules that become obvious or obsolete.

Last Updated: 2026-08-17
