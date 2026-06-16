package com.klarvo.voice

// GENERATED from docs/design/overhaul/source/assets/klarvo.css
// by scripts/gen-android-theme.mjs — DO NOT EDIT BY HAND.
// Provenance: ADR-0019 (cross-platform design SSOT — generate-not-transcribe).
// To regenerate: node scripts/gen-android-theme.mjs
// Drift gate: node scripts/gen-android-theme.mjs --check

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
 *
 * Usage with Canvas/Paint:
 *   paint.color = KlarvoTheme.Teal
 */
object KlarvoTheme {

    // ===== BEGIN GENERATED (canon colors — projected from klarvo.css --k-* tokens) =====
    const val BgDeep        = 0xFF0A0B0C.toInt()
    const val Bg            = 0xFF0F1112.toInt()
    const val Surface       = 0xFF16181A.toInt()
    const val Surface2      = 0xFF1B1E20.toInt()
    const val Elevated      = 0xFF232729.toInt()
    const val Border        = 0xFF282C2F.toInt()
    const val Border2       = 0xFF353A3E.toInt()
    const val Hairline      = 0x0EFFFFFF.toInt()
    const val TextC         = 0xFFECEEEF.toInt()
    const val Muted         = 0xFFA4A9AC.toInt()
    const val Dim           = 0xFF6F7479.toInt()
    const val Faint         = 0xFF4B4F53.toInt()
    const val Teal          = 0xFF29C7AC.toInt()
    const val TealHi        = 0xFF57DDC7.toInt()
    const val TealLo        = 0xFF1B9C88.toInt()
    const val TealBg        = 0x1F29C7AC.toInt()
    const val TealLine      = 0x5229C7AC.toInt()
    const val OnTeal        = 0xFF05201B.toInt()
    const val Amber         = 0xFFE9A24C.toInt()
    const val AmberHi       = 0xFFF4BA72.toInt()
    const val AmberBg       = 0x1FE9A24C.toInt()
    const val AmberLine     = 0x52E9A24C.toInt()
    const val Danger        = 0xFFEE6F63.toInt()
    const val DangerBg      = 0x1FEE6F63.toInt()
    const val Success       = 0xFF4FC58A.toInt()
    const val Info          = 0xFF57DDC7.toInt()
    // ===== END GENERATED =====

    // ---------------------------------------------------------------------------
    // Hand-maintained platform-derived constants (NOT canon --k-* tokens).
    // These are deliberately NOT in the generated region and are NOT diffed by the
    // drift gate. Add new platform-derived values here only — never in the GENERATED
    // region above.
    // ---------------------------------------------------------------------------

    // ShadowColor: 20% black — Android drop-shadow (no CSS --k-* equivalent;
    //   the canon's --k-e* properties are box-shadow strings, not a single color).
    const val ShadowColor = 0x33000000.toInt()
}
