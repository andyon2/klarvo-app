# Story 9.10: Token Codegen — `klarvo.css` → `KlarvoTheme.kt` (post-ADR-0019; before the 9.5 rebuild)

Status: done

## Story

As a developer maintaining two platform implementations of one design,
I want the Android token file (`KlarvoTheme.kt`) generated from the canon CSS rather than hand-typed,
so that the token layer cannot structurally drift (closing the F6 class of copy-errors) and the 9.5
rebuild renders against the real single-source-of-truth.

## Context (why this story exists)

ADR-0019 (Accepted, `docs/adr/0019-cross-platform-design-ssot.md`) made the canon
(`docs/design/overhaul/source/`) the single source of truth for design tokens cross-platform. It found
**two drift surfaces**; this story closes the first one structurally:

> **Token-Drift.** `KlarvoTheme.kt` is a hand-typed copy of `klarvo.css`'s `--k-*` values. Proof: the
> Story-9.5-review fix **F6** corrected `AmberLine` from `0x4D…` (.30 α) to `0x52…` (.32 α) — a pure
> copy-error against a canon that already held `rgba(233,162,76,0.32)` correctly. Hand-copied constants
> drift inevitably.

ADR-0019 **Decision #2:** *"Visual tokens are generated, not transcribed … no hand-typed hex in
platform code. The token layer then structurally cannot drift (closes the F6 class)."* ADR-0019
§Mitigations orders this **first** (mechanical, highest leverage, cheap), and it is the **foundation
for the 9.5 rebuild** — the recording state must render against real SSOT tokens, not a drifting copy.

This story is **colors only** — the proven drift class and exactly what `KlarvoTheme.kt` holds today.
Radii / shadows / easing / durations / font families are explicitly out of scope (see "What This Story
Does NOT Do").

## Acceptance Criteria

**AC1 — A generator projects the canon `--k-*` color tokens into `KlarvoTheme.kt`:**
Given the canon `docs/design/overhaul/source/assets/klarvo.css` holds the `--k-*` custom properties (the
SSOT token set per ADR-0019)
When the generator (`scripts/gen-android-theme.mjs`) runs
Then it emits `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` with **every canon color token** as a
Kotlin `const val Int`, using:
  - hex `#RRGGBB` → `0xFFRRGGBB.toInt()` (opaque)
  - `rgba(r,g,b,a)` → `0xAARRGGBB.toInt()` where `AA = round(a × 255)` (e.g. `0.12 → 0x1F`, `0.32 → 0x52`, `0.055 → 0x0E`)
And **no canon-derived hex value is hand-typed** anywhere in platform Kotlin code (the generated file is
the only home for canon color constants).

**AC2 — Zero visual regression: every currently-referenced identifier resolves byte-identically:**
Given the live consumers reference `KlarvoTheme.*` identifiers (`FloatingBubbleView.kt`,
`ListeningPanelView.kt` — `Bg`, `Surface`, `Surface2`, `Elevated`, `Border`, `Border2`, `TextC`,
`Muted`, `Dim`, `Teal`, `TealHi`, `TealLo`, `OnTeal`, `Amber`, `AmberHi`, `Danger`, `TealBg`, `AmberBg`,
`AmberLine`, `DangerBg`)
When `KlarvoTheme.kt` is regenerated
Then **every** currently-referenced identifier still resolves, with a **byte-identical color value** to
today's hand-written file (zero pixel change)
And the only non-mechanical name is preserved via an explicit alias: `--k-text` → `TextC` (mechanical
PascalCase would give `Text`; the alias keeps the existing identifier so consumers don't break)

**AC3 — The F6 alpha class is produced correctly by the rule, not by hand:**
Given the rgba→ARGB conversion in AC1
When the file is generated
Then `AmberLine == 0x52E9A24C` (the F6-corrected `.32` value, from `--k-amber-line: rgba(233,162,76,0.32)`),
`TealBg == 0x1F29C7AC`, `AmberBg == 0x1FE9A24C`, `DangerBg == 0x1FEE6F63`
And these are derived by the converter — not transcribed — so the F6 copy-error class is structurally
impossible going forward.

**AC4 — A build/CI drift gate fails on any hand-edit of a generated value:**
Given the generated `KlarvoTheme.kt` is committed (the tracked source per the `kotlin-src` pattern)
When the build/smoke flow runs
Then a **drift gate** regenerates the canon-color region to a temp buffer and diffs it against the
committed file's generated region; **any** mismatch (a hand-edited token value, a stale value after a
canon change) **fails the build** with a clear actionable message
  (e.g. `KlarvoTheme.kt drifted from canon klarvo.css — run: node scripts/gen-android-theme.mjs`)
And the gate runs in `scripts/android-smoke.sh` **and** `scripts/android-build.sh`, **before** the
`kotlin-src → gen/android` sync step (so a drifted file never reaches a build).

**AC5 — Canon color tokens missing from today's file are added (complete projection):**
Given the canon holds color tokens not present in the current hand-written file
When the file is generated
Then these are added: `BgDeep` (`--k-bg-deep #0A0B0C`), `Hairline` (`--k-hairline rgba(255,255,255,0.055)`
→ `0x0EFFFFFF`), `Faint` (`--k-faint #4B4F53`), `TealLine` (`--k-teal-line rgba(41,199,172,0.32)` →
`0x5229C7AC`), `Success` (`--k-success #4FC58A`), `Info` (`--k-info #57DDC7`)
And the file is a **complete projection** of the canon color set (no canon color token silently omitted)

**AC6 — Platform-derived non-canon constants are preserved and clearly fenced:**
Given `ShadowColor (0x33000000)` is **not** a canon `--k-*` token (it is a deliberate Android 20%-black
drop-shadow, distinct from the canon's `--k-e* box-shadow` strings which don't map to a single Android color)
When the file is generated
Then `ShadowColor` (and any other genuinely platform-derived constant) is kept in a **clearly-marked,
non-generated region** of the file that the drift gate does NOT overwrite or diff
And the file header documents which region is generated (canon projection) vs. hand-maintained (platform-
derived), and names the canon source + ADR-0019 as provenance.

**AC7 — Build still green, no behavior change:**
Given the generated file replaces the hand-written one
When `scripts/android-smoke.sh` runs
Then the 60 JVM unit tests still pass (no regression), the Kotlin compile is clean, and the DEBUG APK builds
And because all values are byte-identical to today, there is **no pixel change** on-device.

**Inversion (must-fail gate):**
- A submission with any **hand-typed canon hex** for a `--k-*` token outside the generator output must not pass.
- A submission where editing a generated token value in `KlarvoTheme.kt` and rebuilding does **not** fail
  the drift gate must not pass (the gate is vacuous).
- A submission that breaks an existing consumer identifier (renames `TextC`, drops `Border2`, etc.) or
  changes any currently-consumed value must not pass.
- A submission where the drift gate runs **after** the `kotlin-src` sync (so a drifted file still reaches
  a build) must not pass.

**DoD:** Generator + drift gate wired into `scripts/android-smoke.sh` and `scripts/android-build.sh`
before the sync step; 60 JVM unit tests pass; DEBUG APK builds; the generated `KlarvoTheme.kt` color
values are byte-identical to today's (machine-verified diff). **Verifiability symmetry:** this story
changes **no pixels** (values identical) → the human visual gate is *consciously downgraded* (per the
global Verifikations-Symmetrie rule, path 2) to an **optional** sanity glance that the idle bubble +
panel render unchanged; the binding gate is the byte-identity assertion + the drift check, both
machine-verifiable. Andi is **not** asked to verify a state he must hand-produce.

## Tasks / Subtasks

- [x] **Task 1: Build the generator `scripts/gen-android-theme.mjs`** (AC: 1, 2, 3, 5)
  - [x] 1.1 Read `docs/design/overhaul/source/assets/klarvo.css`. Parse the `--k-*` custom-property
    declarations in the `:root` block. Extract only the **color** tokens (hex `#…` or `rgba(…)` values);
    skip radii (`--k-r-*`), shadows (`--k-e*`, `--k-glass`, `--k-focus`), fonts (`--k-font`, `--k-mono`),
    easing (`--k-ease`, `--k-spring`), durations (`--k-t-*`).
  - [x] 1.2 Name mapping: `--k-<a>-<b>-…` → PascalCase join of the hyphen-separated segments, digits kept
    (`surface-2` → `Surface2`, `border-2` → `Border2`, `teal-hi` → `TealHi`, `bg-deep` → `BgDeep`,
    `on-teal` → `OnTeal`, `amber-line` → `AmberLine`, `teal-line` → `TealLine`). **Alias overrides** (one
    entry): `text` → `TextC`. Keep the alias map as an explicit, commented constant at the top of the
    generator — it is the documented exception list.
  - [x] 1.3 Value conversion:
    - `#RRGGBB` → `0xFF` + uppercase `RRGGBB` + `.toInt()`
    - `rgba(r,g,b,a)` → `0x` + 2-hex `round(a*255)` (uppercase, zero-padded) + 2-hex each of r,g,b (uppercase) + `.toInt()`
    - Assert in code: `AmberLine === 0x52E9A24C`, `TealBg === 0x1F29C7AC`, `AmberBg === 0x1FE9A24C`,
      `DangerBg === 0x1FEE6F63` (fail the generator loudly if the converter regresses).
  - [x] 1.4 Emit a deterministic, stable Kotlin file (fixed token ordering so diffs are minimal): a header
    block (provenance: "GENERATED from docs/design/overhaul/source/assets/klarvo.css by
    scripts/gen-android-theme.mjs — do not edit by hand; see ADR-0019"), the color-semantics doc comment
    (carry over the existing teal/amber/danger semantics comment + the Android glass-ring convention note),
    a clearly-fenced **GENERATED region** (`// ===== BEGIN GENERATED (canon colors) =====` …
    `// ===== END GENERATED =====`) with the canon-projected constants, and a separate **hand-maintained
    region** holding `ShadowColor` and the doc preamble (AC6).
  - [x] 1.5 Support two modes: default = **write** the file; `--check` = regenerate to a buffer and exit
    non-zero (with a diff/message) if the committed file's GENERATED region differs (this is what the
    drift gate calls).

- [x] **Task 2: Regenerate `KlarvoTheme.kt` and confirm byte-identity for consumed tokens** (AC: 2, 3, 5, 6)
  - [x] 2.1 Run the generator to write `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt`.
  - [x] 2.2 Diff against the previous version: **every** currently-consumed identifier (see AC2 list) must
    have an identical value; the only additions are the new tokens from AC5; `ShadowColor` unchanged.
    `git diff` should show only additions (new tokens) + the generated-header/fences, **no value changes**
    to existing tokens.
  - [x] 2.3 Grep the platform Kotlin tree for stray hand-typed canon hex outside the generated region:
    `grep -rnE "0x[0-9A-Fa-f]{8}\.toInt\(\)" android/kotlin-src/com/klarvo/voice/` — every hit must be
    inside `KlarvoTheme.kt`'s generated region (or a documented platform-derived constant). No canon color
    re-typed in `FloatingBubbleView.kt` / `ListeningPanelView.kt` / service.

- [x] **Task 3: Wire the drift gate into the build scripts** (AC: 4)
  - [x] 3.1 In `scripts/android-smoke.sh`, add a step **before** "2. Kotlin-Quellen synchronisieren"
    (before the `cp "$SRC"/*.kt "$DST/"`): run `node scripts/gen-android-theme.mjs --check`; on non-zero,
    `fail "KlarvoTheme.kt ist von der Canon-CSS abgedriftet — node scripts/gen-android-theme.mjs ausführen"`.
  - [x] 3.2 In `scripts/android-build.sh`, add the same `--check` gate before its `[sync] Copying Kotlin
    files` step (line ~60). Match the script's existing echo/fail style.
  - [x] 3.3 Confirm the gate is RED when it should be: temporarily edit one token value in `KlarvoTheme.kt`,
    run the `--check`, confirm it exits non-zero with the actionable message; revert. (Inversion proof at
    writing-time — do not ship a vacuous gate.)

- [x] **Task 4: Compile + verify (no regression)** (AC: 7)
  - [x] 4.1 Run `scripts/android-smoke.sh` — drift gate passes (file in sync), 60/60 JVM tests pass, Kotlin
    compile clean, DEBUG APK builds. (adb install may hit the known Xiaomi USER_RESTRICTED constraint —
    that is Andi's optional on-device glance, not a functional blocker, since values are byte-identical.)
  - [x] 4.2 Confirm the byte-identity claim mechanically: the diff from Task 2.2 shows zero value changes
    to consumed tokens. Record the diff summary in the Dev Agent Record.

- [x] **Task 5: Commit** (AC: all)
  - [x] 5.1 Stage only: `scripts/gen-android-theme.mjs` (new), `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt`,
    `scripts/android-smoke.sh`, `scripts/android-build.sh`. Never `git add .`.
  - [x] 5.2 Commit message: `build(android): 9-10 token codegen — generate KlarvoTheme.kt from canon klarvo.css + drift gate`

### Review Findings (code review 2026-06-16 — Opus, 3-layer)

Acceptance Auditor: all ACs satisfied (byte-identity confirmed, gate non-vacuous on a token edit, 26 canon color tokens → 26 constants, ShadowColor fenced out). Blind + Edge Hunter raised gate-robustness items; triaged:

- [x] [Review][Patch] Drift gate is vacuous-green when GENERATED fences are missing/renamed (`'' === ''` → exit 0 on a corrupted file) [scripts/gen-android-theme.mjs:~239-243] — fail loudly if either the committed or freshly-generated GENERATED region is empty/absent.
- [x] [Review][Patch] No completeness guard — a silently dropped consumed identifier (`:root` truncation / parse change) is caught only by kotlinc, not the gate [scripts/gen-android-theme.mjs:~139-157] — assert the AC2 consumed-identifier set is all emitted; throw on any missing.
- [x] [Review][Defer] Parser throws on modern CSS color syntax (`rgb()`, space-separated, %, 3-/8-digit hex) [scripts/gen-android-theme.mjs:hexToArgb/rgbaToArgb] — deferred; fail-loud, not present in current canon.
- [x] [Review][Defer] rgba alpha >1 → malformed literal [scripts/gen-android-theme.mjs:rgbaToArgb] — deferred; all canon alphas ≤1.
- [x] [Review][Defer] `node`-absence on a build host gives an opaque failure before the sync [scripts/android-*.sh] — deferred; node is available in the build env (render-surface.mjs).
- [x] [Review][Defer] `android-smoke.sh` mis-reports the JVM test count ("24" vs 60) [scripts/android-smoke.sh] — deferred; pre-existing harness quirk, unrelated to 9-10.

Dismissed as noise (4): `Info==TealHi` (canon values genuinely equal); PascalCase name-collision (none today); box-drawing message alignment (cosmetic); echo-vs-step style (each gate matches its own host script).

## Dev Notes

### What This Story Does (Exactly)

Replaces the **hand-typed** `KlarvoTheme.kt` with a **generated** one, projected from the canon
`klarvo.css` `--k-*` color tokens, and adds a **drift gate** to the build so a hand-edited value can no
longer merge. Colors only — the proven drift class. The generated values are **byte-identical** to
today's for every consumed token, so there is **no visual change**; the win is purely structural (F6
class becomes impossible).

### Token map (canon `--k-*` → `KlarvoTheme.*`) — the contract

All currently-consumed tokens are byte-identical; the conversion rule reproduces them, it does not change
them. New tokens (absent today) are marked **NEW**.

| Canon `--k-*` | CSS value | `KlarvoTheme.*` | Generated `Int` |
|---|---|---|---|
| `--k-bg-deep` | `#0A0B0C` | `BgDeep` | `0xFF0A0B0C` **NEW** |
| `--k-bg` | `#0F1112` | `Bg` | `0xFF0F1112` |
| `--k-surface` | `#16181A` | `Surface` | `0xFF16181A` |
| `--k-surface-2` | `#1B1E20` | `Surface2` | `0xFF1B1E20` |
| `--k-elevated` | `#232729` | `Elevated` | `0xFF232729` |
| `--k-border` | `#282C2F` | `Border` | `0xFF282C2F` |
| `--k-border-2` | `#353A3E` | `Border2` | `0xFF353A3E` |
| `--k-hairline` | `rgba(255,255,255,0.055)` | `Hairline` | `0x0EFFFFFF` **NEW** |
| `--k-text` | `#ECEEEF` | `TextC` *(alias)* | `0xFFECEEEF` |
| `--k-muted` | `#A4A9AC` | `Muted` | `0xFFA4A9AC` |
| `--k-dim` | `#6F7479` | `Dim` | `0xFF6F7479` |
| `--k-faint` | `#4B4F53` | `Faint` | `0xFF4B4F53` **NEW** |
| `--k-teal` | `#29C7AC` | `Teal` | `0xFF29C7AC` |
| `--k-teal-hi` | `#57DDC7` | `TealHi` | `0xFF57DDC7` |
| `--k-teal-lo` | `#1B9C88` | `TealLo` | `0xFF1B9C88` |
| `--k-teal-bg` | `rgba(41,199,172,0.12)` | `TealBg` | `0x1F29C7AC` |
| `--k-teal-line` | `rgba(41,199,172,0.32)` | `TealLine` | `0x5229C7AC` **NEW** |
| `--k-on-teal` | `#05201B` | `OnTeal` | `0xFF05201B` |
| `--k-amber` | `#E9A24C` | `Amber` | `0xFFE9A24C` |
| `--k-amber-hi` | `#F4BA72` | `AmberHi` | `0xFFF4BA72` |
| `--k-amber-bg` | `rgba(233,162,76,0.12)` | `AmberBg` | `0x1FE9A24C` |
| `--k-amber-line` | `rgba(233,162,76,0.32)` | `AmberLine` | `0x52E9A24C` *(F6)* |
| `--k-danger` | `#EE6F63` | `Danger` | `0xFFEE6F63` |
| `--k-danger-bg` | `rgba(238,111,99,0.12)` | `DangerBg` | `0x1FEE6F63` |
| `--k-success` | `#4FC58A` | `Success` | `0xFF4FC58A` **NEW** |
| `--k-info` | `#57DDC7` | `Info` | `0xFF57DDC7` **NEW** |
| *(none — platform-derived)* | — | `ShadowColor` | `0x33000000` *(hand-maintained; AC6)* |

Alpha sanity: `round(0.055*255)=14=0x0E`, `round(0.12*255)=31=0x1F`, `round(0.32*255)=82=0x52`.

### Why colors only (scope fence)

`KlarvoTheme.kt` is color-only today, and the F6 drift the ADR cites was a color. Radii (`--k-r-*`),
shadows (`--k-e*` box-shadow strings → Android `Modifier.shadow`/elevation), easing (`cubic-bezier` →
`Interpolator`), durations (`--k-t-*` ms → `Long`), and fonts (`--k-font`/`--k-mono` → `res/font` refs)
do **not** map 1:1 to a single Android color constant and need platform-specific hand-mapping — they are
not in this file today and are **out of scope**. A future story could extend the generator to emit a
`KlarvoMotion.kt`/`KlarvoDims.kt` if drift appears there; do not pre-build it (no proven second drift yet).

### Build flow — where the gate slots (critical)

`scripts/android-smoke.sh` step 2 (`# 2. Kotlin-Quellen synchronisieren`, ~line 143) does
`cp "$SRC"/*.kt "$DST/"` from `android/kotlin-src/com/klarvo/voice` → `gen/android/.../java/com/klarvo/voice`.
`scripts/android-build.sh` has the equivalent `[sync] Copying Kotlin files` at ~line 60. The drift gate
must run **before** these copies so a drifted file never reaches `gen/android`. The generator writes the
**tracked** source (`android/kotlin-src/...`), consistent with the "kotlin-src is the tracked tree, build
copies it" pattern (project-context.md, Build Architecture).

Node is available in the WSL build environment (the design pipeline already runs `render-surface.mjs`).
Use Node (`.mjs`) for the generator to match that tooling; no new toolchain dependency.

### Provenance (ADR-0019 #5)

The canon is provenance-tracked via `docs/design/overhaul/source/MANIFEST.md` (`sourceFingerprint`). This
story consumes the canon CSS as input; no canon edit. The generated file header must name the canon path
+ ADR-0019 so a future reader knows it is a projection, not a source. (No MANIFEST change needed — we read
the canon, we don't extend it.)

### What This Story Does NOT Do

- Does NOT generate radii / shadows / easing / durations / font families (color-only; see scope fence).
- Does NOT touch the canon CSS, the MANIFEST, or any `docs/design/overhaul/source/` file (read-only input).
- Does NOT change any consumed token value → **no pixel change** on-device.
- Does NOT rebuild Story 9.5 (that is the next story, against the extended canon).
- Does NOT do the Desktop-parity check (`src/styles.css` semantics) — separate later story.
- Does NOT touch Rust/Tauri/React sources, `FloatingBubbleView.kt`, `ListeningPanelView.kt`, or the service
  (their `KlarvoTheme.*` references are unchanged).
- Does NOT add CI infra beyond the two existing build scripts (the gate lives in the scripts that already gate builds).

### Files

| File | Change |
|------|--------|
| `scripts/gen-android-theme.mjs` | **NEW** — parse canon `--k-*` colors → emit `KlarvoTheme.kt`; `--check` drift mode |
| `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` | regenerated: GENERATED region (canon colors) + hand-maintained `ShadowColor` + header |
| `scripts/android-smoke.sh` | add `--check` drift gate before the `kotlin-src` sync (step 2) |
| `scripts/android-build.sh` | add `--check` drift gate before the `[sync] Copying Kotlin files` step |

No other files.

### References

- [Source: docs/adr/0019-cross-platform-design-ssot.md, Decision #2 + §Mitigations] — generate-not-transcribe; F6 evidence; ordering (codegen first)
- [Source: docs/design/overhaul/source/assets/klarvo.css, `:root` `--k-*`] — the SSOT color token set (generator input)
- [Source: docs/design/overhaul/source/MANIFEST.md] — canon provenance (`sourceFingerprint`); read-only here
- [Source: android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt] — current hand-written file (baseline; F6-corrected AmberLine `0x52`)
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, ListeningPanelView.kt] — the `KlarvoTheme.*` consumers (identifiers that must keep resolving)
- [Source: scripts/android-smoke.sh, ~line 143 "2. Kotlin-Quellen synchronisieren"] — sync step; gate goes before it
- [Source: scripts/android-build.sh, ~line 60 "[sync] Copying Kotlin files"] — sync step; gate goes before it
- [Source: _bmad-output/project-context.md] — kotlin-src is the tracked tree; never `git add .`; Android changes require on-device smoke (here downgraded — no pixel change); code/comments English
- [Source: Memory project_cross_platform_design_ssot] — ADR-0019 summary + follow-up ordering (token codegen = step 1)

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6

### Debug Log References

- Task 2.3 finding: `ListeningPanelView.kt` line 501 contains `0x4DEE6F63.toInt()` (ad-hoc ~30% alpha danger stroke). This is NOT a canon `--k-*` token (no `--k-danger-stroke` exists in klarvo.css). It is a platform-derived ad-hoc value, comparable to `ShadowColor`. Out of scope for this story (story explicitly excludes touching ListeningPanelView.kt). Documented for future cleanup.
- Inversion proof (Task 3.3): Tampered `Teal` from `0xFF29C7AC` → `0xFF29C7AD`; `--check` exited 1 with full drift message and diff hint. Reverted; `--check` exited 0. Gate is non-vacuous.
- android-smoke.sh reported "24 Tests" (reads only one XML file via `-print -quit`); full count verified: `testUniversalDebugUnitTest` = 60/60 green.

### Completion Notes List

- Generator `scripts/gen-android-theme.mjs` built: parses canon `:root` `--k-*` color tokens (hex + rgba), converts to ARGB Kotlin `Int` literals, emits deterministic KlarvoTheme.kt with GENERATED fences + hand-maintained region.
- `KlarvoTheme.kt` regenerated: 26 color tokens (was 21). All 21 pre-existing consumer-referenced identifiers byte-identical. 6 new tokens added: BgDeep, Hairline, Faint, TealLine, Success, Info (AC5).
- `ShadowColor` preserved in hand-maintained region (AC6).
- Drift gate wired into both `android-smoke.sh` (before sync step) and `android-build.sh` (before `[sync] Copying Kotlin files`).
- Inversion-proof done at writing time (AC4 non-vacuous gate confirmed).
- `android-smoke.sh` full run: drift gate GREEN, 60/60 JVM tests green, Kotlin compile clean, DEBUG APK built (104 MB), installed on device.
- Byte-identity: `git diff` shows zero value changes to any existing token — only additions and structural changes (header, fences).

### File List

- `scripts/gen-android-theme.mjs` (new)
- `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` (regenerated)
- `scripts/android-smoke.sh` (drift gate added before sync step)
- `scripts/android-build.sh` (drift gate added before sync step)

## Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-06-16 | Story created (post-ADR-0019 token-codegen, sequenced before the 9.5 rebuild). | bmad-create-story (Opus) |
| 2026-06-16 | Implemented: generator + KlarvoTheme.kt regenerated + drift gate wired into both build scripts. All ACs met. Commit 9ee9761. | dev-story (claude-sonnet-4-6) |
| 2026-06-16 | Fix pass (2 [Review][Patch] findings): (1) Drift gate vacuous-green on missing fences — replaced `(…||[''])[0]` with explicit fail-paths for both committed and generated side. (2) Completeness guard added — `CONSUMED_IDENTIFIERS` set asserted in both write and --check modes; throw if any AC2 identifier absent. Both inversions verified RED. All gates green. | dev-story (claude-sonnet-4-6) |
| 2026-06-16 | Code review (Opus, 3-layer) cleared: all ACs satisfied, gate non-vacuous, 26 canon colors → 26 constants, byte-identity confirmed. 2 patch findings fixed, 4 deferred (deferred-work.md), 4 dismissed. No GATE-4 smoke (build-tooling, byte-identical values → no pixels; human visual gate downgraded per DoD). story-conductor close-out → done. | story-conductor (Opus) |
