# Deferred Work

Follow-ups surfaced during quick-dev runs but deliberately out of the current scope.

## From Task 2.2 (pipeline decision extract), review iteration 1 — 2026-05-29

- **Command-mode selection hold-window micro-delta.** After the extraction, `command_mode_active` / `command_mode_selected_text` are PEEKED+cloned before STT and only reset/`take`-n ("consumed") after `process_audio` returns. This preserves the *reset timing* (still after the post-STT guards) but widens the window during which `command_mode_selected_text` still holds its value (OLD `take()`-d it at the command point, before the LLM rewrite). Unobservable today: the `is_recording()` guard plus `start_command_mode`'s early-return-while-recording prevent re-entrancy during the window. Worth revisiting when command mode / the `is_recording` race is hardened (Task 2.3-adjacent) — ideally model command-mode state as an owned transition rather than two loosely-correlated flags. Source: `spec-2-2-pipeline-decision-extract.md` review iteration 1 (edge-case + blind reviewers).

## From Task 2.3 (is_recording race + session-lock poison), review iteration 1 — 2026-05-29

Surfaced by the edge-case reviewer; out of the spec's declared scope (session/monitor locks only). All in `src-tauri/src/audio/mod.rs` unless noted.

- ~~**Extend poison-recovery to `level_callback`.**~~ DONE 2026-05-30 (pulled forward right after 2.3): both `level_callback.lock().unwrap()` sites (`set_level_callback`, `start_recording`'s `.take()`) now route through `lock_recover`. `silence_config`/`live_buffer` already used `.lock().ok()` (nothing to do). The recorder's hot-path state locks are now uniformly poison-recovering.
- **Decouple the recording-start claim from device init (the deferred `SessionSlot` reserve refactor).** `start_recording` holds the `session` guard across the spawn + blocking `ready_rx.recv()` device-init window (audio/mod.rs 304→362). This is atomic but (a) makes `is_recording()` block during init, and (b) means `lock_recover`'s recovery is only sound while that window stays panic-free — a panic there could leave an orphaned recording thread, then a recovered lock spawns a second one. Fix: acquire/commit the session slot *after* `ready_rx.recv()` (an `Idle/Starting/Active` reserve state) so no lock is held across device I/O. This is the `SessionSlot` rewrite deliberately deferred in `spec-2-3-is-recording-race.md` Design Notes. Severity: medium (latent — no panic source in the window today). **A latent footnote, NOT a planned task** — only revisit via quick-dev if it stops being latent. Source: edge-case reviewer Finding 3.
- **Stale `recording_start = Some` window.** `start_recording_only` sets `recording_start` after the gate; if the task is interrupted/panics between gate-win and that write (e.g. in the bar-recreation block), the recorder is `Some`/recording while `recording_start` is `None`, so a later `stop_and_process_pipeline` reads `duration_ms = 0` → `TooShort` → silently discards a real dictation. Pre-existing (the diff did not worsen it); near-impossible trigger (the bar block only logs). Severity: low. Source: edge-case reviewer Finding 4.

## From Story 1.2 (backup-on-corrupt recovery in load_config) — 2026-05-31

Surfaced by the D1 surfacing-mechanism research; deliberately deferred per A5 scope-line + Premature-Abstraction-Guard (single consumer today). The load-bearing ROB-02 invariant (AC#2) does **not** depend on these — the corrupt backup is safe on disk regardless of whether the boot warning is delivered.

- **Reliable pull-based boot-warning surface.** Story 1.2 emits the corrupt-backup warning best-effort via `emit_pipeline_state(…, PipelineEvent::warn(msg))` over `klarvo://state-changed` after the main window is shown (`lib.rs`). Events emitted at boot race the React listener (`useRecording.ts:26-39`) and are typically **lost** — there is no queue and no pull command. Build a reliable surface: a `get_boot_warnings` Tauri command + a frontend mount-time fetch + a `QuickTip`/banner render, so a recoverable `.corrupt` backup is *proactively* communicated at boot instead of relying on the fire-and-forget toast. Residual risk of the deferral: *"a recoverable `config.json.corrupt-<ts>` backup exists but the user may not be proactively notified at boot"* ⊂ the risk already fixed *"recoverable data irreversibly destroyed."* Source: Story 1.2 Dev Notes **D1**.
- **`load_dictionary` corruption-backup parity.** `dictionary/mod.rs:117-137` has the byte-identical swallow-error-and-default pattern for `dictionary.json` (parse/read → `Dictionary::default()`), so a corrupt dictionary silently loses custom terms (recoverable-class data, lower severity than keys/license). Port the Story 1.2 `backup_corrupt_config` pattern (timestamped `.corrupt-<ts>` backup via `save_atomic` + warning) to `load_dictionary`. The future pull-based boot-warning center above is the natural shared home for both. Source: Story 1.2 "Project Structure Notes" + E7 scope discipline.

## From spec-defer-updater-signing-to-rsign (createUpdaterArtifacts: false) — 2026-05-31

Surfaced by the edge-case reviewer (Q2); explicitly out of scope for the flag change. Not urgent — Early Access is withdrawn, there is no CI generating `latest.json`, and the build that would have generated it was failing anyway, so no working auto-update flow is regressed.

- **Updater release manifest (`latest.json`) must be assembled manually.** With `bundle.createUpdaterArtifacts: false`, `tauri build` no longer generates the updater manifest or its signature. Before any future auto-update release, the release runbook must produce `latest.json` by hand — version, notes, per-platform installer URL, and the **rsign-produced** base64 signature (`sign-installer.sh` already emits the per-installer `.sig`) — and publish it to the updater endpoint (`https://github.com/andyon2/klarvo-app/releases/latest/download/latest.json`). Without it the runtime `tauri_plugin_updater` (registered `lib.rs:698`) fails **gracefully** — a 404 reads as "no update available", no crash — so the failure mode is a silent never-updates, not an error. Alternative: re-enable build-time signing with a valid `TAURI_SIGNING_PRIVATE_KEY` + password and drop the manual step. Belongs in the release runbook, not a code change. Source: edge-case reviewer Q2 + spec Design Notes.

## Deferred from: code review of 1-3-single-writer-serialization-for-state-file-saves (2026-05-31)

Surfaced by the Edge Case Hunter; out of Story 1.3's declared scope (config/mod.rs intentionally untouched per Dev Notes).

- **Migration writes in `load_config` are unguarded by `config_disk_write`** (`src-tauri/src/config/mod.rs:1162`, `:1211`, `:1240`). These `save_config(...)` calls run during boot legacy-field migration and do not hold the disk-write mutex. Safe today: the only caller is Tauri `setup()` (`lib.rs:723`), single-threaded, *before* `AppState` (and therefore the `config_disk_write` mutex) exists — there is no concurrency window. But the invariant introduced by Story 1.3 ("config.json is only ever written under `config_disk_write`") now has an undocumented boot-time exception. If `load_config` is ever called from a runtime command (e.g. a future "reload config" feature), these three writes would race the guarded savers. Cheap hardening: a one-line doc-comment on these sites noting the boot-only exception, or route them through the same lock if/when a runtime reload is added. Source: code review of Story 1.3, Edge Case Hunter.

- **✅ DONE 2026-05-31 — Story 4.3 (`4-3-single-sanctioned-config-write-path-save-config-locked`).** Pulled forward the same day as a one-off scope-fence exception; helper extracted, all 18 sites routed, `save_config`→`pub(crate)`, specs rebound, 3-layer review PASS. The text below is the original deferral note, kept for provenance. ~~**Extract a single `save_config_locked` choke-point so the disk-write invariant is structurally enforced (candidate Epic 4 story).**~~ *(Decision D1, Option 2 — chosen for follow-up by Andi 2026-05-31 over folding it into Story 1.3, to keep the robustness fix's diff tight and auditable.)* Story 1.3 rewired 17 production call sites in `commands/` (settings ×9, misc ×3, license ×4, voice_command ×2) to hand-hold `config_disk_write → config → modify → clone → drop config → save_config`. That pattern is currently **convention, not enforcement** — `config_disk_write` is a bare `Mutex<()>`, trivially bypassable, and the two new specs characterize the *pattern* inline rather than binding to the real commands (a future edit dropping a guard from a real call site would not turn the tests red). Extract a helper, e.g. `AppState::save_config_locked(&self, mutate: impl FnOnce(&mut AppConfig))` (or `save_config_with_lock(state, cfg)` per the story's own Dev Notes), that performs the whole locked cycle in one place; route all 17 sites through it; then the concurrency specs can exercise the real helper and a dropped guard becomes a compile/type concern instead of a reviewer's vigilance. Closes both the test-binding gap and the "no compile-time enforcement" gap in one move. No behavior change — fits Epic 4 (god-file depth refactor). **Formalize via `bmad-create-story`, not an improvised stub.** Source: code review of Story 1.3, D1 (Blind + Edge + Acceptance reviewers).

## Deferred from: code review of 1-4-hardened-config-migration (Opus 4.8 re-review, 2026-05-31)

Surfaced by the independent Opus re-review (requested because the Sonnet review was distrusted). Out of Story 1.4's tight scope; both are real but not actionable as in-story patches.

- **Unbounded pre-migration-backup accumulation.** `backup_pre_migration_config` (`config/mod.rs:1017-1052`) has no GC/retention. When a migration condition stays true every boot while `save_config` keeps failing in a way that does NOT also fail the backup write (e.g. `config.json` itself unwritable but the directory writable), each boot writes a fresh timestamped backup forever. If the *directory* is read-only/full the backup also fails (bounded), so the unbounded case is narrow. Mirrors the identical no-cleanup property of `backup_corrupt_config`, but the migration path genuinely re-fires every boot whereas corrupt-recovery is one-shot. A retention/GC policy (keep newest N, age-out) should span BOTH `.pre-migration-*` and `.corrupt-*` backups — natural fit for a future boot-warning/maintenance story. Source: code review of Story 1.4, Blind + Edge Case Hunters.

- **DoD Windows cross-compile gate not executed.** The story DoD lists `cargo check --target x86_64-pc-windows-gnu` on touched files; the target is not installed in this WSL env (confirmed `rustup target list --installed` shows no windows target). The new code uses only cross-platform APIs (`std::fs::read`, `std::time::SystemTime`, `str::replace`, `crate::fs::save_atomic`), so risk is low, but the gate is technically unmet. Verify on the next canonical Windows build (Team-Script `sync-and-build.ps1`). Source: code review of Story 1.4, Acceptance Auditor.

## Deferred from: code review of story 2-2-min-length-silence-pre-filter (2026-06-01)

- **No WAV magic / channel / bit-depth validation in `SilencePreFilter`.** `computeDurationMs` / `computeWavRms` read sampleRate@24 and dataSize@40 at fixed offsets and assume canonical mono-16-bit `encodeWav` output — no RIFF/WAVE/`data`-chunk-ID check, no `numChannels`/`bitsPerSample` read. Defensive hardening beyond this story's parity scope; the input is always Klarvo's own `encodeWav` output, so unreachable in production. Natural fit for a broader Android WAV-robustness pass. Source: code review of Story 2.2, Blind Hunter. [SilencePreFilter.kt]
- **`prev = currentState` captured asynchronously inside `handler.post` (potential TOCTOU).** Both new filter branches read `currentState` inside the posted lambda rather than at filter-decision time; between `check()` and lambda execution the state could change. This is the pre-existing pattern used by the existing `wavBytes.isEmpty()` guard and elsewhere — not introduced by this story. If addressed, fix the pattern centrally across all guards. Source: code review of Story 2.2, Blind + Edge + Auditor. [KlarvoOverlayService.kt:934-955]
- **gen-mirror `KlarvoOverlayService.kt` drift: 3 stray `[DIAG]` `Log.e` lines.** The gitignored build-target copy carries three pre-existing `android.util.Log.e(TAG, "[DIAG] ...")` lines absent from the canonical `android/kotlin-src/` source (around lines 920, 1079, 1081), unrelated to this story (the pre-STT filter block is byte-identical in both). The gen mirror should be re-synced clean from source on the next Android build. Source: code review of Story 2.2, Acceptance Auditor.

## Deferred from: code review of story 2-3-sanitize-paste-text-on-all-android-paths (2026-06-01)

Surfaced by the 3-layer review (Blind + Edge + Auditor, Opus 4.8). All four are real but out of this Android-parity story's scope; the one in-scope leak (`cleanupLocal` load-fail) was patched in-story.

- **Single-egress sanitize chokepoint vs. N per-branch call sites.** Desktop sanitizes once centrally (`pipeline.rs:1184`); Android wraps sanitization at each finalText-producing branch (now 4 sites after the load-fail fix). The Blind Hunter flagged that per-branch wrapping invites the next raw-return path to be forgotten — which is exactly the `cleanupLocal:552` leak this review caught. A single egress chokepoint in `KlarvoOverlayService` (sanitize `finalText` once after the if/else, drop the per-branch + per-cleanup internal sanitizes) would structurally prevent recurrence, but it's a larger refactor with double-sanitize (AC-2) risk and touches `cleanupLocal`/`cleanup`. Natural fit for an Android-depth or cross-platform-parity story. Source: code review of Story 2.3, Blind Hunter. [KlarvoOverlayService.kt:1108-1155]
- **Sanitizer char-set omits other invisible/control codepoints.** `sanitizeLlmOutput` strips ANSI-ESC, NUL, bidi (U+202A-202E/2066-2069/200E-200F), zero-width (U+200B-200D/FEFF) — exact parity with Rust `sanitize_llm_output`. It does NOT strip other C0/C1 controls, DEL (U+007F), line/paragraph separators (U+2028/2029), NEL (U+0085), Mongolian vowel separator (U+180E), or Hangul fillers (U+115F/U+3164), which can also be invisible/control on some paste targets. Expanding the set is a cross-platform decision — Rust must change in lockstep (ADR-0016 parity mandate). Backlog. Source: code review of Story 2.3, Blind + Edge Hunters. [KlarvoApi.kt:609-642 / pipeline.rs:2081-2128]
- **ANSI malformed-sequence handling.** The ESC-CSI loop breaks only on an ASCII-letter final byte, so a non-letter CSI final (e.g. `ESC[3~`) leaks its final byte, and a bare `ESC[` with no terminator before end-of-string silently consumes (discards) all trailing text. Pre-existing behavior, identical in Rust. Low impact (LLM/STT output rarely contains malformed CSI), out of scope. Backlog. Source: code review of Story 2.3, Blind + Edge Hunters. [KlarvoApi.kt:616-625]
- **Legitimate RTL text may be altered by bidi stripping.** Stripping bidi marks/isolates (U+200E/200F LRM/RLM, U+2066-2069 isolates) is correct anti-spoofing for LLM output but can corrupt genuine Arabic/Hebrew transcripts where those controls are legitimate layout. This is exact parity with desktop (Rust strips the same for all users), so it affects BOTH platforms equally and is not introduced by this story — a cross-platform product decision (does the anti-spoofing strip belong on the no-LLM-key raw-dictation path for RTL users?). Backlog. Source: code review of Story 2.3, Blind Hunter. [KlarvoApi.kt:631]

## Deferred from: code review of story 2-4-banking-app-blocklist (2026-06-01)

Surfaced by the 3-layer review (Blind + Edge + Auditor, Opus 4.8). Both real but out of this story's paste-path-guard scope; the one in-scope parity gap (`autoLoopActive` reset) was patched in-story.

- **Pending auto-send Enter not cancelled on banking block.** `handler.postDelayed({ performEnter() }, 150)` (auto-send) is queued after a successful paste; the banking guard blocks the *current* segment's paste but does not cancel an already-queued Enter from a prior AUTO segment. A prior segment's Enter keystroke can therefore land in whatever app is focused (potentially a banking app) within the ~150ms window. Narrow timing + pre-existing to the auto-send feature, orthogonal to the DIV-04 paste/clipboard guard. Defense-in-depth: re-check `bankingAppActive` inside the `performEnter` lambda, or cancel pending auto-send callbacks on block. Source: code review of Story 2.4, Edge Case Hunter. [KlarvoOverlayService.kt:1238-1240]
- **Banking-app detection latency / defaults-open.** The guard correctly acts on `bankingAppActive` at paste time, but the upstream accessibility-based detection that *sets* that field can lag the actual foreground switch (or never fire), in which case the guard reads `false` and paste proceeds. The guard cannot close this detection gap; it is upstream and pre-existing (the same field drives the existing bubble-visibility guard). A detection-correctness/timeliness hardening pass is the right home. Source: code review of Story 2.4, Blind + Edge Hunters. [KlarvoOverlayService.kt:389-407]

## Cross-platform parity net (Rust↔Kotlin drift detection) — Epic-2 retro AI-2, 2026-06-02

ADR-0016 *mandates* behavioral parity between the Rust core and the Android Kotlin port, but enforces it
only by convention (a developer re-implements Rust logic in Kotlin by hand). Nothing turns a test red when
one side changes and the other does not. **Epic 2 produced concrete drift instances, not hypotheticals:**

- `computeWavRms` divided by the header-*claimed* sample count where Rust divides by the count actually read
  (Story 2.2 review fix).
- `sampleCount==0` → `NaN` on Kotlin vs. `Some(0.0)` on Rust (Story 2.2 parity fix).
- `sanitizeLlmOutput` char-set held "exact parity with Rust" **by hand** — char-set expansion explicitly
  requires "Rust must change in lockstep" (Story 2.3 deferred items).
- Pre-Epic-2: the silence-field divergence (`759087f`) — Android read only `bubbleTapSilenceSecs`, ignoring
  `auto_mode_silence_secs` / `autostop_silence_secs` that desktop+UI use.

**The net (two parts):** (a) **golden vectors** — a language-neutral fixture file (input → expected output)
that BOTH the Rust tests and the JVM tests run, so any drift in a shared behavior (hallucination filter,
silence pre-filter, `sanitizeLlmOutput`, WAV-RMS, banking guard) turns a test red; (b) **config-key
contract** — the set of config keys both sides read must match (catches the `759087f` class).

**Decision (Andi, 2026-06-02 retro):** Deliberately **NOT** a 5th Epic-3 scope story — it is new capability,
not an audited TEST-0x finding, and is bigger than one story (a cross-language test harness is small infra).
Bolting it into Epic 3 would break that epic's finding-traceability. Instead:

1. **Down-payment in Story 3.3 (AI-1, separate from this deferral):** author the 3.3 WAV-RMS vectors as a
   shared language-neutral fixture + a thin Kotlin consumer test. Proves the pattern, closes the exact 2.2
   divisor drift, near-zero extra cost (3.3 writes those vectors anyway).
2. **Full net = post-Epic-3 story**, formalized via `bmad-create-story` when scoped. Likely an **ADR-0016
   amendment** that operationalizes the parity mandate the ADR already asserts. Severity: medium (latent —
   every drift instance so far was caught by review and fixed; no active user-facing leak). Source: Epic 2
   retrospective (`epic-2-retro-2026-06-02.md`) AI-2; supersedes the looser "1 story after Epic 2" framing in
   the project memory.

## Deferred from: code review of story-3.3 (2026-06-02)

All low-severity, gated on a clean machine-written fixture and/or production paths Android never reaches. None block Story 3.3.

- **Kotlin minimal JSON parser robustness** (`WavRmsVectorsTest.kt`): escape handling appends the raw next char (`\n`→`n`, no `\uXXXX`), the number tokenizer accepts `+`/`-` anywhere in the run, and a trailing backslash indexes past end-of-string (`StringIndexOutOfBounds`). Safe for the committed machine-generated fixture; would mis-parse a hand-edited malformed JSON. Add a 1-char EOF bounds guard + tighter number charset if the fixture ever becomes hand-edited.
- **`bits_per_sample==32` conflates float and 32-bit-int PCM** (both `make_float_wav`/`build_vector_wav` in pipeline.rs and `buildVectorWav` in Kotlin): a future `sample_format:"int"` + `bits:32` vector would be silently encoded as IEEE float. Symmetric on both sides, so cross-platform comparison wouldn't catch it. No such vector exists today.
- **`raw_bytes` range validation**: Rust uses `as_u64().unwrap() as u8` (panics on non-int, truncates ≥256), Kotlin `asInt().toByte()`. Constrain fixture bytes to 0..255 + assert range in both builders before any future raw-byte vector ≥128.
- **Fixture path resolution** (`WavRmsVectorsTest.kt` `firstOrNull { exists() }` over 4 relative guesses): depends on Gradle CWD = `:app` module dir; first existing match wins. Prefer a stable anchor (env var / classpath resource) if CI invocation changes.
- **`tested >= 7` is a self-referential count**, not an assertion that RMS-001..007 are each present — a deleted-and-replaced vector passes undetected. Assert the specific expected IDs.
- **Kotlin synthetic amplitude has no clamp** (`* 32767f`) vs Rust path: a future `amplitude > 1.0` vector wraps to a negative Short in Kotlin only. No such vector today.
- **`SilencePreFilter` audioFormat guard assumes a canonical 44-byte header** (fmt at offset 20) and checks only `audioFormat==1`, not bits==16 / channels==1: a WAV with a pre-fmt chunk (JUNK/LIST), 24-bit, or stereo PCM would misparse. Pre-existing limitation — Android `encodeWav` (KlarvoApi.kt:1039) always emits canonical 16-bit mono PCM, so unreachable in production. The guard is no worse than the surrounding parser.
- **`make_float_wav` doc comment says "audioFormat = 3"** — Edge Case Hunter claims hound writes WAVE_FORMAT_EXTENSIBLE (0xFFFE) for 32-bit float. **Unverified**; zero behavioral impact (hound reads its own output back; Rust float32 test green). Verify + correct the comment if touching this helper again.

## Deferred from: code review of story-3.2 (2026-06-02)

Low severity, does not block Story 3.2 — the gate logic and AC-4 inversion property are guarded and empirically verified.

> NOTE: a second item (`send_feedback` call-site binding not regression-protected) was originally deferred here, then **un-deferred at GATE 3 (Andi, 2026-06-02) and closed in-story** (Story 3.2 Tasks 4–5): extracted `send_feedback_inner` (the gate→build→POST seam) + 2 wiremock wire specs that pass `include_dictation` into the seam; empirically verified (flag flip → both wire specs RED). It is therefore no longer a deferral.

- **11-positional-arg payload builders have no compile-time slot safety** (`commands/feedback.rs`: `build_feedback_payload` ~158, and now also `send_feedback_inner` ~237): 6+ of the args are `String`/`Option<String>`; a caller transposition (e.g. `category`/`message`, or `version`/`os`/`platform`) compiles silently. `#[allow(clippy::too_many_arguments)]` suppresses the lint (sanctioned by the "clippy clean on touched files" DoD). Each has a single verified caller today, so per the premature-abstraction guard a shared param-struct/builder is not forced now. Revisit if a third caller appears or if Epic-4 touches this file. Severity: low.
