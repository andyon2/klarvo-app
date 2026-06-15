package com.klarvo.voice

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
    // --- Graphite neutral ladder ---
    const val Bg       = 0xFF0F1112.toInt()
    const val Surface  = 0xFF16181A.toInt()
    const val Surface2 = 0xFF1B1E20.toInt()
    const val Elevated = 0xFF232729.toInt()
    const val Border   = 0xFF282C2F.toInt()
    const val Border2  = 0xFF353A3E.toInt()

    // --- Text ---
    const val TextC    = 0xFFECEEEF.toInt()
    const val Muted    = 0xFFA4A9AC.toInt()
    const val Dim      = 0xFF6F7479.toInt()

    // --- Teal — brand / ready / processing / focus-ring ---
    const val Teal     = 0xFF29C7AC.toInt()
    const val TealHi   = 0xFF57DDC7.toInt()
    const val TealLo   = 0xFF1B9C88.toInt()
    const val OnTeal   = 0xFF05201B.toInt()

    // --- Amber — live/listening only (recording tally light) ---
    const val Amber    = 0xFFE9A24C.toInt()

    // --- Danger — stop / delete / error only ---
    const val Danger   = 0xFFEE6F63.toInt()

    // --- Alpha/glass-ring variants (for Canvas Paint.color) ---
    // TealBg: ~12% alpha teal — ring inner fill background
    const val TealBg      = 0x1F29C7AC.toInt()
    // AmberBg: ~12% alpha amber — ring inner fill background (recording state)
    const val AmberBg     = 0x1FE9A24C.toInt()
    // AmberLine: ~32% alpha amber — top border accent (recording panel; matches --k-amber-line = rgba(233,162,76,0.32))
    const val AmberLine   = 0x52E9A24C.toInt()
    // AmberHi: bright amber highlight — pulse ring / bar highlight
    const val AmberHi     = 0xFFF4BA72.toInt()
    // DangerBg: ~12% alpha danger — stop-button background (matches --k-danger-bg convention)
    const val DangerBg    = 0x1FEE6F63.toInt()
    // ShadowColor: 20% black — drop shadow
    const val ShadowColor = 0x33000000.toInt()
}
