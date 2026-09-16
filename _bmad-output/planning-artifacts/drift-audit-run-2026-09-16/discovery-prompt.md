# Cross-platform drift discovery — Klarvo, frozen tree `e9543fe` (branch v1-ship), 2026-09-16

You are one of two independent discovery readers (A/B run, two different models, identical prompt).
Your findings will be verified claim-by-claim against the real code by separate readers, then merged.
Precision matters as much as recall: every claim needs `file:line` evidence on BOTH sides, and a
confidence tag. A wrong claim costs a verifier a full read; an unsupported claim is discarded.

## Repo and paths (all absolute under /home/andyon2/workspace/products/klarvo)

- Desktop runtime (Rust, Tauri): `src-tauri/src/` — key files: `config/mod.rs` (AppConfig struct,
  serde camelCase keys, defaults, migrations), `pipeline.rs` (record→STT→cleanup→paste orchestration,
  degrade paths, fallback ladder), `llm/mod.rs` (cleanup prompts, chunking, providers), `llm/local.rs`,
  `stt/mod.rs`, `stt/hallucination.rs`, `stt/groq_jni.rs` (the SHARED STT path Android calls via JNI),
  `audio/mod.rs`, `vad/mod.rs`, `paste/mod.rs`, `sync/mod.rs` (Turso history sync), `license/`,
  `commands/` (Tauri commands the React UI calls), `native_pill.rs`/`native_preview.rs` (Win32 overlays).
- Android runtime (Kotlin, bypasses Tauri IPC ~85%): `android/kotlin-src/com/klarvo/voice/` — key files:
  `KlarvoApi.kt` (Config data class + JSON parsing ~lines 170-560, cleanup providers, chunking, prompts),
  `KlarvoOverlayService.kt` (recording → STT via `GroqSttBridge` → cleanup → paste, degrade paths),
  `KlarvoAudioRecorder.kt` (VAD gate, RMS), `KlarvoAccessibilityService.kt` (paste, enter),
  `GroqSttBridge.kt` (JNI into Rust STT), `LicenseValidator.kt`, `BankingGuard.kt`, `LocalLlmInference.kt`.
- Shared React UI (runs on BOTH platforms — on Android inside `TauriActivity`): `src/components/`,
  esp. `SettingsPanel.tsx`, `AdvancedSettingsPanel.tsx`, `settings/*Content.tsx`, `src/platform.ts`
  (`isMobile` / `isDesktop`), `src/types.ts`, `src/tauri-commands.ts`.
- Android JVM tests (may reveal intended contracts): `android/kotlin-test/`. Rust tests inline (`#[cfg(test)]`).
- Shared golden-vector fixtures: `test-fixtures/*.json` + `test-fixtures/README.md`.

READ-ONLY. Do not edit files, do not run git commands that change anything, do not build.
Use grep/sed/cat freely. Work from the tree as it is; do not `git checkout` anything.

## Definition — what counts as a divergence

Scope follows ADR-0016 (`docs/adr/0016-android-path-parity-strategy.md`) as amended:
**same config + same input → different observable behavior across platforms, with no error shown.**
Concretely, five classes — report all of them:

1. **Core-output determinism:** same audio/text/config yields different dictation output (chunking
   rules, prompt content, join separators, guards/filters, sanitization, model/temperature sent).
2. **Config-contract integrity:** a key the shared React UI lets the user SET (on that platform!) that the
   runtime of that platform never READS, or reads with a different default/unit/semantics. To judge
   "settable on Android", check the React render site: a control that is NOT behind `isDesktop` /
   `!isMobile` renders on Android. A control behind `isMobile` renders only on Android — then check the
   Rust side too (reverse direction). Also report keys set on one side and consumed on neither.
3. **Degrade / failure paths:** what happens on STT failure, cleanup failure, clipboard failure, missing
   focus, banking-app foreground, empty transcript, hallucination hit — is the outcome (paste / clipboard
   only / toast / history entry / auto-send suppressed) the same on both sides?
4. **Persistence & sync:** history entries (fields, statuses like pending/failed, raw_text), Turso push/pull,
   config write paths — same schema and semantics?
5. **Guards:** license/trial enforcement, banking blocklist, sanitization, hallucination filter,
   prompt-echo/fragment guards — present on both, same thresholds?

NOT in scope (do not report): pure UI look/feel; Win32-overlay vs Android-bubble rendering differences;
things that differ because the platform genuinely cannot do them (e.g. Bluetooth mic routing) — unless a
user-settable control still pretends it works there.

Severity: **CRITICAL** = data integrity / security / license / silent data loss. **HIGH** = core dictation
output differs at default settings, or a set user value silently dies on one side. **MEDIUM** = differs only
when tuned away from defaults, or edge-case behavior. **LOW** = cosmetic, wire-level, identical outcome.

## Method (do it in this order)

1. **Config contract spine.** Enumerate every field of Rust `AppConfig` (+ nested `advanced`, bubble, etc.)
   with its camelCase key. For each key record: (a) Rust runtime reader? (file:line or "none"),
   (b) Kotlin reader? (file:line or "none" — note `Config` data class parsing is not a reader; find the
   USE), (c) React render site + platform gate (`isDesktop`/`isMobile`/none/not rendered).
   Keep this table — it is your first deliverable and the verifiers' map.
2. **Twin-module walk.** Compare side by side: chunking; cleanup system prompts per style incl. dictionary,
   custom prompt, translation, sandwich defence; provider selection + fallback ladder + retryable predicate;
   local LLM prompt; STT request parameters (Rust `groq_jni.rs` ↔ what Kotlin passes in); pre-STT silence
   filter; VAD gate thresholds/units/floors; hallucination + stockphrase filters; paste/clipboard/enter
   sequence and its degrade branches; history entry writing; Turso sync; license gate; banking guard.
3. **Degrade-path matrix.** For each failure listed in class 3, write the outcome per platform in one line.
4. **Only now** read `docs/cross-platform-drift-audit.md` (the June 2026-06-10 audit). For every June row
   (C1..C2, H1..H17, M1..M16, L1..L7, dead-config cluster, recall #1..#5) state today's status:
   `FIXED (evidence file:line)` / `PERSISTS` / `CHANGED (how)` / `CANNOT VERIFY`. Tag your own findings
   that match a June row with its ID; tag genuinely new ones `NEW`.
5. Do NOT read `docs/backlog.md` or the ADR amendments for adjudication — adjudication is not your job.
   Report what the code does. (You may read `docs/adr/0017-shared-core-stt-path.md` to understand the
   JNI boundary.)

## Output — write ONE markdown file to the path given at the end of this prompt

Sections, in this order:

### 0. Run header
Model name you are, tree hash `e9543fe`, files you actually read (list), time spent roughly.

### 1. Config contract table
`| key | Rust reader | Kotlin reader | React render + gate | verdict |` — verdict ∈ {ok, dead-both,
dead-desktop, dead-android, android-visible-but-dead, desktop-visible-but-dead, unit/default-mismatch}.

### 2. Divergences by severity
One table per severity (CRITICAL, HIGH, MEDIUM, LOW):
`| # | Shared behavior | Desktop (file:line) | Android (file:line) | React gate | Note | June-ID/NEW | confidence |`
Number rows `A1, A2, …` (or `B1…` if you are reader B — the path tells you). confidence ∈ {high, med, low}.
State what the USER experiences in the Note, in one sentence.

### 3. Degrade-path matrix
`| failure | Desktop outcome (file:line) | Android outcome (file:line) | same? |`

### 4. June rows — status today
`| June ID | status | evidence |` for every June row.

### 5. Things you suspected but could not confirm
Short list. These help the recall sweep; keep them out of section 2.

Be terse in prose, exhaustive in tables. Every Desktop/Android cell carries `file:line`.
