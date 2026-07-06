#!/usr/bin/env node
/**
 * gen-android-theme.mjs
 *
 * Generates android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt
 * from the canon docs/design/overhaul/source/assets/klarvo.css
 *
 * Usage:
 *   node scripts/gen-android-theme.mjs          # write the file
 *   node scripts/gen-android-theme.mjs --check  # drift gate: exit 1 if GENERATED region differs
 *
 * ADR-0019: generate-not-transcribe closes the F6 copy-error class.
 * Color tokens only — radii / shadows / easing / durations / fonts are out of scope.
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dir = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dir, '..');

// ---------------------------------------------------------------------------
// Alias overrides (documented exceptions; PascalCase would give a different name)
// ---------------------------------------------------------------------------
// Each entry maps the suffix of the CSS token (after "--k-") to an explicit Kotlin identifier.
// Currently only one: --k-text → TextC (keeps the existing consumer identifier; avoids rename).
const ALIAS_OVERRIDES = {
  'text': 'TextC',
};

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------
const CSS_PATH = resolve(ROOT, 'docs/design/overhaul/source/assets/klarvo.css');
const OUT_PATH = resolve(ROOT, 'android/kotlin-src/com/klarvo/voice/KlarvoTheme.kt');

// ---------------------------------------------------------------------------
// Helpers: name mapping
// ---------------------------------------------------------------------------
/**
 * Convert a CSS token key (e.g. "teal-hi", "surface-2") to PascalCase Kotlin name.
 * Digits are kept ("surface-2" → "Surface2", "border-2" → "Border2").
 */
function toPascalCase(key) {
  // Check alias first
  if (ALIAS_OVERRIDES[key] !== undefined) return ALIAS_OVERRIDES[key];
  return key
    .split('-')
    .map(seg => seg.charAt(0).toUpperCase() + seg.slice(1))
    .join('');
}

// ---------------------------------------------------------------------------
// Helpers: value conversion
// ---------------------------------------------------------------------------
/**
 * Convert a CSS hex color (#RRGGBB) to an ARGB Android Int literal.
 * #RRGGBB → 0xFFRRGGBB.toInt()
 */
function hexToArgb(hex) {
  const clean = hex.replace('#', '').toUpperCase();
  if (clean.length !== 6) throw new Error(`Unexpected hex length: ${hex}`);
  return `0xFF${clean}.toInt()`;
}

/**
 * Convert rgba(r,g,b,a) to an ARGB Android Int literal.
 * AA = round(a * 255), zero-padded, uppercase.
 */
function rgbaToArgb(rgba) {
  const m = rgba.match(/rgba\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([\d.]+)\s*\)/i);
  if (!m) throw new Error(`Cannot parse rgba: ${rgba}`);
  const [, r, g, b, a] = m;
  const aa = Math.round(parseFloat(a) * 255).toString(16).padStart(2, '0').toUpperCase();
  const rr = parseInt(r).toString(16).padStart(2, '0').toUpperCase();
  const gg = parseInt(g).toString(16).padStart(2, '0').toUpperCase();
  const bb = parseInt(b).toString(16).padStart(2, '0').toUpperCase();
  return `0x${aa}${rr}${gg}${bb}.toInt()`;
}

/**
 * Determine if a CSS value is a color we handle (hex or rgba).
 * Returns false for radii (px), shadows (complex), fonts, easing, durations, etc.
 */
function isColorValue(value) {
  const v = value.trim();
  return v.startsWith('#') || /^rgba\s*\(/.test(v);
}

// ---------------------------------------------------------------------------
// Parse canon CSS
// ---------------------------------------------------------------------------
function parseCanonColors(cssPath) {
  const css = readFileSync(cssPath, 'utf8');

  // Extract the :root block
  const rootMatch = css.match(/:root\s*\{([^}]+)\}/s);
  if (!rootMatch) throw new Error('Cannot find :root block in canon CSS');
  const rootBlock = rootMatch[1];

  // Find all --k-* declarations
  const tokens = [];
  const re = /--k-([\w-]+)\s*:\s*([^;]+);/g;
  let m;
  while ((m = re.exec(rootBlock)) !== null) {
    const key = m[1].trim();
    const value = m[2].trim();
    if (isColorValue(value)) {
      tokens.push({ key, value });
    }
  }

  if (tokens.length === 0) throw new Error('No color tokens found in canon CSS');
  return tokens;
}

// ---------------------------------------------------------------------------
// Convert tokens to Kotlin entries
// ---------------------------------------------------------------------------
function convertTokens(tokens) {
  return tokens.map(({ key, value }) => {
    const name = toPascalCase(key);
    let literal;
    if (value.startsWith('#')) {
      literal = hexToArgb(value);
    } else if (/^rgba\s*\(/i.test(value)) {
      literal = rgbaToArgb(value);
    } else {
      throw new Error(`Unhandled color value for --k-${key}: ${value}`);
    }
    return { name, literal, key };
  });
}

// ---------------------------------------------------------------------------
// Assertion: F6 and alpha-class invariants (AC3)
// ---------------------------------------------------------------------------
function assertInvariants(entries) {
  const byName = Object.fromEntries(entries.map(e => [e.name, e.literal]));

  const expected = {
    AmberLine: '0x52E9A24C.toInt()',
    TealBg:    '0x1F29C7AC.toInt()',
    AmberBg:   '0x1FE9A24C.toInt()',
    DangerBg:  '0x1FEE6F63.toInt()',
  };

  for (const [name, lit] of Object.entries(expected)) {
    if (byName[name] !== lit) {
      throw new Error(
        `INVARIANT FAILED: ${name} = ${byName[name] ?? '(missing)'}, expected ${lit}\n` +
        `  Converter regression or canon changed — check round(alpha*255) logic.`
      );
    }
  }
}

// ---------------------------------------------------------------------------
// Completeness guard: all AC2 consumed identifiers must be present (AC2)
// ---------------------------------------------------------------------------
// Full set of identifiers referenced by FloatingBubbleView.kt and ListeningPanelView.kt.
// If any is absent after conversion (parse change, :root truncation, etc.) we fail loudly
// here rather than at kotlinc time.
const CONSUMED_IDENTIFIERS = [
  'Bg', 'Surface', 'Surface2', 'Elevated', 'Border', 'Border2',
  'TextC', 'Muted', 'Dim', 'Teal', 'TealHi', 'TealLo', 'OnTeal',
  'Amber', 'AmberHi', 'Danger', 'TealBg', 'AmberBg', 'AmberLine',
  'DangerBg', 'Success', 'SuccessHi',
];

function assertCompleteness(entries) {
  const emitted = new Set(entries.map(e => e.name));
  const missing = CONSUMED_IDENTIFIERS.filter(id => !emitted.has(id));
  if (missing.length > 0) {
    throw new Error(
      `COMPLETENESS FAILED: the following consumed identifiers are missing from the generated output:\n` +
      `  ${missing.join(', ')}\n` +
      `  This means a :root parse change or token renaming has silently dropped them.\n` +
      `  Check the canon CSS and/or ALIAS_OVERRIDES before proceeding.`
    );
  }
}

// ---------------------------------------------------------------------------
// Emit the Kotlin file content
// ---------------------------------------------------------------------------
function emitKotlin(entries) {
  const genLines = entries.map(({ name, literal }) => {
    const padding = ' '.repeat(Math.max(1, 14 - name.length));
    return `    const val ${name}${padding}= ${literal}`;
  });

  return `package com.klarvo.voice

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
${genLines.join('\n')}
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
`;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
function main() {
  const checkMode = process.argv.includes('--check');

  // 1. Parse canon
  const tokens = parseCanonColors(CSS_PATH);

  // 2. Convert
  const entries = convertTokens(tokens);

  // 3. Assert invariants (F6 + alpha class — AC3) + completeness (AC2)
  assertInvariants(entries);
  assertCompleteness(entries);

  // 4. Emit
  const newContent = emitKotlin(entries);

  if (checkMode) {
    // --check: compare GENERATED region of committed file vs. freshly generated
    if (!existsSync(OUT_PATH)) {
      console.error(`ERROR: ${OUT_PATH} does not exist — run without --check first.`);
      process.exit(1);
    }
    const committed = readFileSync(OUT_PATH, 'utf8');

    // Extract GENERATED regions for comparison
    const generatedRegionRe = /\/\/ ===== BEGIN GENERATED.*?\/\/ ===== END GENERATED =====/s;

    const newRegionMatch       = newContent.match(generatedRegionRe);
    if (!newRegionMatch) {
      // Internal error: the generator itself produced no GENERATED fence — something is
      // badly wrong with emitKotlin(). Never silently pass.
      console.error('INTERNAL ERROR: freshly-generated content has no GENERATED region fences.');
      console.error('This is a generator bug — check emitKotlin() and the fence markers.');
      process.exit(1);
    }
    const newRegion = newRegionMatch[0];

    const committedRegionMatch = committed.match(generatedRegionRe);
    if (!committedRegionMatch) {
      // The committed file has no GENERATED region: fences were removed or renamed.
      // Never compare two empty strings and call it "in sync".
      console.error('');
      console.error('╔══════════════════════════════════════════════════════════════════╗');
      console.error('║  ERROR: KlarvoTheme.kt has no GENERATED region fences.          ║');
      console.error('║  The fence markers were removed or renamed in the committed file.║');
      console.error('║  Aktion: node scripts/gen-android-theme.mjs                     ║');
      console.error('║  Dann committen und erneut bauen.                               ║');
      console.error('╚══════════════════════════════════════════════════════════════════╝');
      process.exit(1);
    }
    const committedRegion = committedRegionMatch[0];

    if (committedRegion !== newRegion) {
      console.error('');
      console.error('╔══════════════════════════════════════════════════════════════════╗');
      console.error('║  DRIFT DETECTED: KlarvoTheme.kt abgedriftet von Canon klarvo.css║');
      console.error('║  Aktion: node scripts/gen-android-theme.mjs                     ║');
      console.error('║  Dann committen und erneut bauen.                               ║');
      console.error('╚══════════════════════════════════════════════════════════════════╝');
      console.error('');
      // Show a minimal diff hint
      const committedLines = committedRegion.split('\n');
      const newLines       = newRegion.split('\n');
      const maxLen = Math.max(committedLines.length, newLines.length);
      let shownDiff = 0;
      for (let i = 0; i < maxLen && shownDiff < 10; i++) {
        const a = committedLines[i] ?? '';
        const b = newLines[i] ?? '';
        if (a !== b) {
          console.error(`  committed: ${a}`);
          console.error(`  canonical: ${b}`);
          shownDiff++;
        }
      }
      process.exit(1);
    }

    console.log('[ok] KlarvoTheme.kt is in sync with canon klarvo.css');
    process.exit(0);
  }

  // Write mode
  writeFileSync(OUT_PATH, newContent, 'utf8');
  console.log(`[ok] Generated ${OUT_PATH} (${entries.length} color tokens)`);
  console.log('[ok] Invariants verified: AmberLine=0x52E9A24C, TealBg=0x1F29C7AC, AmberBg=0x1FE9A24C, DangerBg=0x1FEE6F63');
}

main();
