# Story 13-2 — gate verdict

Story: *Parity sweep: guards and silent loss* (Epic 13, ADR-0016 Amendment 4 rows
B1-Android, B2, B3, B4, B6, D2, D3, D4, D5, D6, D9, D10, D11, E1, E2).

There is **no** desktop proxy harness or device run in this story's evidence. The
surfaces it touches are the pill's terminal state, an Android toast set and a bubble
state — none of them a new drawing, and the two gates below decide logic, wiring and
structure on Linux only.

## Gates run

| Gate | Command | Result |
|---|---|---|
| baseline | `cd src-tauri && cargo test --lib` at `4e4bc00` | **749 passed, 0 failed** — the number the spec predicted |
| Rust | `cd src-tauri && cargo test --lib` | **768 passed, 0 failed** (+19) |
| JVM (device-free) | `./gradlew :app:testUniversalDebugUnitTest --rerun-tasks` in `src-tauri/gen/android`, after syncing `android/kotlin-src` + `android/kotlin-test` | **29 suites / 260 tests, 0 failures, 0 errors** (baseline 25 / 217) |
| inversions | 10 Rust + 18 Kotlin + 8 matrix-audit follow-up | **36/36 RED**, see `code-inversion-report.md` |
| `npm run build` | not run | no `src/` file is touched by this story |

`git status` after the last revert carries only this story's intended changes; no
inversion edit survived.

## What the green runs decided

- **B2 / D-H5, D-H6, D-M9** — one guard chain, `pipeline::guard_transcript`, called by
  the desktop pipeline and by the Android JNI through
  `stt::groq_jni::guard_transcript_for_jni`. Fed the conditioning hint alone, a
  dictionary-word utterance survives and a ≥10-byte dictionary term is not deleted; fed
  the pre-13-2 input (the full built prompt) both still fail, which is asserted so
  "fixed" cannot mean "the guard stopped working". Fragment strip runs before the
  verdict.
- **B3 / D-H7** — the pre-guard ghost strip (Desktop stops dropping a ghosted transcript
  whole) and the post-cleanup ghost strip on Android, the same Rust function over a new
  `nativeStripStockphraseGhosts` extern. No Kotlin twin: asserted by a blocklist-literal
  tripwire over every production `.kt`.
- **B4 / D-H4** — `advanced.sttPrompt{De,En,Auto}` is parsed on Android and selected by
  the twin of `stt::select_stt_hint_override`; every `transcribeWithRetry` call site
  passes it; `config.customPrompt` appears on no line that is not the LLM's
  `customInstructions`. The prompt builder's separator is explicit.
- **D2 / D3 / D10** — `mapCleanupResponse` is the twin of `llm::parse_chat_completion`:
  truncation and emptiness raise named non-retryable exceptions, and an undecodable body
  is retryable on the single-call path as well.
- **D9 / D-M5, D-M6** — the `__ERROR_FORMAT:` sentinel is non-retryable and the retry
  budget is one attempt.
- **D4 / D5 / D6** — `pasteIntoFocusedField` reports, `decideDelivery` takes the outcome
  and the clipboard result, `terminalStateFor` gives a run that did not deliver the
  shipped IDLE, and the desktop `terminal_degrade_cause` names
  `ClipboardWriteFailed` on a clean run whose clipboard write failed — while a focus-only
  failure keeps today's wording.
- **D11** — four of five toasts are gone; `"No audio recorded"` stays.
- **E1 / E2** — one offline predicate, read by the hotkey pipeline, by
  `commands::recording::cleanup_text` and by the Android overlay, pinned row for row by
  `offline-rule-vectors.json` on both sides. The Android preview flush refuses to install
  and re-checks at flush time.
- **B6 / D-L19, D-L21** — the desktop VAD calls Silero on every frame (verdict provably
  unchanged: `advance_state` ANDs `energy_ok` into both thresholds, asserted); the
  Android auto-stop hangover fires on the (N+1)-th frame.

## Matrix Test Audit follow-up (2026-09-21)

An audit of the spec's I/O & Edge-Case Matrix found four rows whose only evidence was
"read the diff": **D11**, **B1-Android**, **D6-Android** and the flush-time half of
**E1**. `OverlayServiceSourceContractTest` (7 tests) closes them, and each assertion is
inverted (A1-A8). D6's catch semantics now run through a real seam,
`KlarvoOverlayService.guardedClipboardWrite`, which takes the write and the failure
handler as parameters for the same reason `KlarvoAudioRecorder.vadGateDecision` takes
`isSpeech` — the real bodies need a `Context` and a `ClipboardManager`. The other three
rows are order-anchored source tripwires, the instrument `Adr0017BoundaryGuardTest`
established; they prove the code says the right thing, never that the device does it.

## What these numbers do NOT cover

Named, not implied:

- **The real Whisper conditioning result for real audio (B4).** That two different
  strings reach the model is wiring; that the transcript's punctuation and casing change is a
  model behaviour only Andi's device can show.
- **The banking guard against a real foreground app (B1-Android).** The statement ORDER
  is now asserted (`bankingVerdictPrecedesTheHistoryAndTursoWrites`); what no test can
  see is a real blocklisted app in the foreground. Andi's check stands: dictate into one,
  then look at History.
- **The real clipboard and accessibility behaviour on a device (D4, D6-Android).** The
  branch logic and the catch semantics are decided; a real `setPrimaryClip` throw and a
  real refused `ACTION_PASTE` are not producible off-device.
- **The real-audio effect of the VAD change (D-L19, D-L21).** D-L19 is **Weg 2,
  agent-verified only** — B6 carries no H+ (`ADR-0016:289`) and the per-frame verdict is
  provably unchanged, so there is nothing for Andi to observe. D-L21 shifts auto-stop by
  one 32 ms frame, which he will not perceive; the honest device check is "auto-stop
  still works at all".
- **D6 on either platform** — agent-verified only; ADR-0016 Amendment 4 already records
  that downgrade.
- **The bubble's appearance.** No new state, no new drawing, no new wording — but that
  the shipped IDLE and the shipped `"Copied: …"` toast *look* right in the new positions
  is Andi's eye.
- **Anything on Windows.** No Windows build was made in this run.
- **`test_silence_stays_silence`** moved from a deterministic `prob = 0.0` path to real
  ONNX output on digital-silence frames. It is **green** (measured, not assumed), but it
  is now model-dependent.

## H+ reproduction, as the spec named it before the build

Unchanged from the spec's Verification section. The test provider drives D2, D3, D9 and
D10 from Settings → Advanced → Expert mode → System, no computer attached; B2 is the
`Klarvo, Kubernetes` dictionary on the Xiaomi; B3 a known ghost phrase on both devices;
B4 the "Technical" preset with identical audio; D4/D5/D11 the Xiaomi; E1/E2 Windows;
B1-Android a dictation into a blocklisted app followed by a look at History.

**Android install: `scripts/android-install-debug.sh <ip:port> --full` is mandatory.**
This story adds a native symbol (`nativeStripStockphraseGhosts`); a stale
`libklarvo_lib.so` has no such export and the call throws `UnsatisfiedLinkError`. The
`nativeTranscribe` signature is deliberately unchanged — the guard hint is rebuilt inside
the JNI from arguments that already cross — so the silent-misbind class does not apply
here, but the `.so` must still be rebuilt.
