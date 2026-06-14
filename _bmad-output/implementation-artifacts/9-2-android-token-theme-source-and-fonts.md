# Story 9.2: Android Token/Theme Source + Fonts (Foundation)

Status: review

## Story

As a developer,
I want the Android color/theme source and bundled fonts created,
so that the bubble and in-app surfaces re-skin against named tokens like the desktop (rather than the scattered hardcoded `Color.parseColor()` calls that exist today).

## Acceptance Criteria

**AC1 — Kotlin token object with Studio-Dark color ladder:**
Given no Android theme file exists today (colors are inline `Color.parseColor("#…")` in `FloatingBubbleView.kt`)
When 9.2 is done
Then a Kotlin object `KlarvoTheme` (or equivalent namespace) defines the Studio-Dark ladder as `Color(0xFF…)` per spec:
- `Bg = Color(0xFF0F1112)`
- `Surface = Color(0xFF16181A)`
- `Surface2 = Color(0xFF1B1E20)`
- `Elevated = Color(0xFF232729)`
- `Border = Color(0xFF282C2F)`
- `Border2 = Color(0xFF353A3E)`
- `TextC = Color(0xFFECEEEF)`
- `Muted = Color(0xFFA4A9AC)`
- `Dim = Color(0xFF6F7479)`
- `Teal = Color(0xFF29C7AC)`
- `TealHi = Color(0xFF57DDC7)`
- `TealLo = Color(0xFF1B9C88)`
- `OnTeal = Color(0xFF05201B)`
- `Amber = Color(0xFFE9A24C)`
- `Danger = Color(0xFFEE6F63)`

**AC2 — Color semantic constants documented:**
Given the Studio-Dark semantic rules (DT5)
When the token file is written
Then the token object (or an adjacent doc-comment block) documents the role of each semantic color:
- Teal = brand / ready / processing / focus-ring
- Amber = live/listening (recording only — tally light)
- Danger = stop / delete / error only
And alpha/glass variants are defined for use in View+Canvas rendering: Teal at 12% alpha (`TealBg`) for ring backgrounds, Amber at 12% alpha (`AmberBg`), and a shadow alpha constant (`ShadowColor = Color(0x33000000)`)

**AC3 — Geist + Geist Mono bundled as font resources:**
Given Android has no font resources today (`res/font/` does not exist)
When 9.2 is done
Then `.ttf` files for Geist (Regular 400, Medium 500, SemiBold 600, Bold 700) and Geist Mono (Regular 400, Medium 500) are added to a tracked `android/res-font/` directory
And the build script (`scripts/android-build.sh`) has a sync step that copies `android/res-font/*.ttf` to `src-tauri/gen/android/app/src/main/res/font/`
And `scripts/android-smoke.sh` also syncs the same font directory (so smoke builds include fonts)
And the fonts are accessed at runtime via `ResourcesCompat.getFont(context, R.font.geist_regular)` (or equivalent)

**AC4 — No-runtime-fetch constraint (BYOK/NFR6):**
Given the BYOK / no-phone-home constraint
When fonts are bundled
Then there is no network fetch for font data — fonts load entirely from bundled resources
And the app launches and displays text (using at least one of the new fonts on a reference surface) without network access

**AC5 — Android no-blur convention encoded:**
Given Android has no native `backdrop-blur`
When the glass surfaces are specified
Then a constant or doc-comment in the token source explicitly documents the glass-ring convention: solid `Surface` fill + 4dp teal/amber ring as the non-blur substitute (not `RenderEffect` backdrop blur)

**AC6 — App builds and launches:**
Given the new token file and font resources are in place
When `scripts/android-build.sh` is run
Then the app builds and installs cleanly on device
And on-device smoke confirms the app launches (no crash on startup) with the new token file and fonts present — verified via APK freshness check (`scripts/android-build.sh` timestamp gate), not a version screen

**Inversion (must-fail gate):** A submission that relies on network font loading, or that uses `Color.parseColor()` instead of the `Color(0xFF…)` form for any token in `KlarvoTheme`, must not pass review.

**DoD:** Android builds; on-device smoke that the app launches with the new theme on ≥1 reference surface; APK freshness verified via `scripts/android-build.sh` timestamp gate; `scripts/android-smoke.sh` syncs fonts correctly.

## Tasks / Subtasks

- [x] **Task 1: Create the Kotlin token source file** (AC: 1, 2, 5)
  - [x] 1.1 Create `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` as a Kotlin `object` (not a class) with all Studio-Dark `Color(0xFF…)` constants named exactly as in AC1
  - [x] 1.2 Add alpha/glass-ring variants as named constants in the same object: `TealBg` (`Color(0x1F29C7AC)` ≈ 12% alpha teal), `AmberBg` (`Color(0x1FE9A24C)` ≈ 12% alpha amber), `ShadowColor` (`Color(0x33000000)`)
  - [x] 1.3 Add a doc-comment block at the top of the file documenting color semantics per AC2 and the no-blur glass-ring convention per AC5
  - [x] 1.4 Do NOT change `FloatingBubbleView.kt` in this story — the bubble re-skin is Epic 9.3+; `KlarvoTheme` is the foundation, not the consumer

- [x] **Task 2: Acquire and add Geist + Geist Mono font files** (AC: 3, 4)
  - [x] 2.1 Download the Geist and Geist Mono font files from the official Vercel Geist GitHub repo (`github.com/vercel/geist-font/releases`) — use `.ttf` format (Android font resources require `.ttf` or `.otf`, not `.woff2`)
  - [x] 2.2 Rename files to snake_case for Android resource compatibility: `geist_regular.ttf`, `geist_medium.ttf`, `geist_semibold.ttf`, `geist_bold.ttf`, `geist_mono_regular.ttf`, `geist_mono_medium.ttf`
  - [x] 2.3 Place all six files in `android/res-font/` (a NEW tracked directory — does NOT currently exist)
  - [x] 2.4 Verify `.ttf` files are valid by checking file size is reasonable (each should be in the range of 50–500 KB)

- [x] **Task 3: Wire font sync into build scripts** (AC: 3, 6)
  - [x] 3.1 Add a sync step to `scripts/android-build.sh` (after the Kotlin sources sync, before the build) that creates `src-tauri/gen/android/app/src/main/res/font/` if it doesn't exist and copies `android/res-font/*.ttf` into it — follow the same pattern as the existing `res/xml` sync
  - [x] 3.2 Add the same sync step to `scripts/android-smoke.sh` immediately after the Kotlin sources sync step (line ~94 area) so smoke builds also get fonts
  - [x] 3.3 Verify the gen-dir sync step correctly handles re-runs (idempotent: re-copying the same files over is fine)

- [ ] **Task 4: Verify the build end-to-end** (AC: 4, 6 + DoD)
  - [ ] 4.1 Run `scripts/android-build.sh` — must succeed without error
  - [ ] 4.2 Verify APK freshness via the build script's timestamp gate output (look for "BUILD OK (Android)")
  - [ ] 4.3 Install and launch the app on device — verify no crash at startup (logcat output clean)
  - [ ] 4.4 Verify the font resources are present in the installed APK: `adb shell cmd package list packages | grep klarvo` to confirm install, then optionally `unzip -l <apk-path> | grep "res/font"` to confirm font files shipped
  - [ ] 4.5 There is no smoke step to visually verify font rendering in this story — the first consumer of `R.font.geist_*` will be Story 9.5+ when text is drawn in the bubble panel; this story's DoD is APK-launch + resource presence, not visual rendering of Geist text

- [ ] **Task 5: Commit** (AC: all)
  - [ ] 5.1 Stage only: `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` (new), `android/res-font/*.ttf` (new, 6 files), `scripts/android-build.sh` (modified), `scripts/android-smoke.sh` (modified)
  - [ ] 5.2 Never `git add .` — verify staged files only (no `.gitignore`, no gen/ artifacts)
  - [ ] 5.3 Commit message: `feat(android): 9-2 Studio-Dark token source + Geist font resources`

## Dev Notes

### What Exists Today (Baseline)

**`FloatingBubbleView.kt`** has all colors inline as hardcoded `Color.parseColor()` calls (lines 67–71):
```kotlin
private val colorIdleBackground = Color.parseColor("#F5F5F5")  // light grey/white
private val colorRecordingBar   = Color.parseColor("#EF4444")  // red
private val colorCancelBtn      = Color.parseColor("#CC2222")  // darker red
private val colorConfirmBtn     = Color.parseColor("#22C55E")  // green
private val colorProcessing     = Color.parseColor("#F59E0B")  // amber
```
These are NOT migrated in Story 9.2 — that's Story 9.3+ (re-skin). This story only creates the token source.

**No `res/font/` directory exists anywhere** — neither in `android/` (tracked) nor in `src-tauri/gen/android/` (generated). `android/res-values/` exists (contains `strings.xml`). The new `android/res-font/` directory is a new tracked directory mirroring the same pattern.

**Existing colors.xml in gen/** is a Tauri scaffold with Material Design placeholder colors (purple, teal_200, black, white) — **completely unrelated to the app's actual rendering**. Story 9.2 does NOT touch `colors.xml` at all; the color tokens live as a Kotlin object, not as XML resources (consistent with how all other colors work in the codebase).

**No Typeface/font references anywhere in Android Kotlin source** — the bubble currently draws only geometric shapes + the app icon via `ContextCompat.getDrawable()`. The `textPaint` in `FloatingBubbleView.kt` is defined but unused in practice. Geist fonts will be first used when Story 9.5 draws the raw transcript text in the listening panel.

### KlarvoTheme.kt — Recommended Structure

```kotlin
package com.klarvo.voice

import android.graphics.Color as AndroidColor

/**
 * Studio-Dark color tokens for Klarvo Android.
 *
 * Color semantics (binding rules — DT5):
 *   Teal   = brand / ready / processing / focus-ring
 *   Amber  = live/listening (recording only — tally light)
 *   Danger = stop / delete / error only
 *
 * Android glass-ring convention (no native backdrop-blur):
 *   Glass effects = solid Surface fill + 4dp Teal/Amber ring.
 *   Use TealBg/AmberBg (12% alpha) for ring inner fill backgrounds.
 *   No RenderEffect.createBlurEffect — unsupported below API 31 (minSdk=24).
 */
object KlarvoTheme {
    // Graphite neutral ladder
    val Bg        = AndroidColor.valueOf(0xFF0F1112.toInt())
    val Surface   = AndroidColor.valueOf(0xFF16181A.toInt())
    val Surface2  = AndroidColor.valueOf(0xFF1B1E20.toInt())
    val Elevated  = AndroidColor.valueOf(0xFF232729.toInt())
    val Border    = AndroidColor.valueOf(0xFF282C2F.toInt())
    val Border2   = AndroidColor.valueOf(0xFF353A3E.toInt())
    // Text
    val TextC     = AndroidColor.valueOf(0xFFECEEEF.toInt())
    val Muted     = AndroidColor.valueOf(0xFFA4A9AC.toInt())
    val Dim       = AndroidColor.valueOf(0xFF6F7479.toInt())
    // Teal — brand / ready / processing
    val Teal      = AndroidColor.valueOf(0xFF29C7AC.toInt())
    val TealHi    = AndroidColor.valueOf(0xFF57DDC7.toInt())
    val TealLo    = AndroidColor.valueOf(0xFF1B9C88.toInt())
    val OnTeal    = AndroidColor.valueOf(0xFF05201B.toInt())
    // Amber — live/listening only
    val Amber     = AndroidColor.valueOf(0xFFE9A24C.toInt())
    // Semantic
    val Danger    = AndroidColor.valueOf(0xFFEE6F63.toInt())
    // Alpha/glass-ring variants (for Canvas Paint.color)
    val TealBg    = AndroidColor.valueOf(0x1F29C7AC.toInt())   // ~12% alpha
    val AmberBg   = AndroidColor.valueOf(0x1FE9A24C.toInt())   // ~12% alpha
    val ShadowColor = AndroidColor.valueOf(0x33000000.toInt()) // 20% black shadow
}
```

**Note on `android.graphics.Color` vs `android.graphics.Color.valueOf`:** `FloatingBubbleView.kt` currently uses `android.graphics.Color` (the integer API via `Color.parseColor()`). For `Canvas`/`Paint` use, the integer form works fine on minSdk 24. Use `Color.valueOf(int)` (API 26+) or the `ARGB` int directly. The safest approach for canvas paints is to define `val Teal = 0xFF29C7AC.toInt()` (an `Int`) which works with `paint.color = KlarvoTheme.Teal` directly. Choose whichever form is more consistent with the canvas usage patterns in the existing codebase.

**Simpler approach (recommended for Canvas-based rendering):**
```kotlin
object KlarvoTheme {
    const val Bg        = 0xFF0F1112.toInt()
    const val Surface   = 0xFF16181A.toInt()
    // ... etc
    const val TealBg    = 0x1F29C7AC.toInt()   // ~12% alpha
}
```
Then usage: `paint.color = KlarvoTheme.Teal` — matches the current `Color.parseColor()` integer API perfectly, no API-level concerns.

### Font Acquisition — Source and Naming

**Official source:** `https://github.com/vercel/geist-font/releases` — download the latest release, look for the TTF files. Alternative: `github.com/vercel/geist-font/tree/main/packages/geist/src/fonts`.

**Naming required for Android XML resources** (resource names must be lowercase letters, digits, underscores only):
- `geist_regular.ttf` (weight 400)
- `geist_medium.ttf` (weight 500)
- `geist_semibold.ttf` (weight 600)
- `geist_bold.ttf` (weight 700)
- `geist_mono_regular.ttf` (weight 400)
- `geist_mono_medium.ttf` (weight 500)

These will be accessible as `R.font.geist_regular`, `R.font.geist_mono_regular`, etc.

**`ResourcesCompat.getFont()` usage (future stories):**
```kotlin
import androidx.core.content.res.ResourcesCompat
val typeface = ResourcesCompat.getFont(context, R.font.geist_regular)
textPaint.typeface = typeface
```
`ResourcesCompat` is available via `appcompat:1.7.1` (already in build.gradle.kts — no new dependency needed).

### Build Script Sync Pattern

The existing `android-build.sh` pattern for resource sync (from the `res/xml` step at line ~72):
```bash
XML_DST="$GEN_ANDROID/app/src/main/res/xml"
mkdir -p "$XML_DST"
if [ ! -f "$XML_DST/accessibility_service_config.xml" ]; then
    cp "android/res-xml/accessibility_service_config.xml" "$XML_DST/"
fi
```

The new font sync step should follow the same `mkdir -p` + `cp` pattern:
```bash
# ---------------------------------------------------------------------------
# X. Font resources (Geist + Geist Mono)
# ---------------------------------------------------------------------------
FONT_DST="$GEN_ANDROID/app/src/main/res/font"
mkdir -p "$FONT_DST"
echo "[sync] Copying font resources from android/res-font/ to gen/android/"
cp android/res-font/*.ttf "$FONT_DST/"
echo "[sync] Done: $(ls -1 android/res-font/*.ttf | wc -l) font files copied"
```
Add this block **before** the build step (#9), after the Kotlin sources sync. Add the same block to `android-smoke.sh` after step #2 (Kotlin sources sync, around line 94).

### No XML font_family Descriptor Needed

Android's XML font-family descriptor (`res/font/geist.xml`) is used for `app:fontFamily` attributes in XML layouts. Since this project has no XML-based text views (the bubble is all-canvas, no `layout/bubble.xml`), no font-family XML descriptor is needed in Story 9.2. Future stories that draw text via `Canvas` will call `ResourcesCompat.getFont()` directly.

### Decision from ADR-0018 (Story 9.1 Gate)

**Substrate: View+Canvas (Option A)** — Compose is rejected due to LifecycleOwner requirement in overlay Service context.

This means:
- `KlarvoTheme` colors will be consumed as `paint.color = KlarvoTheme.Teal` (integer API)
- Fonts will be loaded via `ResourcesCompat.getFont()` and assigned to `textPaint.typeface`
- No Compose Color API, no `MaterialTheme`, no Compose dependencies
- The 9.4 state harness uses `adb shell am broadcast` (BuildConfig.DEBUG) — no impact on 9.2

### Android Source Sync Architecture (Critical)

`android/kotlin-src/` is the **tracked source**; `src-tauri/gen/android/` is **gitignored generated output**. The build scripts (`android-build.sh`, `android-smoke.sh`) are the sync mechanism. Story 9.2 follows the same pattern for fonts:

| Tracked source (git) | Synced to gen/ by script | How accessed in app |
|---|---|---|
| `android/kotlin-src/*.kt` | `gen/android/app/src/main/java/…/*.kt` | Compiled directly |
| `android/res-xml/accessibility_service_config.xml` | `gen/android/app/src/main/res/xml/` | `R.xml.*` |
| `android/res-values/strings.xml` | patched into `gen/android/app/src/main/res/values/strings.xml` | `R.string.*` |
| `android/res-font/*.ttf` **(NEW — 9.2)** | `gen/android/app/src/main/res/font/` | `R.font.*` |

**`android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt`** is also synced automatically by the existing `cp $SRC/*.kt $DST/` step — no change needed for the Kotlin file itself.

### Files Modified in This Story

| File | Change |
|------|--------|
| `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` | **New** — Studio-Dark token object |
| `android/res-font/geist_regular.ttf` | **New** — Geist Regular |
| `android/res-font/geist_medium.ttf` | **New** — Geist Medium |
| `android/res-font/geist_semibold.ttf` | **New** — Geist SemiBold |
| `android/res-font/geist_bold.ttf` | **New** — Geist Bold |
| `android/res-font/geist_mono_regular.ttf` | **New** — Geist Mono Regular |
| `android/res-font/geist_mono_medium.ttf` | **New** — Geist Mono Medium |
| `scripts/android-build.sh` | Modified — add font sync step |
| `scripts/android-smoke.sh` | Modified — add font sync step |

**No files in `src-tauri/` are modified** (the gen/android/ changes are produced by the build script, not tracked).  
**`FloatingBubbleView.kt` is NOT modified in this story** — re-skin is 9.3+.  
**No `colors.xml` changes** — the token ladder lives in Kotlin, not XML.

### What This Story Does NOT Do

- Does NOT migrate `FloatingBubbleView.kt` to use `KlarvoTheme` — that is Story 9.3
- Does NOT create any XML font-family descriptor (not needed for canvas rendering)
- Does NOT create a `themes.xml` overlay (the existing stub is fine; Android theming via XML is not needed for the overlay service rendering approach)
- Does NOT add any new Kotlin dependencies to `build.gradle.kts`
- Does NOT use `ComposeView`, `MaterialTheme`, or any Compose API (ADR-0018 decision: View+Canvas only)
- Does NOT add a `BgDeep` constant — the SPEC Kotlin table does not include `BgDeep` (only the CSS `@theme` has it for the letterbox/behind-windows desktop context; not needed on Android)

### References

- [Source: epics-visual-overhaul.md, Story 9.2] — Story ACs, DoD, requirements DT2/DT3/DT4/DT5
- [Source: docs/design/overhaul/SPEC-studio-dark-overhaul.md, "Android — Kotlin/Compose" section] — Exact `Color(0xFF…)` values for all tokens (binding specification)
- [Source: docs/design/overhaul/SPEC-studio-dark-overhaul.md, "Color semantics" section] — Teal/Amber/Danger semantic rules (DT5)
- [Source: docs/design/overhaul/SPEC-studio-dark-overhaul.md, "⚠️ Machbarkeits-Constraints" #3] — No backdrop-blur on Android; glass ring = solid fill + 4dp ring
- [Source: docs/adr/0018-android-bubble-rendering-tech.md] — Substrate decision: View+Canvas, no Compose; 9.4 harness via adb broadcast
- [Source: android/kotlin-src/com/klarvo/voice/FloatingBubbleView.kt, lines 67–71] — Current hardcoded Color.parseColor() calls being replaced in 9.3+
- [Source: scripts/android-build.sh] — Build/sync architecture; font sync must follow same pattern as res/xml sync
- [Source: scripts/android-smoke.sh] — Smoke sync must mirror android-build.sh font step
- [Source: _bmad-output/project-context.md, "Android" section] — jni 0.21 pinned, minSdk 24, no Compose, canvas-based rendering
- [Source: _bmad-output/project-context.md, "Critical Don't-Miss Rules"] — BYOK / no network font loading; no telemetry
- [Source: _bmad-output/implementation-artifacts/8-1-token-and-type-foundation.md, "Font Bundling" section] — Desktop font bundling pattern (parallel to this story; different mechanism)
- [Source: epics-visual-overhaul.md, AR2] — Token source + fonts are a new artifact; none exists today

## Dev Agent Record

### Agent Model Used

claude-sonnet-4-6 (story-context pass, 2026-06-14)

### Debug Log References

(none — foundation story, no code execution)

### Completion Notes List

- **Task 1 DONE:** `KlarvoTheme.kt` created as a Kotlin `object` with 18 `const val` integer constants (15 primary + 3 alpha/glass variants). Uses raw `0xFF….toInt()` form — direct `Paint.color` assignment, no `android.graphics.Color` import needed, works on minSdk 24. Doc-comment covers color semantics (DT5) + no-blur glass-ring convention (AC5). FloatingBubbleView.kt NOT touched (correct — 9.3+ scope).
- **Task 2 DONE:** Downloaded Geist v1.7.2 from `github.com/vercel/geist-font/releases`. Extracted 6 TTF files: `geist_regular.ttf` (123 KB), `geist_medium.ttf` (125 KB), `geist_semibold.ttf` (125 KB), `geist_bold.ttf` (126 KB), `geist_mono_regular.ttf` (145 KB), `geist_mono_medium.ttf` (146 KB). All in range 50–500 KB. No network fetch at runtime (bundled). New tracked directory `android/res-font/` created.
- **Task 3 DONE:** Font sync block added to `android-build.sh` (section 1b, after Kotlin sources, before accessibility XML). Same block added to `android-smoke.sh` (after Kotlin sources sync, step 2). Pattern: `mkdir -p` + `cp *.ttf` — idempotent. Font sync against real `gen/android/app/src/main/res/font/` tested and confirmed (6 files synced). Both scripts pass `bash -n` syntax check.
- **Task 4 PENDING:** on-device build + smoke is Andi's gate (requires `scripts/android-build.sh` on Windows + device). All agent-observable prerequisites are verified (files in place, scripts syntactically valid, font-sync to gen/ confirmed).

### File List

- `android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt` — **new** — Studio-Dark token object
- `android/res-font/geist_regular.ttf` — **new** — Geist Regular 400 (123 KB)
- `android/res-font/geist_medium.ttf` — **new** — Geist Medium 500 (125 KB)
- `android/res-font/geist_semibold.ttf` — **new** — Geist SemiBold 600 (125 KB)
- `android/res-font/geist_bold.ttf` — **new** — Geist Bold 700 (126 KB)
- `android/res-font/geist_mono_regular.ttf` — **new** — Geist Mono Regular 400 (145 KB)
- `android/res-font/geist_mono_medium.ttf` — **new** — Geist Mono Medium 500 (146 KB)
- `scripts/android-build.sh` — **modified** — added font sync step (section 1b)
- `scripts/android-smoke.sh` — **modified** — added font sync step (after Kotlin sources sync)

## Change Log

- 2026-06-14: Story implemented — KlarvoTheme.kt (18 color constants), Geist v1.7.2 fonts (6 TTF files), font sync steps added to android-build.sh + android-smoke.sh. Task 4 (on-device build) is Andi's gate.
