# Story 9-7-followup: Make Android silence-selection swap-safe (call-site wiring)

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer maintaining `KlarvoOverlayService.startRecording()`,
I want the four silence-duration values passed into `RecordingMode.selectSilenceSecs()` to be distinct,
non-interchangeable types instead of four same-typed `Float`s,
so that a call-site argument swap (e.g. passing `longPressSilenceSecs` where `tapSilence` belongs) fails
to **compile**, instead of silently regressing production silence-selection while every JVM test stays
green.

## Background — why this story exists

Source: Story 9-7 code-review (story-conductor, 2026-06-16), one **Medium** finding accepted as residual
by Andi at GATE 3, routed to `docs/backlog.md` §"Story 9-7 follow-up — make Android silence-selection
swap-safe (call-site wiring)".

Story 9-7 (done) extracted the `activeMode → activeSilenceSecs` selection out of
`KlarvoOverlayService.startRecording()` into a pure companion function,
`RecordingMode.selectSilenceSecs(mode, gesture, tapSilence, longPressSilence, autostopSilence,
autoModeSilence)` (`KlarvoOverlayService.kt:159-173`), and locked its mapping with a 12-test JVM suite
(`RecordingModeSilenceSelectionTest.kt`) using an independent expected-value table.

That test locks the **pure function's** mapping correctly. It does **not** lock the **call site** —
`startRecording()` (`KlarvoOverlayService.kt:1483-1490`) calls `selectSilenceSecs()` with four
same-typed `Float` arguments read from four same-typed `Float` instance fields
(`tapSilenceSecs`, `longPressSilenceSecs`, `autostopSilenceSecs`, `autoModeSilenceSecs`,
`KlarvoOverlayService.kt:257-261`). Kotlin named arguments are used at the call site today, but named
arguments only guard against *positional* mix-ups — they do nothing to stop a developer from writing
the *wrong variable name* against a correctly-spelled parameter name (e.g. `tapSilence =
longPressSilenceSecs`); the call still compiles because all four types are `Float`. Such a swap would
regress production silence-selection while the existing 72-test JVM suite (including
`RecordingModeSilenceSelectionTest`) stays fully GREEN, because that suite only exercises
`selectSilenceSecs()` directly with its own literals — it never touches the call site.

**Why this is value-invisible today:** the four production config defaults
(`bubbleTapSilenceSecs`/`bubbleLongPressSilenceSecs`/`autostopSilenceSecs`/`autoModeSilenceSecs`) all
default to `2.0f`. A swap would produce zero observable difference until a user configures the fields
to different values in Settings — at which point silence-detection timing would silently use the wrong
duration for their gesture/mode.

**This is NOT the original Android silence-field divergence bug** (9-7's AC6 already locked that — the
historical bug was reading the *wrong field entirely*, e.g. `bubbleTapSilenceSecs` for AUTO/AUTOSTOP
instead of the mode-level fields). This is a new, narrower, low-probability surface introduced by the
9-7 testability extraction itself: a same-typed-parameter call-site swap.

## Acceptance Criteria

**AC1 — Distinct, non-interchangeable types for the four silence durations.**
Given the four silence-duration values (tap, long-press, autostop, auto-mode)
When they are declared (as instance fields on `KlarvoOverlayService` and as parameters on
`RecordingMode.selectSilenceSecs()`)
Then each uses its own distinct Kotlin type (e.g. four `@JvmInline value class` wrappers around `Float`,
one per silence-duration role) so that the four are not structurally interchangeable at compile time.

**AC2 — A deliberately swapped call-site argument fails to compile.**
Given `RecordingMode.selectSilenceSecs()`'s four silence-duration parameters now have distinct types
When a developer edits the call site in `startRecording()` to pass one silence-duration value where a
different one belongs (e.g. `tapSilence = <the long-press value>` instead of `tapSilence = <the tap
value>`)
Then the file fails to **compile** with a type-mismatch error — this is the by-construction fix (backlog
Option (a), preferred over the heavier Option (b) real-`startRecording()`-wiring test).

**AC3 — Behavior is byte-identical; no functional regression.**
Given the same four underlying `Float` values flow through the new wrapper types exactly as they did as
plain `Float`s
When `startRecording()` runs for any mode/gesture combination
Then `RecordingMode.selectSilenceSecs()` returns the exact same `Float` value it returned before this
story for every input combination — this is a type-safety hardening, not a behavior change. The existing
9-7 mapping (`AUTO`→`autoModeSilenceSecs`, `AUTOSTOP`→`autostopSilenceSecs`, tap gesture
`HOLD`/`TOGGLE`→`tapSilenceSecs`, longpress gesture `HOLD`/`TOGGLE`→`longPressSilenceSecs`) must not
change.

**AC4 — `RecordingModeSilenceSelectionTest.kt` still passes, updated for the new types.**
Given the existing 12-test JVM suite in `RecordingModeSilenceSelectionTest.kt` asserts the mapping via
an independent expected-value table
When the parameter types change
Then the test file is updated to construct the new wrapper types (still with its own independently-owned
expected values — do NOT call the production path and compare it to itself) and all 12 assertions still
pass unmodified in behavior/intent.

**AC5 — `config.json` / `KlarvoApi.Config` field types are unchanged (scope boundary).**
Given `KlarvoApi.Config`'s silence fields (`bubbleTapSilenceSecs`, `bubbleLongPressSilenceSecs`,
`autostopSilenceSecs`, `autoModeSilenceSecs`) are parsed from JSON as plain `Float`
When this story is implemented
Then the JSON parsing / `Config` data class contract is **not** touched — the new value-class wrapping
is applied only inside `KlarvoOverlayService` (at `loadBubbleControls()`'s assignment into the four
instance fields, and at the `selectSilenceSecs()` signature), converting from the plain `Float` read out
of `Config` into the distinct wrapper type. `config.json`'s on-disk shape and the Rust↔Kotlin camelCase
mirroring (ADR-0016/NFR7) are unaffected.

**AC6 — Scope lock: no other call-site or behavior changes.**
Given this is a narrowly-scoped, by-construction type-safety fix
When implementing
Then do **not** touch: the `RecordingMode` enum's mode-dispatch logic, `handleTap()`, the
AUTOSTOP/AUTO `onSilenceDetected` wiring, `shouldInstallPreviewFlush()`, the Settings UI, or any
`config.json` key names. This story only changes the *type* of four already-correct values and the
function signature that consumes them.

**DoD:** `scripts/android-smoke.sh` exits 0 (Kotlin compiles clean, DEBUG APK builds, all JVM tests
green — expect the same 72 tests as after 9-7, none added/removed unless a type-conversion helper needs
its own trivial unit coverage). No on-device/emulator smoke is required beyond the standard build+JVM
gate: this is a compile-time-only type-safety change with a JVM-test-verifiable behavior-preservation
guarantee (AC3/AC4) — there is no runtime/visual surface for an emulator or real-device gate to observe
(mirrors the reasoning already accepted for 9-7's own byte-identical extraction, but this time also
run the JVM suite, since AC3/AC4 are exactly the kind of behavior-preservation claim a test can verify).

## Tasks / Subtasks

- [ ] **Task 1: Introduce four distinct value-class wrapper types** (AC: 1)
  - [ ] 1.1 In `KlarvoOverlayService.kt`, add four `@JvmInline value class` wrappers around `Float`, one
    per silence-duration role — e.g. `TapSilenceSecs(val value: Float)`, `LongPressSilenceSecs(val
    value: Float)`, `AutostopSilenceSecs(val value: Float)`, `AutoModeSilenceSecs(val value: Float)`.
    Place them near the `RecordingMode` enum (same file, same section) since they exist solely to
    support `selectSilenceSecs()`'s type-safety.
  - [ ] 1.2 Change the four instance fields (`KlarvoOverlayService.kt:257-261`:
    `tapSilenceSecs`, `longPressSilenceSecs`, `autostopSilenceSecs`, `autoModeSilenceSecs`) from `Float`
    to their corresponding new wrapper type.

- [ ] **Task 2: Update `selectSilenceSecs()` signature and call site** (AC: 1, 2, 3)
  - [ ] 2.1 Change `RecordingMode.selectSilenceSecs()`'s four value parameters
    (`tapSilence`, `longPressSilence`, `autostopSilence`, `autoModeSilence`,
    `KlarvoOverlayService.kt:159-173`) from `Float` to the matching wrapper type each. The function must
    still **return** `Float` (unwrap via `.value`) since `KlarvoAudioRecorder`'s `silenceSecs` parameter
    and all downstream consumers expect a plain `Float` — do not change that boundary.
  - [ ] 2.2 Update `loadBubbleControls()` (`KlarvoOverlayService.kt:715-733`) to wrap the plain `Float`
    values read from `config.bubbleTapSilenceSecs` etc. into the new wrapper types when assigning to the
    four instance fields.
  - [ ] 2.3 Update the call site in `startRecording()` (`KlarvoOverlayService.kt:1483-1490`) — the named
    arguments stay, but now pass the wrapper-typed instance fields directly (no explicit construction
    needed since the fields are already the wrapper type after 2.2).
  - [ ] 2.4 Verify: deliberately introduce a swapped argument locally (e.g. swap `tapSilence` and
    `longPressSilence` at the call site) and confirm the Kotlin compiler rejects it with a type mismatch.
    Revert the deliberate swap before proceeding — this is a manual verification step, not a committed
    change.

- [ ] **Task 3: Update the JVM test suite for the new types** (AC: 4)
  - [ ] 3.1 In `RecordingModeSilenceSelectionTest.kt`, update the `select()` helper and the four
    `TAP_SILENCE`/`LONG_PRESS_SILENCE`/`AUTOSTOP_SILENCE`/`AUTO_MODE_SILENCE` constants to construct the
    new wrapper types (e.g. `TapSilenceSecs(1.0f)`) instead of plain `Float` literals.
  - [ ] 3.2 Confirm all 12 existing assertions still pass with identical expected values — the test's
    independent expected-value table and regression-inversion tests (AUTO/AUTOSTOP must not return
    `tapSilenceSecs`) must be preserved verbatim in intent.

- [ ] **Task 4: Compile + verify** (AC: all)
  - [ ] 4.1 `scripts/android-smoke.sh` exits 0 (Kotlin compile clean, APK built, JVM tests green).
  - [ ] 4.2 Confirm JVM test count is unchanged (72 tests) unless Task 1's wrapper types need their own
    trivial construction test (optional, not required by any AC).

- [ ] **Task 5: Commit** (AC: all)
  - [ ] 5.1 Stage only touched files. Never `git add .`.
  - [ ] 5.2 Commit message: `fix(android): 9-7-followup — swap-safe silence-duration types at startRecording() call site`

## Dev Notes

### Context: this is a type-safety hardening, not a feature

This story has **no user-visible behavior change** and **no config/UI surface**. It closes a residual
code-review finding from Story 9-7 by making an already-correct call site fail to compile if it is ever
edited incorrectly, rather than adding a runtime guard or a heavier test-infra investment (backlog
Option (b), explicitly not preferred).

### Critical constraints: what MUST NOT change

- **The `selectSilenceSecs()` mapping logic itself is correct and must not change** — only its parameter
  *types*. AUTO→`autoModeSilenceSecs`, AUTOSTOP→`autostopSilenceSecs`, tap gesture
  (HOLD/TOGGLE)→`tapSilenceSecs`, longpress gesture (HOLD/TOGGLE)→`longPressSilenceSecs`.
- **`selectSilenceSecs()` must keep returning plain `Float`.** `KlarvoAudioRecorder`'s constructor
  (`KlarvoOverlayService.kt:1493-1504`, `silenceSecs = activeSilenceSecs`) and
  `KlarvoLogger.d` string interpolation at line 1491 both expect a `Float`, not a wrapper type. Only the
  four *input* parameters become distinct types; the function boundary back out to the rest of
  `startRecording()` stays `Float`.
- **`config.json` / `KlarvoApi.Config` are out of scope (AC5).** Do not change the JSON key names, the
  `Config` data class field types, or the Rust `AppConfig` struct. The wrapping happens entirely inside
  `KlarvoOverlayService`, at the boundary where `Config`'s plain `Float`s are read into the service's
  instance fields (`loadBubbleControls()`).
- **No other call site of `selectSilenceSecs()` exists** — it is called exactly once, in
  `startRecording()`. Do not add new call sites or generalize this pattern elsewhere.
- **Never `git add .`** — stage only the files actually touched.
- **No premature abstraction.** Per project convention ("factor out only on proven duplication"), the
  four value classes are justified here specifically because they are the *mechanism* of the compile-time
  safety this story requires (AC2) — not introduced speculatively.

### Key files (read before touching)

| File | Purpose |
|------|---------|
| `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt` | `RecordingMode.selectSilenceSecs()` companion function (lines 159-173); four silence instance fields (lines 257-261); `loadBubbleControls()` (lines 715-733); `startRecording()` call site (lines 1483-1490) |
| `android/kotlin-test/com/klarvo/voice/RecordingModeSilenceSelectionTest.kt` | 12-test JVM suite locking the mapping — must be updated to construct the new wrapper types |
| `android/kotlin-src/com/klarvo/voice/KlarvoApi.kt` | `Config` data class — confirm its silence fields stay plain `Float` (AC5, out of scope) |

### Current state (as of story creation — read the live file before editing, it may have moved)

`RecordingMode.selectSilenceSecs()` (companion function inside the `RecordingMode` enum):

```kotlin
fun selectSilenceSecs(
    mode: RecordingMode,
    gesture: String?,
    tapSilence: Float,
    longPressSilence: Float,
    autostopSilence: Float,
    autoModeSilence: Float,
): Float = when (mode) {
    AUTO     -> autoModeSilence
    AUTOSTOP -> autostopSilence
    else -> when (gesture) {
        "longpress" -> longPressSilence
        else        -> tapSilence
    }
}
```

The four instance fields it is fed from (`startRecording()`, via named arguments):

```kotlin
private var tapSilenceSecs = 2.0f
private var longPressSilenceSecs = 2.0f
private var autostopSilenceSecs = 2.0f
private var autoModeSilenceSecs = 2.0f
```

The call site (`startRecording()`):

```kotlin
val activeSilenceSecs = RecordingMode.selectSilenceSecs(
    mode              = activeMode,
    gesture           = activeGesture,
    tapSilence        = tapSilenceSecs,
    longPressSilence  = longPressSilenceSecs,
    autostopSilence   = autostopSilenceSecs,
    autoModeSilence   = autoModeSilenceSecs,
)
```

Note the named arguments already present — this story's fix is orthogonal to (and stronger than) named
arguments, since named arguments only prevent *positional* swaps, not *wrong-variable* swaps against a
correctly-named parameter.

### Kotlin `value class` notes

- `@JvmInline value class Foo(val value: Float)` erases to a plain `Float` at most call sites (no boxing
  overhead in the common case) while remaining a distinct type at compile time — exactly the by-
  construction guarantee AC2 needs. No prior usage of `value class` exists elsewhere in this codebase;
  this introduces the pattern for the first time, scoped narrowly to these four types.
- Value classes are supported since Kotlin 1.5; the project's Android Kotlin toolchain (Tauri-generated
  Gradle project, not checked into this repo — see `scripts/android-build.sh`) is well past that
  baseline, so no toolchain upgrade is implied.

### Testing standards summary

- JVM unit tests live under `android/kotlin-test/com/klarvo/voice/`, run via `scripts/android-smoke.sh`
  (which also builds the DEBUG APK). Current baseline after 9-7: 72 tests, 0 failures.
- Tests must own an **independent expected-value table** — never call the production path and assert it
  equals itself (established convention in `RecordingModeSilenceSelectionTest.kt`, and a named project
  rule: "Bind tests to the real code paths/files they cover, not to a parallel mock").
- No on-device/emulator smoke is required for this story (see DoD rationale above) — this deviates from
  the general "Android changes require an on-device/emulator smoke" rule only because there is no
  runtime/visual surface whatsoever (compile-time type change + JVM-verifiable behavior preservation).
  If in doubt during implementation, running `scripts/android-smoke.sh`'s build step is still mandatory
  (it also gates Kotlin compilation).

### Project Structure Notes

- No new files needed — the four value classes live in the same file as `RecordingMode`
  (`KlarvoOverlayService.kt`), consistent with how `shouldInstallPreviewFlush()` and
  `shouldApplyPreviewAppearance()` (other small pure/testable helpers) were added in-place rather than
  extracted to new files.
- No conflicts with the unified project structure — this touches only the two files already touched by
  Story 9-7.

### Previous story intelligence (Story 9-7)

- Story 9-7 (done, `_bmad-output/implementation-artifacts/9-7-short-press-gesture-modes-mirror-desktop.md`)
  was itself a verification-plus-extraction story: it found all four tap gesture modes already wired,
  and its only functional code change was extracting `selectSilenceSecs()` as a byte-identical pure
  function for testability (its own AC6). Its code-review flagged that the extraction closed the
  wrong-field-read risk (AC6) but left the call site's same-typed-parameter swap risk open — that
  residual finding is precisely this story's scope.
- Lesson from 9-7's own retro (change log): a byte-identical refactor with no behavioral surface does
  not need a real-device gate — reviewers confirmed no runtime/visual delta, and the JVM suite was the
  binding gate. This story is the same shape (compile-time type change, behavior-preservation verified by
  the existing/updated JVM suite), so the same reasoning applies to its DoD.
- 9-7 also carries a hard lesson about **not** closing GATE-4 on "the refactor is safe" reasoning when a
  *feature* claim is unverified (the Auto-mode real-device re-open). That lesson does not apply here
  because this story makes **no feature claim** at all — AC3's "byte-identical behavior" is fully
  verifiable by the JVM suite, unlike 9-7's Auto-mode DoD line which required live mic/VAD behavior no
  test exercised.

### References

- [Source: `docs/backlog.md` §"Story 9-7 follow-up — make Android silence-selection swap-safe (call-site wiring)"] — the originating backlog item, verbatim scope and AC.
- [Source: `_bmad-output/implementation-artifacts/9-7-short-press-gesture-modes-mirror-desktop.md`] — parent story; AC6; the code-review Medium finding and its acceptance as residual (change log, 2026-06-16 entries).
- [Source: `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:159-187`] — `RecordingMode.selectSilenceSecs()` and `shouldInstallPreviewFlush()` (sibling pure-function pattern to follow).
- [Source: `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:257-261`] — the four silence instance fields.
- [Source: `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:715-733`] — `loadBubbleControls()`.
- [Source: `android/kotlin-src/com/klarvo/voice/KlarvoOverlayService.kt:1475-1504`] — `startRecording()`'s mode selection + call site.
- [Source: `android/kotlin-test/com/klarvo/voice/RecordingModeSilenceSelectionTest.kt`] — the existing 12-test suite to update.
- [Source: `_bmad-output/planning-artifacts/epics-visual-overhaul.md` §"Story 9.7"] — parent epic story (AC3/AC6 origin: mode-centric shared silence fields, "avoid the Android silence-field divergence").
- [Source: `docs/adr/0016-android-path-parity-strategy.md`] — Rust↔Kotlin config mirroring convention (AC5 scope boundary).
- [Source: `_bmad-output/project-context.md`] — Android smoke DoD conventions, "factor out only on proven duplication", English code/comments rule, never `git add .`.

## Dev Agent Record

### Agent Model Used

### Debug Log References

### Completion Notes List

### File List
