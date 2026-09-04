# Klarvo Architecture — v1-ship

**Status:** Living · **Last updated:** 2026-06-17 · **App version:** 0.5.0 · **Branch:** `v1-ship`

## What this document is (and is not)

This is the **front door** to Klarvo's architecture: a current system map plus an index
that ties together the decisions and rules that already live elsewhere. It exists because
the architecture truth was scattered (ADRs + a rules digest + an audit) with **no single
navigable map** — `docs/index.md` itself notes "no full project scan has been run".

- This is **not** the rules digest agents read first — that is
  [`_bmad-output/project-context.md`](../_bmad-output/project-context.md).
- This is **not** where decisions are made — those are [`docs/adr/`](./adr/).
- This **points to** those authorities rather than copying them. When they change, this
  doc gets a one-line update and a pointer, never a second copy. (See *Keeping this true*.)

> ⚠️ A previous architecture doc lived in the **team repo** (`teams/klarvo/knowledge/architecture.md`)
> and went stale (~2 months untouched) because it sat away from the code. This doc lives
> **next to the code** on purpose, and is deliberately thin: index + map, not prose canon.

---

## 1. What Klarvo is

A source-available, BYOK voice-dictation app for **Windows desktop** and **Android**:
global hotkey / floating bubble → record mic → STT → LLM cleanup → paste into the focused
field, with offline-capable paths and no remote telemetry. Business Source License 1.1
(source-available, not open source; SSOT is `LICENSE` at the repo root).

Pipeline shape (both platforms): **record → transcribe (STT) → clean up (LLM) → sanitize →
paste → persist (history) → optional sync.**

---

## 2. Current state (2026-06-17)

`v1-ship` is the **shipping product** — a brownfield Tauri+Kotlin v1. (A v2 BMAD-greenfield
blueprint with a plugin/uniffi architecture exists in-repo but is **archive/reference only** —
do not apply its patterns, versions, or APIs to v1. See `project-context.md` top note.)

Epic status (authority: [`sprint-status.yaml`](../_bmad-output/implementation-artifacts/sprint-status.yaml)):

| Epic | Theme | Status |
|------|-------|--------|
| 1 | Config/state persistence hardening (ADR-0015) | **done** |
| 2 | Android security guardians (ADR-0016) | **done** |
| 3 | Test integrity — close the false-safety islands | **done** |
| 4 | God-file depth refactor (behavior-preserving) | **done** |
| 5 + 6 | Live-cleanup-preview (feature) | **done** |
| 7 | Cross-platform parity | **parked** 2026-06-13 — but **7-3 done** (see §3) |
| 8 | Desktop visual overhaul ("Studio Dark") | **in-progress** (live strand; 8-1 done) |
| 9 | Android visual overhaul + bubble interaction | **in-progress** (9-5 in review) |

The remediation epics (1–4) and the biggest structural drift driver (7-3, §3) are **done**.
The active work is the visual overhaul (8 desktop, 9 android).

---

## 3. The two-platform model — the central structural fact

Klarvo renders the same product two ways, maintained **independently**:

- **Desktop:** Tauri 2 — a **Rust core** (`src-tauri/`) plus a **React/Tailwind webview**
  (`src/`). The frontend calls the Rust core through Tauri IPC commands.
- **Android:** **native Kotlin** (`android/kotlin-src/com/klarvo/voice/`), not a webview.
  It **bypasses Tauri IPC for ~85% of behavior** — Kotlin talks to Groq over HTTP and to
  the Rust core over a **raw JNI bridge** directly.

**Consequence — and the #1 historical source of cross-platform drift:** shared behavior
either has to be *mirrored* in both the Rust and Kotlin paths, or *single-sourced* in Rust
and consumed over JNI. Getting this wrong is what produced the divergences the remediation
epics fixed.

**The dominant shared logic is now single-sourced (no longer duplicated):**

- **STT path** — request + output guards (hallucination filter, prompt-echo, fragment-strip)
  + pre-STT silence filter — lives **only in the Rust core** and Android consumes it over
  JNI (`src-tauri/src/stt/jni_bridge.rs`, `stt/groq_jni.rs`; Kotlin side `GroqSttBridge.kt`).
  The old Kotlin twins (`HallucinationFilter.kt`, `SilencePreFilter.kt`,
  `KlarvoApi.transcribe`) were **deleted**. **Hard rule (ADR-0017): a parallel Kotlin
  re-implementation of any STT/guard behavior is forbidden.**
- **License validation** — shared in Rust over JNI (`src-tauri/src/license/jni.rs`).
- **Design tokens** — generated, not hand-copied (see §7).

**Deliberately still platform-local** (accepted asymmetry, ADR-0016 Amendment 2; tracked in
the parked Epic 7, *not* bugs): text chunking (7-1), the live auto-stop VAD gate (7-2 — a
realtime frame stream over JNI is intentionally out of scope), and LLM-provider routing (7-5).

> Rule of thumb when editing shared behavior: **config keys, silence/VAD thresholds, paste
> logic, anything user-observable** — if it isn't single-sourced in Rust, it must be changed
> in BOTH paths. See `project-context.md` → "Android bypasses Tauri IPC".

---

## 4. System map

### 4.1 Rust core — `src-tauri/src/` (~31k LOC)

| Module | LOC | Role |
|--------|-----|------|
| `config/mod.rs` | 4242 | `AppConfig` (SSOT, → `config.json`), load/migrate/normalize, single sanctioned write path (`save_config_locked`, ADR-0015 / story 4-3). Largest file; see debt §8. |
| `pipeline.rs` | 4081 | The dictation orchestrator: record → STT → cleanup → paste, state transitions, retries, offline fallbacks, per-stage Tauri events. |
| `audio/mod.rs` | 2435 | cpal capture (desktop), WASAPI fallback, device resilience, WAV encode. |
| `commands/settings.rs` | 2158 | Tauri command handlers for settings load/save. |
| `llm/mod.rs` (+`local.rs`) | 2063 | Cleanup providers (DeepSeek/Groq/OpenAI/OpenRouter/offline Qwen), 3 styles, PI/sanitization. |
| `lib.rs` | 1611 | Tauri bootstrap, `AppState`, command registry. |
| `license/` (`mod`,`ls_client`,`jni`) | 1270+ | Offline HMAC + Lemon Squeezy; shared to Android via JNI. |
| `history/mod.rs` | 983 | SQLite (`history.db`) + Turso HTTP sync. |
| `stt/` (`mod`,`hallucination`,`jni_bridge`,`groq_jni`,`local_whisper`,`model_manager`) | 954 + … | STT providers + the shared-core JNI path + guards. |
| `voice_command/` | 864 | "Klarvo stop"-style command mode (desktop). |
| `vad/mod.rs` | 715 | Voice-activity detection (Silero / RMS). |
| `sync/mod.rs` | 626 | Turso push/pull. |
| `paste/mod.rs` | 593 | Win32 `SendInput` (desktop). |
| `dictionary/`, `hotkey/`, `commands/*`, `fs.rs` | — | Custom dictionary, global hotkey slots, IPC endpoints, fs helpers. |

### 4.2 Frontend — `src/` (~13k LOC, React 19 + TS strict + Tailwind 4)

`App.tsx` (main container) · `Onboarding.tsx` (1579, two-track wizard) · `FloatingBar.tsx`
(docked status pill, waveform) · `PreviewPanel.tsx` (live cleanup preview) ·
`tauri-commands.ts` (1087, typed IPC wrappers) · `types.ts` · `components/` (Settings tree,
model managers, dashboards, `ui.tsx` atoms, `settings/` subpages) · `hooks/` (useSettings,
useRecording, useLicense, usePanels, …). Styling: `styles.css` holds the `--k-*` design
tokens (§7).

### 4.3 Android — `android/kotlin-src/com/klarvo/voice/` (~7.8k LOC, minSdk 24)

`KlarvoOverlayService.kt` (1971 — overlay window, touch/drag, mode switching, owns the
native pipeline) · `KlarvoApi.kt` (1092 — HTTP/JNI calls; STT request removed, now via
`GroqSttBridge.kt`) · `FloatingBubbleView.kt` + `ListeningPanelView.kt` (View+Canvas bubble,
ADR-0018) · `KlarvoAudioRecorder.kt` (AudioRecord→WAV) · `KlarvoAccessibilityService.kt`
(paste + keyboard-state) · `BankingAppBlocklist.kt`/`BankingGuard.kt` (sensitive-app guard) ·
`KlarvoTheme.kt` (**generated** from CSS tokens, §7) · `Local{Whisper,Llm}Inference.kt`,
`LicenseValidator.kt`, `DebugHarnessReceiver.kt` (state harness, ADR-0018).

### 4.4 Persistence & config

- **`config.json`** (in AppData) is the **single source of truth** for all settings, API
  keys (plaintext today — OS-keystore is noted future work), license, hotkey slots. Not
  SQLite, not Tauri storage. Atomic + single-writer writes (ADR-0015).
- **`history.db`** (SQLite) is the *only* DB — recording history; UUID PKs; synced to Turso.

---

## 5. Binding rules (authority: `project-context.md` + ADRs)

The unobvious ones, with pointers — do not treat this list as complete; read the digest:

- **Platform-gate heavy deps** (`whisper-rs`, `llama-cpp-2`, `cpal`, `arboard`, `jni`,
  `windows`) behind `#[cfg(...)]`. Never break the Android/Linux build.
- **No panics** — structured `Result`/`AppError`, fail-soft; no `todo!`/`unimplemented!`.
- **Tauri event names use colons** (`klarvo:state-changed`), never dots.
- **State-file writes are atomic + single-writer** (ADR-0015). Don't add a second writer.
- **Linux `cargo test` is near-zero signal for surface features** — a real **Windows
  release build + manual press-to-paste smoke** is the DoD gate; run
  [`docs/surface-smoke-checklist.md`](./surface-smoke-checklist.md) first.
- **Never make the user the rendering oracle** — isolate/observe a visual defect yourself
  before changing app code; never iterate guess→build→test blind. (`project-context.md`.)
- **BYOK, no telemetry** — no Sentry/analytics/phone-home. **Never hardcode keys.**
- **Factor out only on proven duplication (≥2 consumers).** No premature abstraction.

---

## 6. Build & run

- **Windows:** `scripts/sync-and-build.ps1` (robocopy WSL → `D:\apps\klarvo` → Tauri build).
  Signer hangs → run `rsign` after (`scripts/sign-installer.sh`). Dev in WSL, build/test on Windows.
- **Android:** `scripts/android-build.sh`; smoke via `scripts/android-smoke.sh` (runs the
  token-drift `--check` gate, §7). No in-UI version screen — verify freshness by timestamp/APK.

---

## 7. Design system (visual SSOT)

- **Canon = `docs/design/overhaul/source/`** (+ `MANIFEST.md` with provenance fingerprints)
  is the single source of truth for **visual tokens, color semantics, and interaction
  meaning**, cross-platform (ADR-0019).
- **Tokens are generated, not hand-copied.** `--k-*` custom properties in `src/styles.css`
  are the source; **Android `KlarvoTheme.kt` is codegen'd** from them
  (`scripts/gen-android-theme.mjs`, story 9-10 **done**) with a **build-gate** (`--check`
  fails if hand-edited). **Regenerate, never hand-edit Kotlin tokens.**
- **Color semantics are a codified rule** (ADR-0019 §3): **teal** = brand/ready/processing/
  success/focus · **amber** = live/recording only · **danger/red** = destructive
  (cancel/discard/delete/error) **only**, never the primary send/confirm. Send/confirm is a
  non-red affordance.
- Layout is *not* shared (desktop docked pill vs. android floating bubble + panel); **meaning**
  is shared. Android bubble is View+Canvas, not Compose (ADR-0018).

---

## 8. Known limitations (current state)

Factual gaps in what the system does today — this is part of the *state description*, not a
plan. The **"Owned by"** column points to the planned work item that covers a gap;
**"— not yet owned"** means no plan covers it yet. Those un-owned gaps are surfaced here for
visibility only — the call to act on them lives in the backlog / a new story (see
`docs/backlog.md`), never in this doc.

| Limitation (what the system does *not* do today) | Owned by |
|---|---|
| Desktop has **no token-enforcement gate** — raw hex/`rgba()` can be hardcoded in `src/` components (e.g. `FloatingBar.tsx`); only Android has the `--check` codegen gate. | **— not yet owned** (Epic 8 re-skins surfaces, but no enforcement-gate story exists). |
| **No full mechanical project map** — `document-project` has never run a full scan. | **— not yet owned** (BMAD `document-project` re-scan would produce it). |
| `config/mod.rs` is 4242 LOC (largest file). | Epic 4 (done) — `load_config` core isolated (4-1), single write path (4-3); remaining size accepted. |
| Chunking / live-VAD / LLM-routing are duplicated per platform. | ADR-0016 A2 — deliberate asymmetry; Epic 7 (parked): 7-1, 7-2, 7-5. |
| Dead config keys (settable, consumed nowhere) + no golden-vector parity net. | Epic 7-7 (backlog) + `docs/backlog.md`. |
| API keys are plaintext in `config.json`. | Noted future (OS-keystore) — `project-context.md`. |

---

## 9. Where truth lives (authorities)

- **Agent rules digest (read first):** [`_bmad-output/project-context.md`](../_bmad-output/project-context.md)
- **Decisions + rationale:** [`docs/adr/`](./adr/) — 0015 state writes · 0016 Android parity (+A1/A2) · 0017 shared-core STT · 0018 bubble rendering · 0019 design SSOT
- **Remediation context:** [`docs/robustness-audit-2026-05-30.md`](./robustness-audit-2026-05-30.md) *(pre-remediation; many findings now fixed — cross-check against sprint-status)*
- **Epics / stories / status:** `_bmad-output/planning-artifacts/epics*.md`, [`sprint-status.yaml`](../_bmad-output/implementation-artifacts/sprint-status.yaml)
- **Design canon:** `docs/design/overhaul/source/MANIFEST.md`
- **Surface traps:** [`docs/surface-smoke-checklist.md`](./surface-smoke-checklist.md)
- **Conductor contract:** `_bmad/custom/bmad-epic-conductor.toml`

---

## 10. Keeping this true (anti-staleness)

This doc earns its place only if it stays the **thin index**, not a second canon:

1. When an ADR lands or an epic's status flips, add/adjust **one line** here + a pointer.
   Never copy an ADR's content in — link it.
2. Regenerate the mechanical code map via BMAD `document-project`; this doc stays the
   human-facing tie-together layer above it.
3. If this file starts duplicating `project-context.md` or the ADRs, delete the duplication.
   The failure mode to avoid is the stale `teams/.../architecture.md` — distance from the
   code + prose accretion is what killed it.
</content>
</invoke>
