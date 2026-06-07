---
project_name: 'klarvo'
user_name: 'Andi'
date: '2026-06-05'
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
rule_count: 29
optimized_for_llm: true
---

# Project Context for AI Agents

_This file contains critical rules and patterns that AI agents must follow when implementing code in this project. Focus on unobvious details that agents might otherwise miss._

> **Active branch is `v1-ship` — this is a brownfield v1 product (Tauri desktop + native Kotlin Android).** The repo also contains a v2 BMAD-greenfield blueprint (plugin architecture, uniffi/manifest executor, `jni 0.22`). **v2 is archive/reference — do NOT apply its patterns, versions, or APIs to v1 code.** When a doc or memory mentions plugins, manifest executor, Phase-N, or Epic-N greenfield work, it is v2-historical.

---

## Technology Stack & Versions

**Desktop backend (Rust):** Tauri 2 · tokio 1 · reqwest 0.12 (`rustls-tls`, `default-features = false`) · rusqlite 0.32 (`bundled`) · serde 1 · anyhow 1 + thiserror 2 · cpal 0.15 (desktop audio) · arboard 3 (clipboard) · voice_activity_detector 0.2.1 · windows 0.61 (Win32 APIs)

**Offline inference (target-gated):** whisper-rs 0.15.1 (Windows + Android only) · llama-cpp-2 0.1.140 (**Windows only** — needs libclang + CMake)

**Frontend:** React 19.1 · TypeScript 5.8.3 · Vite 7 · TailwindCSS 4.2 (`@tailwindcss/vite`) · `@tauri-apps/api` 2 · ESM (`"type": "module"`)

**Android:** native Kotlin (minSdk 24) · `jni 0.21` (raw JNI bridge to Rust) · once_cell 1 (global model cache) · AudioRecord + AccessibilityService + overlay (`TYPE_APPLICATION_OVERLAY`)

**Crate layout:** bin `klarvo` (`src-tauri/src/main.rs`, thin) → lib `klarvo_lib` (crate-type `staticlib`+`cdylib`+`rlib`). App version 0.5.0. Tauri identifier `com.klarvo.voice`.

**Version constraints:** `jni` is pinned at **0.21** (NOT 0.22 — that is v2). `reqwest` MUST keep `default-features = false` + `rustls-tls` (no native OpenSSL). whisper-rs/llama-cpp-2 are **not** available on Linux/macOS builds.

## Critical Implementation Rules

### Language-Specific Rules

- **Platform-gate heavy deps.** `whisper-rs`, `llama-cpp-2`, `cpal`, `arboard`, `jni`, `windows` are all behind `#[cfg(...)]` targets in `Cargo.toml`. Never add an unconditional dependency or `use` that breaks the Android or Linux build. Mirror the existing `cfg(target_os = ...)` / `cfg(windows)` gates.
- **Errors are structured `Result`, never panics.** Use `thiserror`/`anyhow`. Scaffolds and not-yet-implemented paths return a structured `AppError` — **never** `todo!()`, `unimplemented!()`, or `panic!()` (fail-soft pattern).
- **No `debug_assert!` with side-effects.** It compiles out in release → silent behavior divergence on Windows.
- **TypeScript runs in strict mode** (`tsc` gates `npm run build`). ESM only.

### Framework-Specific Rules

- **Tauri event names use colons, never dots:** `klarvo:state-changed`, not `klarvo.state-changed`. Tauri reserves `.` in event strings. (The ADR-0002 dot-rule applies only to Core-Bus `Event::*` variants, which are v2 — not present in v1.)
- **Android bypasses Tauri IPC (~85%).** Kotlin (`KlarvoApi.kt`, `LocalWhisperInference.kt`) talks to Groq HTTP and the Rust JNI bridge directly — NOT through Tauri commands. **Any change to shared behavior (config keys, silence/VAD thresholds, paste logic) must be mirrored in BOTH the Rust path AND the Kotlin path.** This is the #1 source of cross-platform drift. See ADR-0016 (Android path parity).
- **Config is the single source of truth, in `config.json`** (in AppData, loaded via `load_config_reporting`) — NOT SQLite and NOT Tauri storage. All settings (`groq_api_key`/other API keys, `license_key`, `hotkey_slots`, languages) live in `AppConfig` → `config.json`. The only SQLite DB is `history.db` (recording history). API keys are currently plaintext in `config.json`; an OS-keystore is a noted future improvement, not yet implemented. (A v2-era note claimed a SQLite `config.db` + a "hotkey boot bypasses DB" bug — that was the abandoned v2 architecture; v1's boot path reads `hotkey_slots` from the same `config.json` the UI save writes.)
- **The hotkey fires an async pipeline**, it does not block in the OS callback. Pipeline lives in `src-tauri/src/pipeline.rs` (STT → cleanup → paste), emitting a Tauri event per stage.
- **Provider trait pattern:** STT/LLM providers are swappable via config (`stt_provider`, `cleanup_provider`). Add new providers behind the existing trait, don't special-case in the pipeline.

### Testing Rules

- **Tests are inline `#[cfg(test)]` modules**, not a separate `tests/` tree. Snapshot tests use `insta`; snapshots live in `src-tauri/src/snapshots/`. Accept with `cargo insta review`.
- **Linux `cargo test` + lint do NOT satisfy the DoD for surface/UI stories.** Hard gate: a real **Windows release build + manual press-to-paste smoke** is required. (`cargo check` and Linux tests mask Tauri-runtime bugs and Windows-only code paths.) **Before the smoke, run the applicable items of `docs/surface-smoke-checklist.md`** — the running ledger of traps that are green on Linux (camelCase config keys, Settings resync-`useEffect`, FloatingBar separate-window reactivity, window-geometry/region clip, event push-wiring). Mechanical check, not a self-attestation (Epic-5 retro AI-1).
- **Android changes require an on-device smoke** via `scripts/android-smoke.sh` before a story is done.
- **Bind tests to the real code paths/files they cover**, not to a parallel mock — divergence otherwise goes undetected (Epic-1 lesson; a real paste-path leak was caught this way).

### Code Quality & Style Rules

- **Code and comments are English. Chat/discussion is German.** Commit subjects English.
- **Naming:** module files `snake_case.rs`, traits `PascalCase`, functions `snake_case`. Kotlin classes `PascalCase` under `com.klarvo.voice`.
- No `rustfmt.toml` / `clippy.toml` — defaults apply. Match surrounding code style.
- **Factor out only on proven duplication** (≥2 real consumers). No premature abstraction; keep helpers module-local until a second consumer appears.

### Development Workflow Rules

- **Canonical Windows build is `scripts/sync-and-build.ps1`** (robocopy from `\\wsl$\…\products\klarvo` → `D:\apps\klarvo` → Tauri build). Dev runs in WSL; Andy builds/tests on Windows. **Signing gotcha:** the Tauri signer hangs → run `rsign` afterwards (`scripts/sign-installer.sh`).
- **Android build/sign/freshness:** `scripts/android-build.sh`. There is **no in-UI version/About screen** — verify a fresh APK landed via the build script's timestamp gate + APK filename (or `adb … lastUpdateTime`), NOT a version number.
- **When touching `shells/windows/`**, verify the Windows cross-compile before closing: `cargo check --target x86_64-pc-windows-gnu` (Linux tests mask Win-shell bugs).
- **Commits:** small and scoped, **never `git add .`**. Branch off `main`; do not commit directly to `main`. Keep BMAD planning/story artifacts committed per-story.

### Critical Don't-Miss Rules

- **Release-Build blind spot.** `cargo check` + Linux tests are green while Tauri runtime, Windows-only paths, and signing are still broken. Treat "compiles on Linux" as nearly zero signal for surface features.
- **Never make the user the rendering oracle ("ich bin die Test-Maschine").** For a visual / rendering / geometry defect that is only observable on the real build (transparent windows, OS compositing, DPI, native widgets, separate-window paint): do **NOT** change app code on a hypothesis and have the user build + test to find out — that turns the user into the test machine and burns a whole build cycle per guess. Instead: (1) first get the defect into something **you** can observe or deterministically isolate — a self-contained reproduction the user flips through once, a zoomed screenshot, instrumented logging of the computed geometry/styles — and **name the cause**; (2) only then change app code, **once**; (3) if you genuinely cannot observe it and have no isolated cause, **say so and request the one specific observation you need** — never iterate blind. A failed surface smoke re-enters the gated dev flow (`bmad-dev-story` / `bmad-quick-dev`); it is **never** hot-patched from the bare main loop. (Born 2026-06-07: the Epic-6 preview-corner artifact was debugged via repeated guess→build→test cycles — exactly this anti-pattern. Memory `feedback-surface-feature-operable-ux`.)
- **BYOK is non-negotiable — no remote telemetry.** No Sentry, no analytics calls. Logging is local files + a user-triggered zip export only. Do not add network calls that phone home.
- **Never hardcode API/license keys.** Dev keys live in `.dev-keys/` (gitignored). License validation is offline HMAC + LemonSqueezy.
- **State-file writes are single-writer + atomic** (write-temp-then-rename, one owner). Don't add a second writer to a config/state file. See ADR-0015.
- **Multi-symptom upstream check.** When several subsystems report the same failure ("X doesn't arrive"), find the shared upstream cause — don't debug each subsystem in isolation.
- **Verbatim is the default output style.** The "Polished" cleanup style over-edits and loses user intent — be conservative when changing cleanup behavior.

## Usage Guidelines

- AI agents implementing code in this repo should read this file first.
- This file is a **lean rules digest**, not the authority. For decisions and rationale see `docs/adr/` (ADR-0015 state writes, ADR-0016 Android parity). For the audit/remediation context see `docs/robustness-audit-2026-05-30.md` and `_bmad-output/planning-artifacts/epics.md`.
- Keep this file lean. Add a rule only when it is unobvious AND prevents a real mistake. Remove rules that become obvious or obsolete.
